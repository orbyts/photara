# Library, project context, and sync architecture

**D19 supersession note (2026-09-12):** [Libraries and node Work Surfaces](architecture/LIBRARY_AND_NODE_WORK_SURFACES.md)
is the current conceptual target. Library replaces durable Library; each Project
has one Library and explicit Project access. Graphs use connected AssetSets and
declared frozen context; a private package ledger is not an ambient Gallery/asset
union. Library management is app-owned; node Work Surfaces embed authorized host
pickers/components. First install opens local My Library. The pre-D19 implementation,
physical identifiers, examples and fixture contracts below remain baseline evidence,
not approval to reinterpret stored bytes. Exact contract/static schema review and
revised CXT1/CXT3 precede L3; no migration, source or fixture bytes change here.

Photara has three different catalog-like concepts. They must remain separate:

- **Node Catalog** discovers installed node definitions.
- **Gallery** presents visual assets produced by the active project or node.
- **Library** stores reusable user or studio knowledge: People, Organizations,
  Location Kinds, and concrete Locations.

The Library is local-first. Every Mac has a SQLite working copy, created lazily
in Application Support on first use. **Photara Cloud** and future **Apple CloudKit**
option add synchronization to that local store; they do not replace it. A
project remains usable offline and the portable Project Document remains the
authority for graph-authored state.

## Domain model

Accounts and People are distinct. An authenticated account owns or belongs to
a Library library; a Person is a photographic subject, model, photographer,
stylist, or other collaborator. This leaves room for team/studio ownership
without equating an Auth0 identity with a Person record.

- **People** have stable identity, display name, aliases, and roles such as
  `model`.
- **Organizations** represent studios, companies, and other non-person identities.
  Client is a relationship or project role held by a Person or Organization,
  not a duplicate identity type.
- **Location Kinds** are library-unique semantic concepts such as Beach, Home
  Studio, Apartment, or Commercial Studio. They have descriptions and aliases.
  `beach`, `Beach`, and `beaches` must resolve to one concept under an approved
  canonicalization policy.
- **Locations** are concrete places such as Ocean Beach or Shane's Apartment,
  may form a hierarchy, and must reference exactly one Location Kind.
- A project-specific **Location assignment** may carry date, notes, participants,
  and historical snapshots without duplicating the reusable Location.

The current schema-v1 `Scene` and `Client` record kinds predate this target and
are transitional. Do not silently rename them in storage: the schema slice must
define explicit migrations, including ambiguity reports for legacy Scene values.
The UI may continue to use “Scene” as a friendly label for Location Kind if the
terminology review approves it.

Projects reference stable Library record IDs and retain display-name and
revision snapshots. The snapshot keeps a project intelligible while offline
and after a Library record is renamed, merged, or deleted. Project Info is a
project-facing composition of these assignments, not a second copy of Library
records.

## Native modules and labs

The macOS library provides independently identified modules for Graph,
Gallery, Inspector, optional node Work Surface, People, Location Kinds,
Locations, and Project Info. Each can be closed and restored by the user.
Placement, visibility, splits, tabs, floating geometry, and selected library
preset are native library preferences and never dirty the Project Document.

Each new module has an authoring lab that compiles the same production sources:

| Shared module | Authoring host | Responsibility |
| --- | --- | --- |
| `photara-people` | `photara-people-lab` | Browse, search, create, edit, and select People and Organizations, including client relationships. |
| `photara-locations` | `photara-locations-lab` | Browse and author hierarchical Locations and sub-locations. |
| `photara-scenes` (transitional name) | `photara-scenes-lab` | Browse and author Location Kinds until final module/UI naming is approved. |
| `photara-project-info` | `photara-project-info-lab` | Assign Library records to the current project and author project-specific Location details. |

Client relationships begin in the People lab alongside people and organizations.
If real workflows show that client management needs substantially different
interaction, it can be extracted into its own module without changing record
identity or persistence contracts.

All modules consume immutable presentation values and emit semantic actions.
They do not issue SQL, hold cloud credentials, or interpret provider payloads.

## Application surface frame

The Spotify reference defines composition rather than branding. Photara uses
an application canvas behind independent, rounded, filled module surfaces with
visible gutters. Each surface includes its own header and scrolling region.
Light and Dark appearances use semantic theme roles rather than copying
Spotify's black palette.

Shell Lab owns the authorable application-frame values: canvas color/material,
surface fill, radius, gutter, inset, elevation or border, active emphasis,
header treatment, compact stacking behavior, and status surface. Feature labs
continue to own the content inside their surfaces. Native title-bar behavior
remains system-managed.

The top-right application area may expose Account, People, Location Kinds, and
Locations shortcuts. Those shortcuts reveal or focus the corresponding module;
they are not separate modal databases. Project Info is a peer library module.
Optional node Work Surfaces remain opt-in contributions from exact node
definitions.

## Persistence and synchronization

`photara-library` defines stable records, optimistic revisions, tombstones,
queries, portable project references, and a backend-neutral repository. Its
first adapter is SQLite. The schema must evolve through explicit migrations and
retain change sequencing suitable for an outbox and incremental synchronization.

The sibling Rust crate [`storexa`](../../storexa/README.md) is the intended
database mechanics layer. Version 0.2.0 supplies explicit PostgreSQL and SQLite
SQLx connections, transactions, health, tracing, and application-owned migration
execution. `photara-library` continues to own its
domain types, repository behavior, SQL/schema, validation, and sync policy.
Migration from the current direct `rusqlite` adapter happens only after a Storexa
SQLite adapter proves behavioral and recovery parity. CloudKit is modeled as a
later record/change synchronization capability, not as a relational transaction.

User-facing storage choices are presented as **Library & Sync**:

1. **On This Mac** — SQLite only; fully usable offline.
2. **Photara Cloud** — the same local SQLite working copy synchronized through
   a Photara service backed by Neon/PostgreSQL.
3. **Apple CloudKit** — a future Apple-client-only synchronization adapter over
   the same domain and local cache. It remains listed as planned/unavailable
   until an Apple Developer account, container, entitlements, and test environment
   exist.

The desktop application must not ship a privileged Neon connection string.
Auth0 Universal Login authenticates a native client using Authorization Code
Flow with PKCE. The app sends access tokens to a Photara API; that service
enforces library ownership, subscription, validation, migrations, and
conflict policy before accessing Neon. PostgreSQL row-level security is useful
defense in depth, not a substitute for the service boundary.

Sync records need a stable library owner, monotonically checked revisions,
modified timestamps, tombstones, change cursors, idempotent mutations, and a
deterministic conflict policy. Deletes must synchronize as tombstones until all
relevant replicas have observed them. Credentials, refresh tokens, device
paths, and security-scoped bookmarks remain in platform-secure device storage.

## Delivery sequence

Before Layout UI authoring expands:

1. Freeze the Library domain vocabulary and local SQLite migration policy.
2. Expose immutable Library and Project Info presentation contracts.
3. Migrate and add People, Organizations, Location Kinds, Locations, and Project
   Info shared modules and labs.
4. Add their independently restorable shell identities and a default library.
5. Add Library & Sync settings, with On This Mac functional first.
6. Exercise cross-project queries by person, client relationship, concrete
   location, and Location Kind.

Photara Cloud follows the same contracts with Auth0/API/Neon integration.
CloudKit remains a later adapter. Migration from the Photara 0.1.x Neon schema
begins only after the new identities and relationships can represent legacy
records without loss.

## Working first model (September 2026)

`PhotaraLibrary` is a UniFFI host facade over `photara-library`. Its constructor
retains a path and owner ID; the SQLite connection and schema are created only
on the first Library read/write. The default location is
`~/Library/Application Support/Photara/Library/library.sqlite`. A device-local
library UUID is stored in native preferences independently from Person records.
A Swift actor serializes local Library reads and edits away from the main actor.

Schema v1 uses explicit `user_version`, WAL, a busy timeout, and an immediate
transaction for migrations and compare-and-swap writes. Record kind is stable;
revisions start at one and advance exactly once per accepted replacement.
Location parents must be live records in the same owner scope; cycles and
parent deletion while live children exist are rejected atomically. Deletion
retains a tombstone. The backend-neutral `changes` API pages owner-scoped change
identities after an exclusive sequence cursor; consumers resolve the latest
record/tombstone with `get`. This is change discovery groundwork, not a complete
cloud outbox, mutation deduplication or conflict-resolution service.

Schema v1 currently gives every Person, Client, Location and Scene a thumbnail
slot. The target migration preserves media while replacing Client and Scene
semantics with Organizations/relationships and Location Kinds. The host prepares
an orientation-correct PNG up to 256 px and imports it into the local managed
`Library/media/<sha256>.png` store. Only the digest enters the portable Library
record. Native DTOs resolve bytes separately; missing images use a record-specific
symbol/initials placeholder. Thumbnails are Library media, separate from project
HDR/SDR Gallery proxies. A future sync adapter must synchronize these media objects;
this build does not upload them or reclaim unreferenced media automatically.

New Project reveals Project Info. Assign from Library searches existing records
by name, alias, labels or description, and offers Create New for a missing record.
Creation currently uses the exact People/Locations/Scenes editors, saves one
Library record, then assigns its stable identity. A failed project assignment leaves the reusable
Library record intact. Existing projects can open Project Info from Library.
The first native browsing projection is bounded to 500 live records; paged,
server-backed discovery and cross-project indexed search are later work.

Project assignments serialize under the versioned `photara.library.v1` Project
Document extension. A generic Core `SetExtension` compare-and-swap command owns
publication; Core does not depend on SQLite or Library rendering types. The
extension has its own monotonic edit revision in addition to the document's
save revision, so two unsaved edits cannot bypass stale-value checks. Each
assignment includes owner, record ID/kind, project role and name/revision snapshot.
Every current scene assignment receives a unique occurrence UUID, even for the
same scene. In the target model this becomes a project-specific Location
assignment referencing a concrete Location and its Location Kind snapshot;
date/schedule and notes belong to that assignment. Removing an assignment does
not delete its Library record. Project Info provides session undo through the
same Core command. Unknown future Project Info fields reject editing instead
of being silently erased. Assignment changes dirty the project but leave the
graph digest unchanged; local Library edits and library preferences dirty neither.

The Settings pane and Account shortcut expose Library & Sync with On This Mac
active. Photara Cloud and iCloud are visibly planned/unavailable. Auth0 PKCE,
Photara API, Neon access, subscriptions, media transfer, sync conflicts, CloudKit,
backup/recovery UI and legacy migration remain intentionally unimplemented.
