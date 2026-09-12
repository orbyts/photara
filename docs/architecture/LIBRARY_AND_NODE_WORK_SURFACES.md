# D19 — Libraries, explicit dataflow and node Work Surfaces

Status: **conceptual architecture and exact R1–R8 accepted; CXT2 inert DDL complete**, 2026-09-12. The user explicitly approved this model
and requested the documentation amendment. D19 supersedes conflicting domain,
asset-access, presentation and distribution statements in D18 and the S1–S7
baseline. The physical design is now accepted through R1–R8; execution, runtime, UI and deployment remain separately gated.
L3 remains paused. [Execution order](../ROADMAP_0_2_EXECUTION.md) and
[active handoff](../ACTIVE_HANDOFF.md) carry the next gate.

## Accepted exact review outcome

[Accepted D19 contract freeze](D19_CONTRACT_FREEZE.md) resolves the previously
open identities, access matrix, offline truth, resources/slots, variables,
AssetSet v2 and NodeSDK axes. [Static schema delta](D19_STATIC_SCHEMA_DELTA.md)
specifies package/SQL/sync/DTO/fixture changes and compatibility. **R1–R8 were accepted as proposed by Suhail on 2026-09-12.** Later “freeze next” wording records the amendment's original
sequence; the next eligible action is separately selected CXT1a pure Rust.
No applied migration, source, fixture byte or runtime behavior changed.

## Library and Project boundaries

**Library** replaces the persistent domain term **Workspace**. Like a Notion
workspace, it is a durable, user-named catalog and collaboration boundary for an
individual or team. A user can have multiple Libraries. First installation
automatically creates and opens a local **My Library**, without requiring an
Account, sign-in or cloud choice. Renaming it preserves its stable LibraryId.
Local, Photara Cloud and a later CloudKit adapter use the same domain and IDs;
claiming a local Library and joining another remain distinct explicit operations.
An Account is authentication identity, never a Person.

A Library owns durable People, Organizations, SocialProfiles, relationships,
LocationKinds, Locations, policies, variables, memberships and the Project
Catalog. Every Project belongs to **exactly one Library**. Project packages own
authored Project state, bounded Library snapshots/assignments, Graphs and durable
execution evidence; catalog entries are projections. Importing an older package
with nullable origin requires an explicit destination-Library association plan,
preserving original provenance. A foreign historical snapshot does not create a
second owning Library or permission to query its source. Moving package bytes
does not transfer Library ownership; transfer requires a separately reviewed
identity, access, snapshot and synchronization operation.

Library membership is not an automatic grant to every Project. Logical
**ProjectAccessGrant** records and Project invitations authorize named Accounts
for bounded Project actions. A Project declares a **restricted** or
**library-visible** policy: restricted access requires an explicit Project grant;
library-visible access uses the Project's explicitly chosen member-role/action
policy, not an unconditional membership bypass. Discovery, reading, editing,
running and inviting are distinct action decisions; exact roles, defaults,
invitation lifecycle, grant precedence and revocation rules must be frozen before
physical implementation. Project creators receive explicit governing access;
Library administration alone does not imply unrestricted Project content access.

A project-only collaborator can access the granted Project and its assigned,
bounded snapshots, not the Library's whole catalog, People directory or change
feed. Search, counts, previews, media fetches, receipts and reset snapshots must
respect that scope; hiding rows in a UI is insufficient. Project policy and grants
are Library/service authorization state, not a claim that the package contains
executable permissions. Possessing a package conveys its included bytes, not
current Library or service authority.

Package/SMB access is separate device binding authorization. A Project grant does
not mount storage or mint bookmarks, and a filesystem grant does not create
Library membership. Current authorization is checked for each new protected
operation. Previously shared package/snapshot bytes cannot be remotely recalled
merely by revoking membership. Exact offline retention and revocation behavior
must be made truthful in the contract and UI.

Library-owned logical Storage Locations and device-owned Host Bindings are
separate authorities. Portable Project and asset references name stable Storage
Location IDs plus normalized relative resources or provider object IDs; absolute
paths, mounts, bookmarks and credentials remain host state. The canonical
resolution, storage-class, variable and rebind rules are in
[Storage locations and host bindings](STORAGE_LOCATIONS_AND_HOST_BINDINGS.md).

## Presentation vocabulary and ownership

| Term | Meaning |
| --- | --- |
| Account | Authenticated user/service identity and personal preferences; never a Person record |
| Library | Durable catalog and collaboration domain |
| Project | One Library-owned body of work containing assignments, package state, Graphs and evidence |
| Graph | Project-owned executable authored workflow; owns its Nodes, connections, Graph Canvas and Inspector surface |
| Node | Graph-owned instance pinned to one exact node definition; supplies a required Inspector contract and may supply a Work Surface |
| Inspector | Graph-owned Panel/surface that renders the selected Node's required Inspector contract, state and diagnostics |
| Work Surface | Node-provided authoring or inspection UI |
| Canvas | Spatial editor within a Work Surface, or the Graph's spatial editor |
| Window Layout / Layout Preset | Native arrangement and saved arrangement of UI regions |
| Panel | Reusable dockable UI region with identity independent of placement |
| Browser | Shared list/grid/card collection presentation primitive |
| Gallery component | Asset-specific Browser/Gallery presentation reused in multiple Work Surfaces |
| Gallery node | Built-in inspection node with explicit AssetSet ports and a Gallery Work Surface |

Retire ambiguous product UI **Workspace** labels. A native window is bound to one
Library; a Project window is additionally bound to one Project. Display both
Library and Project scope. A project-only collaborator sees that bounded scope
without acquiring catalog navigation. Different windows may target different
Libraries. Window Layout, docking, selection, filters, zoom and restoration are
client preferences; semantic authored edits cross revision-checked commands.
Existing `WorkspaceModel`, `NSWorkspace`, manifest field names, module paths and
compiled labels are implementation/compatibility identifiers until a separate
reviewed change. This amendment performs no physical source rename or UI work.

Library management and its People/Organization/Location/Project Browsers are
owned by the application and Library, not by nodes. Authoring inside a node must
not require leaving its Work Surface to assign a Person or Location. A Work
Surface may embed standard **host-owned** People/Location pickers and authorized
Browser/Gallery components through declared presentation and capability contracts.
Inline creation invokes an authorized Library command and returns a typed
reference; it is not node-private data or arbitrary database access. A Project
collaborator lacking Library permission can select permitted saved assignments
and sees an unavailable/denied state for wider browsing or creation.

The **Graph owns the Inspector surface**, its placement, selection binding, empty
state and lifecycle. Every Node definition supplies a required Inspector contract
covering its identity, typed ports, parameters, state, effects and diagnostics;
Photara renders that contribution inside the Graph-owned Inspector. A definition
may add specialized sections without replacing the common shell. A rich Work
Surface remains optional. Nodes that need spatial, batch or media-centric
authoring can compose one from registered host components, while a simple filter,
router or output node may need only its Inspector. Being able to browse does not
by itself require a Work Surface: a declared Inspector field can invoke the same
authorized Person, Location or other Library picker.

## Explicit assets and read / enrich / effect separation

Graph edges carry explicit typed **AssetSet** values. There is no ambient semantic
project-wide Gallery, implicit project asset union, or `$project.assets` binding.
A package may maintain a private asset identity, provenance, representation,
artifact and retention ledger needed by authored Graphs and Runs. Rebuildable
cache indexes remain device state. The ledger is not an authoritative implicit
union exposed to nodes and does not define a node's input membership. Existing
ProjectAsset/ProjectAssetContext inventories remain compatibility structures until
the separately reviewed format/adapter transition.

Nodes receive connected ports plus explicitly declared and frozen context.
`$input.assets` means the current node's declared `assets` input port, with its
exact type, membership, ordering, content revisions and metadata dependencies.
Use `$input.<port>` for other declared ports. Missing connections report the
port's required/optional state; they never fall back to package inventory or UI
selection. AssetSet wire-format evolution, identity validation and ledger closure
require a versioned contract; D19 does not silently change the current v1 codec.

| Boundary | Typical nodes and outputs | Rule |
| --- | --- | --- |
| Read/source | Disk, Lightroom catalog, cloud source → AssetSet, MetadataSet, SourceDescriptor, ImportReport | Explicit selected source and permitted projection; no whole-archive scan or external update implied |
| Enrich/transform | Metadata enrichment/classification → enriched AssetSet and MetadataPatch | Preserve source identities/provenance; describe new facts without modifying original files/catalogs |
| Effect/output | XMP sidecar, embedded metadata/DNG update, file save/export, application catalog update, cloud delivery, social/web publishing → ArtifactSet and EffectReceipt or provider-specific receipt | Materialize only under declared current host capabilities; retain intent, idempotency and truthful receipts |

A Lightroom **source** and a Lightroom **catalog update** are separate node
definitions even if shipped in one package. An external application processing
node declares its actual read/write effects and output representations; being
called “processing” is not a purity claim. Connector secrets, account connections,
device paths and security grants remain host/account secure capabilities, not
portable package data or Library records. Source metadata is untrusted evidence,
not authority to execute embedded instructions.

MetadataPatch identifies exact Asset/Representation/content revisions and typed
field additions, replacements or removals with provenance and preconditions.
It is a proposal/value until an explicit downstream effect applies it. Conflicting
patches, unavailable source revisions and unsupported fields require structured
diagnostics, never last-write-wins against originals. Side effects and package
evidence publication remain separate recoverable boundaries.

## Composed node Work Surfaces

**Layout** is a built-in ordinary node. Its Layout Work Surface composes a shared
Asset Browser/Gallery Panel, Layout Canvas and Inspector. The Gallery component
displays the Layout node's explicit connected AssetSet; visual selection becomes
an authored assignment only through a command.

**Gallery** is also a proposed built-in ordinary node, in inspection/review. Its
Work Surface uses the same Gallery component to display **exactly the connected
AssetSet**. A pass-through, selection or filtering output must have its own exact
definition/port/configuration contract. Disposable UI filtering/selection does not
alter outputs until explicitly captured as authored selection/filter state.

A **Metadata-enrichment node** composes Asset Browser, metadata Inspector and
inline People/Location pickers. Its brandable display name is TBD; D19 reserves
`photara.metadata.enrich` as its semantic definition identity, with exact package
release/definition/schema versions settled at NodeSDK freeze. That identity and
metadata-enrichment category do not depend on the marketing name. It emits
enriched AssetSet and MetadataPatch, retaining
original observations and user assertions separately. No mandatory reference node
or navigation away from this Work Surface is needed to capture authorized refs.

Its assignment target is explicit and authored: one asset, the current selected
subset, the full connected AssetSet, or an upstream named grouping/album value.
Disposable Gallery selection never silently becomes a batch target. The same
Work Surface can browse the connected AssetSet and, under separately declared
Library capabilities, search permitted People, Organizations, Locations and
LocationKinds. It can therefore assign a Person or Location at asset or group
granularity without leaving the node.

**EXIF is one metadata family, not the generic node identity.** The semantic
contract must accommodate namespaced EXIF, IPTC and XMP fields plus Photara-typed
Person/Location assertions. A later brandable display name may be “Exif” only if
the exact definition is deliberately limited to that standard. Enrichment stays
non-destructive. Separately declared effect definitions choose how to materialize
the patch: write an XMP sidecar, update supported embedded metadata (including a
supported DNG policy), update an application catalog, or produce another artifact.
A combined convenience node or package may present those choices, but its manifest,
graph contract and Run evidence must still expose the effect mode, permissions,
preconditions and receipt rather than hiding mutation inside pure enrichment.

Layout and Gallery share the same NodeSDK contract as downloaded, community,
private and development packages. Built-in status adds no evaluator branch, SQL
access or undeclared context. Layout/Gallery are the D19 proposed built-ins; other
bundling decisions remain separate, including the planned free first-party
Lightroom Classic, Lightroom Cloud/Desktop and Photoshop downloads.

## Stable discovery taxonomy

Each exact definition declares **one primary CategoryId** from a versioned
hierarchy, plus secondary provider, capability and search tags. The following
semantic IDs establish the D19 discovery vocabulary; exact manifest encoding,
versioning and compatibility mappings are frozen with NodeSDK before use.

| Primary category ID | Discovery purpose / examples |
| --- | --- |
| `photara.category.sources.filesystem` | Disk/local/NAS source |
| `photara.category.sources.application-catalog` | Lightroom, Photos, application catalog read |
| `photara.category.sources.cloud` | Cloud object/media source |
| `photara.category.selection` | Select, combine, match, order AssetSets |
| `photara.category.filtering` | Explicit predicate/query filtering of connected values |
| `photara.category.context.reference` | Person/Organization/Location reference values |
| `photara.category.context.query` | Authorized bounded Library query/context capture |
| `photara.category.metadata.enrichment` | Add/correct metadata and typed assignments |
| `photara.category.metadata.classification` | Classify or tag with typed provenance |
| `photara.category.layout.composition` | Layout and visual composition |
| `photara.category.processing.application` | Photoshop, Capture One and other application processing |
| `photara.category.processing.ai` | Learned/generative processing |
| `photara.category.inspection.review` | Gallery, review and diagnostics |
| `photara.category.output.metadata` | Materialize XMP/metadata sidecars |
| `photara.category.output.files` | Save/render/export files |
| `photara.category.output.application-update` | Update an external application catalog |
| `photara.category.delivery.cloud` | Cloud upload/delivery |
| `photara.category.publishing.social` | Social publishing |
| `photara.category.publishing.web` | Website publishing |
| `photara.category.automation.control` | Routing, scheduling and workflow control |

Parent paths organize discovery, not execution. A provider such as Adobe, an AI
capability, built-in provenance, price, localized label or brand name is not an
evaluator variant. Multi-purpose nodes choose their primary discovery placement
and advertise secondary tags; permissions and purity/effect declarations remain
independent validated contracts. Moving a discovery label cannot retarget saved
package/definition pins or silently change behavior.

## Typed value families and reference capture

| Value family | Semantic content |
| --- | --- |
| AssetSet | Explicit ordered duplicate-free membership and exact referenced asset/representation dependencies |
| MetadataSet | Typed observations keyed to source identity/content revision with provenance |
| MetadataPatch | Typed proposed metadata changes with exact targets and preconditions |
| PersonRef / PersonSet | Scoped typed Person IDs plus captured revisions/projections |
| OrganizationRef / OrganizationSet | Scoped typed Organization IDs plus captured revisions/projections |
| LocationRef / LocationSet | Scoped typed Location IDs with required kind and captured facts |
| ProjectContext / ShootContext | Explicit bounded Project assignments, schedule/participants/Location context |
| ArtifactSet | Exact output artifact identities, descriptors and availability/evidence references |
| SourceDescriptor | Portable source identity, selection/query and capability requirements; no secret or device path |
| ImportReport | Observed/imported/missing/rejected/ambiguous items and provenance; no fabricated complete scan |
| EffectReceipt / provider-specific receipt | Immutable observed outcome tied to intent/operation/attempt, including uncertainty |

These are value families, not approved new JSON schemas, SQL tables or all-to-all
port coercions. Exact descriptors, cardinality, schema versions, membership order,
projection closure, unsupported cases and digest rules must be frozen. Typed refs
are scoped identities, not names, raw rows or bearer capabilities.

Reference/query nodes are optional when Library facts need to participate visibly
in dataflow. A host-owned picker may instead capture typed refs directly into node
authored configuration. In both cases runtime captures exact Library/record IDs,
revisions, selected fields and query membership/negative facts. A reference can
resolve only under declared permission; runtime cannot follow arbitrary related
records or expand the projection by knowing an ID.

## Authoring visibility and runtime access

A user can browse the authorized Library while authoring; that visibility is
not inherited by a graph runtime. The host resolves declared context before a
Run and freezes the permitted projection. Nodes receive only connected ports
and their declared captured capability data. Inline Library edits commit through
Library commands; capturing/applying their refs into a Project is a separate
revision-checked Project command with explicit partial-failure behavior.

Use `$library.delivery_policy` for Library variables and `$input.assets` for
connected assets. D18's lowercase qualified semantic scopes remain case-sensitive.
Closed uppercase HostPlace symbols `$HOME`, `$DOWNLOADS`, `$DESKTOP`, `$DOCUMENTS`,
`$PICTURES` and `$TEMP` retain native host resolution and declared grants; there is
no arbitrary process environment lookup. `$project.root` remains a logical
capability-backed resource root, not a string or general package-write grant.
Library storage slots such as conceptual `$library.storage.raw_archive` resolve
to typed logical handles whose physical bindings are selected and authorized by
the current host. Node and Project variables never serialize absolute host paths
or use variables to hide AssetSet dataflow.

Cache keys cover exact input membership/order, captured revisions/projections,
negative facts, metadata/representation fingerprints and relevant implementation
and context versions. Unrelated Library browsing or edits do not invalidate an
undeclared dependency. Offline evaluation uses permitted captures or reports
unavailable; it never silently queries wider Library state. Sensitive captures
require the approved projection/portability policy. Revocation blocks new protected
reads, refreshes and effects without falsifying history or promising erasure of
already shared bytes. Sharing/cache reuse must not cross authorization boundaries
merely because digests match.

## Compatibility, physical review and exact next gates

S2–S6 and S7 D1–D17 remain the historical approved baseline. Existing physical
`Workspace`/`workspace_id` names, ProjectAsset inventory contracts, package
examples, endpoint paths and fixture bytes are **conceptually superseded where
they conflict with D19**, pending a separately reviewed additive/rename/migration
plan. This is not a global text rename or approval to reinterpret stored bytes.
S3 remains **44 tables / 179 statements**; S4 remains **35 tables / 275 statements**.
All six applied L2 migration files and checksums remain unchanged. S6 fixtures
and the inert D18 addendum are preserved as baseline evidence, not D19 conformance.

The exact next gates, in order, are:

1. **D19 documentation consistency review:** ratify cross-document ownership,
   Project access, input semantics, vocabulary and taxonomy; record open details.
2. **Logical/package/NodeSDK contract freeze:** specify LibraryId association,
   ProjectAccessGrant/invitations/policies, private ledger versus port membership,
   typed families/captures, Work Surface composition and compatible manifests.
   Revise D18 and the CXT1 scope around these contracts before implementation.
3. **CXT2 static exact schema delta review:** reconcile SQLite, PostgreSQL,
   package and synchronization/export formats; review concrete fields, migrations,
   API/RLS and project-filtered feed/snapshot/media/receipt authorization, rename
   mapping, required features, counts and fixtures. No DDL execution at this gate.
4. **Revised CXT1 then CXT3 implementation scopes:** separately authorize the
   pure Core/NodeSDK foundation, then disposable migrations/adapters/codec and
   authorization tests against the frozen contracts. Never rewrite migrations
   0001–0006. Test old L1/L2 compatibility and project-only access explicitly.
5. **Resume L3 only after those gates and CXT1–3 acceptance:** integrate package
   creation/publication with one owning Library and the settled context format;
   prove staged package/catalog recovery and effect-receipt publication boundaries.

This replaces the former instruction to begin CXT1 directly from D18. No live
database/service, runtime source, migration SQL, fixture baseline/hash, package
manifest or UI source is changed by D19. No commit, staging, push, new cloud
resource or deployment is part of this documentation task.

## Canonical terminology policy

The terms in [Presentation vocabulary and ownership](#presentation-vocabulary-and-ownership)
and [Library and Project boundaries](#library-and-project-boundaries) are the
canonical product, architecture, schema and future UI language. Earlier conversation,
historical documents and current code may still use Workspace, Scene, project-wide
Gallery or similar pre-D19 names. Treat those as migration aliases, not competing
concepts: persistent Workspace → Library; Scene → LocationKind where it means a
reusable place classification; node workspace → Work Surface; spatial editor →
Canvas; saved pane arrangement → Window Layout/Layout Preset. New contracts and
UI copy use the canonical term unless a compatibility surface must show an older
identifier explicitly.
