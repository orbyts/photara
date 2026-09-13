# Project assets and representations

## Current exact review packet — 2026-09-12

The [D19 AssetSet contract](D19_CONTRACT_FREEZE.md#assetset-metadata-and-variables) now defines immutable v2 snapshots, exact ordered member/content/metadata dependencies, bounded pages and explicit patch/group targets. The package ledger supplies closure, never implicit node membership. Portable external/managed resources and device handles have distinct IDs. [CXT1a](CXT1A_CONTRACTS.md) implements these pure snapshot/resource contracts in `photara_core::contracts`; package closure and compatibility conversion remain CXT3a. [CXT1b](CXT1B_CONTEXT_CONTRACTS.md) adds pure metadata values/common/distinct over supplied immutable selections and observation references, with explicit negative facts and no extraction. Existing AssetSet v1/v2 digests remain unchanged. Implemented v1 behavior below remains compatibility evidence.

## D19 explicit dataflow and private ledger — current conceptual authority

[D19](LIBRARY_AND_NODE_WORK_SURFACES.md) replaces the ambient Project asset context
as an input authority with explicit AssetSet graph ports. There is no semantic
project-wide Gallery, implicit union or `$project.assets`. A package may keep a
private asset identity/provenance/representation/artifact ledger required by Graphs
and Runs, and devices maintain disposable cache indexes. Ledger presence never
grants input membership or runtime access. Nodes receive connected ports and
declared frozen context only; the accepted wire/ledger contract is implemented only to the bounded [CXT1a extent](CXT1A_CONTRACTS.md).

Sources read explicit selections into AssetSet, MetadataSet, SourceDescriptor and
ImportReport. Metadata enrichment emits enriched AssetSet and MetadataPatch without
modifying original files/catalogs. A patch's authored target is one asset, an
explicit selected subset, the full connected set, or a typed upstream grouping;
transient Gallery selection is not an implicit target. EXIF, IPTC, XMP and
Photara-typed Person/Location assertions remain namespaced. XMP sidecar,
supported embedded metadata/DNG update, file output, application catalog updates,
cloud delivery and social/web publishing are explicit effects producing typed
ArtifactSet/EffectReceipt evidence. Lightroom source and update are separate nodes.
Provider secrets and device paths stay in host/account capabilities.

The shared Asset Browser/Gallery component appears inside Layout's Work Surface,
the built-in Gallery inspection node and metadata authoring Work Surfaces. Each
shows its explicit AssetSet; selection/filtering changes graph outputs only through
a versioned authored command. Library pickers may capture authorized typed refs
without a reference node, but every runtime dependency freezes exact IDs/revisions/
projections. Existing Stage 5/6 implementation descriptions below are compatibility
evidence, not permission to expose an implicit project union in the target model.

## D18 typed metadata proposal — revised under D19, pending

[D18](TYPED_CONTEXT_AND_EXPRESSIONS.md), as revised by D19, uses `$input.assets`
for the current node's explicit `assets` port, never a directory string or an
inventory fallback. Immutable metadata
observations bind AssetId/RepresentationId to an exact fingerprint/content revision,
namespaced schema/key, typed value, extractor/version and provenance. User
assertions remain distinguishable from extracted/provider facts. EXIF camera/lens
is not identity proof; GPS/serial/face fields require privacy policy. Discovery
caches must be explicitly captured before becoming durable Run dependencies.
Collection queries return per-asset missing/ambiguous/conflicting facts;
`metadata.common` never selects an arbitrary first camera/lens, and representation
selection is explicit. Package captures are authority, SQL search indexes only
commit/checksum-qualified projections. This adds no extractor or runtime today.

## Current implementation: ownership and identity

The project owns asset context. An `AssetId` identifies the creative item a user
recognizes; it is not a path, file, provider record, Gallery row, or host
application object. An asset remains the same semantic item when storage moves
or a different rendition becomes available.

Each asset contains independently identified `RepresentationDescriptor`
records. A representation is one concrete rendition of the asset and carries:

- `AssetRepresentationId`;
- a namespaced semantic role;
- a revision fingerprint plus explicit evidence kind;
- namespaced capabilities;
- a portable project-resource or stable runtime-resolution binding;
- forward-compatible portable extensions.

Paired HDR and SDR flattened TIFFs are two representations of one asset. The
initial vocabulary also accommodates original Camera RAW, a RAW-preview TIFF,
a layered master PSB, and flattened SDR/HDR renditions. These are ordinary
namespaced roles, not privileged Core asset variants.

Capabilities should increasingly describe what a consumer can do with a
representation: for example `visual`, `raster`, `hdr`, `has-alpha`,
`previewable`, `color-managed`, `timed-media`, `seekable`, or `audio`. TIFF,
AVIF, EXR, ProRes, and similar format or container identities belong in
representation metadata rather than defining the asset model. The initial
Stage 5 vocabulary still contains image, TIFF, flattened-image, and HDR/SDR
declarations for the paired-TIFF adapter; it is a narrow starting vocabulary,
not a closed capability taxonomy or a Core assumption that visual assets are
still images. Those declarations do not themselves assert decoded color
correctness; the Stage 6 measurements test that separately.

Asset and representation identity survive a project-relative path change. The
resource binding points to a `ProjectResourceId`, whose path may change without
changing either semantic identity or the content fingerprint. When upstream
content changes, the representation keeps its identity but receives a new
fingerprint. Future proxy keys therefore change naturally.

The fingerprint is the representation's current cache/revision identity, but
`revision_evidence` states how that identity was obtained. `content-digest`
means Photara verified the source bytes; `provider-revision` is a provider's
stable revision/version contract; and `file-observation` is cheap filesystem
evidence such as size plus modification time. Cheap evidence lets a provider
publish an asset immediately without pretending that it already read a large
remote file. A verified materializer or production proxy accepts only evidence
strong enough for its contract. Older project documents omit this field and
therefore default to the historical `content-digest` meaning.

## Bindings and configurable placement

`RepresentationBinding::ProjectResource` points to a portable project-relative
resource. `RepresentationBinding::RuntimeResolved` carries only a stable
`RepresentationStorageBindingId`; the actual filesystem, library, cloud, mount,
account, or credential locator is resolved outside the Project Document. A
runtime materialization returns a verified local path without making that path
portable authority.

The target model generalizes this into a Library-owned logical Storage Location
plus a device-owned Host Binding. It also distinguishes external sources,
managed Project resources, external output artifacts and transient cache. See
[Storage locations and host bindings](STORAGE_LOCATIONS_AND_HOST_BINDINGS.md).

Output placement is separate from representation identity and current binding.
Configurable storage policy may choose targets per role. A useful default is:

```text
RAW/original location
├── camera RAW
└── layered PSB

Project location
├── RAW preview TIFF
├── flattened SDR TIFF
└── flattened HDR TIFF
```

That is a preference, not a Core requirement. A user may place any role in any
resolved storage target. Storage-policy adapters control output placement;
assets, representations, fingerprints, graph values, and proxy keys remain
independent of those choices.

## Portable versus runtime state

`ProjectDocument.asset_context` is portable authored project state. It contains
semantic assets, representations, roles, capabilities, fingerprints, and
portable project-resource or runtime-resolution handles. It contains no
machine-resolved path.

Availability and materialization are runtime queries. A materialization request
names the asset, representation, and expected fingerprint. A materializer
reports available, missing, or inaccessible and returns a verified local path
only at runtime. It rejects stale requests and files whose current SHA-256 no
longer matches the portable descriptor.

Preview readiness is not one authoritative asset status. Native clients observe
the best currently displayed revision and its transient loading, updating,
ready, or failed activity per preview profile. Those observations may differ
between devices and consumers and never enter the project or graph digest.

The portable schema rejects availability, materialization, proxy, thumbnail,
preview, cache, credential, library, and Gallery-selection extension fields.
Those values cannot silently become project authority.

Representation format/container metadata is distinct from consumer
capabilities. The initial portable `photara.format.extension` field supplies a
normalized presentation label such as TIFF, DNG, JPEG, or AVIF; it does not
replace capabilities such as visual, raster, HDR, previewable, or
color-managed, and consumers must not route solely from the filename label.

## Explicit AssetSet input

`AssetSet` is the version-1 `photara.asset-set` typed value. It carries explicit,
ordered, duplicate-free asset membership and validates every ID against project
asset context. Graph nodes consume this value through declared ports. Gallery
selection is transient client state; drag/drop or another user action must
create an explicit command or AssetSet binding before it can affect evaluation.

Layout's existing `assets` port now uses Core's exact AssetSet descriptor rather
than a package-local approximation.

## Development local adapter

`LocalProjectAssetAdapter` resolves project-resource bindings relative to an
explicit project root. The paired-TIFF import helper fingerprints local `.tif`
or `.tiff` files and creates one asset with HDR and SDR representations. These
files are development fixtures or imported stand-ins for output that future
Photoshop, Lightroom, Lureva, cloud, or other upstream nodes may produce.

The adapter deliberately performs no TIFF decoding, proxy generation, ICC
conversion, tone mapping, or backend selection. It streams bytes for SHA-256,
tracks availability, verifies materialization, and can refresh a descriptor's
fingerprint after changed upstream content. Production asset edits will pass
through semantic application commands; the helper only constructs development
project state for this stage.

No Photoshop, Lightroom, Lureva, filesystem-root, or cloud-provider type enters
Core. Future upstream nodes publish ordinary assets and representations through
the same semantic contracts. Photoshop will be a separate node shipped with a
bundled UXP panel; the paired TIFF adapter remains its development stand-in and
does not implement or anticipate Photoshop behavior in Core.

## Downstream proxy rule

Stage 6 supplies one project-scoped proxy service shared by Layout, Gallery,
Inspector, and future consumers. Proxies are derived cache objects keyed by at
least the representation fingerprint, proxy profile, and relevant color/HDR
policy. They are never stored in `ProjectDocument.asset_context` and are never
owned by Layout.
