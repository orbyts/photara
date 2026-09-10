# Library, project context, and sync architecture

Photara has three different catalog-like concepts. They must remain separate:

- **Node Catalog** discovers installed node definitions.
- **Gallery** presents visual assets produced by the active project or node.
- **Library** stores reusable user or studio knowledge: People, Clients,
  Locations, and Scenes.

The Library is local-first. Every Mac has a SQLite working copy, created lazily
in Application Support on first use. **Photara Cloud** and a future **iCloud**
option add synchronization to that local store; they do not replace it. A
project remains usable offline and the portable Project Document remains the
authority for graph-authored state.

## Domain model

Accounts and People are distinct. An authenticated account owns or belongs to
a Library workspace; a Person is a photographic subject, model, photographer,
stylist, or other collaborator. This leaves room for team/studio ownership
without equating an Auth0 identity with a Person record.

- **People** have stable identity, display name, aliases, and roles such as
  `model`.
- **Clients** represent an individual or organization commissioning work.
- **Locations** may form a hierarchy, such as Ocean Beach and a specific
  sub-location.
- **Scenes** are reusable semantic descriptions such as Beach Shoot. A scene
  assignment to a project is a unique occurrence and may later carry date,
  notes, location, and people.

Projects reference stable Library record IDs and retain display-name and
revision snapshots. The snapshot keeps a project intelligible while offline
and after a Library record is renamed, merged, or deleted. Project Info is a
project-facing composition of these assignments, not a second copy of Library
records.

## Native modules and labs

The macOS workspace provides independently identified modules for Graph,
Gallery, Inspector, optional node Work Surface, People, Locations, Scenes, and
Project Info. Each can be closed and restored by the user. Placement,
visibility, splits, tabs, floating geometry, and selected workspace preset are
native workspace preferences and never dirty the Project Document.

Each new module has an authoring lab that compiles the same production sources:

| Shared module | Authoring host | Responsibility |
| --- | --- | --- |
| `photara-people` | `photara-people-lab` | Browse, search, create, edit, and select People and Clients. |
| `photara-locations` | `photara-locations-lab` | Browse and author hierarchical Locations and sub-locations. |
| `photara-scenes` | `photara-scenes-lab` | Browse and author reusable Scene definitions. |
| `photara-project-info` | `photara-project-info-lab` | Assign Library records to the current project and author project-specific occurrence details. |

Clients begin in the People lab as an adjacent person/organization category.
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

The top-right application area may expose Account, People, Locations, and
Scenes shortcuts. Those shortcuts reveal or focus the corresponding module;
they are not separate modal databases. Project Info is a peer workspace module.
Optional node Work Surfaces remain opt-in contributions from exact node
definitions.

## Persistence and synchronization

`photara-library` defines stable records, optimistic revisions, tombstones,
queries, portable project references, and a backend-neutral repository. Its
first adapter is SQLite. The schema must evolve through explicit migrations and
retain change sequencing suitable for an outbox and incremental synchronization.

User-facing storage choices are presented as **Library & Sync**:

1. **On This Mac** — SQLite only; fully usable offline.
2. **Photara Cloud** — the same local SQLite working copy synchronized through
   a Photara service backed by Neon/PostgreSQL.
3. **iCloud** — a future Apple-only synchronization adapter over the same
   domain and local cache.

The desktop application must not ship a privileged Neon connection string.
Auth0 Universal Login authenticates a native client using Authorization Code
Flow with PKCE. The app sends access tokens to a Photara API; that service
enforces workspace ownership, subscription, validation, migrations, and
conflict policy before accessing Neon. PostgreSQL row-level security is useful
defense in depth, not a substitute for the service boundary.

Sync records need a stable workspace owner, monotonically checked revisions,
modified timestamps, tombstones, change cursors, idempotent mutations, and a
deterministic conflict policy. Deletes must synchronize as tombstones until all
relevant replicas have observed them. Credentials, refresh tokens, device
paths, and security-scoped bookmarks remain in platform-secure device storage.

## Delivery sequence

Before Layout UI authoring expands:

1. Freeze the Library domain vocabulary and local SQLite migration policy.
2. Expose immutable Library and Project Info presentation contracts.
3. Add People, Locations, Scenes, and Project Info shared modules and labs.
4. Add their independently restorable shell identities and a default workspace.
5. Add Library & Sync settings, with On This Mac functional first.
6. Exercise cross-project queries by person, client, location, and scene.

Photara Cloud follows the same contracts with Auth0/API/Neon integration.
CloudKit remains a later adapter. Migration from the Photara 0.1.x Neon schema
begins only after the new identities and relationships can represent legacy
records without loss.

## Working first model (September 2026)

`PhotaraLibrary` is a UniFFI host facade over `photara-library`. Its constructor
retains a path and owner ID; the SQLite connection and schema are created only
on the first Library read/write. The default location is
`~/Library/Application Support/Photara/Library/library.sqlite`. A device-local
workspace UUID is stored in native preferences independently from Person records.
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

Every Person, Client, Location and Scene has a thumbnail slot. The host prepares
an orientation-correct PNG up to 256 px and imports it into the local managed
`Library/media/<sha256>.png` store. Only the digest enters the portable Library
record. Native DTOs resolve bytes separately; missing images use a record-specific
symbol/initials placeholder. Thumbnails are Library media, separate from project
HDR/SDR Gallery proxies. A future sync adapter must synchronize these media objects;
this build does not upload them or reclaim unreferenced media automatically.

New Project reveals Project Info. Assign from Library searches existing records
by name, alias, labels or description, and offers Create New for a missing record.
Creation uses the exact People/Locations/Scenes editors, saves one Library record,
then assigns its stable identity. A failed project assignment leaves the reusable
Library record intact. Existing projects can open Project Info from Workspace.
The first native browsing projection is bounded to 500 live records; paged,
server-backed discovery and cross-project indexed search are later work.

Project assignments serialize under the versioned `photara.library.v1` Project
Document extension. A generic Core `SetExtension` compare-and-swap command owns
publication; Core does not depend on SQLite or Library rendering types. The
extension has its own monotonic edit revision in addition to the document's
save revision, so two unsaved edits cannot bypass stale-value checks. Each
assignment includes owner, record ID/kind, project role and name/revision snapshot.
Every scene assignment receives a unique occurrence UUID, even for the same scene.
Date/schedule and notes belong to that occurrence. Removing an assignment does
not delete its Library record. Project Info provides session undo through the
same Core command. Unknown future Project Info fields reject editing instead
of being silently erased. Assignment changes dirty the project but leave the
graph digest unchanged; local Library edits and workspace preferences dirty neither.

The Settings pane and Account shortcut expose Library & Sync with On This Mac
active. Photara Cloud and iCloud are visibly planned/unavailable. Auth0 PKCE,
Photara API, Neon access, subscriptions, media transfer, sync conflicts, CloudKit,
backup/recovery UI and legacy migration remain intentionally unimplemented.
