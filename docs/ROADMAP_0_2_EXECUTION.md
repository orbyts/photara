# Photara 0.2 execution roadmap

This is the authoritative execution order for generation two. `ROADMAP.md` retains
the broader product and node-contract rationale; this document defines what task
starts next, its bounded slices, and where user approval stops progress.

## CXT4b-dev runtime provisioning and live readiness verified

The first development Mac now has three distinct SQL-created least-privilege
Neon `main` logins and a stable cursor key stored only in a non-sync macOS
Keychain bundle. The native operator launcher directly starts the checked
development service with a minimal environment. Real Auth0 JWKS initialization,
certificate-verified Neon pools, role admission and exact ledger readiness pass;
both health endpoints return HTTP 200, including readiness after restart.
Accounts/Libraries/defaults/receipts remain zero. The proof service is stopped.
See [exact evidence and limits](architecture/CXT4B_SERVICE_CHECKPOINT.md#cxt4b-dev-runtime-provisioning-and-live-readiness).

**CXT4d-dev has a signed first-enrollment path.** Apple Development signing now
authorizes separate desktop and operator application identifiers. Existing runtime
credentials were copied in memory into the operator's device-only Data Protection
Keychain item; desktop access is denied and two post-install/restart service runs
reached Auth0/Neon readiness. The legacy item remains recovery-only. The current
The production app now injects the Rust-backed driver: checked capabilities,
Rust-authored canonical challenge/bootstrap bytes, Auth0 PKCE, modern Keychain,
durable unknown-outcome dispatch, receipt retention and session reconciliation are
wired. The installed signed operator embeds the local service. CXT4d's real
sign-in/restart/lifecycle acceptance remains; CXT4e owns the full first-Account and
second-Mac audit. Fly resources and native sign-in have not been activated.

## Earlier CXT4b-dev configuration checkpoint

The identity/environment seam and secure loopback profile are implemented and
verified with offline configuration/native checks. One checked public descriptor
drives typed Rust, generated Swift and native bundle identity/callback metadata.
The development profile binds only `127.0.0.1:8080` and reuses the original
production verifier, HTTP routes and database role/ledger checks. Unconfigured
hosted profiles refuse. No credentials, provider changes, Fly resources or user
data were created. See [the exact checkpoint](architecture/CXT4B_SERVICE_CHECKPOINT.md#cxt4b-dev-configuration-checkpoint).

**Historical next gate, completed above:** separately provision host-only least-privilege runtime credentials
and a stable cursor key for Neon `main`; prove readiness with no user seed/sign-in.
CXT4d/e remain separate. Daily cloud switching between Macs is an acceptance
target, not functionality activated by the descriptor alone.

## Earlier CXT4b checkpoint and selected development policy

[CXT4b implementation and verification](architecture/CXT4B_SERVICE_CHECKPOINT.md)
records the bounded service code and ten passing disposable PostgreSQL suites.
Migration 0014/service floor 3 are active on Neon `main`; the dedicated Auth0
development client/API tuple is configured. No runtime database identities or
secrets, hosted origin or release-signing identity exists. The local service
profile has since been implemented as recorded above.
Remote host/HTTPS/secret-injection proof is deliberately deferred rather than a
blocker for development-cloud onboarding.
Do not describe CXT4b as remotely deployed or advance CXT4d/e automatically. The complete
approved CXT4c native-shell text is preserved byte-for-byte.

The 2026-09-13 development policy makes the next bounded slice **CXT4b-dev**:
add the typed environment/product-identity seam and run the same service contract
locally against Neon `main`. This allows real multi-Mac development cloud use.
Fly.io is separately exercised only for a brief remote-acceptance gate and is kept
continuously deployed only after the production identity and operating prerequisites
are ready. Read the
[authoritative policy](architecture/CXT4_DEVELOPMENT_CLOUD_AND_PRODUCT_IDENTITY.md).

## Historical CXT4a contract approved and complete

The [onboarding/security contract](architecture/CXT4A_ONBOARDING_SECURITY_CONTRACT.md)
and Chordrift read-only audit are approved. Google is the only initial Auth0
connection and the user-facing action is `Sign in with Google`; future providers
are additive and cannot merge identities by email. The device credential, token
policy, additive persistence/floors, bootstrap state machine and reconciliation
boundaries are accepted. Separately select CXT4b with exact Auth0, signing and
service-host inputs. CXT4c's approved native opening-shell boundary is unchanged.
No provider, credential, database or production code was changed in CXT4a.

## CXT3d Neon schema activation complete

[CXT3d](architecture/CXT3D_NEON_ACTIVATION.md) is complete in the existing Neon
`photara` project. Primary/default `production` is now `main`; populated
`development` is preserved unchanged as `legacy-v0.1.x`. The exact 13 checked
migrations and four non-login role boundaries are installed on empty `main`.
The live inventory and every ledger checksum match CXT3c; Accounts, Libraries,
and Projects remain empty. No runtime credentials or service deployment exists.

The next separately bounded sequence is
[CXT4 onboarding and opening Library shell](architecture/CXT4_ONBOARDING_AND_OPENING.md),
not a manual seed: Astra-led security/contract review, minimal cloud service,
native system-owned opening sidebar plus Photara's neutral content hierarchy,
Auth0 integration, and first-Account acceptance. Stop at each gate. All privileged
Neon credentials stay outside the desktop application.

## Historical CXT3c disposable service proof completion

[CXT3c](architecture/CXT3C_SERVICE_RUNTIME.md) is complete from clean synchronized
`7e6d794`, committed and pushed as `be1ed80`. Thirteen migrations execute in disposable
PostgreSQL 18.6 through Storexa. Five real PostgreSQL suites pass with 678 counted
authorization/privilege/sensitivity cells plus scoped command, media and fake sync
recovery proofs. The [exact inventory](architecture/CXT3C_IMPLEMENTATION_INVENTORY.md)
records the full Rust/native gates and the initial Graph randomized failure plus
successful replay/full rerun. All local runtime and Graph/UI sources are unchanged;
the real CXT3b database is byte-identical. All disposable clusters are stopped.

The 2026-09-12 authorization scoped this slice to the service boundary and injected
fakes. Native SQLite online-worker activation remains inactive; current service
content codecs cover Library storage and Project catalog projections. The fake is
not a full database backup or native online cutover. The subsequently authorized
CXT3d schema activation is recorded above. Real providers and package publication
still require their corresponding gates.

## Historical CXT3b local runtime completion

[CXT3b](architecture/CXT3B_LOCAL_RUNTIME.md) is complete from clean `415039e`.
Twelve executable local migrations install 70 domain tables with floor 2/2.
Typed local authority/access, storage/context/recovery and scoped projection
controllers are implemented; real AppModel startup safely establishes its local
My Library in `State/photara-local-v2.sqlite`. 258 offline Rust tests, all-target
check/Clippy/fmt, schema/naming/fixture guards, dedicated Graph Lab, bridge and
shared/production UI checks pass. See the runtime record for the initial Graph
randomized failure and successful replay/full rerun. All Graph sources are exact.
The slice remains uncommitted/unpushed. **Stop after CXT3b; separately authorize
CXT3c.** Online channel activation/service receipts/snapshot installation and
PostgreSQL remain CXT3c; package publication stays L3.

## Historical CXT3a and Library nomenclature rebaseline completion

CXT2, CXT1a/b and separately selected [CXT3a](architecture/CXT3A_PACKAGE_READER.md)
are complete. CXT3a passed 204 offline tests, repository checks/Clippy and canonical
package validation. Its original fixture/migration preservation evidence is recorded
before the next phase.

Suhail explicitly superseded D19 R1 physical-name preservation on 2026-09-12:
Generation Two is unshipped and has no valuable user data. The completed
phase is a clean **Library nomenclature rebaseline** across Rust, package, SQLite,
PostgreSQL proposals, service vocabulary, fixtures and documentation. Use `libraries`
and `library_id`; do not add rename migrations, aliases, shadow columns or dual writes.
The v0.1.3 database remains untouched reference material. An optional historical
importer is non-gating and is not part of this work.

The [rebaseline evidence](architecture/LIBRARY_NOMENCLATURE_REBASELINE.md) records
245 passing offline tests, clean SQLite integrity/FKs, regenerated hashes and
SQL inventories, naming/fixture guards and passing Swift/bridge/UI verification.
Gate A sub-items A2–A5 now correctly show their accepted status.

The forward sequence is: verified clean rebaseline → separately selected CXT3b
(executable local SQLite and real app initialization) → CXT3c (disposable PostgreSQL
and RLS) → separately authorized fresh Generation Two Neon deployment → minimum
usable Project/UI/node vertical slice. Current authorization stops after the
rebaseline, before CXT3b, PostgreSQL execution, Neon, deployment or commit/push.

## Operating protocol

Every milestone is divided into independently verifiable slices. A slice owns one
contract or observable behavior, its fixtures or migrations, integration,
documentation, and tests. Do not begin the next approval-gated milestone merely
because its design seems obvious.

For every UI slice:

1. freeze the data and state contract;
2. generate Light and Dark raster mockups for its meaningful states;
3. obtain explicit user approval or revise the mockups;
4. implement production sources and expose those exact sources through the owning lab;
5. assemble them into Photara;
6. run visual, interaction, persistence, accessibility, and regression checks;
7. record the result and next task in `ACTIVE_HANDOFF.md`.

Use Astra High for architecture, schema/security reviews, package/storage changes,
multi-graph or execution-history refactors, cloud synchronization, and stubborn
cross-boundary failures. Use the default Sol model for bounded module authoring,
fixtures, adapters, assembly, and ordinary tests. A heavy task must leave the next
bounded task consumable without rereading conversation history.

Prefer one focused commit for each completed, approved slice. Never mix a future
slice into that commit, and never push without the user's explicit request.

## Gate A — generation-two architecture

**Status:** complete; explicitly approved by the user on 2026-09-11.

- **A1 — repository and legacy audit:** compare current Core, Store, Library, bridge,
  native composition, and `v0.1.3`. **Complete.**
- **A2 — domain aggregates:** freeze Account, Library, Library records, Project,
  Location Kind, Location assignment, Project Asset, saved Graph, Graph Run,
  Node Attempt, evidence,
  and Project Catalog ownership. **Complete; accepted with Gate A/A7 on 2026-09-11.**
- **A3 — authority matrix:** identify package-authoritative, local-repository,
  cloud-projection, device-only, and disposable state. **Complete; accepted with Gate A/A7 on 2026-09-11.**
- **A4 — project package and locator contract:** define `.photara`, local/SMB writer
  rules, storage-root identities, per-device bindings, move, rebind, missing package,
  duplicate identity, copy, and fork behavior. **Complete; accepted with Gate A/A7 on 2026-09-11.**
  The cross-resource clarification is now canonical in
  [Storage locations and host bindings](architecture/STORAGE_LOCATIONS_AND_HOST_BINDINGS.md):
  the same logical/device split covers external assets and output targets, with
  explicit managed/external/cache classes and typed variable resolution.
- **A5 — extension audit:** prove the typed node, Inspector contribution, optional
  work-surface, asset representation, evaluation, and command contracts can evolve
  without node-specific Core or Shell branches. Confirm NodeSDK category/search
  metadata and exact package coordinates support a future store without special
  built-in semantics. **Complete; accepted with Gate A/A7 on 2026-09-11.**
- **A6 — legacy reference audit:** inventory reusable concepts and implementation
  patterns in the YAML registries, Neon schema, and `v0.1.3` without treating
  legacy records as generation-two schema inputs. **Complete at architecture
  level; any future importer is a separate, non-gating project.**
- **A7 — architecture approval:** present the complete architecture and unresolved
  decisions. **Complete; approved 2026-09-11.**

**Done when:** the architecture document, authority matrix, package invariants,
migration inventory, and boundary review agree and the user explicitly approves them.

## 0.2.0-alpha.1 — schema freeze

**Status:** S7 approved 2026-09-11 after user review of D1–D17. Separately authorized
L1 and L2 are complete. Local SQLite now runs in disposable tests; PostgreSQL and
service protocols remain unexecuted. No publication/locks, cloud/auth, live storage,
UI, staging/commits or release authority is implied.

**2026-09-12 amendments:** D19 supersedes D18's Library and ambient asset
assumptions. [Revised D18](architecture/TYPED_CONTEXT_AND_EXPRESSIONS.md) retains
closed uppercase HostPlaces and bounded expressions; `$library.*` and explicit
`$input.<port>` replace old examples. Exact contracts are accepted through R1–R8;
the revised gates below precede L3. No schema or fixture changes are implied.

- **S0 — Storexa boundary:** freeze how Photara consumes Storexa for PostgreSQL
  and SQLite while Photara retains domain repositories and schemas.
  Define CloudKit as a later record/change-sync capability, not SQL emulation or
  a distributed transaction spanning local, package, and cloud stores. **Complete:
  Storexa 0.2.0 released at commit `22c4270`, tagged and published to crates.io.**
- **S1 — logical model:** IDs, ownership, relationships, revisions, timestamps,
  tombstones, immutable evidence, lifecycle vocabularies, and deterministic
  Location Kind canonical-name/alias uniqueness (`beach`/`Beach`/`beaches`).
  **Complete; accepted as the schema baseline on 2026-09-11, with physical and
  conflict-policy details allocated to S2–S5.**
- **S2 — project package schemas:** manifest, authored Project state, named Graphs,
  project asset inventory, Location assignments, immutable runs/attempts,
  receipts, and compatibility versions. Do not put an actively written WAL
  database on SMB. **Proposal prepared:**
  [Project package schema](architecture/PROJECT_PACKAGE_SCHEMA.md) specifies
  immutable objects/commits, authored/history roots, single-writer publication,
  recovery, identity operations and current generation-two compatibility.
  Policy choices are accepted through R1–R8; executable fixture verification remains gated implementation work.
- **S3 — local SQLite schema:** Library records, Project Catalog, per-device
  locators, storage-root bindings, change log, sync outbox, and migration ledger.
  Specify the Storexa SQLite adapter and safe migration path from the current
  direct `rusqlite` repository without changing domain behavior. **Proposal prepared:**
  [Local SQLite schema](architecture/LOCAL_SQLITE_SCHEMA.md) defines ordered DDL,
  typed Library constraints and term claims, commit-qualified catalog projections,
  device bindings, recovery, mutations/sync queues, migration boundaries, and S6
  verification. Static grammar/link checks do not replace runtime migration/FK/
  trigger tests or S7 approval. S3 itself created no database or executed migrations;
  subsequently authorized L2 now verifies these migrations on fresh temporary files.
- **S4 — Neon/PostgreSQL schema:** Auth0 identity mapping, Libraries, membership,
  developer/subscription entitlement, synchronized Library records, Project Catalog
  projection, mutation deduplication, cursors, and server-side constraints. Use
  Storexa's PostgreSQL lifecycle/transaction/migration foundation behind the
  Photara service; Photara continues to own SQL and authorization. **Proposal prepared:**
  [Service PostgreSQL schema](architecture/SERVICE_POSTGRESQL_SCHEMA.md) provides
  clean ordered DDL, identity/membership and entitlement boundaries, typed Library,
  catalog reports, idempotent receipts, commit-ordered Library feeds, private
  authorization helpers/RLS and verification gates. SQL/function text was checked
  statically without connecting to Neon or opening a database. S5 reconciles both
  physical proposals for atomic Kind claim transfer with retired provenance;
  canonical promotion/offline collisions must preserve unique concept ownership.
- **S5 — synchronization contract:** local-first mutation envelope, conflict rules,
  deletion retention, media synchronization, retry/idempotency, and offline behavior.
  **Proposal prepared:** [Synchronization contract](architecture/SYNCHRONIZATION_CONTRACT.md)
  specifies versioned endpoints/envelopes, immutable receipts, ordered application,
  offline chains/rebase, one-batch feeds, bounded snapshots, media/catalog privacy,
  and a v1 Kind transfer/collision recommendation. S3/S4 proposal DDL was reconciled
  for that mechanism and missing durability records, then statically rechecked.
  S6 specifies runtime fixtures; execution follows explicit S7/disposable-scope
  authorization. No service/DB was contacted.
- **S6 — generation-two fixtures:** specify representative new generation-two
  projects and backend conformance fixtures. **Prepared:**
  [Generation-two fixtures](architecture/GENERATION_TWO_FIXTURES.md) includes
  a 33-file inert package archive, actual Rust canonical byte vectors, typed
  Library/device data, sealed offline trace, proposed normalizer/limits and 51
  conformance scenarios. Hash/reference/JSON/SQL grammar/link checks are static;
  runtime database, authorization and SMB tests remain post-S7 implementation
  work requiring a disposable scope. A future `v0.1.3` importer may use
  a separate loss-accounted mapping, but legacy import is not part of this schema
  gate and does not block implementation or release.
- **S7 — schema approval:** present ER diagrams, DDL, package examples, and migration
  behavior. **Approved 2026-09-11:** [S7 review index](architecture/SCHEMA_REVIEW.md)
  consolidates authority, counts, D1–D17 recommendations and exact implementation
  gates. D16 adds typed manual-first social profiles with optional provider/avatar
  adapters; D17 reserves future non-gating portable Library export/import. Revised
  schema/fixture details and conditional approval scope are explicit in
  [the additions](architecture/SOCIAL_PROFILES_AND_LIBRARY_EXPORT.md). D1–D17 are
  accepted; L1 and subsequently L2 were separately authorized, with no deployment/UI approval.

**Done when:** SQLite, PostgreSQL, and package schemas validate the same logical
invariants, generation-two migration tests are specified, and the user explicitly
approves them. Legacy Neon data conversion is not a completion condition.

## 0.2.0-alpha.2 — local project storage

**Status:** bounded L1 and L2 complete, authorized and verified 2026-09-11. See
[L1 limits](architecture/PROJECT_PACKAGE_CODEC.md) and
[L2 implementation](architecture/LOCAL_LIBRARY_IMPLEMENTATION.md).
Publication/writer locking, conversion, application cutover and live storage remain gated.

- **L1:** read-only multi-file codecs/validator implemented additively in
  `photara-store::package`; 25 focused tests and 29 retained tests pass. Current
  portable Project Document import/export stays unchanged. Directory conversion,
  writing and integration remain deferred to the separately scoped L3/L4 work;
  this completion is not a claim that all package migration behavior exists.
- **L2 — complete:** six Storexa/SQLx-backed local migrations (44 tables), separate
  generation-two typed Library CRUD/CAS/tombstones/claims and catalog/device
  foundations. Sixteen new and five retained Library tests pass. Merge/claim
  transfer remains a separately scoped **L2b** with no exposed API; sync/media/
  recovery workers and application cutover remain deferred. CXT2, CXT1a/b, CXT3a
  and the clean Library rebaseline are complete; R1 naming is superseded, CXT3b
  local runtime and CXT3c service proof are complete, and CXT3d activated the
  empty Generation Two Neon schema. Real onboarding is the next separate gate.
- **D19 / CXT0 — complete:** review approved conceptual direction
  and revised D18 together: Library/Project ownership and grants, explicit ports,
  frozen context, shared host components and taxonomy. Inert baseline cases remain
  unchanged until an exact reviewed addendum.
- **Contract freeze — accepted 2026-09-12:** logical/package/NodeSDK LibraryId,
  ProjectAccessGrant/invitation/policy, ledger versus input membership, typed
  value families, declarations and Work Surface/component contracts.
- **CXT2 — accepted, inert DDL complete:** package features/fields/closure,
  additive SQLite/PostgreSQL and sync/export/apply protocol, rename/compatibility
  plan, Project-filtered RLS/queries/feeds/snapshots/media/receipts. Exact inventory is recorded outside runtime paths. No SQL execution or runtime
  migration installation occurred; fixture changes await the corresponding slice.
- **CXT1a — complete:** additive pure Core/NodeSDK IDs, access, resource, AssetSet v2
  and manifest v2 contracts; one Rust-generated golden addendum. Old APIs/keys intact.
- **CXT1b — complete:** bounded expressions/templates, AST/types/dependencies,
  variables/snapshots/metadata/cache v2/proposal planning; 57 new integration and
  three compile-fail tests plus 99 retained tests. No DB/host I/O/UI/effects.
- **CXT3a — complete, verified:** package 1.1 reader/closure and
  explicit source mapping DTOs in disposable roots, with no publisher; 204 tests pass.
- **Library nomenclature rebaseline — complete:** clean canonical domain/database
  naming, regenerated schemas/fixtures and verified Rust/SQLite/Swift/UI evidence.
- **CXT3b — complete:** executable local D19 migrations, typed local repositories,
  permission/CAS/recovery fakes and real app startup with explicit My Library.
  Empty known 0006 upgrades; populated/unknown/newer baselines refuse adoption.
- **CXT3c — complete for the authorized service/fake slice:** executable PostgreSQL
  schema, unprivileged RLS, typed controllers, scoped receipts/media and fake online
  transport/bootstrap/recovery. Native SQLite worker activation and additional
  content codecs remain inactive, as documented in the service runtime record.
  Package-target application joins L3; no cross-store atomicity claim.
- **CXT3d — complete:** the exact schema and non-login role boundaries are active
  on Neon `main`; `legacy-v0.1.x` is preserved and user-data tables are empty.
  The next action is real Auth0 onboarding and default-Library reconciliation,
  not a manual seed.
- **L3 — paused until the above and CXT1–3 accepted:** create a package with exactly
  one owning Library and a catalog record through a recoverable staged operation;
  default to `~/Pictures/Photara/Projects` and allow another authorized root.
- **L4:** open, save, close, and recover a package without loading external archives.
- **L5:** implement move, locate/rebind, missing, duplicate-ID, copy/fork, and removal
  from catalog semantics.
- **L6:** verify local disk and `/Volumes/whisk/Pictures/Projects`, interrupted
  operations, stale writers, revision conflicts, and rebuildable indexes.

**Gate:** a mock project can be created, moved, lost, rebound, copied/forked, saved,
and reopened with no dependency on cloud or UI.

## 0.2.0-alpha.3 — multiple graphs and durable execution

- **G1:** replace the singular embedded Graph assumption with multiple named,
  stably identified Graph aggregates and a compatibility migration.
- **G2:** define consistent edit tokens, authored Graph revisions, and package save
  revisions while preserving revision-checked commands and undo/redo.
- **G3:** persist immutable Graph Runs and Node Attempts, including input/config/runtime
  fingerprints, timestamps, cancellation, failures, and diagnostics.
- **G4:** persist side-effect intent, idempotency keys, artifacts, evidence, and
  provider receipts; keep live progress and reproducible cache disposable.
- **G5:** define explicit Project lifecycle separately from derived workflow status.
- **G6:** regression-test existing Graph, Layout, Disk, AssetSet, Inspector DTOs, and
  standalone graph export without privileged node kinds.

**Gate:** multiple graphs survive package save/reopen and successful, failed, and
cancelled runs retain truthful history without changing authored graph state.

## 0.2.0-alpha.4 — Photara Cloud foundation

**Immediate execution order:** CXT4a contract → CXT4b host-independent service →
CXT4c native opening Library shell → CXT4b-dev identity/environment and loopback
service → CXT4d Auth0 integration → CXT4e first-Account/multi-Mac
acceptance. The [bounded record](architecture/CXT4_ONBOARDING_AND_OPENING.md)
defines ownership, states, UI/system boundaries, tests, and approval stops.

- **C1 — service/schema complete:** the Photara service boundary and exact Neon
  migrations are implemented and active; the desktop client never receives
  privileged Neon credentials. Runtime service deployment remains separate.
- **C2 — CXT4a contract complete:** Chordrift reference audit and Auth0 native
  Authorization Code + PKCE, Google sign-in, service security, bootstrap/recovery,
  and local/cloud reconciliation contract are approved. Google is the only initial
  provider; CXT4b implementation remains separately bounded.
- **C3:** create or claim a Library and provision the developer's account and
  entitlement separately from future billing.
- **C4:** synchronize Library records, media references, and Project Catalog entries
  through the local outbox/change-cursor protocol.
- **C5:** resolve logical storage roots independently on each Mac; cloud metadata must
  not treat `/Volumes/...` as a universal path or imply package availability.
- **C6:** test offline edits, retries, conflicts, tombstones, lost connectivity,
  multiple Macs, and an SMB-hosted package.

**Gate:** the developer can authenticate and use the same Library/Project Catalog on
two Macs while project packages remain correctly resolved and independently safe.

## 0.2.0-alpha.5 — opening and project browser

The opening Library shell foundation moves forward into CXT4c so the real Auth0
flow is tested in its intended native window. Later O-slices retain project-browser
depth, recent/missing/moved Project states, and post-onboarding refinement.

- **O1 — mockups:** automatic local My Library creation/open on first launch,
  named/multiple Libraries, explicit Library/Project window scope, optional cloud
  onboarding, restricted/project-only access, empty projects, recent
  projects, grid/list/card views, missing/moved project, offline/syncing state, and
  New Project destination. Obtain approval.
- **O2 — shared browser primitives:** extract typed grid/list/card presentation,
  search, sorting, selection, thumbnails, paging, and empty/loading/error states.
  Domain records remain typed; do not add an untyped Gallery domain to Core.
- **O3 — opening implementation:** local My Library first, optional cloud association,
  Library switching, New, Open, permission-filtered Browse Projects,
  project thumbnail, recent projects, and clear recovery actions.
- **O4 — real wiring:** New Project runs the staged package/catalog operation; Browse
  Projects uses Project Catalog; Locate verifies embedded identity.
- **O5 — Shell Lab:** show only Opening controls in the Opening context. Completed
  contexts remain selectable but their controls do not leak into another context.
- **O6 — freeze:** visual, keyboard, accessibility, persistence, missing-volume, and
  local/cloud integration tests; then treat Opening as stable.

## 0.2.0-alpha.6 — Graph authoring and Window Layout

- **W1 — mockups:** empty Graph, first selected node, Inspector closed, node work
  surface, multiple-Graph selection, running, successful, failed, and diagnostics.
- **W2 — composition:** Window Layout owns placement and context for the independently
  implemented Inspector and optional node work surface; it does not absorb their code.
- **W3 — hierarchy:** apply shared Application, Work Surface, Accessory, Grouped
  Control and system-overlay roles in Light and Dark. Map existing theme-role
  identifiers explicitly; no implicit source rename. Use no permanent separator; use a
  system scroll-edge/material response when content passes below the fixed header.
- **W4 — integration:** wire named Graphs and durable run selection/status to the
  accepted Graph canvas without redesigning nodes or connections.
- **W5 — verification:** interaction, resize, focus, undo/redo, failure, missing
  package, restart, accessibility, and Graph regression suites.

## 0.2.0-alpha.7 — assets and reusable browsing

- **B1 — mockups:** explicit connected AssetSet empty/populated in Layout, Gallery
  and metadata Work Surfaces; visible Library/Project/node/input scope, HDR/SDR, updating, unavailable, error, selection, list/grid/card/full view.
- **B2:** adapt shared browser primitives to asset-specific Gallery presentation;
  keep HDR, proxy, full-image, and `AssetSet` behavior specialized.
- **B3:** sources emit only a chosen collection/source AssetSet with MetadataSet,
  SourceDescriptor and ImportReport. A private package ledger retains identities/
  provenance; never expose it as an implicit input union or whole-project Gallery.
- **B4:** compose the same Asset Browser/Gallery in built-in Layout/Gallery and
  metadata Work Surfaces; enrichment emits AssetSet/MetadataPatch, while explicit
  XMP/file/application-update/delivery/publishing effects emit artifacts/receipts.
  Preserve fingerprints/provenance without implied cross-project authority.
- **B5:** test large projects, paging, virtualization, cache reuse, changed sources,
  disconnected providers, HDR/SDR displays, and package relocation.

## 0.2.0-alpha.8 — Library and Project Info

- Library management Browsers are application-owned. Node Work Surfaces embed
  host-owned People/Location pickers without navigation away; authorized inline
  creation and typed capture use separate Library/Project commands. Runtime access
  is limited to explicit ports and declared frozen IDs/revisions/projections.
- **P1 — People/Organizations:** mockup, approve, implement, integrate, and test
  multiple capabilities, personal labels, and project-specific roles.
- **P2 — Location Kinds:** mockup, approve, implement, integrate, and test the
  Library-unique taxonomy, descriptions, aliases, duplicate prevention, rename,
  and merge behavior. Decide the approachable UI label before implementation.
- **P3 — Locations:** mockup, approve, implement, integrate, and test hierarchy and
  the required reference from every concrete Location to one Location Kind.
- **P4 — Project Location assignments:** mockup, approve, implement, integrate, and
  test typed links among a concrete Location, kind snapshot, participants,
  schedule, notes, and historical snapshots.
- **P5 — Project Info:** mockup, approve, implement, integrate, and test assignments,
  lifecycle, status, and historical meaning after Library rename/delete/offline.
- **P6 — discovery:** cross-project search by person, organization/client, location,
  Location Kind, lifecycle, and workflow status through Project Catalog projections.

## 0.2.0-beta.1 — daily-use acceptance

- first installation creates/opens local My Library; sign-in/cloud association is optional;
- create a package on the default root and on Whisk;
- add People, Location Kind, concrete Location, and project Location context;
- add a source node and reconcile only the selected asset subset;
- author and run multiple graphs, inspect success/failure, and retain evidence;
- browse HDR/SDR assets and project history;
- save/reopen on another Mac, work offline, synchronize, move, lose, and rebind;
- pass recovery, performance, accessibility, security, migration, and compatibility
  gates.

`0.2.0` is releasable only after this workflow is truthful and recoverable. Visual
polish, provider breadth, Node Store/marketplace, generalized smart-query nodes,
Windows, CloudKit, and billing UI do not bypass these gates.

## Deferred platform and distribution tracks

- **Apple CloudKit:** list it in Library & Sync on macOS as planned/unavailable.
  Implement it only after Apple Developer membership and a CloudKit container are
  available. It adapts the same local-first domain and never changes portable IDs.
  Any Storexa support is a record/change-sync capability or platform-hosted
  adapter, not a relational database façade.
- **Node Store:** preserve NodeSDK metadata and package identity now; defer the
  storefront, free/paid commerce, publishers, signing, review, installation, and
  licensing. Layout and Gallery are D19's proposed built-in product nodes. LrC, Lr, and Ps
  are planned free first-party downloads; every store node has category and search
  metadata.
