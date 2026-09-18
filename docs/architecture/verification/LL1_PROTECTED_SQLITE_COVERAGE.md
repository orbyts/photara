# LL1 expanded protected SQLite closure coverage

Executed 2026-09-18 with SQLite 3.53.4:

```sh
python3 scripts/test_ll1_protected_sqlite_coverage.py
```

The disposable experiment loads the unchanged generation-two migrations: **79 tables, 137 foreign keys, 190 triggers**, source fingerprint `02c638bb087862adb764f28d6c3caf86c812a71b28da0170bc409f3a1211d220`. It builds on [the protected SQLite executor](LL1_PROTECTED_SQLITE_EXECUTOR.md). The complete 79-table catalog is instantiated; populated deletion coverage is **41 tables / 48 target rows**, increased from 14 tables / 18 rows. This is not complete full-schema lifecycle or authorization implementation evidence.

The fixture uses temporary databases only. It never opens production databases, resolves host paths, reads package/media/cache files, calls credentials, or executes a migration against an existing installation. All production migration hashes and table declarations are compared before and after the run. `foreign_keys=ON`, `defer_foreign_keys=OFF`, and all original FK actions remain unchanged. The same narrow transaction-bound manifest exception qualifies deletion guards. The two RESTRICT name cycles retain the existing temporary-current-name transaction strategy. No NO ACTION migration is proposed.

## Evidence

An insertion ledger records the added target rows independently of the closure walker; equality with the computed FK plus explicit logical closure is asserted before deletion. The previously verified base closure supplies the initial 18 ledger entries. Every new target row has an unrelated-Library counterpart. Shared `account_cache`, `local_device`, `normalization_policies`, `onboarding_session`, and `schema_metadata` rows survive byte-for-byte, as do all unrelated-Library rows. The terminal receipt, removed marker, and inventory entry each appear exactly once after success; retry returns the same result with no change in source tables.

The expanded data exercises active and tombstoned records, location parent hierarchy, the generated location-kind canonical claim cycle, both storage/variable name cycles, scoped media links, memberships, revoked invitations and grants, host selection, expression dependencies, and observation-owned CASCADE projection rows. Projection snapshots deliberately name the opposite Library in `source_library_id`; ownership follows the observation, so unrelated projections referencing the removed Library remain intact. Both non-FK Project-keyed tables (`device_context_snapshots`, `legacy_external_resource_resolutions`) are removed only through the explicitly selected Project ownership.

The generated location columns exposed a closure-decoding defect: `PRAGMA table_info` omits generated columns while `SELECT *` includes them. The shared executor now reads cursor result column names, preserving `claim_owner_id` and `required_canonical_key` for the FK walk. This is covered by the populated canonical claim cycle.

The base executor first refuses the expanded manifest as `unverified_table_disposition`. Only this fixture process temporarily extends its whitelist with the 27 explicitly seeded D-disposition tables, restoring it on exit. The baseline whitelist and production code are unchanged. This prevents the wider catalog from being mistaken for an authorization to erase unverified lifecycle state.

Ten fault cases pass with exact full-schema rollback, empty terminal relations, no residual permit, no open transaction, FK enforcement on, and global FK deferral off: missing manifest row; deleting the other Library; failure after cycle rename; premature commit while the backlink is unsatisfied; omitted logical rows; failure after aggregate deletion; failure after receipt; marker mismatch; inventory mismatch; omitted marker. Successful deletion and idempotent replay then pass exact set-subtraction checks and `foreign_key_check`.

## Populated deletion coverage: 41 tables

| Group | Tables |
| --- | --- |
| Roots and membership | `libraries`, `library_contract_state`, `membership_cache` |
| Media references | `library_media`, `media_local_state`, `project_media_links` |
| Parties | `people`, `organizations`, `social_profiles`, `person_capabilities`, `person_labels`, `organization_labels`, `person_organization_relationships` |
| Locations | `location_kinds`, `location_kind_terms`, `locations` |
| Project ownership/access | `project_ownership`, `project_catalog`, `project_locators`, `project_access_policies`, `project_access_grants`, `project_invitations` |
| Project observations | `device_project_bindings`, `project_observations`, `project_party_projection`, `project_location_projection`, `project_graph_projection` |
| Storage | `storage_roots`, `storage_location_specs`, `storage_slots`, `storage_slot_names`, `device_root_bindings`, `host_bindings`, `host_binding_selections`, `legacy_external_resource_resolutions` |
| Context | `library_variables`, `library_variable_names`, `library_variable_values`, `library_expressions`, `library_expression_dependencies`, `device_context_snapshots` |

## Precise remaining gaps: 33 unseeded tables

These relations are installed and included in all before/after snapshots, but contain no rows in this fixture. Their empty preservation is not populated closure or lifecycle evidence. No table outside the explicit whitelist gains deletion support.

| Gap | Tables | Evidence still needed |
| --- | --- | --- |
| Legacy mutation/sync chain (13) | `sync_targets`, `mutations`, `local_changes`, `mutation_baselines`, `sync_object_state`, `sync_outbox`, `sync_inbox`, `sync_cursors`, `sync_conflicts`, `media_transfers`, `remote_mutation_receipts`, `local_mutation_dispositions`, `sync_snapshot_installs` | Populated terminal/replay evidence transfer and queue/channel closure; media transfers also need their parent target seeded. |
| Scoped sync chain (6) | `scoped_sync_channels`, `scoped_sync_operations`, `scoped_sync_operation_roots`, `scoped_sync_base`, `scoped_sync_inbox`, `scoped_sync_snapshot_installs` | Populated operation identities, replacement/predecessor edges, replay evidence, and channel closure. |
| Recovery/context chain (6) | `operation_intents`, `operation_events`, `recovery_items`, `recovery_publications`, `context_apply_intents`, `context_apply_receipts` | Unresolved-work blockers and completed-work terminal transfer before deletion. |
| Onboarding/cloud chain (6) | `onboarding_intents`, `onboarding_receipts`, `onboarding_dispositions`, `onboarding_replacements`, `library_cloud_bindings`, `onboarding_library_selection` | Closed-chain authenticated/replay evidence, unresolved-work blockers, and selected cloud binding reconciliation. |
| Credential retention (1) | `onboarding_credential_references` | Populated retention independent of deleted Library-bound onboarding rows. |
| Project creation (1) | `project_creation_intents` | Unresolved-work blocker and completed/cancelled terminal result transfer; no staging file operation. |

The groups above total 33: 13 + 6 + 6 + 6 + 1 + 1. Together with 41 populated deletion tables and 5 populated retained tables, they account for all 79 source tables. The script emits the exact table lists and source hashes on each run.

The existing disposition matrix already specifies blockers and terminal transfer for these missing chains; this experiment does not establish a new product/security decision. Implementing those semantics remains necessary before claiming complete full-schema protected deletion. A Python actor object remains the base fixture's caller placeholder; production authentication/Account ownership proof belongs to the separate authorization verification and is not claimed here. Canonical payload codecs, state combinations beyond the selected rows, every FK edge being populated, all concurrency interleavings, and actual service integration also remain outside this test.
