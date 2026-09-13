# D19 CXT2 exact DDL inventory

**Inert proposal only; not applied or runtime-validated.** R1 naming superseded;
Library rebaseline authorized 2026-09-12. Counts below come from the actual eleven proposal files. A statement
means one top-level SQL statement; function/trigger bodies count with their owner.
Explicit indexes exclude implicit indexes backing PRIMARY KEY/UNIQUE constraints.

## Per-file inventory

| Backend / file | Statements | Tables | Indexes | Triggers | Functions | Create policies | Drop policies | ALTER TABLE | Other |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| postgresql / [0008_library_project_access.proposal.sql](postgresql/0008_library_project_access.proposal.sql) | 15 | 6 | 9 | 0 | 0 | 0 | 0 | 0 | 0 |
| postgresql / [0009_storage_locations.proposal.sql](postgresql/0009_storage_locations.proposal.sql) | 6 | 3 | 2 | 0 | 0 | 0 | 0 | 1 | 0 |
| postgresql / [0010_library_context.proposal.sql](postgresql/0010_library_context.proposal.sql) | 13 | 5 | 7 | 0 | 0 | 0 | 0 | 1 | 0 |
| postgresql / [0011_scoped_sync.proposal.sql](postgresql/0011_scoped_sync.proposal.sql) | 23 | 6 | 15 | 0 | 0 | 0 | 0 | 2 | 0 |
| postgresql / [0012_d19_access_guards.proposal.sql](postgresql/0012_d19_access_guards.proposal.sql) | 374 | 0 | 0 | 42 | 27 | 173 | 25 | 40 | 67 |
| sqlite / [0007_library_project_access.proposal.sql](sqlite/0007_library_project_access.proposal.sql) | 14 | 5 | 9 | 0 | 0 | 0 | 0 | 0 | 0 |
| sqlite / [0008_storage_locations_bindings.proposal.sql](sqlite/0008_storage_locations_bindings.proposal.sql) | 12 | 6 | 6 | 0 | 0 | 0 | 0 | 0 | 0 |
| sqlite / [0009_library_context.proposal.sql](sqlite/0009_library_context.proposal.sql) | 12 | 5 | 7 | 0 | 0 | 0 | 0 | 0 | 0 |
| sqlite / [0010_context_apply_recovery.proposal.sql](sqlite/0010_context_apply_recovery.proposal.sql) | 7 | 3 | 4 | 0 | 0 | 0 | 0 | 0 | 0 |
| sqlite / [0011_scoped_sync.proposal.sql](sqlite/0011_scoped_sync.proposal.sql) | 19 | 7 | 12 | 0 | 0 | 0 | 0 | 0 | 0 |
| sqlite / [0012_d19_guards_and_floor.proposal.sql](sqlite/0012_d19_guards_and_floor.proposal.sql) | 75 | 0 | 0 | 74 | 0 | 0 | 0 | 0 | 1 |
| **postgresql total** | **431** | **20** | **33** | **42** | **27** | **173** | **25** | **44** | **67** |
| **sqlite total** | **139** | **26** | **38** | **74** | **0** | **0** | **0** | **0** | **1** |

The proposed totals remain local 44+26=70 and service 35+20=55 relations.
The Library-named baseline and service 0007 reservation were rechecked; counts are unchanged.

## Relation/signature cross-check

| Code | Relation | SQLite | PostgreSQL |
| --- | --- | --- | --- |
| A1 | `library_contract_state` | yes | yes |
| A2 | `project_ownership` | yes | yes |
| A3 | `project_access_policies` | yes | yes |
| A4 | `project_access_grants` | yes | yes |
| A5 | `project_invitations` | yes | yes |
| C1 | `project_invitation_secrets` | — | yes |
| F1 | `scoped_streams` | — | yes |
| F2 | `scoped_change_batches` | — | yes |
| F3 | `scoped_changes` | — | yes |
| F4 | `scoped_command_receipts` | — | yes |
| F5 | `scoped_sync_clients` | — | yes |
| H1 | `host_bindings` | yes | — |
| H2 | `host_binding_selections` | yes | — |
| H3 | `legacy_external_resource_resolutions` | yes | — |
| H4 | `device_context_snapshots` | yes | — |
| O1 | `context_apply_intents` | yes | — |
| O2 | `context_apply_receipts` | yes | — |
| P1 | `project_media_links` | yes | yes |
| Q1 | `scoped_sync_channels` | yes | — |
| Q2 | `scoped_sync_operations` | yes | — |
| Q3 | `scoped_sync_operation_roots` | yes | — |
| Q4 | `scoped_sync_base` | yes | — |
| Q5 | `scoped_sync_inbox` | yes | — |
| Q6 | `scoped_sync_snapshot_installs` | yes | — |
| S1 | `storage_location_specs` | yes | yes |
| S2 | `storage_slots` | yes | yes |
| S3 | `storage_slot_names` | yes | yes |
| V1 | `library_variables` | yes | yes |
| V2 | `library_variable_values` | yes | yes |
| V3 | `library_variable_names` | yes | yes |
| V4 | `library_expressions` | yes | yes |
| V5 | `library_expression_dependencies` | yes | yes |

Actual CREATE TABLE columns were independently compared to every accepted signature,
expanding L/M and the local revision adapter. Every proposed FK target/column tuple
was resolved against these additions and text-only baseline keys. Full child FK
index coverage was checked, including constraints added after forward references.

| Backend | Foreign keys in proposal CREATE/ALTER | CHECK clauses in CREATE/ALTER | PK clauses | UNIQUE clauses |
| --- | ---: | ---: | ---: | ---: |
| sqlite | 54 | 261 | 26 | 14 |
| postgresql | 50 | 190 | 20 | 13 |

## Static verification and limits

- Eleven files lexically checked for balanced delimiters, quoted/dollar-quoted
  bodies, statement termination and exact counts. This is not a grammar parser.
- This scanner does not use a PostgreSQL grammar parser; SQL grammar is not
  claimed. This script starts no SQL engine and opens no database.
- 26 local and 20 service relation signatures, scoped FKs, referenced unique keys,
  full child-FK indexes, object-name uniqueness and PostgreSQL identifier lengths
  checked against actual proposal text and baseline schema text.
- All 25 old library_scope policy drops are enumerated; all 20 new service
  tables explicitly enable and force RLS. Raw transport/access/private media grants
  and five required authorization facade helpers are spelled out below.
- The [responsibility ledger](RESPONSIBILITIES.md) identifies every invariant that
  SQL cannot prove alone. SQL grammar/type/name resolution, engine semantics,
  RLS/grants, race behavior and codecs remain untested CXT3 gates.

## Regenerated Library baseline hashes

SHA-256 values below were recomputed from the rebaselined files.
These are synthetic/unshipped Generation Two baselines, not deployed files. File
hashes are distinct from SQLx ledger checksums. Baseline SQL fences are checked
separately; package fixture semantics are verified by the Rust package tests.

| Protected file | SHA-256 |
| --- | --- |
| `crates/photara-library/migrations/generation_two/0001_local_identity.sql` | `b99f33adc424a0aae3e717c972e33d72be5f0ec82f11ac0e7fb4ec28037377fd` |
| `crates/photara-library/migrations/generation_two/0002_typed_library.sql` | `7562bc582d3d8a6a8bc1a8d0e5451c24670763c7d3031b78e903c9960c1416a9` |
| `crates/photara-library/migrations/generation_two/0003_catalog_device.sql` | `7331ab5c6742d194fc64eb41557763f92cb93ecdac358aa564d01b82f700fc3e` |
| `crates/photara-library/migrations/generation_two/0004_mutations_sync.sql` | `72c5d345b3a93ee6a5813b8000afd2f33fc614eb45109f2d37f310225148cf4f` |
| `crates/photara-library/migrations/generation_two/0005_durable_recovery.sql` | `d0c97f3f49bbcb1581aac714ff5798d90200039867022d01ffed6e18bf59b3fb` |
| `crates/photara-library/migrations/generation_two/0006_invariant_guards.sql` | `85ff08feb48040eb5bcd1324e275e5593ac98cd4ba2b826e66d867eecfdfd8de` |
| `docs/fixtures/generation-two/canonical-vectors.json` | `2fc9018d84767d4d63cbb385f60db8cf4dd45fd810dfc038c62007388eb7463d` |
| `docs/fixtures/generation-two/context-amendment.json` | `5db3146e0aaad53c6e2c52f1d16737531c02d77f2b29426e42d9e0bee3c1e83b` |
| `docs/fixtures/generation-two/d19-compatibility.json` | `dba279e37302c4d2d7d3ddafc13d855ea74dbc640b148d0eb2115cbf227bb3a9` |
| `docs/fixtures/generation-two/d19-context.json` | `6e49a4156fa664f3844372ccdb1932a04e681abec48b762386a81a1826a6dcd4` |
| `docs/fixtures/generation-two/d19-contracts.json` | `c4cd563e2a454bb1f8353250ee3bfd38a5c88a825dc0da64677ffcdeb2c0b893` |
| `docs/fixtures/generation-two/d19-package-specimen.json` | `be32df34ad90d00430dd4fb0e6e475dc2c5c0093d89b261ab8c6fb26a95113b7` |
| `docs/fixtures/generation-two/normalization-and-limits.json` | `6ef0289b65e9da84d96655287257e29149fe800daca5aa9b75c44cc0111e65b0` |
| `docs/fixtures/generation-two/package-specimen.json` | `ff83aa26537bc9d134ae0257a4d92870e9b91dfa1c938f5a60bb8086230fdcef` |
| `docs/fixtures/generation-two/records.json` | `79c4ce72cfbab22d0dad4898c9f19523607800b93a9c305de69c6b8e8c49a741` |
| `docs/fixtures/generation-two/scenarios.json` | `61185ac18d9a69cd5724adf728cb4550d8e8c489295ceaca6efeba47c47b409b` |
| `docs/fixtures/generation-two/social-export.json` | `29dde1b14af1e3655db162564fda45845cafadca75d30cfb5c89ac844cf71e0a` |
| `docs/fixtures/generation-two/sync-trace.json` | `561e0240ddd6881d0e3287eb48d7deb5e4e6cc97f41c2b94c7da03c3403b56d6` |

## Exact statement and object list

Line numbers refer to the proposal files linked in the per-file table. CREATE
TABLE includes its inline constraints; forward-reference ALTERs and all explicit
indexes/triggers/policies/functions/privilege/floor statements appear separately.

### postgresql / 0008_library_project_access.proposal.sql

| Statement | Line | Kind | Object / operation |
| ---: | ---: | --- | --- |
| 1 | 9 | CREATE TABLE | `photara.library_contract_state` |
| 2 | 29 | CREATE TABLE | `photara.project_ownership` |
| 3 | 51 | CREATE INDEX | `d19_project_ownership_library_state on photara.project_ownership` |
| 4 | 54 | CREATE TABLE | `photara.project_access_policies` |
| 5 | 81 | CREATE TABLE | `photara_identity.project_access_grants` |
| 6 | 108 | CREATE INDEX | `d19_project_access_grants_account_id on photara_identity.project_access_grants` |
| 7 | 110 | CREATE INDEX | `d19_project_access_grants_local_principal_id on photara_identity.project_access_grants` |
| 8 | 112 | CREATE INDEX | `d19_project_access_grants_account on photara_identity.project_access_grants` |
| 9 | 115 | CREATE TABLE | `photara_identity.project_invitations` |
| 10 | 146 | CREATE INDEX | `d19_project_invitations_pending on photara_identity.project_invitations` |
| 11 | 148 | CREATE INDEX | `d19_project_invitations_target on photara_identity.project_invitations` |
| 12 | 150 | CREATE INDEX | `d19_project_invitations_inviter on photara_identity.project_invitations` |
| 13 | 153 | CREATE TABLE | `photara_private.project_invitation_secrets` |
| 14 | 163 | CREATE INDEX | `d19_project_invitations_fk1 on photara_identity.project_invitations` |
| 15 | 165 | CREATE INDEX | `d19_project_invitations_fk4 on photara_identity.project_invitations` |

### postgresql / 0009_storage_locations.proposal.sql

| Statement | Line | Kind | Object / operation |
| ---: | ---: | --- | --- |
| 1 | 9 | CREATE TABLE | `photara.storage_location_specs` |
| 2 | 29 | CREATE TABLE | `photara.storage_slots` |
| 3 | 52 | CREATE INDEX | `d19_storage_slots_root on photara.storage_slots` |
| 4 | 55 | CREATE TABLE | `photara.storage_slot_names` |
| 5 | 67 | ALTER TABLE | `photara.storage_slots — add constraints/columns` |
| 6 | 69 | CREATE INDEX | `d19_storage_slots_current_name on photara.storage_slots` |

### postgresql / 0010_library_context.proposal.sql

| Statement | Line | Kind | Object / operation |
| ---: | ---: | --- | --- |
| 1 | 9 | CREATE TABLE | `photara.library_variables` |
| 2 | 45 | CREATE INDEX | `d19_library_variables_browse on photara.library_variables` |
| 3 | 48 | CREATE TABLE | `photara.library_variable_values` |
| 4 | 69 | CREATE TABLE | `photara.library_variable_names` |
| 5 | 81 | ALTER TABLE | `photara.library_variables — add constraints/columns` |
| 6 | 83 | CREATE INDEX | `d19_library_variables_current_name on photara.library_variables` |
| 7 | 86 | CREATE TABLE | `photara.library_expressions` |
| 8 | 111 | CREATE INDEX | `d19_library_expressions_owner on photara.library_expressions` |
| 9 | 114 | CREATE TABLE | `photara.library_expression_dependencies` |
| 10 | 132 | CREATE INDEX | `d19_library_expression_dependencies_variable_id on photara.library_expression_dependencies` |
| 11 | 134 | CREATE INDEX | `d19_library_expression_dependencies_slot_id on photara.library_expression_dependencies` |
| 12 | 136 | CREATE INDEX | `d19_library_expression_dependencies_fk2 on photara.library_expression_dependencies` |
| 13 | 138 | CREATE INDEX | `d19_library_expression_dependencies_fk3 on photara.library_expression_dependencies` |

### postgresql / 0011_scoped_sync.proposal.sql

| Statement | Line | Kind | Object / operation |
| ---: | ---: | --- | --- |
| 1 | 9 | CREATE TABLE | `photara.project_media_links` |
| 2 | 34 | CREATE INDEX | `d19_project_media_links_media on photara.project_media_links` |
| 3 | 37 | CREATE TABLE | `photara_private.scoped_streams` |
| 4 | 54 | CREATE INDEX | `d19_scoped_streams_library on photara_private.scoped_streams` |
| 5 | 56 | CREATE INDEX | `d19_scoped_streams_project on photara_private.scoped_streams` |
| 6 | 59 | CREATE TABLE | `photara_private.scoped_change_batches` |
| 7 | 80 | CREATE TABLE | `photara_private.scoped_changes` |
| 8 | 104 | CREATE TABLE | `photara_private.scoped_command_receipts` |
| 9 | 135 | ALTER TABLE | `photara_private.scoped_change_batches — add constraints/columns` |
| 10 | 137 | CREATE INDEX | `d19_scoped_command_receipts_actor on photara_private.scoped_command_receipts` |
| 11 | 140 | CREATE TABLE | `photara_private.scoped_sync_clients` |
| 12 | 153 | CREATE INDEX | `d19_scoped_sync_clients_device on photara_private.scoped_sync_clients` |
| 13 | 155 | ALTER TABLE | `photara_private.media_upload_sessions — add constraints/columns` |
| 14 | 170 | CREATE INDEX | `d19_media_upload_sessions_project_actor on photara_private.media_upload_sessions` |
| 15 | 172 | CREATE INDEX | `d19_media_upload_sessions_actor on photara_private.media_upload_sessions` |
| 16 | 174 | CREATE INDEX | `d19_scoped_streams_fk3 on photara_private.scoped_streams` |
| 17 | 176 | CREATE INDEX | `d19_scoped_change_batches_fk1 on photara_private.scoped_change_batches` |
| 18 | 178 | CREATE INDEX | `d19_scoped_changes_fk1 on photara_private.scoped_changes` |
| 19 | 180 | CREATE INDEX | `d19_scoped_changes_fk5 on photara_private.scoped_changes` |
| 20 | 182 | CREATE INDEX | `d19_scoped_command_receipts_fk1 on photara_private.scoped_command_receipts` |
| 21 | 184 | CREATE INDEX | `d19_scoped_command_receipts_fk2 on photara_private.scoped_command_receipts` |
| 22 | 186 | CREATE INDEX | `d19_scoped_command_receipts_fk3 on photara_private.scoped_command_receipts` |
| 23 | 188 | CREATE INDEX | `d19_scoped_sync_clients_fk1 on photara_private.scoped_sync_clients` |

### postgresql / 0012_d19_access_guards.proposal.sql

| Statement | Line | Kind | Object / operation |
| ---: | ---: | --- | --- |
| 1 | 8 | CREATE FUNCTION | `photara_private.d19_immutable` |
| 2 | 13 | CREATE FUNCTION | `photara_private.d19_mutable` |
| 3 | 32 | CREATE TRIGGER | `d19_library_contract_state_retention on photara.library_contract_state` |
| 4 | 35 | CREATE TRIGGER | `d19_project_ownership_retention on photara.project_ownership` |
| 5 | 38 | CREATE TRIGGER | `d19_project_access_policies_retention on photara.project_access_policies` |
| 6 | 41 | CREATE TRIGGER | `d19_project_access_grants_retention on photara_identity.project_access_grants` |
| 7 | 44 | CREATE TRIGGER | `d19_project_invitations_retention on photara_identity.project_invitations` |
| 8 | 47 | CREATE TRIGGER | `d19_project_invitation_secrets_retention on photara_private.project_invitation_secrets` |
| 9 | 50 | CREATE TRIGGER | `d19_storage_location_specs_retention on photara.storage_location_specs` |
| 10 | 53 | CREATE TRIGGER | `d19_storage_slots_retention on photara.storage_slots` |
| 11 | 56 | CREATE FUNCTION | `photara_private.d19_storage_slots_no_resurrection` |
| 12 | 64 | CREATE TRIGGER | `d19_storage_slots_no_resurrection on photara.storage_slots` |
| 13 | 67 | CREATE TRIGGER | `d19_storage_slot_names_retention on photara.storage_slot_names` |
| 14 | 70 | CREATE TRIGGER | `d19_library_variables_retention on photara.library_variables` |
| 15 | 73 | CREATE FUNCTION | `photara_private.d19_library_variables_no_resurrection` |
| 16 | 81 | CREATE TRIGGER | `d19_library_variables_no_resurrection on photara.library_variables` |
| 17 | 84 | CREATE TRIGGER | `d19_library_variable_values_retention on photara.library_variable_values` |
| 18 | 87 | CREATE TRIGGER | `d19_library_variable_names_retention on photara.library_variable_names` |
| 19 | 90 | CREATE TRIGGER | `d19_library_expressions_retention on photara.library_expressions` |
| 20 | 93 | CREATE TRIGGER | `d19_library_expression_dependencies_retention on photara.library_expression_dependencies` |
| 21 | 96 | CREATE TRIGGER | `d19_project_media_links_retention on photara.project_media_links` |
| 22 | 99 | CREATE FUNCTION | `photara_private.d19_project_media_links_no_resurrection` |
| 23 | 107 | CREATE TRIGGER | `d19_project_media_links_no_resurrection on photara.project_media_links` |
| 24 | 110 | CREATE TRIGGER | `d19_scoped_streams_retention on photara_private.scoped_streams` |
| 25 | 113 | CREATE TRIGGER | `d19_scoped_change_batches_retention on photara_private.scoped_change_batches` |
| 26 | 116 | CREATE TRIGGER | `d19_scoped_changes_retention on photara_private.scoped_changes` |
| 27 | 119 | CREATE TRIGGER | `d19_scoped_command_receipts_retention on photara_private.scoped_command_receipts` |
| 28 | 122 | CREATE TRIGGER | `d19_scoped_sync_clients_retention on photara_private.scoped_sync_clients` |
| 29 | 125 | CREATE FUNCTION | `photara_private.d19_project_ownership_lifecycle` |
| 30 | 133 | CREATE TRIGGER | `d19_project_ownership_lifecycle on photara.project_ownership` |
| 31 | 136 | CREATE FUNCTION | `photara_private.d19_project_invitations_terminal` |
| 32 | 144 | CREATE TRIGGER | `d19_project_invitations_terminal on photara_identity.project_invitations` |
| 33 | 147 | CREATE FUNCTION | `photara_private.d19_library_contract_state_generation` |
| 34 | 155 | CREATE TRIGGER | `d19_library_contract_state_generation on photara.library_contract_state` |
| 35 | 158 | CREATE FUNCTION | `photara_private.d19_project_access_policies_generation` |
| 36 | 166 | CREATE TRIGGER | `d19_project_access_policies_generation on photara.project_access_policies` |
| 37 | 169 | CREATE FUNCTION | `photara_private.d19_project_invitation_secrets_consume` |
| 38 | 177 | CREATE TRIGGER | `d19_project_invitation_secrets_consume on photara_private.project_invitation_secrets` |
| 39 | 180 | CREATE FUNCTION | `photara_private.d19_scoped_streams_sequence` |
| 40 | 188 | CREATE TRIGGER | `d19_scoped_streams_sequence on photara_private.scoped_streams` |
| 41 | 191 | CREATE FUNCTION | `photara_private.d19_storage_slots_target_insert` |
| 42 | 199 | CREATE TRIGGER | `d19_storage_slots_target_insert on photara.storage_slots` |
| 43 | 202 | CREATE FUNCTION | `photara_private.d19_storage_slots_target_update` |
| 44 | 210 | CREATE TRIGGER | `d19_storage_slots_target_update on photara.storage_slots` |
| 45 | 213 | CREATE FUNCTION | `photara_private.d19_storage_roots_live_dependents` |
| 46 | 221 | CREATE TRIGGER | `d19_storage_roots_live_dependents on photara.storage_roots` |
| 47 | 228 | CREATE FUNCTION | `photara_private.d19_actor_valid` |
| 48 | 242 | CREATE FUNCTION | `photara_private.d19_action_bit` |
| 49 | 250 | CREATE FUNCTION | `photara_private.d19_can_library` |
| 50 | 266 | CREATE FUNCTION | `photara_private.can_read_library` |
| 51 | 272 | CREATE FUNCTION | `photara_private.can_project` |
| 52 | 296 | CREATE FUNCTION | `photara_private.can_project_media` |
| 53 | 303 | CREATE FUNCTION | `photara_private.d19_lock_actor` |
| 54 | 316 | CREATE FUNCTION | `photara_private.authorize_library` |
| 55 | 333 | CREATE FUNCTION | `photara_private.authorize_project` |
| 56 | 354 | CREATE FUNCTION | `photara_private.d19_managers_at_commit` |
| 57 | 371 | CREATE TRIGGER | `d19_project_ownership_managers on photara.project_ownership` |
| 58 | 374 | CREATE TRIGGER | `d19_project_access_grants_managers on photara_identity.project_access_grants` |
| 59 | 377 | CREATE TRIGGER | `d19_accounts_managers on photara_identity.accounts` |
| 60 | 380 | CREATE TRIGGER | `d19_account_identities_managers on photara_identity.account_identities` |
| 61 | 383 | CREATE FUNCTION | `photara_private.d19_feed_at_commit` |
| 62 | 415 | CREATE TRIGGER | `d19_scoped_streams_closure on photara_private.scoped_streams` |
| 63 | 418 | CREATE TRIGGER | `d19_scoped_change_batches_closure on photara_private.scoped_change_batches` |
| 64 | 421 | CREATE TRIGGER | `d19_scoped_changes_closure on photara_private.scoped_changes` |
| 65 | 424 | CREATE TRIGGER | `d19_scoped_command_receipts_closure on photara_private.scoped_command_receipts` |
| 66 | 427 | CREATE FUNCTION | `photara_private.d19_scoped_sync_clients_ack_insert` |
| 67 | 435 | CREATE TRIGGER | `d19_scoped_sync_clients_ack_insert on photara_private.scoped_sync_clients` |
| 68 | 438 | CREATE FUNCTION | `photara_private.d19_scoped_sync_clients_ack_update` |
| 69 | 446 | CREATE TRIGGER | `d19_scoped_sync_clients_ack_update on photara_private.scoped_sync_clients` |
| 70 | 449 | DROP POLICY | `library_scope on photara.libraries` |
| 71 | 451 | DROP POLICY | `library_scope on photara_private.library_subscriptions` |
| 72 | 453 | DROP POLICY | `library_scope on photara_private.library_entitlement_grants` |
| 73 | 455 | DROP POLICY | `library_scope on photara.people` |
| 74 | 457 | DROP POLICY | `library_scope on photara.organizations` |
| 75 | 459 | DROP POLICY | `library_scope on photara.social_profiles` |
| 76 | 461 | DROP POLICY | `library_scope on photara.person_capabilities` |
| 77 | 463 | DROP POLICY | `library_scope on photara.person_labels` |
| 78 | 465 | DROP POLICY | `library_scope on photara.organization_labels` |
| 79 | 467 | DROP POLICY | `library_scope on photara.person_organization_relationships` |
| 80 | 469 | DROP POLICY | `library_scope on photara.location_kinds` |
| 81 | 471 | DROP POLICY | `library_scope on photara.location_kind_terms` |
| 82 | 473 | DROP POLICY | `library_scope on photara.locations` |
| 83 | 475 | DROP POLICY | `library_scope on photara.storage_roots` |
| 84 | 477 | DROP POLICY | `library_scope on photara.project_catalog` |
| 85 | 479 | DROP POLICY | `library_scope on photara.project_locators` |
| 86 | 481 | DROP POLICY | `library_scope on photara.package_observations` |
| 87 | 483 | DROP POLICY | `library_scope on photara.library_media` |
| 88 | 485 | DROP POLICY | `library_scope on photara.library_change_batches` |
| 89 | 487 | DROP POLICY | `library_scope on photara.library_changes` |
| 90 | 489 | DROP POLICY | `library_scope on photara_private.media_objects` |
| 91 | 491 | DROP POLICY | `library_scope on photara_private.library_streams` |
| 92 | 493 | DROP POLICY | `library_scope on photara_private.mutation_receipts` |
| 93 | 495 | DROP POLICY | `library_scope on photara_private.sync_clients` |
| 94 | 497 | DROP POLICY | `library_scope on photara_private.media_upload_sessions` |
| 95 | 499 | REVOKE ALL | `REVOKE ALL ON photara . library_change_batches , photara . library_changes , photara_private . library_streams , photara_private . mutation_receipts , photara_private . sync_clients FROM photara_api` |
| 96 | 501 | REVOKE EXECUTE | `REVOKE EXECUTE ON FUNCTION photara_private . authorize_library ( uuid , text ) FROM photara_api , photara_control` |
| 97 | 503 | ALTER TABLE | `photara.library_contract_state — enable RLS` |
| 98 | 505 | ALTER TABLE | `photara.library_contract_state — force RLS` |
| 99 | 507 | ALTER TABLE | `photara.project_ownership — enable RLS` |
| 100 | 509 | ALTER TABLE | `photara.project_ownership — force RLS` |
| 101 | 511 | ALTER TABLE | `photara.project_access_policies — enable RLS` |
| 102 | 513 | ALTER TABLE | `photara.project_access_policies — force RLS` |
| 103 | 515 | ALTER TABLE | `photara_identity.project_access_grants — enable RLS` |
| 104 | 517 | ALTER TABLE | `photara_identity.project_access_grants — force RLS` |
| 105 | 519 | ALTER TABLE | `photara_identity.project_invitations — enable RLS` |
| 106 | 521 | ALTER TABLE | `photara_identity.project_invitations — force RLS` |
| 107 | 523 | ALTER TABLE | `photara_private.project_invitation_secrets — enable RLS` |
| 108 | 525 | ALTER TABLE | `photara_private.project_invitation_secrets — force RLS` |
| 109 | 527 | ALTER TABLE | `photara.storage_location_specs — enable RLS` |
| 110 | 529 | ALTER TABLE | `photara.storage_location_specs — force RLS` |
| 111 | 531 | ALTER TABLE | `photara.storage_slots — enable RLS` |
| 112 | 533 | ALTER TABLE | `photara.storage_slots — force RLS` |
| 113 | 535 | ALTER TABLE | `photara.storage_slot_names — enable RLS` |
| 114 | 537 | ALTER TABLE | `photara.storage_slot_names — force RLS` |
| 115 | 539 | ALTER TABLE | `photara.library_variables — enable RLS` |
| 116 | 541 | ALTER TABLE | `photara.library_variables — force RLS` |
| 117 | 543 | ALTER TABLE | `photara.library_variable_values — enable RLS` |
| 118 | 545 | ALTER TABLE | `photara.library_variable_values — force RLS` |
| 119 | 547 | ALTER TABLE | `photara.library_variable_names — enable RLS` |
| 120 | 549 | ALTER TABLE | `photara.library_variable_names — force RLS` |
| 121 | 551 | ALTER TABLE | `photara.library_expressions — enable RLS` |
| 122 | 553 | ALTER TABLE | `photara.library_expressions — force RLS` |
| 123 | 555 | ALTER TABLE | `photara.library_expression_dependencies — enable RLS` |
| 124 | 557 | ALTER TABLE | `photara.library_expression_dependencies — force RLS` |
| 125 | 559 | ALTER TABLE | `photara.project_media_links — enable RLS` |
| 126 | 561 | ALTER TABLE | `photara.project_media_links — force RLS` |
| 127 | 563 | ALTER TABLE | `photara_private.scoped_streams — enable RLS` |
| 128 | 565 | ALTER TABLE | `photara_private.scoped_streams — force RLS` |
| 129 | 567 | ALTER TABLE | `photara_private.scoped_change_batches — enable RLS` |
| 130 | 569 | ALTER TABLE | `photara_private.scoped_change_batches — force RLS` |
| 131 | 571 | ALTER TABLE | `photara_private.scoped_changes — enable RLS` |
| 132 | 573 | ALTER TABLE | `photara_private.scoped_changes — force RLS` |
| 133 | 575 | ALTER TABLE | `photara_private.scoped_command_receipts — enable RLS` |
| 134 | 577 | ALTER TABLE | `photara_private.scoped_command_receipts — force RLS` |
| 135 | 579 | ALTER TABLE | `photara_private.scoped_sync_clients — enable RLS` |
| 136 | 581 | ALTER TABLE | `photara_private.scoped_sync_clients — force RLS` |
| 137 | 583 | CREATE POLICY | `d19_owner on photara.libraries` |
| 138 | 585 | CREATE POLICY | `d19_read on photara.libraries` |
| 139 | 587 | CREATE POLICY | `d19_insert on photara.libraries` |
| 140 | 589 | CREATE POLICY | `d19_update on photara.libraries` |
| 141 | 591 | CREATE POLICY | `d19_control on photara.libraries` |
| 142 | 593 | CREATE POLICY | `d19_owner on photara_private.library_subscriptions` |
| 143 | 595 | CREATE POLICY | `d19_control on photara_private.library_subscriptions` |
| 144 | 597 | REVOKE ALL | `REVOKE ALL ON photara_private . library_subscriptions FROM photara_api` |
| 145 | 599 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara_private . library_subscriptions TO photara_control` |
| 146 | 601 | CREATE POLICY | `d19_owner on photara_private.library_entitlement_grants` |
| 147 | 603 | CREATE POLICY | `d19_control on photara_private.library_entitlement_grants` |
| 148 | 605 | REVOKE ALL | `REVOKE ALL ON photara_private . library_entitlement_grants FROM photara_api` |
| 149 | 607 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara_private . library_entitlement_grants TO photara_control` |
| 150 | 609 | CREATE POLICY | `d19_owner on photara.people` |
| 151 | 611 | CREATE POLICY | `d19_read on photara.people` |
| 152 | 613 | CREATE POLICY | `d19_insert on photara.people` |
| 153 | 615 | CREATE POLICY | `d19_update on photara.people` |
| 154 | 617 | CREATE POLICY | `d19_control on photara.people` |
| 155 | 619 | CREATE POLICY | `d19_owner on photara.organizations` |
| 156 | 621 | CREATE POLICY | `d19_read on photara.organizations` |
| 157 | 623 | CREATE POLICY | `d19_insert on photara.organizations` |
| 158 | 625 | CREATE POLICY | `d19_update on photara.organizations` |
| 159 | 627 | CREATE POLICY | `d19_control on photara.organizations` |
| 160 | 629 | CREATE POLICY | `d19_owner on photara.social_profiles` |
| 161 | 631 | CREATE POLICY | `d19_read on photara.social_profiles` |
| 162 | 633 | CREATE POLICY | `d19_insert on photara.social_profiles` |
| 163 | 635 | CREATE POLICY | `d19_update on photara.social_profiles` |
| 164 | 637 | CREATE POLICY | `d19_control on photara.social_profiles` |
| 165 | 639 | CREATE POLICY | `d19_owner on photara.person_capabilities` |
| 166 | 641 | CREATE POLICY | `d19_read on photara.person_capabilities` |
| 167 | 643 | CREATE POLICY | `d19_insert on photara.person_capabilities` |
| 168 | 645 | CREATE POLICY | `d19_update on photara.person_capabilities` |
| 169 | 647 | CREATE POLICY | `d19_delete on photara.person_capabilities` |
| 170 | 649 | CREATE POLICY | `d19_control on photara.person_capabilities` |
| 171 | 651 | CREATE POLICY | `d19_owner on photara.person_labels` |
| 172 | 653 | CREATE POLICY | `d19_read on photara.person_labels` |
| 173 | 655 | CREATE POLICY | `d19_insert on photara.person_labels` |
| 174 | 657 | CREATE POLICY | `d19_update on photara.person_labels` |
| 175 | 659 | CREATE POLICY | `d19_delete on photara.person_labels` |
| 176 | 661 | CREATE POLICY | `d19_control on photara.person_labels` |
| 177 | 663 | CREATE POLICY | `d19_owner on photara.organization_labels` |
| 178 | 665 | CREATE POLICY | `d19_read on photara.organization_labels` |
| 179 | 667 | CREATE POLICY | `d19_insert on photara.organization_labels` |
| 180 | 669 | CREATE POLICY | `d19_update on photara.organization_labels` |
| 181 | 671 | CREATE POLICY | `d19_delete on photara.organization_labels` |
| 182 | 673 | CREATE POLICY | `d19_control on photara.organization_labels` |
| 183 | 675 | CREATE POLICY | `d19_owner on photara.person_organization_relationships` |
| 184 | 677 | CREATE POLICY | `d19_read on photara.person_organization_relationships` |
| 185 | 679 | CREATE POLICY | `d19_insert on photara.person_organization_relationships` |
| 186 | 681 | CREATE POLICY | `d19_update on photara.person_organization_relationships` |
| 187 | 683 | CREATE POLICY | `d19_control on photara.person_organization_relationships` |
| 188 | 685 | CREATE POLICY | `d19_owner on photara.location_kinds` |
| 189 | 687 | CREATE POLICY | `d19_read on photara.location_kinds` |
| 190 | 689 | CREATE POLICY | `d19_insert on photara.location_kinds` |
| 191 | 691 | CREATE POLICY | `d19_update on photara.location_kinds` |
| 192 | 693 | CREATE POLICY | `d19_control on photara.location_kinds` |
| 193 | 695 | CREATE POLICY | `d19_owner on photara.location_kind_terms` |
| 194 | 697 | CREATE POLICY | `d19_read on photara.location_kind_terms` |
| 195 | 699 | CREATE POLICY | `d19_insert on photara.location_kind_terms` |
| 196 | 701 | CREATE POLICY | `d19_update on photara.location_kind_terms` |
| 197 | 703 | CREATE POLICY | `d19_control on photara.location_kind_terms` |
| 198 | 705 | CREATE POLICY | `d19_owner on photara.locations` |
| 199 | 707 | CREATE POLICY | `d19_read on photara.locations` |
| 200 | 709 | CREATE POLICY | `d19_insert on photara.locations` |
| 201 | 711 | CREATE POLICY | `d19_update on photara.locations` |
| 202 | 713 | CREATE POLICY | `d19_control on photara.locations` |
| 203 | 715 | CREATE POLICY | `d19_owner on photara.storage_roots` |
| 204 | 717 | CREATE POLICY | `d19_read on photara.storage_roots` |
| 205 | 719 | CREATE POLICY | `d19_insert on photara.storage_roots` |
| 206 | 721 | CREATE POLICY | `d19_update on photara.storage_roots` |
| 207 | 723 | CREATE POLICY | `d19_control on photara.storage_roots` |
| 208 | 725 | CREATE POLICY | `d19_owner on photara.project_catalog` |
| 209 | 727 | CREATE POLICY | `d19_read on photara.project_catalog` |
| 210 | 729 | CREATE POLICY | `d19_insert on photara.project_catalog` |
| 211 | 731 | CREATE POLICY | `d19_update on photara.project_catalog` |
| 212 | 733 | CREATE POLICY | `d19_control on photara.project_catalog` |
| 213 | 735 | CREATE POLICY | `d19_owner on photara.project_locators` |
| 214 | 737 | CREATE POLICY | `d19_read on photara.project_locators` |
| 215 | 739 | CREATE POLICY | `d19_insert on photara.project_locators` |
| 216 | 741 | CREATE POLICY | `d19_update on photara.project_locators` |
| 217 | 743 | CREATE POLICY | `d19_control on photara.project_locators` |
| 218 | 745 | CREATE POLICY | `d19_owner on photara.package_observations` |
| 219 | 747 | CREATE POLICY | `d19_read on photara.package_observations` |
| 220 | 749 | CREATE POLICY | `d19_insert on photara.package_observations` |
| 221 | 751 | CREATE POLICY | `d19_update on photara.package_observations` |
| 222 | 753 | CREATE POLICY | `d19_control on photara.package_observations` |
| 223 | 755 | CREATE POLICY | `d19_owner on photara.library_media` |
| 224 | 757 | CREATE POLICY | `d19_read on photara.library_media` |
| 225 | 759 | CREATE POLICY | `d19_insert on photara.library_media` |
| 226 | 761 | CREATE POLICY | `d19_update on photara.library_media` |
| 227 | 763 | CREATE POLICY | `d19_control on photara.library_media` |
| 228 | 765 | CREATE POLICY | `d19_owner on photara.library_change_batches` |
| 229 | 767 | CREATE POLICY | `d19_control on photara.library_change_batches` |
| 230 | 769 | REVOKE ALL | `REVOKE ALL ON photara . library_change_batches FROM photara_api` |
| 231 | 771 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara . library_change_batches TO photara_control` |
| 232 | 773 | CREATE POLICY | `d19_owner on photara.library_changes` |
| 233 | 775 | CREATE POLICY | `d19_control on photara.library_changes` |
| 234 | 777 | REVOKE ALL | `REVOKE ALL ON photara . library_changes FROM photara_api` |
| 235 | 779 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara . library_changes TO photara_control` |
| 236 | 781 | CREATE POLICY | `d19_owner on photara_private.media_objects` |
| 237 | 783 | CREATE POLICY | `d19_control on photara_private.media_objects` |
| 238 | 785 | REVOKE ALL | `REVOKE ALL ON photara_private . media_objects FROM photara_api` |
| 239 | 787 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara_private . media_objects TO photara_control` |
| 240 | 789 | CREATE POLICY | `d19_owner on photara_private.library_streams` |
| 241 | 791 | CREATE POLICY | `d19_control on photara_private.library_streams` |
| 242 | 793 | REVOKE ALL | `REVOKE ALL ON photara_private . library_streams FROM photara_api` |
| 243 | 795 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara_private . library_streams TO photara_control` |
| 244 | 797 | CREATE POLICY | `d19_owner on photara_private.mutation_receipts` |
| 245 | 799 | CREATE POLICY | `d19_control on photara_private.mutation_receipts` |
| 246 | 801 | REVOKE ALL | `REVOKE ALL ON photara_private . mutation_receipts FROM photara_api` |
| 247 | 803 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara_private . mutation_receipts TO photara_control` |
| 248 | 805 | CREATE POLICY | `d19_owner on photara_private.sync_clients` |
| 249 | 807 | CREATE POLICY | `d19_control on photara_private.sync_clients` |
| 250 | 809 | REVOKE ALL | `REVOKE ALL ON photara_private . sync_clients FROM photara_api` |
| 251 | 811 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara_private . sync_clients TO photara_control` |
| 252 | 813 | CREATE POLICY | `d19_owner on photara_private.media_upload_sessions` |
| 253 | 815 | CREATE POLICY | `d19_control on photara_private.media_upload_sessions` |
| 254 | 817 | REVOKE ALL | `REVOKE ALL ON photara_private . media_upload_sessions FROM photara_api` |
| 255 | 819 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara_private . media_upload_sessions TO photara_control` |
| 256 | 821 | CREATE POLICY | `d19_owner on photara.library_contract_state` |
| 257 | 823 | CREATE POLICY | `d19_auth_read on photara.library_contract_state` |
| 258 | 825 | CREATE POLICY | `d19_control on photara.library_contract_state` |
| 259 | 827 | REVOKE ALL | `REVOKE ALL ON photara . library_contract_state FROM photara_api` |
| 260 | 829 | GRANT SELECT | `GRANT SELECT ON photara . library_contract_state TO photara_auth_read` |
| 261 | 831 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara . library_contract_state TO photara_control` |
| 262 | 833 | CREATE POLICY | `d19_owner on photara.project_ownership` |
| 263 | 835 | CREATE POLICY | `d19_auth_read on photara.project_ownership` |
| 264 | 837 | CREATE POLICY | `d19_control on photara.project_ownership` |
| 265 | 839 | REVOKE ALL | `REVOKE ALL ON photara . project_ownership FROM photara_api` |
| 266 | 841 | GRANT SELECT | `GRANT SELECT ON photara . project_ownership TO photara_auth_read` |
| 267 | 843 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara . project_ownership TO photara_control` |
| 268 | 845 | CREATE POLICY | `d19_owner on photara.project_access_policies` |
| 269 | 847 | CREATE POLICY | `d19_auth_read on photara.project_access_policies` |
| 270 | 849 | CREATE POLICY | `d19_control on photara.project_access_policies` |
| 271 | 851 | REVOKE ALL | `REVOKE ALL ON photara . project_access_policies FROM photara_api` |
| 272 | 853 | GRANT SELECT | `GRANT SELECT ON photara . project_access_policies TO photara_auth_read` |
| 273 | 855 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara . project_access_policies TO photara_control` |
| 274 | 857 | CREATE POLICY | `d19_owner on photara_identity.project_access_grants` |
| 275 | 859 | CREATE POLICY | `d19_auth_read on photara_identity.project_access_grants` |
| 276 | 861 | CREATE POLICY | `d19_control on photara_identity.project_access_grants` |
| 277 | 863 | REVOKE ALL | `REVOKE ALL ON photara_identity . project_access_grants FROM photara_api` |
| 278 | 865 | GRANT SELECT | `GRANT SELECT ON photara_identity . project_access_grants TO photara_auth_read` |
| 279 | 867 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara_identity . project_access_grants TO photara_control` |
| 280 | 869 | CREATE POLICY | `d19_owner on photara_identity.project_invitations` |
| 281 | 871 | CREATE POLICY | `d19_auth_read on photara_identity.project_invitations` |
| 282 | 873 | CREATE POLICY | `d19_control on photara_identity.project_invitations` |
| 283 | 875 | REVOKE ALL | `REVOKE ALL ON photara_identity . project_invitations FROM photara_api` |
| 284 | 877 | GRANT SELECT | `GRANT SELECT ON photara_identity . project_invitations TO photara_auth_read` |
| 285 | 879 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara_identity . project_invitations TO photara_control` |
| 286 | 881 | CREATE POLICY | `d19_owner on photara_private.project_invitation_secrets` |
| 287 | 883 | CREATE POLICY | `d19_control on photara_private.project_invitation_secrets` |
| 288 | 885 | REVOKE ALL | `REVOKE ALL ON photara_private . project_invitation_secrets FROM photara_api` |
| 289 | 887 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara_private . project_invitation_secrets TO photara_control` |
| 290 | 889 | CREATE POLICY | `d19_owner on photara.storage_location_specs` |
| 291 | 891 | CREATE POLICY | `d19_read on photara.storage_location_specs` |
| 292 | 893 | CREATE POLICY | `d19_insert on photara.storage_location_specs` |
| 293 | 895 | CREATE POLICY | `d19_update on photara.storage_location_specs` |
| 294 | 897 | CREATE POLICY | `d19_control on photara.storage_location_specs` |
| 295 | 899 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara . storage_location_specs TO photara_api , photara_control` |
| 296 | 901 | CREATE POLICY | `d19_owner on photara.storage_slots` |
| 297 | 903 | CREATE POLICY | `d19_read on photara.storage_slots` |
| 298 | 905 | CREATE POLICY | `d19_insert on photara.storage_slots` |
| 299 | 907 | CREATE POLICY | `d19_update on photara.storage_slots` |
| 300 | 909 | CREATE POLICY | `d19_control on photara.storage_slots` |
| 301 | 911 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara . storage_slots TO photara_api , photara_control` |
| 302 | 913 | CREATE POLICY | `d19_owner on photara.storage_slot_names` |
| 303 | 915 | CREATE POLICY | `d19_read on photara.storage_slot_names` |
| 304 | 917 | CREATE POLICY | `d19_insert on photara.storage_slot_names` |
| 305 | 919 | CREATE POLICY | `d19_update on photara.storage_slot_names` |
| 306 | 921 | CREATE POLICY | `d19_control on photara.storage_slot_names` |
| 307 | 923 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara . storage_slot_names TO photara_api , photara_control` |
| 308 | 925 | CREATE POLICY | `d19_owner on photara.library_variables` |
| 309 | 927 | CREATE POLICY | `d19_read on photara.library_variables` |
| 310 | 929 | CREATE POLICY | `d19_insert on photara.library_variables` |
| 311 | 931 | CREATE POLICY | `d19_update on photara.library_variables` |
| 312 | 933 | CREATE POLICY | `d19_control on photara.library_variables` |
| 313 | 935 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara . library_variables TO photara_api , photara_control` |
| 314 | 937 | CREATE POLICY | `d19_owner on photara.library_variable_values` |
| 315 | 939 | CREATE POLICY | `d19_read on photara.library_variable_values` |
| 316 | 941 | CREATE POLICY | `d19_insert on photara.library_variable_values` |
| 317 | 943 | CREATE POLICY | `d19_update on photara.library_variable_values` |
| 318 | 945 | CREATE POLICY | `d19_control on photara.library_variable_values` |
| 319 | 947 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara . library_variable_values TO photara_api , photara_control` |
| 320 | 949 | CREATE POLICY | `d19_owner on photara.library_variable_names` |
| 321 | 951 | CREATE POLICY | `d19_read on photara.library_variable_names` |
| 322 | 953 | CREATE POLICY | `d19_insert on photara.library_variable_names` |
| 323 | 955 | CREATE POLICY | `d19_update on photara.library_variable_names` |
| 324 | 957 | CREATE POLICY | `d19_control on photara.library_variable_names` |
| 325 | 959 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara . library_variable_names TO photara_api , photara_control` |
| 326 | 961 | CREATE POLICY | `d19_owner on photara.library_expressions` |
| 327 | 963 | CREATE POLICY | `d19_read on photara.library_expressions` |
| 328 | 965 | CREATE POLICY | `d19_insert on photara.library_expressions` |
| 329 | 967 | CREATE POLICY | `d19_update on photara.library_expressions` |
| 330 | 969 | CREATE POLICY | `d19_control on photara.library_expressions` |
| 331 | 971 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara . library_expressions TO photara_api , photara_control` |
| 332 | 973 | CREATE POLICY | `d19_owner on photara.library_expression_dependencies` |
| 333 | 975 | CREATE POLICY | `d19_read on photara.library_expression_dependencies` |
| 334 | 977 | CREATE POLICY | `d19_insert on photara.library_expression_dependencies` |
| 335 | 979 | CREATE POLICY | `d19_update on photara.library_expression_dependencies` |
| 336 | 981 | CREATE POLICY | `d19_control on photara.library_expression_dependencies` |
| 337 | 983 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara . library_expression_dependencies TO photara_api , photara_control` |
| 338 | 985 | CREATE POLICY | `d19_owner on photara.project_media_links` |
| 339 | 987 | CREATE POLICY | `d19_read on photara.project_media_links` |
| 340 | 989 | CREATE POLICY | `d19_insert on photara.project_media_links` |
| 341 | 991 | CREATE POLICY | `d19_update on photara.project_media_links` |
| 342 | 993 | CREATE POLICY | `d19_control on photara.project_media_links` |
| 343 | 995 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara . project_media_links TO photara_api , photara_control` |
| 344 | 997 | CREATE POLICY | `d19_owner on photara_private.scoped_streams` |
| 345 | 999 | CREATE POLICY | `d19_control on photara_private.scoped_streams` |
| 346 | 1001 | REVOKE ALL | `REVOKE ALL ON photara_private . scoped_streams FROM photara_api` |
| 347 | 1003 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara_private . scoped_streams TO photara_control` |
| 348 | 1005 | CREATE POLICY | `d19_owner on photara_private.scoped_change_batches` |
| 349 | 1007 | CREATE POLICY | `d19_control on photara_private.scoped_change_batches` |
| 350 | 1009 | REVOKE ALL | `REVOKE ALL ON photara_private . scoped_change_batches FROM photara_api` |
| 351 | 1011 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara_private . scoped_change_batches TO photara_control` |
| 352 | 1013 | CREATE POLICY | `d19_owner on photara_private.scoped_changes` |
| 353 | 1015 | CREATE POLICY | `d19_control on photara_private.scoped_changes` |
| 354 | 1017 | REVOKE ALL | `REVOKE ALL ON photara_private . scoped_changes FROM photara_api` |
| 355 | 1019 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara_private . scoped_changes TO photara_control` |
| 356 | 1021 | CREATE POLICY | `d19_owner on photara_private.scoped_command_receipts` |
| 357 | 1023 | CREATE POLICY | `d19_control on photara_private.scoped_command_receipts` |
| 358 | 1025 | REVOKE ALL | `REVOKE ALL ON photara_private . scoped_command_receipts FROM photara_api` |
| 359 | 1027 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara_private . scoped_command_receipts TO photara_control` |
| 360 | 1029 | CREATE POLICY | `d19_owner on photara_private.scoped_sync_clients` |
| 361 | 1031 | CREATE POLICY | `d19_control on photara_private.scoped_sync_clients` |
| 362 | 1033 | REVOKE ALL | `REVOKE ALL ON photara_private . scoped_sync_clients FROM photara_api` |
| 363 | 1035 | GRANT SELECT | `GRANT SELECT , INSERT , UPDATE ON photara_private . scoped_sync_clients TO photara_control` |
| 364 | 1037 | REVOKE INSERT | `REVOKE INSERT , UPDATE ON photara . project_media_links FROM photara_api` |
| 365 | 1039 | GRANT SELECT | `GRANT SELECT ON photara . project_media_links TO photara_api` |
| 366 | 1041 | REVOKE ALL | `REVOKE ALL ON ALL TABLES IN SCHEMA photara , photara_identity , photara_private FROM PUBLIC` |
| 367 | 1043 | REVOKE ALL | `REVOKE ALL ON ALL FUNCTIONS IN SCHEMA photara , photara_identity , photara_private FROM PUBLIC` |
| 368 | 1045 | GRANT EXECUTE | `GRANT EXECUTE ON FUNCTION photara_private . authorize_library ( uuid , text ) TO photara_api , photara_control` |
| 369 | 1047 | GRANT EXECUTE | `GRANT EXECUTE ON FUNCTION photara_private . authorize_project ( uuid , uuid , text ) TO photara_api , photara_control` |
| 370 | 1049 | GRANT EXECUTE | `GRANT EXECUTE ON FUNCTION photara_private . can_read_library ( uuid , text ) TO photara_api , photara_control` |
| 371 | 1051 | GRANT EXECUTE | `GRANT EXECUTE ON FUNCTION photara_private . can_project ( uuid , uuid , text ) TO photara_api , photara_control` |
| 372 | 1053 | GRANT EXECUTE | `GRANT EXECUTE ON FUNCTION photara_private . can_project_media ( uuid , uuid , bytea , text ) TO photara_api , photara_control` |
| 373 | 1055 | GRANT EXECUTE | `GRANT EXECUTE ON FUNCTION photara_private . d19_can_library ( uuid , text ) TO photara_api , photara_control` |
| 374 | 1060 | UPDATE photara | `photara.schema_metadata` |

### sqlite / 0007_library_project_access.proposal.sql

| Statement | Line | Kind | Object / operation |
| ---: | ---: | --- | --- |
| 1 | 9 | CREATE TABLE | `library_contract_state` |
| 2 | 29 | CREATE TABLE | `project_ownership` |
| 3 | 51 | CREATE INDEX | `d19_project_ownership_library_state on project_ownership` |
| 4 | 54 | CREATE TABLE | `project_access_policies` |
| 5 | 81 | CREATE TABLE | `project_access_grants` |
| 6 | 107 | CREATE INDEX | `d19_project_access_grants_account_id on project_access_grants` |
| 7 | 109 | CREATE INDEX | `d19_project_access_grants_local_principal_id on project_access_grants` |
| 8 | 111 | CREATE INDEX | `d19_project_access_grants_account on project_access_grants` |
| 9 | 114 | CREATE TABLE | `project_invitations` |
| 10 | 145 | CREATE INDEX | `d19_project_invitations_pending on project_invitations` |
| 11 | 147 | CREATE INDEX | `d19_project_invitations_target on project_invitations` |
| 12 | 149 | CREATE INDEX | `d19_project_invitations_inviter on project_invitations` |
| 13 | 151 | CREATE INDEX | `d19_project_invitations_fk1 on project_invitations` |
| 14 | 153 | CREATE INDEX | `d19_project_invitations_fk4 on project_invitations` |

### sqlite / 0008_storage_locations_bindings.proposal.sql

| Statement | Line | Kind | Object / operation |
| ---: | ---: | --- | --- |
| 1 | 9 | CREATE TABLE | `storage_location_specs` |
| 2 | 29 | CREATE TABLE | `storage_slots` |
| 3 | 53 | CREATE INDEX | `d19_storage_slots_root on storage_slots` |
| 4 | 56 | CREATE TABLE | `storage_slot_names` |
| 5 | 69 | CREATE TABLE | `host_bindings` |
| 6 | 104 | CREATE INDEX | `d19_host_bindings_device_root on host_bindings` |
| 7 | 107 | CREATE TABLE | `host_binding_selections` |
| 8 | 120 | CREATE TABLE | `legacy_external_resource_resolutions` |
| 9 | 134 | CREATE INDEX | `d19_storage_slots_fk6 on storage_slots` |
| 10 | 136 | CREATE INDEX | `d19_host_bindings_fk3 on host_bindings` |
| 11 | 138 | CREATE INDEX | `d19_host_binding_selections_fk1 on host_binding_selections` |
| 12 | 140 | CREATE INDEX | `d19_host_binding_selections_fk2 on host_binding_selections` |

### sqlite / 0009_library_context.proposal.sql

| Statement | Line | Kind | Object / operation |
| ---: | ---: | --- | --- |
| 1 | 9 | CREATE TABLE | `library_variables` |
| 2 | 45 | CREATE INDEX | `d19_library_variables_browse on library_variables` |
| 3 | 48 | CREATE TABLE | `library_variable_values` |
| 4 | 69 | CREATE TABLE | `library_variable_names` |
| 5 | 82 | CREATE TABLE | `library_expressions` |
| 6 | 107 | CREATE INDEX | `d19_library_expressions_owner on library_expressions` |
| 7 | 110 | CREATE TABLE | `library_expression_dependencies` |
| 8 | 128 | CREATE INDEX | `d19_library_expression_dependencies_variable_id on library_expression_dependencies` |
| 9 | 130 | CREATE INDEX | `d19_library_expression_dependencies_slot_id on library_expression_dependencies` |
| 10 | 132 | CREATE INDEX | `d19_library_variables_fk8 on library_variables` |
| 11 | 134 | CREATE INDEX | `d19_library_expression_dependencies_fk2 on library_expression_dependencies` |
| 12 | 136 | CREATE INDEX | `d19_library_expression_dependencies_fk3 on library_expression_dependencies` |

### sqlite / 0010_context_apply_recovery.proposal.sql

| Statement | Line | Kind | Object / operation |
| ---: | ---: | --- | --- |
| 1 | 9 | CREATE TABLE | `device_context_snapshots` |
| 2 | 24 | CREATE INDEX | `d19_device_context_snapshots_run on device_context_snapshots` |
| 3 | 27 | CREATE TABLE | `context_apply_intents` |
| 4 | 57 | CREATE INDEX | `d19_context_apply_intents_state on context_apply_intents` |
| 5 | 60 | CREATE TABLE | `context_apply_receipts` |
| 6 | 80 | CREATE INDEX | `d19_context_apply_receipts_final on context_apply_receipts` |
| 7 | 82 | CREATE INDEX | `d19_context_apply_receipts_operation on context_apply_receipts` |

### sqlite / 0011_scoped_sync.proposal.sql

| Statement | Line | Kind | Object / operation |
| ---: | ---: | --- | --- |
| 1 | 9 | CREATE TABLE | `project_media_links` |
| 2 | 34 | CREATE INDEX | `d19_project_media_links_media on project_media_links` |
| 3 | 37 | CREATE TABLE | `scoped_sync_channels` |
| 4 | 59 | CREATE INDEX | `d19_scoped_sync_channels_library on scoped_sync_channels` |
| 5 | 61 | CREATE INDEX | `d19_scoped_sync_channels_project on scoped_sync_channels` |
| 6 | 64 | CREATE TABLE | `scoped_sync_operations` |
| 7 | 95 | CREATE INDEX | `d19_scoped_sync_operations_queue on scoped_sync_operations` |
| 8 | 98 | CREATE TABLE | `scoped_sync_operation_roots` |
| 9 | 116 | CREATE INDEX | `d19_scoped_sync_operation_roots_entity on scoped_sync_operation_roots` |
| 10 | 119 | CREATE TABLE | `scoped_sync_base` |
| 11 | 133 | CREATE TABLE | `scoped_sync_inbox` |
| 12 | 155 | CREATE INDEX | `d19_scoped_sync_inbox_pending on scoped_sync_inbox` |
| 13 | 158 | CREATE TABLE | `scoped_sync_snapshot_installs` |
| 14 | 179 | CREATE INDEX | `d19_scoped_sync_snapshot_installs_pending on scoped_sync_snapshot_installs` |
| 15 | 181 | CREATE INDEX | `d19_scoped_sync_channels_fk1 on scoped_sync_channels` |
| 16 | 183 | CREATE INDEX | `d19_scoped_sync_channels_fk4 on scoped_sync_channels` |
| 17 | 185 | CREATE INDEX | `d19_scoped_sync_operations_fk3 on scoped_sync_operations` |
| 18 | 187 | CREATE INDEX | `d19_scoped_sync_operation_roots_fk1 on scoped_sync_operation_roots` |
| 19 | 189 | CREATE INDEX | `d19_scoped_sync_operation_roots_fk2 on scoped_sync_operation_roots` |

### sqlite / 0012_d19_guards_and_floor.proposal.sql

| Statement | Line | Kind | Object / operation |
| ---: | ---: | --- | --- |
| 1 | 8 | CREATE TRIGGER | `d19_library_contract_state_no_delete on library_contract_state` |
| 2 | 15 | CREATE TRIGGER | `d19_library_contract_state_update on library_contract_state` |
| 3 | 22 | CREATE TRIGGER | `d19_project_ownership_no_delete on project_ownership` |
| 4 | 29 | CREATE TRIGGER | `d19_project_ownership_update on project_ownership` |
| 5 | 36 | CREATE TRIGGER | `d19_project_access_policies_no_delete on project_access_policies` |
| 6 | 43 | CREATE TRIGGER | `d19_project_access_policies_update on project_access_policies` |
| 7 | 50 | CREATE TRIGGER | `d19_project_access_grants_no_delete on project_access_grants` |
| 8 | 57 | CREATE TRIGGER | `d19_project_access_grants_update on project_access_grants` |
| 9 | 64 | CREATE TRIGGER | `d19_project_invitations_no_delete on project_invitations` |
| 10 | 71 | CREATE TRIGGER | `d19_project_invitations_update on project_invitations` |
| 11 | 78 | CREATE TRIGGER | `d19_storage_location_specs_no_delete on storage_location_specs` |
| 12 | 85 | CREATE TRIGGER | `d19_storage_location_specs_update on storage_location_specs` |
| 13 | 92 | CREATE TRIGGER | `d19_storage_slots_no_delete on storage_slots` |
| 14 | 99 | CREATE TRIGGER | `d19_storage_slots_update on storage_slots` |
| 15 | 106 | CREATE TRIGGER | `d19_storage_slots_no_resurrection on storage_slots` |
| 16 | 113 | CREATE TRIGGER | `d19_storage_slot_names_no_delete on storage_slot_names` |
| 17 | 120 | CREATE TRIGGER | `d19_storage_slot_names_update on storage_slot_names` |
| 18 | 127 | CREATE TRIGGER | `d19_host_bindings_no_delete on host_bindings` |
| 19 | 134 | CREATE TRIGGER | `d19_host_bindings_update on host_bindings` |
| 20 | 141 | CREATE TRIGGER | `d19_host_binding_selections_update on host_binding_selections` |
| 21 | 148 | CREATE TRIGGER | `d19_legacy_external_resource_resolutions_no_delete on legacy_external_resource_resolutions` |
| 22 | 155 | CREATE TRIGGER | `d19_legacy_external_resource_resolutions_update on legacy_external_resource_resolutions` |
| 23 | 162 | CREATE TRIGGER | `d19_library_variables_no_delete on library_variables` |
| 24 | 169 | CREATE TRIGGER | `d19_library_variables_update on library_variables` |
| 25 | 176 | CREATE TRIGGER | `d19_library_variables_no_resurrection on library_variables` |
| 26 | 183 | CREATE TRIGGER | `d19_library_variable_values_no_delete on library_variable_values` |
| 27 | 190 | CREATE TRIGGER | `d19_library_variable_values_update on library_variable_values` |
| 28 | 197 | CREATE TRIGGER | `d19_library_variable_names_no_delete on library_variable_names` |
| 29 | 204 | CREATE TRIGGER | `d19_library_variable_names_update on library_variable_names` |
| 30 | 211 | CREATE TRIGGER | `d19_library_expressions_no_delete on library_expressions` |
| 31 | 218 | CREATE TRIGGER | `d19_library_expressions_update on library_expressions` |
| 32 | 225 | CREATE TRIGGER | `d19_library_expression_dependencies_no_delete on library_expression_dependencies` |
| 33 | 232 | CREATE TRIGGER | `d19_library_expression_dependencies_update on library_expression_dependencies` |
| 34 | 239 | CREATE TRIGGER | `d19_device_context_snapshots_no_delete on device_context_snapshots` |
| 35 | 246 | CREATE TRIGGER | `d19_device_context_snapshots_update on device_context_snapshots` |
| 36 | 253 | CREATE TRIGGER | `d19_context_apply_intents_no_delete on context_apply_intents` |
| 37 | 260 | CREATE TRIGGER | `d19_context_apply_intents_update on context_apply_intents` |
| 38 | 267 | CREATE TRIGGER | `d19_context_apply_receipts_no_delete on context_apply_receipts` |
| 39 | 274 | CREATE TRIGGER | `d19_context_apply_receipts_update on context_apply_receipts` |
| 40 | 281 | CREATE TRIGGER | `d19_project_media_links_no_delete on project_media_links` |
| 41 | 288 | CREATE TRIGGER | `d19_project_media_links_update on project_media_links` |
| 42 | 295 | CREATE TRIGGER | `d19_project_media_links_no_resurrection on project_media_links` |
| 43 | 302 | CREATE TRIGGER | `d19_scoped_sync_channels_no_delete on scoped_sync_channels` |
| 44 | 309 | CREATE TRIGGER | `d19_scoped_sync_channels_update on scoped_sync_channels` |
| 45 | 316 | CREATE TRIGGER | `d19_scoped_sync_operations_no_delete on scoped_sync_operations` |
| 46 | 323 | CREATE TRIGGER | `d19_scoped_sync_operations_update on scoped_sync_operations` |
| 47 | 330 | CREATE TRIGGER | `d19_scoped_sync_operation_roots_no_delete on scoped_sync_operation_roots` |
| 48 | 337 | CREATE TRIGGER | `d19_scoped_sync_operation_roots_update on scoped_sync_operation_roots` |
| 49 | 344 | CREATE TRIGGER | `d19_scoped_sync_base_no_delete on scoped_sync_base` |
| 50 | 351 | CREATE TRIGGER | `d19_scoped_sync_base_update on scoped_sync_base` |
| 51 | 358 | CREATE TRIGGER | `d19_scoped_sync_inbox_no_delete on scoped_sync_inbox` |
| 52 | 365 | CREATE TRIGGER | `d19_scoped_sync_inbox_update on scoped_sync_inbox` |
| 53 | 372 | CREATE TRIGGER | `d19_scoped_sync_snapshot_installs_no_delete on scoped_sync_snapshot_installs` |
| 54 | 379 | CREATE TRIGGER | `d19_scoped_sync_snapshot_installs_update on scoped_sync_snapshot_installs` |
| 55 | 386 | CREATE TRIGGER | `d19_project_ownership_lifecycle on project_ownership` |
| 56 | 393 | CREATE TRIGGER | `d19_project_invitations_terminal on project_invitations` |
| 57 | 400 | CREATE TRIGGER | `d19_library_contract_state_generation on library_contract_state` |
| 58 | 407 | CREATE TRIGGER | `d19_project_access_policies_generation on project_access_policies` |
| 59 | 414 | CREATE TRIGGER | `d19_project_access_grants_principal_insert on project_access_grants` |
| 60 | 421 | CREATE TRIGGER | `d19_host_binding_selections_verified_insert on host_binding_selections` |
| 61 | 428 | CREATE TRIGGER | `d19_project_access_grants_principal_update on project_access_grants` |
| 62 | 435 | CREATE TRIGGER | `d19_host_binding_selections_verified_update on host_binding_selections` |
| 63 | 442 | CREATE TRIGGER | `d19_host_binding_selections_cas on host_binding_selections` |
| 64 | 449 | CREATE TRIGGER | `d19_host_bindings_selected on host_bindings` |
| 65 | 456 | CREATE TRIGGER | `d19_host_bindings_generation on host_bindings` |
| 66 | 463 | CREATE TRIGGER | `d19_scoped_sync_channels_cas on scoped_sync_channels` |
| 67 | 470 | CREATE TRIGGER | `d19_scoped_sync_operations_seal on scoped_sync_operations` |
| 68 | 477 | CREATE TRIGGER | `d19_scoped_sync_inbox_terminal on scoped_sync_inbox` |
| 69 | 484 | CREATE TRIGGER | `d19_scoped_sync_snapshot_installs_terminal on scoped_sync_snapshot_installs` |
| 70 | 491 | CREATE TRIGGER | `d19_context_apply_intents_terminal on context_apply_intents` |
| 71 | 498 | CREATE TRIGGER | `d19_context_apply_receipts_authority on context_apply_receipts` |
| 72 | 505 | CREATE TRIGGER | `d19_storage_slots_target_insert on storage_slots` |
| 73 | 512 | CREATE TRIGGER | `d19_storage_slots_target_update on storage_slots` |
| 74 | 519 | CREATE TRIGGER | `d19_storage_roots_live_dependents on storage_roots` |
| 75 | 532 | UPDATE schema_metadata | `schema_metadata` |
