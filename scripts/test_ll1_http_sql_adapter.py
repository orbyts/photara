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
            for name, digest in hashes.items():
                import hashlib
                assert hashlib.sha256((base.ROOT/'crates/photara-service/migrations/postgres'/name).read_bytes()).hexdigest() == digest
            print(json.dumps({'result':'pass','baseline':measured,'production_changes':False,
                              'scope':'signed HTTP and protected SQL, private fixture only',
                              'lifecycle':args.lifecycle,
                              'commit_relay':args.commit_relay}, sort_keys=True))
        finally:
            try:
                if relay:
                    relay.close()
            finally:
                pg.close()
    print(json.dumps({'temporary_cluster':'stopped and removed'}))


if __name__ == '__main__':
    main()
