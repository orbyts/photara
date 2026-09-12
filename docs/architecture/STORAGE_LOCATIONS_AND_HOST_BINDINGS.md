# Storage locations and host bindings

Status: **CXT1a portable storage contracts complete**, 2026-09-12. R1–R8 and
CXT2 inert DDL remain accepted. See [implementation and limits](CXT1A_CONTRACTS.md).
[CXT1b](CXT1B_CONTEXT_CONTRACTS.md) additionally binds captured slot revision/target, narrows logical addresses and freezes private device facts. No filesystem resolver, migration, package codec, native UI or deployment is added.

This document is the canonical boundary between portable storage meaning and the
way one device reaches bytes. It complements [D19](LIBRARY_AND_NODE_WORK_SURFACES.md),
the [typed context contract](TYPED_CONTEXT_AND_EXPRESSIONS.md), and the
[Project package schema](PROJECT_PACKAGE_SCHEMA.md).

## Accepted exact contract and compatibility

The [D19 freeze](D19_CONTRACT_FREEZE.md#storage-resolution-slots-and-retention)
resolves this page's checklist: stable SlotId distinct from VariableId, versioned
external resource identity distinct from HostBindingId, per-device candidate and
selection records, portable ResourceDescriptor versus live lease, verified rebind,
Linux/Windows/macOS behavior and publisher-only managed artifact namespace.
The [static delta](D19_STATIC_SCHEMA_DELTA.md) maps existing StorageRoot UUIDs
without changing them and defines exact new tables/package objects. R3 and the
R1–R8 packet were accepted as proposed 2026-09-12. [CXT1a](CXT1A_CONTRACTS.md) now provides portable resource/rights/coordinate validation and nonserializable live leases. CXT3b still owns verified host resolution, rebind CAS, repositories and fake-platform proof.

Correction to the earlier cache list below: resumable transfer **chunks** may be
cache; any journal needed to prevent duplicate effects is durable device recovery.
The proposed `$project.artifacts` spelling is now reserved by R3 as a publisher
request target, never general package-write permission.

## Decision

Photara never uses an absolute path, mounted-volume name, drive letter, provider
credential or security bookmark as portable asset or Project identity. A durable
resource reference has two deliberately separate parts:

1. a stable, Library-owned **Storage Location** describing the logical source or
   destination; and
2. a **Host Binding** that lets one authorized device resolve that logical
   location to a filesystem root, native handle or provider connection.

The same logical reference can therefore resolve on macOS, Windows and Linux:

```text
photara-storage://raw-archive/2026/Coastal-Studies/IMG_0001.CR3

macOS    /Volumes/Whisk/Pictures/Archive/2026/Coastal-Studies/IMG_0001.CR3
Windows  \\whisk\Pictures\Archive\2026\Coastal-Studies\IMG_0001.CR3
Linux    /mnt/whisk/Pictures/Archive/2026/Coastal-Studies/IMG_0001.CR3
```

The URI above is explanatory source notation, not an approved serialized URI
scheme. Portable authority uses typed IDs and normalized relative components.

## Identities and authorities

The exact type names are frozen with CXT2/NodeSDK, but the semantic roles are:

| Concept | Owner / authority | Portable or synchronized content |
| --- | --- | --- |
| Storage Location | Library | Stable ID, user-facing name, purpose, provider/storage kind, declared capabilities and lifecycle |
| Storage slot | Library variable/configuration | Stable machine name such as `raw_archive` bound internally to a Storage Location ID |
| Host Binding | One device/host | Physical root/native handle, bookmark or secure provider-connection reference, validation and availability |
| External resource reference | Project package | Storage Location ID, normalized relative path or stable provider object ID, content/revision evidence and provenance |
| Managed Project resource | Project package | Stable resource ID plus package-managed immutable object/version |
| Materialized handle | One operation/run | Revocable verified local handle with bounded read/write rights |

Renaming a Storage Location or slot never changes its stable ID or retargets a
compiled expression. Rebinding a device changes only host resolution. It does not
change AssetId, RepresentationId, content revision, Graph digest or ProjectId.

The Library database may synchronize logical Storage Locations and permitted
metadata. Host Bindings are device authority and do not synchronize. Credentials,
bookmark bytes, mount paths and bearer tokens remain in platform secure storage
or other host-owned protected state; the database/package stores only opaque
references where necessary.

## Resolution and capability flow

Resolution is explicit and capability checked:

```text
typed portable resource reference
        -> Library Storage Location ID
        -> current device Host Binding
        -> current authorization and containment validation
        -> verified materialized handle
        -> bounded node operation
```

A Library or Project permission does not mount a share, grant filesystem access,
or mint provider credentials. Conversely, possession of a local mount or package
does not grant Library membership. A node receives only the materialized handle
and rights declared for its operation; it never receives a database connection,
raw bookmark, credential, or unrestricted root path.

Availability is an observation: `available`, `unavailable`, `denied`, `stale`,
`ambiguous` and `unsupported` remain distinguishable. An unavailable Host Binding
does not mean that an Asset, Project or Storage Location was deleted.

## Asset and artifact storage classes

Photara distinguishes storage class from semantic asset identity:

1. **External source resources** — existing RAW archives, Lightroom-controlled
   media, cloud objects, NAS files and other provider-owned bytes. The Project
   records portable references and evidence; it does not silently copy or own the
   archive.
2. **Managed Project resources** — deliberately collected originals or durable
   artifacts that travel with the `.photara` package. Publication is immutable
   and package-controlled.
3. **External output artifacts** — exports, sidecars, application updates,
   deliveries and publications written through an explicit effect. The package
   retains their logical references, intent and receipts without claiming to own
   or be able to recall external bytes.
4. **Transient device cache** — thumbnails, proxies, decoded previews, temporary
   renders and resumable-transfer state. It is disposable, host-local and never
   portable Project authority.

A deliberately retained cover image, historical snapshot or evidence artifact is
a managed Project resource; a regenerable thumbnail is cache. Node definitions
declare output/effect behavior and retention expectations rather than relying on
filename or destination inference.

## Project package and AssetSet boundary

The Project package is the portable workflow document. It owns authored Project
metadata, saved Graphs, Project variables, bounded Library snapshots/assignments,
its private graph/run asset and provenance ledger, managed resources, and durable
effect evidence. It records only the asset/resource subset encountered or retained
by its Graphs and Runs. It does not embed or enumerate the entire Library archive.

Graph ports continue to carry explicit typed AssetSet values. A node consumes
only the exact connected membership plus declared frozen context. The private
package ledger supports identity, caching, provenance, recovery and history; it
is not an ambient `$project.assets` input or implicit Gallery union.

Large AssetSets are logically immutable snapshots. Their eventual transport may
use an identity/digest plus paged membership/materialization access rather than
copying full records through every process boundary. That optimization must
preserve exact membership, order, referenced revisions and authorization.

## Variables and expressions

Storage expressions return typed handles, never interpolated absolute path
strings. Examples of approved conceptual source notation are:

```text
`$library.storage.raw_archive`
`path.join($library.storage.raw_archive, "2026", "Coastal-Studies")`
`$project.root`
`path.join($project.artifacts, "masters")`
`$HOME`
```

- `$library.storage.<slot>` is a Library-scoped typed Storage Location reference.
- `$project.root` is the capability-backed root of the current Project package.
- `$project.artifacts` is a proposed typed managed-artifact root; its exact
  built-in name and rights require the context/package freeze.
- uppercase HostPlaces such as `$HOME` and `$DOWNLOADS` are resolved at runtime
  from a closed registry and are never serialized as machine paths.
- node-private and Project variables may contain typed portable resource
  references, but not credentials, bookmarks or raw absolute paths.

Nodes may propose changes to declared Project-scoped variables only through a
revision-checked Project command and the node's declared capability. Data that is
part of workflow flow belongs on a typed port; a variable must not create a hidden
AssetSet or data dependency between distant nodes.

## Move, locate and rebind

Moving a Project package and rebinding a Storage Location are different actions:

- moving a package verifies ProjectId and commit lineage, then updates its Project
  locator/catalog observation;
- rebinding a Storage Location verifies the selected root/provider using identity,
  sentinel and/or content evidence appropriate to that provider, then updates only
  the current device Host Binding;
- a missing mount keeps the logical reference intact and reports unavailable;
- two plausible package locations or two incompatible roots report ambiguity
  instead of choosing the newest path automatically;
- copying a package is a backup/replica until an explicit duplicate-as-new/fork
  allocates and remaps identity.

The UI must name the unavailable logical location and offer connect, locate or
rebind actions without presenting absence as deletion. Rebinding must preview its
scope and verification result before committing.

## Contract-freeze checklist

Before physical schema/NodeSDK implementation resumes, freeze:

- exact Storage Location, slot, Host Binding and external-resource-reference IDs;
- storage/provider kinds, capability vocabulary and lifecycle states;
- slot uniqueness, rename/tombstone and ID-bound expression behavior;
- relative-path/provider-object normalization and containment rules;
- resolution, materialization, revocation and secure-reference interfaces;
- rebind verification and ambiguous/offline diagnostics;
- managed/external/cache retention declarations;
- AssetSet snapshot/paging semantics without weakening exact dataflow;
- package, SQLite, PostgreSQL and synchronization deltas without modifying the
  already-applied L2 migrations in place.
