#!/usr/bin/env python3
"""Disposable signed HTTP -> SQL authority proof; never accepts a database URL."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
import test_ll1_fresh_device_receipt as receipt
from test_ll1_fresh_device_receipt import authority, base, executor

NOW = 1800000000000


def authority_privileges(pg):
    rows = json.loads(pg.scalar("""SELECT jsonb_agg(jsonb_build_object(
 'role',r.name,'function',p.proname,'execute',has_function_privilege(r.name,p.oid,'EXECUTE'))
 ORDER BY r.name,p.proname)
 FROM (VALUES('ll1_api'),('ll1_control'),('ll1_auth'),('ll1_service'),('ll1_authority')) r(name)
 CROSS JOIN pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace
 WHERE n.nspname='ll1_probe' AND p.proname IN
 ('http_command','authorize','execute','query_receipt','execute_review')"""))
    assert len(rows) == 25
    for row in rows:
        assert row['execute'] == (row['role'] in ('ll1_service', 'll1_authority')
                                  and row['function'] == 'http_command'), row
    return rows


def commit_guard_state(pg, installed):
    present = pg.scalar("SELECT to_regprocedure('ll1_probe.no_committed_work()') IS NOT NULL") == 't'
    assert present == installed
    if not installed:
        return {'installed': False}
    function = json.loads(pg.scalar("""SELECT jsonb_build_object(
 'owner',pg_get_userbyid(p.proowner),'definer',p.prosecdef,'config',p.proconfig)
 FROM pg_proc p WHERE p.oid='ll1_probe.no_committed_work()'::regprocedure"""))
    assert function == {'owner':'photara_owner','definer':True,
                        'config':['search_path=pg_catalog, pg_temp']}
    triggers = json.loads(pg.scalar("""SELECT jsonb_agg(jsonb_build_object(
 'name',t.tgname,'schema',n.nspname,'table',c.relname,'enabled',t.tgenabled,
 'deferrable',t.tgdeferrable,'initially_deferred',t.tginitdeferred)
 ORDER BY t.tgname) FROM pg_trigger t JOIN pg_class c ON c.oid=t.tgrelid
 JOIN pg_namespace n ON n.oid=c.relnamespace
 WHERE t.tgfoid='ll1_probe.no_committed_work()'::regprocedure"""))
    expected = [{'name':name,'schema':'ll1_probe','table':table,'enabled':'O',
                 'deferrable':True,'initially_deferred':True}
                for name,table in [('authorization_no_commit','authorization'),
                                   ('permit_no_commit','permit')]]
    assert triggers == expected, triggers
    assert pg.scalar("""SELECT bool_and(NOT has_function_privilege(r.name,
 'll1_probe.no_committed_work()','EXECUTE'))
 FROM (VALUES('ll1_api'),('ll1_control'),('ll1_auth'),('ll1_service'),('ll1_authority')) r(name)""") == 't'
    assert pg.scalar("""SELECT NOT EXISTS(SELECT 1 FROM pg_proc p,
 LATERAL aclexplode(coalesce(p.proacl,acldefault('f',p.proowner))) a
 WHERE p.oid='ll1_probe.no_committed_work()'::regprocedure
 AND a.grantee=0 AND a.privilege_type='EXECUTE')""") == 't'
    return {'installed':True,'function':function,'triggers':triggers,
            'public_and_runtime_execute_denied':True}


def overlay(pg, owned):
    base.require(pg, f"""SET ROLE photara_owner;
CREATE FUNCTION ll1_probe.http_deadline() RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
DECLARE deadline bigint:=nullif(current_setting('ll1_http.expires',true),'')::bigint;
 review_deadline bigint:=nullif(current_setting('ll1_http.review_expires',true),'')::bigint;
 instant bigint;
BEGIN
 SELECT (extract(epoch FROM review_clock.instant)*1000)::bigint INTO STRICT instant FROM ll1_probe.review_clock;
 IF deadline IS NULL OR deadline<=instant OR (review_deadline IS NOT NULL AND review_deadline<=instant)
 THEN RAISE EXCEPTION 'http_deadline'; END IF;
END $$;
CREATE FUNCTION ll1_probe.http_boundary() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
 IF TG_TABLE_SCHEMA='ll1_probe' AND TG_TABLE_NAME='authorization' THEN
  -- Test-only late wait, after authorize's locks and before grant insertion.
  PERFORM pg_advisory_xact_lock(741852963);
 END IF;
 PERFORM ll1_probe.http_deadline();
 IF TG_OP='DELETE' THEN RETURN OLD; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER http_boundary BEFORE INSERT ON ll1_probe.authorization
 FOR EACH ROW EXECUTE FUNCTION ll1_probe.http_boundary();
CREATE FUNCTION ll1_probe.http_command(p_issuer text,p_subject text,p_expires bigint,
 p_device uuid,p_secret bytea,p_credential_revision bigint,p_operation uuid,p_digest bytea,p_execute boolean)
RETURNS bytea LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
DECLARE v_identity uuid; v_actor uuid; response bytea; reviewed ll1_probe.device_review;
BEGIN
 -- No caller-selected account/identity; only HTTP-verified issuer/subject.
 SELECT i.identity_id,i.account_id INTO STRICT v_identity,v_actor
 FROM photara_identity.account_identities i JOIN photara_identity.accounts a USING(account_id)
 WHERE i.issuer=p_issuer AND i.subject=p_subject AND i.state='active' AND a.state='active';
 PERFORM set_config('ll1_http.expires',p_expires::text,true);
 PERFORM set_config('ll1_http.review_expires','',true);
 response:=ll1_probe.query_receipt(v_identity,v_actor,p_device,p_secret,p_credential_revision,p_operation,p_digest);
 PERFORM ll1_probe.http_deadline(); -- Includes any device/credential lock wait.
 IF response IS NULL AND p_execute THEN
  SELECT * INTO reviewed FROM ll1_probe.device_review WHERE actor=v_actor AND operation=p_operation;
  IF NOT FOUND OR reviewed.identity<>v_identity OR reviewed.device<>p_device
  THEN RAISE EXCEPTION 'original_review_device_required'; END IF;
  PERFORM set_config('ll1_http.review_expires',(extract(epoch FROM reviewed.expires)*1000)::bigint::text,true);
  PERFORM ll1_probe.http_deadline();
  response:=ll1_probe.execute_review(v_identity,v_actor,p_device,p_secret,p_credential_revision,p_operation,p_digest);
 END IF;
 PERFORM ll1_probe.http_deadline(); -- Additional terminal check, not the deletion guard.
 RETURN response;
END $$;
REVOKE ALL ON FUNCTION ll1_probe.http_deadline(),ll1_probe.http_boundary(),
 ll1_probe.http_command(text,text,bigint,uuid,bytea,bigint,uuid,bytea,boolean) FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION ll1_probe.query_receipt(uuid,uuid,uuid,bytea,bigint,uuid,bytea),
 ll1_probe.execute_review(uuid,uuid,uuid,bytea,bigint,uuid,bytea) FROM ll1_authority;
GRANT EXECUTE ON FUNCTION ll1_probe.http_command(text,text,bigint,uuid,bytea,bigint,uuid,bytea,boolean) TO ll1_authority;
""")
    # Every actual row deletion is checked after its row lock, including any
    # waits after the grant gate. These are candidate-only overlay triggers.
    for table in owned:
        base.require(pg, f"SET ROLE photara_owner; CREATE TRIGGER ll1_http_deadline BEFORE DELETE ON {table} FOR EACH ROW EXECUTE FUNCTION ll1_probe.http_boundary();")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--commit-guard', action='store_true',
                        help='compose the disposable deferred no-committed-work guard')
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument('--lifecycle', action='store_true',
                      help='run private cancellation/connection-loss/lost-response cases')
    mode.add_argument('--commit-relay', action='store_true',
                      help='run bounded private Unix-socket COMMIT acknowledgement cases')
    args = parser.parse_args()
    if not __debug__:
        raise RuntimeError('Assertions required')
    with tempfile.TemporaryDirectory(prefix='photara-ll1-constraints-', dir='/private/tmp') as root:
        pg = base.Postgres(root)
        relay = None
        try:
            hashes = base.install(pg)
            measured = base.measure(pg)
            executor.eligible_seed(pg)
            owned = executor.overlay(pg)
            authority.boundary(pg)
            receipt.overlay(pg, now_ms=NOW)
            overlay(pg, owned)
            import test_ll1_authority_commit_guard as guard
            prior_catalog = guard.catalog(pg)
            prior_privileges = authority_privileges(pg)
            commit_guard_state(pg, False)
            if args.commit_guard:
                guard.overlay(pg)
            selected_guard = commit_guard_state(pg, args.commit_guard)
            assert guard.catalog(pg) == prior_catalog
            assert authority_privileges(pg) == prior_privileges
            env = {k: v for k, v in os.environ.items()
                   if not k.startswith(('PG', 'PHOTARA_TEST_', 'PHOTARA_LL1_')) and k != 'DATABASE_URL'}
            env['CARGO_NET_OFFLINE'] = 'true'
            env['PHOTARA_LL1_ADAPTER_SOCKET'] = str(pg.socket)
            if args.commit_relay:
                from ll1_commit_relay import CommitRelay, verify_absolute_read_deadline
                verify_absolute_read_deadline()
                relay = CommitRelay(pg.root, pg.socket)
                env['PHOTARA_LL1_RELAY_SOCKET'] = str(relay.socket)
                test = 'postgres_ll1_signed_sql_commit_relay'
            else:
                test = ('postgres_ll1_signed_sql_lifecycle' if args.lifecycle
                        else 'postgres_ll1_signed_sql_adapter')
            command = ['cargo','test','--offline','-p','photara-service','--lib',
                       f'http::ll1_sql_tests::{test}','--',
                       '--ignored','--exact','--nocapture','--test-threads=1']
            result = subprocess.run(command, cwd=base.ROOT, env=env, text=True,
                                    stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                    timeout=180 if args.commit_relay else None)
            print(result.stdout, end='')
            if result.returncode:
                raise RuntimeError(f'disposable adapter test failed: {result.returncode}')
            if relay:
                print(json.dumps({'private_commit_relay':relay.proof(),
                                  'absolute_read_deadline_trickle_check':True}, sort_keys=True))
            assert commit_guard_state(pg, args.commit_guard) == selected_guard
            assert guard.catalog(pg) == prior_catalog
            assert authority_privileges(pg) == prior_privileges
            work = json.loads(pg.scalar("""SELECT jsonb_build_object(
 'authorization',(SELECT count(*) FROM ll1_probe.authorization),
 'permit',(SELECT count(*) FROM ll1_probe.permit),
 'original_receipt',(SELECT count(*) FROM ll1_probe.original_receipt))"""))
            assert work == {'authorization':0,'permit':0,'original_receipt':1}, work
            print(json.dumps({'commit_guard':selected_guard,'terminal_rows':work,
                              'baseline_catalog_unchanged_after_inherited_overlays':True,
                              'raw_entry_denials_preserved':True}, sort_keys=True))
            for name, digest in hashes.items():
                import hashlib
                assert hashlib.sha256((base.ROOT/'crates/photara-service/migrations/postgres'/name).read_bytes()).hexdigest() == digest
            print(json.dumps({'result':'pass','baseline':measured,'production_changes':False,
                              'scope':'signed HTTP and protected SQL, private fixture only',
                              'lifecycle':args.lifecycle,
                              'commit_relay':args.commit_relay,
                              'commit_guard':args.commit_guard}, sort_keys=True))
        finally:
            try:
                if relay:
                    relay.close()
            finally:
                pg.close()
    print(json.dumps({'temporary_cluster':'stopped and removed'}))


if __name__ == '__main__':
    main()
