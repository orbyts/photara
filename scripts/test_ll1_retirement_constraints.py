#!/usr/bin/env python3
"""Disposable LL1 constraint experiment; never loads production migrations.

SQLite is in-memory. --postgres-disposable starts an exclusively owned temporary
cluster on its private Unix socket with TCP disabled; accepts no connection URL.
Tiny integer identities and simplified payloads are NOT the proposed wire/schema.
"""

import argparse
import json
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile


FAMILIES = {
    "slot": ("storage_slots", "storage_slot_names", "slot_id"),
    "variable": ("library_variables", "library_variable_names", "variable_id"),
}


def schema(engine, family, action):
    parent, names, key = FAMILIES[family]
    strict = " STRICT" if engine == "sqlite" else ""
    tx = "" if engine == "sqlite" else ", transaction_id BIGINT NOT NULL"
    backlink = (
        f"CONSTRAINT current_name_fk FOREIGN KEY(library_id,{key},current_name) "
        f"REFERENCES {names}(library_id,{key},name) ON DELETE {action} "
        "DEFERRABLE INITIALLY DEFERRED"
    )
    # Exact relevant FK actions/column ordering from local 0008/0009 and PG
    # 0009/0010. Most production columns, indexes, incoming edges and guards omitted.
    sql = f"""
CREATE TABLE libraries(library_id INTEGER PRIMARY KEY, payload TEXT NOT NULL){strict};
CREATE TABLE {parent}(
  {key} INTEGER PRIMARY KEY, library_id INTEGER NOT NULL,
  current_name TEXT NOT NULL, payload TEXT NOT NULL,
  UNIQUE(library_id,{key}),
  FOREIGN KEY(library_id) REFERENCES libraries ON DELETE RESTRICT
  {',' + backlink if engine == 'sqlite' else ''}
){strict};
CREATE TABLE {names}(
  library_id INTEGER NOT NULL, name TEXT NOT NULL, {key} INTEGER NOT NULL,
  PRIMARY KEY(library_id,name), UNIQUE(library_id,{key},name),
  FOREIGN KEY(library_id,{key}) REFERENCES {parent}(library_id,{key})
    ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
  FOREIGN KEY(library_id) REFERENCES libraries ON DELETE RESTRICT
){strict};
{f'ALTER TABLE {parent} ADD {backlink};' if engine == 'postgres' else ''}
CREATE TABLE lifecycle_commit_guard(guard_id INTEGER PRIMARY KEY CHECK(guard_id=0)){strict};
CREATE TABLE retirement_work(
  operation_id INTEGER PRIMARY KEY, library_id INTEGER NOT NULL,
  debt_id INTEGER NOT NULL CHECK(debt_id=1){tx},
  CONSTRAINT work_debt FOREIGN KEY(debt_id) REFERENCES lifecycle_commit_guard
    ON DELETE NO ACTION DEFERRABLE INITIALLY DEFERRED
){strict};
CREATE TABLE retirement_rows(
  operation_id INTEGER NOT NULL REFERENCES retirement_work ON DELETE RESTRICT,
  table_tag TEXT NOT NULL CHECK(table_tag IN ('libraries','{parent}','{names}')),
  library_id INTEGER NOT NULL, object_id INTEGER NOT NULL, row_name TEXT NOT NULL,
  PRIMARY KEY(operation_id,table_tag,library_id,object_id,row_name)
){strict};
CREATE TABLE lifecycle_receipts(
  authority_id INTEGER NOT NULL, account_id INTEGER NOT NULL,
  operation_id INTEGER NOT NULL, historical_library_id INTEGER NOT NULL,
  result TEXT NOT NULL CHECK(result IN ('removed','renamed')),
  PRIMARY KEY(authority_id,account_id,operation_id),
  UNIQUE(authority_id,account_id,operation_id,historical_library_id,result)
){strict};
CREATE TABLE removed_library_identities(
  authority_id INTEGER NOT NULL, account_id INTEGER NOT NULL,
  operation_id INTEGER NOT NULL, historical_library_id INTEGER NOT NULL,
  result TEXT NOT NULL CHECK(result='removed'),
  PRIMARY KEY(authority_id,historical_library_id),
  FOREIGN KEY(authority_id,account_id,operation_id,historical_library_id,result)
    REFERENCES lifecycle_receipts(authority_id,account_id,operation_id,historical_library_id,result)
    ON DELETE NO ACTION DEFERRABLE INITIALLY DEFERRED
){strict};
"""
    # Ordinary denial plus exact typed row admission; not an authentication facade.
    for table, object_field, row_name in (
        ("libraries", "0", "''"),
        (parent, f"OLD.{key}", "''"),
        (names, f"OLD.{key}", "OLD.name"),
    ):
        exists = f"""SELECT 1 FROM retirement_work w JOIN retirement_rows m
          ON m.operation_id=w.operation_id WHERE w.library_id=OLD.library_id
          AND m.library_id=OLD.library_id AND m.table_tag='{table}'
          AND m.object_id={object_field} AND m.row_name={row_name}
          {'AND w.transaction_id=txid_current()' if engine == 'postgres' else ''}"""
        if engine == "sqlite":
            sql += f"""
CREATE TRIGGER guard_{table} BEFORE DELETE ON {table}
WHEN NOT EXISTS({exists}) BEGIN SELECT RAISE(ABORT,'retirement_guard'); END;
"""
        else:
            sql += f"""
CREATE FUNCTION guard_{table}() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN IF NOT EXISTS({exists}) THEN
  RAISE EXCEPTION 'retirement_guard' USING ERRCODE='P0001';
END IF; RETURN OLD; END $$;
CREATE TRIGGER guard_{table} BEFORE DELETE ON {table}
FOR EACH ROW EXECUTE FUNCTION guard_{table}();
"""
    for table in ("lifecycle_receipts", "removed_library_identities"):
        for event in ("UPDATE", "DELETE"):
            name = f"immutable_{table}_{event.lower()}"
            if engine == "sqlite":
                sql += f"CREATE TRIGGER {name} BEFORE {event} ON {table} BEGIN SELECT RAISE(ABORT,'terminal_immutable'); END;\n"
            else:
                sql += f"""
CREATE FUNCTION {name}() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'terminal_immutable' USING ERRCODE='P0001'; END $$;
CREATE TRIGGER {name} BEFORE {event} ON {table} FOR EACH ROW EXECUTE FUNCTION {name}();
"""
    condition = "EXISTS(SELECT 1 FROM removed_library_identities WHERE authority_id=1 AND historical_library_id=NEW.library_id)"
    if engine == "sqlite":
        sql += f"CREATE TRIGGER prevent_reuse BEFORE INSERT ON libraries WHEN {condition} BEGIN SELECT RAISE(ABORT,'removed_identity'); END;\n"
    else:
        sql += f"""
CREATE FUNCTION prevent_reuse() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN IF {condition} THEN RAISE EXCEPTION 'removed_identity'; END IF; RETURN NEW; END $$;
CREATE TRIGGER prevent_reuse BEFORE INSERT ON libraries FOR EACH ROW EXECUTE FUNCTION prevent_reuse();
"""
    sql += f"""
BEGIN;
INSERT INTO libraries VALUES(1,'target'),(2,'unrelated-exact-payload');
INSERT INTO {parent} VALUES(11,1,'primary','target-parent'),(22,2,'other','unrelated-parent');
INSERT INTO {names} VALUES(1,'primary',11),(2,'other',22);
COMMIT;
"""
    return sql


def permit(engine, family, *, wrong_library=False, wrong_key=False, wrong_tx=False):
    parent, names, _ = FAMILIES[family]
    tx = "" if engine == "sqlite" else f",{0 if wrong_tx else 'txid_current()'}"
    library = 2 if wrong_library else 1
    object_id = 99 if wrong_key else 11
    return [
        "BEGIN",
        f"INSERT INTO retirement_work VALUES(10,{library},1{tx})",
        f"INSERT INTO retirement_rows VALUES(10,'libraries',{library},0,''),"
        f"(10,'{parent}',{library},{object_id},''),(10,'{names}',{library},{object_id},'primary')",
    ]


def deletion(family):
    parent, names, _ = FAMILIES[family]
    return [
        f"DELETE FROM {names} WHERE library_id=1",
        f"DELETE FROM {parent} WHERE library_id=1",
        "DELETE FROM libraries WHERE library_id=1",
    ]


def terminal(marker="1,7,10,1,'removed'", receipt="1,7,10,1,'removed'"):
    return [
        f"INSERT INTO lifecycle_receipts VALUES({receipt})",
        f"INSERT INTO removed_library_identities VALUES({marker})",
    ]


def finish(engine):
    # Validate domain constraints while permit exists, excluding intentional debt.
    return (["SET CONSTRAINTS current_name_fk IMMEDIATE"] if engine == "postgres" else []) + [
        "DELETE FROM retirement_rows", "DELETE FROM retirement_work", "COMMIT"
    ]


class SQLite:
    name = "sqlite"

    def __init__(self):
        self.connection = None

    def reset(self, family, action):
        if self.connection:
            self.connection.close()
        self.connection = sqlite3.connect(":memory:", isolation_level=None)
        self.connection.execute("PRAGMA foreign_keys=ON")
        assert self.connection.execute("PRAGMA foreign_keys").fetchone() == (1,)
        self.connection.executescript(schema(self.name, family, action))

    def run(self, statements, error=None):
        failed = None
        try:
            for statement in statements:
                self.connection.execute(statement)
        except sqlite3.DatabaseError as exception:
            failed = str(exception)
            if self.connection.in_transaction:
                self.connection.execute("ROLLBACK")
        check_error(failed, error)

    def scalar(self, sql):
        return str(self.connection.execute(sql).fetchone()[0])

    def fk_check(self):
        assert self.connection.execute("PRAGMA foreign_key_check").fetchall() == []

    def close(self):
        if self.connection:
            self.connection.close()


class Postgres:
    name = "postgres"

    def __init__(self, root):
        self.root = Path(root).resolve(strict=True)
        if not str(self.root).startswith("/private/tmp/photara-ll1-constraints-"):
            raise RuntimeError("Refusing unexpected disposable root")
        self.data = self.root / "data"
        self.socket = self.root / "socket"
        self.socket.mkdir(mode=0o700)
        self.env = {k: v for k, v in os.environ.items() if not k.startswith("PG")}
        self.started = False
        for binary in ("initdb", "pg_ctl", "psql"):
            if not shutil.which(binary):
                raise RuntimeError(f"Missing installed {binary}; no installation attempted")
        subprocess.run(
            ["initdb", "-D", str(self.data), "-U", "ll1_fixture", "--auth=trust", "--no-instructions"],
            check=True, capture_output=True, text=True, env=self.env,
        )
        subprocess.run(
            ["pg_ctl", "-D", str(self.data), "-l", str(self.root / "postgres.log"), "-w", "start",
             "-o", f"-F -h '' -k {self.socket} -p 55439"],
            check=True, capture_output=True, text=True, env=self.env,
        )
        self.started = True
        try:
            assert self.scalar("SHOW listen_addresses") == ""
            assert self.scalar("SHOW unix_socket_directories") == str(self.socket)
        except BaseException:
            self.close()
            raise

    def execute(self, sql):
        return subprocess.run(
            ["psql", "-X", "-qAt", "-v", "ON_ERROR_STOP=1", "-h", str(self.socket),
             "-p", "55439", "-U", "ll1_fixture", "-d", "postgres"],
            input="SET search_path=ll1_fixture,pg_catalog;\n" + sql,
            capture_output=True, text=True, env=self.env,
        )

    def reset(self, family, action):
        result = self.execute("DROP SCHEMA IF EXISTS ll1_fixture CASCADE; CREATE SCHEMA ll1_fixture;\n" + schema(self.name, family, action))
        assert result.returncode == 0, result.stderr

    def run(self, statements, error=None):
        result = self.execute(";\n".join(statements) + ";")
        check_error(result.stderr if result.returncode else None, error)

    def scalar(self, sql):
        result = self.execute(sql + ";")
        assert result.returncode == 0, result.stderr
        return result.stdout.strip()

    def fk_check(self):
        assert self.scalar("SELECT count(*) FROM pg_constraint WHERE connamespace='ll1_fixture'::regnamespace AND NOT convalidated") == "0"

    def close(self):
        # Both shutdown target and later TemporaryDirectory cleanup are the exact
        # generated root, not a PG environment variable or existing service path.
        assert self.data.parent.resolve(strict=True) == self.root
        assert self.socket.parent.resolve(strict=True) == self.root
        if self.started:
            subprocess.run(["pg_ctl", "-D", str(self.data), "-w", "-m", "fast", "stop"],
                           check=True, capture_output=True, text=True, env=self.env)
            self.started = False


def check_error(actual, expected):
    if expected is None:
        assert actual is None, actual
    else:
        assert actual is not None and expected.lower() in actual.lower(), (expected, actual)


def state_check(backend, family, removed):
    parent, names, key = FAMILIES[family]
    for table in ("libraries", parent, names):
        assert backend.scalar(f"SELECT count(*) FROM {table} WHERE library_id=1") == ("0" if removed else "1")
    assert backend.scalar("SELECT payload FROM libraries WHERE library_id=2") == "unrelated-exact-payload"
    assert backend.scalar(f"SELECT payload FROM {parent} WHERE library_id=2 AND {key}=22 AND current_name='other'") == "unrelated-parent"
    assert backend.scalar(f"SELECT count(*) FROM {names} WHERE library_id=2 AND {key}=22 AND name='other'") == "1"
    for table in ("retirement_work", "retirement_rows"):
        assert backend.scalar(f"SELECT count(*) FROM {table}") == "0"
    for table in ("lifecycle_receipts", "removed_library_identities"):
        assert backend.scalar(f"SELECT count(*) FROM {table}") == ("1" if removed else "0")
    backend.fk_check()


def cases(backend, family):
    engine = backend.name
    parent, names, _ = FAMILIES[family]
    fk = "FOREIGN KEY constraint failed" if engine == "sqlite" else "foreign key constraint"
    tests = [
        ("baseline_names_first_restrict", "RESTRICT", permit(engine, family) + [deletion(family)[0]], fk),
        ("baseline_parent_first_restrict", "RESTRICT", permit(engine, family) + [deletion(family)[1]], fk),
        ("ordinary_name_delete_guarded", "NO ACTION", ["BEGIN", deletion(family)[0]], "retirement_guard"),
        ("ordinary_parent_delete_guarded", "NO ACTION", ["BEGIN", deletion(family)[1]], "retirement_guard"),
        ("ordinary_root_delete_guarded", "NO ACTION", ["BEGIN", deletion(family)[2]], "retirement_guard"),
        ("wrong_library_manifest", "NO ACTION", permit(engine, family, wrong_library=True) + [deletion(family)[0]], "retirement_guard"),
        ("wrong_row_manifest", "NO ACTION", permit(engine, family, wrong_key=True) + [deletion(family)[0]], "retirement_guard"),
        ("wrong_name_manifest", "NO ACTION", permit(engine, family) + [f"UPDATE retirement_rows SET row_name='wrong' WHERE table_tag='{names}'", deletion(family)[0]], "retirement_guard"),
        ("wrong_table_manifest", "NO ACTION", permit(engine, family) + [f"DELETE FROM retirement_rows WHERE table_tag='{names}'", deletion(family)[0]], "retirement_guard"),
        ("permit_debt_cannot_commit", "NO ACTION", permit(engine, family) + ["COMMIT"], fk),
        ("permit_debt_rolls_back_deleted_aggregate", "NO ACTION", permit(engine, family) + deletion(family) + terminal() + ["COMMIT"], fk),
        ("debt_parent_cannot_be_forged", "NO ACTION", ["INSERT INTO lifecycle_commit_guard VALUES(1)"], "check constraint"),
        ("incomplete_closure_cannot_commit", "NO ACTION", permit(engine, family) + [deletion(family)[0]] + finish(engine), fk),
    ]
    if engine == "postgres":
        tests.append(("wrong_transaction_manifest", "NO ACTION", permit(engine, family, wrong_tx=True) + [deletion(family)[0]], "retirement_guard"))
    for field, marker in (
        ("authority", "2,7,10,1,'removed'"),
        ("actor", "1,8,10,1,'removed'"),
        ("operation", "1,7,11,1,'removed'"),
        ("library", "1,7,10,2,'removed'"),
    ):
        tests.append((f"marker_{field}_mismatch", "NO ACTION", permit(engine, family) + deletion(family) + terminal(marker=marker) + finish(engine), fk))
    tests.append(("marker_result_mismatch", "NO ACTION", permit(engine, family) + deletion(family) + terminal(receipt="1,7,10,1,'renamed'") + finish(engine), fk))
    for name, action, statements, error in tests:
        backend.reset(family, action)
        backend.run(statements, error)
        state_check(backend, family, removed=False)
        print(json.dumps({"engine": engine, "family": family, "case": name, "result": "pass"}), flush=True)
    backend.reset(family, "NO ACTION")
    backend.run(permit(engine, family) + deletion(family) + terminal() + finish(engine))
    state_check(backend, family, removed=True)
    backend.run([f"DELETE FROM {names} WHERE library_id=2"], "retirement_guard")
    backend.run([f"DELETE FROM {parent} WHERE library_id=2"], "retirement_guard")
    for table in ("lifecycle_receipts", "removed_library_identities"):
        backend.run([f"DELETE FROM {table}"], "terminal_immutable")
        backend.run([f"UPDATE {table} SET account_id=8"], "terminal_immutable")
    backend.run(["INSERT INTO libraries VALUES(1,'resurrection')"], "removed_identity")
    state_check(backend, family, removed=True)
    print(json.dumps({"engine": engine, "family": family, "case": "admitted_atomic_closure_and_postcommit_guards", "result": "pass"}), flush=True)
    return len(tests) + 1


def main():
    if not __debug__:
        raise RuntimeError("Assertions must be enabled for this test harness")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--postgres-disposable", action="store_true")
    args = parser.parse_args()
    sqlite = SQLite()
    try:
        count = sum(cases(sqlite, family) for family in FAMILIES)
        print(json.dumps({"engine": "sqlite", "version": sqlite3.sqlite_version, "passed": count}))
    finally:
        sqlite.close()
    if args.postgres_disposable:
        with tempfile.TemporaryDirectory(prefix="photara-ll1-constraints-", dir="/private/tmp") as root:
            pg = Postgres(root)
            try:
                version = pg.scalar("SHOW server_version")
                count = sum(cases(pg, family) for family in FAMILIES)
                print(json.dumps({"engine": "postgres", "version": version, "passed": count,
                                  "temporary_root": root, "tcp": "disabled"}))
            finally:
                pg.close()
        print(json.dumps({"temporary_cluster": "stopped and removed", "root": root}))


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as failure:
        print(json.dumps({"subprocess_failed": failure.cmd, "stderr": failure.stderr,
                          "stdout": failure.stdout}), flush=True)
        raise
