#!/usr/bin/env python3
"""Disposable fresh-device receipt proof; no production route, codec or live URL.

Composes the unchanged full-schema executor and authority fixtures in their
private socket-only cluster. All additional DDL is a candidate test overlay.
"""
import argparse
import hashlib
import json
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
import test_ll1_protected_authorization as authority
from test_ll1_protected_authorization import base, executor


def overlay(pg):
    base.require(pg, """SET ROLE photara_owner;
-- Synthetic clock/review, seeded only by the harness administrator. No token codec.
CREATE TABLE ll1_probe.review_clock(singleton boolean PRIMARY KEY CHECK(singleton), instant timestamptz NOT NULL);
INSERT INTO ll1_probe.review_clock VALUES(true,clock_timestamp());
CREATE TABLE ll1_probe.device_review(
 actor uuid NOT NULL, operation uuid NOT NULL, identity uuid NOT NULL,
 device uuid NOT NULL, library uuid NOT NULL, revision bigint NOT NULL,
 digest bytea NOT NULL, expires timestamptz NOT NULL, PRIMARY KEY(actor,operation));
CREATE TRIGGER immutable BEFORE UPDATE OR DELETE ON ll1_probe.device_review
 FOR EACH ROW EXECUTE FUNCTION photara_private.append_only();
CREATE TABLE ll1_probe.original_receipt(
 actor uuid NOT NULL, operation uuid NOT NULL, digest bytea NOT NULL,
 body bytea NOT NULL, PRIMARY KEY(actor,operation));
CREATE TRIGGER immutable BEFORE UPDATE OR DELETE ON ll1_probe.original_receipt
 FOR EACH ROW EXECUTE FUNCTION photara_private.append_only();
CREATE FUNCTION ll1_probe.capture_receipt() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
DECLARE reviewed ll1_probe.device_review;
BEGIN
 SELECT * INTO STRICT reviewed FROM ll1_probe.device_review
 WHERE actor=NEW.actor AND operation=NEW.operation;
 IF reviewed.library<>NEW.library OR reviewed.revision<>NEW.revision
 THEN RAISE EXCEPTION 'receipt_binding'; END IF;
 -- Deliberately test-only bytes, captured once, never reconstructed on lookup.
 INSERT INTO ll1_probe.original_receipt VALUES(NEW.actor,NEW.operation,reviewed.digest,
 convert_to(jsonb_build_object('fixture_codec',1,'actor',NEW.actor,
 'operation',NEW.operation,'library',NEW.library,'revision',NEW.revision,
 'result',NEW.result,'original_device',reviewed.device,
 'digest',encode(reviewed.digest,'hex'),'captured_at',clock_timestamp())::text,'UTF8'));
 RETURN NEW;
END $$;
CREATE TRIGGER capture_receipt AFTER INSERT ON ll1_probe.receipts
 FOR EACH ROW EXECUTE FUNCTION ll1_probe.capture_receipt();
CREATE FUNCTION ll1_probe.device_actor(p_identity uuid,p_actor uuid,p_device uuid,p_secret bytea,p_credential_revision bigint) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
BEGIN
 -- Identity/account are synthetic trusted-service inputs, not OIDC verification.
 PERFORM ll1_probe.context(p_identity,p_actor,NULL);
 PERFORM 1 FROM photara_identity.devices
 WHERE account_id=p_actor AND device_id=p_device AND state='active' FOR SHARE;
 IF NOT FOUND THEN RAISE EXCEPTION 'device_denied'; END IF;
 PERFORM 1 FROM photara_identity.device_credentials
 WHERE account_id=p_actor AND device_id=p_device AND state='active'
 AND revision=p_credential_revision AND octet_length(p_secret)=32
 AND commitment=sha256(p_secret) FOR SHARE;
 IF NOT FOUND THEN RAISE EXCEPTION 'credential_denied'; END IF;
END $$;
CREATE FUNCTION ll1_probe.query_receipt(p_identity uuid,p_actor uuid,p_device uuid,p_secret bytea,p_credential_revision bigint,p_operation uuid,p_digest bytea) RETURNS bytea
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
DECLARE receipt ll1_probe.original_receipt;
BEGIN
 PERFORM ll1_probe.device_actor(p_identity,p_actor,p_device,p_secret,p_credential_revision);
 SELECT * INTO receipt FROM ll1_probe.original_receipt
 WHERE actor=p_actor AND operation=p_operation;
 IF NOT FOUND THEN RETURN NULL; END IF; -- NotFound, never permission to execute.
 IF receipt.digest IS DISTINCT FROM p_digest THEN RAISE EXCEPTION 'operation_conflict'; END IF;
 RETURN receipt.body;
END $$;
CREATE FUNCTION ll1_probe.execute_review(p_identity uuid,p_actor uuid,p_device uuid,p_secret bytea,p_credential_revision bigint,p_operation uuid,p_digest bytea) RETURNS bytea
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,pg_temp AS $$
DECLARE reviewed ll1_probe.device_review; receipt bytea;
BEGIN
 -- Committed receipt access precedes review validation, with fresh authentication.
 receipt:=ll1_probe.query_receipt(p_identity,p_actor,p_device,p_secret,p_credential_revision,p_operation,p_digest);
 IF receipt IS NOT NULL THEN RETURN receipt; END IF;
 SELECT * INTO reviewed FROM ll1_probe.device_review
 WHERE actor=p_actor AND operation=p_operation;
 IF NOT FOUND OR reviewed.identity IS DISTINCT FROM p_identity
 OR reviewed.device IS DISTINCT FROM p_device THEN RAISE EXCEPTION 'original_review_device_required'; END IF;
 IF reviewed.digest IS DISTINCT FROM p_digest THEN RAISE EXCEPTION 'operation_conflict'; END IF;
 IF reviewed.expires<=(SELECT instant FROM ll1_probe.review_clock)
 THEN RAISE EXCEPTION 'review_expired'; END IF;
 PERFORM ll1_probe.authorize(p_identity,p_actor,reviewed.library,reviewed.revision,p_operation,p_digest);
 PERFORM ll1_probe.execute(p_identity,p_actor,reviewed.library,reviewed.revision,p_operation,p_digest);
 RETURN (SELECT body FROM ll1_probe.original_receipt WHERE actor=p_actor AND operation=p_operation);
END $$;
REVOKE ALL ON ALL TABLES IN SCHEMA ll1_probe FROM ll1_authority;
REVOKE ALL ON FUNCTION ll1_probe.capture_receipt(),
 ll1_probe.device_actor(uuid,uuid,uuid,bytea,bigint),
 ll1_probe.query_receipt(uuid,uuid,uuid,bytea,bigint,uuid,bytea),
 ll1_probe.execute_review(uuid,uuid,uuid,bytea,bigint,uuid,bytea) FROM PUBLIC;
-- Only wrappers are service-callable in this fixture; no bypass of device review.
REVOKE EXECUTE ON FUNCTION ll1_probe.authorize(uuid,uuid,uuid,bigint,uuid,bytea),
 ll1_probe.execute(uuid,uuid,uuid,bigint,uuid,bytea) FROM ll1_authority;
GRANT EXECUTE ON FUNCTION ll1_probe.query_receipt(uuid,uuid,uuid,bytea,bigint,uuid,bytea),
 ll1_probe.execute_review(uuid,uuid,uuid,bytea,bigint,uuid,bytea) TO ll1_authority;
""")
    base.require(pg, f"""SET ROLE photara_owner;
INSERT INTO photara_identity.account_identities VALUES
 ('{base.uid(111)}','{base.uid(10)}','https://fake.invalid/','second-owner-identity','active',1,now(),now(),NULL);
INSERT INTO photara_identity.devices VALUES
 ('{base.uid(10)}','{base.uid(211)}','B','active',now(),NULL),
 ('{base.uid(20)}','{base.uid(220)}','Other','active',now(),NULL);
INSERT INTO photara_identity.device_credentials VALUES
 ('{base.uid(10)}','{base.uid(210)}',sha256(decode(repeat('aa',32),'hex')),'active',1,now(),now()),
 ('{base.uid(10)}','{base.uid(211)}',sha256(decode(repeat('bb',32),'hex')),'active',1,now(),now()),
 ('{base.uid(20)}','{base.uid(220)}',sha256(decode(repeat('cc',32),'hex')),'active',1,now(),now());
INSERT INTO ll1_probe.device_review
 SELECT '{base.uid(10)}',operation,'{base.uid(110)}','{base.uid(210)}',
 '{base.uid(1)}',1,sha256('request-one'::bytea),instant+interval '5 minutes'
 FROM ll1_probe.review_clock CROSS JOIN
 (VALUES('{base.uid(9200)}'::uuid),('{base.uid(9201)}'::uuid)) q(operation);
""")


def snapshot(pg):
    tables = json.loads(pg.scalar("SELECT json_agg(tablename ORDER BY tablename) FROM pg_tables WHERE schemaname='ll1_probe'"))
    parts = [f"SELECT '{t}' AS name,coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),'[]'::jsonb) AS rows FROM ll1_probe.{t} t" for t in tables]
    return base.snapshot(pg), pg.scalar("SELECT jsonb_object_agg(name,rows) FROM (" + " UNION ALL ".join(parts) + ") q")


def args(identity=110, actor=10, device=210, secret='aa', credential_revision=1,
         operation=9200, digest='request-one'):
    return (f"'{base.uid(identity)}','{base.uid(actor)}','{base.uid(device)}',"
            f"decode(repeat('{secret}',32),'hex'),{credential_revision},"
            f"'{base.uid(operation)}',sha256('{digest}'::bytea)")


def query(**kw):
    return f"SELECT coalesce(encode(ll1_probe.query_receipt({args(**kw)}),'hex'),'NotFound');"


def execute(**kw):
    return f"SELECT encode(ll1_probe.execute_review({args(**kw)}),'hex');"


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
            overlay(pg)
            initial = snapshot(pg)
            cases = []

            def check(name, sql, *, value=None, error=None, login='ll1_service', preserve=True):
                before = snapshot(pg)
                result = base.execute(pg, sql, login)
                if error:
                    first = next((s for s in result.stderr.splitlines() if s.startswith('ERROR:')), '')
                    assert result.returncode and error in first, result.stderr
                else:
                    assert result.returncode == 0, result.stderr
                    if value is not None:
                        assert result.stdout.strip() == value, result.stdout
                if preserve:
                    assert snapshot(pg) == before, name
                cases.append(name)
                print(json.dumps({'case': name, 'result': 'pass'}), flush=True)

            b = {'device': 211, 'secret': 'bb'}
            check('b_no_terminal_is_not_found', query(**b), value='NotFound')
            check('b_cannot_execute_a_review', execute(**b), error='original_review_device_required')
            base.require(pg, "SET ROLE photara_owner; UPDATE ll1_probe.review_clock SET instant=instant+interval '6 minutes';")
            check('a_expired_review_cannot_first_execute', execute(), error='review_expired')
            base.require(pg, "SET ROLE photara_owner; UPDATE ll1_probe.review_clock SET instant=instant-interval '6 minutes';")
            for label, kw in [('other_device_secret', {'secret': 'bb'}),
                              ('random_secret', {'secret': 'dd'}),
                              ('wrong_revision', {'credential_revision': 2})]:
                check(label, execute(**kw), error='credential_denied')
            check('identity_substitution', execute(identity=120), error='not_found_or_forbidden')
            check('execution_changed_hash', execute(digest='changed'), error='operation_conflict')
            for login in ('ll1_api', 'll1_control', 'll1_auth'):
                for label, sql in [('query', query()), ('execute', execute()),
                                   ('receipt_read', 'SELECT * FROM ll1_probe.original_receipt;'),
                                   ('receipt_write', 'INSERT INTO ll1_probe.original_receipt DEFAULT VALUES;')]:
                    check(login + '_' + label + '_denied', 'BEGIN;' + base.ctx() + sql,
                          login=login, error='permission denied')
            for label, sql in [('raw_authorize', f'SELECT ll1_probe.authorize({authority_args()});'),
                               ('raw_execute', f'SELECT ll1_probe.execute({authority_args()});'),
                               ('review_write', 'INSERT INTO ll1_probe.device_review DEFAULT VALUES;')]:
                check('service_' + label + '_denied', sql, error='permission denied')

            # Administrator-only faults at exact grant/terminal boundaries.
            # Each failed wrapper transaction must restore every baseline AND
            # overlay row, including request binding, grant and receipt work.
            for label, timing, table, count, extra in [
                ('before_grant_consumption', 'BEFORE DELETE', 'authorization', 1, ''),
                ('after_grant_consumption', 'AFTER DELETE', 'authorization', 0, ''),
                ('after_terminal_capture', 'AFTER INSERT', 'receipts', 0,
                 "IF (SELECT count(*) FROM ll1_probe.original_receipt)<>1 "
                 "THEN RAISE EXCEPTION 'fault_boundary_not_reached'; END IF;"),
            ]:
                base.require(pg, f"""SET ROLE photara_owner;
CREATE FUNCTION ll1_probe.inject_boundary_failure() RETURNS trigger
LANGUAGE plpgsql AS $$ BEGIN
 IF (SELECT count(*) FROM ll1_probe.authorization)<>{count}
 THEN RAISE EXCEPTION 'fault_boundary_not_reached'; END IF;
 {extra}
 RAISE EXCEPTION 'injected_{label}';
END $$;
CREATE TRIGGER zz_boundary_failure {timing} ON ll1_probe.{table}
 FOR EACH ROW EXECUTE FUNCTION ll1_probe.inject_boundary_failure();
""")
                check(label + '_exact_rollback', 'BEGIN;' + execute() + 'COMMIT;',
                      error='injected_' + label)
                base.require(pg, f"SET ROLE photara_owner; DROP TRIGGER zz_boundary_failure ON ll1_probe.{table}; DROP FUNCTION ll1_probe.inject_boundary_failure();")
            check('explicit_rollback_after_terminal_capture', 'BEGIN;' + execute() + 'ROLLBACK;')

            # Valid A executes, but the simulated client loses the entire reply.
            # Server evidence is read independently; no response is used for recovery.
            check('a_commit_reply_discarded', 'BEGIN;' + execute() + 'COMMIT;', preserve=False)
            original = pg.scalar('SELECT encode(body,\'hex\') FROM ll1_probe.original_receipt')
            decoded = json.loads(bytes.fromhex(original))
            assert decoded['original_device'] == base.uid(210) and decoded['result'] == 'removed'
            assert pg.scalar(f"SELECT count(*) FROM photara_identity.memberships WHERE library_id='{base.uid(1)}'") == '0'
            after = snapshot(pg)
            for table, rows in json.loads(initial[0]).items():
                expected = [r for r in rows if not (table in owned and r.get('library_id') == base.uid(1))]
                assert json.loads(after[0])[table] == expected, table
            check('a_original_bytes', query(), value=original)
            base.require(pg, f"""SET ROLE photara_owner;
UPDATE photara_identity.devices SET state='revoked' WHERE device_id='{base.uid(210)}';
UPDATE photara_identity.device_credentials SET state='suspended',revision=revision+1,updated_at=clock_timestamp()
 WHERE device_id='{base.uid(210)}';
-- Poison grant issuance: receipt recovery must never enter grant/executor admission.
CREATE FUNCTION ll1_probe.no_grant() RETURNS trigger LANGUAGE plpgsql AS $$
 BEGIN RAISE EXCEPTION 'receipt_must_not_mint_grant'; END $$;
CREATE TRIGGER no_grant BEFORE INSERT ON ll1_probe.authorization
 FOR EACH ROW EXECUTE FUNCTION ll1_probe.no_grant();
""")
            check('revoked_a_receipt_denied', query(), error='device_denied')
            check('revoked_a_retry_denied', execute(), error='device_denied')
            check('eligible_b_exact_original_without_grant', query(**b), value=original)
            check('eligible_b_current_identity_same_initiating_account', query(**b, identity=111), value=original)
            check('eligible_b_retry_only_reads_terminal', execute(**b), value=original)
            check('other_account_no_receipt_disclosure', query(identity=120, actor=20, device=220, secret='cc'), value='NotFound')
            check('b_changed_hash_denied', query(**b, digest='changed'), error='operation_conflict')
            check('b_unknown_operation_not_found', query(**b, operation=9299), value='NotFound')
            check('b_unexecuted_review_not_found', query(**b, operation=9201), value='NotFound')
            check('b_unexecuted_review_cannot_execute', execute(**b, operation=9201), error='original_review_device_required')
            base.require(pg, "SET ROLE photara_owner; UPDATE ll1_probe.review_clock SET instant=instant+interval '6 minutes';")
            check('b_receipt_after_review_expiry', query(**b), value=original)
            check('b_retry_after_review_expiry', execute(**b), value=original)
            check('b_missing_terminal_after_expiry', execute(**b, operation=9201), error='original_review_device_required')
            base.require(pg, f"""SET ROLE photara_owner;
UPDATE photara_identity.device_credentials SET state='suspended',revision=revision+1,updated_at=clock_timestamp()
 WHERE device_id='{base.uid(211)}';""")
            check('active_b_suspended_credential_denied', query(**b, credential_revision=2), error='credential_denied')
            base.require(pg, f"""SET ROLE photara_owner;
UPDATE photara_identity.device_credentials SET state='active',revision=revision+1,updated_at=clock_timestamp()
 WHERE device_id='{base.uid(211)}';""")
            b['credential_revision'] = 3
            before_restart = snapshot(pg)
            # A clean server restart proves persisted recovery, not power-loss durability.
            subprocess.run(['pg_ctl', '-D', str(pg.data), '-l', str(pg.root / 'postgres.log'),
                            '-w', '-m', 'fast', 'restart'], check=True, capture_output=True, text=True, env=pg.env)
            assert pg.scalar('SHOW listen_addresses') == ''
            assert pg.scalar('SHOW unix_socket_directories') == str(pg.socket)
            assert snapshot(pg) == before_restart
            check('b_exact_receipt_after_restart', query(**b), value=original)
            check('b_no_terminal_still_no_execution_after_restart', execute(**b, operation=9201), error='original_review_device_required')
            check('revoked_a_still_denied_after_restart', query(), error='device_denied')
            for table in ('authorization', 'permit'):
                assert pg.scalar(f'SELECT count(*) FROM ll1_probe.{table}') == '0', table
            for table in ('receipts', 'original_receipt', 'markers', 'inventory', 'request_binding'):
                assert pg.scalar(f'SELECT count(*) FROM ll1_probe.{table}') == '1', table
            assert pg.scalar("SELECT count(*) FROM pg_constraint c JOIN pg_namespace n ON n.oid=c.connamespace WHERE n.nspname IN " + base.SCHEMAS + " AND c.contype='f'") == str(measured['foreign_keys'])
            for name, digest in hashes.items():
                assert hashlib.sha256((base.ROOT / 'crates/photara-service/migrations/postgres' / name).read_bytes()).hexdigest() == digest
            print(json.dumps({'cases': len(cases), 'catalog': measured,
                              'status': 'disposable_fresh_device_receipt_proven',
                              'limitations': ['synthetic trusted identity; no HTTP/OIDC integration',
                                              'fixture review/receipt/secret codecs; not production formats',
                                              'sparse aggregate; no complete lifecycle concurrency proof',
                                              'clean restart with fsync-disabled inherited harness; no power-loss qualification',
                                              'NotFound never establishes a conclusive rejection']}), flush=True)
        finally:
            pg.close()
    print(json.dumps({'temporary_cluster': 'stopped and removed'}), flush=True)


def authority_args():
    return f"'{base.uid(110)}','{base.uid(10)}','{base.uid(1)}',1,'{base.uid(9200)}',sha256('request-one'::bytea)"


if __name__ == '__main__':
    try:
        main()
    except subprocess.CalledProcessError as error:
        # Startup errors otherwise hide the reason a private cluster is unavailable.
        print(error.stderr or error.stdout, file=sys.stderr)
        raise
