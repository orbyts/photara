#!/usr/bin/env python3
"""Opt-in disposable no-committed-work overlay; no migration or live URL.

The historical raw-authority fixture remains unchanged. This separate fixture
rejects a commit that would retain transaction authorization or permit rows.
"""
import argparse
import hashlib
import json
import os
import select
import subprocess
import sys
import tempfile
import time

sys.dont_write_bytecode = True
import test_ll1_protected_authorization as authority
from test_ll1_fresh_device_receipt import snapshot

base = authority.base
executor = authority.executor


def overlay(pg):
    base.require(pg, """SET ROLE photara_owner;
CREATE FUNCTION ll1_probe.no_committed_work() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
 IF TG_TABLE_NAME='authorization' THEN
  IF EXISTS(SELECT 1 FROM ll1_probe.authorization
   WHERE tx=NEW.tx AND backend=NEW.backend AND operation=NEW.operation)
  THEN RAISE EXCEPTION 'unconsumed_authorization_at_commit' USING ERRCODE='23514'; END IF;
 ELSIF TG_TABLE_NAME='permit' THEN
  IF EXISTS(SELECT 1 FROM ll1_probe.permit WHERE tx IS NOT DISTINCT FROM NEW.tx)
  THEN RAISE EXCEPTION 'unconsumed_permit_at_commit' USING ERRCODE='23514'; END IF;
 ELSE RAISE EXCEPTION 'unknown_work_relation';
 END IF;
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION ll1_probe.no_committed_work() FROM PUBLIC,photara_api,
 photara_control,photara_auth_read,ll1_authority;
CREATE CONSTRAINT TRIGGER authorization_no_commit
 AFTER INSERT OR UPDATE ON ll1_probe.authorization DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION ll1_probe.no_committed_work();
CREATE CONSTRAINT TRIGGER permit_no_commit
 AFTER INSERT OR UPDATE ON ll1_probe.permit DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION ll1_probe.no_committed_work();
""")


def arguments(operation=9200, digest='request-one'):
    return (f"'{base.uid(110)}','{base.uid(10)}','{base.uid(1)}',1,"
            f"'{base.uid(operation)}',sha256('{digest}'::bytea)")


def mint(**kw):
    return f"SELECT ll1_probe.authorize({arguments(**kw)});"


def consume(**kw):
    return f"SELECT ll1_probe.execute({arguments(**kw)});"


def catalog(pg):
    # Compare the actual baseline definitions before/after this new overlay.
    # The inherited executor's candidate guard changes precede this snapshot.
    schemas = base.SCHEMAS
    return pg.scalar(f"""SELECT jsonb_build_object(
 'constraints',(SELECT jsonb_agg(jsonb_build_array(c.oid::text,c.conrelid::regclass::text,c.conname,pg_get_constraintdef(c.oid),c.confdeltype,c.condeferrable,c.condeferred,c.convalidated) ORDER BY c.oid)
 FROM pg_constraint c JOIN pg_namespace n ON n.oid=c.connamespace WHERE n.nspname IN {schemas}),
 'functions',(SELECT jsonb_agg(jsonb_build_array(p.oid::text,pg_get_functiondef(p.oid),p.proacl::text) ORDER BY p.oid)
 FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname IN {schemas}),
 'triggers',(SELECT jsonb_agg(jsonb_build_array(t.oid::text,pg_get_triggerdef(t.oid),t.tgenabled) ORDER BY t.oid)
 FROM pg_trigger t JOIN pg_class c ON c.oid=t.tgrelid JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname IN {schemas}))""")


def process(pg, login):
    return subprocess.Popen(base.login_command(pg, login), stdin=subprocess.PIPE,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=pg.env)


def send(child, sql):
    child.stdin.write(sql.encode())
    child.stdin.flush()


def pid_line(child, prefix):
    pending = b''
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        ready, _, _ = select.select([child.stdout], [], [], max(0, deadline-time.monotonic()))
        if not ready:
            break
        chunk = os.read(child.stdout.fileno(), 4096)
        assert chunk, 'private backend exited before handshake'
        pending += chunk
        while b'\n' in pending:
            line, pending = pending.split(b'\n', 1)
            if line.startswith(prefix):
                return int(line[len(prefix):])
    raise AssertionError('private backend did not reach handshake')


def await_scalar(pg, sql, expected):
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        if pg.scalar(sql) == expected:
            return
    raise AssertionError('bounded backend/lock handshake failed: '+sql)


def cancel_after_mint(pg):
    before = snapshot(pg)
    blocker = process(pg, 'll1_fixture')
    request = process(pg, 'll1_service')
    try:
        send(blocker, "BEGIN; SELECT pg_advisory_xact_lock(741852965); SELECT 'blocker:'||pg_backend_pid();\n")
        blocker_pid = pid_line(blocker, b'blocker:')
        # The observer runs inside the original transaction; an administrator
        # cannot observe its uncommitted authorization row via MVCC.
        send(request, "\\set VERBOSITY verbose\nBEGIN;"+mint()+
             "SELECT ll1_probe.minted_observer(); SELECT 'minted:'||pg_backend_pid(); "
             "SELECT pg_advisory_xact_lock(741852965);\n")
        request_pid = pid_line(request, b'minted:')
        await_scalar(pg, f"SELECT {blocker_pid}=ANY(pg_blocking_pids({request_pid}))", 't')
        assert pg.scalar(f"SELECT usename='ll1_service' AND xact_start IS NOT NULL FROM pg_stat_activity WHERE pid={request_pid}") == 't'
        assert pg.scalar(f'SELECT pg_cancel_backend({request_pid})') == 't'
        _, stderr = request.communicate(timeout=5)
        assert request.returncode and b'57014' in stderr and b'canceling statement' in stderr, stderr
        await_scalar(pg, f"SELECT NOT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid={request_pid}) AND NOT EXISTS(SELECT 1 FROM pg_locks WHERE pid={request_pid})", 't')
        stdout, stderr = blocker.communicate(b'ROLLBACK;\n', timeout=5)
        assert blocker.returncode == 0, (stdout, stderr)
        assert snapshot(pg) == before, 'cancellation left baseline or overlay rows'
    finally:
        for child in (request, blocker):
            if child.poll() is None:
                child.kill()
                child.communicate(timeout=5)


def main():
    argparse.ArgumentParser(description=__doc__).parse_args()
    if not __debug__:
        raise RuntimeError('Assertions required')
    with tempfile.TemporaryDirectory(prefix='photara-ll1-constraints-', dir='/private/tmp') as root:
        pg = base.Postgres(root)
        try:
            hashes = base.install(pg)
            measured = base.measure(pg)
            executor.eligible_seed(pg)
            owned = executor.overlay(pg)
            authority.boundary(pg)
            original_catalog = catalog(pg)
            overlay(pg)
            assert catalog(pg) == original_catalog
            cases = []

            def passed(name):
                cases.append(name)
                print(json.dumps({'case': name, 'result': 'pass'}), flush=True)

            def check(name, sql, login='ll1_service', error=None, code=None, preserve=True):
                before = snapshot(pg)
                result = base.execute(pg, '\\set VERBOSITY verbose\n'+sql, login)
                if error:
                    assert result.returncode and error in result.stderr, result.stderr
                    if code:
                        assert code in result.stderr, result.stderr
                else:
                    assert result.returncode == 0, result.stderr
                if preserve:
                    assert snapshot(pg) == before, name
                passed(name)
                return result.stdout.strip()

            check('mint_only_commit_rejected', 'BEGIN;'+mint()+'COMMIT;',
                  error='unconsumed_authorization_at_commit', code='23514')
            check('mint_then_explicit_rollback', 'BEGIN;'+mint()+'ROLLBACK;')
            check('early_constraint_check_refuses_unconsumed_grant',
                  'BEGIN;'+mint()+'SET CONSTRAINTS ALL IMMEDIATE;COMMIT;',
                  error='unconsumed_authorization_at_commit', code='23514')
            for role in ('ll1_api', 'll1_control', 'll1_auth'):
                for label, sql in (('mint', mint()), ('consume', consume()),
                                   ('grant_write', 'INSERT INTO ll1_probe.authorization DEFAULT VALUES;'),
                                   ('permit_write', "INSERT INTO ll1_probe.permit VALUES(txid_current(),'x','{}');"),
                                   ('guard', 'SELECT ll1_probe.no_committed_work();')):
                    check(role+'_'+label+'_denied', 'BEGIN;'+base.ctx()+sql,
                          login=role, error='permission denied', code='42501')
            check('authority_direct_guard_denied', 'SELECT ll1_probe.no_committed_work();',
                  error='permission denied', code='42501')
            check('standalone_permit_commit_rejected', "BEGIN;SET LOCAL ROLE photara_owner;INSERT INTO ll1_probe.permit VALUES(txid_current(),'fixture-only','{}');COMMIT;",
                  login='ll1_fixture', error='unconsumed_permit_at_commit', code='23514')
            check('null_transaction_permit_cannot_escape_commit_guard', "BEGIN;SET LOCAL ROLE photara_owner;INSERT INTO ll1_probe.permit VALUES(NULL,'fixture-only','{}');COMMIT;",
                  login='ll1_fixture', error='unconsumed_permit_at_commit', code='23514')
            # A callable observer checks this boundary without granting the
            # authority login raw read access to its transient relations.
            base.require(pg, """SET ROLE photara_owner;
CREATE FUNCTION ll1_probe.minted_observer() RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN IF (SELECT count(*) FROM ll1_probe.authorization WHERE tx=txid_current() AND backend=pg_backend_pid())<>1
 THEN RAISE EXCEPTION 'mint_boundary_not_reached'; END IF; END $$;
REVOKE ALL ON FUNCTION ll1_probe.minted_observer() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ll1_probe.minted_observer() TO ll1_authority;
""")
            cancel_after_mint(pg)
            passed('cancel_after_mint_exact_rollback_backend_and_locks_gone')
            base.require(pg, 'SET ROLE photara_owner; DROP FUNCTION ll1_probe.minted_observer();')
            base.require(pg, """SET ROLE photara_owner;
CREATE FUNCTION ll1_probe.fail_after_consumption() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
 IF EXISTS(SELECT 1 FROM ll1_probe.authorization WHERE tx=txid_current() AND backend=pg_backend_pid())
 OR EXISTS(SELECT 1 FROM ll1_probe.permit WHERE tx=txid_current()) OR NEW.result<>'removed'
 THEN RAISE EXCEPTION 'consumption_boundary_not_reached'; END IF;
 RAISE EXCEPTION 'after_consumed_terminal';
END $$;
CREATE TRIGGER fail_after_consumption AFTER INSERT ON ll1_probe.receipts FOR EACH ROW EXECUTE FUNCTION ll1_probe.fail_after_consumption();
""")
            check('execution_failure_after_consumption_restores_all_rows',
                  'BEGIN;'+mint()+consume()+'COMMIT;', error='after_consumed_terminal', code='P0001')
            base.require(pg, 'SET ROLE photara_owner; DROP TRIGGER fail_after_consumption ON ll1_probe.receipts; DROP FUNCTION ll1_probe.fail_after_consumption();')
            before = snapshot(pg)
            assert check('consumed_grant_successful_commit', 'BEGIN;'+mint()+consume()+'COMMIT;', preserve=False) == 'removed'
            after = snapshot(pg)
            for table, rows in json.loads(before[0]).items():
                expected = [r for r in rows if not(table in owned and (r.get('library_id') == base.uid(1) or r.get('claimed_library_id') == base.uid(1)))]
                assert json.loads(after[0])[table] == expected, table
            overlay_rows = json.loads(after[1])
            assert overlay_rows['authorization'] == overlay_rows['permit'] == []
            assert len(overlay_rows['receipts']) == len(overlay_rows['request_binding']) == 1
            original_receipt = overlay_rows['receipts']
            assert check('same_original_retry_preserves_receipt_and_all_rows', 'BEGIN;'+mint()+consume()+'COMMIT;') == 'removed'
            check('mint_only_retry_commit_rejected_preserves_original_receipt',
                  'BEGIN;'+mint()+'COMMIT;', error='unconsumed_authorization_at_commit', code='23514')
            check('changed_original_digest_refuses', 'BEGIN;'+mint(digest='changed')+consume(digest='changed')+'COMMIT;', error='operation_conflict')
            assert json.loads(snapshot(pg)[1])['receipts'] == original_receipt
            assert snapshot(pg) == after
            assert catalog(pg) == original_catalog
            for name, digest in hashes.items():
                assert hashlib.sha256((base.ROOT/'crates/photara-service/migrations/postgres'/name).read_bytes()).hexdigest() == digest
            print(json.dumps({'cases': len(cases), 'result': 'pass', 'baseline': measured,
                              'baseline_catalog_unchanged_by_commit_guard': True,
                              'baseline_measured_before_inherited_executor_overlay': True,
                              'production_changes': False, 'historical_raw_fixture_changed': False,
                              'scope': 'disposable deferred commit guard, raw authority surface only'}, sort_keys=True))
        finally:
            pg.close()
    print(json.dumps({'temporary_cluster': 'stopped and removed'}))


if __name__ == '__main__':
    main()
