#!/usr/bin/env python3
"""Disposable authority-role boundary proof, not production authentication."""
import hashlib
import json
import sys
import tempfile

sys.dont_write_bytecode = True
import test_ll1_protected_postgres_executor as executor
from test_ll1_protected_postgres_executor import base


def boundary(pg):
    base.require(pg, """
CREATE ROLE ll1_authority NOLOGIN NOSUPERUSER NOBYPASSRLS;
CREATE ROLE ll1_service LOGIN NOSUPERUSER NOBYPASSRLS;
GRANT ll1_authority TO ll1_service;
SET ROLE photara_owner;
REVOKE EXECUTE ON FUNCTION ll1_probe.remove(uuid,bigint,uuid) FROM photara_api;
GRANT USAGE ON SCHEMA ll1_probe TO ll1_authority;
CREATE TABLE ll1_probe.authorization(
 tx bigint, backend integer, operation uuid, identity uuid, actor uuid,
 library uuid, revision bigint, digest bytea, expires timestamptz,
 PRIMARY KEY(tx,backend,operation));
REVOKE ALL ON ll1_probe.authorization FROM PUBLIC,photara_api,photara_control,photara_auth_read,ll1_authority;
CREATE TABLE ll1_probe.request_binding(actor uuid,operation uuid,identity uuid,library uuid,revision bigint,digest bytea,PRIMARY KEY(actor,operation));
REVOKE ALL ON ll1_probe.request_binding FROM PUBLIC,photara_api,photara_control,photara_auth_read,ll1_authority;
CREATE TRIGGER immutable BEFORE UPDATE OR DELETE ON ll1_probe.request_binding FOR EACH ROW EXECUTE FUNCTION photara_private.append_only();
CREATE FUNCTION ll1_probe.context(p_identity uuid,p_actor uuid,p_library uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
 PERFORM set_config('photara.identity_id',p_identity::text,true);
 PERFORM set_config('photara.account_id',p_actor::text,true);
 PERFORM set_config('photara.library_id',p_library::text,true);
 PERFORM set_config('photara.scope_kind','library',true);
 PERFORM photara_private.d19_lock_actor();
 IF NOT EXISTS(SELECT 1 FROM photara_identity.account_identities i
 JOIN photara_identity.accounts a USING(account_id)
 WHERE i.identity_id=p_identity AND i.account_id=p_actor AND i.state='active' AND a.state='active')
 THEN RAISE EXCEPTION 'authenticated_identity_mismatch'; END IF;
END $$;
CREATE FUNCTION ll1_probe.authorize(p_identity uuid,p_actor uuid,p_library uuid,p_revision bigint,p_operation uuid,p_digest bytea) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
DECLARE previous ll1_probe.receipts;
BEGIN
 PERFORM ll1_probe.context(p_identity,p_actor,p_library);
 INSERT INTO ll1_probe.request_binding VALUES(p_actor,p_operation,p_identity,p_library,p_revision,p_digest) ON CONFLICT DO NOTHING;
 IF NOT EXISTS(SELECT 1 FROM ll1_probe.request_binding WHERE actor=p_actor AND operation=p_operation AND identity=p_identity AND library=p_library AND revision=p_revision AND digest=p_digest)
 THEN RAISE EXCEPTION 'operation_conflict'; END IF;
 SELECT * INTO previous FROM ll1_probe.receipts WHERE actor=p_actor AND operation=p_operation;
 IF FOUND THEN
  IF previous.library<>p_library OR previous.revision<>p_revision THEN RAISE EXCEPTION 'operation_conflict'; END IF;
 ELSE
  PERFORM photara_private.authorize_library(p_library,'manage-access');
  IF NOT EXISTS(SELECT 1 FROM photara_identity.memberships WHERE library_id=p_library AND account_id=p_actor AND role='owner' AND state='active')
  THEN RAISE EXCEPTION 'owner_required'; END IF;
 END IF;
 INSERT INTO ll1_probe.authorization VALUES(txid_current(),pg_backend_pid(),p_operation,p_identity,p_actor,p_library,p_revision,p_digest,clock_timestamp()+interval '30 seconds');
END $$;
CREATE FUNCTION ll1_probe.execute(p_identity uuid,p_actor uuid,p_library uuid,p_revision bigint,p_operation uuid,p_digest bytea) RETURNS text
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
DECLARE grant_row ll1_probe.authorization; result text;
BEGIN
 DELETE FROM ll1_probe.authorization WHERE tx=txid_current() AND backend=pg_backend_pid()
 AND operation=p_operation AND identity=p_identity AND actor=p_actor AND library=p_library
 AND revision=p_revision AND digest=p_digest AND expires>clock_timestamp() RETURNING * INTO grant_row;
 IF NOT FOUND THEN RAISE EXCEPTION 'authorization_required'; END IF;
 PERFORM ll1_probe.context(grant_row.identity,grant_row.actor,grant_row.library);
 result:=ll1_probe.remove(grant_row.library,grant_row.revision,grant_row.operation);
 RETURN result;
END $$;
REVOKE ALL ON FUNCTION ll1_probe.context(uuid,uuid,uuid) FROM PUBLIC;
REVOKE ALL ON FUNCTION ll1_probe.authorize(uuid,uuid,uuid,bigint,uuid,bytea) FROM PUBLIC;
REVOKE ALL ON FUNCTION ll1_probe.execute(uuid,uuid,uuid,bigint,uuid,bytea) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ll1_probe.authorize(uuid,uuid,uuid,bigint,uuid,bytea),ll1_probe.execute(uuid,uuid,uuid,bigint,uuid,bytea) TO ll1_authority;
""")


def main():
    if not __debug__:
        raise RuntimeError('Assertions required')
    with tempfile.TemporaryDirectory(prefix='photara-ll1-constraints-',dir='/private/tmp') as root:
        pg=base.Postgres(root)
        try:
            hashes=base.install(pg)
            measured=base.measure(pg)
            executor.eligible_seed(pg)
            before=base.snapshot(pg)
            owned=executor.overlay(pg)
            boundary(pg)
            cases=[]
            def args(identity=110,actor=10,library=1,revision=1,operation=9200,digest='request-one'):
                return f"'{base.uid(identity)}','{base.uid(actor)}','{base.uid(library)}',{revision},'{base.uid(operation)}',sha256('{digest}'::bytea)"
            def authorize(**kw):
                return f'SELECT ll1_probe.authorize({args(**kw)});'
            def execute(**kw):
                return f'SELECT ll1_probe.execute({args(**kw)});'
            def call(end='COMMIT',**kw):
                return 'BEGIN;'+authorize(**kw)+execute(**kw)+end+';'
            def check(name,sql,login='ll1_service',error=None,value=None,preserve=True):
                result=base.execute(pg,sql,login)
                if error:
                    first=next((s for s in result.stderr.splitlines() if s.startswith('ERROR:')),'')
                    assert result.returncode and error in first,result.stderr
                else:
                    assert result.returncode==0,result.stderr
                    if value is not None:
                        assert result.stdout.strip()==value,result.stdout
                if preserve:
                    assert base.snapshot(pg)==before,name
                cases.append(name)
                print(json.dumps({'case':name,'result':'pass'}),flush=True)
            for login in ('ll1_api','ll1_control','ll1_auth'):
                for label,sql in [('mint',authorize()),('execute',execute()),('internal',f"SELECT ll1_probe.remove('{base.uid(1)}',1,'{base.uid(9200)}');"),('grant_write','INSERT INTO ll1_probe.authorization DEFAULT VALUES;'),('permit_write',"INSERT INTO ll1_probe.permit VALUES(1,'x','{}');"),('role','SET ROLE ll1_authority;')]:
                    check(login+'_'+label+'_denied','BEGIN;'+base.ctx()+sql,login=login,error='permission denied')
            check('authority_cannot_invoke_internal',f"SELECT ll1_probe.remove('{base.uid(1)}',1,'{base.uid(9200)}');",error='permission denied')
            check('missing_authorization',execute(),error='authorization_required')
            check('identity_substitution',call(identity=120),error='not_found_or_forbidden')
            check('wrong_library_ownership',call(identity=120,actor=20),error='not_found_or_forbidden')
            check('viewer_identity',call(identity=130,actor=30),error='not_found_or_forbidden')
            for label,kw in [('identity',{'identity':120}),('actor',{'actor':20}),('library',{'library':2}),('revision',{'revision':2}),('operation',{'operation':9201}),('digest',{'digest':'changed'})]:
                check('authorization_'+label+'_substitution','BEGIN;'+authorize()+execute(**kw),error='authorization_required')
            check('guc_substitution_ignored','BEGIN;'+authorize()+base.ctx(2,20)+execute()+'ROLLBACK;',value='removed')
            check('single_use','BEGIN;'+authorize()+execute()+execute(),error='authorization_required')
            check('authorization_transaction_expired','BEGIN;'+authorize()+'COMMIT;BEGIN;'+execute(),error='authorization_required')
            check('authorization_backend_expired',execute(),error='authorization_required')
            base.require(pg,"SET ROLE photara_owner; CREATE FUNCTION ll1_probe.wrong_backend() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.backend=pg_backend_pid()+1; RETURN NEW; END $$; CREATE TRIGGER wrong_backend BEFORE INSERT ON ll1_probe.authorization FOR EACH ROW EXECUTE FUNCTION ll1_probe.wrong_backend();")
            check('backend_binding_independently_enforced',call(),error='authorization_required')
            base.require(pg,'SET ROLE photara_owner; DROP TRIGGER wrong_backend ON ll1_probe.authorization; DROP FUNCTION ll1_probe.wrong_backend();')
            # Administrator-only fault injection; authority cannot edit grants.
            check('authority_grant_update_denied','UPDATE ll1_probe.authorization SET expires=clock_timestamp();',error='permission denied')
            base.require(pg,"SET ROLE photara_owner; CREATE FUNCTION ll1_probe.expire() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.expires=clock_timestamp()-interval '1 second'; RETURN NEW; END $$; CREATE TRIGGER expire BEFORE INSERT ON ll1_probe.authorization FOR EACH ROW EXECUTE FUNCTION ll1_probe.expire();")
            check('expired_authorization',call(),error='authorization_required')
            base.require(pg,'SET ROLE photara_owner; DROP TRIGGER expire ON ll1_probe.authorization; DROP FUNCTION ll1_probe.expire();')
            check('stale_terminal',call(revision=9,operation=9202),value='stale')
            check('stale_terminal_replay',call(revision=9,operation=9202),value='stale')
            check('terminal_coordinate_conflict',call(operation=9202),error='operation_conflict')
            check('ordinary_delete_guard','BEGIN;SET LOCAL ROLE photara_owner;'+base.ctx()+f"DELETE FROM photara.storage_slots WHERE library_id='{base.uid(1)}';",login='ll1_fixture',error='d19_no_delete')
            base.require(pg,"SET ROLE photara_owner; CREATE FUNCTION ll1_probe.omit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RETURN NULL; END $$; CREATE TRIGGER omit BEFORE INSERT ON ll1_probe.inventory FOR EACH ROW EXECUTE FUNCTION ll1_probe.omit();")
            check('terminal_failure_rolls_back_authorized_delete',call(),error='terminal_closure')
            assert pg.scalar('SELECT count(*) FROM ll1_probe.permit')=='0'
            base.require(pg,'SET ROLE photara_owner; DROP TRIGGER omit ON ll1_probe.inventory; DROP FUNCTION ll1_probe.omit();')
            check('legitimate_authorized_commit',call(),value='removed',preserve=False)
            after=base.snapshot(pg)
            check('old_grant_replay_denied',execute(),error='authorization_required',preserve=False)
            check('terminal_retry_fresh_authority',call(),value='removed',preserve=False)
            check('terminal_changed_digest_denied',call(digest='changed'),error='operation_conflict',preserve=False)
            check('terminal_wrong_identity_denied',call(identity=120),error='not_found_or_forbidden',preserve=False)
            assert base.snapshot(pg)==after
            for table,rows in json.loads(before).items():
                expected=[r for r in rows if not(table in owned and r.get('library_id')==base.uid(1))]
                assert json.loads(after)[table]==expected,table
            assert pg.scalar('SELECT count(*) FROM ll1_probe.permit')=='0'
            assert pg.scalar('SELECT count(*) FROM ll1_probe.markers')=='1'
            # Baseline source/catalog FK semantics are unchanged by this overlay.
            assert pg.scalar("SELECT count(*) FROM pg_constraint c JOIN pg_namespace n ON n.oid=c.connamespace WHERE n.nspname IN "+base.SCHEMAS+" AND c.contype='f'")==str(measured['foreign_keys'])
            for name,digest in hashes.items():
                assert hashlib.sha256((base.ROOT/'crates/photara-service/migrations/postgres'/name).read_bytes()).hexdigest()==digest
            print(json.dumps({'cases':len(cases),'catalog':measured,'status':'authority_role_boundary_proven','limitations':['synthetic trusted-service identity input, no OIDC integration','sparse full-schema row coverage inherited','fixture grants and receipts are not production wire format']}))
        finally:
            pg.close()


if __name__=='__main__':
    main()
