#!/usr/bin/env python3
"""Disposable retained-chain mechanics, with explicit fixture-only evidence format."""
import hashlib
import json
from pathlib import Path
import sqlite3
import sys
import tempfile

sys.dont_write_bytecode = True
import test_ll1_protected_sqlite_executor as base
from test_ll1_protected_sqlite_coverage import seed_expanded, ADDITIONAL
from test_ll1_actual_sqlite_baseline import bid, snapshot, MIGRATIONS


class RetainedFixture(base.Fixture):
    def __init__(self, path):
        super().__init__(path)
        self.check_retained = False
        self.retirement = None
        self.expected_evidence = []
        self.evidence_fault = None
        self.db.execute('CREATE TABLE ll1_detached_evidence(retirement TEXT NOT NULL, library BLOB NOT NULL, source_table TEXT NOT NULL, source_rowid INTEGER NOT NULL, terminal_json TEXT NOT NULL, PRIMARY KEY(library,source_table,source_rowid)) STRICT')
        self.db.execute("CREATE TRIGGER ll1_evidence_terminal_agreement BEFORE INSERT ON ll1_inventory WHEN EXISTS(SELECT 1 FROM ll1_detached_evidence e WHERE NOT EXISTS(SELECT 1 FROM ll1_terminal t WHERE t.operation=e.retirement AND t.library=e.library)) BEGIN SELECT RAISE(ABORT,'detached_terminal_disagreement'); END")
        self.db.create_function('ll1_evidence_complete', 0, lambda: sorted(self.expected_evidence) == sorted(self.db.execute('SELECT * FROM ll1_detached_evidence').fetchall()))
        self.db.execute("CREATE TRIGGER ll1_evidence_complete BEFORE INSERT ON ll1_inventory WHEN NOT ll1_evidence_complete() BEGIN SELECT RAISE(ABORT,'detached_evidence_incomplete'); END")
        for event in ('UPDATE', 'DELETE'):
            self.db.execute(f"CREATE TRIGGER ll1_evidence_{event} BEFORE {event} ON ll1_detached_evidence BEGIN SELECT RAISE(ABORT,'terminal_immutable'); END")

    def execute(self, *args, **kwargs):
        self.check_retained = True
        self.retirement = kwargs.get('operation', 'remove-1')
        self.evidence_fault = kwargs.get('fault')
        try:
            return super().execute(*args, **kwargs)
        finally:
            self.check_retained = False

    def closure(self):
        manifest = super().closure()
        if not getattr(self, 'check_retained', False) or not self.db.in_transaction:
            return manifest
        self.check_retained = False
        predicates = {
            'operation_intents': "state NOT IN ('committed','failed','cancelled')",
            'context_apply_intents': "state<>'settled'",
            'project_creation_intents': "state NOT IN ('complete','cancelled')",
            'onboarding_intents': "state NOT IN ('applied','cancelled') AND NOT EXISTS(SELECT 1 FROM onboarding_dispositions d WHERE d.operation_id=onboarding_intents.operation_id)",
        }
        for table, predicate in predicates.items():
            if self.db.execute(f'SELECT 1 FROM {table} WHERE library_id=? AND ({predicate})', (self.target,)).fetchone():
                raise base.Refused('pending_operation:' + table)
        # The disposable evidence codec retains explicit identity/digest/outcome
        # columns only. Production codec acceptance is deliberately not claimed.
        self.expected_evidence = []
        for table, rowid in sorted(manifest):
            if table not in EVIDENCE:
                continue
            columns = EVIDENCE[table]
            row = self.db.execute(f'SELECT {",".join(columns)} FROM {table} WHERE rowid=?', (rowid,)).fetchone()
            value = {c: {'hex': v.hex()} if isinstance(v, bytes) else v for c, v in zip(columns, row)}
            evidence = (self.retirement, self.target, table, rowid, json.dumps(value, sort_keys=True))
            self.expected_evidence.append(evidence)
            if self.evidence_fault == 'omit_detached' and table == 'operation_intents':
                continue
            if self.evidence_fault == 'mismatch_detached' and table == 'operation_intents':
                evidence = ('wrong-retirement', *evidence[1:])
            self.db.execute('INSERT INTO ll1_detached_evidence VALUES(?,?,?,?,?)', evidence)
        return manifest


EVIDENCE = {
    'mutations': ('mutation_id', 'envelope_sha256'),
    'remote_mutation_receipts': ('target_id', 'mutation_id', 'request_sha256', 'outcome', 'response_sha256', 'accepted_epoch', 'accepted_sequence'),
    'local_mutation_dispositions': ('mutation_id', 'disposition', 'replacement_mutation_id', 'reason_code'),
    'scoped_sync_operations': ('operation_id', 'channel_id', 'local_intent_sha256', 'request_sha256', 'receipt_sha256', 'state', 'replacement_operation_id'),
    'operation_intents': ('operation_id', 'idempotency_key', 'request_sha256', 'state', 'proposed_commit_id', 'proposed_commit_sha256'),
    'recovery_publications': ('recovery_item_id', 'project_id', 'commit_id', 'commit_sha256'),
    'context_apply_intents': ('operation_id', 'proposal_id', 'request_sha256', 'state'),
    'context_apply_receipts': ('receipt_id', 'operation_id', 'observation_kind', 'result_sha256', 'package_commit_id', 'package_commit_sha256'),
    'onboarding_intents': ('operation_id', 'database_id', 'device_id', 'environment_id', 'issuer', 'subject', 'command_sha256', 'state'),
    'onboarding_receipts': ('operation_id', 'receipt_sha256'),
    'onboarding_dispositions': ('operation_id', 'disposition', 'absence_sha256', 'session_generation'),
    'onboarding_replacements': ('operation_id', 'previous_operation_id'),
    'project_creation_intents': ('operation_id', 'project_id', 'request_sha256', 'state', 'cloud_receipt'),
}


def seed_retained(f):
    expected = seed_expanded(f)
    db = f.db
    digest = hashlib.sha256(b'{}').digest()
    target_scope = False
    def ins(table, **values):
        cur = db.execute(f'INSERT INTO {table} ({",".join(values)}) VALUES ({",".join("?" for _ in values)})', tuple(values.values()))
        if target_scope:
            expected.add((table, cur.lastrowid))
        return cur.lastrowid
    db.execute('BEGIN')
    db.execute("UPDATE onboarding_session SET generation=generation+1,issuer='fixture-issuer',subject='fixture-subject'")
    ins('onboarding_credential_references', environment_id='fixture', issuer='fixture-issuer', subject='fixture-subject', device_id=bid(10), credential_reference='opaque-fixture', device_commitment='a'*64)
    for library, offset in ((f.target, 1000), (f.other, 0)):
        target_scope = library == f.target
        key = lambda n: bid(n+offset)
        scope = dict(library_id=library)
        ins('sync_targets', **scope, target_id=key(300), account_id=bid(9000), backend='photara-cloud', environment_id='fixture', state='paused', created_at_ms=0, updated_at_ms=0)
        ins('mutations', **scope, mutation_id=key(301), source_device_id=bid(10), origin='local', command_schema=1, primary_entity_kind='person', primary_entity_id=key(201), created_at_ms=0, envelope_json='{}', envelope_sha256=digest)
        ins('local_changes', **scope, mutation_id=key(301), entity_kind='person', entity_id=key(201), local_revision=1, change_kind='create', changed_at_ms=0, post_state_json='{}', post_state_sha256=digest)
        ins('mutation_baselines', **scope, mutation_id=key(301), entity_kind='person', entity_id=key(201))
        ins('sync_object_state', **scope, target_id=key(300), entity_kind='person', entity_id=key(201), server_revision='1', synced_local_revision=1, base_snapshot_json='{}', base_snapshot_sha256=digest)
        sequence = ins('sync_outbox', **scope, target_id=key(300), mutation_id=key(301), state='rejected', not_before_ms=0, request_json='{}', request_sha256=digest)
        ins('sync_inbox', **scope, batch_id=key(302), target_id=key(300), stream_epoch=key(303), batch_sequence='1', cursor_after='1', payload_json='{}', payload_sha256=digest, received_at_ms=0, state='applied', applied_at_ms=0)
        ins('sync_cursors', **scope, target_id=key(300), applied_cursor='1', local_revision=1, updated_at_ms=0)
        ins('sync_conflicts', **scope, conflict_id=key(304), target_id=key(300), outbox_sequence=sequence, entity_kind='person', entity_id=key(201), local_json='{}', remote_json='{}', state='resolved', resolution_mutation_id=key(301), created_at_ms=0, resolved_at_ms=0)
        media_digest = db.execute('SELECT sha256 FROM library_media WHERE library_id=?', (library,)).fetchone()[0]
        ins('media_transfers', **scope, target_id=key(300), sha256=media_digest, direction='upload', state='complete', updated_at_ms=0)
        ins('remote_mutation_receipts', **scope, target_id=key(300), mutation_id=key(301), request_sha256=digest, outcome='rejected', response_json='{}', response_sha256=digest, received_at_ms=0)
        ins('local_mutation_dispositions', **scope, mutation_id=key(301), disposition='discarded', reason_code='fixture', created_at_ms=0)
        ins('sync_snapshot_installs', **scope, installation_id=key(305), target_id=key(300), server_snapshot_id=key(306), stream_epoch=key(303), high_water_cursor='1', payload_json='{}', payload_sha256=digest, state='installed', created_at_ms=0, installed_at_ms=0)
        ins('scoped_sync_channels', **scope, channel_id=key(307), account_id=bid(9000), environment_id='fixture', scope_kind='project', project_id=key(20), authorization_generation=1, epoch=key(303), applied_cursor='1', state='paused', local_revision=1, updated_at=0)
        ins('scoped_sync_operations', operation_id=key(308), channel_id=key(307), command_kind='fixture', local_intent_canonical=b'{}', local_intent_sha256=digest, sealed_request=b'{}', request_sha256=digest, receipt_canonical=b'{}', receipt_sha256=digest, state='acknowledged', created_at=0, updated_at=0)
        ins('scoped_sync_operation_roots', operation_id=key(308), channel_id=key(307), entity_kind='person', entity_id=key(201), local_poststate=b'{}', local_poststate_sha256=digest, local_revision=1)
        ins('scoped_sync_base', channel_id=key(307), entity_kind='person', entity_id=key(201), server_revision='1', poststate_canonical=b'{}', poststate_sha256=digest, observed_local_revision=1)
        ins('scoped_sync_inbox', inbox_id=key(309), channel_id=key(307), authorization_generation=1, epoch=key(303), batch_sequence=1, cursor_before='0', cursor_after='1', batch_canonical=b'{}', batch_sha256=digest, state='applied', received_at=0, applied_at=0)
        ins('scoped_sync_snapshot_installs', installation_id=key(310), channel_id=key(307), snapshot_id=key(311), authorization_generation=1, epoch=key(303), high_water_cursor='1', payload_canonical=b'{}', payload_sha256=digest, state='installed', created_at=0, installed_at=0)
        ins('operation_intents', **scope, operation_id=key(312), project_id=key(20), operation_kind='save-package', idempotency_key='fixture', request_json='{}', request_sha256=digest, proposed_commit_id=key(313), proposed_commit_sha256=digest, state='committed', local_revision=1, created_at_ms=0, updated_at_ms=0)
        ins('operation_events', operation_id=key(312), phase='committed', occurred_at_ms=0, details_json='{}')
        ins('recovery_items', recovery_item_id=key(314), operation_id=key(312), record_schema='fixture', payload_json='{}', payload_sha256=digest, durable_relative_path='recovery/untouched', blob_sha256=digest, byte_length=2, created_at_ms=0)
        ins('recovery_publications', recovery_item_id=key(314), project_id=key(20), commit_id=key(313), commit_sha256=digest, verified_at_ms=0)
        ins('context_apply_intents', **scope, operation_id=key(315), proposal_id=key(316), request_canonical=b'{}', request_sha256=digest, source_run_id=key(317), source_attempt_id=key(318), target_authority='library', state='settled', record_schema=1, local_revision=1, created_at=0, updated_at=0)
        ins('context_apply_receipts', **scope, receipt_id=key(319), operation_id=key(315), observation_kind='local-applied', result_canonical=b'{}', result_sha256=digest, observed_at=0)
        principal = db.execute('SELECT local_principal_id FROM library_contract_state WHERE library_id=?', (library,)).fetchone()[0]
        onboarding = dict(**scope, database_id=bid(42), device_id=bid(10), environment_id='fixture', expected_issuer='fixture-issuer', local_principal_id=principal, library_revision=1, contract_revision=1, command_canonical=b'{}', command_sha256=digest, credential_reference='opaque-fixture', device_commitment='a'*64, created_at=0)
        ins('onboarding_intents', **onboarding, operation_id=key(320), issuer='fixture-issuer', subject='fixture-subject', state='unknown')
        ins('onboarding_dispositions', operation_id=key(320), disposition='abandoned-expired', absence_canonical=b'{}', absence_sha256=digest, session_generation=1, observed_at=0)
        ins('onboarding_intents', **onboarding, operation_id=key(321), state='prepared')
        ins('onboarding_replacements', operation_id=key(321), previous_operation_id=key(320))
        db.execute("UPDATE onboarding_intents SET issuer='fixture-issuer',subject='fixture-subject',state='applied' WHERE operation_id=?", (key(321),))
        ins('onboarding_receipts', operation_id=key(321), receipt_canonical=b'{}', receipt_sha256=digest, received_at=0)
        ins('library_cloud_bindings', **scope, operation_id=key(321), environment_id='fixture', issuer='fixture-issuer', subject='fixture-subject', account_id=bid(9000), identity_id=key(322), access_canonical=b'{}', access_sha256=digest, observed_at=0, local_revision=1, state='signed-out', created_at=0)
        if target_scope:
            ins('onboarding_library_selection', **scope, singleton=1, operation_id=key(321), chosen_at=0)
        ins('project_creation_intents', **scope, operation_id=key(323), project_id=key(324), request_canonical='{}', request_sha256=digest, destination_key='opaque-destination-'+str(offset), state='complete', stage_pin='opaque-stage-never-opened', cloud_receipt='fixture-terminal-result', created_at_ms=0)
    db.execute('COMMIT')
    assert not db.execute('PRAGMA foreign_key_check').fetchall()
    f.initial = snapshot(db, f.source['tables'])
    assert f.closure() == expected
    assert all(f.initial.values()), 'all 79 source tables must be populated'
    return expected


def main():
    if not __debug__:
        raise RuntimeError('Assertions required')
    original_supported = base.SUPPORTED
    with tempfile.TemporaryDirectory(prefix='ll1-retained-sqlite-') as tmp:
        f = RetainedFixture(str(Path(tmp)/'disposable.sqlite'))
        try:
            manifest = seed_retained(f)
            # This broad whitelist is local to a populated mechanical experiment,
            # not a change to the base or production acceptance policy.
            base.SUPPORTED = set(f.source['tables'])
            initial = snapshot(f.db, f.source['tables'])
            survivors = {t: sorted(repr(r[1:]) for r in f.db.execute(f'SELECT rowid,* FROM {t}') if (t, r[0]) not in manifest) for t in f.source['tables']}
            def refused(fault, message):
                try:
                    f.execute(f.actor, fault=fault)
                except (sqlite3.DatabaseError, base.Refused) as error:
                    assert message in str(error), (fault, str(error))
                else:
                    raise AssertionError(fault)
                assert snapshot(f.db, f.source['tables']) == initial
                assert f.db.execute('SELECT count(*) FROM ll1_detached_evidence').fetchone() == (0,)
                assert all(f.db.execute(f'SELECT count(*) FROM {t}').fetchone() == (0,) for t in ('ll1_terminal', 'll1_marker', 'll1_inventory'))
                assert f.permit is None and not f.db.in_transaction
                assert f.db.execute('PRAGMA foreign_keys').fetchone() == (1,)
                assert f.db.execute('PRAGMA defer_foreign_keys').fetchone() == (0,)
            for fault, message in [('after_rename','injected'), ('commit_unbound','FOREIGN KEY'), ('omit_logical','incomplete_closure'), ('after_delete','injected'), ('after_receipt','injected'), ('marker_mismatch','terminal_disagreement'), ('omit_detached','detached_evidence_incomplete'), ('mismatch_detached','detached_evidence_incomplete')]:
                refused(fault, message)
            # A pending operation is admitted by unchanged source constraints;
            # protected deletion must block it before any detached transfer.
            f.db.execute("INSERT INTO operation_intents(operation_id,library_id,operation_kind,idempotency_key,request_json,request_sha256,state,local_revision,created_at_ms,updated_at_ms) VALUES(?,?,'save-package','pending','{}',?,'prepared',1,0,0)", (bid(9999), f.target, hashlib.sha256(b'{}').digest()))
            pending = snapshot(f.db, f.source['tables'])
            try:
                f.execute(f.actor)
            except base.Refused as error:
                assert str(error) == 'pending_operation:operation_intents'
            else:
                raise AssertionError('pending operation deleted')
            assert snapshot(f.db, f.source['tables']) == pending
            assert f.db.execute('SELECT count(*) FROM ll1_detached_evidence').fetchone() == (0,)
            # Resolve through the existing ordinary state/revision update; no
            # deletion guard or source trigger is removed to restore the test.
            f.db.execute("UPDATE operation_intents SET state='cancelled',local_revision=local_revision+1 WHERE operation_id=?", (bid(9999),))
            f.initial = snapshot(f.db, f.source['tables'])
            manifest = f.closure()
            evidence_count = sum(t in EVIDENCE for t, _ in manifest)
            assert f.execute(f.actor) == 'removed'
            assert snapshot(f.db, f.source['tables']) == survivors
            evidence = list(f.db.execute('SELECT * FROM ll1_detached_evidence ORDER BY source_table,source_rowid'))
            assert len(evidence) == evidence_count
            assert f.execute(f.actor) == 'removed'
            assert evidence == list(f.db.execute('SELECT * FROM ll1_detached_evidence ORDER BY source_table,source_rowid'))
            for event in ('UPDATE ll1_detached_evidence SET terminal_json=terminal_json', 'DELETE FROM ll1_detached_evidence'):
                try:
                    f.db.execute(event)
                except sqlite3.IntegrityError as error:
                    assert str(error) == 'terminal_immutable'
                else:
                    raise AssertionError('terminal evidence mutable')
            assert not f.db.execute('PRAGMA foreign_key_check').fetchall()
            assert f.tables_sql == list(f.db.execute("SELECT name,sql FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'll1_%'"))
            assert f.hashes == {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in MIGRATIONS.glob('*.sql')}
            print(json.dumps(dict(result='pass', sqlite_version=sqlite3.sqlite_version, catalog=f.measured, populated_tables=len(f.initial), target_tables=len({t for t,_ in manifest}), target_rows=len(manifest), detached_evidence_rows=evidence_count, unseeded_tables=[], retained_tables=sorted(t for t in f.source['tables'] if t not in {t for t,_ in manifest}), pending_blocker='operation_intents:prepared', rollback_cases=8, production_codec_approved=False), indent=2))
        finally:
            base.SUPPORTED = original_supported
            f.db.close()


if __name__ == '__main__':
    main()
