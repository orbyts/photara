# LL1 full-schema inventory: decision gate and remaining proof

Status: **static review only, 2026-09-18; stop for physical-design review**.
No migration, SQL execution, production modification or LL2 approval in this slice.
The [generated inventory](verification/LL1_FULL_SCHEMA_STATIC_INVENTORY.md) and
[read-only analyzer](../../scripts/audit_ll1_schema_inventory.py) enumerate every
baseline table, incoming FK, trigger, current service policy/function and ordered
migration privilege statement. The [38/40-case experiment](verification/LL1_RETIREMENT_CONSTRAINT_FIXTURE.md)
still proves only its minimal slot/variable fixtures.

## New gate: scoped receipt/batch cycle

The [physical review](LL1_PHYSICAL_SCHEMA_AND_PRIVILEGE_REVIEW.md#exact-fk-changes-and-closure-findings)
incorrectly grouped the scoped feed cycle with the older unscoped deferred
NO ACTION cycle. The actual scoped edges are **both RESTRICT**:

| Incoming edge | Exact source |
| --- | --- |
| `photara_private.scoped_command_receipts(accepted_stream_id,accepted_epoch,accepted_sequence,library_id)` → `photara_private.scoped_change_batches(stream_id,epoch,sequence,library_id)` | [0011_scoped_sync.sql, line 118](../../crates/photara-service/migrations/postgres/0011_scoped_sync.sql); generated PostgreSQL E102 |
| `photara_private.scoped_change_batches(library_id,operation_id)` → `photara_private.scoped_command_receipts(library_id,operation_id)` (`d19_batch_receipt`) | [0011_scoped_sync.sql, line 129](../../crates/photara-service/migrations/postgres/0011_scoped_sync.sql); generated PostgreSQL E104 |

Both say `DEFERRABLE INITIALLY DEFERRED`, but their RESTRICT actions cannot be
postponed to transaction commit. For an accepted receipt with its matching batch,
ordinary **separate-statement** child-first deletion has no starting side. The four
proposed slot/variable current-name FK changes do not touch this component; their
minimal fixture success is insufficient for full PostgreSQL retirement.

This is a physical-design gate, not evidence that LL0 hard deletion should change.
Alternatives for explicit review, **none selected or implemented**:

- Review changing one scoped-cycle action to deferred NO ACTION, identify the
  resulting exact deletion order, and preserve its immutable/ordinary-write guards.
  Changing the receipt→batch edge would permit batches before receipts; changing
  batch→receipt would permit receipts before batches, subject to all other edges.
- Investigate a single-statement multi-table deletion strategy retaining both
  actions, but accept it only after PostgreSQL execution and complete trigger/RLS
  proofs. This analyzer neither proves it works nor recommends relying on it.

Do not null accepted coordinates, rewrite receipt outcomes, disable constraints,
or keep live aggregate rows as a workaround. Those evade accepted evidence/closure
requirements. Stop before selecting a new FK rewrite or enabling retirement.

## Verified inventory and corrections

| Static source coverage | SQLite | PostgreSQL |
| --- | ---: | ---: |
| Tables, exactly matched to LL1 disposition matrix | 79 | 59 |
| Actual FK edges | 137 | 114 |
| Trigger declarations | 190 | 122 |
| Triggers whose event includes DELETE | 54 | 84 |
| Deferred constraint triggers | 0 | 14 |
| Current function definitions / policies / ordered privilege statements | n/a | 52 / 173 / 98 |

The previous physical review's coarse SQLite count **138** is corrected to **137**
actual REFERENCES clauses. Word-boundary source count and token-aware extraction
agree; no migration source changed. Both source-set fingerprints still match the
physical review. Eight cyclic components are identified per engine, including
self-references; Location Kind merge self-edges inside a larger component also
need ordering analysis, not just the Location/Person/Organization self-edges.

## Exact next proof gaps, without new design authority

| Gap beyond minimal fixtures | Required evidence before approval |
| --- | --- |
| Complete FK deletion order | Resolve the new scoped-cycle gate; cover every incoming E-edge, remaining multi-table/self-reference component and tombstone. `photara_identity.account_defaults` is retained and blocks removal when it targets the Library; reachability never grants deletion authority. |
| Two local tables lack any Library FK path | `legacy_external_resource_resolutions` and `device_context_snapshots` require typed pre-removal Project-ownership/codec closure. Missing or ambiguous scope blocks completion; no package scan or broad shared-row deletion. |
| Every guard, not pattern matching | Review all 190/122 T-entries, including non-DELETE admission/revision/immutability triggers. Shared functions require table-specific retirement exceptions without widening retained account/default/authentication behavior. |
| Deferred state and actual visibility | PostgreSQL `guard_owner`, `guard_stream_commit`, `guard_batch`, `d19_managers_at_commit`, `d19_feed_at_commit`, `close_onboarding_challenge` and `close_onboarding_receipt` have 14 trigger consumers, all enumerated. Prove final-state checks under real owner/RLS visibility and preserve unrelated aggregates; minimal current-name checks do not exercise these functions. |
| Permit authority and terminal publication | The debt FK prevents committed permit rows, not unauthorized creation/use. A complete protected facade must enforce actor/default/last/billing checks, exact manifest and request hash, evidence staging, root deletion, matching receipt/marker and inventory publication. Direct privileged writes can otherwise omit required terminal evidence. |
| Generation and locking | Map all tables and background/native/service mutation entry points to aggregate/access invalidation; cover shared retained account/device changes too. Source inventory does not prove common lock order or old-writer refusal. |
| Migration/compatibility | Full SQLite FK-enabled rebuild, indexes/triggers, fresh/upgrade parity and reader/writer floors remain unproved. This inventory is not migration input or authorization to replace historical definitions. |
| Logical evidence/privacy/session closure | FK reachability excludes canonical payload IDs, immutable exceptions, outbox replay, stale restores, inventory disclosure and PS3/PS4 activation/capsule ownership. Their existing LL1 gates remain unchanged. |

## Role boundary: source facts, not an effective privilege proof

The inventory preserves all 98 GRANT/REVOKE/default-privilege statements in order,
and resolves replaced policies/functions (including `authorize_library` from
0012 rather than its superseded 0007 body). Schema-wide grants apply at their
statement point, not automatically to future tables. Existing role names are
referenced by migrations; this does not establish their deployed attributes.

Separately inspected [runtime role template](../../crates/photara-service/deploy/runtime_roles.sql)
defines three LOGIN/INHERIT/NOSUPERUSER/NOBYPASSRLS runtime identities with distinct
`photara_api`, `photara_control`, `photara_auth_read` memberships. The
[runtime validator](../../crates/photara-service/src/runtime.rs) checks login,
superuser/bypass/admin flags and owner membership; its entry points have distinct
membership checks. Neither file creates the proposed retirement executor/facade.
No deploy/provisioning script was run and no deployed role catalog was consulted.

Required security proof still includes actual role inheritance, default privileges,
PUBLIC EXECUTE, SECURITY DEFINER owners/search paths, forced RLS hidden rows,
cross-account denial and permit-table denial. Existing broad control grants and
caller context settings must not be treated as a new retirement capability.

## Reproduction and analyzer limits

```sh
python3 scripts/audit_ll1_schema_inventory.py
python3 scripts/audit_ll1_schema_inventory.py --check docs/architecture/verification/LL1_FULL_SCHEMA_STATIC_INVENTORY.md
git diff --check
```

The deterministic lexical scanner handles comments, quoted/dollar-quoted bodies,
SQLite trigger blocks, inline/table FKs and multi-action ALTER ADD columns/FKs.
It cross-checks token declaration counts, all endpoint columns, implicit referenced
primary keys, trigger function targets and every LL1 table disposition. Generated
links point to declaration starts, not necessarily the FK's interior line.
It is not a complete SQL grammar, engine introspection, recursive function-body
dependency analysis, row-level scope proof or effective ACL calculation. Current
policies are listed with command/roles and source-linked predicates, not evaluated.
No additional isolated SQL tests were run in this static slice. Review the new
scoped-cycle decision before expanding the minimal engine fixture.
