# L2 generation-two local Library implementation

## Current exact review packet — 2026-09-12

The [D19 static proposal](D19_STATIC_SCHEMA_DELTA.md#proposed-migration-inventory) is complete for review. L2 remains exactly the six-migration/44-table implementation described below. Additive 0007–0012 and reader/writer floor 2 are proposals only; old migration bytes/checksums are preserved. R1–R8 were accepted as proposed 2026-09-12 and [inert DDL](proposals/d19-cxt2/README.md) is complete; separate CXT3b scope precedes runtime migration/repository work.

**D19 supersession note (2026-09-12):** [Libraries and node Work Surfaces](LIBRARY_AND_NODE_WORK_SURFACES.md)
is the current conceptual target. Library replaces durable Workspace; each Project
has one Library and explicit Project access. Graphs use connected AssetSets and
declared frozen context; a private package ledger is not an ambient Gallery/asset
union. Library management is app-owned; node Work Surfaces embed authorized host
pickers/components. First install opens local My Library. The pre-D19 implementation,
physical identifiers, examples and fixture contracts below remain baseline evidence,
not approval to reinterpret stored bytes. Exact contract/static schema review and
revised CXT1/CXT3 precede L3; no migration, source or fixture bytes change here.

Status: bounded L2 complete, 2026-09-11, explicitly authorized after S7/L1.
Only fresh temporary SQLite databases and disposable inert package copies were
used. No existing user database, service, SMB storage or application UI was opened
or changed. This is an additive API, not application cutover or legacy import.

## Separate store and dependency boundary

[`photara_library::gen2`](../../crates/photara-library/src/gen2/mod.rs) exposes
`LocalLibraryStore::open(path, OpenMode::{CreateNew, OpenExisting}, DeviceId, Timestamp)`.
The caller must explicitly select a protected local directory. There is no default
path, environment lookup, Cloud connection or automatic v1 conversion. Create
requires an absent `.sqlite` file; Unix creates it with mode 0600. Paths containing
symlink components, non-normal components or a `.photara` ancestor are rejected.
Host code must still verify local storage, directory ownership and OS grants;
lexical checks are not a cross-process hostile-filesystem security boundary.

Existing files must have the generation-two SQLite family header before the
adapter connects. The transactional metadata check then verifies family/epoch,
reader/writer floor, codec, device identity and exact normalization policy.
Future/cloud-associated stores are rejected rather than edited without their
required synchronization semantics. A failed new initialization can leave an
empty/uninitialized owned file; it is not silently adopted on a later open.

Storexa is pinned to published `=0.2.0`, SQLx to `=0.9.0`. SQL and domain records
remain Photara-owned; the concrete Storexa pool is private. No domain/ORM/sync
abstraction was added to Storexa, Core, nodes or the bridge.

The old synchronous `SqliteLibraryRepository` source/API and schema-v1 behavior
remain intact. Cargo cannot link two `sqlite3` providers: SQLx 0.9 requires
`libsqlite3-sys <0.38`, whereas rusqlite 0.40.2 requires 0.38.x. The narrowly approved
compatibility pin is **rusqlite `=0.39.0` / libsqlite3-sys 0.37.0** (bundled SQLite
3.51.3 source). All five retained adapter tests pass unchanged. This is a dependency
alignment, not a database migration. The lockfile retains the exact resolved graph.

## Migrations and transactions

The six ordered [migration files](../../crates/photara-library/migrations/generation_two/0001_local_identity.sql)
are copied from the approved [S3 DDL](LOCAL_SQLITE_SCHEMA.md):

1. `0001_local_identity.sql`: family metadata, local device, Workspaces and media/cache foundations.
2. `0002_typed_library.sql`: typed parties, relationships, Kind claims, Locations and social profiles.
3. `0003_catalog_device.sql`: catalog/locators, roots/bindings and immutable projections.
4. `0004_mutations_sync.sql`: exact local changes and future sync bookkeeping.
5. `0005_durable_recovery.sql`: future package-operation intent/recovery tables.
6. `0006_invariant_guards.sql`: identity/revision/lifecycle/reference/immutability guards.

All 44 domain/support tables install; SQLx additionally owns its migration ledger
and SQLite its autoincrement support table. `application_id=0x50485432`,
`user_version=2`, family `photara.local.g2`, epoch/floors 1 and canonical-json.v1
identify this family. Metadata does not fabricate an applied-migration ledger.
SQLx verifies exact migration checksums and refuses unknown/dirty history.

Startup holds a SQLite `BEGIN IMMEDIATE` reservation while the public SQLx Migrator
runs on that connection and metadata/device/policy bootstrap completes. Migration
transactions become nested savepoints. Failure rolls back the family/ledger/schema
unit. A new store checkpoints its family header before returning so an independent
pool can safely identify an active WAL database. This is database initialization,
not a package-writer lease or a cross-store transaction.

One retained Storexa connection per handle's shared pool uses local WAL, FULL,
foreign keys, recursive triggers, a two-second busy timeout and three-second
acquisition timeout. Replacement connections inherit the same options. Separate
pools/processes still serialize via SQLite's writer reservation. Pool lifecycle,
health/acquire, deferred read snapshots, commit/rollback and close use Storexa;
short CAS writes use SQLx's explicit `BEGIN IMMEDIATE` through the private pool.

Authority commands validate typed input, read the expected revision while holding
the writer reservation, use explicit INSERT or UPDATE, replace aggregate children,
append one immutable mutation/change pair, then commit. Root UPSERT is deliberately
not used: SQLite's BEFORE INSERT revision guard fires before conflict resolution.
The approved initial-revision and update guards remain unchanged. Errors/drop roll
back child changes, deferred FKs and change history together. No network or package
read occurs while holding a write transaction.

## Typed API and invariants

Distinct UUID wrappers reject nil IDs and round-trip BLOB16. `Revision` is positive
signed-64-bit with checked increment; `Timestamp` is UTC Unix milliseconds bounded
to years 0001–9999. Package revisions remain decimal TEXT/u64, not SQLite REAL.
Each record has typed `Metadata<Id>`; `advance`/`tombstone` prepare an edit, and the
corresponding `put_*` still requires its exact prior revision (`None` means create).
Identity, Workspace and created time are immutable; no ordinary resurrection.

Implemented read, stable UUID keyset list (1..500), create, CAS update and tombstone:

- Workspace, Person with multiple namespaced capabilities/labels, Organization
  with client labels, and typed Person–Organization relationships.
- Relationships enforce same-Workspace active endpoints and nonoverlapping
  half-open periods for the same pair/type; different roles may overlap.
- LocationKind canonical/alias claims and required-kind, acyclic Locations.
  Live child/relationship/profile/Location references block retirement rather
  than implicitly cascading. Workspace retirement requires live dependents retired.
- Manual-first SocialProfile with one immutable Person/Organization owner,
  mutable handle/display/public URL, provider/account kind, exact optional
  provider subject/namespace, provenance, verification and refresh facts.
  Subject adoption is once-only; bound subjects stay reserved through tombstone.
  Same-provider normalized handle matches return review hints, never merges or
  permanent handle identity. Profile URLs reject credentials/query/fragment data;
  no provider is contacted and verification fields are recorded provenance, not
  proof that a Person is an authenticated Account.

Text/collection/extension limits follow the S6 bounds. `normalize_term` uses pinned
[unicode-normalization 0.1.24](https://docs.rs/crate/unicode-normalization/0.1.24/source/)
and [caseless 0.2.2](https://docs.rs/caseless/0.2.2/caseless/), with compile-time
checks of their Unicode 16.0.0 data versions. The pipeline is NFC, full default
case fold, NFC and an explicit UCD White_Space set. Beach and beaches keep distinct
text keys but either input claims **both** keys; Studio/studios behaves likewise.
Custom aliases recompute the same closure. SQL arbitrates concurrent ownership;
rename retains old claims and tombstone never releases them. All eight S6 vectors pass.

`changes` returns exact immutable local post-states and verifies their canonical
bytes/SHA-256, rather than joining old IDs to current rows. Its local-command.v1
envelope and typed local-record encoding are **not** S5 sealed wire requests or a
cloud feed. There is no outbox scheduler, remote baseline or sync cursor movement.
Future claiming/sync must explicitly adapt those local records and revisions.

## Catalog and device foundations

Typed root/catalog/locator APIs preserve stable IDs and revision checks. Rooted
locators contain only safe relative hints; catalog removal means hidden visibility,
not package deletion. Root bindings hold a local path or opaque secure-store UUID,
never bookmark bytes. Bindings and catalog observation selection do not enter
mutation payloads. Debug/errors do not include paths, values or SQL details.

`observe_package` is read-only: it requires a rooted locator and path binding,
checks the supplied authorized path against that binding, runs L1 verification
outside the database transaction, then rechecks locator/binding revisions. It
records a CommitId/checksum-qualified immutable summary and party/location/Graph
projections, plus a timestamped device identity observation. It does not select
anything automatically. `select_observation` is a separate catalog-CAS command;
another locator with the same ProjectId never wins by time/revision. Package bytes
are never authored from SQL. “Available” records historical verification time,
not a promise of continuing access. A failed later scan leaves the older evidence
intact and returns a failure; it does not certify current availability.

## Exact remaining scope

- **L2b Kind merge/claim transfer:** no public merge/raw-SQL API is exposed. Before
  enabling it, implement a bounded affected-root plan with all expected revisions,
  explicit live Location rebinds, immutable source retirement terms, claim transfer,
  optional target canonical promotion and one atomic multi-root mutation. Prove
  omitted/concurrent dependents, chains and complete rollback using KIND-03–07.
  Dormant SQL guards are installed; that is not passing merge-runtime conformance.
- Social avatar/media promotion, media bytes, Location label storage, unrooted or
  secure-grant package resolution, device availability/rebind workflows, latest-run
  summaries, projection pruning and backup/export/import remain unexposed. Supplying
  Location labels is rejected, not discarded. No provider adapter or OAuth exists.
- Sync/account authorization, cloud claims, recovery workers, application cutover,
  package create/save/locks/moves/forks and current-document conversion are not L2.
  Existing user stores and one-JSON package APIs remain unchanged.

As of 2026-09-12, next is [D19 consistency and contract freeze](LIBRARY_AND_NODE_WORK_SURFACES.md),
then static CXT2 review before revised CXT1/CXT3; L3 is paused.
No Library variable/AST table, Project grant, context API or apply-receipt
protocol was added to L2. CXT2 must review additive DDL before implementation;
these six migration checksums remain unchanged. Afterwards, explicitly authorize
L3 package creation and crash/intent/catalog tests on named temporary roots. Resolve L1 writer
readiness gaps before publication. No SMB/user-storage, UI, staging/commit, cloud
or deployment authority is inferred. L2b can be separately scheduled and must
precede exposing merge or cloud reconciliation, but does not block local creation.

## Verification recorded

`cargo test --offline -p photara-library -p photara-store -p photara-core -p photara-node-sdk`
passed **75 tests**: 16 new L2, five retained Library, 25 L1 package, two retained
store, 24 Core and three NodeSDK tests. Their doc-test targets passed.
`cargo clippy --offline --workspace --all-targets -- -D warnings`, whole-workspace
all-target check and formatting checks passed. Temporary tests cover fresh/reopen,
all migrations/PRAGMAs/FK/integrity, checksum/newer/family/device refusal, failed
migration rollback, replacement connections, CRUD/CAS/history/retirement, Unicode/
alias collisions and competing pools, relationship/hierarchy/reference guards,
social identity reservation, catalog/device privacy, explicit selection and corrupt
or duplicate package copies. These are local runtime results, not Neon/RLS, media,
merge, sync, package-publication, SMB or release certification.
