# LL1 populated retained-chain SQLite verification

Executed on 2026-09-18 with SQLite 3.53.4:

```sh
python3 scripts/test_ll1_protected_sqlite_retained_coverage.py
```

**All 79 source tables are now populated.** This separate disposable harness extends [the 41-table D-disposition experiment](LL1_PROTECTED_SQLITE_COVERAGE.md) to every previously unseeded table. The unchanged generation-two catalog contains 79 tables, 137 FK edges and 190 original triggers; fingerprint `02c638bb087862adb764f28d6c3caf86c812a71b28da0170bc409f3a1211d220`.

The measured successful deletion removes **82 rows across 73 source tables**, preserving the other Library's complete aggregate and six populated retained tables: `account_cache`, `local_device`, `normalization_policies`, `onboarding_credential_references`, `onboarding_session`, and `schema_metadata`. It produces **15 detached terminal evidence rows** in a fixture-only relation, plus the existing terminal receipt, removed marker and inventory entry. **No source table is unseeded.** This is all-table populated transaction evidence, not production authentication, canonical receipt codec, or exhaustive lifecycle-state validation.

## Additional chains populated

| Tables | Selected state and purpose |
| --- | --- |
| `sync_targets`, `mutations`, `local_changes`, `mutation_baselines`, `sync_object_state`, `sync_outbox`, `sync_inbox`, `sync_cursors`, `sync_conflicts`, `media_transfers`, `remote_mutation_receipts`, `local_mutation_dispositions`, `sync_snapshot_installs` | Paused target, rejected outbox with matching receipt and discarded disposition, applied inbox, resolved conflict, completed media transfer and installed snapshot. Tests full mutation/target child closure and detached mutation replay identity/digests. |
| `scoped_sync_channels`, `scoped_sync_operations`, `scoped_sync_operation_roots`, `scoped_sync_base`, `scoped_sync_inbox`, `scoped_sync_snapshot_installs` | Paused Project channel, acknowledged operation with request and receipt digests, applied inbox and installed snapshot. Exercises ownership through channels for tables without a direct Library column. |
| `operation_intents`, `operation_events`, `recovery_items`, `recovery_publications` | Committed save-package operation, event, retained recovery payload and verified publication. Copies terminal identity/request digest/outcome/commit evidence before deleting the live payload chain. The fixture path `recovery/untouched` is never resolved or opened. |
| `context_apply_intents`, `context_apply_receipts` | Settled Library-authority intent with `local-applied` final receipt. Copies identity/request digest and result digest/outcome. |
| `onboarding_intents`, `onboarding_dispositions`, `onboarding_replacements`, `onboarding_receipts`, `library_cloud_bindings`, `onboarding_library_selection` | Authenticated unknown intent with admitted `abandoned-expired` disposition, replacement admitted while prepared and then marked applied with a receipt, signed-out cloud binding, and target-bound singleton selection. Original admission and immutability triggers remain in force. The original intent's state remains unknown; its additive disposition closes that work. |
| `onboarding_credential_references` | Shared credential reference survives byte-for-byte. No secret-store call or sign-out action occurs. |
| `project_creation_intents` | Complete intent with opaque stage pin and terminal result. Copies operation/Project identity, request digest, state and terminal result, then removes live destination/staging references without touching the filesystem. |

Every new target chain has an unrelated-Library counterpart except the singleton selection, which intentionally names the target. Both logical Project-keyed tables from the original fixture remain in scope. The insertion ledger independently records target row membership and must equal the FK-plus-logical closure before execution. Source snapshots enforce exact set subtraction afterward, including rows with cross-Library snapshot references.

## Transaction and failure evidence

The test extends only its own process's table whitelist, restoring the base whitelist in `finally`. Original source table definitions, FK actions and migration hashes remain unchanged. Foreign keys stay enabled; global FK deferral stays disabled. The existing narrow transaction manifest exception and RESTRICT-preserving current-name strategy remain behind the same executor contract.

Before granting the transaction deletion permit, the harness checks target operation, context-apply, onboarding and Project-creation chains for unresolved work. A separately inserted `operation_intents` row in `prepared` state demonstrably blocks deletion with `pending_operation:operation_intents`. The complete pending snapshot remains unchanged and no detached evidence is written. The fixture then cancels this synthetic operation through the existing ordinary revision-checked UPDATE path and includes its cancelled identity in the final retirement.

Detached transfer executes inside the same `BEGIN IMMEDIATE` transaction as deletion. Its fixture relation has no FK to deleted Library rows. The inventory insertion verifies exact agreement with the expected evidence set and verifies that every evidence row has the matching retirement operation/Library receipt. Evidence has unconditional no-update/no-delete guards. Eight injected failures pass exact rollback assertions:

- Failure after cycle rename; premature commit with unsatisfied name backlink; omitted logical rows.
- Failure after aggregate deletion or after the terminal receipt; marker mismatch.
- Omitted detached evidence or detached evidence bound to the wrong retirement.

Each refusal restores every source row, leaves all terminal/evidence relations empty, revokes the permit, ends the transaction, and leaves FK settings unchanged. The successful retry produces the evidence once. An idempotent replay returns `removed` without changing it. Direct UPDATE and DELETE against the detached evidence are rejected. Final `foreign_key_check` is empty.

## Review boundary and unverified semantics

The detached relation is deliberately a **test-only evidence format**. The executable `EVIDENCE` map lists every copied source column. It retains operation identities, request/result digests, terminal outcomes, selected predecessor/replacement bindings and publication commit evidence. It omits live request/payload/destination data. It does not assert that these fields are the complete production replay DTO, nor that synthetic `{}` receipt/absence bytes authenticate anything. Production canonical codecs and receipt verification must supply that proof at the service boundary. The Python actor placeholder remains inherited from the base mechanical fixture.

All 79 tables are seeded, but the following lifecycle semantics still require implementation review and dedicated tests:

- Legacy and scoped sync: pending/unknown/sealed transport states, replacement/predecessor chains, exact minimal terminal DTOs and stale transport replies. This harness selects completed states and does not authorize pending transport deletion generically.
- Recovery/context: the other unresolved states and unknown-result paths, Project-authority publication and final receipt/result agreement. The pending save-package blocker is executed; the other three blocker predicates are present but not exhaustively exercised.
- Onboarding: cryptographic/session authentication, complete replacement chains across both Libraries or environments, exact authenticated absence/receipt preservation, and reconciliation with the newer service selection model. The SQL admission conditions alone do not authenticate the synthetic fixture bytes.
- Project creation: remote-committed/dispatching/publication recovery, stale replies and remote-removal disposition; no staging cancellation or package authority is exercised.
- Terminal records: production table design, exact minimal evidence fields, durable replay lookup keyed by semantic operation identity (the fixture uses source rowid only as an internal test key), and service-only write privileges. The fixture's inventory-time completeness guard proves the chosen execution sequence; it is not a general production transaction authorization mechanism.

These are existing T/B contract implementation obligations, not newly chosen product behavior. Populating the full schema found no independent reason for a NO ACTION migration and no new product/security policy decision. No production deletion, live-data access, production schema migration, package/media mutation, or external publication was performed.
