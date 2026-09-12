# Project package schema

## Current exact review packet — 2026-09-12

The [D19 package delta](D19_STATIC_SCHEMA_DELTA.md#package-object-delta-and-closure) now specifies package 1.1 required features, authored owner association, versioned ledger/context/resource/history objects and exact closure. Existing bootstrap/ancestor/fixture bytes stay unchanged; a successor commit carries new semantics. Packages never carry executable ACLs or host grants. R1–R8 were accepted as proposed 2026-09-12; this is accepted design, not codec or writer implementation.

## D19 supersession notice — baseline bytes retained

[D19](LIBRARY_AND_NODE_WORK_SURFACES.md) is the current conceptual authority.
Library replaces persistent Workspace; every Project has exactly one owning
Library. Explicit ProjectAccessGrant/invitations and restricted/library-visible
policies govern Project access independently of blanket Library membership.
Project-only collaborators receive bounded assigned snapshots, not full catalog,
Library feed, media or reset-snapshot access. Package/SMB grants remain separate.

Graph assets are explicit connected AssetSet values (`$input.<port>`), with
no `$project.assets` or ambient semantic Gallery/project union. The package may
retain a private graph/run identity/provenance/artifact ledger; old ProjectAsset
inventory fields are compatibility structures, not implicit node inputs.

The S2–S6 names, DDL, endpoint paths, JSON examples and baseline fixture bytes
below remain unchanged except explanatory amendment prose. Their Workspace,
ProjectAsset, nullable-origin and membership-wide access assumptions are
conceptually superseded where they conflict with D19. They require a separately
reviewed additive/rename/migration plan, **not** direct implementation or silent
reinterpretation. S3 remains 44 tables/179 statements; S4 35 tables/275 statements;
all six applied L2 migration files/checksums are unchanged. Freeze logical/package/
NodeSDK contracts, then review exact static SQLite/PostgreSQL/package/sync deltas,
including grants, project-filtered queries/feeds/snapshots/media/receipts and
compatibility. Revised CXT1 and CXT3 follow that review; L3 remains paused.


## D18 amendment boundary — pending detailed approval

[Typed context and expressions](TYPED_CONTEXT_AND_EXPRESSIONS.md) proposes a
required `photara.context.v1` feature and exact schema-selected references for
Project/Graph/node variables, typed AST/source records, Workspace captures,
metadata selections, immutable Run ContextSnapshots and proposal/application
evidence. Project/Graph variable edits and metadata selections enter authored
digests; recording a Run/snapshot/receipt alone does not change authored Graph
identity. Exact new fields/versions and inventory closure are deferred to CXT2;
the S2 examples below and L1 supported schemas are not silently extended today.

Backticks/fences are opted-in source syntax, not portable executable scripts;
the versioned typed AST is execution authority. `$project.root` is a logical
capability-backed root descriptor, never an absolute path; `$input.assets` names
the node's explicit connected AssetSet port. Persist only permitted typed snapshots, not SecretRefs/grants or
device bindings. Source and AST checksums remain distinct. Existing one-JSON
documents stay valid without D18 data; old L1 readers must reject required D18
semantics rather than ignore them or save lossy packages. L3 is paused for CXT1–3.

Status: S2 design approved with S1–S6/D1–D17 at S7, 2026-09-11. Bounded L1
read-only codecs/validation are implemented; see [supported scope and limits](PROJECT_PACKAGE_CODEC.md).
This document specifies the full target format/protocol, not an implemented
writer, application integration or SMB durability guarantee. It contains no DDL.

## Scope and authority

A `.photara` directory package contains portable Project authority: authored
metadata, typed Library snapshots/assignments, the Project's asset subset,
named Graphs, and separately recorded durable execution history. Its ProjectId
survives movement; its directory name and current device location do not identify
it. Default creation is under `~/Pictures/Photara/Projects`; another authorized
local or SMB root may be selected.

Local SQLite stores Library records, catalog projections, device bindings and
operational intents outside the package. Storexa 0.2.0 provides database
mechanics, not package publication, coordination or authorization; see
[Storexa integration](STOREXA_INTEGRATION.md). No live SQLite database, WAL,
credential, bookmark, mount path or window layout belongs in a package.

S2 compatibility covers the current generation-two single-JSON
[Project Document](PROJECT_DOCUMENTS.md) only. Earlier generations and their
databases are outside this slice and are not acceptance conditions.

## Proposed directory layout

```text
Ocean Study.photara/
├── manifest.json                      immutable bootstrap identity/format
├── HEAD.json                          current publication pointer
├── commits/
│   └── <commit-uuid>.json              immutable commit manifests
├── objects/
│   ├── json/sha256/<64-lowercase-hex>.json
│   └── blobs/sha256/<64-lowercase-hex>  immutable managed bytes
├── .staging/
│   └── <write-uuid>/                   unpublished objects and proposed HEAD
└── .coordination/
    └── writer.lock/                    exclusive ownership, not project meaning
        ├── owner.json                 session nonce and base commit
        └── heartbeat.json             advisory diagnostic observation
```

Graph, assignment, asset and history documents have independent schemas and
object references. Their physical filenames are checksums, while their payloads
retain semantic UUIDs. Thus a saved Graph is still a distinct JSON document,
without overwriting `graphs/<id>.json` during a multi-file save. A read-only
inspection/export tool can produce friendly filenames and pretty JSON.

Content objects and commit manifests are never modified in place. A commit
publishes a complete authored root and history root; unchanged objects are
reused. There is one publication writer for the entire package, including run
history. `.staging` and `.coordination` are operational state, excluded from
semantic digests and ordinary copy/export. They are not discarded while an
owner may still be writing.

Initial v1 retains committed ancestors and their reachable objects. Automatic
compaction or deletion is not part of v1. Storage quotas must reject new work
cleanly rather than evict required evidence. This favors understandable recovery
over minimum disk usage; a separately approved retention format can follow.

## Encoding, identifiers and references

Authoritative JSON is UTF-8, without BOM or trailing newline, serialized with
the named codec `photara.canonical-json.v1`: the existing Core
[`canonical_json`](../../crates/photara-core/src/canonical.rs) behavior—recursive
lexicographic object-key ordering, compact JSON, array order retained, and the
current validated `serde_json::Value` number/string serialization contract.
This is not a claim of RFC 8785 compatibility. S6 must freeze byte vectors for
Unicode, escaping, integer bounds, floating-point spellings and negative zero
before an implementation can write the format. Dependency upgrades must pass
those vectors or introduce another codec version.

Reject duplicate JSON keys, non-finite/unrepresentable numbers and malformed
UTF-8 before canonicalization. Do not normalize user strings as part of hashing.
Numbers in opaque node state retain the supported Core number semantics;
unrepresentable newer values make that object read-only rather than rounding it.

UUIDs use lowercase hyphenated strings. New package counters use unsigned
decimal strings, so package revisions are not limited by a JavaScript number.
Current embedded GraphDocument numeric encodings remain versioned Core fields;
they are not silently rewritten by this envelope. New UTC instants use RFC 3339
with `Z` and explicit millisecond precision. Nullable dates stay null. Schedules
use an explicit tagged calendar date or UTC interval with an IANA display zone;
an interval must have end after start and no ambiguous local wall-clock parsing.

An `ObjectRef` is `{ "kind": "json" | "blob", "sha256": <digest>,
"byte_length": <decimal-string> }`. Its path is computed from kind and digest;
there is no caller-supplied object path. SHA-256 verifies exact stored bytes.
Every authoritative JSON object has a schema ID/version and ProjectId except
package-release manifests, which use their exact package identity. A copied
Library snapshot carries the owning ProjectId as well as its source WorkspaceId.

Keep these digests distinct:

| Digest/revision | Meaning |
| --- | --- |
| Object checksum | Exact stored bytes; formatting changes alter it |
| Graph authored digest/revision | Existing versioned Core GraphDocument semantics |
| Authored-root checksum | Project metadata plus current assignments/assets/Graph references |
| Run source/evaluation digests | Captured graph, configuration, explicit inputs and implementations |
| Package revision/commit checksum | A complete published authored root plus retained history |

Run publication changes the package revision but preserves authored and Graph
digests when those roots are unchanged. Graph name edits affect authored Project
state but need not alter the nested Core GraphDocument or its evaluation key.

## Bootstrap, HEAD and commit manifests

`manifest.json` is created before initial publication and is not the mutable
Project summary. It contains:

| Field | Contract |
| --- | --- |
| `format` | Exact value `photara.project-package` |
| `format_version` | `{major: 1, minor: 0}` for this proposed format |
| `project_id` | Stable Project UUID |
| `created_at` | Package-format creation instant, distinct from Project creation |
| `canonical_json` | `photara.canonical-json.v1` |
| `required_features` | Features needed to safely interpret the package |
| `extensions` | Preserved namespaced optional metadata |

`HEAD.json` contains a HEAD schema version, ProjectId, CommitId and commit byte
checksum. It is deliberately small and has no graph or catalog summary. A valid
HEAD designates exactly one committed snapshot; directory enumeration, newest
timestamp and highest revision do not override it.

Every immutable commit contains CommitId, ProjectId, package revision, parent
commit reference (null for initial revision `1`), creation time, write-operation
ID, required feature/minimum-reader declarations, bootstrap manifest checksum,
authored-root reference, history-root reference and an inventory reference.
Each successor increments the package revision once and names the exact parent
checksum. A fork starts a new chain. A timestamp is never a concurrency token.

The inventory is a sorted list of ObjectRefs for the commit's complete reachable
authored/history closure, including retained opaque objects and blobs. It is a
checksummed authoritative completeness inventory, not a mutable database index.
It excludes itself and the commit/HEAD to avoid checksum cycles. The inventory
records do not replace reference/schema validation. Initial inventory and history
manifests may be linear; sharding requires a compatible schema evolution rather
than an unbounded-performance promise.

Opening validates bootstrap, HEAD, referenced commit and inventory, then typed
JSON objects and internal references. Managed blob size/presence is checked;
large blob hashing can be deferred until materialization or explicit full
verification. Until hashed, bytes are not reported as content-verified. Missing
managed bytes produce a damaged/incomplete-package report; missing external
bytes produce representation unavailability. Both allow safe metadata inspection
without inventing assets or silently discarding descriptors.

Checksums detect inconsistency, not authenticity. A malicious actor able to
rewrite the whole package can recompute them. Package data cannot grant node
permissions or prove trusted executable installation.

## Authored Project root and typed Library context

The authored root has schema `photara.project.authored` version 1 and contains:

- ProjectId; nullable originating WorkspaceId for unassociated current-format
  imports; Project creation/update times; title; description; `active | archived`;
- a monotonically checked authored revision, distinct from package save revision;
- party-assignment, Location-assignment, asset-inventory and resource-inventory
  ObjectRefs, plus a GraphId-sorted list of named Graph ObjectRefs;
- preserved namespaced extension data and explicitly marked compatibility facts.

New Projects receive the selected local/cloud WorkspaceId. A nullable origin
does not imply membership or authorization. Cross-Workspace snapshot references
are permitted as historical data: importing a package conveys only contained
facts, never permission to fetch the source Library. Refreshing a snapshot needs
an authorized lookup in its source Workspace.

Each typed `LibraryReferenceSnapshot` has a SnapshotId, owning ProjectId, source
WorkspaceId/record ID/kind/revision, captured time, display name and typed payload.
Snapshot payloads contain the fields needed by the assignment, not an entire
Account/Library export. Supported kinds are Person, Organization, Location and
LocationKind. Source revision is interpreted in its source authority, never
compared numerically with project revisions. Snapshots are immutable objects;
explicit refresh creates another snapshot and authored revision.

Party assignments have AssignmentId, ProjectId, Person-or-Organization snapshot
reference, a nonempty set of namespaced project roles, notes, revision and
creation/update times. Active assignments are unique by Project plus source
Workspace/kind/record ID; several roles share that assignment.

Location assignments have their own ID/revision/times and exactly one Location
snapshot plus its LocationKind snapshot. The Location snapshot's source kind ID
must match the kind snapshot. They also contain nullable schedule, notes and
participants referencing existing party AssignmentIds plus occurrence roles.
Repeated uses of a Location have different assignment IDs. Removing a party used
by participants requires an explicit reconciliation command. Referenced historical
snapshots remain reachable by older commits and Runs.

LocationKind snapshots preserve canonical display/key, description, aliases and
normalization-policy version. They do not create a second writable taxonomy.
Historical location/kind facts remain unchanged if the Library later reclassifies
the place. Current-Library search and snapshot refresh are explicit operations.

## Asset inventory, representation revisions and resources

An asset inventory contains only the chosen Project subset, keyed by AssetId.
Each asset retains display metadata, creation/update/revision fields and zero or
more representation identities. Removal from current membership is an authored
change; it does not erase history. An AssetSet remains an ordered, duplicate-free
Project-scoped value and can reference only its captured source inventory.

A representation belongs to one AssetId and holds namespaced role/capability
metadata plus a current content-revision reference. An immutable content revision
has a typed UUID, RepresentationId/AssetId/ProjectId, fingerprint and evidence
strength, media descriptor, byte size when known, lineage references, and binding.
New bytes or stronger verification evidence produce a new content revision.
Moving a physical binding does not create a new semantic asset. Runs retain exact
content-revision references, never just a mutable current pointer.

Fingerprint evidence preserves `content-digest`, `provider-revision` and
`file-observation`. Provider/file observations may detect change but do not prove
that bytes equal a SHA-256 content digest. The current Core digest-plus-evidence
representation can be embedded losslessly during compatibility import; no weak
observation is upgraded merely because it is stored in this package.

A content revision binds to exactly one:

1. **Managed ProjectResource:** ResourceId plus immutable blob ObjectRef and a
   generated normalized package-relative path matching that reference. Original
   filename/media type is descriptive metadata. Byte replacement creates another
   resource/content version and never overwrites the previous blob.
2. **External resolution handle:** stable RepresentationStorageBindingId with
   optional logical StorageRootId and normalized relative resource path. Actual
   mounts, provider account locators, security bookmarks and credentials resolve
   through the authorized device host. Import does not silently authorize them.

`StorageRootId` here has the semantic role of a stable Library Storage Location;
the exact compatibility/type rename is part of the D19/CXT2 freeze rather than a
reinterpretation of existing bytes. The same logical location may bind to a macOS
mount, Windows UNC/native handle, Linux mount or provider connection on different
hosts. See [Storage locations and host bindings](STORAGE_LOCATIONS_AND_HOST_BINDINGS.md).

The resource inventory retains resource identity/version and current bindings;
content revisions capture the resource version used. Several descriptors may
refer to the same byte object without merging their semantic identities.
Lineage names source content revisions and OperationId/evidence where known;
self-origin and cycles are invalid.

Managed import copies bytes into staging, verifies them, then publishes the
descriptor and blob in the same commit. Explicit **Collect into Package** copies
external resources without deleting their originals. An external representation
can remain offline indefinitely without corrupting portable authored state.
No hardlink to a mutable outside source is accepted as managed storage.

Reproducible proxies, Gallery thumbnails, decoded images, materialization paths
and rendered previews stay in device cache. A deliberately supplied Project
cover image or snapshot media needed as durable historical content is a managed
blob; regenerable thumbnails of it are cache. Essential evidence artifacts are
managed durable blobs or explicit non-embedded evidence references whose limits
are recorded. An external URL alone does not claim future byte availability.

## Multiple named Graph documents and node packages

Each `photara.project.saved-graph` v1 object contains ProjectId, GraphId, name,
name-normalization version, metadata revision/times, exact required package
releases, and the ordinary versioned Core GraphDocument. The two GraphIds must
match. The existing GraphDocument retains node IDs, connections, configuration,
authored state, revisions and opaque extensions.

Active names are unique per Project after the pinned Unicode/casefold/whitespace
name policy selected in S3/S6. Plural/semantic LocationKind aliasing does not
apply to graph names. A newly created package can contain zero Graphs. Graph
deletion removes a current reference; old commits and Run source snapshots keep
the prior graph available. Multiple Graphs do not imply inter-Graph execution.

Exact package releases are described by immutable manifest ObjectRefs when
available. Definitions retain their namespaced IDs/versions, typed ports/schema
references, declared capabilities, stable primary CategoryId, search metadata,
neutral icon resources and optional Inspector/Work Surface contributions.
The Graph's requirement list equals the package coordinates actually pinned by
its node instances; it must not silently follow the installed latest version.

Installed code, signing/trust state, licenses, prices and provider credentials
are not packaged executable authority. Missing manifests or runtimes leave exact
pins and opaque node state intact with diagnostics. Current manifests lacking
new category IDs are preserved under their old manifest schema; a defined
compatibility mapping supplies catalog presentation later, never an invented pin.
Category/schema upgrades are independent of the package file-format version.

## Immutable execution history

`photara.project.history` v1 is an immutable history root containing RunId-sorted
run references and project-level evidence/operation references. Each updated
root reuses previous immutable facts. Rebuildable browsing indexes are stored
outside the package and keyed by commit checksum; deleting them loses no history.

| Record | Required immutable facts |
| --- | --- |
| GraphRun start | RunId, ProjectId, RequestId, source GraphId/revision/digest, source authored snapshot reference, explicit targets, relevant dependency references/digests, exact implementations, queued/start facts |
| NodeAttempt start | AttemptId, RunId, source NodeInstanceId, unique ordinal within run/node, implementation fingerprint, configuration/input/environment digests, captured representation revisions and start time |
| Terminal outcome | RunId or AttemptId, exactly one terminal status, end time, structured diagnostics, output/artifact/evidence refs |
| Node disposition | Explicit skipped/cache-reused fact and provenance; not an invented attempt |
| EffectIntent | OperationId, ProjectId, originating Run/Attempt when applicable, scoped target, request digest, idempotency key and captured intent time |
| Receipt | ReceiptId, OperationId, observing AttemptId where applicable, provider identifiers, receipt/verification time, typed redacted evidence refs |
| Evidence/artifact | EvidenceId or artifact ID, typed schema, provenance, capture time, immutable content descriptor, managed blob or explicit external reference |

Start, phase and terminal facts are separate immutable objects. A run index
cannot make an existing terminal record change status. Corrections/reconciliation
append observations linked to the original record. Valid terminal statuses are
`succeeded | failed | cancelled | interrupted`; queued/running are derived from
durable start/phase facts. A crash can leave a started attempt with no terminal
fact; recovery records `interrupted` when justified and keeps uncertain external
effects unknown. Cancellation requested is not a cancellation result.

Running an unsaved graph captures an immutable source authored snapshot as a
history dependency. It does not silently replace the Project's saved authored
root. The Run records the saved base commit plus the exact captured snapshot and
Graph revision, so later reopening distinguishes the saved graph from that Run's
input. The publisher serializes publication against concurrent Save commands.

Persist an attempt/start and effect intent before executing an external effect.
Retries reuse the intended OperationId/idempotency key and append new attempts;
an unknown effect is reconciled before unsafe replay. Publish receipts/terminal
facts after the effect. If package storage becomes unavailable, pause new effects
and retain an explicit durable device recovery intent/receipt queue outside cache.
The UI must distinguish **effect observed, package evidence pending** from saved
history. On reconnect, reconcile against the exact package chain and operation
IDs before appending. Losing the response after an external success cannot be
made atomic with an SMB commit; never claim exactly-once delivery from this format.

Essential output bytes must be stored before the terminal fact claims they are
durably available. Run/attempt records refer to immutable dependency versions,
not current Library rows or a mutable graph name. Recording history never enters
the authored graph digest. Deleting cached outputs never deletes receipts or
necessary evidence.

## Single-writer publication protocol

This v1 proposal permits many readers and one cooperative publication writer
per package. One process serializes editor saves and execution evidence commits.
It is not a collaborative multiwriter protocol or a distributed lock service.

1. Open the selected root through a scoped host grant; validate identity, format,
   HEAD and its complete referenced JSON state. Record the expected CommitId,
   checksum and revision. Verify the configured filesystem profile supports the
   necessary exclusive-create/rename/flush operations.
2. Acquire `writer.lock` with an exclusive directory creation primitive. Write
   owner data containing random session nonce, ProjectId, DeviceId, WriteId,
   base commit and acquisition time. Failure or an incomplete owner record means
   unavailable ownership, never permission to delete the lock. Re-read HEAD after
   acquiring; rebase semantic commands or report a conflict if it changed.
3. Stage new objects, inventory, commit and proposed HEAD within the same package
   filesystem. Validate schemas, identity scope, references, sizes, checksums and
   expected authored revisions before publication. Never stage authoritative
   metadata in a different volume and assume its move is atomic.
4. Flush new files; publish immutable objects and the immutable commit without
   replacing existing content. If a digest-named object already exists, verify
   it before reuse. Flush the affected directories where supported. Preserve the
   old HEAD throughout this preparation.
5. Recheck owner nonce and expected parent HEAD immediately before replacement.
   This check relies on non-stealable exclusive ownership; it is not a filesystem
   compare-and-swap primitive. Publish by atomically replacing `HEAD.json` with a
   fully written/flushed sibling temporary HEAD on the same filesystem. Flush
   the package directory according to the validated host/filesystem profile.
6. Re-read and verify HEAD/commit before reporting committed state. Then enqueue
   catalog/index projection updates. Catalog failure does not roll back the
   already published package or make SQLite a second project authority.
7. Remove completed staging and release only the lock bearing this session's
   nonce. Preserve ambiguous staging/recovery material on any uncertain failure.

The publication point is HEAD replacement. Until then, readers use the old
immutable closure. A reader pins the complete HEAD reference it first read;
it may recheck for a newer HEAD and refresh, but never combines roots from two
commits. Immutable objects reachable by old readers are not garbage-collected
in v1.

### Local and SMB limits

An atomic rename controls visible namespace replacement; it does not itself
prove power-loss durability, cross-volume atomicity or a network-wide transaction.
File synchronization and directory-entry persistence are separate concerns in
filesystem APIs. The [rename](https://man7.org/linux/man-pages/man2/rename.2.html)
and [fsync](https://man7.org/linux/man-pages/man2/fsync.2.html) contracts illustrate
that distinction; these Linux references do not certify macOS/SMB behavior.
The implementation must use and test the appropriate macOS primitives, server
flush behavior and the actual SMB configuration before advertising writable
support. A successful client flush cannot establish an unverified NAS's power-loss
guarantees. Record the supported profile and practical limits in L6 results.

Network interruption can make success uncertain. Reopen and inspect the proposed
CommitId/HEAD before retrying. The same WriteId is idempotent only for the same
payload and parent; a different payload requires a new operation. If another
HEAD advanced, the writer stops and resolves revisions; it never overwrites the
newer state because its old request timed out.

The heartbeat is advisory, **not an expiring lease**. A slow or partitioned Mac
may still own a server handle. There is no automatic lock stealing or timeout
takeover. A nonce/fence field in a JSON file cannot prevent a stale client from
writing after an unsafe takeover; server-enforced fencing is not claimed.
Recovery that removes a lock requires confirmed termination/release of the old
writer and revalidation of server ownership and HEAD. If this cannot be
established, remain read-only or recover to an explicit separate copy. Reconnection
invalidates the previous writer session until ownership and parent state are
revalidated. Unsupported exclusive-create/flush behavior means read-only package
support, not a fallback to last-writer-wins.

## Recovery states

| Observed state | Required behavior |
| --- | --- |
| Valid HEAD, old/incomplete staging | Open committed state; keep staging until ownership is resolved; never replay automatically |
| Fully written orphan commit, HEAD unchanged | Treat as uncommitted; present explicit recoverable draft only after validation |
| Save timed out, HEAD equals proposed commit | Verify closure and report that exact operation committed |
| Save timed out, HEAD equals expected parent | Proposed state is not currently published; safe retry requires exclusive ownership and revalidation |
| Different valid HEAD | Conflict/reload; preserve draft/evidence for reconciliation |
| Missing/truncated/corrupt HEAD | Read-only recovery; enumerate verified chains and require explicit selection; do not choose by highest revision/time |
| Referenced metadata/checksum missing or invalid | Damaged package; preserve files and offer verified ancestor/recovery inspection |
| Managed media missing | Incomplete package; retain descriptors and report affected assets/evidence |
| External source missing | Valid package with unavailable representation; authorized locate/rebind may repair it |
| Lock owner crashed or ambiguous | Read-only until prior writer termination and server ownership are established |

Recovering an older committed snapshot creates a new explicit recovery commit
with recorded origin; it does not mutate historical objects or silently rewrite
the old chain. If the current chain cannot be identified, create a separate
recovery candidate and keep the damaged original until the user chooses it.
Recovery of in-flight effects records uncertainty and reconciles provider facts;
it never fabricates terminal success.

## Open, create, move, copy and identity

| Operation | Identity and publication behavior |
| --- | --- |
| New | Allocate ProjectId, create a hidden sibling package, write/verify initial commit, publish final directory without replacing an existing destination, then add catalog projection |
| Open | Validate embedded ProjectId and current commit; no implicit source scan, Library refresh, node execution or migration |
| Save | Publish against the opened exact HEAD; preserve semantic IDs and unresolved optional state |
| Same-volume move/rename | Close/release writer, move the complete package under authorized host operation, verify identity/HEAD and update locator; filenames are not identities |
| Cross-volume move | Verified staged copy plus explicit retirement/removal of source after confirmation; not one atomic rename |
| Locate/rebind | Verify ProjectId and commit lineage at selected candidate; update device binding/catalog, not asset/graph IDs |
| Missing package | Mark locator unavailable; retain catalog identity and last observed summary; no deletion or new project |
| Duplicate ProjectId | Show both candidate locations and lineage/digests; open read-only until the intended active copy is chosen and other writers are excluded |
| Backup copy | Pin one committed snapshot and copy all retained referenced data without locks/staging; preserve ProjectId and commits, so it is not a second independent editing project |
| Duplicate as new / fork | Allocate new ProjectId and remap owned IDs under the explicit rules below; publish a new initial chain |
| Remove from catalog | Remove/hide discovery entry only; package/evidence remains intact |

Even identical copies cannot independently edit under one identity: their local
locks do not coordinate separate package directories. v1 has one nominated
active package location per ProjectId, no automatic replica synchronization.
Selection of an active copy is explicit; divergent copies are never auto-merged.

Fork remaps Graph, NodeInstance, Connection, party/Location assignment, snapshot,
Asset, Representation/content revision, Resource and external binding-handle IDs,
updating all known owned references. Source Library IDs/Workspace provenance and
content fingerprints stay unchanged. Device grants/bookmarks are not copied.
Package/definition/value/schema/category identifiers remain exact external
coordinates. Byte blobs may be copied/reflinked safely; mutable outside hardlinks
are excluded.

A fork begins with authored state and an immutable provenance reference to the
source ProjectId/commit. It does not relabel old Runs/Attempts/receipts as facts
of the new project. Copying execution history into a separately marked source
archive is deferred. Unknown node state that may contain owned IDs requires a
package-supplied remapping contract; absent one, block fork with a diagnostic
instead of guessing. Plain backup copying still preserves such packages losslessly.

## Path safety and untrusted input

Generated internal paths use fixed ASCII directories, validated UUIDs and lowercase
hex digests. Validate relative resource paths independently: no absolute/drive/URI
prefix, NUL, `..`, alternate separators or empty components; a resource cannot
target HEAD, commits, staging or coordination. File/path comparison must not allow
case or Unicode aliases to bypass reserved paths on a case-insensitive volume.

Resolve each component beneath the granted package root with no-follow semantics
and revalidate file identity around publication/materialization; a string-prefix
check or one earlier canonicalization is insufficient against path replacement.
Reject symlinks, directory traversal, device files and unexpected hardlinks in
authoritative managed content. Optional sidecars such as Finder metadata do not
enter inventories or get interpreted as executable input.

Impose host limits on JSON bytes, nesting, object count and materialization size;
oversized input is unsupported/read-only, not truncated. Unknown required files
or features are not run. Schema validation never loads package executables.
Importing assets and following external handles always requires scoped host
capabilities. The native `.photara` package association is presentation/OS metadata
and does not substitute for format validation.

## Versions, unknown fields and current-document compatibility

Bootstrap format, commit, authored Project, snapshot, asset, Graph wrapper,
execution, node-state and canonical-codec versions are independent. An unknown
major version, required feature or incompatible minimum reader/writer requirement
opens only for safe inspection; no save-through is allowed. Minor additions can
be preserved only when explicitly optional and lossless under the supported codec.

Unknown fields on a known document are preserved with namespace ownership.
Unchanged unknown objects are copied/referenced as original verified bytes.
If an edit cannot preserve unknown semantics/references, reject that edit rather
than strip fields. Opaque object inventory entries stay reachable across saves.
Unknown node packages do not prevent safe project opening or an unchanged backup.

Compatibility import of the current generation-two ProjectDocument is explicit
and writes a new sibling package; it never overwrites the input:

1. Preserve the original JSON bytes as an immutable compatibility blob and record
   its schema, checksum and original Project revision as provenance.
2. Preserve ProjectId, GraphId, node/connection IDs, exact pins, resource/asset/
   representation IDs, opaque node state and unknown extensions. Wrap the one
   graph as `Main`, with the original Core GraphDocument unchanged.
3. Map existing metadata and asset context into typed objects. Introduce content
   revision IDs without claiming new verified bytes. The first package revision
   is `1`; source Project revision remains separately recorded.
4. Resolve managed resource bytes against an explicitly selected current project
   root. Copy/verify available bytes; unresolved bytes remain explicit external
   binding requirements or an incomplete-import draft requiring review. Never
   serialize the old machine's absolute path as portable identity.
5. Preserve `photara.library.v1` assignments as compatibility facts when a typed
   Location/LocationKind or Person/Organization mapping is unavailable. Do not
   create a Location without its required kind or silently reinterpret a Client.
   The package stays inspectable/editable outside unresolved contexts; edits of
   those facts require explicit mapping or remain disabled.
6. Preserve session/runtime absence truthfully: the current document has no
   durable Run history to manufacture. Publish only after round-trip inventory
   and semantic-equivalence checks, then update the locator separately.

The current standalone NodeGraphDocument remains an explicit graph-sharing
export. Export only the selected Graph and its exact requirements. Whole-project
export back to the current one-graph format must reject multiple graphs or other
unrepresentable authoritative content unless the user explicitly chooses a
clearly described lossy export; never silently drop durable history or assignments.

## Structural JSON examples

These are valid JSON templates, not checksum-verified fixture packages. Tokens
such as `$PROJECT`, `$COMMIT` and `$SHA_*` must be replaced with typed UUIDs and
computed digests/lengths in S6. They are never permitted literal values in a real
package. Omitted optional fields follow the contracts above.

Bootstrap and publication pointer:

```json
{
  "format": "photara.project-package",
  "format_version": {"major": 1, "minor": 0},
  "project_id": "$PROJECT",
  "created_at": "2026-09-11T18:00:00.000Z",
  "canonical_json": "photara.canonical-json.v1",
  "required_features": ["photara.immutable-objects.v1", "photara.history.v1"],
  "extensions": {}
}
```

```json
{
  "schema": {"id": "photara.package.head", "version": 1},
  "project_id": "$PROJECT",
  "commit_id": "$COMMIT",
  "commit_sha256": "$SHA_COMMIT"
}
```

An evidence-only successor preserves the prior authored reference:

```json
{
  "schema": {"id": "photara.package.commit", "version": 1},
  "project_id": "$PROJECT",
  "commit_id": "$COMMIT",
  "package_revision": "8",
  "parent": {"commit_id": "$PARENT", "sha256": "$SHA_PARENT"},
  "write_id": "$WRITE",
  "created_at": "2026-09-11T18:05:00.000Z",
  "bootstrap_sha256": "$SHA_BOOTSTRAP",
  "minimum_reader": {"major": 1, "minor": 0},
  "required_features": ["photara.history.v1"],
  "authored": {"kind": "json", "sha256": "$SHA_AUTHORED_UNCHANGED", "byte_length": "720"},
  "history": {"kind": "json", "sha256": "$SHA_HISTORY_NEW", "byte_length": "360"},
  "inventory": {"kind": "json", "sha256": "$SHA_INVENTORY", "byte_length": "1800"}
}
```

A concrete Location use references immutable snapshots and project parties:

```json
{
  "schema": {"id": "photara.project.location-assignment", "version": 1},
  "project_id": "$PROJECT",
  "assignment_id": "$LOCATION_USE",
  "revision": "1",
  "created_at": "2026-09-11T18:00:00.000Z",
  "updated_at": "2026-09-11T18:00:00.000Z",
  "location_snapshot": {"kind": "json", "sha256": "$SHA_LOCATION", "byte_length": "420"},
  "location_kind_snapshot": {"kind": "json", "sha256": "$SHA_KIND", "byte_length": "380"},
  "schedule": {"kind": "calendar-date", "date": "2026-09-12"},
  "participants": [{"party_assignment_id": "$PERSON_USE", "roles": ["photara.role.photographer"]}],
  "notes": "Morning session at Shane's Apartment"
}
```

A named empty Graph still preserves the ordinary Core graph payload:

```json
{
  "schema": {"id": "photara.project.saved-graph", "version": 1},
  "project_id": "$PROJECT",
  "graph_id": "$GRAPH",
  "name": "Main",
  "name_normalization_version": 1,
  "metadata_revision": "1",
  "created_at": "2026-09-11T18:00:00.000Z",
  "updated_at": "2026-09-11T18:00:00.000Z",
  "required_packages": [],
  "graph": {"schema_version": 1, "id": "$GRAPH", "revision": 0, "nodes": [], "connections": []}
}
```

Effect uncertainty remains separate from the attempt's terminal fact:

```json
{
  "schema": {"id": "photara.history.attempt-outcome", "version": 1},
  "project_id": "$PROJECT",
  "run_id": "$RUN",
  "attempt_id": "$ATTEMPT",
  "status": "interrupted",
  "ended_at": "2026-09-11T18:06:00.000Z",
  "diagnostics": [{"code": "photara.effect.response-unavailable", "severity": "warning", "message": "Provider outcome requires reconciliation."}],
  "operations": [{"operation_id": "$OPERATION", "knowledge": "unknown"}],
  "evidence": []
}
```

## Decisions requiring approval before implementation

1. Immutable object/commit layout and single mutable HEAD, including initial
   retain-all history and no automatic compaction.
2. Codec v1 conformance vectors, schema-version/feature rules and practical
   metadata/size limits; graph-name normalization is coordinated with S3/S6.
3. One non-expiring cooperative writer and explicit stale-owner recovery;
   writable SMB availability depends on tested server behavior, not timeout theft.
4. Managed copy versus external handles, deliberate collection, durable evidence
   byte retention and handling of pending device recovery receipts.
5. Run snapshots may capture unsaved edits without replacing the saved authored
   root; history-only commits still advance package publication revision.
6. Backup versus fork semantics, owned-ID remapping, no live independent replicas,
   and blocking remap when unknown node state cannot be transformed safely.
7. Read-only damaged/unsupported handling, explicit current-document conversion,
   unresolved Library compatibility facts and no silent lossy export.
8. Historical cross-Workspace references do not grant source access; nullable
   Workspace origin is limited to unassociated current-format imports.

## S2 acceptance checklist

- [x] Concrete directory, bootstrap, HEAD, commit and object-reference contracts proposed.
- [x] Authored metadata, typed snapshots/Location assignments, asset/resource inventory and named Graphs specified.
- [x] Exact node pins/category metadata remain separate from executable installation/trust.
- [x] Immutable Runs/Attempts/effects/evidence and history-only publication are separated from authored graph digests.
- [x] Checksum inventory, version/unknown-state preservation and path safety are defined.
- [x] Single-writer staging/publication and failure/recovery states make local/SMB limitations explicit.
- [x] Create/open/save/move/rebind/missing/duplicate/backup/fork semantics are proposed.
- [x] Current generation-two single-JSON compatibility is bounded and non-destructive.
- [x] Structural JSON templates are supplied for S6 executable fixtures.
- [ ] User reviews S2 policy choices as part of the schema proposal.
- [ ] S6 resolves templates into valid checksummed fixture packages and fault-injection cases.
- [x] S7 approved the complete design; bounded L1 read-only validation passes.
- [ ] L3/L4/L6 implement and verify publication, conversion, recovery and SMB behavior.

The next planning slice is S3 local SQLite schemas: Library authority, catalog
projections, locator/device bindings, local operation/outbox records and migration
ledger. It must reference package CommitIds/checksums rather than introduce a
second authoritative Project/Graph database model. See the
[execution roadmap](../ROADMAP_0_2_EXECUTION.md) and
[active handoff](../ACTIVE_HANDOFF.md).
