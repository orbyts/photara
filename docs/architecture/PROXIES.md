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

## Next implementation boundary

The measured decision authorizes a production macOS backend adapter; it does
not move platform objects into Core. The next slice is request deduplication,
content-addressed cache storage, atomic writes, descriptor verification, quotas,
corruption recovery, and unavailable/remounted-source behavior behind these
contracts. No Layout or Gallery UI belongs in that slice.
