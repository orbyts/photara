#!/usr/bin/env python3
"""Expanded disposable SQLite D-disposition closure; never opens user data."""
import hashlib
import json
from pathlib import Path
import sqlite3
import sys
import tempfile

sys.dont_write_bytecode = True
import test_ll1_protected_sqlite_executor as executor
from test_ll1_actual_sqlite_baseline import bid, snapshot, MIGRATIONS


ADDITIONAL = {
    'membership_cache', 'library_media', 'media_local_state', 'organizations',
    'social_profiles', 'person_capabilities', 'person_labels', 'organization_labels',
    'person_organization_relationships', 'location_kinds', 'location_kind_terms',
    'locations', 'device_project_bindings', 'project_observations',
    'project_party_projection', 'project_location_projection', 'project_graph_projection',
    'project_access_policies', 'project_access_grants', 'storage_location_specs',
    'host_bindings', 'host_binding_selections', 'library_variable_values',
    'library_expressions', 'library_expression_dependencies', 'project_invitations',
    'project_media_links',
}


def seed_expanded(f):
    db = f.db
    expected = set(f.closure())
    def insert(table, **values):
        cursor = db.execute(f'INSERT INTO {table} ({",".join(values)}) VALUES ({",".join("?" for _ in values)})', tuple(values.values()))
        if target_scope:
            expected.add((table, cursor.lastrowid))
    target_scope = False
    insert('account_cache', account_id=bid(9000), display_name='Shared account', state='active', server_revision='1', observed_at_ms=0)
    digest = hashlib.sha256(b'fixture bytes, no file').digest()
    db.execute('BEGIN')
    for library, offset in ((f.target, 1000), (f.other, 0)):
        target_scope = library == f.target
        key = lambda n: bid(n + offset)
        scope = dict(library_id=library)
        record = dict(record_schema=1, local_revision=1, created_at_ms=0, updated_at_ms=0)
        insert('membership_cache', **scope, account_id=bid(9000), membership_id=key(200), role='owner', state='active', server_revision='1', observed_at_ms=0)
        insert('library_media', **scope, sha256=digest, media_type='image/png', byte_length=0, created_at_ms=0)
        insert('media_local_state', **scope, sha256=digest, state='missing')
        insert('people', **scope, **record, person_id=key(201), state='active', display_name='Active', sort_key='active', thumbnail_sha256=digest)
        insert('organizations', **scope, **record, organization_id=key(202), state='active', display_name='Organization', sort_key='organization', thumbnail_sha256=digest)
        insert('social_profiles', **scope, **record, social_profile_id=key(203), person_id=key(201), provider_id='fixture', handle='fixture', account_kind='personal', verification_kind='unverified', provenance_json='{}', fetch_state='never', avatar_policy='none', state='active')
        insert('person_capabilities', **scope, person_id=key(201), capability_id='fixture')
        insert('person_labels', **scope, person_id=key(201), label_key='fixture', display_label='Fixture')
        insert('organization_labels', **scope, organization_id=key(202), label_key='fixture', display_label='Fixture')
        insert('person_organization_relationships', **scope, **record, relationship_id=key(204), person_id=key(201), organization_id=key(202), relationship_type='fixture', state='active')
        insert('location_kinds', **scope, **record, location_kind_id=key(205), canonical_key='studio', canonical_display='Studio', state='active')
        insert('location_kind_terms', **scope, location_kind_id=key(205), term_key='studio', policy_version=1, spellings_json='["Studio"]')
        insert('locations', **scope, **record, location_id=key(206), location_kind_id=key(205), display_name='Studio', sort_key='studio', state='active')
        insert('locations', **scope, **record, location_id=key(207), location_kind_id=key(205), parent_location_id=key(206), display_name='Room', sort_key='room', state='active')
        insert('device_project_bindings', **scope, device_id=bid(10), project_id=key(20), locator_id=key(21), availability='unknown')
        insert('project_observations', **scope, observation_id=key(208), project_id=key(20), locator_id=key(21), commit_id=key(209), commit_sha256=digest, package_revision='1', title='Fixture', project_lifecycle='archived', asset_count=0, graph_count=1, observed_at_ms=0, index_schema=1)
        # Source references are snapshots, not ownership: target-owned projections
        # refer to the other Library, and surviving projections refer to target.
        source_library = f.other if target_scope else f.target
        insert('project_party_projection', observation_id=key(208), assignment_id=key(210), source_library_id=source_library, source_kind='person', source_record_id=key(201), source_revision='1', display_name_snapshot='Snapshot', roles_json='[]')
        insert('project_location_projection', observation_id=key(208), assignment_id=key(211), source_library_id=source_library, location_id=key(206), location_kind_id=key(205), location_name_snapshot='Snapshot', kind_name_snapshot='Snapshot')
        insert('project_graph_projection', observation_id=key(208), graph_id=key(212), graph_name='Graph', graph_digest=digest, graph_revision='1')
        insert('project_access_policies', **scope, project_id=key(20), visibility_policy='restricted', owner_mask=0, admin_mask=0, editor_mask=0, viewer_mask=0, authorization_generation=1, record_schema=1, local_revision=1, created_at=0, updated_at=0)
        insert('project_access_grants', **scope, grant_id=key(213), project_id=key(20), local_principal_id=db.execute('SELECT local_principal_id FROM library_contract_state WHERE library_id=?', (library,)).fetchone()[0], action_mask=0, state='revoked', revoked_at=0, record_schema=1, local_revision=1, created_at=0, updated_at=0)
        insert('project_invitations', **scope, invitation_id=key(218), project_id=key(20), inviter_account_id=bid(9000), target_account_id=bid(9000), action_mask=0, expected_policy_revision=1, expires_at=100, state='revoked', completed_at=0, record_schema=1, local_revision=1, created_at=0, updated_at=0)
        insert('project_media_links', **scope, project_id=key(20), sha256=digest, purpose='cover', source_commit_id=key(209), source_commit_sha256=digest, consent_projection_sha256=digest, state='tombstoned', retired_at=0, record_schema=1, local_revision=1, created_at=0, updated_at=0)
        insert('storage_location_specs', **scope, storage_root_id=key(30), storage_kind='filesystem', supported_rights_json='[]', record_schema=1, local_revision=1, created_at=0, updated_at=0)
        insert('host_bindings', **scope, binding_id=key(214), device_id=bid(10), storage_root_id=key(30), host_kind='macos', binding_kind='bookmark', secure_handle_ref=key(215), state='verified', generation=1, availability='available', verification_json='{}', verified_at=0, record_schema=1, local_revision=1, created_at=0, updated_at=0)
        insert('host_binding_selections', **scope, device_id=bid(10), storage_root_id=key(30), binding_id=key(214), selection_revision=1, updated_at=0)
        insert('library_variable_values', **scope, variable_id=key(41), value_id=key(216), is_present=1, binding_json='{}', origin='manual', updated_at=0)
        insert('library_expressions', **scope, expression_id=key(217), owner_variable_id=key(41), language='photara.expression.v1', container_mode='expression', compiler_version='1', source_utf8=b'1', source_sha256=digest, ast_canonical=b'{}', ast_sha256=digest, result_type_id='fixture', result_type_version=1, created_at=0)
        insert('library_expression_dependencies', **scope, expression_id=key(217), ordinal=0, dependency_kind='variable', variable_id=key(43), expected_type_id='fixture', expected_type_version=1)
        insert('library_expression_dependencies', **scope, expression_id=key(217), ordinal=1, dependency_kind='storage-slot', slot_id=key(31), expected_type_id='fixture', expected_type_version=1)
    db.execute('COMMIT')
    assert not db.execute('PRAGMA foreign_key_check').fetchall()
    f.initial = snapshot(db, f.source['tables'])
    assert f.closure() == expected, 'independent insertion ledger disagrees with FK/logical closure'
    return expected


def main():
    if not __debug__:
        raise RuntimeError('Assertions required')
    with tempfile.TemporaryDirectory(prefix='ll1-sqlite-coverage-') as tmp:
        f = executor.Fixture(str(Path(tmp) / 'disposable.sqlite'))
        original_supported = executor.SUPPORTED
        try:
            manifest = seed_expanded(f)
            covered = sorted({t for t, _ in manifest})
            initial = snapshot(f.db, f.source['tables'])
            expected_survivors = {t: sorted(repr(r[1:]) for r in f.db.execute(f'SELECT rowid,* FROM {t}') if (t, r[0]) not in manifest) for t in f.source['tables']}
            try:
                f.execute(f.actor)
            except executor.Refused as error:
                assert str(error) == 'unverified_table_disposition'
            else:
                raise AssertionError('base fixture must reject additional tables')
            assert snapshot(f.db, f.source['tables']) == initial
            # Keep the base fixture's fail-closed whitelist intact; this process
            # admits only explicitly seeded D tables for this mechanical test.
            executor.SUPPORTED = original_supported | ADDITIONAL
            passed = ['base_executor_refuses_expanded_closure']
            for fault, message in [('wrong_manifest', 'manifest_scope'), ('other_library', 'manifest_scope'), ('after_rename', 'injected'), ('commit_unbound', 'FOREIGN KEY'), ('omit_logical', 'incomplete_closure'), ('after_delete', 'injected'), ('after_receipt', 'injected'), ('marker_mismatch', 'terminal_disagreement'), ('inventory_mismatch', 'terminal_disagreement'), ('omit_marker', 'terminal_disagreement')]:
                try:
                    f.execute(f.actor, fault=fault)
                except (sqlite3.DatabaseError, executor.Refused) as error:
                    assert message in str(error), (fault, error)
                else:
                    raise AssertionError(fault)
                assert snapshot(f.db, f.source['tables']) == initial
                assert all(f.db.execute(f'SELECT count(*) FROM {t}').fetchone() == (0,) for t in ('ll1_terminal', 'll1_marker', 'll1_inventory'))
                assert f.permit is None and not f.db.in_transaction
                assert f.db.execute('PRAGMA foreign_keys').fetchone() == (1,)
                assert f.db.execute('PRAGMA defer_foreign_keys').fetchone() == (0,)
                passed.append(fault)
            assert f.execute(f.actor) == 'removed'
            assert snapshot(f.db, f.source['tables']) == expected_survivors
            assert all(f.db.execute(f'SELECT count(*) FROM {t}').fetchone() == (1,) for t in ('ll1_terminal', 'll1_marker', 'll1_inventory'))
            assert f.execute(f.actor) == 'removed'
            assert snapshot(f.db, f.source['tables']) == expected_survivors
            assert f.tables_sql == list(f.db.execute("SELECT name,sql FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'll1_%'"))
            assert not f.db.execute('PRAGMA foreign_key_check').fetchall()
            assert f.hashes == {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in MIGRATIONS.glob('*.sql')}
            assert len(covered) == 41 and len(manifest) == 48
            preserved = sorted(t for t in f.source['tables'] if initial[t] and t not in covered)
            unseeded = sorted(t for t in f.source['tables'] if not initial[t])
            print(json.dumps(dict(result='pass', sqlite_version=sqlite3.sqlite_version, catalog=f.measured, target_tables=len(covered), target_rows=len(manifest), covered_tables=covered, preserved_populated_tables=preserved, unseeded_tables=unseeded, rollback_cases=passed, unchanged_migration_sha256=f.hashes), indent=2))
        finally:
            executor.SUPPORTED = original_supported
            f.db.close()


if __name__ == '__main__':
    main()
