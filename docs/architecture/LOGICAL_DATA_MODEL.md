# Logical data model

## Current exact review packet — 2026-09-12

The exact identities, cardinalities, lifecycle, authority and access matrices are now specified in the [accepted D19 freeze](D19_CONTRACT_FREEZE.md). Storage slots are distinct identities; ResourceDescriptors are portable while materialized handles are not; variables cannot contain nested AssetSets or workflow-output families. The [static delta](D19_STATIC_SCHEMA_DELTA.md) defines the physical mapping. R1–R8 were accepted as proposed 2026-09-12; older pending details below are baseline history.

Status: S1 baseline accepted, 2026-09-11. This freezes semantic entities,
authority, cardinality and invariants before S2 package schemas, S3 SQLite DDL,
S4 PostgreSQL DDL and S5 synchronization. It is not DDL or implementation.
Storexa 0.2.0 supplies database mechanics; Photara owns this model.

## D19 current conceptual model

[D19](LIBRARY_AND_NODE_WORK_SURFACES.md) is the current conceptual authority:
Library is the durable ownership domain; each Project belongs to exactly
one Library, with explicit Project access independent of blanket membership.
The model below incorporates those semantics. Existing S2–S6 physical names and
bytes remain the approved historical baseline pending exact contract/schema
review; this document does not rename applied migrations or approve new DDL.

## D18 typed-context amendment — revised under D19, details pending

[D18](TYPED_CONTEXT_AND_EXPRESSIONS.md) adds explicit Library/Project/Graph/node
variable aggregates, immutable Run overrides/ContextSnapshots and typed immutable
asset metadata observations. Account remains preferences only. VariableDefinition
owns at most one current VariableValue under one CAS revision; typed VariableId,
VariableValueId and immutable ScopeRef preserve identity. Scoped machine names
remain reserved through tombstone; type-breaking edits require explicit replacement.
Package variables remain package authority; Library variables join typed Library
authority. ResourceRootHandle and AssetSet are not path strings. Library facts
resolve through typed references/queries and explicit captures, never a global bag.
Run captures and metadata evidence are immutable; updates/proposals require an
explicit command, expected revisions and one authority boundary. D18 defines
provenance/sensitivity/portability and opaque host-only secrets. These are proposed
logical additions, not D1–D17 approval or new physical tables; CXT2 freezes DDL.

## Common rules

- Use distinct typed UUIDs for Accounts, Libraries, Memberships, Library
  entities and relationships, Projects, assignments, assets, representations,
  resources, Graphs, Runs, Attempts, operations, evidence and receipts.
- Names, paths, slugs, hashes, emails and authentication subjects are never
  substitutes for semantic IDs.
- Every Library entity has exactly one immutable `LibraryId`. Every
  project-owned entity has exactly one `ProjectId`; references retain scope.
- Mutable entities carry schema version, revision, created time and updated time.
  Accepted edits compare an expected revision. Undo creates another revision.
- Distinguish in-memory edit token, committed package revision, Library record
  revision, Graph authored revision, server revision and change cursor.
- Instants are UTC. Human schedules may instead be a calendar date or zoned
  interval. Unknown imported dates remain unknown.
- Tombstones retain identity, deletion revision/time and sync metadata. Deletion
  never cascades into project snapshots or execution evidence.
- Package state is Project authority; catalogs are projections. Local SQLite is
  the local Library authority and outbox. Cloud adds accepted server history,
  not another unconstrained editor.

## Account and Library

| Entity | Cardinality and invariants | Authority |
| --- | --- | --- |
| Account | Can join many Libraries; authentication identity, never implicitly a Person | Photara service; local projection |
| AccountIdentity | One Account has one or more; unique `(issuer, subject)`; email changes do not change identity | Photara service |
| Library | User-named durable catalog/collaboration boundary; owns records, policies/variables and Project catalog; stable LibraryId survives local-to-cloud association | Local or service according to association state |
| Membership | At most one current row per Account/Library; governs Library actions, not automatic access to every Project | Photara service; local association policy |
| ProjectAccessGrant | Stable grant identity; one owning Library and Project, named Account, action/role scope, revision and active/revoked lifecycle; exact uniqueness/invitation rules pending contract freeze | Library/service authorization, not package permission |
| Project invitation | Explicit grant acceptance workflow; cannot infer identity from Person/email display facts | Library/service controller; exact token/lifecycle format pending |

A local-only Library does not fabricate an Account or Membership. Claiming a
local Library and joining an existing cloud Library are explicit, different
operations. An active cloud Library must retain at least one owner. Users may own
or join multiple Libraries. First install automatically creates/opens local
**My Library**. Each Project selects restricted or library-visible access policy;
only an explicit Project policy may grant defined member actions. Project-only
collaborators receive bounded assigned snapshots, not catalog/Library visibility.
Package/SMB authorization remains a separate device binding.

## People, Organizations and relationships

`Person` and `Organization` each belong to one Library. They have stable IDs,
display names, aliases, descriptions/labels and optional media/contact fields.
Display names need not be unique.

Person capabilities such as photographer, model or stylist are descriptive and
never confer application permissions. Client is a project or relationship role
held by a Person or Organization—not a duplicate record kind.

`PersonOrganizationRelationship` has its own ID and joins same-Library records.
It carries a typed relationship/role, labels and optional validity interval.
Multiple roles are allowed; duplicate simultaneous pair/role facts are rejected.

Proposed Library lifecycle is `active | tombstoned | merged`. Merge retains a
same-kind source ID redirect to a same-Library target. Redirect chains cannot
cycle. Existing project snapshots never mutate because of merge or rename.

## Social profiles and social identity coordinates

Requested S7 addition: `SocialProfile` is a typed Library aggregate with its own
SocialProfileId, revision/timestamps and `active | tombstoned` lifecycle. Exactly
one immutable same-Library Person or Organization owns each profile; owners
have zero to many profiles, including multiple accounts for one provider. Store
optional exact provider subject + namespace, mutable handle/display/public URL,
provider/account kind, verification/provenance, fetch/refresh facts and an optional
consented avatar media reference. Manual handle/URL works without OAuth.

Handle is not stable identity; a provider subject can be adopted once but never
silently replaced. Bound subject uniqueness is Library/provider/namespace/subject,
including tombstones: it cannot attach to two owners, but is not identity proof.
Auth0/Account identity is separate. Parent retirement/merge explicitly retires
profiles; bound subjects cannot be cloned onto the target without a separately
approved transfer policy. Unbound manual successors may use new IDs/provenance.
Tokens/provider connection secrets never enter this aggregate. Project creation
resolves a profile to its Person/Organization assignment and captures selected
display facts only. Provider avatars respect consent, rights, expiry and fallback;
Instagram lookup is an optional authorized adapter, never a universal requirement.

See [D16/D17 details](SOCIAL_PROFILES_AND_LIBRARY_EXPORT.md) for complete invariants,
privacy and the proposed portable Library export/import boundary. Export preserves
typed IDs/revisions/provenance and permitted durable media, but excludes Accounts,
credentials, machine bindings and sync/recovery runtime state. Packages are backed
up separately. Dry-run, side-by-side restore, explicit collisions and root rebind
are future non-gating capabilities, not permission to replace a user's database.

## Location Kind and Location

`LocationKind` is a reusable, Library-unique concept such as Beach, Home
Studio, Apartment or Commercial Studio. It has:

- stable `LocationKindId` and immutable Library owner;
- canonical display term, description and zero or more aliases;
- a normalized key for every canonical or alias term;
- the normalization-policy version that produced each key;
- lifecycle/revision metadata.

One normalized term in a Library belongs to at most one kind across canonical
names and aliases. Renaming retains the prior canonical term as an alias by
default. Merge transfers terms atomically or reports collisions. Retired terms
remain reserved unless a later explicit reassignment policy permits reuse.

The Rust normalization contract—not database collation—is authoritative. Its
initial policy must pin Unicode normalization, case folding and whitespace, plus
an explicit versioned vocabulary for approved inflections/aliases. Thus `beach`,
`Beach` and `beaches` resolve to one concept regardless of creation order. An
unrestricted stemmer cannot safely decide semantic equivalence; unknown custom
terms require user confirmation and explicit aliases.

`Location` is a concrete place such as Ocean Beach or Shane's Apartment. It:

- belongs to one Library;
- requires exactly one live `LocationKindId` when created;
- may have one parent Location in the same Library;
- has an acyclic hierarchy;
- may carry display name, aliases, description, address/geography and media.

A live Location Kind cannot be deleted while Locations require it. A Location
with live children requires explicit reparenting or cascading intent.

There is no separate target Scene entity. Current Scene records and the Scenes
lab are migration inputs; legacy values become Location Kinds only through an
explicit mapping that reports ambiguities.

## Project and Library snapshots

`Project` has one stable ID and exactly one owning LibraryId, authored metadata,
lifecycle, Library assignments, a private asset/provenance/artifact ledger and
zero or more named Graphs. A catalog entry cannot establish a second owner.
An empty Project is valid. Proposed lifecycle is `active | archived`; it is
independent of run success or failure.

`ProjectPartyAssignment` references exactly one Person or Organization and has
one or more project roles plus optional notes. Recommended uniqueness is one
active assignment per Project/subject, with roles represented as a set.

`LibraryReferenceSnapshot` captures source Library, typed record ID, source
revision, display name and a bounded typed historical snapshot. It is immutable.
Refreshing a snapshot is an explicit Project edit; source rename/delete/merge or
opening online never silently refreshes it.

`ProjectLocationAssignment` references exactly one concrete Location snapshot
and that Location's Location Kind snapshot. It may carry a calendar/zoned
schedule, notes and participants. Repeated visits to the same Location have
distinct assignment IDs. Participants reference project party assignments plus
assignment-specific roles.

Historical search answers “where was this project shot?” from the saved snapshot.
Searching by the current Library classification is a separate explicit mode.

## Explicit assets and private package ledger

Graph ports carry explicit AssetSet values. There is no ambient project-wide
Gallery, implicit asset union or `$project.assets`; a node reads `$input.assets`
(or another declared port) and declared frozen context. D19 separates source/read,
metadata enrichment and external effect/output; MetadataPatch never mutates
original files or catalogs by itself.

- Existing `ProjectAsset` is a physical/compatibility identity record unique on
  `(ProjectId, AssetId)`. The target private ledger retains graph/run dependencies
  and provenance, not an implicit authoritative input union available to nodes.
- An Asset may temporarily have zero representations so missing or incomplete
  imports remain representable.
- Each Representation belongs to one project Asset and has stable identity,
  extensible namespaced roles/capabilities and immutable content-revision
  descriptors with fingerprint algorithm/value and evidence strength.
- A mutable current reference may advance to a new content revision; Runs retain
  the captured descriptor they used.
- A binding is either a stable project Resource or a runtime-resolution handle.
  Resources use stable IDs and normalized project-relative paths. Device paths,
  credentials and bookmarks remain outside portable state.
- Content equality may suggest reuse but never merges identities or grants access.
- Removing current membership cannot destroy representations referenced by
  retained execution history. Garbage collection is separate and explicit.
- Representation lineage references source content revisions and operation or
  evidence IDs; self-origin and cycles are invalid.
- `AssetSet` remains an explicit ordered, duplicate-free project-scoped value.
  Gallery selection never becomes invisible membership.

## Project Catalog and device locations

Keep these records distinct:

1. `ProjectCatalogEntry`: unique `(LibraryId, ProjectId)` discovery identity,
   summary/search projections, observed package revision/digest and observation
   time. It is never Project authority.
2. `ProjectLocator`: stable ID, Project ID, optional logical `StorageRootId` and
   relative package location. Several candidates may exist; divergent packages
   sharing a Project ID produce ambiguity rather than newest-file selection.
3. `DeviceStorageBinding`: Device ID plus Storage Root or locator resolution to
   a host path, bookmark or provider grant. It is device authority.

The same Library Storage Location and device Host Binding separation applies to
external asset resources and output targets, not only Project package locators.
Portable references retain the stable logical ID plus normalized relative path
or provider object ID; device state resolves it to a currently authorized handle.
The Project package records only the resource subset reached by Graphs and Runs,
never the whole Library archive. See
[Storage locations and host bindings](STORAGE_LOCATIONS_AND_HOST_BINDINGS.md).

Logical Storage Roots have stable IDs and Library-scoped labels. A different
`/Volumes/...` mount path does not alter identity. Missing/unmounted is an
availability observation, never deletion. Removing a catalog entry does not
delete a package. Finder copy, intentional replica and duplicate-as-new/fork
require distinct policies in S2/S5.

## Graphs and nodes

Each Saved Graph belongs to one Project and has stable `GraphId`, nonempty name,
authored revision, nodes/connections and exact package pins. Proposed active Graph
names are unique within a Project under a pinned simple text-normalization policy.
Graph-to-Graph invocation and shared mutable state are out of S1.

A Node Instance belongs to one Graph and pins exact package release, definition
identity/version, typed configuration and optional authored state. Connections
must reference existing nodes/ports with compatible type and cardinality.

Each production catalog definition has one stable primary `CategoryId`, nonempty
search metadata, display identity, neutral icon/resource IDs, typed ports and
capabilities, a required Inspector contract and optional specialized Inspector
sections/Work Surface contribution IDs. The Graph owns the Inspector surface and
selection lifecycle; the selected Node definition supplies what Photara renders.
Category
hierarchy and localization are catalog metadata, never evaluator variants.
[D19's hierarchical taxonomy](LIBRARY_AND_NODE_WORK_SURFACES.md#stable-discovery-taxonomy)
adds provider/capability/search tags independently of branding and execution.
Layout and Gallery are proposed built-ins using ordinary NodeSDK contracts.
Library management Browsers are app-owned; node Work Surfaces embed authorized
host pickers/components and create records only through Library commands.

[D19's typed value families](LIBRARY_AND_NODE_WORK_SURFACES.md#typed-value-families-and-reference-capture)
include AssetSet, MetadataSet/Patch, Person/Organization/Location Ref/Set,
ProjectContext/ShootContext, ArtifactSet, SourceDescriptor, ImportReport and typed
EffectReceipts. Authoring can capture refs directly; query/reference nodes are
optional. Runs retain exact IDs/revisions/projections, not live Library authority.

MetadataPatch targets one exact Asset, an explicitly authored subset, the full
connected AssetSet or a typed upstream grouping/album value. UI selection alone
is not a semantic target. EXIF, IPTC, XMP and Photara Person/Location assertions
remain distinct namespaced schemas. Patch creation is enrichment; XMP sidecar,
supported embedded/DNG mutation and application-catalog update are separately
declared effects with preconditions, permissions and receipts. A package may
offer a combined user experience without collapsing those semantic boundaries.

Installed/trusted/enabled state is device/package-manager state. Pricing and
licensing are future Node Store state and do not change saved coordinates.

## Runs, attempts, effects and evidence

`GraphRun` is immutable execution history. Its start facts include Project,
Graph and Graph revision/digest or snapshot, explicit targets, relevant asset and
project-context revisions, exact implementations, request ID and start time.

`NodeAttempt` belongs to one Run and source Node Instance. Attempt ordinal is
unique within `(RunId, NodeInstanceId)`. Retry creates a new Attempt ID and
preserves earlier facts. Skips and cache reuse record truthful dispositions
rather than fabricated execution.

Proposed run/attempt statuses are `queued | running | succeeded | failed |
cancelled | interrupted`. Cancellation requested is not terminal cancellation.
A crash yields interrupted or unknown—not invented success/failure. Terminal
facts finalize at most once; later reconciliation appends evidence.

`EffectIntent` owns stable Operation ID, target, request digest and idempotency
key. A retry of the same intended side effect reuses that operation identity.
`Receipt` is immutable provider evidence tied to an intent/attempt. Uncertain
external outcomes remain unknown until reconciliation.

Evidence and artifact references are immutable typed records with provenance,
capture time, content descriptors and durable-object references. Necessary
evidence survives cache cleanup, graph deletion and package movement. Imported
legacy evidence may belong to a Project without a fabricated Graph Run.

## Current-generation compatibility and future legacy import

The current generation-two records require explicit compatibility mappings as
the schema evolves. The existing Neon database and v0.1.3 projects are not inputs
to the new schema and do not block it. A later optional legacy importer can use
the following reference mappings:

- owner strings to Library IDs;
- Client records to Person or Organization plus relationships;
- Scene records to Location Kinds or unresolved migration facts;
- untyped Locations to required kinds without inventing certainty;
- one embedded graph to one named Saved Graph;
- legacy global assets to project inventories while preserving provenance;
- workflow events to evidence only when their meaning is known—never fabricated Runs.

That future importer should preserve valid existing UUIDs when semantics survive,
preserve ambiguous input and report it rather than silently discarding or guessing.
None of these legacy mappings is an S1–S7 or release acceptance condition.

## Policy details allocated to S2–S5

1. Local Library claim versus join behavior and initial membership roles.
2. D19 permits bounded historical foreign-Library snapshots; exact capture,
   refresh and one-owning-Library association/transfer contracts need review.
3. Initial taxonomy languages, alias/inflection vocabulary, custom-kind flow and
   retired-term reservation.
4. Party relationship cardinality and schedule representation.
5. Cross-project explicit asset reuse and representation/evidence retention.
6. Catalog visibility, package replica and fork identity policies.
7. Active Graph-name uniqueness and Graph deletion behavior.
8. Evidence retention/redaction and interrupted-effect reconciliation.
9. Exact local edit versus server revision/conflict semantics, completed in S5.

D19 supersedes the earlier S1 ownership/access/asset assumptions where stated.
Freeze the revised logical/package/NodeSDK contracts and then review exact
physical/sync deltas before revised CXT1/CXT3 implementation or resuming L3.

## S1 acceptance checklist

- [x] Every entity has a typed ID, owner, lifecycle, cardinality and authority.
- [x] Account/Person, Location Kind/Location/use and catalog/package identity are distinct.
- [x] Beach case/plural order, alias collision, rename and merge invariants are defined.
- [x] Every valid concrete Location has exactly one kind; hierarchy/deletion is specified.
- [x] Project snapshots survive source edits, deletion, offline access and migration.
- [x] Asset/representation/resource IDs remain independent of names and byte equality.
- [x] Named Graph and Run/Attempt facts preserve existing node contracts.
- [x] Local edits, package saves, Graph revisions and sync cursors cannot be confused.
- [x] Immutable evidence survives retry, interruption, Graph deletion and cache cleanup.
- [x] Legacy import is explicitly separate and non-blocking.
- [x] Storexa owns mechanics only; no DDL or deployment is implied by S1.
