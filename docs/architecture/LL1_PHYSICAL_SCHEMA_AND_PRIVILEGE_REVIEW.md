# LL1 physical schema and privilege delta — unnumbered review

> **Historical proposal, superseded for current execution.** Later full-schema
> PostgreSQL and SQLite protected-executor fixtures close the populated cycles
> while preserving the existing `RESTRICT` actions; current evidence does not
> justify the `NO ACTION` changes proposed below. Begin with the
> [current protected-executor contract](verification/LL1_PROTECTED_EXECUTOR_CONTRACT.md),
> then follow its authorization, dense PostgreSQL, retained SQLite and service
> handoff links. The cycle counts and recommendations below record the earlier
> static review and must not be treated as current implementation direction.

Status: **proposal and static review only, 2026-09-18**. Inputs are accepted
[LL0](LIBRARY_LIFECYCLE.md) and the pending [LL1 R1–R5 recommendations](LL1_TYPED_CONTRACT_AND_SCHEMA_DELTA.md).
No migration ordinal, executable installer, DDL execution, production code,
live-data operation, or deployment is included. Physical names below are proposed
names. LL1 acceptance and LL2 implementation/rollout authority remain separate.

## Baseline and limits of proof

Inspected source at `c6b664640d59348a4d442b8fbaf3598cf492bc34`, with no migration
source changes. Counts below are lexical declarations in the ordered migration
files, **not** live catalog introspection or a proof that deletion works.

| Source | Tables | Trigger declarations | `REFERENCES` clauses | Source-set SHA-256 |
| --- | ---: | ---: | ---: | --- |
| [SQLite](../../crates/photara-library/migrations/generation_two/0001_local_identity.sql) through `0015_project_creation.sql` | 79 | 190 | 138 | `02c638bb087862adb764f28d6c3caf86c812a71b28da0170bc409f3a1211d220` |
| [PostgreSQL](../../crates/photara-service/migrations/postgres/0001_identity_library.sql) through `0014_onboarding.sql` | 59 | 122 | 114 | `7228575621958f4f5063e40bbb80ec5102d00b232bf21d3fb95bad4779c9c638` |

Fingerprint algorithm: sorted migration basenames; SHA-256 over each basename,
NUL, exact file bytes, NUL. Existing migration numbers identify the inspected
baseline only; none is assigned to this delta. [LL1's table disposition matrix](LL1_TYPED_CONTRACT_AND_SCHEMA_DELTA.md#enumerated-table-dispositions)
still covers every baseline table. The proposals below add relations requiring
their own disposition and trigger coverage before an executable migration exists.

Static findings: current removal is blocked by immutable/no-delete triggers,
two unavoidable `RESTRICT` cycles in each engine, and recovery records whose
Library FKs cannot survive hard deletion. Existing roles/RLS do not establish
the new retirement authority. Final FK/trigger, privilege and failure proofs
require separately authorized disposable databases; this document does not claim
those checks passed.

## Physical encoding and common keys

The following are precise relational declarations in review notation, not SQL
that can be installed. `?` means nullable; all other columns are NOT NULL. PK,
UK, FK and CHECK denote the required physical constraints. No implied cascade.
Each listed bounded enum is implemented by CHECK, not free-form extensions JSON.

| Symbol | SQLite | PostgreSQL |
| --- | --- | --- |
| `Id` | BLOB, exactly 16 bytes, nonzero | UUID, nonnil |
| `Hash` | BLOB, exactly 32 bytes | BYTEA, exactly 32 bytes |
| `Counter` | INTEGER, 1..9223372036854775807 | BIGINT, same bounds |
| `Position` | INTEGER, 0..9223372036854775807 | BIGINT, same bounds |
| `Time` | INTEGER, bounded UTC Unix milliseconds using existing validator | TIMESTAMPTZ(3), existing supported timestamp range |
| `Bytes(n)` | BLOB, byte length 1..n | BYTEA, octet length 1..n |
| `Name` | TEXT, byte length 1..128, validated LL0 whitespace/control rules | TEXT COLLATE "C", same UTF-8 byte rule |
| `Flag` | INTEGER CHECK IN (0,1) | BOOLEAN |

SQLite tables are STRICT. All copied canonical bytes retain their original hash;
Rust validates pinned codec and digest rather than treating JSON serialization
as canonical bytes. PostgreSQL may additionally verify SHA-256 in constraints
where supported by the existing baseline. No SQLite SHA extension is assumed.

Local historical keys use `A = authority_id`, and `P = (principal_kind,principal_id)`
with kind `account|local`. `O = (A,P,operation_id)` is the operation key. Local
authorities distinguish service environment/epoch from database identity; a cloud
projection cannot be rerouted as a local Library. Service operation keys are
`O = (account_id,operation_id)` within the explicitly identified service authority
epoch. Device IDs are provenance, not part of the idempotency key: a new eligible
device for the same account must be able to recover an old receipt.

Historical Library/Project IDs deliberately have **no FK** to deleted aggregates.
They have a typed historical use and cannot be joined by ordinary domain readers
to recreate access. Live pointers have FKs and are explicitly cleared/deleted;
this distinction is part of the schema, not a nullable-FK shortcut.

## SQLite relation delta

All names in this section are in the local database. `O` expands to four columns;
local `principal_id` is the account UUID or verified local-principal UUID, never
an Auth0 subject or path. Historical scope rows do not depend on `account_cache`
being hydrated by the current onboarding projection.

| New relation | Columns and physical constraints |
| --- | --- |
| `lifecycle_authorities` | `authority_id Id PK; kind TEXT CHECK local\|cloud; epoch Id; database_id Id?; environment_id TEXT?` (1..255 bytes). CHECK local iff database_id set and environment null; cloud iff environment set and database null. Partial UK(database_id,epoch) for local; UK(environment_id,epoch) for cloud. Rows retained as evidence scope. |
| `library_lifecycle_admission` | `library_id Id PK FK libraries RESTRICT; aggregate_generation Counter; bootstrap_default Flag`. Root identity immutable. Generation advances on covered effects; default provenance is installed from verified bootstrap state, never inferred from name/path. Delete after descendants and before Library root. |
| `lifecycle_intents` | `O PK; target_library_id Id; action TEXT CHECK create\|rename\|remove; request_canonical Bytes(65536); request_sha256 Hash; initiating_device_id Id; state TEXT CHECK prepared\|dispatching\|unknown\|terminal; created_at Time; updated_at Time`. FK A→authorities RESTRICT; **no Library FK**. Target/create coordinates and request immutable. On removed target, eliminate command payload after detached disposition is durable; terminal intents are not an archive. |
| `lifecycle_receipts` | `O PK; historical_library_id Id; action enum; result TEXT CHECK created\|renamed\|removed\|rejected; request_sha256 Hash; result_canonical Bytes(65536); result_sha256 Hash; initiating_device_id Id; committed_at Time; reviewed_revision Counter?`. FK A→authorities RESTRICT; UK(O,historical_library_id,result) for marker binding. Strict terminal codec excludes path/token/membership/stream payload. Immutable, retained. |
| `removed_library_identities` | `A; historical_library_id Id; P; removal_operation_id Id; result TEXT CHECK removed; terminal_version Counter; removed_at Time`. PK(A,historical_library_id); FK(A,P,removal_operation_id,historical_library_id,result)→receipt's corresponding UK, NO ACTION DEFERRABLE INITIALLY DEFERRED. Immutable marker; no Library FK. |
| `retired_operation_dispositions` | `A; P; source_kind TEXT CHECK creation\|context\|package\|claim\|mutation\|onboarding\|activation; source_operation_id Id; source_request_sha256 Hash; source_result_sha256 Hash?; historical_library_id Id; original_outcome TEXT (closed source-specific enum); disposition TEXT CHECK library-removed; retirement_receipt principal/id; recorded_at Time`. PK(A,P,source_kind,source_operation_id). FK retirement receipt→lifecycle_receipts RESTRICT; target agreement checked by retirement closure. No command bytes or live references. |
| `historical_authentication_evidence` | `A; P; source_operation_id Id; historical_library_id Id; codec TEXT CHECK photara.onboarding.v1; receipt_canonical Bytes(65536); receipt_sha256 Hash; retirement_receipt principal/id; recorded_at Time`. PK(A,P,source_operation_id); FK retirement receipt→lifecycle_receipts RESTRICT. Exact validated receipt only, immutable; ordinary replay readers cannot consume it. |
| `account_inventory_positions` | `A; account_id Id; device_id Id FK local_device RESTRICT; epoch Id; applied_sequence Position; snapshot_high_water Position?; snapshot_after_id Id?; snapshot_complete Flag; privacy_generation Counter; cursor Bytes(2048)?`. PK(A,account_id,device_id); FK A→authorities RESTRICT. CHECK snapshot-after requires high-water; complete pages and delta apply advance position in their cleanup transaction. No Library FK. |
| `account_inventory_snapshot_items` | `A; account_id Id; device_id Id; snapshot_epoch Id; high_water Position; historical_library_id Id; item_canonical Bytes(2048); item_sha256 Hash`. PK(all scope columns,historical_library_id); FK(A,account_id,device_id)→positions RESTRICT. Staging only, not independently selectable; validate epoch/H against positions. Removed/revoked items contain no name. Clear completed/invalidated snapshot rows without deleting domain data by inferred absence. |
| `activation_intents` | `activation_id Id PK; A; P; device_id Id FK local_device; previous_activation_id Id?; request_generation Counter; expected_selection_generation Position; actor_generation Counter; owner_epoch Id; attachment_id Id; attachment_generation Counter; target_library_id Id; target_project_id Id?; barrier_commit_id Id?; barrier_sha256 Hash?; capsule_reference Id?; state TEXT CHECK preparing\|frozen\|ready\|unknown; created_at Time`. FK A→authorities; paired barrier columns; project target requires verified barrier/capsule semantics from PS3/PS4. These target IDs are validated intent coordinates, not live membership proof. One active intent per(A,P,device) through partial UK. |
| `activation_receipts` | `activation_id Id PK; A; P; device_id Id; historical_library_id Id; historical_project_id Id?; committed_generation Counter; request_generation Counter; request_sha256 Hash; committed_at Time`. UK(A,P,device_id,committed_generation); UK(A,P,device_id,activation_id). Immutable, no live Library/Project FK and no capsule/path fields. |
| `device_activation` | `A; P; device_id Id FK local_device; committed_generation Position; activation_id Id?; library_id Id?; project_id Id?; state TEXT CHECK selected\|none\|fenced`. PK(A,P,device_id); FK library_id→libraries RESTRICT; FK(library_id,project_id)→project_ownership NO ACTION DEFERRABLE INITIALLY DEFERRED; FK(A,P,device_id,activation_id)→activation_receipts corresponding UK RESTRICT. CHECK project requires Library; selected requires Library+activation, none/fenced have null live IDs. Receipt identity/generation/target equality validated in publication unit. |

This recommends **one** live `device_activation` row for both Library and Project,
not two separately committed preference stores. `onboarding_library_selection`
becomes a bootstrap input only: one reviewed conversion initializes the new
scoped pointer, then old readers are excluded by floors. Existing receipt bytes
and unrelated account state are preserved. PS3 capsule storage/leases remain
owned by PS3; the new relation conveys no package-write capability.

Separate local review-token storage is ephemeral: `lifecycle_removal_reviews`
with `token_sha256 Hash PK; O; target_library_id Id; reviewed_name Name;
library_revision Counter; aggregate_generation Counter; authorization_generation
Counter; impact_sha256 Hash; actor_device_id Id; issued_at Time; expires_at Time`.
UK(O); exact expiry=issued+300000 ms; no Library FK. It is excluded from aggregate
generation and not retained after terminal outcome. Never store the raw token.

## PostgreSQL relation and privilege delta

Use `photara_private` for terminal evidence, review, inventory and retirement
internals; `photara.library_lifecycle_admission` for the live Library generation.
No GUI selection/activation tables belong on the service. Propose these exact
engine adaptations of the local shapes:

| Relation | PostgreSQL-specific physical shape |
| --- | --- |
| `photara_private.lifecycle_authority` | Singleton boolean PK CHECK true; `authority_id uuid UNIQUE; epoch uuid; environment_id text COLLATE "C" UNIQUE`. IDs nonnil, environment 1..255 bytes; immutable within this installed authority. |
| `photara.library_lifecycle_admission` | `library_id uuid PK FK photara.libraries RESTRICT; aggregate_generation bigint CHECK >=1`. No local bootstrap flag; immutable `photara_identity.account_defaults` remains the source of protected cloud defaults. |
| `photara_private.lifecycle_receipts` | Local receipt shape with O=(account_id uuid,operation_id uuid), FK account_id→accounts RESTRICT; authority_epoch uuid. Historical Library/device IDs remain scalars. UK(authority_epoch,account_id,operation_id,historical_library_id,result). All enums, bounds, immutable result/digests as above. |
| `photara_private.removed_library_identities` | PK(authority_epoch,historical_library_id); epoch/account/removal-operation/Library/result reference exactly the removed receipt UK through deferred NO ACTION. No Library FK. |
| `photara_private.retired_operation_dispositions` | O/source_kind/source_operation key scoped by account and authority epoch; local disposition fields in PostgreSQL types; FK retirement receipt composite key RESTRICT. Preserve original actor scope; it need not be the remover. Copy no command/stream payload. |
| `photara_private.lifecycle_removal_reviews` | Local review shape with O=(account_id,operation_id), bound identity_id/device_id and authority_epoch. FK account→accounts only; token digest PK and O UK; exact five-minute expiry. No live Library dependency or raw token. |
| `photara_private.account_inventory_heads` | `account_id uuid PK FK accounts RESTRICT; epoch uuid; last_sequence bigint CHECK >=0; privacy_generation bigint CHECK >=1; baseline_sequence bigint CHECK BETWEEN 0 AND last_sequence`. Serialize sequence/privacy changes under the account lock. |
| `photara_private.account_inventory_events` | `account_id uuid; epoch uuid; sequence bigint CHECK >0; historical_library_id uuid; kind text CHECK baseline\|available\|renamed\|removed\|access-revoked; item_canonical bytea (1..2048); item_sha256 bytea(32); source_operation_id uuid?; occurred_at timestamptz(3)`. PK(account_id,epoch,sequence); FK account→heads RESTRICT with epoch checked by publisher; no Library FK. Index(account_id,epoch,historical_library_id,sequence DESC). Same source operation can produce multiple Library events, so do not wrongly make operation ID globally unique here. |
| `photara_private.account_inventory_current` | `account_id uuid; historical_library_id uuid; epoch uuid; sequence bigint`. PK(account_id,historical_library_id); FK(account_id,epoch,sequence)→events RESTRICT. Matching Library ID checked by publisher/constraint trigger. Derived only, with no independently writable name/role payload. |

Service does not need a second durable copy of full lifecycle command bytes after
terminal receipt: request hash and strict result are enough for equality/recovery.
The authenticated client stores pending exact requests locally. Execution claiming
and outcome publication must be one transaction/unique operation key; an uncommitted
claim is not durable proof of acceptance. Existing account-scoped onboarding
receipts/challenges stay in place. New `historical_authentication_evidence` is local
only unless a **specific** service codec is shown to require exact byte retention;
generalizing that exception is a review gate.

New-relation removal closure is explicit: delete every matching live/pending
`lifecycle_intents` payload and `lifecycle_removal_reviews` row, including another
actor's now-invalid review, after needed dispositions exist. Invalidate staged
inventory snapshots containing an affected old availability/name projection.
Fence and retire activation intents that depend on the removed source or target,
and clear the matching live `device_activation` pointer. Delete the target's
`library_lifecycle_admission` row. Retain only authority/account inventory positions,
strict historical events/receipts/dispositions, terminal markers and allowlisted
authentication evidence. Retained `account_inventory_current` points to the minimal
removed/access-revoked event; it is not an accessible Library catalog entry.
Do not retain an old name/role in a separate materialized column. Transaction work
rows must be absent at commit. Other scopes and unrelated active intents remain.

### Narrow executor, no ambient delete role

Recommend a NOLOGIN, NOSUPERUSER, NOBYPASSRLS `photara_lifecycle_executor` role,
not inherited by service logins. It owns only the reviewed SECURITY DEFINER facade
and internal functions. Migration administration installs it; no runtime CREATE,
ALTER, TRUNCATE, role membership, trigger disable or `session_replication_role` grant.

| Principal | Proposed access |
| --- | --- |
| PUBLIC, `photara_api`, `photara_auth_read` | No direct access to new private tables or retirement functions. No new delete capability. |
| `photara_control` | EXECUTE only on explicit prepare/commit/query/inventory facade signatures. No direct permit/manifest/event/evidence insertion or protected-row deletion. Existing unrelated authority stays unchanged. |
| `photara_lifecycle_executor` | Explicit SELECT/INSERT for permitted new evidence/inventory; narrowly specified UPDATE for heads/current/reviews; DELETE only on reviewed D/T table allowlist and ephemeral work rows. No schema-wide `ON ALL TABLES` grant, no table ownership or BYPASSRLS. |
| Migration owner | Installs reviewed constraints/triggers/policies and grants; never used for runtime lifecycle. |

Facade signatures for physical review: `prepare_library_removal(bytea) → bytea`,
`commit_library_removal(bytea) → bytea`, `query_library_lifecycle(bytea) → bytea`,
`read_library_inventory(bytea) → bytea`. They consume strict bounded typed
envelopes from the trusted authenticated service; bytes or caller-set actor/GUC
fields alone are not authentication. Fixed fully qualified object references and
`search_path=pg_catalog` exclude caller-writable schemas. Public EXECUTE is revoked
explicitly after creation. Unsupported commands, codecs and unknown table tags fail.

**Privilege gate:** current [service roles/RLS](../../crates/photara-service/migrations/postgres/0007_privileges.sql)
and [runtime role checks](../../crates/photara-service/src/runtime.rs) do not implement
this role/facade. Forced Library RLS would deny the new executor unless reviewed
policies explicitly admit the exact current transaction's retirement target.
That policy predicate must read a protected permit, not trust `photara.library_id`.
Prepare/receipt/inventory need separate narrowly scoped read policies; they cannot
reuse owner membership after deletion. The trusted-service credential boundary
remains explicit; this design does not claim to resist arbitrary code already
holding all service-control credentials. Role creation and SECURITY DEFINER/forced
RLS changes require explicit security approval and disposable denial tests.

## Exact FK changes and closure findings

`RESTRICT` does not become deferred merely because its FK says DEFERRABLE:
[SQLite action semantics](https://www.sqlite.org/foreignkeys.html#fk_actions) and
[PostgreSQL constraint semantics](https://www.postgresql.org/docs/current/ddl-constraints.html#DDL-CONSTRAINTS-FK)
both distinguish it from deferred NO ACTION. Change only the unavoidable
**current-name backlinks** below; retain each child→parent RESTRICT edge.

| Engine / referencing columns | Referenced columns | Unnumbered proposed delta |
| --- | --- | --- |
| SQLite `storage_slots(library_id,slot_id,current_name)` | `storage_slot_names(library_id,slot_id,name)` | RESTRICT → NO ACTION, keep DEFERRABLE INITIALLY DEFERRED. |
| SQLite `library_variables(library_id,variable_id,current_name)` | `library_variable_names(library_id,variable_id,name)` | Same. |
| PostgreSQL `photara.storage_slots(library_id,slot_id,current_name)`, constraint `d19_slot_current_name` | `photara.storage_slot_names(library_id,slot_id,name)` | Same. |
| PostgreSQL `photara.library_variables(library_id,variable_id,current_name)`, constraint `d19_variable_current_name` | `photara.library_variable_names(library_id,variable_id,name)` | Same. |

Source: [local slots](../../crates/photara-library/migrations/generation_two/0008_storage_locations_bindings.sql),
[local variables](../../crates/photara-library/migrations/generation_two/0009_library_context.sql),
[service slots](../../crates/photara-service/migrations/postgres/0009_storage_locations.sql),
[service variables](../../crates/photara-service/migrations/postgres/0010_library_context.sql).
Names can then be deleted first inside admitted retirement; parent deletion follows
after all remaining dependents. Final FKs must still be satisfied. Ordinary name/root
delete triggers remain denying unless the exact row is in the admitted retirement
manifest. This is not permission to disable FKs or widen ordinary deletion semantics.
SQLite needs a separately reviewed table rebuild to change inline FKs; the rebuild
and preservation of dependent triggers/indexes are **not specified as an executable
migration here** and must prove foreign-key-enabled cutover before approval.

Other statically identified strongly connected components:

| Component | Closure disposition / remaining proof |
| --- | --- |
| Both engines: Location Kind↔term claim | Both cyclic claim edges already deferred default NO ACTION. Delete claims and kinds in one unit after incoming Location/merge/media references are accounted for; preserve ordinary claim immutability. |
| SQLite: catalog→selected observation/locator→catalog | Catalog selection edges already deferred NO ACTION. Delete observation projection children, observations, then locators, then catalog; remaining incoming roots/bindings/ownership must be enumerated. No need to null authored/catalog identity to break the cycle. |
| Service: old receipt↔change batch; scoped receipt↔scoped batch | Existing cycle FKs are deferred NO ACTION. Stage permitted detached evidence, delete changes and both cycle sides, then stream. Deferred feed closure must see a completely absent target aggregate, not a retained empty stream. |
| Both engines: Person/Organization merge self-reference, Location parent chain; SQLite scoped-operation replacement chain | RESTRICT self-edges require verified reverse dependency order. Do not assume valid data is acyclic merely because the type suggests a hierarchy. Invalid/cyclic data blocks removal until an explicitly reviewed repair or further narrow FK delta; no constraint bypass. |

Two local tables have **no Library FK path** and need typed logical closure:

- `legacy_external_resource_resolutions`: join `project_id` to the complete
  pre-removal `project_ownership` set for the target Library; cross-check catalog
  membership and collect the full composite key `(device_id,project_id,
  legacy_binding_id,source_object_sha256)`. Never resolve external/file bindings.
- `device_context_snapshots`: the same verified Project ownership closure selects
  `(snapshot_id,device_id,project_id,run_id)`; inspect the recognized typed context
  codec for additional live Library references. Do not scan packages to infer scope.

An unowned, ambiguously owned or unknown-codec row cannot be silently classified as
unrelated. Flag `ClosureUnresolved` and block voluntary removal pending a reviewed
classification. Remote-removal cleanup may fence immediately but stays visibly
pending until safe local closure is known; it must not fabricate completed cleanup.
Historical source-Library IDs in **another Library's** package-derived snapshots
are not permission to delete that other Library's rows. Classify those fields as
historical and prevent live-authority lookup, or explicitly resolve a real live
reference; preserve unrelated rows byte-for-byte.

## Guarded retirement and trigger replacement plan

Add transaction work relations, with no Library FK so they can validate root
deletion: `retirement_work(operation key PK, target_library_id, request_sha256,
reviewed_generations, manifest_sha256, transaction_binding, debt_id)` and
`retirement_rows(operation key FK work, table_tag SMALLINT/INTEGER, row_key,
source_sha256, disposition_kind, evidence_key?, PRIMARY KEY(operation,table_tag,row_key))`.
Table tags are a closed generated allowlist, not names supplied by clients.
`row_key` is a bounded array of the table's typed PK fields (SQLite JSON TEXT up to
4096 bytes; PostgreSQL JSONB array with equivalent bound), generated by that table's
fixed trigger expression. It is internal comparison material, not canonical wire
JSON. Rust verifies full source payload digest under the same locked snapshot;
SQLite triggers do not pretend to compute SHA-256.

Require **no committed work permits**. Proposed physical guard: a
`lifecycle_commit_guard(guard_id INTEGER PRIMARY KEY CHECK guard_id=0)` parent;
work has `debt_id INTEGER CHECK debt_id=1` and a deferred NO ACTION FK to it.
No valid parent 1 can exist. An open work row therefore makes commit fail until
all work rows are removed in the transaction. Manifest children are deleted first.
This is a candidate integrity mechanism, not a tested migration or authorization
primitive. PostgreSQL additionally binds permit to its actual transaction ID;
SQLite private-repository confinement plus single-writer transaction supplies its
host boundary, without inventing a SQLite transaction-ID function.

Required admitted transaction order:

1. Validate floors, authenticated actor/device, owner/controller, default/last,
   billing/pending-work policy and exact two-gate review under R2's lock order.
   Compute complete old-row closure and manifest; mismatch fails before deletion.
2. Insert protected work/manifest, copy only allowed detached evidence, and verify
   copied digests/keys. Every old-row DELETE exception matches target, operation,
   actual transaction, table tag and typed PK; full-row digest was already verified
   with no concurrent writer possible.
3. Delete child rows/cycles in the proven order, then generation/controller row and
   Library root. Insert receipt, terminal marker and account invalidations in this
   same transaction. New terminal relations have no live aggregate dependencies.
4. Check zero remaining owned/live references and preserved unrelated rows.
   PostgreSQL explicitly forces named **domain** deferred constraints/triggers
   while the permit still exists, excluding the intentionally unsatisfied work-debt
   FK. Then remove work rows and force remaining constraints. SQLite has no deferred
   trigger layer: remove work, run FK closure checks and commit deferred FKs.
5. Commit once. Any failure rolls back evidence/deletion/invalidation together.
   Cloud receipt and later local cleanup remain separate transactions with truthful
   reconciliation-pending status. Files/object stores are never compensation.

| Existing guard family | Required narrow change / proof |
| --- | --- |
| SQLite `*_no_delete`, `*_immutable_delete`, onboarding retained guards, `project_creation_retained` | Replace in the new approved schema with same ordinary denial plus exact DELETE-only manifest exception. Preserve UPDATE identity/revision checks. Inventory every declaration; naming patterns alone are not the implementation. |
| SQLite D19 context/snapshot/host-binding guards | Add table-specific ownership resolution and matched permit check for logical children. A missing direct Library column must not bypass or permanently block the legitimate path. |
| PostgreSQL `immutable_record`, `claim_receipt_immutable`, `d19_mutable`/`d19_immutable`, admission/authorization triggers | Keep denial for retained account/default/provider rows. Split shared functions or dispatch by a closed reviewed relation identifier so an exception cannot spread to an unrelated table. Matched retirement DELETE may skip revision increments, never authorization or row ownership. |
| PostgreSQL `d19_managers_at_commit` / scoped feed closure | Complete removed aggregate should satisfy absence, while surviving aggregates retain existing invariants. Test under actual forced RLS and function owner: an invisible surviving orphan must not make closure appear empty. |
| New receipt/marker/evidence and authority epoch | Reject UPDATE/DELETE. Permit insertion only from the validated terminal publication path. Marker and receipt must agree on Library, operation, actor, outcome and epoch. |

No proof is claimed until complete generated replacement definitions and disposable
negative tests cover all 190/122 declarations and new guards. This draft provides
the replacement rules and known blockers; it intentionally does not provide a
partially effective “delete everything” function.

## Generation and lock coverage

Use the separate admission relation for aggregate counters so advancing one does
not recursively dirty `library_contract_state` or increment the Library name
revision. Generation triggers act only on effective covered mutations, not reads;
counter rows themselves, ephemeral review tokens, receipt lookup, inventory
delivery position and device activation do not recurse into generation updates.
Manifest-admitted retirement uses the already checked terminal snapshot: ordinary
generation writers are fenced, and terminal removal must not attempt to bump an
admission row after it has been deleted.

Coverage must map all 79/59 existing tables and new relations to direct Library,
parent-join Library, multiple affected Libraries, or excluded historical/account
scope. Shared account/device/default changes also invalidate authorization for
their affected Libraries; an actor revocation cannot leave a prepared review
valid. Changes to queue/cursor/upload/recovery facts counted by impact invalidate
impact even when they do not change authored metadata. Terminal immutable reads do not.

Service order stays authentication-principal locks → all affected Accounts sorted
by UUID → identities/devices/credentials/defaults in fixed table/key order →
Library admission/root locks sorted → Project policy → operation key → children.
Preflight discovery is untrusted: finding an additional account/Library after
locking causes full rollback/restart. Current
[runtime context](../../crates/photara-service/src/runtime.rs) locks account/identity
facts with FOR SHARE, while lifecycle last-owned serialization needs compatible
exclusive account admission across create/remove/membership/default writers.
Do not upgrade SHARE to UPDATE ad hoc in inconsistent paths; choose the strongest
required mode in the common admission prologue. No SQL transaction waits on UI,
package IO or remote authentication. Receipt recovery authenticates before lookup
and never acquires a live Library lock after its operation key.

**Coverage gate:** neither current API floor 3 nor existing per-table Library
locks prove this complete protocol. Inventory every entry point, background
worker and trigger path; old writers must refuse before deletion is enabled.

## Uniqueness, inventory privacy and session boundaries

The receipt operation PK arbitrates concurrent retries; changed bytes for the same
key conflict even if the old expected revision is stale. Receipt lookup requires
the authenticated initiating account/local principal, not a surviving membership.
Marker PK permanently prevents ID reuse within its authority. Publication guards
must reject root INSERT whenever a matching terminal marker exists, including
bootstrap/import/replay paths; no content restoration from historical receipts.

Inventory sequence allocation and receipt/invalidation publication share the same
account-locked transaction. Current rows point to immutable event keys; old pages
reconstruct as-of H using the event index, not current names. Cursor binds account,
environment, device, epoch, H, last Library key and privacy generation. New account
authorization or removal invalidates stale disclosure before a page is returned.
Available events can contain approved name/access summary; removed/revoked events
must have no name, project content, members, paths, actor details or impact counts.
These variant checks live in typed codec validation plus physical variant checks;
JSON shape alone is not a privacy proof. No user directly SELECTs event tables.

Incomplete snapshot absence cannot delete local data. Complete absence fences
access; only explicit authenticated removal installs marker/cleanup. Local applied
sequence advances with the corresponding dispositions/projection cleanup. A row
that remains unresolved leaves cleanup pending and preserves its retry position.

PS3/PS4 own preparing/freeze/flush/restoration and truthful package barriers. The
single local activation transaction inserts its receipt, CAS-updates
`device_activation` and removes the intent. No independent Swift preference write
or cloud activation pointer. Before receipt commit, failure retains previous
selection/capsule; after commit, recover target or explicit recovery. Library
removal fences matching attachments, clears live pointer and retires intent into
minimal evidence without reading/deleting capsule or package bytes. The exact
PS3 capsule reference and barrier type are a **coordination gate**, not a wire or
storage format invented by LL1.

## Compatibility and acceptance gates

Recommend reader/writer floors 6/6 and service API floor 4 **only against the
inspected 5/5 and 3 baseline**, plus negotiated `photara.library-lifecycle.v1`.
No migration number is reserved. If another slice advances the floor first,
rebase the proposal explicitly. Local table rebuilds and service guard/privilege
replacement require exclusive reviewed migration; unchanged historical checksums,
verified receipt bytes, fresh/upgrade parity and old-client refusal must be proved.
Feature remains disabled until all writers, scopes and activation dependencies
are ready. A stale database restore cannot reintroduce removed identities.

| Gate | Disposition |
| --- | --- |
| FK cycles / SQLite rebuild | **Physical design approval required:** four current-name backedges only; preserve ordinary delete guards and final no-orphan closure. No executable rebuild supplied or tested. |
| Retirement role / SECURITY DEFINER / forced RLS | **Security approval required:** narrow executor and permit policy must pass spoofed actor/GUC, role inheritance, table/function privilege, hidden-row and cross-account negative tests. |
| Immutable evidence | Existing LL0 exception stays narrow; an unknown/path-bearing codec or unsatisfied accounting/authentication obligation blocks removal and requires a specific decision. |
| Logical closure | Resolve typed Project joins and canonical embedded IDs before completion. No blanket deletion of unrelated shared records, invented ownership, package scan or secret lookup. |
| Trigger/generation/lock coverage | Not yet proved. Build reviewed full replacement definitions and test every old/new writer and deferred trigger, including rollback after each phase and counter overflow. |
| Inventory/privacy | Test duplicate operation/event, wrong scope/epoch/device, between-page revocation, incomplete snapshot, lost reply and stale backup. Source schema alone cannot prove disclosure correctness. |
| PS3/PS4 session publication | Owners must accept the single local activation authority and capsule boundary; choosing independent pointer stores would require another recovery contract. |
| LL2 / rollout | Separate schema acceptance, authorized disposable implementation, signed two-Library/two-project acceptance and rollout permission remain pending. No removal endpoint/UI becomes enabled from this review. |

LL0 behavior is unchanged: both confirmations, owner/controller checks, protected
defaults/last Library, complete database cleanup, permanent terminal evidence and
zero removal-origin filesystem/object-store actions. If physical implementation
cannot preserve any of these, stop at the corresponding gate; do not weaken it
to make an FK or test pass.
