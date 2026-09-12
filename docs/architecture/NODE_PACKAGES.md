# Node packages

## Current exact review packet — 2026-09-12

The [exact NodeSDK axes](D19_CONTRACT_FREEZE.md#nodesdk-exact-contract-axes) and [manifest/DTO delta](D19_STATIC_SCHEMA_DELTA.md#dto-and-manifest-delta) now specify manifest v2, mandatory Inspector, independent execution/capability/determinism/cache contracts, optional Work Surface, portability and explicit migrations/fallback. V1 pins/manifests remain unchanged. R6 and the wider R1–R8 packet were accepted as proposed 2026-09-12. [CXT1a](CXT1A_CONTRACTS.md) implements structural and registered-schema validation through `photara_node_sdk::v2`, with exact port connection checks and byte-preserving fallback. No installed manifest, node runtime or migration executor is changed. [CXT1b](CXT1B_CONTEXT_CONTRACTS.md) adds the pure Core field/type/context entry point using those explicit field modes; NodeSDK v1/v2 source and validation remain byte-identical.

## D19 package and Work Surface contract — conceptual amendment

[D19](LIBRARY_AND_NODE_WORK_SURFACES.md) governs the next NodeSDK freeze. A node
receives explicit connected AssetSet/other typed ports plus declared frozen context;
no project inventory union or `$project.assets`. Source/read, enrichment and effects
have separate typed contracts: sources emit AssetSet/MetadataSet/SourceDescriptor/
ImportReport; enrichment emits AssetSet/MetadataPatch; XMP/files/application update/
cloud delivery/social or web publishing emit ArtifactSet and typed EffectReceipts.
Lightroom source and catalog update are distinct definitions. Provider/account
secrets and machine paths remain host capabilities, never Library/package records.
The Graph owns the Inspector surface and selection lifecycle. Every definition
must supply the typed Inspector contract Photara renders there; specialized
sections are additive and a rich Work Surface is optional. Inspector fields and
Work Surfaces may invoke the same authorized
host-owned Library pickers without giving package code database access.

Use one primary hierarchical CategoryId from
[D19's taxonomy](LIBRARY_AND_NODE_WORK_SURFACES.md#stable-discovery-taxonomy), plus
provider/capability/search tags. Category paths and brandable display names are
discovery metadata only. Person/Organization/Location Ref/Set and ProjectContext/
ShootContext are typed families; exact descriptors/versioning remain freeze work.
Reference/query nodes are optional: host pickers can capture authorized typed refs
into authored configuration, with exact IDs/revisions/projections frozen at Run.

Layout and Gallery are proposed built-ins. Layout's Work Surface composes shared
Asset Browser/Gallery Panel, Layout Canvas and Inspector; Gallery's uses that same
component for exactly its connected AssetSet. A Metadata-enrichment Work Surface
may compose an Asset Browser, metadata Inspector and inline People/Location
pickers, targeting one asset, an explicit subset, the full connected set or a
typed upstream grouping. EXIF, IPTC and XMP are metadata namespaces rather than
one universal “Exif” identity. Enrichment emits a patch; XMP sidecar, supported
embedded/DNG update and application-catalog update are declared effects with
permissions and receipts. The application
owns Library management Browsers and embedded pickers; declared host permissions
and authorized Library commands govern browsing and inline creation. None grant
nodes SQL or database credentials. Work Surface/Canvas/Panel/Browser/Window Layout
replace ambiguous presentation Workspace terms; existing source fields await a
separately reviewed compatibility change. Authoring visibility is never runtime
permission. D18/CXT1 must be revised and exact contracts frozen before code.

## D18 context declaration amendment — revised under D19, pending

[D18](TYPED_CONTEXT_AND_EXPRESSIONS.md) requires a versioned exact-definition
contract for context/variable/metadata read selectors, resource rights and typed
write proposals. Bound slots resolve exact IDs and transitive dependencies;
declarations are not host/Library grants. Uppercase HostPlaces require exact
place/subtree/rights declarations; lowercase scopes and user snake_case members
remain explicitly qualified. Neither arbitrary process environment lookup nor
node-controlled grant prompts are allowed. Existing broad capability IDs and
optional manifest metadata do not implement these permissions. Required D18
contracts must fail closed in old readers, not hide in ignored extensions.

Node field schemas opt into literal-only, expression or template mode. Single
backticks delimit inline Photara expressions; future tagged triple-fences contain
bounded Photara expressions/templates only, never arbitrary scripts. Ordinary
text/backticks remain literal. Exact source/compiler and typed AST versions stay
separate. Pure nodes emit outputs/proposals; an optional Set Variable built-in
uses ordinary post-success application/CAS/receipt policy, not special evaluator
mutation privileges. No new node, manifest API or runtime is implemented here.

A package has a canonical namespaced ID, independent package version, manifest,
one or more versioned definitions, schemas, typed ports, capabilities,
determinism/effect policy, diagnostics, presentation metadata, state migrations,
and implementation fingerprint.

Package release version, definition version, value-type version, and persisted
schema/state version are distinct coordinates. A persisted node instance pins
the exact package release, definition identity, and definition version it uses;
updating a registry entry never silently retargets that instance.

Portable projects and standalone graph exports list exact package release
requirements while each node instance retains its exact definition identity and
version. Opening or resaving a document does not require those packages to be
installed; opaque schema-tagged node state remains preserved for later
resolution.

A definition is code and metadata. A node instance is user-owned graph state
that pins a definition version. Installing a newer package never silently
changes an existing instance.

Each exact definition also owns its independent presentation brand: display
identity, package-owned icon resource identifier, catalog category path and
search terms, required Inspector contract, optional specialized Inspector
sections, and optional rich Work Surface contribution. The Graph owns the
Inspector surface while Photara renders the selected definition's contract.
These fields are versioned definition metadata, not Swift-side
conditionals and not Core evaluator variants. Resource identifiers are neutral
package resource names rather than SF Symbols, Windows resources, or Linux
theme names; each native client resolves them into its own platform skin.
Changing a definition's brand follows ordinary package/definition versioning,
so an existing node instance remains pinned to the presentation metadata of the
exact definition it already references.

The Stage 4A `NodePackageRegistry` accepts the same validated manifest returned
by a live bundled package or reopened from persistence. It builds exact Core
definition coordinates from the package release plus each definition identity
and version. Resolution never falls forward to another release or definition
version. Manifests require schema version 1, a nonempty display name, at least
one valid definition, unique definition coordinates, and definitions inside the
package namespace. Unknown top-level manifest fields survive JSON round trips.

Built-in and downloadable nodes use the same semantic package contract. A
built-in may be trusted and bundled by the installer, but that does not give it
special graph semantics or direct database access.

Every store-visible definition requires a stable category identifier and search
metadata in its NodeSDK manifest. Categories support discovery; they do not alter
evaluation, ports, permissions, or saved graph identity. Localized category names,
keywords, pricing, rankings, and merchandising remain store projections.

Asset-provider nodes are ordinary packages as well. Disk, Dropbox, Google
Drive, Box, iCloud/File Provider, Photos/PhotoKit, Lightroom Cloud, and a studio
DAM can each own independent branding, authentication requirements, Inspector
controls, and runtime adapters while producing the same semantic `AssetSet`.
Credentials and provider handles are scoped host capabilities; consumers never
branch on the upstream provider.

Future database-backed node state follows the same rule. A node may receive a
logically private, versioned namespace and narrow state capability from the
host, but never an ambient database connection, arbitrary SQL, or authority
over another node's namespace. One Core-owned state service remains the
transaction, migration, backup, and recovery boundary.

```text
Core package/definition registry
├── photara.layout                 built-in Layout
├── Gallery definition             proposed built-in; exact package pin TBD
├── future first-party packages   bundled or installed later
└── future community/private packages
```

Bundled, official, community, local-development, and private/studio delivery
are distribution policies over the same identities and registry contract, not
different Core node types. The `0.2.0` critical path implements bundled
registration and sensible missing-package behavior, not the remote store or
complete install/update/uninstall lifecycle.

Persisted manifest records describe registrations only. They do not claim that
package code was downloaded, trusted, enabled, migrated, or executable; those
are later lifecycle concerns.

The first package is:

```text
package:    photara.layout
definition: photara.layout.compose
input:      photara.asset-set at value-type version 1
output:     photara.layout-plan at value-type version 1
```

D19 proposes Layout and Gallery as built-ins through the ordinary NodeSDK contract.
Lightroom Classic (`LrC`), Lightroom Desktop/Cloud (`Lr`), and Photoshop (`Ps`)
are planned free first-party downloads from the future Node Store. Nodes from
Photara or other publishers may later be free or paid. Existing Disk, AssetSet,
and test registrations validate ordinary package contracts and do not establish
their final distribution policy.

Marketplace loading is deferred until signing, publisher identity, permission
consent, credential isolation, runtime/process isolation, dependency resolution,
revocation, update, rollback, and recovery are designed and tested.
