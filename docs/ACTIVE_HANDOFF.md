# Active handoff

Updated 2026-09-12. **CXT3a and the separately authorized Library nomenclature
rebaseline are complete and verified.** Nothing is committed or pushed.

Read the [CXT3a reader/API record](architecture/CXT3A_PACKAGE_READER.md),
[rebaseline authority/evidence](architecture/LIBRARY_NOMENCLATURE_REBASELINE.md)
and [exact file/hash inventory](architecture/LIBRARY_REBASELINE_INVENTORY.md).
The [execution roadmap](ROADMAP_0_2_EXECUTION.md) is the current gate order.

Suhail superseded D19 R1 physical-name preservation: unshipped Generation Two uses
`Library`, `LibraryId`, `libraries`, `library_id` throughout Rust, package, SQLite,
PostgreSQL proposals and service vocabulary. There are no rename migrations,
aliases, shadow columns, dual writes or runtime naming adapters. The old v0.1.3
and live databases are untouched. An optional future historical importer is
non-gating; it has not been implemented.

**Verification:** 245 full offline Rust tests pass; all-target compilation, Clippy,
formatting and whitespace checks pass. Fresh SQLite baseline: 44 tables, 30 explicit
indexes, 95 triggers, 171 statements; integrity and FK checks pass. S3/S4 documentation
totals include examples (179/275); PostgreSQL baseline DDL is 263 statements and was
never executed. D19 static proposals retain 26/20 new tables, 139/431 statements,
46 checked signatures and 104 FKs. All 12 canonical fixture containers and 92 embedded
byte records pass Rust codec/hash verification. Naming checks, Swift/Rust bridge,
98 shared UI snapshots and 10 production snapshots pass; representative Light/Dark
screens were visually checked.

## Authority and exact next task

Library/account/catalog/access/device-binding records belong in local/cloud
databases. Authored Project graphs, nodes, connections, configuration, Work Surface
state, variables and asset/resource ledgers belong in `.photara`, with the designed
catalog/sync projections. Immutable runs/evidence follow package/cloud projection
rules. UI tokens are code/presets. Window geometry, panes, selection, zoom, scroll,
transient progress and unsaved edits are per-device preferences/session state.
The shell now calls that model `EditorSessionModel`; it is not Library data.

**Next separately authorize CXT3b:** use the clean Library baseline and accepted
D19 proposals to implement executable local SQLite repositories/facade and real
Generation Two app initialization. Prove fresh disposable migrations, floor refusal,
rollback/FKs, scoped permission/CAS behavior and local recovery with fake hosts.
Initialize local My Library explicitly; do not route through a legacy importer.
The current app's existing Library/UI regression is not a claim that D19 app
initialization or the new repositories are implemented.

Then CXT3c proves disposable PostgreSQL/RLS and scoped service behavior. Fresh
Generation Two Neon deployment requires its own authorization after those checks.
The minimum usable Project/UI/node vertical slice follows that deployment gate.
No CXT3b implementation, D19 migration execution, PostgreSQL/Neon execution,
service deployment, live database change, staging, commit or push occurred here.

## Historical CXT1b handoff — superseded next-step labels


Updated 2026-09-12. **CXT1b pure Rust context contracts are complete.** Suhail
separately selected this slice after CXT1a. Read the [implementation/API/grammar/
verification record](architecture/CXT1B_CONTEXT_CONTRACTS.md), then the
[accepted freeze](architecture/D19_CONTRACT_FREEZE.md) and
[bounded gates](architecture/D19_STATIC_SCHEMA_DELTA.md#fixture-delta-implementation-slices-and-gates).

The additive `photara_core::context` API implements explicit literal/expression/
template admission, bounded parser and ID-bound typed AST/interpreter, variable
CAS/precedence/cycles, immutable consent-checked captures, bounded metadata queries,
Run overrides, private device/frozen contexts, cache v2 and proposal/receipt planning.
Literal-only fields never become expression records; fenced containers remain
unsupported. Evaluated results retain privacy labels; no existing graph evaluator,
NodeSDK validator, package codec, database/host adapter or effect executor is changed.

**Verification passed offline: 159 tests**, comprising 57 new context integration
tests, three new serialization compile-fail tests and 99 retained CXT1a/Core/SDK/node
tests. New coverage includes a 441-case deterministic arithmetic matrix. The separate
[d19-context.json](fixtures/generation-two/d19-context.json) was generated and checked
by the unchanged Rust canonical encoder; previous fixture bytes remain exact.
Affected formatting, all-target library compilation, Clippy with `-D warnings`,
whitespace, Markdown links and pre-slice hash checks pass. Database suites were not
executed; retained node tests use disposable filesystem fixtures.

CXT1a source/tests/implementation record, six applied L2 migrations, all 14 CXT2
proposal files, ten pre-existing fixture files, Cargo files and unrelated dirty work
are preserved. The hash audit confirms 317 pre-existing files are byte-identical;
232 local Markdown links/anchors pass. Only the additive context module declaration
changes existing Rust.
No database/SQL, resolver, package reader/writer, Swift/UI, user Project/SMB, service,
staging/commit/push or release work occurred.

**Next eligible slice: separately select CXT3a.** Its package 1.1 reader/closure and
explicit compatibility mapping DTOs belong in disposable roots with no publisher.
CXT3b/c retain separate local/fake-host and service/RLS/sync proof gates. L3 remains
paused. Stop at CXT1b; do not automatically continue into those slices.

## CXT1b exact file inventory

New source files:

- `crates/photara-core/src/context/mod.rs`
- `crates/photara-core/src/context/value.rs`
- `crates/photara-core/src/context/expression.rs`
- `crates/photara-core/src/context/parser.rs`
- `crates/photara-core/src/context/interpreter.rs`
- `crates/photara-core/src/context/variable.rs`
- `crates/photara-core/src/context/snapshot.rs`
- `crates/photara-core/src/context/metadata.rs`
- `crates/photara-core/src/context/cache.rs`
- `crates/photara-core/src/context/proposal.rs`
- `crates/photara-core/tests/context.rs`

New documentation/data: `docs/architecture/CXT1B_CONTEXT_CONTRACTS.md` and
`docs/fixtures/generation-two/d19-context.json`.
Existing Rust edit: `crates/photara-core/src/lib.rs`, additive module declaration only.
Updated documentation: `README.md`, `ROADMAP.md`, `docs/ACTIVE_HANDOFF.md`,
`docs/ROADMAP_0_2_EXECUTION.md`, and architecture `README.md`, `CORE.md`, `ASSETS.md`,
`NODE_PACKAGES.md`, `STORAGE_LOCATIONS_AND_HOST_BINDINGS.md`,
`TYPED_CONTEXT_AND_EXPRESSIONS.md`, `GENERATION_TWO_FIXTURES.md`,
`D19_CONTRACT_FREEZE.md`, `D19_STATIC_SCHEMA_DELTA.md`, `SCHEMA_REVIEW.md`.
This slice adds 13 files and updates 15 pre-existing files. Earlier dirty-tree
changes in git status are not part of this inventory.

## Prior CXT1a completion — historical scope

The following records earlier completed slices. Their then-next-step statements
are historical; the CXT1b status and CXT3a gate above govern current work.

Updated 2026-09-12. **CXT1a pure Rust contracts are complete.** Suhail separately
selected this bounded implementation after accepting R1–R8 and CXT2. Read the
[implementation/API/verification record](architecture/CXT1A_CONTRACTS.md), then the
[accepted freeze](architecture/D19_CONTRACT_FREEZE.md) and
[bounded gates](architecture/D19_STATIC_SCHEMA_DELTA.md#fixture-delta-implementation-slices-and-gates).

Additive `photara_core::contracts` and `photara_node_sdk::v2` implement portable
IDs/adapters, Project access, resource rights/coordinates and nonserializable live
handles, immutable AssetSet v2 snapshots/pages/digests, complete manifest v2
validation and minimal response/opaque context coordinates. The one new
[d19-contracts.json](fixtures/generation-two/d19-contracts.json) is generated and
verified with the existing Rust canonical encoder in the reserved synthetic namespace.
No parser, expression evaluation, capture engine, cache v2 implementation,
package 1.1 codec or host/database adapter is included.

**Verification passed offline: 99 tests** (58 new integration, two compile-fail,
39 retained Core/SDK/node tests), plus explicit golden generation and the final
SDK rerun. Affected-crate formatting, whole-library all-target compilation and
Clippy with `-D warnings` pass. Database suites were not executed. The pre-slice
SHA-256 inventory confirms 302 pre-existing files remain byte-identical and preserves existing dirty work, v1 source/API/cache behavior,
Cargo files, all six L2 migrations, all 14 CXT2 proposal files and all nine
pre-existing fixture files. No database/service, user Project/SMB, UI,
staging/commit/push or release action occurred.

**Next eligible slice: separately select CXT1b.** Its bounded parser/AST/types,
dependencies/snapshots/cache/proposals remain unstarted. CXT3a/b/c retain their
separate package/local/service proof gates. L3 remains paused. Stop at CXT1a;
there is no automatic migration, package publication or live-service continuation.

## CXT1a exact file inventory

New Rust files:

- `crates/photara-core/src/contracts/{mod,ids,schema,access,resource,asset_set,dto}.rs`
- `crates/photara-core/tests/contracts.rs`
- `crates/photara-node-sdk/src/v2/{mod,types,validation}.rs`
- `crates/photara-node-sdk/tests/contracts_v2.rs`

Existing Rust changes: only additive module declarations in
`crates/photara-core/src/lib.rs` and `crates/photara-node-sdk/src/lib.rs`.
New documents/data: `docs/architecture/CXT1A_CONTRACTS.md` and
`docs/fixtures/generation-two/d19-contracts.json`.
Updated documentation: `README.md`, `ROADMAP.md`, `docs/ACTIVE_HANDOFF.md`,
`docs/ROADMAP_0_2_EXECUTION.md`, and architecture `README.md`, `CORE.md`, `ASSETS.md`,
`NODE_PACKAGES.md`, `STORAGE_LOCATIONS_AND_HOST_BINDINGS.md`,
`TYPED_CONTEXT_AND_EXPRESSIONS.md`, `GENERATION_TWO_FIXTURES.md`,
`D19_CONTRACT_FREEZE.md`, `D19_STATIC_SCHEMA_DELTA.md`, `SCHEMA_REVIEW.md`.
This slice adds 14 files and updates 16 pre-existing files; earlier dirty-tree
changes shown by git status are not part of this inventory.

## Prior CXT2 acceptance — historical scope

The following records the completed CXT2 slice before CXT1a was selected. Its
then-next-step and no-source-change statements are historical evidence only.

Updated 2026-09-12. **Suhail accepted R1–R8 as proposed; CXT2 acceptance is complete.**
Read the [accepted contract freeze](architecture/D19_CONTRACT_FREEZE.md),
[static schema delta](architecture/D19_STATIC_SCHEMA_DELTA.md), and
[inert DDL/inventory](architecture/proposals/d19-cxt2/README.md).
The primary review found no blocking inconsistency; approval was explicitly
recorded for all eight decisions on 2026-09-12. Only CXT2 acceptance was selected.

CXT2 translated the approved signatures into eleven `.proposal.sql` files outside
runtime migration directories: SQLite 0007–0012 and PostgreSQL 0008–0012. The
service's unexecuted 0007 privileges reservation is preserved. Added relations
remain **26 local (44 → 70)** and **20 service (35 → 55)**, plus four service
upload-session columns. These are proposed counts, not installed schemas.

The proposal includes concrete constraints, FK indexes, retention/CAS/lifecycle
guards, service access helpers and replacement RLS/grants. Its
[responsibility ledger](architecture/proposals/d19-cxt2/RESPONSIBILITIES.md) names
exact Rust/repository/controller checks that SQL cannot prove, including local
commit-level last-manager checks, access-generation aggregation, canonical typed
bytes, consent, scoped sync installs and package evidence.

**Next eligible slice: separately select CXT1a pure Rust**—IDs, access masks,
portable resource contracts, AssetSet v2 and complete manifest v2 validation.
CXT1b and CXT3a/b/c follow their own bounded gates. CXT1/CXT3/L3 have not begun;
Neon or any live service is not the next step. L3 remains paused.

No runtime/UI/source, applied/local migration, baseline service SQL, fixture
byte/hash, manifest or package specimen changed. No database, service, user
Project or SMB storage was opened. No staging, commit or push occurred. Existing
dirty-tree changes remain intact. The prior documentation-only review evidence
below is retained as history, followed by this acceptance slice's verification.

**S7 D1–D17 approved; bounded L1 and L2 complete.** After L1,
the user explicitly authorized L2 local SQLite implementation and fresh disposable
database tests. No existing user database, service, SMB storage, UI or live Project
was opened/changed. Publication/locks, cloud/auth, staging, commits and release
remain outside this authorization.

## Resume here

Canonical repository:
`/Users/suhail/Library/CloudStorage/Dropbox/matrix/crates/photara`.
Verify the path/branch/status before editing: the task environment may open another
worktree. Read in order:

1. This handoff and [execution roadmap](ROADMAP_0_2_EXECUTION.md), then
   [D19](architecture/LIBRARY_AND_NODE_WORK_SURFACES.md) and
   [revised D18](architecture/TYPED_CONTEXT_AND_EXPRESSIONS.md), followed by
   [storage locations and host bindings](architecture/STORAGE_LOCATIONS_AND_HOST_BINDINGS.md),
   then [exact R1–R8 contracts](architecture/D19_CONTRACT_FREEZE.md) and
   [static delta](architecture/D19_STATIC_SCHEMA_DELTA.md).
2. [Approved product architecture](architecture/GENERATION_TWO_PRODUCT_ARCHITECTURE.md)
   and [S7 decision record](architecture/SCHEMA_REVIEW.md).
3. [L1 implementation boundary](architecture/PROJECT_PACKAGE_CODEC.md).
   Then [L2 implementation boundary](architecture/LOCAL_LIBRARY_IMPLEMENTATION.md).
4. [Logical model](architecture/LOGICAL_DATA_MODEL.md),
   [package format](architecture/PROJECT_PACKAGE_SCHEMA.md),
   [local SQLite](architecture/LOCAL_SQLITE_SCHEMA.md),
   [service PostgreSQL](architecture/SERVICE_POSTGRESQL_SCHEMA.md),
   [synchronization](architecture/SYNCHRONIZATION_CONTRACT.md),
   [fixtures](architecture/GENERATION_TWO_FIXTURES.md), and
   [social profiles/export](architecture/SOCIAL_PROFILES_AND_LIBRARY_EXPORT.md).
5. [Storexa integration](architecture/STOREXA_INTEGRATION.md),
   [current project documents](architecture/PROJECT_DOCUMENTS.md),
   [persistence](architecture/PERSISTENCE.md), and
   [Library architecture](LIBRARY_ARCHITECTURE.md).
6. [Core](architecture/CORE.md), [assets](architecture/ASSETS.md),
   [Node packages](architecture/NODE_PACKAGES.md), and
   [native clients](architecture/NATIVE_CLIENTS.md). For presentation work read
   [Shared UI](../platform/macos/SHARED_UI.md) and
   [design language](../platform/macos/DESIGN_LANGUAGE.md).

[ROADMAP](../ROADMAP.md), [CODEX_HANDOFF](CODEX_HANDOFF.md) and
[FRAME_LIBRARY_HANDOFF](FRAME_LIBRARY_HANDOFF.md) retain historical/operator
context; their older next-work sections do not override this scope.

## Locked decisions

- Native macOS presentation over portable Rust; Graph authoring composition hosts modular
  Inspector and optional Node Work Surfaces. UI implementation still requires
  raster mockup approval; schema approval is not UI approval.
- General versioned Node packages own behavior. No privileged Layout/provider
  cases in Core/bridge. Layout and Gallery are D19's proposed built-ins; LrC/Lr/Ps are
  planned free first-party downloads. Node Store commerce remains future work.
- Stable identity is independent of paths. Package authority owns authored
  Project metadata, typed Library snapshots/assignments, a private graph/run asset
  identity/provenance/artifact ledger, multiple named Graphs and immutable run/attempt/effect/evidence records.
  Catalogs are projections, never a second source of package authority.
- Library Storage Locations are portable logical identities; per-device Host
  Bindings resolve them to macOS, Windows, Linux or provider resources. Packages
  and synchronized Library data contain no absolute paths, bookmarks or secrets.
  External sources, managed Project resources, external artifacts and disposable
  cache are distinct storage classes. Variables resolve typed handles and never
  hide AssetSet membership or workflow dataflow.
- Library is the durable catalog/collaboration boundary, local-first and typed: People with multiple roles and
  relationships, client Organizations, unique LocationKinds and concrete
  Locations with a required kind. Account/Auth0 identity is never a Person.
  Scene is not a separate target domain record; a friendly UI label remains a
  presentation choice. Current Scene code is transitional, not silently removed.
- People/Locations/Project catalog management belongs to the app/Library. Node
  Work Surfaces embed host-owned pickers/Browsers under declared permissions;
  inline creation uses Library commands. Authoring visibility does not grant runtime
  access. Project-only collaborators receive bounded assigned snapshots. Package/SMB
  access remains separate device authorization.
- Graph inputs are explicit typed ports plus declared frozen context. Read/source,
  enrichment and effect/output are distinct; MetadataPatch never mutates originals
  by itself. Stable hierarchical categories/tags are discovery-only metadata.
- Kind claims use the approved Unicode 16.0.0 policy and explicit Beach/beaches
  seed groups. Narrow atomic merge/claim transfer preserves Library/key
  uniqueness. The full normalizer/merge runtime is not implemented by L1.
- Auth0/API/Neon is the cloud trust boundary; desktop has no privileged Neon
  credentials. CloudKit is a deferred explicit record-sync adapter, not SQL.
- Storexa 0.2.0 is published from sibling commit `22c4270`, with explicit SQLite
  and PostgreSQL SQLx types. Photara retains entities, SQL/migrations, package
  publication, authorization and sync policy. L1 uses no database adapter.
- D16 reserves typed manual-first social profiles. Stable bound subjects are
  Library-unique including tombstones; no ordinary reassignment or identity
  proof. Mutable/reusable handles never silently merge owners. Provider adapters,
  Instagram lookup and consent/expiry-aware fetched avatars remain optional.
- D17 reserves future logical checksummed/optionally encrypted Library export:
  separate Project backups, safe root hints/rebind, dry-run/isolated restore,
  no credentials, device IDs, absolute paths, bookmarks or sync/recovery state.
  Export/import is non-gating and unimplemented.
- Clean generation two only. Existing Neon and `v0.1.3` are optional reference/
  salvage. Legacy import is never a schema, implementation or release gate.

## Completed bounded L1

Additive `photara-store::package` provides strict raw JSON validation before
Value construction, canonical-json.v1 checking, versioned typed control/authored/
Graph envelopes, safe descriptor-relative no-follow reads, managed-byte hashing,
HEAD/parent/bootstrap/inventory closure and focused typed authored/history checks.
Original canonical bytes and optional unknown data are retained. No writer API
exists. Current one-JSON ProjectDocument import/export and Core are unchanged.

The S6 33-file inert archive is materialized only in fresh temporary roots.
All 25 focused tests and 29 retained Core/NodeSDK/store tests passed; selected doc
tests passed. Store all-target Clippy with warnings denied and whole-library
all-target compilation passed offline. No database, service, user Project or SMB
storage was opened. Full Unicode graph-name uniqueness, embedded manifest support,
application integration, conversion, publication and recovery remain outside this
bounded reader. See [precise limitations](architecture/PROJECT_PACKAGE_CODEC.md).

## Completed L2 and next exact slice

`photara-library::gen2::LocalLibraryStore` now implements the separate family and
six ordered migrations (44 tables), typed Library CRUD/CAS/tombstones, exact
Unicode-16 Kind claims, local changes and catalog/device/verified-observation
foundations. Storexa 0.2/SQLx 0.9 is adopted privately; existing v1 APIs are intact.
The necessary rusqlite compatibility pin is 0.39.0/libsqlite3-sys 0.37.0. All five
retained Library tests and 16 new L2 tests pass; selected regression total is 75,
with doc tests, library Clippy `-D warnings` and whole-library compile passing.
See [L2 scope and limits](architecture/LOCAL_LIBRARY_IMPLEMENTATION.md).

**Next: separately select CXT3a.** R1–R8, CXT2 inert DDL and CXT1a/b pure contracts
are complete. CXT3a package readers, CXT3b disposable local migrations/repositories
and CXT3c disposable service/RLS/fake transport each require their own scope. L3 waits for accepted
contracts and required conformance. See the companion's exact slices and gates.
S2–S6 remain baseline evidence: S3 is 44 tables/S4 35; all six L2 migration
checksums and pre-existing inert fixture bytes are unchanged. The proposed totals are neither
installed schema counts nor runtime test evidence.
Resolve remaining L1
writer-readiness gaps before publishing any package. L2b Kind merge/claim transfer
remains unexposed: its exact affected-root/CAS/rebind/retirement/promotion proof must
pass before merge or cloud reconciliation is offered. No automatic L3, SMB/user
storage, cloud, UI, staging/commit or release authorization is implied.

## Design evidence still awaiting runtime proof

S3's 44 SQLite tables/179 statements now install through L2's six migrations;
selected local FK/trigger/concurrency tests pass. S4 has 35 PostgreSQL tables/275
statements, RLS and service functions and remains unexecuted. Full merge/sync,
service privilege/concurrency and publication tests remain future work.
S5 defines sealed commands/receipts, offline chains, ordered feeds, bounded reset,
conflict/rebase, media staging and privacy; no API/Auth0/object-store was contacted.
S6 has 51 scenario specifications and 12 crash points; passing L1 does not mark
their database/service/SMB/import families passed. S2's publication and recovery
protocol is approved design, not an fsync/rename/SMB guarantee.

## Working tree warning

The canonical tree already contains substantial uncommitted docs, shell/library
presentation, Inspector, lab, build configuration and UI verification work.
These belong to the user/ongoing work and were preserved. The planning docs and
L1 sources are also uncommitted. Inspect live status; do not reset, restore, clean,
stage or commit unrelated changes. Storexa's separate release is already complete.

## Task and model allocation

Use Astra with high reasoning for architecture/heavy Rust/persistence and Sol for
bounded UI/module slices after contracts and scope approval. This does not itself
authorize spawning tasks/agents. Give each slice a concrete boundary and gate.

## Completion checklist

- [x] Architecture and S1 logical model accepted; Storexa 0.2.0 published.
- [x] S2–S6 designs, inert fixtures and static SQL/JSON/hash/link checks prepared.
- [x] S7 D1–D17 approval recorded on 2026-09-11.
- [x] Bounded L1 read-only implementation/test scope selected and passed.
- [x] Actual changes, limits and next gate recorded.
- [x] L2 database scope separately authorized; implementation and temporary tests passed.
- [x] D18 concept inclusion and source syntax requested; documentation/inert addendum prepared.
- [x] D19 conceptual direction approved and documentation amendment prepared.
- [x] D19 consistency reviewed; exact logical/package/NodeSDK freeze candidate prepared.
- [x] Static physical/package/sync/DTO/fixture delta specified as inert documentation.
- [x] Suhail accepted R1–R8 as proposed, 2026-09-12; CXT2 acceptance only selected.
- [x] CXT2 inert SQL, exact inventory and static checks complete; no execution.
- [x] CXT1a separately selected and completed; pure contracts and golden fixture verified.
- [x] CXT1b separately selected and complete; pure context/golden/regression verified.
- [ ] CXT3a separately selected, followed by CXT3b/c proof before L3.
- [ ] Remaining migration/service/publication/recovery scenarios pass before
  their corresponding release claims.

## CXT2 acceptance verification — historical, 2026-09-12

This acceptance slice adds **14 files** (11 inert SQL proposals and three proposal
README/inventory/responsibility documents) and updates **27 Markdown files**.
The exact paths are listed below; the earlier review's delta is separate history.

| Proposal | Statements | New tables | Explicit indexes | Triggers | Functions | New policies | Old policies removed |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| SQLite 0007–0012 | 139 | 26 | 38 | 74 | 0 | 0 | 0 |
| PostgreSQL 0008–0012 | 431 | 20 | 33 | 42 | 27 | 173 | 25 |

The service total includes 44 ALTER TABLE statements (40 enable/force-RLS, three
forward-reference constraints and the upload-column extension), privilege changes
and the inert API-floor update. Local floor changes likewise remain inert text.
No generated proposal is referenced by a runtime migration runner.

Checks passed: **46 exact relation-column/nullability signatures**, **104 scoped
FK target/key/index checks**, lexical SQL statement/delimiter/quote checks,
identifier uniqueness/length checks, explicit replacement of all 25 old policies,
enable/force RLS on all 20 service additions, **319 local Markdown links/anchors**,
changed-file whitespace and `git diff --check`. These checks read SQL as text;
no database, interpreter, compiler or service was started. No installed pglast,
sqlglot or sqlparse was available in the checked Python environments; grammar,
SQL type/name resolution and runtime behavior remain unverified CXT3 gates.

All **236 pre-existing non-Markdown files** retain their pre-slice SHA-256.
All six applied/local migration files, all nine fixture-directory files and the
original complete S2/S3/S4/S5 baseline bodies remain byte-identical. The inventory
records protected-file hashes. Existing SQLx checksums, package specimen, fixture
hashes, runtime/Rust/Swift/UI, manifests and unrelated dirty files were preserved.

SQL cannot by itself prove local commit-level manager/aggregate invariants,
expected-revision intent, canonical/typed/privacy agreement, complete controller
lock ordering, bootstrap/cursor correctness, identity/token checks or package
publication evidence. The responsibility ledger states the exact future checks.
No CXT1a implementation, CXT3 database work or L3 publication was begun. No staging,
commit or push occurred. **Next: separately select CXT1a pure Rust, not Neon.**

Exact changed files for this acceptance slice:

```text
README.md
ROADMAP.md
docs/ACTIVE_HANDOFF.md
docs/ROADMAP_0_2_EXECUTION.md
docs/architecture/CORE.md
docs/architecture/D19_CONTRACT_FREEZE.md
docs/architecture/D19_STATIC_SCHEMA_DELTA.md
docs/architecture/GENERATION_TWO_FIXTURES.md
docs/architecture/GENERATION_TWO_PRODUCT_ARCHITECTURE.md
docs/architecture/LIBRARY_AND_NODE_WORK_SURFACES.md
docs/architecture/LOCAL_LIBRARY_IMPLEMENTATION.md
docs/architecture/LOCAL_SQLITE_SCHEMA.md
docs/architecture/LOGICAL_DATA_MODEL.md
docs/architecture/NATIVE_CLIENTS.md
docs/architecture/NODE_PACKAGES.md
docs/architecture/PERSISTENCE.md
docs/architecture/PROJECT_DOCUMENTS.md
docs/architecture/PROJECT_PACKAGE_CODEC.md
docs/architecture/PROJECT_PACKAGE_SCHEMA.md
docs/architecture/README.md
docs/architecture/SCHEMA_REVIEW.md
docs/architecture/SERVICE_POSTGRESQL_SCHEMA.md
docs/architecture/SOCIAL_PROFILES_AND_LIBRARY_EXPORT.md
docs/architecture/STORAGE_LOCATIONS_AND_HOST_BINDINGS.md
docs/architecture/STOREXA_INTEGRATION.md
docs/architecture/SYNCHRONIZATION_CONTRACT.md
docs/architecture/TYPED_CONTEXT_AND_EXPRESSIONS.md
docs/architecture/proposals/d19-cxt2/INVENTORY.md
docs/architecture/proposals/d19-cxt2/README.md
docs/architecture/proposals/d19-cxt2/RESPONSIBILITIES.md
docs/architecture/proposals/d19-cxt2/postgresql/0008_library_project_access.proposal.sql
docs/architecture/proposals/d19-cxt2/postgresql/0009_storage_locations.proposal.sql
docs/architecture/proposals/d19-cxt2/postgresql/0010_library_context.proposal.sql
docs/architecture/proposals/d19-cxt2/postgresql/0011_scoped_sync.proposal.sql
docs/architecture/proposals/d19-cxt2/postgresql/0012_d19_access_guards.proposal.sql
docs/architecture/proposals/d19-cxt2/sqlite/0007_library_project_access.proposal.sql
docs/architecture/proposals/d19-cxt2/sqlite/0008_storage_locations_bindings.proposal.sql
docs/architecture/proposals/d19-cxt2/sqlite/0009_library_context.proposal.sql
docs/architecture/proposals/d19-cxt2/sqlite/0010_context_apply_recovery.proposal.sql
docs/architecture/proposals/d19-cxt2/sqlite/0011_scoped_sync.proposal.sql
docs/architecture/proposals/d19-cxt2/sqlite/0012_d19_guards_and_floor.proposal.sql
```

## Prior documentation review verification — 2026-09-12

Passed for this slice: **292 local Markdown links/anchors**, all changed-file
whitespace including untracked Markdown, Project action-mask arithmetic, exact
relation inventories (14 common + 12 local-only; 14 common + 6 service-only),
26 proposed package schema rows, eight unchanged fixture JSON parses, and
`git diff --check`. The six migration files and all nine fixture-directory files
(including README) are byte-identical to the task-start baseline. Hash comparison
also proves all **236 existing non-Markdown files** unchanged. Reconstructing the
pre-notice S2/S3/S4/S5 documents matches their original full-file hashes, so their
SQL, JSON and protocol baseline text is preserved exactly.

No new SQL grammar/runtime/RLS/codec/interpreter test was run: the proposal uses
inert relation signatures, not executable DDL. No Rust build or database test was
needed or authorized for this documentation-only slice. The commands below are
historical L1/L2 checks, not new runtime evidence.

Exact task delta: **two new review documents and 26 updated Markdown files**.
New files: `docs/architecture/D19_CONTRACT_FREEZE.md` and
`docs/architecture/D19_STATIC_SCHEMA_DELTA.md`. Updated files:

```text
README.md
ROADMAP.md
docs/ACTIVE_HANDOFF.md
docs/ROADMAP_0_2_EXECUTION.md
docs/architecture/ASSETS.md
docs/architecture/CORE.md
docs/architecture/GENERATION_TWO_FIXTURES.md
docs/architecture/GENERATION_TWO_PRODUCT_ARCHITECTURE.md
docs/architecture/LIBRARY_AND_NODE_WORK_SURFACES.md
docs/architecture/LOCAL_LIBRARY_IMPLEMENTATION.md
docs/architecture/LOCAL_SQLITE_SCHEMA.md
docs/architecture/LOGICAL_DATA_MODEL.md
docs/architecture/NATIVE_CLIENTS.md
docs/architecture/NODE_PACKAGES.md
docs/architecture/PERSISTENCE.md
docs/architecture/PROJECT_DOCUMENTS.md
docs/architecture/PROJECT_PACKAGE_CODEC.md
docs/architecture/PROJECT_PACKAGE_SCHEMA.md
docs/architecture/README.md
docs/architecture/SCHEMA_REVIEW.md
docs/architecture/SERVICE_POSTGRESQL_SCHEMA.md
docs/architecture/SOCIAL_PROFILES_AND_LIBRARY_EXPORT.md
docs/architecture/STORAGE_LOCATIONS_AND_HOST_BINDINGS.md
docs/architecture/STOREXA_INTEGRATION.md
docs/architecture/SYNCHRONIZATION_CONTRACT.md
docs/architecture/TYPED_CONTEXT_AND_EXPRESSIONS.md
```

Canonical pages now point to exact decisions instead of leaving the freeze
unspecified; older historical baseline text remains labeled and preserved.
Unrelated existing dirty changes are not included in this task's change claims.
Git's full diff includes prior work and omits untracked file content; do not use
its aggregate diff-stat as this review's change inventory.

## Verification commands

Run from canonical repository. L1 checks actually run offline:

```sh
git status --short
git diff --check
cargo fmt -p photara-store -- --check
cargo fmt -p photara-library -- --check
cargo test --offline -p photara-library -p photara-store -p photara-core -p photara-node-sdk
cargo clippy --offline --workspace --all-targets -- -D warnings
cargo check --offline --workspace --all-targets
```

Untracked files need explicit inspection; ordinary git diff omits them. Whole-
library tests were not needed for this bounded slice; selected Library tests
now run with L2's temporary-database authority. Compilation is not service or
package-publication verification.
