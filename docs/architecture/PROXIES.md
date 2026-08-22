# Project proxy infrastructure

## Contract boundary

Proxy infrastructure is a project-scoped service shared by nodes and native UI
consumers. Layout, Gallery, and Inspector may request a proxy; none owns proxy
generation, cache placement, or invalidation.

Core defines backend-neutral `ProxyRequest`, `ProxyProfile`, `ProxyProfileRef`,
`ProxyCacheKey`, and `ProxyDescriptor` records. A request names the project,
asset, representation, expected source fingerprint, and complete versioned
profile. The profile includes purpose, sizing, upscale behavior, resampling,
orientation, color conversion and rendering intent, HDR/tone-map policy,
channel depth, alpha policy, and an exact encoder recipe revision. The service
also supplies an exact generator revision when deriving the cache key.

Cache identity is the canonical SHA-256 digest of contract version, source
fingerprint, the entire proxy profile, and the selected generator revision.
Request, project, asset, representation, and consumer identities are excluded
because they do not alter derived bytes. Consequently:

- changing upstream content creates a different key through its fingerprint;
- changing color, HDR, tone-map, resampling, depth, or encoding policy creates a
  different key;
- changing encoder or generator implementation revision creates a different key;
- multiple nodes and UI views can reuse the same project-scoped result;
- deleting the proxy cache cannot delete or mutate authoritative project state.

`ProxyDescriptor` is derived cache metadata, not a portable Project Document
field. It describes actual dimensions, encoding, depth, color space, embedded
ICC fingerprint, dynamic range, normalized orientation, output fingerprint,
and byte length without containing a machine path.

## Measured corpus and method

Measurements ran on Quasar on 2026-08-21: Apple M1 Ultra with 128 GiB RAM,
macOS 26.5.2, Xcode 26.6, Swift 6.3.3, Rust 1.95.0, libvips 8.18.4, and
ImageMagick 7.1.2-27 Q16-HDRI with LCMS and TIFF delegates.

The reproducible corpus is generated under `/private/tmp` and is not checked
into Git:

- 8000×5333 Display-P3 unsigned-16 SDR TIFF, 179 MiB compressed;
- 8000×5333 linear-ACEScg float-32 HDR TIFF with negative samples and headroom
  above 4.0, 302 MiB compressed;
- asymmetric 1600×1000 Display-P3 unsigned-16 TIFF with orientation 6.

Deterministic fine-grain texture prevents the large inputs from behaving like
trivially compressed gradients. Three isolated process runs were measured with
macOS `/usr/bin/time -lp`; the table reports medians. Correctness checks covered
dimensions, orientation normalization, ColorSync-visible output profile,
wide-gamut sample values, TIFF sample depth, and preservation of HDR headroom.

| Backend path | SDR 512 px | Peak RSS | HDR 2048 px | Peak RSS | Exact result |
|---|---:|---:|---:|---:|---|
| ImageIO float thumbnail + Core Image | 0.45 s | 475 MiB | 1.09 s | 954 MiB | Pass |
| libvips + LCMS/libtiff | 1.60 s | 261 MiB | 2.28 s | 413 MiB | SDR pass; HDR emitted F32, not requested F16 |
| ImageMagick Q16-HDRI | 6.76 s | 1,213 MiB | 3.39 s | 1,203 MiB | Pass |
| Rust `image` | 1.28 s | 534 MiB | 3.71 s | 1,210 MiB | Failed color transform and HDR/depth |

The Apple, libvips, and ImageMagick SDR results agreed within one 8-bit code
value at measured wide-gamut points after Display-P3-to-sRGB conversion. The
Rust path left the result tagged Display P3 and produced materially different
samples because extracting an ICC profile is not applying it. Its HDR path also
clipped the source to `[0, 1]` and emitted F32. All paths normalized orientation
to an upright 320×512 output.

The optimized Apple HDR path is important: eager full-image decode took 6.48 s
and about 2.0 GiB RSS. `CGImageSourceCreateThumbnailAtIndex` with float samples
allowed, followed by Core Image F16 export, reduced that to 1.09 s and about
954 MiB while preserving linear ACEScg, negative samples, and HDR values above
4.0. CPU and GPU Core Image context variants were indistinguishable at this
size because ImageIO dominated the work.

Raw medians are preserved in
[`benchmarks/proxy-backends/results/quasar-2026-08-21.csv`](../../benchmarks/proxy-backends/results/quasar-2026-08-21.csv).

## Decision

Use ImageIO plus Core Image as the first macOS production backend, behind the
general project proxy service:

- it passed both exact profiles, including ColorSync-visible ICC metadata,
  orientation, wide gamut, F16 HDR, negative samples, and headroom;
- it was fastest for both measured profiles after float-thumbnail optimization;
- it requires no bundled third-party imaging runtime;
- the spike compiles on Quasar with Xcode 26.6 and introduces no macOS 27 API.

Use automatic/default Core Image rendering initially. The measured CPU/GPU
choice did not affect elapsed time or RSS, so it should remain a backend tuning
choice rather than enter Core profile or cache identity unless later evidence
shows it changes output bytes.

Keep libvips as the leading future Windows/portable backend candidate. It had
the lowest memory and strong throughput, and its official API supports embedded
ICC transforms, intent, black-point compensation, TIFF ICC metadata, and
orientation. It is not the first macOS choice because the measured HDR path did
not satisfy the F16 contract and bundling a minimal libvips/LCMS/libtiff stack is
additional deployment work.

Do not select ImageMagick by default. It was correct but slow for thumbnails,
used about 1.2 GiB for both profiles, and carries a larger delegate/runtime
surface. Do not select the current Rust-native path: portability and static
deployment are attractive, but the measured implementation did not perform the
requested ICC transform and clipped HDR. Revisit Rust-native color management
only with an exact-profile implementation and the same corpus.

Relevant capability references: Apple documents ImageIO thumbnail creation,
float decode, cache behavior, and orientation transforms in
[CGImageSource](https://developer.apple.com/documentation/imageio/cgimagesource),
and F16 TIFF export through
[Core Image](https://developer.apple.com/documentation/coreimage/cicontext/writetiffrepresentation%28of%3Ato%3Aformat%3Acolorspace%3Aoptions%3A%29).
libvips documents TIFF ICC/orientation handling in
[`tiffload`](https://www.libvips.org/API/current/ctor.Image.tiffload.html) and
explicit intent/black-point-compensated transforms in
[`icc_transform`](https://www.libvips.org/API/current/method.Image.icc_transform.html).
ImageMagick documents its [HDRI model](https://imagemagick.org/high-dynamic-range/)
and [ICC color management](https://imagemagick.org/color-management/). The Rust
`image` decoder documents that it exposes
[ICC and orientation metadata](https://docs.rs/image/latest/image/trait.ImageDecoder.html),
which the measurement confirms is not by itself a complete color pipeline.

## Production project service

`photara-proxy` implements the runtime service without adding cache state to
Core or the Project Document. One service instance is scoped to one semantic
project ID and one cache root. Its request order is deliberate:

1. derive the Stage 6A key and verify an existing object;
2. join or install the key's in-flight record;
3. only the leader waits for a bounded generation slot;
4. materialize and fingerprint-check the explicit source representation;
5. generate into a unique staging directory;
6. validate backend-reported output policy and dimensions, then verify bytes
   and SHA-256 fingerprint;
7. synchronize metadata and atomically publish the complete directory.

This ordering means identical concurrent requests consume one slot, one source
materialization, and one generation. Followers receive the same verified
artifact. Failures are not negatively cached, so a missing or unmounted source
can be retried normally after it becomes available.

Each content-addressed object contains only `proxy`, `descriptor.json`, and a
derived access marker. Cache hits re-hash the payload and verify its byte length,
profile digest, source fingerprint, generator revision, and cache key. A failed
check deletes that exact derived entry and regenerates it. Publication renames a
synchronized staging directory, preventing interrupted jobs from appearing as
hits. The quota counts payload and metadata bytes and evicts least-recently-used
objects while preserving the just-published result. An object larger than the
quota is rejected before publication. Returned artifacts hold a lightweight
lease, so quota enforcement and cache clearing cannot delete a payload while a
consumer still holds its runtime handle; eviction resumes on later requests
after leases are released.

Clearing the service deletes only the project's derived cache directory and is
rejected while requests or artifact leases are live. Project JSON, authored
node state, artifacts, receipts, and evidence live outside this directory and
cannot be removed by cache operations.

## macOS production adapter

The first adapter launches the bundled `photara-proxy-imageio` helper for one
generation, then reads a small JSON metadata result. The helper contains the
large ImageIO/Core Image decoder working set and exits after the job; no pixel
buffer, `CGColorSpace`, `CIImage`, filesystem locator, or macOS object crosses
into Core. Process isolation is intentional for this memory-heavy boundary, not
a replacement for the selected in-process UniFFI application facade.

The helper implements the measured 512 px Display-P3-to-sRGB U8 PNG thumbnail
path and the 2048 px embedded-color F16 TIFF HDR authoring-preview path. It
normalizes orientation and composites to opaque output. The current HDR adapter
recognizes the system sRGB, Display P3, and linear ACEScg profiles; an unknown
embedded HDR profile fails explicitly instead of silently writing mislabeled
color. It builds on Quasar with Xcode 26.6 and uses no macOS 27 API.

## Measured concurrency policy

Stage 6A measured the isolated 42.7 MP Apple HDR path at about 954 MiB peak RSS.
Stage 6B additionally sampled the summed RSS of production helper processes at
20 ms intervals for three one-job and three two-job groups:

| Simultaneous jobs | Median group time | Median sampled aggregate RSS |
|---:|---:|---:|
| 1 | 0.795 s | 658 MiB |
| 2 | 0.857 s | 1,316 MiB |

The sampled comparison is useful for scaling, while `/usr/bin/time` from Stage
6A remains the more conservative per-process peak. Two jobs nearly doubled
resident memory. Until the complete native application is measured on supported
lower-memory Macs, `ProxyServiceConfig::conservative` therefore permits exactly
one active generation. The bound is explicit and configurable; it is not based
on core count. The service tests separately prove that deduplicated followers
do not consume this single slot.

Raw concurrency samples are in
[`benchmarks/proxy-concurrency/results/quasar-2026-08-21.csv`](../../benchmarks/proxy-concurrency/results/quasar-2026-08-21.csv).

## Layout consumption boundary

Stage 7 adds `ProjectVisualProxyService`, a deliberately narrow runtime
interface keyed by project, semantic asset identity, and an exact proxy
profile. Its initial asset-context binding chooses a representation by HDR/SDR
capability, materializes it, and delegates to the existing shared service.
Selection is capability-based rather than TIFF- or provider-based.

Layout resolves its authoritative semantic plan without this interface. A
preview consumer may then request one proxy per distinct placed asset and hold
the returned `LayoutProxySet` ephemerally. The set is not serializable; cache
keys, descriptors, leases, and local paths cannot enter Layout state or plan.
Consequently proxy failure or complete cache deletion cannot invalidate a
project's authored Layout.
