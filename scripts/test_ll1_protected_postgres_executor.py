#!/usr/bin/env python3
"""Disposable PostgreSQL executor experiment; no deployable migration or live URL.

The fixture uses the actual schema and a test-only protected guard overlay.
Authentication transport, canonical review tokens and durable inventory are not
implemented by this experiment. See the companion evidence report.
"""
import hashlib
import json
import re
import select
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
import test_ll1_actual_postgres_baseline as base
from audit_ll1_schema_inventory import inventory, dispositions


def eligible_seed(pg):
    original = base.require
    def capture(pg, sql, **kwargs):
        sql = re.sub(r"INSERT INTO photara_identity.account_defaults[^;]+;", "", sql)
        sql = sql.replace("'test','active',1,now()", "'test','cancelled',1,now()")
        return original(pg, sql, **kwargs)
    base.require = capture
    try:
        base.seed(pg)
    finally:
        base.require = original
    base.require(pg, f"""BEGIN; SET LOCAL ROLE photara_owner; {base.ctx(2,10)}
INSERT INTO photara_identity.memberships VALUES('{base.uid(321)}','{base.uid(2)}','{base.uid(10)}','owner','active',1,now(),now(),NULL);
INSERT INTO photara_identity.account_defaults VALUES('{base.uid(10)}','{base.uid(2)}',1,now(),now()); COMMIT;""")


def overlay(pg):
    source = inventory('postgres')
    owned = [t for t, d in dispositions('postgres').items() if d.startswith(('D ', 'T ', 'B active', 'B default'))]
    assert len(owned) == 48, owned
    predicates = {}
    for table in owned:
        if 'library_id' in source['tables'][table]['columns']:
            predicates[table] = 't.library_id=p_library'
        elif table.endswith('project_invitation_secrets'):
            predicates[table] = 'EXISTS (SELECT 1 FROM photara_identity.project_invitations i WHERE i.invitation_id=t.invitation_id AND i.library_id=p_library)'
        elif table.endswith('scoped_sync_clients'):
            predicates[table] = 'EXISTS (SELECT 1 FROM photara_private.scoped_streams s WHERE s.stream_id=t.stream_id AND s.epoch=t.epoch AND s.library_id=p_library)'
        elif table.endswith('library_claim_receipts'):
            predicates[table] = 't.claimed_library_id=p_library'
        else:
            raise AssertionError('Unclassified ownership: '+table)
    base.require(pg, """SET ROLE photara_owner;
CREATE SCHEMA ll1_probe;
REVOKE ALL ON SCHEMA ll1_probe FROM PUBLIC;
GRANT USAGE ON SCHEMA ll1_probe TO photara_api;
CREATE TABLE ll1_probe.permit(tx bigint,relation text,row_value jsonb);
CREATE TABLE ll1_probe.receipts(actor uuid,operation uuid,library uuid,revision bigint,result text,PRIMARY KEY(actor,operation));
CREATE TABLE ll1_probe.markers(library uuid PRIMARY KEY,actor uuid,operation uuid,FOREIGN KEY(actor,operation) REFERENCES ll1_probe.receipts);
CREATE TABLE ll1_probe.evidence(actor uuid,operation uuid,relation text,row_value jsonb);
CREATE TABLE ll1_probe.inventory(actor uuid,operation uuid,library uuid,PRIMARY KEY(actor,operation));
REVOKE ALL ON ALL TABLES IN SCHEMA ll1_probe FROM PUBLIC,photara_api,photara_control,photara_auth_read;
CREATE FUNCTION ll1_probe.permitted(relation text,row_value jsonb) RETURNS boolean
LANGUAGE sql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
 SELECT EXISTS(SELECT 1 FROM ll1_probe.permit p WHERE p.tx=txid_current() AND p.relation=$1 AND p.row_value=$2)
$$;
REVOKE ALL ON FUNCTION ll1_probe.permitted(text,jsonb) FROM PUBLIC;
""")
    # Original function bodies and their ACL/security settings survive. Only a
    # DELETE of a byte-for-byte reviewed row may return early. No GUC switch.
    for name in ('guard_revision','append_only','d19_immutable','d19_mutable'):
        definition = pg.scalar(f"SELECT pg_get_functiondef('photara_private.{name}()'::regprocedure)")
        insertion = "\n IF TG_OP='DELETE' AND ll1_probe.permitted(TG_TABLE_SCHEMA||'.'||TG_TABLE_NAME,to_jsonb(OLD)) THEN RETURN OLD; END IF;\n"
        definition = definition.replace('BEGIN', 'BEGIN'+insertion, 1)
        base.require(pg, 'SET ROLE photara_owner;'+definition)
    # Exact permit rows are captured under the root lock. The explicit allowlist
    # comes from the reviewed disposition matrix, not unrestricted FK traversal.
    prepare = '\n'.join(f"INSERT INTO ll1_probe.permit SELECT txid_current(),'{t}',to_jsonb(t) FROM {t} t WHERE {predicates[t]};" for t in owned)
    # Root last, dependency chained; all deletes remain one PostgreSQL statement.
    owned.remove('photara.libraries')
    owned.append('photara.libraries')
    ctes = []
    for i, table in enumerate(owned):
        dependency = f' AND (SELECT count(*) FROM d{i-1})>=0' if i else ''
        ctes.append(f"d{i} AS (DELETE FROM {table} t WHERE EXISTS(SELECT 1 FROM ll1_probe.permit p WHERE p.tx=txid_current() AND p.relation='{table}' AND p.row_value=to_jsonb(t)){dependency} RETURNING 1)")
    deletion = 'WITH '+','.join(ctes)+f' SELECT count(*) INTO removed FROM d{len(ctes)-1};'
    base.require(pg, f"""SET ROLE photara_owner;
CREATE FUNCTION ll1_probe.remove(p_library uuid,p_revision bigint,p_operation uuid) RETURNS text
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
DECLARE v_actor uuid:=nullif(current_setting('photara.account_id',true),'')::uuid; previous ll1_probe.receipts; removed bigint;
BEGIN
 PERFORM photara_private.d19_lock_actor();
 PERFORM 1 FROM photara_identity.accounts WHERE account_id=v_actor FOR UPDATE;
 SELECT * INTO previous FROM ll1_probe.receipts r WHERE r.actor=v_actor AND r.operation=p_operation;
 IF FOUND THEN
  IF previous.library<>p_library OR previous.revision<>p_revision THEN RAISE EXCEPTION 'operation_conflict'; END IF;
  RETURN previous.result;
 END IF;
 PERFORM photara_private.authorize_library(p_library,'manage-access');
 IF NOT EXISTS(SELECT 1 FROM photara_identity.memberships WHERE library_id=p_library AND account_id=v_actor AND role='owner' AND state='active') THEN RAISE EXCEPTION 'owner_required'; END IF;
 IF EXISTS(SELECT 1 FROM photara_identity.account_defaults WHERE library_id=p_library) THEN RAISE EXCEPTION 'default_protected'; END IF;
 IF EXISTS(SELECT 1 FROM photara_private.library_subscriptions WHERE library_id=p_library AND state<>'cancelled') THEN RAISE EXCEPTION 'billing_protected'; END IF;
 IF NOT EXISTS(SELECT 1 FROM photara.libraries WHERE library_id=p_library AND revision=p_revision AND state='active') THEN
  INSERT INTO ll1_probe.receipts VALUES(v_actor,p_operation,p_library,p_revision,'stale'); RETURN 'stale';
 END IF;
 {prepare}
 INSERT INTO ll1_probe.evidence SELECT v_actor,p_operation,relation,row_value FROM ll1_probe.permit WHERE tx=txid_current() AND relation IN ('photara_private.scoped_command_receipts','photara_private.library_subscriptions');
 {deletion}
 IF removed<>1 THEN RAISE EXCEPTION 'closure_root_missing'; END IF;
 DELETE FROM ll1_probe.permit WHERE tx=txid_current();
 INSERT INTO ll1_probe.receipts VALUES(v_actor,p_operation,p_library,p_revision,'removed');
 INSERT INTO ll1_probe.markers VALUES(p_library,v_actor,p_operation);
 INSERT INTO ll1_probe.inventory VALUES(v_actor,p_operation,p_library);
 RETURN 'removed';
END $$;
REVOKE ALL ON FUNCTION ll1_probe.remove(uuid,bigint,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ll1_probe.remove(uuid,bigint,uuid) TO photara_api;
CREATE TRIGGER immutable BEFORE UPDATE OR DELETE ON ll1_probe.receipts FOR EACH ROW EXECUTE FUNCTION photara_private.append_only();
CREATE TRIGGER immutable BEFORE UPDATE OR DELETE ON ll1_probe.markers FOR EACH ROW EXECUTE FUNCTION photara_private.append_only();
CREATE TRIGGER immutable BEFORE UPDATE OR DELETE ON ll1_probe.evidence FOR EACH ROW EXECUTE FUNCTION photara_private.append_only();
CREATE FUNCTION ll1_probe.terminal_closure() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM ll1_probe.markers m JOIN ll1_probe.receipts r USING(actor,operation)
 JOIN ll1_probe.inventory i ON i.actor=m.actor AND i.operation=m.operation AND i.library=m.library
 WHERE m.library=OLD.library_id AND r.library=OLD.library_id AND r.result='removed')
 OR EXISTS(SELECT 1 FROM ll1_probe.permit WHERE tx=txid_current())
 THEN RAISE EXCEPTION 'terminal_closure'; END IF;
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION ll1_probe.terminal_closure() FROM PUBLIC;
CREATE CONSTRAINT TRIGGER ll1_terminal_closure AFTER DELETE ON photara.libraries
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ll1_probe.terminal_closure();
""")
    return owned


def main():
    if not __debug__:
        raise RuntimeError('Assertions required')
    with tempfile.TemporaryDirectory(prefix='photara-ll1-constraints-',dir='/private/tmp') as root:
        pg=base.Postgres(root)
        try:
            hashes=base.install(pg)
            measured=base.measure(pg)
            eligible_seed(pg)
            before=base.snapshot(pg)
            owned=overlay(pg)
            cases=[]
            def check(name,sql,login='ll1_api',error=None,value=None):
                result=base.execute(pg,sql,login)
                if error:
                    first_error=next((line for line in result.stderr.splitlines() if line.startswith('ERROR:')), '')
                    assert result.returncode and error in first_error,result.stderr
                else:
                    assert result.returncode==0,result.stderr
                    if value is not None:
                        assert result.stdout.strip()==value,result.stdout
                cases.append(name)
                print(json.dumps({'case':name,'result':'pass'}),flush=True)
            def call(revision=1,operation=9000,actor=10,library=1,end='COMMIT'):
                return f"BEGIN;{base.ctx(library,actor)} SELECT ll1_probe.remove('{base.uid(library)}',{revision},'{base.uid(operation)}');{end};"
            check('viewer_denied',call(actor=30),error='not_found_or_forbidden')
            check('cross_library_denied',call(actor=20),error='not_found_or_forbidden')
            check('identity_account_mismatch_denied',call().replace(base.uid(110),base.uid(120)),error='not_found_or_forbidden')
            check('control_execute_denied',call(),login='ll1_control',error='permission denied')
            check('permit_write_denied','INSERT INTO ll1_probe.permit VALUES(1,\'x\',\'{}\');',error='permission denied')
            check('ordinary_delete_guard',f"BEGIN;SET LOCAL ROLE photara_owner;{base.ctx()} DELETE FROM photara.storage_slots WHERE library_id='{base.uid(1)}';",login='ll1_fixture',error='d19_no_delete')
            check('default_refused',call(library=2),error='default_protected')
            check('stale_terminal',call(revision=9,operation=9001),value='stale')
            check('stale_replay',call(revision=9,operation=9001),value='stale')
            check('operation_conflict',call(revision=1,operation=9001),error='operation_conflict')
            check('stale_permit_refused',f"BEGIN;SET LOCAL ROLE photara_owner;{base.ctx()} INSERT INTO ll1_probe.permit SELECT txid_current()-1,'photara.storage_slots',to_jsonb(t) FROM photara.storage_slots t; DELETE FROM photara.storage_slots;",login='ll1_fixture',error='d19_no_delete')
            # Negative security evidence: a SQL-login holder can assert the full
            # owner context. Runtime GUCs are not an authenticated capability.
            check('sql_login_can_assert_owner_context_rollback',call(end='ROLLBACK'),value='removed')
            assert base.snapshot(pg)==before
            for table in ('photara.storage_slot_names','photara.library_variable_names','photara_private.scoped_command_receipts'):
                base.require(pg,f"SET ROLE photara_owner; CREATE FUNCTION ll1_probe.omit_row() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.relation='{table}' THEN RETURN NULL; END IF; RETURN NEW; END $$; CREATE TRIGGER omit_row BEFORE INSERT ON ll1_probe.permit FOR EACH ROW EXECUTE FUNCTION ll1_probe.omit_row();")
                check('incomplete_cycle_'+table,call(),error='foreign key')
                assert base.snapshot(pg)==before
                base.require(pg,"SET ROLE photara_owner; DROP TRIGGER omit_row ON ll1_probe.permit; DROP FUNCTION ll1_probe.omit_row();")
            # Fault-inject missing and mismatched terminal publication. Constraint
            # closure rejects commit after the real aggregate delete has executed.
            for relation in ('markers','inventory','receipts'):
                base.require(pg,f"SET ROLE photara_owner; CREATE FUNCTION ll1_probe.suppress() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RETURN NULL; END $$; CREATE TRIGGER suppress BEFORE INSERT ON ll1_probe.{relation} FOR EACH ROW EXECUTE FUNCTION ll1_probe.suppress();")
                check('omitted_'+relation+'_rollback',call(),error='foreign key' if relation=='receipts' else 'terminal_closure')
                assert base.snapshot(pg)==before
                base.require(pg,f"SET ROLE photara_owner; DROP TRIGGER suppress ON ll1_probe.{relation}; DROP FUNCTION ll1_probe.suppress();")
            base.require(pg,"SET ROLE photara_owner; CREATE FUNCTION ll1_probe.mismatch() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.library='00000000-0000-0000-0000-000000000002'; RETURN NEW; END $$; CREATE TRIGGER mismatch BEFORE INSERT ON ll1_probe.inventory FOR EACH ROW EXECUTE FUNCTION ll1_probe.mismatch();")
            check('mismatched_inventory_rollback',call(),error='terminal_closure')
            assert base.snapshot(pg)==before
            base.require(pg,"SET ROLE photara_owner; DROP TRIGGER mismatch ON ll1_probe.inventory; DROP FUNCTION ll1_probe.mismatch();")
            # First delete is held uncommitted. The identical and new operation
            # contenders must wait rather than publish a competing outcome.
            process=subprocess.Popen(base.login_command(pg,'ll1_api'),stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,env=pg.env)
            try:
                process.stdin.write(call(end='')[:-1]+"\n\\echo READY\n")
                process.stdin.flush()
                assert select.select([process.stdout],[],[],10)[0]
                assert process.stdout.readline().strip()=='removed'
                assert process.stdout.readline().strip()=='READY'
                for operation in (9000,9002):
                    check('concurrent_delete_wait_'+str(operation),call(operation=operation).replace('BEGIN;','BEGIN;SET LOCAL lock_timeout=\'100ms\';'),error='lock timeout')
                process.stdin.write('COMMIT;\n\\q\n'); process.stdin.flush()
                out,err=process.communicate(timeout=10)
                assert process.returncode==0,err
                cases.append('full_execution_commit')
            finally:
                if process.poll() is None:
                    process.kill();process.communicate()
            check('terminal_replay_without_membership',call(),value='removed')
            check('terminal_immutable',"SET ROLE photara_owner; DELETE FROM ll1_probe.receipts;",login='ll1_fixture',error='append_only_record')
            after=json.loads(base.snapshot(pg)); original=json.loads(before)
            for table, rows in original.items():
                expected=[r for r in rows if not(table in owned and r.get('library_id')==base.uid(1))]
                assert after[table]==expected,table
            assert pg.scalar('SELECT count(*) FROM ll1_probe.permit')=='0'
            assert pg.scalar('SELECT count(*) FROM ll1_probe.markers')=='1'
            assert pg.scalar('SELECT count(*) FROM ll1_probe.evidence')=='2'
            for name,digest in hashes.items():
                assert hashlib.sha256((base.ROOT/'crates/photara-service/migrations/postgres'/name).read_bytes()).hexdigest()==digest
            print(json.dumps({'catalog':measured,'cases':len(cases),'owned_table_allowlist':len(owned),'status':'partial_executor_proof','limitations':['sparse rows across actual full schema','trusted runtime actor context','no review token/session/generation/inventory implementation','no last-library concurrency proof','no production migration']}))
        finally:
            pg.close()


if __name__=='__main__':
    main()
