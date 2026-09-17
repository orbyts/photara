# PS2 sealed-root wire appendix and canonical-byte review

Status: **candidate exact wire contract for review; no production codec is
implemented**. This fills in the [approved production boundary](PS2_PRODUCTION_CODEC_AND_PUBLICATION_CONTRACT.md)
without enabling live conversion, writing, Asset Store publication, retirement,
or project switching. The [golden vectors](proposals/ps2/sealed-wire-golden.json)
are byte-encoding examples, not proof of storage durability or semantic closure.

## Version coordinates are independent

The proposed `minimum_reader: {"major":1,"minor":2}` is **only the selected
commit's required reader capability floor**. It is not a public app/Cargo/API
version, package filename extension, bootstrap `format_version`, HEAD schema,
outer commit schema, canonical JSON version, journal frame version, authored or
Graph revision, or a migration number. Conversion preserves the original
bootstrap bytes and HEAD envelope. Existing `photara.package.head` and
`photara.package.commit` remain schema version 1; `photara.canonical-json.v1`
remains unchanged. New root/resource records each have their own schema version
1; the new authored `photara.project.representation-content` form is version 3
because version 2 already exists. Stable internal `photara.*` protocol IDs do
not change with the eventual public brand.

The selected commit v1 adds required `root_set`,
`photara.sealed-roots.v1` in its sorted `required_features`, and minimum reader
1.2. `photara.resource-backings.v1` is also required if any new managed-backing
record or v3 `managed-captured` binding is selected. All retained bootstrap
features remain required. A 1.2-capable reader requires feature, floor and
typed root-set discriminator together; none is an optional-marker fallback.
The current 1.1 reader must refuse before its legacy parent walk. It neither
rewrites nor reinterprets an unconverted package.

## Common encoding and reference rules

- Existing canonical JSON: compact UTF-8, recursively sorted object keys,
  array order preserved, no BOM or trailing newline. This is Photara's current
  codec, not a claim of RFC 8785. Parse rejects duplicate keys, invalid UTF-8,
  noncanonical numeric spelling and noncanonical byte re-encoding.
- `Id` is an existing typed, nonnil, lowercase canonical UUID. `Digest` is 64
  lowercase SHA-256 hex characters. `Decimal` is an unsigned u64 encoded as a
  canonical decimal **string**. Timestamps use the existing checked millisecond
  UTC representation. `JRef` is the existing exact JSON `ObjectRef`:
  `{ "kind":"json", "sha256":Digest, "byte_length":Decimal }`.
- An `ObjectRef` digest is SHA-256 of the exact stored canonical bytes, with no
  new digest domain. Existing ObjectRef order is Blob before Json, then digest,
  then **numeric** byte length. Inventory arrays are strictly sorted/unique by
  that order. Identity-keyed arrays below are sorted/unique by their stated ID
  or ordinal, not by JSON lexical order.
- Every named new JSON record has required `schema:{id,version:1}`,
  `project_id:Id` and `extensions:{}` (or checked, preserved namespaced optional
  entries). Required fields never disappear; nullable fields encode `null`.
  Unknown required schema/feature refuses. Unknown optional extensions are
  preserved byte-exactly and never create reference edges.
- Only typed fields designated `JRef` create closure edges. UUIDs, digests,
  logical coordinates, optional extension values and predecessor commitments
  are **not** filesystem edges, even when they look like refs. No root may
  require its former recovery root merely because it records predecessor
  provenance.

## Dispatch, RootSet and StateRoot

`HEAD.json` alone selects the immutable commit by exact ProjectId, CommitId and
commit digest. Its bytes remain the sole authoritative atomic dispatch point.
The existing outer commit v1 fields stay present. Under the sealed feature its
`parent` is a prior-commit provenance commitment, **not** a required traversal
edge; the new reader does not apply the legacy revision-1 genesis rule.
`package_revision` advances by one from the persisted old HEAD on publication;
standalone opening cannot prove a missing predecessor's numerical succession.
The outer `authored`, `history` and `inventory` JRefs equal the active root's
same fields exactly.

The required commit member `root_set` is an inline object with **exact** fields:

| Field | Type / rule |
| --- | --- |
| `schema` | `photara.package.root-set` v1 |
| `project_id`, `library_id` | Id; match selected package and roots |
| `bootstrap_sha256` | Digest of original `manifest.json` bytes |
| `kind` | Exact string `sealed` |
| `active`, `recovery` | JRef to `photara.package.state-root` v1 |
| `pinned_roots` | Array sorted/unique by `pin_id`; `{pin_id:Id, reason: "explicit-history"\|"unresolved-recovery"\|"undo", root:JRef}`; no implicit ancestry pin |
| `operation_index` | JRef to `photara.package.operation-index` v1; equals active accepted index |
| `conversion_source` | `null` or JRef to one `photara.package.conversion-source` v1; separately pinned |
| `inventory` | JRef to existing `photara.package.inventory` v1 containing exact RootSet dependency union |
| `extensions` | Checked/preserved namespaced optional object |

Each StateRoot v1 has required `schema`, `project_id`, `library_id`,
`bootstrap_sha256`, `root_id:Id`, `authored_revision:Decimal`, `authored:JRef`,
`history:JRef`, `resource_state:null|JRef`, `operation_index:JRef`,
`accepted:{through_ordinal:Decimal,prefix_sha256:Digest}`,
`journal_inclusion:null|{journal_id:Id,through_sequence:Decimal,
prefix_sha256:Digest,resulting_authored_revision:Decimal,
resulting_authored_sha256:Digest}`, `predecessor:null|{root_sha256:Digest,
authored_revision:Decimal}`, `inventory:JRef`, and `extensions`.
`predecessor` is a commitment, not a closure edge. Initial opt-in conversion
may set active and recovery to the same independently valid root; its accepted
ordinal is zero and journal inclusion is null. If `resource_state` is null, no
new managed-backing semantics are selected. A commit carrying the backing
feature requires a non-null ResourceState, even when it selects empty arrays.

The StateRoot inventory equals the schema-defined package-object closure from
`authored`, `history`, `resource_state` if non-null, and the selected
`operation_index`. It excludes the StateRoot and its own inventory object to
avoid a digest cycle. The RootSet inventory equals the union of active,
recovery and explicitly pinned StateRoot objects, each root inventory and
closure, the selected operation index/receipts, and conversion-source record
dependencies. It excludes itself and the containing commit/HEAD. Original
conversion snapshot *files* have their own exact path manifest, not fake
`objects/json` references. The retained package keep-set adds the RootSet
inventory and selected commit/HEAD/bootstrap plus separately counted journal,
index and staging overhead. Active and recovery are each fully validated
without traversing the other root or predecessor.

## Accepted operations, intent digest and Saved

`photara.package.operation-index` v1 contains required `schema`, `project_id`,
`library_id`, `bootstrap_sha256`, `accepted:{through_ordinal:Decimal,
prefix_sha256:Digest}`, `entries`, and `extensions`. `entries` are ordered by
package-wide acceptance ordinal 1..N without gaps or duplicate operation IDs.
Each entry is `{acceptance_ordinal:Decimal,operation_id:Id,
request_sha256:Digest,receipt:JRef}`. The receipt target is
`photara.package.operation-receipt` v1 with matching `project_id`,
`library_id`, `bootstrap_sha256`, `operation_id`, `request_sha256`, `acceptance_ordinal`,
`journal_id:Id`, `journal_sequence:Decimal`, `outcome:"accepted"`,
`before` and `after` authored coordinates `{revision:Decimal,digest:Digest}`,
`undo_group_id:null|Id`, and `provenance` as checked credential-free historical
evidence. Before/after digests are **commitments**, not JRefs retaining obsolete
authored trees. A no-op may have equal before/after coordinates.

The canonical semantic intent digest is SHA-256 of Photara canonical JSON for
`{domain:"photara.package.operation-intent.v1",version:1,project_id,
library_id,bootstrap_sha256,operation_id,expected,command,updated_at,
undo_group_id,boundary}`. `expected` is the typed PS1 AuthoredCoordinate;
`command` is the reviewed PS1 tagged AuthoredCommand subset (ProjectMetadata,
RenameGraph, and Core GraphCommandEnvelope only where its mapping is supported).
`boundary` is `begin|continue|end|single`; the first writer supports `single`
and `undo_group_id:null`, refusing unsupported grouping rather than silently
dropping it. Transport attachment, owner epoch, device incarnation, native
paths, credentials and refreshed authorization are not digest inputs. A future
command variant requires an explicit compatible command/feature evolution, not
arbitrary JSON execution.

The intent's `expected` is exactly `{revision:Decimal,
authored_digest:Digest,graphs:{GraphId:{revision:Decimal,
semantic_digest:Digest,payload_digest:Digest,envelope_digest:Digest}}}`;
`graphs` is a JSON object keyed by canonical GraphId strings, with no
duplicate key. Its `command` is exactly one of
`{kind:"project-metadata",title:String,description:String}`,
`{kind:"rename-graph",graph_id:Id,name:String}`, or
`{kind:"graph",envelope:{command_id:Id,graph_id:Id,
expected_revision:u64 JSON number,command:GraphCommand}}`. The Core
GraphRevision is a transparent numeric u64 here, distinct from the PS1
authored/Graph coordinate `Decimal` string. Supported GraphCommand
forms are `batch:{commands:[GraphCommand]}`, `connect:{connection:Connection}`,
`disconnect:{connection_ids:[Id]}`, `set-node-position:{node_id:Id,x:i64,y:i64}`,
`set-connection-routing:{connection_id:Id,routing:null|GraphRoutingPoint}`,
`set-configuration:{node_id:Id,configuration:SchemaValue}`, and
`set-authored-state:{node_id:Id,authored_state:null|SchemaValue}`. Each form
is a JSON object with `kind` and exactly the listed fields. Nested
Connection, GraphRoutingPoint and SchemaValue use their existing reviewed Core
contract encodings and validators; unsupported `add-node`, unknown variants or
ambiguous extension fields refuse. This appendix fixes the envelope and
supported tag spellings, rather than treating a future Rust serializer change
as wire authorization.

`provenance` records the original accepted actor; retries never overwrite it.
Its exact v1 shape is `{principal:Principal,actor:{kind,actor_id:Id},
grantor:Principal,grant_ref:{kind,reference_id:Id},
effective_scope:{project_id:Id,actions:ActionMask},
policy_decision_sha256:Digest}`. `Principal` is the existing checked account or
local-controller union. `kind` in `actor` and `grant_ref` is a checked qualified
name; `ActionMask` uses the existing R2 checked u16 encoding. All references
are nonsecret historical IDs, not bearer grants. The package reader verifies
shape and scope equality, not current permission or signature authenticity;
only the shared trusted authority may construct an accepted receipt after a
live authorization decision. A source unable to supply exact grantor/decision
evidence is not silently assigned invented values; admission must wait for a
checked source or an explicitly reviewed new provenance version.

Acceptance-prefix digests use exactly these domain-separated canonical values:

```text
P0 = SHA256(canonical({domain:"photara.package.accepted-prefix.v1",
  project_id,library_id,bootstrap_sha256,through_ordinal:"0"}))
Pi = SHA256(canonical({domain:"photara.package.accepted-prefix-link.v1",
  previous_sha256:P(i-1),acceptance_ordinal:String(i),operation_id,
  request_sha256,receipt_sha256:<receipt ObjectRef digest>}))
```

Index `accepted` equals the recomputed final `(N,PN)`. A StateRoot's accepted
coordinate must equal its own selected OperationIndex's **final** coordinate;
the recovery
ordinal is no greater than active, with identical entries and receipts over
the shared prefix. Same operation ID and same canonical request digest returns
its original outcome without mutation; a different digest refuses. Accepted
portable operation receipts do not prove HEAD publication. A **separate**
post-barrier Saved receipt is device-local recovery evidence binding exact HEAD,
included prefix and qualification; it can label the session `Saved` only while
its authored revision/digest equal the current accepted state. This separation
avoids a receipt→containing-commit digest cycle.

The journal-inclusion prefix commits a finite sequence of accepted journal
frames, not a bare sequence number. A v1 accepted frame is canonical JSON with
exact `{schema:{id:"photara.package.accepted-journal-frame",version:1},
journal_id:Id,sequence:Decimal,operation_id:Id,request_sha256:Digest,
receipt_sha256:Digest}`. `sequence` is the actual monotonic PS0 journal
sequence. Accepted frames form an ordered projection and may have gaps because
checkpoint, barrier or other journal records occupy intervening sequences.
The first actual sequence need not be 1. `J0 = SHA256(canonical({domain:"photara.package.journal-prefix.v1",
journal_id,through_sequence:"0"}))`; `Ji = SHA256(canonical(
{domain:"photara.package.journal-prefix-link.v1",previous_sha256:J(i-1),
frame_sha256:SHA256(exact canonical frame bytes)}))`. The included `Ji`,
sequence and journal ID equal the StateRoot `journal_inclusion`; each included
frame must resolve to the matching accepted operation receipt, and the final
frame's **referenced receipt.after** revision/digest must equal the root's authored
coordinate. Prepared, failed and pending frames are not accepted frames and
cannot enter this prefix. Device-local recovery may retain them separately.

## Opt-in original conversion source

`photara.package.conversion-source` v1 has required `schema`, `project_id`,
`conversion_id:Id`, `source_format_version:{major,minor}` (exact original
bootstrap coordinate), `source_bootstrap_sha256:Digest`,
`source_head_sha256:Digest`, `source_commit_id:Id`,
`snapshot_directory:["conversion-sources",<conversion_id>,"package"]`,
`files`, and `extensions`. `files` is path-sorted/unique, each entry
`{components:[validated portable component],sha256:Digest,
byte_length:Decimal}`. It includes exact original `HEAD.json` and
`manifest.json`, all original commit/objects and **all** unknown optional
regular-file bytes. The snapshot namespace is reserved and within the package;
no external managed medium or user-owned source is copied. Original bytes are
never canonicalized on copy. Exact enumeration, length/hash and the actual
legacy reader verify the independently reopenable snapshot before HEAD changes.

Snapshot enumeration happens under the cooperative lease before writing the
snapshot. Only the exact registered stable lock path, this attempt's registered
staging files, and this attempt's own snapshot namespace are excluded; unknown
content under a similarly named directory is preserved or explicitly refused,
never broadly skipped. The original intent fixes conversion ID, source HEAD,
bootstrap, file manifest and attempted paths. An exact matching partial
snapshot resumes under that ID; unknown/conflicting occupancy refuses. Original
source remains pinned beyond active/recovery turnover, with no automatic
release or rollback by changing HEAD to an older value.

## Additive managed backing records and portable binding

`photara.resource-backings.v1` selects new immutable package metadata records.
Every record below contains required `schema`, `project_id`, and checked
`extensions`. `photara.resource.state` v1 binds `library_id` and ordered, unique selections
of `identities`, `working_bindings`, `versions`, `backings`, and `obligations`.
Their exact entry shapes are `{resource_id:Id,identity:JRef}`,
`{binding_id:Id,binding:JRef}`, `{version_id:Id,version:JRef}`,
`{backing_id:Id,backing:JRef}` and `{obligation_id:Id,obligation:JRef}`.
Each array is sorted/unique by its named ID; target schema and target ID must
match. At most one *current working binding* is selected per ResourceId; a
ResourceId may have several immutable selected captured versions. Every
selected backing/obligation targets a selected version of the same ResourceId.
Capture evidence and backing publication evidence are JRef edges; source
coordinates and Host Bindings are not.

| Schema ID (v1) | Required fields beyond common fields |
| --- | --- |
| `photara.resource.identity` | `resource_id:Id`, `purpose:QualifiedName`, `custody:"photara-managed"\|"user-source"`, `producer:null\|{kind:QualifiedName,producer_id:Id}` |
| `photara.resource.working-binding` | `resource_id:Id`, `binding_id:Id`, `revision:Decimal`, `location:null\|{library_id:Id,storage_location_id:Id,coordinate:ExternalCoordinate}` |
| `photara.resource.captured-version` | `resource_id:Id`, `version_id:Id`, `media_type:MediaType`, `byte_length:Decimal`, `sha256:Digest`, `captured_at:Timestamp`, `capture_operation_id:Id`, `capture_evidence:JRef` |
| `photara.resource.backing` | `resource_id:Id`, `version_id:Id`, `backing_id:Id`, `revision:Decimal`, `sha256:Digest`, `byte_length:Decimal`, `location:{library_id:Id,storage_location_id:Id,coordinate:ExternalCoordinate}`, `publication_evidence:JRef` |
| `photara.resource.retention-obligation` | `obligation_id:Id`, `revision:Decimal`, `resource_id:Id`, `version_id:Id`, `origin:{kind:"authored"\|"history"\|"recovery"\|"explicit"\|"pending",source_id:Id}`, `minimum_qualified_copies:Decimal`, `required_backing_ids:[Id]` sorted/unique, `required_qualification:{profile_id:Id,minimum_profile_revision:Decimal,failure_model_sha256:Digest}` |
| `photara.resource.publication-evidence` | `evidence_id:Id`, `operation_id:Id`, `request_sha256:Digest`, `resource_id:Id`, `version_id:Id`, `sha256:Digest`, `byte_length:Decimal`, `established_at:Timestamp`, `evidence_kind:"capture"\|"retained-publication"`, `destination:null\|{backing_id:Id,backing_revision:Decimal,location:{library_id:Id,storage_location_id:Id,coordinate:ExternalCoordinate}}`, `qualification:null\|{profile_id:Id,profile_revision:Decimal,failure_model_sha256:Digest}` |

The `working-binding` record is **portable resource state only**. Its
`location` uses a logical StorageLocationId and the existing validated
`ExternalCoordinate` relative-filesystem/provider-object union. It contains
**no** device HostBindingId, absolute native path, inode/file-size/mtime
observation, watcher generation, security-scoped bookmark, credential, mount
state, current availability or access grant. Provider coordinates must be
nonsecret identities, never presigned URLs or tokens. All such observations and
authorizations remain device/runtime state, not root-selected package facts.

CapturedVersion has no current-backing or retention-policy pointer. The
root-selected ResourceState independently selects current backing revisions and
obligations so verified relocation or released pins do not change immutable
version identity. A matching digest/length alone is not a publication receipt;
`capture` evidence proves exact capture only, while a non-null qualified
`retained-publication` evidence record is required to claim a durably retained
backing. Capture evidence has null destination and qualification. Retained
publication requires both non-null, and its destination must equal the
selecting BackingId, revision and logical location; this is no Host Binding.
An obligation requires at least one qualified copy, and every required
BackingId must identify a selected backing of its version. Counting qualified
copies and any required failure-domain separation follow the explicit reviewed
profile; this appendix creates no universal replica or failure-domain default.
Duplicate backing records alone cannot satisfy a profile's independent-replica
requirement. The evidence profile
ID and failure-model digest must equal the obligation requirement, and its
revision must meet the minimum. An authored/history/recovery pin source must
resolve to the corresponding selected package state; explicit/pending pin
sources require the corresponding portable operation or policy evidence. A
missing pin source or unqualified copy cannot silently satisfy retention.
This appendix selects **no** actual qualification profile or fault
model. An offline qualified backing may remain an established obligation with
unknown current availability; confirmed loss is a separate fact. The reader
validates metadata and package closure without opening or strongly hashing
large external media during ordinary open, autosave, validation or root turnover.

`photara.project.representation-content` v3 retains every v2 field/meaning and
adds `binding:{kind:"managed-captured",resource_id:Id,version_id:Id,
version:JRef}`. That JRef targets the captured-version record and must match
selected ResourceState IDs, media type/length and fingerprint. Existing v2
`managed` and `external` remain unchanged. The new binding requires
`photara.resource-backings.v1`. Existing context `ResourceValue`, variable,
expression and snapshot encodings reject a captured-managed value until their
own separate additive typed contract exists; no old value silently gains a
different meaning.

## Golden vectors and review limits

The [golden JSON vectors](proposals/ps2/sealed-wire-golden.json) record both a
deliberately non-key-sorted input value and its exact expected compact canonical
UTF-8 string and SHA-256. The fixture-only test compares those bytes against
`photara_core::canonical_json`; it does **not** dispatch, validate, publish or
open a package. Nineteen vectors cover unchanged HEAD/outer envelopes and
complete field shapes for RootSet,
StateRoot, empty operation index, conversion source, ResourceState and each
managed record, v3 representation, accepted receipt, semantic intent, journal
frame and acceptance-prefix commitments. Reference digests and qualification
IDs are syntactically illustrative; no vector claims a complete valid package
closure or an approved storage profile. The byte vectors become stable only when this appendix
is reviewed; changing them afterward needs an explicit format review.

Remaining implementation gates: real typed parser/closure tests, exact
conversion/publication process and fault furnace, macOS storage qualification,
shared Rust session/autosave, and only then real project browsing/switching.
No production writer, live migration/conversion, deletion/GC, Asset Store,
threshold or GUI/CLI/agent routing is approved by this appendix.
