# Generation-two product architecture

## Current exact review packet — 2026-09-12

The [accepted D19 freeze](D19_CONTRACT_FREEZE.md) now resolves exact access, storage, dataflow, context and NodeSDK contracts, with the [physical/portable delta](D19_STATIC_SCHEMA_DELTA.md). Suhail accepted R1–R8 as proposed 2026-09-12; CXT2 inert DDL is complete. This packet supersedes older next-freeze instructions below without changing implementation evidence.

Status: architecture approved by the user on 2026-09-11. This is the current
product architecture, not a claim that the model is implemented. Concrete
schema policies and formats remain subject to the separate S7 database-schema
approval. No schema or schema-driven UI implementation proceeds before S7.
See [Active handoff](../ACTIVE_HANDOFF.md) for the immediate gate and read order.

## Current D19 amendment

[D19 — Libraries, explicit dataflow and node Work Surfaces](LIBRARY_AND_NODE_WORK_SURFACES.md)
is the approved conceptual direction as of 2026-09-12. It supersedes conflicting
S1–S7 and D18 ownership, asset-access, UI terminology and built-in assumptions.
Exact logical/package/NodeSDK freeze and static physical/sync delta review remain
next gates; existing schema/fixture bytes and L2 migrations are unchanged.

## Established boundaries

- Portable Rust owns domain identity, validation, commands, revisions, graph
  evaluation, and persistence contracts. Native clients own presentation and
  interaction through immutable, versioned application contracts.
- Nodes are ordinary independently namespaced/versioned packages with typed
  ports, registered runtimes, scoped capabilities, and namespaced private state.
  Layout is the first serious package, not a privileged Core node kind.
- Asset identity is independent of location. Authored state, durable evidence,
  runtime progress, disposable caches, credentials, and device bindings have
  different owners and lifetimes.
- Library use is local-first. Accounts are not People. Cloud access crosses
  Auth0 → Photara API → Neon; desktop clients and nodes receive no privileged
  database credentials or ambient SQL access.

These continue the [Core](CORE.md), [node package](NODE_PACKAGES.md),
[asset](ASSETS.md), and [native client](NATIVE_CLIENTS.md) contracts.

## Approved target product model

An **Account** is an authenticated identity. A **Library** is the durable,
user-named catalog/collaboration boundary, analogous to a Notion workspace.
Users can own or join multiple Libraries. First installation automatically
creates and opens local **My Library** without sign-in. LibraryId survives rename
and explicit local-to-cloud association; joining an existing Library is distinct.
Local, Photara Cloud and deferred CloudKit use the same domain.

Library owns records, policies/variables, memberships and the Project catalog.
Every Project belongs to exactly one Library. Membership alone does not grant
every Project: explicit ProjectAccessGrant/invitations and restricted versus
library-visible Project policies govern access. Project-only collaborators see
assigned bounded snapshots, not the whole catalog. Package/SMB grants remain
separate device binding authorization. Exact grant and migration contracts are
pending the D19 freeze.

The **Library** contains reusable knowledge:

| Domain | Meaning and relationship |
| --- | --- |
| People | One identity per person, with roles/capabilities such as photographer, model, or stylist and contextual relationship labels such as client. These descriptive capabilities do not grant application access. |
| SocialProfiles | Typed manual-first profiles owned by People/Organizations; provider secrets remain host/account capabilities. |
| Organizations | Studios, companies, and other organizations; people may relate to them. A commissioning client is a relationship to a Person or Organization, not a duplicate Client record. |
| Location Kinds | A Library-unique semantic taxonomy such as Beach, Home Studio, Apartment, or Commercial Studio. A kind has a description and aliases. “Scene” may remain approachable presentation language, but the domain concept is `LocationKind`. |
| Locations | Reusable concrete places, such as Shane's Apartment or Ocean Beach, with optional hierarchy and stable identity independent of display names. Every Location must reference one Location Kind. |

Project assignments retain stable Library IDs and explicit historical snapshots
so offline use, renaming, merging, and deletion do not erase project meaning.
Editing a project assignment does not silently edit its Library source.
Updating snapshots is an explicit semantic operation; conflict and merge rules
remain part of the schema review.

A **Project** is the portable aggregate for a particular body of work. It owns
project metadata and Library assignments, project-specific uses of concrete
Locations, a private asset/provenance/artifact ledger, multiple named graphs, and durable execution
evidence. Project lifecycle (for example active or archived) is separate from workflow state
(a run waiting, executing, failing, or completing). Exact lifecycle and workflow
state vocabularies are not yet frozen.

A **Location Kind** represents one concept, not one spelling. Its canonical key
and aliases are unique within a Library under a schema-approved normalization
policy. Therefore `beach`, `Beach`, and `beaches` must resolve to the same
concept and cannot be created as separate kinds. Case folding, Unicode and
whitespace normalization, singular/plural handling, alias ownership, rename,
merge, and multilingual behavior require deterministic rules in the schema
review; a raw case-insensitive database index alone is insufficient.

A project's use of a **Location** may carry its own stable assignment identity,
date/schedule, participants, notes, and historical snapshots without creating a
second Location or Location Kind. This supports questions such as “show every
project shot in a Home Studio” and returns projects using concrete places such
as Shane's Apartment. The exact assignment fields and cardinalities need schema
approval.

Photara may borrow proven structural ideas from **OpenUSD**: namespaced stable
identities, typed schemas that impart known properties, explicit relationships,
hierarchical namespaces, metadata separated from identity, and composition by
reference rather than duplication. Photara does not treat a photographic
Location Kind as an OpenUSD scenegraph, does not require USD files for Library
records, and does not inherit OpenUSD behavior without an explicit Photara
contract. The schema slice must document each adopted analogy and where it ends.
Use the official [OpenUSD terms and concepts](https://openusd.org/dev/glossary.html)
as the reference rather than relying on the name “scene.”

## Project package and storage authority

The target is a movable **`.photara` directory package** on local storage or
SMB/NAS. Its stable project UUID identifies the project; its filename and mount
path do not. Exact package filenames, versioning, resource layout, and save
protocol remain proposals to settle before schema implementation.

| Storage boundary | Authority and lifetime |
| --- | --- |
| Project package | Portable project authority: authored metadata, assignments/occurrences, named graphs, resource descriptors and fingerprints, plus separately versioned durable run/evidence records. Evidence is not graph-authored state. |
| Device-local SQLite | Local-first Library working copy and durable local changes awaiting optional sync; also rebuildable project discovery/search projections and explicitly scoped operational records. A project index is not another project authority. Local-only Library records are not disposable cache. |
| Photara API / Neon | Authorized Library synchronization and explicitly designed cloud services. They do not silently replace the package as project authority or become necessary for offline project opening. |
| Device bindings and secure storage | Current project location, mounts, bookmarks, resource/provider locators, credential handles, and authorization grants. Portable IDs resolve through scoped host adapters. |
| Device cache | Rebuildable proxies, thumbnails, derived indexes, previews, and intermediate values. Eviction cannot erase authored state, Library changes, or durable evidence. |

### Rust persistence implementation — Storexa

Photara intends to consume the sibling Rust library
[`storexa`](../../../storexa/README.md) as its database connection, transaction,
migration-execution, error-classification, health, and tracing foundation.
Storexa 0.2.0 supports PostgreSQL and SQLite through explicit SQLx types; its public contract
explicitly leaves application SQL, schemas, migrations, domain records, and
repositories to the consuming application. It was released from sibling commit
`22c4270`, tagged `v0.2.0`, and published to crates.io on 2026-09-11.

The ownership boundary is:

- `photara-core` owns semantic rules and commands;
- `photara-library` and other Photara domain crates own typed repositories,
  queries, schemas, migrations, and mapping between records and domain values;
- `photara-store` owns project/package persistence contracts and portable file
  publication where those contracts are not database operations;
- Storexa owns reusable backend mechanics and capability-specific adapters;
- native hosts own platform authorization and secure credential handles.

Storexa now provides a SQLite capability alongside PostgreSQL without forcing
both engines behind a least-common-denominator SQL connection type.
Shared lifecycle, diagnostics, transaction outcomes, and migration reports may
be common; connection/query types and engine-specific features remain explicit.
Photara can migrate its direct `rusqlite` adapter only after parity,
migration, concurrency, cancellation, and recovery tests pass.

CloudKit is not a relational SQL engine and must not masquerade as a PostgreSQL
or SQLite transaction. A later Storexa CloudKit integration should expose a
record/change synchronization capability—or coordinate a platform-hosted
adapter—whose batches, conflict tokens, zones, retries, partial failures, and
idempotency are explicit. Photara's local outbox applies remote changes through
its own domain repositories. There is no distributed transaction spanning a
`.photara` package, local SQLite, Neon/PostgreSQL, and CloudKit; staged operations,
outbox records, idempotency keys, and recovery/compensation provide consistency.

Storexa remains domain-agnostic: it must not acquire Photara entity types,
Library authorization, subscription policy, package semantics, or NodeSDK
permissions. Photara should consume a pinned Storexa release rather than rely
indefinitely on an unversioned sibling path.

**No live SQLite database or WAL resides inside an SMB project package.** Local
SQLite stays on local storage. Package persistence must tolerate interrupted
writes, reconnects, and stale revisions through an explicit publication and
recovery protocol. Existing local-filesystem locking/rename behavior is not
proof of safe SMB multiwriter operation. Single-writer ownership versus
concurrent editing, lock expiry, recovery, and atomic publication are unresolved.

Graphs receive explicit typed **AssetSet** values on connected ports. There is
no ambient project-wide Gallery, `$project.assets` or implicit asset union.
The package may retain a private identity/provenance/representation/artifact ledger
for Graphs and Runs; its contents are not automatic node inputs. Each asset has
stable identity and zero or more representations with
their own identities, capabilities, and fingerprints. Physical bytes may be
managed inside the package or externally referenced through portable handles
and device bindings; the embedding policy needs approval. Moving a binding
does not change identity; replacing bytes changes the fingerprint.

Optional cross-project content recognition can index fingerprints to suggest
matches or reuse safe derived work. Matching content never automatically merges
project identities, grants access, or turns a global index into package
authority. Its scope, privacy, and relationship to explicit asset reuse remain
open; implementing a global media catalog is not a prerequisite.

## Graphs, Work Surfaces, and execution

A project can own multiple named graphs, each with stable identity, revisions,
nodes, connections, and exact package pins. Names are presentation metadata,
not graph identity. Whether graphs may call/reference each other, share authored
values, or participate in one execution requires a separate explicit contract.
Do not infer those semantics from supporting multiple graphs.

The native **Graph authoring composition** hosts a Graph Canvas, the
independently implemented **Inspector**, and optional node **Work Surfaces**.
Independent modules keep their own sources, presentation contracts, and labs;
composition does not fold their implementation into the graph renderer. Node
contributions use the ordinary registered presentation boundary. Selection,
panel placement, split geometry, and Layout Presets remain native preferences
and do not dirty graph semantics. See [shared UI](../../platform/macos/SHARED_UI.md)
and the existing [Inspector module](../../platform/macos/photara-inspector/Sources/InspectorView.swift).

An explicit execution creates a durable **Graph Run** associated with the
project, graph identity, and evaluated revision. A **Node Attempt** records an
individual attempt within that run; retrying must preserve previous evidence.
Durable results can include outcome, structured diagnostics, artifact references,
and external-effect receipts sufficient to explain what happened. Evidence must
identify the relevant inputs/fingerprints and exact implementations where needed
for interpretation; final retention, redaction, and replay/idempotency contracts
need approval.

Transient progress, cancellation signals, in-flight scheduler state, and
recomputable cache are separate. A terminal cancellation outcome may be durable
even though its live signal is not. Evidence survives ordinary cache cleanup,
reopening, and package movement; it does not enter the authored graph digest.
Durable history is proposed work, not a description of current runtime storage.

## Project discovery, movement, and duplication

Local project discovery maintains a device projection of project UUID → known
location and availability. Moving or renaming a package rebinds its location;
portable project-relative resources resolve against the opened package root.
Unavailable mounts produce an offline/unavailable state, not a new project or
deleted asset. External resource rebinding is explicit and fingerprint-aware.

A filesystem copy may carry the same UUID. Discovering both copies must surface
the ambiguity without silently merging divergent content, choosing the newest
path, or rewriting identity. An explicit duplicate-as-new-project operation
needs a defined identity-remapping policy; a moved project preserves identity.
How intentional replicas, restored backups, and concurrent copies are recognized
and resolved remains an approval decision.

## Shared collection presentation

D19 standardizes **Work Surface** for node-provided UI, **Canvas** for a spatial
editor, **Window Layout / Layout Preset** for native arrangement, **Panel** for a
dockable region and **Browser** for list/grid/card primitives. Retire ambiguous UI
Workspace labels without renaming source identifiers in this documentation slice.
Each window binds one Library; Project windows also bind one Project and display
both scopes. Library management Browsers are app-owned. Node Work Surfaces embed
host-owned People/Location pickers and Browser/Gallery components with declared
permissions; inline creation uses authorized Library commands.

Layout composes shared Asset Browser/Gallery Panel, Layout Canvas and Inspector.
Gallery is an ordinary built-in inspection node displaying exactly its connected
AssetSet through the same component. A brandable Metadata-enrichment node composes
Asset Browser, metadata Inspector and inline pickers, emitting enriched AssetSet
and MetadataPatch without mutating originals. Sources/read, enrichment and effects
are separate: Lightroom source and catalog-update are distinct definitions;
XMP/file/application/cloud/social/web effects emit typed artifacts and receipts.
Reference/query nodes are optional; authored typed refs can be captured directly.
Runtime gets only explicit ports plus declared frozen IDs/revisions/projections,
regardless of what the user can browse while authoring.


Propose typed **CollectionBrowser** presentation primitives for search/filter
chrome, selection, cards/rows/grids, paging, loading/empty/error states, and
action slots where Library, Projects, or other collections share behavior.
Feature adapters supply typed item presentation and semantic actions. People,
Organizations, Location Kinds, Locations, Projects, and Assets remain distinct Rust
domains; a reusable Swift browser is not justification for one generic mutable
database entity or a universal metadata bag. Feature editors and specialized
Gallery behavior retain their owners. The existing
[LibraryBrowser](../../platform/macos/photara-library-ui/Sources/LibraryBrowser.swift)
is an implementation reference, not an approved universal abstraction.

## Trust and migration boundaries

Auth0 authenticates the native Account through Authorization Code with PKCE.
The Photara API validates tokens and enforces Library membership, authorization,
subscription policy, revision checks, and sync semantics before accessing Neon.
Database row-level security is defense in depth. Secrets and refresh tokens stay
in secure device/service storage; node runtimes get scoped capabilities. Local
work does not depend on an authenticated cloud session. Exact sync conflict,
tombstone, media-transfer, and offline enrollment policies remain unimplemented.

The same local-first synchronization contract may later have a **CloudKit**
adapter. It is an Apple-client option alongside On This Mac and Photara Cloud,
not a separate domain model and not a replacement for local SQLite. It remains
deferred until an Apple Developer account, CloudKit container, entitlement,
schema, test environment, and conflict/migration plan exist. Non-Apple clients
must not depend on CloudKit-only identities or semantics. Apple documents
CloudKit as a container-based transfer/synchronization service rather than a
replacement for an application's own data objects; see
[CloudKit](https://developer.apple.com/icloud/cloudkit/) and
[the framework overview](https://developer.apple.com/documentation/cloudkit).

## NodeSDK and future Node Store

The existing NodeSDK/package boundary is part of the product architecture, not
only an internal implementation detail. Every node definition carries a stable
namespaced identity, version, typed ports and values, capabilities, permissions,
implementation fingerprint, presentation metadata, and a required catalog
category plus search terms. Category identity is stable metadata; localized
display labels and future merchandising do not alter graph semantics.

For the initial product, **Layout and Gallery are D19's proposed built-in nodes**.
Lightroom Classic (`LrC`), Lightroom Desktop/Cloud (`Lr`), and Photoshop (`Ps`)
are planned first-party nodes distributed free through the future Node Store.
Other first- or third-party nodes may be free or paid. Existing Disk, AssetSet,
or fixture packages prove contracts but do not by themselves establish future
bundling policy.

The Node Store, payments, publisher accounts, signing/notarization, permissions,
review, search/ranking, download/update/rollback, revocation, licensing, and
recovery are deferred. Core and saved graphs nevertheless must preserve exact
package/definition coordinates now so a store can be added without changing
graph meaning or creating privileged built-in node kinds.

Legacy **v0.1.3** is a migration and behavior source, not the generation-two
architecture. Its tagged `src/project.rs`, `src/asset.rs`, `src/master.rs`,
`src/transfer.rs`, `src/delivery.rs`, and `migrations/0001`–`0020` preserve lessons
about project/asset identity, separate locations, paired renditions, transfers,
decisions, and delivery evidence. Consult `METADATA.md`, `LAYOUTS.md`, provider
adapters, and fixtures on that tag for original behavior. Inspect with
`git ls-tree -r --name-only v0.1.3` and `git show v0.1.3:<path>`; do not restore
the old tree or copy its database model wholesale. The
[older handoff](../CODEX_HANDOFF.md#historical-archive) also names v0.1.0; v0.1.3
includes later migration history and must be considered explicitly.

Generation two does **not** migrate or extend the existing Neon database as part
of its architecture or schema gate. It creates clean package, SQLite and Neon
schemas for the new model. The old code and DDL may be salvaged where they fit
the approved boundaries. Importing selected old projects is a later, explicit
adapter and is not a prerequisite for schema approval, implementation or release.

Current generation-two migration sources are equally concrete:
[ProjectDocument](../../crates/photara-core/src/project.rs) currently embeds one
graph; [store adapters](../../crates/photara-store/src/lib.rs) persist whole
documents; [Library schema v1](../../crates/photara-library/src/lib.rs) includes
Client records and project assignments; the
[Library bridge](../../crates/photara-bridge/src/production/library_project.rs)
publishes assignments through a document extension. Existing
[project documents](PROJECT_DOCUMENTS.md), [persistence](PERSISTENCE.md), and
[Library architecture](../LIBRARY_ARCHITECTURE.md) describe that earlier slice.
Their single-graph shape, Clients, and non-durable runtime history must not be
mistaken for approval of this target model or silently migrated.

## Historical architecture review questions and current gate

The list below records the original architecture questions; S7 accepted D1–D17
on 2026-09-11. D19 now supplies the conceptual Library, access, dataflow and Work
Surface decisions. Remaining exact contracts follow
[D19's next gates](LIBRARY_AND_NODE_WORK_SURFACES.md#compatibility-physical-review-and-exact-next-gates),
not a restart of S7 or automatic implementation permission.

1. Ratify the remaining target entity/ownership model around the confirmed
   Location Kind/Location distinction: Library/Account association,
   Organizations and relationship labels, project-specific Location assignments,
   multiple graphs, and project lifecycle separate from workflow outcomes.
2. Ratify the `.photara` package authority split, asset embedding/reference
   policy, durable evidence portability, and SMB writer/recovery model.
3. Set move/rebind/duplicate/replica behavior and identity-remapping rules.
4. Set cross-project recognition scope, graph relationships, evidence retention,
   and cloud conflict/ownership rules at the depth needed for the first slice.
5. Approve Graph authoring composition composition, the scope of shared CollectionBrowser
   primitives, and whether the UI label is “Location Kinds,” “Scenes,” or a
   transitional “Scenes · Location Kinds” before UI restructuring.
6. With architecture approved, review and separately approve concrete portable
   formats, typed Rust records, SQLite/PostgreSQL schemas, constraints, revisions,
   migrations, and compatibility/recovery examples. Only then implement an
   explicitly selected slice. Document intentional deferrals instead of filling
   missing decisions with code.
