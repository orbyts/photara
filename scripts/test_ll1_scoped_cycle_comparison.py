#!/usr/bin/env python3
"""Private disposable PostgreSQL comparison; no production DDL or live endpoint.

Reuses the previously reviewed private-cluster lifecycle. Fixed miniature schema,
integer surrogate IDs and model guards are deliberately not production migration
inputs, authenticated capabilities, canonical codecs or full RLS proof.
"""
import argparse
import json
import subprocess
import sys
import tempfile

# Avoid import-generated files in the repository; harness itself writes no files.
sys.dont_write_bytecode = True
from test_ll1_retirement_constraints import Postgres  # noqa: E402


OPTIONS = {
    "both_restrict": ("RESTRICT", "RESTRICT"),
    "receipt_to_batch_no_action": ("NO ACTION", "RESTRICT"),
    "batch_to_receipt_no_action": ("RESTRICT", "NO ACTION"),
}
TABLE_KEYS = {
    "libraries": ["library_id"],
    "scoped_streams": ["stream_id", "epoch"],
    "scoped_change_batches": ["stream_id", "epoch", "sequence"],
    "scoped_command_receipts": ["library_id", "operation_id"],
    "scoped_changes": ["stream_id", "epoch", "sequence", "ordinal"],
}


def schema(option):
    receipt_action, batch_action = OPTIONS[option]
    sql = f"""
DROP SCHEMA IF EXISTS ll1_fixture CASCADE;
CREATE SCHEMA ll1_fixture;
CREATE TABLE libraries(library_id integer PRIMARY KEY, payload text NOT NULL);
CREATE TABLE scoped_streams(
  stream_id integer NOT NULL, epoch integer NOT NULL, library_id integer NOT NULL,
  payload text NOT NULL, PRIMARY KEY(stream_id,epoch), UNIQUE(stream_id,epoch,library_id),
  FOREIGN KEY(library_id) REFERENCES libraries ON DELETE RESTRICT);
CREATE TABLE scoped_change_batches(
  stream_id integer NOT NULL, epoch integer NOT NULL, sequence integer NOT NULL,
  library_id integer NOT NULL, operation_id integer NOT NULL, payload text NOT NULL,
  PRIMARY KEY(stream_id,epoch,sequence), UNIQUE(stream_id,epoch,sequence,library_id),
  UNIQUE(library_id,operation_id),
  FOREIGN KEY(stream_id,epoch,library_id) REFERENCES scoped_streams(stream_id,epoch,library_id) ON DELETE RESTRICT,
  FOREIGN KEY(library_id) REFERENCES libraries ON DELETE RESTRICT);
CREATE TABLE scoped_command_receipts(
  library_id integer NOT NULL, operation_id integer NOT NULL, outcome text NOT NULL,
  accepted_stream_id integer, accepted_epoch integer, accepted_sequence integer,
  payload text NOT NULL, PRIMARY KEY(library_id,operation_id),
  CONSTRAINT receipt_to_batch FOREIGN KEY(accepted_stream_id,accepted_epoch,accepted_sequence,library_id)
    REFERENCES scoped_change_batches(stream_id,epoch,sequence,library_id)
    ON DELETE {receipt_action} DEFERRABLE INITIALLY DEFERRED,
  CHECK ((outcome='accepted' AND accepted_stream_id IS NOT NULL AND accepted_epoch IS NOT NULL
    AND accepted_sequence IS NOT NULL AND accepted_sequence>0) OR
    (outcome<>'accepted' AND accepted_stream_id IS NULL AND accepted_epoch IS NULL AND accepted_sequence IS NULL)),
  FOREIGN KEY(library_id) REFERENCES libraries ON DELETE RESTRICT);
ALTER TABLE scoped_change_batches ADD CONSTRAINT batch_to_receipt
  FOREIGN KEY(library_id,operation_id) REFERENCES scoped_command_receipts(library_id,operation_id)
  ON DELETE {batch_action} DEFERRABLE INITIALLY DEFERRED;
CREATE TABLE scoped_changes(
  stream_id integer NOT NULL, epoch integer NOT NULL, sequence integer NOT NULL,
  ordinal integer NOT NULL, library_id integer NOT NULL, payload text NOT NULL,
  PRIMARY KEY(stream_id,epoch,sequence,ordinal),
  FOREIGN KEY(stream_id,epoch,sequence,library_id) REFERENCES scoped_change_batches(stream_id,epoch,sequence,library_id) ON DELETE RESTRICT,
  FOREIGN KEY(library_id) REFERENCES libraries ON DELETE RESTRICT);
CREATE TABLE commit_guard(guard_id integer PRIMARY KEY CHECK(guard_id=0));
CREATE TABLE retirement_work(
  operation_id integer PRIMARY KEY, library_id integer NOT NULL, actor_id integer NOT NULL,
  request_hash text NOT NULL, valid_until timestamptz NOT NULL, transaction_id bigint NOT NULL,
  debt integer NOT NULL CHECK(debt=1),
  CONSTRAINT work_debt FOREIGN KEY(debt) REFERENCES commit_guard
    ON DELETE NO ACTION DEFERRABLE INITIALLY DEFERRED);
CREATE TABLE retirement_rows(
  operation_id integer NOT NULL REFERENCES retirement_work ON DELETE RESTRICT,
  table_tag text NOT NULL CHECK(table_tag IN ({','.join(repr(t) for t in TABLE_KEYS)})),
  library_id integer NOT NULL, row_key jsonb NOT NULL, request_hash text NOT NULL,
  PRIMARY KEY(operation_id,table_tag,library_id,row_key));
CREATE TABLE lifecycle_receipts(
  authority_id integer NOT NULL, actor_id integer NOT NULL, operation_id integer NOT NULL,
  historical_library_id integer NOT NULL, result text NOT NULL CHECK(result IN ('removed','rejected')),
  request_hash text NOT NULL,
  PRIMARY KEY(authority_id,actor_id,operation_id),
  UNIQUE(authority_id,actor_id,operation_id,historical_library_id,result,request_hash));
CREATE TABLE removed_library_identities(
  authority_id integer NOT NULL, actor_id integer NOT NULL, operation_id integer NOT NULL,
  historical_library_id integer NOT NULL, result text NOT NULL CHECK(result='removed'),
  request_hash text NOT NULL,
  PRIMARY KEY(authority_id,historical_library_id),
  FOREIGN KEY(authority_id,actor_id,operation_id,historical_library_id,result,request_hash)
    REFERENCES lifecycle_receipts(authority_id,actor_id,operation_id,historical_library_id,result,request_hash)
    ON DELETE NO ACTION DEFERRABLE INITIALLY DEFERRED);
"""
    for table, key in TABLE_KEYS.items():
        key_expr = "jsonb_build_array(" + ",".join("OLD." + k for k in key) + ")"
        sql += f"""
CREATE FUNCTION guard_{table}() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF TG_OP='UPDATE' THEN RAISE EXCEPTION 'ordinary_update_guard'; END IF;
  IF NOT EXISTS(SELECT 1 FROM retirement_work w JOIN retirement_rows m USING(operation_id)
    WHERE w.library_id=OLD.library_id AND m.library_id=OLD.library_id
      AND w.operation_id=10 AND w.actor_id=7 AND w.request_hash='reviewed-request'
      AND m.request_hash=w.request_hash AND w.transaction_id=txid_current()
      AND w.valid_until>statement_timestamp() AND m.table_tag='{table}' AND m.row_key={key_expr})
  THEN RAISE EXCEPTION 'retirement_guard'; END IF;
  RETURN OLD;
END $$;
CREATE TRIGGER guard_{table} BEFORE UPDATE OR DELETE ON {table}
  FOR EACH ROW EXECUTE FUNCTION guard_{table}();
"""
    sql += """
CREATE FUNCTION terminal_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'terminal_immutable'; END $$;
CREATE TRIGGER receipt_immutable BEFORE UPDATE OR DELETE ON lifecycle_receipts
FOR EACH ROW EXECUTE FUNCTION terminal_immutable();
CREATE TRIGGER marker_immutable BEFORE UPDATE OR DELETE ON removed_library_identities
FOR EACH ROW EXECUTE FUNCTION terminal_immutable();
BEGIN;
INSERT INTO libraries VALUES(1,'target'),(2,'unrelated-library');
INSERT INTO scoped_streams VALUES(11,1,1,'source-stream'),(22,1,2,'unrelated-stream');
INSERT INTO scoped_change_batches VALUES(11,1,1,1,101,'target-batch'),(22,1,1,2,202,'unrelated-batch');
INSERT INTO scoped_command_receipts VALUES(1,101,'accepted',11,1,1,'target-exact-receipt'),
  (2,202,'accepted',22,1,1,'unrelated-exact-receipt');
INSERT INTO scoped_changes VALUES(11,1,1,0,1,'target-change'),(22,1,1,0,2,'unrelated-change');
COMMIT;
"""
    return sql


def prepare():
    result = ["BEGIN", "INSERT INTO retirement_work VALUES(10,1,7,'reviewed-request',statement_timestamp()+interval '5 minutes',txid_current(),1)"]
    for table, key in TABLE_KEYS.items():
        result.append(f"INSERT INTO retirement_rows SELECT 10,'{table}',library_id,jsonb_build_array({','.join(key)}),'reviewed-request' FROM {table} WHERE library_id=1")
    return result


def cycle(strategy, *, wrong_batch_scope=False):
    receipt = "DELETE FROM scoped_command_receipts WHERE library_id=1"
    batch = "DELETE FROM scoped_change_batches WHERE library_id=" + ("2" if wrong_batch_scope else "1")
    if strategy == "receipts_first":
        return [receipt, batch]
    if strategy == "batches_first":
        return [batch, receipt]
    if strategy == "cte_receipts_first":
        return [f"WITH retired_receipts AS ({receipt} RETURNING library_id), retired_batches AS ({batch} AND (SELECT count(*) FROM retired_receipts)>=0 RETURNING library_id) SELECT (SELECT count(*) FROM retired_receipts)+(SELECT count(*) FROM retired_batches)"]
    if strategy == "cte_batches_first":
        return [f"WITH retired_batches AS ({batch} RETURNING library_id), retired_receipts AS ({receipt} AND (SELECT count(*) FROM retired_batches)>=0 RETURNING library_id) SELECT (SELECT count(*) FROM retired_receipts)+(SELECT count(*) FROM retired_batches)"]
    raise ValueError(strategy)


def terminal(marker="1,7,10,1,'removed','reviewed-request'", receipt="1,7,10,1,'removed','reviewed-request'"):
    return [f"INSERT INTO lifecycle_receipts VALUES({receipt})", f"INSERT INTO removed_library_identities VALUES({marker})"]


def finish():
    return ["SET CONSTRAINTS receipt_to_batch,batch_to_receipt IMMEDIATE",
            "DELETE FROM retirement_rows", "DELETE FROM retirement_work", "COMMIT"]


def retired(strategy):
    return ["DELETE FROM scoped_changes WHERE library_id=1"] + cycle(strategy) + [
        "DELETE FROM scoped_streams WHERE library_id=1", "DELETE FROM libraries WHERE library_id=1"]


def snapshot(pg, library):
    return {table: pg.scalar(f"SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),'[]'::jsonb) FROM {table} t WHERE library_id={library}") for table in TABLE_KEYS}


def run_case(pg, option, name, statements, error=None, removed=False):
    setup = pg.execute(schema(option))
    assert setup.returncode == 0, setup.stderr
    unrelated = snapshot(pg, 2)
    initial = snapshot(pg, 1)
    pg.run(statements, error)
    assert snapshot(pg, 2) == unrelated, (option, name, "unrelated row changed")
    assert snapshot(pg, 1) == ({t: "[]" for t in TABLE_KEYS} if removed else initial)
    for table in ("retirement_work", "retirement_rows"):
        assert pg.scalar(f"SELECT count(*) FROM {table}") == "0"
    for table in ("lifecycle_receipts", "removed_library_identities"):
        assert pg.scalar(f"SELECT count(*) FROM {table}") == ("1" if removed else "0")
    pg.fk_check()
    if removed:
        for table in TABLE_KEYS:
            pg.run([f"DELETE FROM {table} WHERE library_id=2"], "retirement_guard")
        for table in ("lifecycle_receipts", "removed_library_identities"):
            pg.run([f"DELETE FROM {table}"], "terminal_immutable")
            pg.run([f"UPDATE {table} SET actor_id=99"], "terminal_immutable")
        assert snapshot(pg, 2) == unrelated
    print(json.dumps({"option": option, "case": name, "expected": error or "commit", "result": "pass"}), flush=True)


def compare(pg):
    counts = {}
    fk = "foreign key constraint"
    for option in OPTIONS:
        count = 0
        strategies = ("receipts_first", "batches_first", "cte_receipts_first", "cte_batches_first")
        for strategy in strategies:
            success = strategy.startswith("cte_") or (option == "receipt_to_batch_no_action" and strategy == "batches_first") or (option == "batch_to_receipt_no_action" and strategy == "receipts_first")
            run_case(pg, option, strategy, prepare() + retired(strategy) + terminal() + finish(),
                     None if success else fk, success)
            count += 1
        strategy = {"both_restrict": "cte_receipts_first", "receipt_to_batch_no_action": "batches_first", "batch_to_receipt_no_action": "receipts_first"}[option]
        for table in TABLE_KEYS:
            run_case(pg, option, f"ordinary_delete_{table}", [f"DELETE FROM {table} WHERE library_id=1"], "retirement_guard")
            count += 1
        bad_permits = {
            "wrong_library": "UPDATE retirement_work SET library_id=2",
            "wrong_actor": "UPDATE retirement_work SET actor_id=8",
            "wrong_request": "UPDATE retirement_work SET request_hash='different'",
            "expired_permit": "UPDATE retirement_work SET valid_until=statement_timestamp()-interval '1 second'",
            "wrong_transaction": "UPDATE retirement_work SET transaction_id=0",
            "mismatched_manifest_hash": "UPDATE retirement_rows SET request_hash='different'",
            "mismatched_manifest_key": "UPDATE retirement_rows SET row_key='[999]'::jsonb",
            "wrong_table_tag": "DELETE FROM retirement_rows WHERE table_tag='scoped_changes'",
        }
        for name, tamper in bad_permits.items():
            run_case(pg, option, name, prepare() + [tamper] + retired(strategy) + terminal() + finish(), "retirement_guard")
            count += 1
        for name, statements, error in (
            ("expired_after_leaf_delete", prepare() + ["DELETE FROM scoped_changes WHERE library_id=1", bad_permits["expired_permit"]] + cycle(strategy), "retirement_guard"),
            ("missing_leaf_closure", prepare() + cycle(strategy) + terminal() + finish(), fk),
            ("wrong_cte_library", prepare() + ["DELETE FROM scoped_changes WHERE library_id=1"] + cycle("cte_receipts_first", wrong_batch_scope=True), "retirement_guard"),
            ("permit_cannot_persist", prepare() + retired(strategy) + terminal() + ["COMMIT"], fk),
            ("rollback_after_cycle_delete", prepare() + retired(strategy) + ["SELECT 1/0"], "division by zero"),
            ("accepted_coordinate_update_denied", prepare() + ["UPDATE scoped_command_receipts SET accepted_stream_id=NULL WHERE library_id=1"], "ordinary_update_guard"),
        ):
            run_case(pg, option, name, statements, error)
            count += 1
        for field, marker in (
            ("authority", "2,7,10,1,'removed','reviewed-request'"),
            ("actor", "1,8,10,1,'removed','reviewed-request'"),
            ("operation", "1,7,11,1,'removed','reviewed-request'"),
            ("library", "1,7,10,2,'removed','reviewed-request'"),
            ("request", "1,7,10,1,'removed','different'"),
        ):
            run_case(pg, option, f"terminal_{field}_mismatch", prepare() + retired(strategy) + terminal(marker=marker) + finish(), fk)
            count += 1
        run_case(pg, option, "terminal_outcome_mismatch", prepare() + retired(strategy) + terminal(receipt="1,7,10,1,'rejected','reviewed-request'") + finish(), fk)
        counts[option] = count + 1
    return counts


def main():
    argparse.ArgumentParser(description=__doc__).parse_args()
    if not __debug__:
        raise RuntimeError("Assertions must be enabled")
    with tempfile.TemporaryDirectory(prefix="photara-ll1-constraints-", dir="/private/tmp") as root:
        pg = Postgres(root)
        try:
            version = pg.scalar("SHOW server_version")
            counts = compare(pg)
            print(json.dumps({"version": version, "passed": counts, "total": sum(counts.values()), "root": root, "tcp": "disabled"}), flush=True)
        finally:
            pg.close()
    print(json.dumps({"temporary_cluster": "stopped and removed", "root": root}), flush=True)


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as failure:
        print(json.dumps({"subprocess_failed": failure.cmd, "stderr": failure.stderr, "stdout": failure.stdout}), flush=True)
        raise
