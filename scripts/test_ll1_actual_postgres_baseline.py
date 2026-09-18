#!/usr/bin/env python3
"""PARTIAL LL1 proof: unchanged full PG schema, actual roles and baseline blockers.

Only a generated private cluster; no connection arguments, live endpoint, package
access, migration rewrite, trigger bypass or retirement implementation.
"""
import argparse
import hashlib
import json
from pathlib import Path
import select
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
from test_ll1_retirement_constraints import Postgres
from audit_ll1_schema_inventory import inventory

ROOT = Path(__file__).resolve().parent.parent
SCHEMAS = "('photara','photara_identity','photara_private')"


def uid(n):
    return f"00000000-0000-0000-0000-{n:012x}"


def ctx(library=1, actor=10):
    return f"SET LOCAL photara.library_id='{uid(library)}'; SET LOCAL photara.account_id='{uid(actor)}'; SET LOCAL photara.identity_id='{uid(actor+100)}'; SET LOCAL photara.scope_kind='library';"


def login_command(pg, login):
    return ["psql", "-X", "-qAt", "-v", "ON_ERROR_STOP=1", "-h", str(pg.socket), "-p", "55439", "-U", login, "-d", "postgres"]


def execute(pg, sql, login="ll1_fixture"):
    return subprocess.run(login_command(pg, login), input=sql, text=True, capture_output=True, env=pg.env)


def require(pg, sql, *, login="ll1_fixture", error=None, value=None):
    result = execute(pg, sql, login)
    if error:
        assert result.returncode != 0 and error in result.stderr, result.stderr
    else:
        assert result.returncode == 0, result.stderr
        if value is not None:
            assert result.stdout.strip() == value, result.stdout
    return result.stdout.strip()


def install(pg):
    require(pg, """
CREATE ROLE photara_owner NOLOGIN NOSUPERUSER NOBYPASSRLS;
CREATE ROLE photara_api NOLOGIN NOSUPERUSER NOBYPASSRLS;
CREATE ROLE photara_control NOLOGIN NOSUPERUSER NOBYPASSRLS;
CREATE ROLE photara_auth_read NOLOGIN NOSUPERUSER NOBYPASSRLS;
GRANT CREATE ON DATABASE postgres TO photara_owner;
GRANT CREATE,USAGE ON SCHEMA public TO photara_owner;
SET ROLE photara_owner;
CREATE TABLE public._sqlx_migrations(version bigint PRIMARY KEY, description text NOT NULL,
 installed_on timestamptz NOT NULL DEFAULT now(), success boolean NOT NULL,
 checksum bytea NOT NULL, execution_time bigint NOT NULL);
""")
    template = (ROOT / "crates/photara-service/deploy/runtime_roles.sql").read_text()
    for key, name in (("api_login", "ll1_api"), ("control_login", "ll1_control"), ("auth_login", "ll1_auth")):
        template = template.replace(f':"{key}"', '"' + name + '"')
    require(pg, template)
    fingerprints = {}
    for path in sorted((ROOT / "crates/photara-service/migrations/postgres").glob("*.sql")):
        raw = path.read_bytes()
        fingerprints[path.name] = hashlib.sha256(raw).hexdigest()
        body = raw.decode()
        if path.name.startswith("0007_"):
            # Matches Service::migrate's metadata/normalization initialization point.
            body += "\nINSERT INTO photara.schema_metadata VALUES(true,'photara.service.g2',1,1,'photara.canonical-json.v1',now());"
            body += "\nINSERT INTO photara.normalization_policies VALUES(1,'16.0.0',sha256('fixture'::bytea),'disposable fixture');"
        ledger = f"INSERT INTO public._sqlx_migrations(version,description,success,checksum,execution_time) VALUES({int(path.name[:4])},'{path.stem}',true,decode('{hashlib.sha384(raw).hexdigest()}','hex'),0);"
        require(pg, "BEGIN; SET LOCAL ROLE photara_owner;\n" + body + "\n" + ledger + "COMMIT;")
    return fingerprints


def measure(pg):
    source = inventory("postgres")
    def rows(sql):
        return json.loads(pg.scalar(f"SELECT coalesce(json_agg(q),'[]'::json) FROM ({sql}) q"))
    tables = rows(f"SELECT n.nspname||'.'||c.relname AS name,pg_get_userbyid(c.relowner) AS owner,c.relrowsecurity AS rls,c.relforcerowsecurity AS forced FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname IN {SCHEMAS} AND c.relkind='r'")
    assert {r['name'] for r in tables} == set(source['tables'])
    assert all(r['owner'] == 'photara_owner' for r in tables)
    fk = rows(f"""SELECT c.conrelid::regclass::text AS child,c.confrelid::regclass::text AS parent,
 ARRAY(SELECT a.attname FROM unnest(c.conkey) WITH ORDINALITY k(n,o) JOIN pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.n ORDER BY k.o) AS columns,
 ARRAY(SELECT a.attname FROM unnest(c.confkey) WITH ORDINALITY k(n,o) JOIN pg_attribute a ON a.attrelid=c.confrelid AND a.attnum=k.n ORDER BY k.o) AS parent_columns,
 c.confdeltype AS action,c.condeferred AS deferred,c.convalidated AS valid
 FROM pg_constraint c JOIN pg_namespace n ON n.oid=c.connamespace WHERE n.nspname IN {SCHEMAS} AND c.contype='f'""")
    actions = {'r':'RESTRICT','a':'NO ACTION','c':'CASCADE','n':'SET NULL','d':'SET DEFAULT'}
    def edge(e):
        return (e['child'],tuple(e['columns']),e['parent'],tuple(e['parent_columns']),e['delete'],e['deferred'])
    assert sorted(edge(e) for e in source['fks']) == sorted(edge(dict(e,delete=actions[e['action']])) for e in fk)
    assert all(e['valid'] for e in fk)
    triggers = rows(f"SELECT c.oid::regclass::text AS relation,t.tgname AS name,t.tgenabled AS enabled FROM pg_trigger t JOIN pg_class c ON c.oid=t.tgrelid JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname IN {SCHEMAS} AND NOT t.tgisinternal")
    assert {(t['relation'],t['name']) for t in triggers} == {(t['table'],t['name']) for t in source['triggers']}
    assert all(t['enabled']=='O' for t in triggers)
    policies = rows(f"SELECT schemaname||'.'||tablename||':'||policyname AS name FROM pg_policies WHERE schemaname IN {SCHEMAS}")
    assert {p['name'] for p in policies} == set(source['policies'])
    functions = rows(f"SELECT n.nspname||'.'||p.proname AS name,pg_get_userbyid(p.proowner) AS owner,p.prosecdef AS definer FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname IN {SCHEMAS}")
    assert {f['name'] for f in functions} == set(source['functions'])
    assert all(f['owner']=='photara_owner' for f in functions)
    for table in tables:
        assert table['rls'] == bool(source['tables'][table['name']]['rls'])
        assert table['forced'] == any('FORCE ROW' in s for s in source['tables'][table['name']]['rls'])
    require(pg, "SELECT count(*) FROM pg_roles WHERE rolname IN ('ll1_api','ll1_control','ll1_auth') AND rolcanlogin AND NOT rolsuper AND NOT rolbypassrls AND NOT rolcreaterole AND NOT rolcreatedb AND NOT rolreplication", value="3")
    for login, role in (('ll1_api','photara_api'),('ll1_control','photara_control'),('ll1_auth','photara_auth_read')):
        require(pg, f"SELECT current_user||':'||pg_has_role(current_user,'{role}','MEMBER')::text||':'||pg_has_role(current_user,'photara_owner','MEMBER')::text", login=login, value=f"{login}:true:false")
    return {'tables':len(tables),'foreign_keys':len(fk),'triggers':len(triggers),'policies':len(policies),'functions':len(functions),'forced_rls_tables':sum(t['forced'] for t in tables),'fingerprint':source['fingerprint']}


def seed(pg):
    require(pg, f"""BEGIN; SET LOCAL ROLE photara_owner; {ctx()}
INSERT INTO photara_identity.accounts VALUES('{uid(10)}','owner','active',1,now(),now(),NULL),('{uid(20)}','other','active',1,now(),now(),NULL),('{uid(30)}','viewer','active',1,now(),now(),NULL);
INSERT INTO photara_identity.account_identities VALUES('{uid(110)}','{uid(10)}','https://fake.invalid/','owner','active',1,now(),now(),NULL),('{uid(120)}','{uid(20)}','https://fake.invalid/','other','active',1,now(),now(),NULL),('{uid(130)}','{uid(30)}','https://fake.invalid/','viewer','active',1,now(),now(),NULL);
INSERT INTO photara_identity.devices VALUES('{uid(10)}','{uid(210)}','fixture','active',now(),NULL);
INSERT INTO photara.libraries VALUES('{uid(1)}','Target','active',1,now(),now(),NULL,1,'{{}}');
INSERT INTO photara.library_contract_state VALUES('{uid(1)}',1,'cloud-member',NULL,1,1,1,now(),now());
INSERT INTO photara_identity.memberships VALUES('{uid(310)}','{uid(1)}','{uid(10)}','owner','active',1,now(),now(),NULL),('{uid(330)}','{uid(1)}','{uid(30)}','viewer','active',1,now(),now(),NULL);
INSERT INTO photara_identity.account_defaults VALUES('{uid(10)}','{uid(1)}',1,now(),now());
INSERT INTO photara.people(person_id,library_id,record_schema,revision,created_at,updated_at,state,retired_at,display_name,sort_key) VALUES('{uid(410)}','{uid(1)}',1,1,now(),now(),'tombstoned',now(),'Retained tombstone','retained');
INSERT INTO photara.project_catalog VALUES('{uid(1)}','{uid(510)}','hidden',1,now(),now());
INSERT INTO photara.project_locators VALUES('{uid(511)}','{uid(1)}','{uid(510)}',NULL,NULL,'retired',1,now(),now(),now());
INSERT INTO photara_private.billing_events VALUES('fixture','event',sha256('fixture'::bytea),now(),'fixture','{{}}');
INSERT INTO photara_private.library_subscriptions VALUES('{uid(610)}','{uid(1)}','fixture','subscription','test','active',1,now(),now()+interval '1 day','event',now(),now());
INSERT INTO photara_private.scoped_streams VALUES('{uid(710)}','{uid(1)}','library',NULL,'{uid(711)}',1,now());
INSERT INTO photara_private.scoped_change_batches VALUES('{uid(710)}','{uid(711)}',1,'{uid(1)}',NULL,'{uid(712)}',1,'{{}}'::bytea,sha256('{{}}'::bytea),now());
INSERT INTO photara_private.scoped_command_receipts VALUES('{uid(1)}','{uid(712)}','{uid(10)}','{uid(210)}','library',NULL,'fixture','{{}}'::bytea,sha256('{{}}'::bytea),'accepted','{{}}'::bytea,sha256('{{}}'::bytea),'{uid(710)}','{uid(711)}',1,now());
INSERT INTO photara_private.scoped_changes VALUES('{uid(710)}','{uid(711)}',1,0,'{uid(1)}',NULL,'fixture','{uid(410)}',1,'fixture',1,'{{}}','{{}}'::bytea,sha256('{{}}'::bytea));
INSERT INTO photara.storage_roots VALUES('{uid(810)}','{uid(1)}','Retired root','retired','fixture',1,'tombstoned',now(),now(),now());
INSERT INTO photara.storage_slots VALUES('{uid(811)}','{uid(1)}','first','First','{uid(810)}','tombstoned',now(),1,1,now(),now()),('{uid(812)}','{uid(1)}','second','Second','{uid(810)}','tombstoned',now(),1,1,now(),now());
INSERT INTO photara.storage_slot_names VALUES('{uid(1)}','first','{uid(811)}',now()),('{uid(1)}','second','{uid(812)}',now());
INSERT INTO photara.library_variables VALUES('{uid(910)}','{uid(1)}','fixture','first','First','','fixture',1,'fixture',1,NULL,'[]','ordinary','portable','{{}}','tombstoned',now(),1,1,now(),now()),('{uid(911)}','{uid(1)}','fixture','second','Second','','fixture',1,'fixture',1,NULL,'[]','ordinary','portable','{{}}','tombstoned',now(),1,1,now(),now());
INSERT INTO photara.library_variable_names VALUES('{uid(1)}','first','{uid(910)}',now()),('{uid(1)}','second','{uid(911)}',now());
COMMIT;
BEGIN; SET LOCAL ROLE photara_owner; {ctx(2,20)}
INSERT INTO photara.libraries VALUES('{uid(2)}','Unrelated','active',1,now(),now(),NULL,1,'{{}}');
INSERT INTO photara.library_contract_state VALUES('{uid(2)}',1,'cloud-member',NULL,1,1,1,now(),now());
INSERT INTO photara_identity.memberships VALUES('{uid(320)}','{uid(2)}','{uid(20)}','owner','active',1,now(),now(),NULL);
COMMIT;""")


def snapshot(pg):
    tables = inventory('postgres')['tables']
    parts = [f"SELECT '{table}' AS name,coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),'[]'::jsonb) AS rows FROM {table} t" for table in tables]
    return pg.scalar("SELECT jsonb_object_agg(name,rows) FROM (" + " UNION ALL ".join(parts) + ") q")


def test_cases(pg):
    initial = snapshot(pg)
    tests = []
    def check(name, sql, login='ll1_fixture', error=None, value=None):
        require(pg, sql, login=login, error=error, value=value)
        assert snapshot(pg) == initial, name + ': baseline changed'
        tests.append(name)
        print(json.dumps({'case':name,'result':'pass','expected':error or value or 'rollback'}),flush=True)
    def tx(sql, library=1, actor=10, owner=False):
        return 'BEGIN;' + ('SET LOCAL ROLE photara_owner;' if owner else '') + ctx(library,actor) + sql + ';ROLLBACK;'
    check('api_owner_manage_access',tx(f"SELECT photara_private.authorize_library('{uid(1)}','manage-access')"),login='ll1_api')
    check('api_viewer_read',tx(f"SELECT count(*) FROM photara.libraries WHERE library_id='{uid(1)}'",actor=30),login='ll1_api',value='1')
    check('api_viewer_cannot_manage_access',tx(f"SELECT photara_private.authorize_library('{uid(1)}','manage-access')",actor=30),login='ll1_api',error='not_found_or_forbidden')
    check('api_cross_library_hidden',tx(f"SELECT count(*) FROM photara.libraries WHERE library_id='{uid(2)}'",library=2),login='ll1_api',value='0')
    check('api_identity_mismatch',f"BEGIN;{ctx()} SET LOCAL photara.identity_id='{uid(120)}'; SELECT photara_private.authorize_library('{uid(1)}','manage-access');",login='ll1_api',error='not_found_or_forbidden')
    check('auth_role_domain_read_denied',f"SELECT * FROM photara.libraries",login='ll1_auth',error='permission denied')
    for login in ('ll1_api','ll1_control'):
        check(login+'_root_delete_privilege',tx(f"DELETE FROM photara.libraries WHERE library_id='{uid(1)}'"),login=login,error='permission denied')
    for table, error in (('photara.libraries','hard_delete_not_supported'),('photara.people','hard_delete_not_supported'),('photara.project_catalog','hard_delete_not_supported'),('photara.project_locators','hard_delete_not_supported'),('photara_private.library_subscriptions','hard_delete_not_supported'),('photara.library_contract_state','d19_no_delete'),('photara_private.scoped_command_receipts','d19_immutable'),('photara_private.scoped_change_batches','d19_immutable'),('photara_private.scoped_changes','d19_immutable')):
        check('owner_delete_'+table,tx(f"DELETE FROM {table} WHERE library_id='{uid(1)}'",owner=True),error=error)
    check('owner_default_delete_denied',tx(f"DELETE FROM photara_identity.account_defaults WHERE account_id='{uid(10)}'",owner=True),error='immutable onboarding record')
    check('owner_default_reassignment_denied',tx(f"UPDATE photara_identity.account_defaults SET library_id='{uid(2)}',revision=revision+1 WHERE account_id='{uid(10)}'",owner=True),error='immutable onboarding record')
    check('last_active_library_owner_guard',tx(f"UPDATE photara_identity.memberships SET state='revoked',revoked_at=now(),revision=revision+1 WHERE membership_id='{uid(310)}'; SET CONSTRAINTS ALL IMMEDIATE",owner=True),error='library_requires_active_owner')
    check('stale_library_revision_guard',tx(f"UPDATE photara.libraries SET display_name='stale',revision=revision WHERE library_id='{uid(1)}'",owner=True),error='revision_or_identity_conflict')
    check('accepted_receipt_nulling_denied',tx(f"UPDATE photara_private.scoped_command_receipts SET accepted_stream_id=NULL WHERE library_id='{uid(1)}'",owner=True),error='d19_immutable')
    for first,second in (('scoped_command_receipts','scoped_change_batches'),('scoped_change_batches','scoped_command_receipts')):
        check('actual_cte_'+first,tx(f"WITH a AS (DELETE FROM photara_private.{first} WHERE library_id='{uid(1)}' RETURNING library_id), b AS (DELETE FROM photara_private.{second} WHERE library_id='{uid(1)}' AND (SELECT count(*) FROM a)>=0 RETURNING library_id) SELECT count(*) FROM b",owner=True),error='d19_immutable')
    for first,second in (('storage_slots','storage_slot_names'),('storage_slot_names','storage_slots'),('library_variables','library_variable_names'),('library_variable_names','library_variables')):
        check('actual_cte_'+first,tx(f"WITH a AS (DELETE FROM photara.{first} WHERE library_id='{uid(1)}' RETURNING library_id), b AS (DELETE FROM photara.{second} WHERE library_id='{uid(1)}' AND (SELECT count(*) FROM a)>=0 RETURNING library_id) SELECT count(*) FROM b",owner=True),error='d19_')
    check('ordinary_update_rollback',tx(f"UPDATE photara.libraries SET display_name='temporary',revision=revision+1 WHERE library_id='{uid(1)}'",owner=True))
    return tests,initial


def concurrent_lock(pg,initial):
    process = subprocess.Popen(login_command(pg,'ll1_fixture'),stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,env=pg.env)
    try:
        process.stdin.write(f"BEGIN; SET LOCAL ROLE photara_owner; {ctx()} DO $$ BEGIN PERFORM 1 FROM photara.libraries WHERE library_id='{uid(1)}' FOR UPDATE; END $$;\n\\echo LOCKED\n")
        process.stdin.flush()
        # psql emits one row before the marker; bounded pipe wait, no timed sleeps.
        while True:
            assert select.select([process.stdout],[],[],10)[0], 'lock holder did not become ready'
            line=process.stdout.readline().strip()
            if line=='LOCKED':
                break
        require(pg,f"BEGIN; SET LOCAL ROLE photara_owner; {ctx()} SET LOCAL lock_timeout='100ms'; UPDATE photara.libraries SET display_name='blocked',revision=revision+1 WHERE library_id='{uid(1)}';",error='lock timeout')
    finally:
        if process.poll() is None:
            process.stdin.write('ROLLBACK;\n\\q\n')
            process.stdin.flush()
        process.communicate(timeout=10)
    assert snapshot(pg)==initial
    print(json.dumps({'case':'concurrent_root_lock_rollback','result':'pass'}),flush=True)


def main():
    argparse.ArgumentParser(description=__doc__).parse_args()
    if not __debug__:
        raise RuntimeError('Assertions required')
    with tempfile.TemporaryDirectory(prefix='photara-ll1-constraints-',dir='/private/tmp') as root:
        pg=Postgres(root)
        pg.env={k:v for k,v in pg.env.items() if k not in ('DATABASE_URL','PHOTARA_TEST_MIGRATOR_URL')}
        try:
            hashes=install(pg)
            measured=measure(pg)
            print(json.dumps({'catalog':measured}),flush=True)
            seed(pg)
            tests,initial=test_cases(pg)
            concurrent_lock(pg,initial)
            assert hashes=={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in (ROOT/'crates/photara-service/migrations/postgres').glob('*.sql')}
            missing=pg.scalar("SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname IN ('photara','photara_private') AND c.relname IN ('lifecycle_receipts','removed_library_identities','account_inventory_events','retirement_work')")
            assert missing=='0'
            print(json.dumps({'stage':'partial actual PostgreSQL baseline only','passed':len(tests)+1,'catalog':measured,'new_retirement_relations':0,'root':root}),flush=True)
        finally:
            pg.close()
    print(json.dumps({'temporary_cluster':'stopped and removed','root':root}),flush=True)


if __name__=='__main__':
    main()
