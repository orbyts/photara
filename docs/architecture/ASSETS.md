# Project assets and representations

## Ownership and identity

The project owns asset context. An `AssetId` identifies the creative item a user
recognizes; it is not a path, file, provider record, Gallery row, or host
application object. An asset remains the same semantic item when storage moves
or a different rendition becomes available.

Each asset contains independently identified `RepresentationDescriptor`
records. A representation is one concrete rendition of the asset and carries:

- `AssetRepresentationId`;
- a namespaced semantic role;
- an immutable SHA-256 content fingerprint;
- namespaced capabilities;
- a portable project-resource binding;
- forward-compatible portable extensions.

Paired HDR and SDR flattened TIFFs are two representations of one asset. The
initial roles are `photara.rendition.hdr` and `photara.rendition.sdr`; declared
capabilities describe image, TIFF, flattened-image, and HDR/SDR properties.
Stage 5 does not decode TIFFs or assert color correctness from those declarations.
That measurement and imaging work belongs to Stage 6.

Asset and representation identity survive a project-relative path change. The
resource binding points to a `ProjectResourceId`, whose path may change without
changing either semantic identity or the content fingerprint. When upstream
content changes, the representation keeps its identity but receives a new
fingerprint. Future proxy keys therefore change naturally.

## Portable versus runtime state

`ProjectDocument.asset_context` is portable authored project state. It contains
semantic assets, representations, roles, capabilities, fingerprints, and
project-resource bindings. It contains no machine-resolved path.

Availability and materialization are runtime queries. A materialization request
names the asset, representation, and expected fingerprint. A materializer
reports available, missing, or inaccessible and returns a verified local path
only at runtime. It rejects stale requests and files whose current SHA-256 no
longer matches the portable descriptor.

The portable schema rejects availability, materialization, proxy, thumbnail,
preview, cache, credential, workspace, and Gallery-selection extension fields.
Those values cannot silently become project authority.

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
the same semantic contracts.

## Downstream proxy rule

Stage 6 supplies one project-scoped proxy service shared by Layout, Gallery,
Inspector, and future consumers. Proxies are derived cache objects keyed by at
least the representation fingerprint, proxy profile, and relevant color/HDR
policy. They are never stored in `ProjectDocument.asset_context` and are never
owned by Layout.
