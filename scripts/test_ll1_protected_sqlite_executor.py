#!/usr/bin/env python3
"""Disposable actual SQLite schema executor experiment; never opens user databases.

The protected guard overlay and receipt are prototypes, not migration inputs.
Only fresh TemporaryDirectory databases are opened. FK actions are unchanged.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3
import sys
import tempfile

sys.dont_write_bytecode = True
from test_ll1_actual_sqlite_baseline import install, catalog, seed, snapshot, bid, MIGRATIONS


LOGICAL = ('device_context_snapshots', 'legacy_external_resource_resolutions')
CYCLES = (('storage_slots', 'storage_slot_names'),
          ('library_variables', 'library_variable_names'))
SUPPORTED = {'libraries', 'library_contract_state', 'people', 'project_ownership',
             'project_catalog', 'project_locators', 'storage_roots', 'storage_slots',
             'storage_slot_names', 'library_variables', 'library_variable_names',
             'device_root_bindings', *LOGICAL}


class Refused(Exception):
    pass


class Fixture:
    def __init__(self, path):
        self.db = sqlite3.connect(path, isolation_level=None, timeout=0)
        self.db.execute('PRAGMA foreign_keys=ON')
        self.hashes, declarations = install(self.db)
        self.source, self.measured = catalog(self.db, declarations)
        target, _, _ = seed(self.db)
        self.target = bytes.fromhex(target[2:-1])
        self.other = bid(2)
        self.actor = object()  # trusted caller identity fixture, not an auth implementation
        self.permit = None
        self.db.create_function('ll1_permits', 2, self.permits)
        self.db.create_function('ll1_active', 0, lambda: self.permit is not None)
        self.clone_unrelated()
        self.default_library = self.target
        self.target, self.other = self.other, self.target
        self.tables_sql = list(self.db.execute("SELECT name,sql FROM sqlite_schema WHERE type='table'"))
        self.overlay()
        self.initial = snapshot(self.db, self.source['tables'])

    def clone_unrelated(self):
        # Duplicate the seeded aggregate with disjoint typed IDs; retain shared device.
        order = ('people', 'project_ownership', 'project_catalog', 'project_locators',
                 'storage_roots', 'storage_slots', 'storage_slot_names',
                 'library_variables', 'library_variable_names', 'device_root_bindings', *LOGICAL)
        def remap(value):
            if value == self.target:
                return self.other
            if isinstance(value, bytes) and len(value) == 16 and value != bid(10):
                return bid(int.from_bytes(value, 'big') + 1000)
            return value
        self.db.execute('BEGIN')
        for table in order:
            for row in self.db.execute(f'SELECT * FROM {table}').fetchall():
                self.db.execute(f'INSERT INTO {table} VALUES ({",".join("?" for _ in row)})', tuple(map(remap, row)))
        self.db.execute('COMMIT')
        assert not self.db.execute('PRAGMA foreign_key_check').fetchall()

    def overlay(self):
        # Only deletion rejection bodies are qualified. All original UPDATE,
        # admission, immutable and CAS triggers remain byte-for-byte unchanged.
        for name, table, sql in self.db.execute("SELECT name,tbl_name,sql FROM sqlite_schema WHERE type='trigger'").fetchall():
            if 'BEFORE DELETE ON' not in sql:
                continue
            assert sql.count('BEGIN') == 1
            head, body = sql.split('BEGIN', 1)
            condition = f"NOT ll1_permits('{table}',OLD.rowid)"
            if 'WHEN' in head:
                prefix, existing = head.split('WHEN', 1)
                head = prefix + 'WHEN (' + existing.strip() + ') AND ' + condition + ' '
            else:
                head += ' WHEN ' + condition + ' '
            self.db.execute(f'DROP TRIGGER {name}')
            self.db.execute(head + 'BEGIN' + body)
        for table in self.source['tables']:
            self.db.execute(f"CREATE TRIGGER ll1_scope_{table} BEFORE DELETE ON {table} WHEN ll1_active() AND NOT ll1_permits('{table}',OLD.rowid) BEGIN SELECT RAISE(ABORT,'manifest_scope'); END")
        self.db.execute('CREATE TABLE ll1_terminal(operation TEXT PRIMARY KEY, library BLOB NOT NULL UNIQUE, request TEXT NOT NULL, result TEXT NOT NULL CHECK(result=\'removed\'), UNIQUE(operation,library,request)) STRICT')
        self.db.execute('CREATE TABLE ll1_marker(library BLOB PRIMARY KEY, operation TEXT NOT NULL, request TEXT NOT NULL, FOREIGN KEY(operation,library,request) REFERENCES ll1_terminal(operation,library,request) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED) STRICT')
        self.db.execute('CREATE TABLE ll1_inventory(position INTEGER PRIMARY KEY, library BLOB NOT NULL UNIQUE, operation TEXT NOT NULL, request TEXT NOT NULL, FOREIGN KEY(operation,library,request) REFERENCES ll1_terminal(operation,library,request) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED) STRICT')
        for table in ('ll1_terminal', 'll1_marker', 'll1_inventory'):
            for event in ('UPDATE', 'DELETE'):
                self.db.execute(f"CREATE TRIGGER {table}_{event} BEFORE {event} ON {table} BEGIN SELECT RAISE(ABORT,'terminal_immutable'); END")
        assert self.tables_sql == list(self.db.execute("SELECT name,sql FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'll1_%'"))

    def permits(self, table, rowid):
        return self.permit is not None and self.db.in_transaction and (table, rowid) in self.permit

    def closure(self):
        rows = {}
        for table, info in self.source['tables'].items():
            cols = ['rowid'] + [c[1] for c in self.db.execute(f'PRAGMA table_info({table})')]
            rows[table] = [dict(zip(cols, row)) for row in self.db.execute(f'SELECT rowid,* FROM {table}')]
        selected = {('libraries', r['rowid']) for r in rows['libraries'] if r['library_id'] == self.target}
        while True:
            before = len(selected)
            for edge in self.source['fks']:
                parents = [r for r in rows[edge['parent']] if (edge['parent'], r['rowid']) in selected]
                for child in rows[edge['child']]:
                    values = tuple(child[c] for c in edge['columns'])
                    if None in values:
                        continue
                    if any(values == tuple(p[c] for c in edge['parent_columns']) for p in parents):
                        selected.add((edge['child'], child['rowid']))
            if len(selected) == before:
                break
        projects = {r['project_id'] for r in rows['project_ownership'] if ('project_ownership', r['rowid']) in selected}
        for table in LOGICAL:
            selected.update((table, r['rowid']) for r in rows[table] if r['project_id'] in projects)
        # Explicitly verify every direct Library-bearing row, independently of FK walk.
        for table, records in rows.items():
            for row in records:
                if row.get('library_id') == self.target:
                    assert (table, row['rowid']) in selected
                if row.get('library_id') == self.other:
                    assert (table, row['rowid']) not in selected
        return selected

    def execute(self, actor=None, operation='remove-1', revision=1, fault=None):
        if actor is not self.actor:
            raise Refused('authorization')
        request = hashlib.sha256(self.target + str(revision).encode()).hexdigest()
        self.db.execute('BEGIN IMMEDIATE')
        try:
            receipt = self.db.execute('SELECT request,result FROM ll1_terminal WHERE operation=?', (operation,)).fetchone()
            if receipt:
                if receipt[0] != request:
                    raise Refused('operation_reuse')
                self.db.execute('COMMIT')
                return receipt[1]
            current = self.db.execute('SELECT local_revision FROM libraries WHERE library_id=?', (self.target,)).fetchone()
            if current is None or current[0] != revision:
                raise Refused('stale_revision')
            if self.db.execute('SELECT count(*) FROM libraries').fetchone()[0] < 2:
                raise Refused('last_library')
            if self.target == self.default_library:
                raise Refused('default_library')
            manifest = self.closure()
            if any(table not in SUPPORTED for table, _ in manifest):
                raise Refused('unverified_table_disposition')
            before = {t: {r[0]: repr(r[1:]) for r in self.db.execute(f'SELECT rowid,* FROM {t}')} for t in self.source['tables']}
            survivors = {t: sorted(value for rowid, value in rows.items() if (t, rowid) not in manifest) for t, rows in before.items()}
            self.permit = manifest.copy()
            if fault == 'wrong_manifest':
                self.permit.remove(next(x for x in manifest if x[0] == 'people'))
            if fault == 'other_library':
                self.db.execute('DELETE FROM libraries WHERE library_id=?', (self.other,))
            # A valid syntax name intentionally has no matching name row. Only
            # these existing deferred backlinks become temporarily unsatisfied.
            for parent, _ in CYCLES:
                self.db.execute(f"UPDATE {parent} SET current_name='ll1_unbound',local_revision=local_revision+1 WHERE library_id=?", (self.target,))
            if fault == 'observe_unbound':
                self.observer()
                raise Refused('observed_isolation')
            if fault == 'commit_unbound':
                self.db.execute('COMMIT')
            if fault == 'after_rename':
                raise Refused('injected_after_rename')
            remaining = manifest.copy()
            # Retrying RESTRICT refusals derives ordering from actual constraints;
            # no FK is disabled, globally deferred, dropped or rewritten.
            while remaining:
                progress = False
                for table, rowid in sorted(remaining):
                    if fault == 'omit_logical' and table in LOGICAL:
                        continue
                    try:
                        self.db.execute(f'DELETE FROM {table} WHERE rowid=?', (rowid,))
                    except sqlite3.IntegrityError as error:
                        if str(error) != 'FOREIGN KEY constraint failed':
                            raise
                        continue
                    remaining.remove((table, rowid))
                    progress = True
                if not progress:
                    raise Refused('incomplete_closure')
            if fault == 'after_delete':
                raise Refused('injected_after_delete')
            assert not self.db.execute('PRAGMA foreign_key_check').fetchall()
            assert not self.closure()
            # Exact set subtraction also covers logical rows once owner rows vanish.
            assert sum(len(v) for v in self.initial.values()) - sum(len(v) for v in snapshot(self.db, self.source['tables']).values()) == len(manifest)
            assert snapshot(self.db, self.source['tables']) == survivors
            self.db.execute('INSERT INTO ll1_terminal VALUES(?,?,?,?)', (operation, self.target, request, 'removed'))
            if fault == 'after_receipt':
                raise Refused('injected_after_receipt')
            marker_request = 'wrong-request' if fault == 'marker_mismatch' else request
            if fault != 'omit_marker':
                self.db.execute('INSERT INTO ll1_marker VALUES(?,?,?)', (self.target, operation, marker_request))
            inventory_request = 'wrong-request' if fault == 'inventory_mismatch' else request
            self.db.execute('INSERT INTO ll1_inventory VALUES(1,?,?,?)', (self.target, operation, inventory_request))
            if self.db.execute('SELECT count(*) FROM ll1_terminal t JOIN ll1_marker m USING(library,operation,request) JOIN ll1_inventory i USING(library,operation,request)').fetchone() != (1,):
                raise Refused('terminal_disagreement')
            assert not self.db.execute('PRAGMA foreign_key_check').fetchall()
            self.db.execute('COMMIT')
            return 'removed'
        except BaseException:
            if self.db.in_transaction:
                self.db.execute('ROLLBACK')
            raise
        finally:
            self.permit = None


def main():
    argparse.ArgumentParser(description=__doc__).parse_args()
    if not __debug__:
        raise RuntimeError('Assertions required')
    passed = []
    def report(name):
        passed.append(name)
        print(json.dumps({'case': name, 'result': 'pass'}), flush=True)
    with tempfile.TemporaryDirectory(prefix='ll1-protected-sqlite-') as tmp:
        f = Fixture(str(Path(tmp) / 'disposable.sqlite'))
        db = f.db
        manifest = f.closure()
        initial_rows = {t: {r[0]: repr(r[1:]) for r in db.execute(f'SELECT rowid,* FROM {t}')} for t in f.source['tables']}
        survivors = {t: sorted(value for rowid, value in rows.items() if (t, rowid) not in manifest) for t, rows in initial_rows.items()}
        def refused(name, call, message):
            try:
                call()
            except (sqlite3.DatabaseError, Refused) as error:
                assert message in str(error), (name, error)
            else:
                raise AssertionError(name)
            assert snapshot(db, f.source['tables']) == f.initial
            assert db.execute('SELECT count(*) FROM ll1_terminal').fetchone() == (0,)
            assert db.execute('SELECT count(*) FROM ll1_marker').fetchone() == (0,)
            assert db.execute('SELECT count(*) FROM ll1_inventory').fetchone() == (0,)
            assert f.permit is None and not db.in_transaction
            assert db.execute('PRAGMA foreign_keys').fetchone() == (1,)
            assert db.execute('PRAGMA defer_foreign_keys').fetchone() == (0,)
            report(name)
        refused('authorization', lambda: f.execute(object()), 'authorization')
        refused('stale_revision', lambda: f.execute(f.actor, revision=2), 'stale_revision')
        refused('ordinary_guard', lambda: db.execute('DELETE FROM libraries WHERE library_id=?', (f.target,)), 'hard_delete_not_supported')
        for fault, message in [('wrong_manifest','manifest_scope'), ('other_library','manifest_scope'),
                               ('after_rename','injected'), ('commit_unbound','FOREIGN KEY'),
                               ('omit_logical','incomplete_closure'), ('after_delete','injected'), ('after_receipt','injected'),
                               ('marker_mismatch','terminal_disagreement'), ('inventory_mismatch','terminal_disagreement'),
                               ('omit_marker','terminal_disagreement')]:
            refused(fault, lambda fault=fault: f.execute(f.actor, fault=fault), message)
        competitor = sqlite3.connect(str(Path(tmp) / 'disposable.sqlite'), isolation_level=None, timeout=0)
        competitor.execute('PRAGMA foreign_keys=ON')
        def observer():
            for parent, _ in CYCLES:
                assert db.execute(f"SELECT count(*) FROM {parent} WHERE current_name='ll1_unbound'").fetchone() == (2,)
                assert competitor.execute(f"SELECT count(*) FROM {parent} WHERE current_name='ll1_unbound'").fetchone() == (0,)
            try:
                competitor.execute('BEGIN IMMEDIATE')
            except sqlite3.OperationalError as error:
                assert 'locked' in str(error)
            else:
                raise AssertionError('concurrent writer was admitted')
        f.observer = observer
        refused('unbound_names_invisible_and_writer_blocked', lambda: f.execute(f.actor, fault='observe_unbound'), 'observed_isolation')
        competitor.execute('BEGIN IMMEDIATE')
        refused('concurrent_writer_busy', lambda: f.execute(f.actor), 'locked')
        competitor.execute('ROLLBACK')
        assert f.execute(f.actor) == 'removed'
        assert snapshot(db, f.source['tables']) == survivors
        report('exact_complete_seeded_closure_and_unrelated_preservation')
        assert f.execute(f.actor) == 'removed'
        report('terminal_idempotent_retry')
        for name, call, message in [('terminal_request_conflict', lambda: f.execute(f.actor, revision=2), 'operation_reuse'),
                                    ('concurrent_loser_new_operation', lambda: f.execute(f.actor, operation='remove-2'), 'stale_revision'),
                                    ('terminal_immutable', lambda: db.execute('DELETE FROM ll1_terminal'), 'terminal_immutable'),
                                    ('guard_after_success', lambda: db.execute('DELETE FROM libraries WHERE library_id=?', (f.other,)), 'hard_delete_not_supported')]:
            before = snapshot(db, f.source['tables'])
            try:
                call()
            except (Refused, sqlite3.DatabaseError) as error:
                assert message in str(error), (name, error)
            else:
                raise AssertionError(name)
            assert snapshot(db, f.source['tables']) == before
            report(name)
        assert db.execute('PRAGMA foreign_key_check').fetchall() == []
        assert f.hashes == {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in MIGRATIONS.glob('*.sql')}
        print(json.dumps({'sqlite': sqlite3.sqlite_version, 'catalog': f.measured, 'migrations':len(f.hashes), 'seeded_manifest_rows':len(manifest), 'seeded_manifest_tables':len({t for t,_ in manifest}), 'passed':len(passed), 'constraints':'unchanged; foreign_keys=ON; defer_foreign_keys=OFF', 'scope':'disposable executor diagnostic; guard overlay and trusted caller model; review required'}), flush=True)
        competitor.close()
        db.close()


if __name__ == '__main__':
    main()
