# Photara Layout and HDR Packaging Reference

This document records the working design for Photara layouts, HDR packaging,
Photoshop execution, Web Sharp Pro (WSP) handoff, and publication-package
versioning. It is the durable reference for the `0.0.9` implementation. The
examples are concrete enough to test with Red Meridian, while the model remains
project- and platform-independent.

The JSON fragments are illustrative contracts. Field names may be refined as
the Rust types and migrations are implemented, but the behavioral rules in
this document should not change accidentally.

## Responsibilities

Photara core is Rust. Rust owns:

- template and package schemas;
- stable asset and rendition resolution;
- layout geometry, crops, transforms, frame ordering, and safe areas;
- validation, previews, manifests, checksums, provenance, and state;
- platform profiles and the adapter contract presented to WSP;
- database projections and publication evidence.

Photoshop automation uses UXP and remains a thin deterministic host adapter.
UXP places 32-bit sources, creates the prescribed layer tree, applies the
resolved transforms, preserves color profiles and bit depth, saves the working
document, and returns a machine-readable report. UXP must not invent layout or
workflow policy.

WSP owns final HDR delivery processing:

- interpreting the top HDR layer/group and lower SDR base layer/group;
- gain-map generation;
- splitting continuous multi-frame compositions;
- final resizing and high-quality downsampling;
- output sharpening and delivery encoding.

Photara records what WSP was asked to produce and verifies its outputs, but
does not depend on WSP internals.

## Layout vocabulary

- **Template**: reusable, versioned geometry and styling such as `full-frame`,
  `stacked-two`, `grid-four`, `dynamic-range-comparison`, or
  `edit-comparison`.
- **Placement**: one use of an asset rendition inside a template. One asset may
  have any number of placements, including repeated placements in one post.
- **Editorial item**: one authored composition. It may produce one delivery
  frame or a continuous composition that WSP splits into several frames.
- **Post specification**: a project-owned, ordered, platform-specific editorial
  sequence, such as `Package A` for Instagram.
- **Post snapshot**: an internally immutable resolution of a post
  specification, including its ordered items, exact template versions,
  placements, transforms, text, and source bindings. This exists for
  provenance; it is not the same as a user-facing template version.
- **Rendition**: a representation of an asset for a purpose, including the
  layered PSB, flattened HDR TIFF, SDR-authored rendition, WSP master, and
  provider delivery file.
- **Publication**: a provider attempt and its durable evidence. It references
  an immutable post snapshot rather than a mutable project file.

## Global template versioning and project posts

Reusable templates are global and explicitly versioned. The project post is an
ordered composition that selects templates and assets. These are separate
concerns:

```text
global template: dynamic-range-comparison@2
project post:    red-meridian/instagram/package-a
```

Global templates may be built into Photara or installed in the configured
device-independent registry. Suhail's registry is synchronized by Dropbox:

```text
$DROPBOX/Pictures/Photara/Templates/
    ├── full-frame/
    │   └── v1.json
    ├── stacked-two/
    │   └── v1.json
    ├── grid-four/
    │   └── v1.json
    ├── grid-four-threads/
    │   └── v1.json
    └── dynamic-range-comparison/
        ├── v1.json
        ├── v1/
        │   └── reference.psd
        ├── v2.json
        └── v2/
            └── reference.psd
```

The main Photara configuration pins global defaults to exact versions:

```toml
[layouts.defaults]
full_frame = "full-frame@1"
stacked_two = "stacked-two@1"
stacked_three = "stacked-three@2"
continuous_panorama = "continuous-panorama@1"
dynamic_range_comparison = "dynamic-range-comparison@2"
edit_comparison = "edit-comparison@1"
```

The registry and disposable local cache are configured separately:

```toml
templates_root = "$DROPBOX/Pictures/Photara/Templates"
templates_cache = "~/Library/Caches/photara/templates"
```

Photara verifies the immutable registry bytes, materializes a matching local
cache entry, and then prepares the project-scoped Photoshop handoff. Neither a
post specification nor a database record stores the machine-specific Dropbox
or cache path.

Updating the default affects future draft resolution only. It must never alter
an existing rendered or published post. A post item may explicitly override a
global default when a particular design version is desired.

## Four-image 3:4 grid family

`grid-four@1` is the Instagram 2×2 grid. Its 4500×6000 canvas divides into four
exact, gapless 2250×3000 cells named `top-left`, `top-right`, `bottom-left`, and
`bottom-right`.

`grid-four-threads@1` divides the complete 4500×8000 Threads canvas into four
exact, gapless 2250×4000 cells. Each cell is 9:16. The two immutable templates
are separate because the target crop belongs to the platform composition.

The layered PSB and flattened TIFF pair may use any final authored aspect. Each
cell has the same explicit `fill`, `contain`, or `crop` policy as every other
ordinary placement. A dual-platform authoring session opens only unresolved
`crop` contexts from both ordered platform sets and pins both post files, but
never copies one platform's transform into the other.

Platform-specific posts always own their own transforms. Full-frame, grid,
stacked, comparison, panorama, and rotated placements remain independently
authored even when one Photoshop session prepares both platforms.

Project-owned post specifications live beneath the configured project root,
for example:

```text
red-meridian/
└── posts/
    └── instagram/
        ├── package-a.json
        └── previews/
```

Freezing a post resolves every default or alias to an exact template version,
canonicalizes the project JSON, stores its SHA-256 and exact resolved snapshot
in PostgreSQL, and assigns an immutable snapshot ID. Render and publication
records refer to that ID. Editing the project post later creates a new internal
snapshot; it never rewrites the evidence for a prior publication. This internal
revisioning is an audit mechanism, not the template-version concept exposed to
the photographer.

During layout authoring, `posts prepare-render --item <item-id>` creates a
single-item Photoshop handoff for rapid visual debugging and review. Omitting
`--item` remains the production behavior and resolves, verifies, and renders
the complete ordered package.

The database indexes relationships and evidence. It must not infer asset
identity from filenames. Human-readable filenames may appear in reports, but
layout bindings use stable Photara asset and rendition IDs.

A post may have `draft`, `frozen`, `rendered`, `published`, or `superseded`
state. Only the project draft is editable. A new draft may be cloned from any
prior snapshot.

### Rendering unit and reuse

A post is a logical ordered aggregate, not one monolithic Photoshop
document or layer group. Each editorial item is an independently restartable
rendering unit with its own WSP source document:

```text
resolved post snapshot
├── item 1 → one WSP document → one delivery frame
├── item 2 → one WSP document → one delivery frame
└── item 3 → one continuous WSP document → several split delivery frames
```

The post manifest expands verified item outputs into final provider order.
The UXP panel may batch-build many items, but a failure in one item must not
invalidate or require rebuilding the others.

Each frozen item receives a render key derived from its template and style
versions, platform profile, source checksums, SDR/HDR bindings, transforms,
annotations, and continuous-frame topology. A later post snapshot may reuse an
already verified item output when that key is unchanged. Reordering a post
therefore does not force image rendering, while changing a crop or source does.

Before freezing, Photara expands continuous items and validates the resulting
delivery-frame count against the selected provider capability profile. It must
never silently discard frames. If a package exceeds a provider limit, the user
must revise it or explicitly divide it into separately versioned publication
segments.

## Geometry and resolution

Layout geometry is expressed in normalized coordinates and resolved by Rust
against a platform authoring profile. Delivery resolution is a separate WSP
concern.

Current authoring masters:

- Instagram single frame: `4500 × 6000` (3:4).
- Threads single frame: `4500 × 8000` (9:16).

These are standardized working resolutions, not assumed provider delivery
limits. WSP may reduce them for the final output.

Photara prefers but does not require:

- 3:4 for portrait compositions;
- 2:3 for landscape compositions, because two stacked 2:3 images occupy a 3:4
  frame.

Every ordinary placement—including full-frame, stacked, grid, comparison, and
future platform placements—supports one explicit fit policy:

- `fill`: preserve source aspect, scale to cover the target, and crop
  automatically around the focal point;
- `contain`: preserve source aspect and fit the complete image inside the
  target, leaving letterbox or pillarbox area; or
- `crop`: require an operator-authored crop in that platform's target aspect.

Aspect mismatch never implies authoring by itself. Only unresolved `crop`
placements enter generalized authoring. A captured transform is independent
per platform. Sources are never stretched. Continuous panoramas retain their
specialized crop and seam contract.

### Flexible three-image stacks

`stacked-three@2` is one global parameterized template for Instagram and
Threads. Omitting row percentages always means an equal one-third,
gapless distribution. A post item may instead store three positive integer
percentages such as `[30, 40, 30]`. Photara resolves those proportions against
each platform's authoring canvas, so crop authoring receives 4500×1800,
4500×2400, and 4500×1800 targets on Instagram and 4500×2400, 4500×3200,
and 4500×2400 targets on Threads.

Row percentages may total less than 100 only when the post explicitly requests
outer letterboxing. Photara splits the unused height equally above and below
the stack and fills it with black in both the HDR and SDR composites. Totals
above 100, zero-height rows, and implicit underfill are errors. Stack geometry
is separate from each placement's `crop`, `fill`, or `contain` policy.

### Continuous multi-frame compositions

Photara authors a continuous composition and describes its logical frame
boundaries. WSP performs the physical split. Photara does not need to rasterize
two Instagram frames as an upsampled `9000 × 6000` document.

Two horizontal 3:4 frames form a 3:2 surface. This naturally matches the
camera-native landscape dimensions used in the current archive:

- Sony A7R III: `7952 × 5304` (effectively 3:2);
- Sony A7R V: `9504 × 6336` (3:2).

The resolved working raster uses a configurable no-upscale policy based on the
available sources. The package records logical frame count, aspect ratio,
flow, seam guides, and WSP split order independently from raster dimensions.

```json
{
  "surface": {
    "frame_aspect": "3:4",
    "frame_count": 2,
    "flow": "horizontal",
    "resolution_policy": "no-upscale"
  },
  "delivery": {
    "splitter": "web-sharp-pro"
  }
}
```

One continuous transform is calculated before splitting, preventing scale or
crop drift at the seam. A multi-frame item remains one editorial item even
though it expands into several ordered delivery frames.

### Panorama crop authoring

`continuous-panorama@1` uses one continuous 3:2 crop representing two
horizontal 3:4 frames. The subjective crop uses a two-step Photoshop-native
handoff instead of holding Photoshop in a modal script while the photographer
decides:

1. Rust resolves and fingerprints the verified source and project post.
2. The UXP authoring script opens the HDR TIFF without changing it and creates
   a centered 3:2 rectangular selection.
3. The photographer moves or transforms the selection while preserving 3:2.
4. The UXP capture script records both pixel and normalized selection bounds
   and previews the seam at the midpoint of the current crop. The crop can be
   adjusted and recaptured until approved.
5. Rust rechecks the source and post fingerprints, ratio, and bounds before
   writing the normalized crop into the project-owned post.

The exact crop drives both HDR and SDR. Photoshop scripts never write the post
directly. A later UXP panel can present this as a draggable overlay with seam
preview and Apply/Cancel while retaining the same durable data contract.

## Authoritative paired master contract

Starting in `0.0.9`, the authoritative 32-bit PSB owns both deliberate display
renditions. It must contain exactly named top-level groups or Smart Objects in
this order:

```text
HDR    ← top, the completed HDR edit
SDR    ← below, the deliberately authored SDR interpretation
```

The existing completed edit can be wrapped as `HDR`; it is not rebuilt. The
`SDR` version is authored deliberately—normally by adjusting an embedded Smart
Object through Camera Raw—while the document remains 32-bit. Neither rendition
is synthesized from the other without review.

The UXP master action validates the names, order, dimensions, profile, and bit
depth before exporting two independent, flattened 32-bit TIFFs. Photara records
them as:

- `flattened-hdr-tiff`
- `flattened-sdr-tiff`

For a canonical master base such as `DSC05217_2021_06_11_SUHAIL`, the portable
filenames are:

```text
DSC05217_2021_06_11_SUHAIL_HDR.TIF
DSC05217_2021_06_11_SUHAIL_SDR.TIF
```

Both live in the project's `masters/flattened/` directory. The rendition role
is retained in the filename even when the file is copied away from that
directory. Smart Filters—including the Camera Raw Filter used for SDR
authoring—are rendered by Photoshop when the corresponding container is
isolated and flattened.

Both records point back to the same layered PSB and carry independent paths,
checksums, byte sizes, and verification evidence. Existing `0.0.8` flattened
TIFFs migrate in place as HDR renditions; migration does not copy or reinterpret
them as SDR. A missing SDR record keeps a layout in `authoring-required` state.

Layouts consume these verified TIFFs rather than opening PSBs directly. This
keeps composition repeatable and makes the paired input contract identical for
full-frame, stacked, comparison, and continuous multi-frame templates.

## WSP HDR/SDR document contract

Every HDR-capable WSP document contains two complete composites:

```text
HDR    ← top layer, group, or Smart Object
SDR    ← lower base layer, group, or Smart Object
```

The SDR composite is the ordinary base image. WSP derives the gain map from the
relationship between the SDR base and HDR alternate. Pixels that are identical
between them contribute no additional HDR headroom.

Backgrounds, typography, pills, borders, annotations, geometry, masks, and
other non-HDR content must be pixel-identical in both composites. Only declared
HDR-variable image or ramp regions may differ.

UXP should construct the SDR group first, duplicate it as HDR, place HDR above
SDR, and replace only declared HDR-variable contents. A duplicated annotation
Smart Object may be used in each group so both instances share the same
embedded source.

Before WSP handoff, Photara should compare the SDR and HDR composites outside
the declared variable masks. Unexpected differences fail validation. The UXP
report must also confirm group order, bounds, profile, bit depth, and expected
layer names.

### Ordinary photographic frame

For a full-frame or stacked layout:

- the SDR group contains the deliberately authored SDR rendering of every
  HDR-capable image;
- the HDR group contains the corresponding HDR rendering in exactly the same
  geometry;
- SDR-only images are identical in both groups;
- all layout chrome is identical.

The SDR rendering is an authored result, not an automatic flat or intentionally
inferior conversion. The initial workflow may create it through Adobe Camera
Raw on a Smart Object. Photara records the relationship between the SDR and HDR
renditions and the originating layered PSB.

## Comparison template family

`dynamic-range-comparison` and `edit-comparison` share the same canvas, title
area, divider, 2×2 image grid, gutters, corner treatment, typography, and lower
right pill labels. Only the informational band changes.

### Dynamic Range Comparison

Each row compares one pair:

```text
left: SDR
right: HDR
```

Editorial role and WSP encoding role are separate. The complete WSP composites
are:

| Region | WSP SDR base | WSP HDR top |
| --- | --- | --- |
| Background and annotations | Identical | Identical |
| Left `SDR` image | SDR rendition | Same SDR rendition |
| Right `HDR` image | SDR rendition of that right-side image | HDR rendition of that image |
| Standard ramp | `0 → 1` | Identical `0 → 1` |
| HDR headroom ramp | Flat SDR white, `1 → 1` | True HDR ramp, `1 → 10` |

The right cell remains labeled `HDR` in both composites because `HDR` is its
editorial role. Its SDR-group content is the SDR base needed by WSP.

The reference bands mean:

- `0` = black;
- `1` = SDR white;
- `2` = +1 stop;
- `4` = +2 stops;
- `10` ≈ +3.3 stops above SDR white.

The standard ramp is identical between composites. The HDR-headroom region is
flat white in the SDR base, visibly demonstrating that SDR has no representable
range above white. In the HDR alternate it progresses from 1 to 10, allowing
WSP to encode the additional headroom in the gain map.

Ramp positions use exposure stops rather than raw numeric distance:

```text
position(value) = log2(value) / log2(10)
```

The `+3` label is intentionally omitted for phone readability. The endpoint is
labelled `+3.3 HDR Peak`.

Pairs should use matching orientation. The SDR and HDR rendition of a given
right-side image should use identical composition so dynamic range is the
intended difference.

### Edit Comparison

Each row compares:

```text
left:  BEFORE — honest default/original RAW rendering
right: AFTER  — final authored edit
```

The information band shows compact camera and capture metadata. The before
rendering must not be deliberately degraded to exaggerate the edit.

For `edit-comparison@1`, the Before image is rendered from the authoritative
camera RAW using Lightroom Classic's **Reset** result with the **Adobe Color**
profile and no user adjustment. Photara temporarily applies that state,
exports a full-resolution 16-bit ProPhoto RGB TIFF, restores the complete
authored develop settings, verifies the restoration, and fingerprints the
export before Photoshop may consume it. This operation does not write XMP.

Camera and capture labels come from the camera RAW's Lightroom metadata, not
from a flattened derivative. The normalized presentation is:

```text
Camera:  <friendly camera model> · <lens>
Capture: ISO <value> · <focal length>mm · ƒ/<aperture> · <shutter>
```

Photara creates these value layers afresh rather than replacing placeholder
contents. Their locked style is `SFCompact-Ultralight`, 27 pt, zero tracking,
no synthetic bold or italic, and the reference neutral gray. At 300 PPI the
UXP API receives 112.5 document pixels, which is the exact 27-point size.
After rendering at 27 pt, Photara measures Photoshop's actual glyph bounds
against the appropriate metadata column. Overflow scales proportionally only
as needed, with a 22-point readability floor and fixed right padding. Text
that cannot fit above the floor is rejected for editorial shortening rather
than made illegible.

The immutable `edit-comparison@1` reference uses two 2000 × 2000 square cells
per row on the 4500 × 6000 Instagram canvas. Before and After imagery uses
contain-fit, leaving the reference background as intentional letterboxing or
pillarboxing rather than cropping either comparison.

The WSP SDR composite contains an SDR rendition of the entire slide. Its HDR
alternate preserves identical layout and annotations while replacing only the
HDR-capable final-image renditions. The same variable-mask validation applies.

## PSD visual reference

The current visual source of truth is `3x4-6000.psd`. Static inspection confirms:

- canvas `4500 × 6000` at 300 PPI;
- 32-bit RGB;
- Display P3 (Linear RGB Profile);
- actual linear ramp channel values reaching approximately `9.947`;
- a readable composite and 19 text layers.

Representative extracted text bounds are:

| Element | Position | Bounds |
| --- | ---: | ---: |
| Dynamic Range Comparison | 217, 198 | 3189×258 |
| Subtitle | 209, 519 | 3474×124 |
| Standard | 231, 787 | 531×93 |
| HDR Headroom | 2279, 787 | 880×93 |
| Black | 233, 1275 | 248×69 |
| SDR White | 1756, 1269 | 473×74 |
| +3.3 · HDR Peak | 3558, 1273 | 717×70 |
| Top SDR pill | 1953, 3525 | 205×79 |
| Top HDR pill | 3987, 3527 | 211×75 |
| Bottom SDR pill | 1953, 5753 | 205×79 |
| Bottom HDR pill | 3987, 5755 | 211×75 |

A Photara UXP template inspector should export exact Photoshop-native canvas,
group, layer, bounds, text, font, tracking, leading, color, opacity, shape,
stroke, and corner-radius descriptors to JSON. Rust validates and converts that
inspection into a reusable template. When Photoshop text metadata is
ambiguous, the rendered reference remains visually authoritative and the value
becomes an explicit configurable token.

## Red Meridian example: Instagram Package A

This is the accepted 20-slot Instagram plan as of 2026-08-12. Repeated use of
an asset is intentional: each placement gives the viewer a different
experience without duplicating the underlying asset or master. An editorial
item and a delivered carousel slot are not the same thing. Each panorama is
one Photara editorial surface which WSP splits into two adjacent Instagram
slots.

| IG slot | Photara item | Source | Layout | Status |
| --- | --- | --- | --- | --- |
| 1 | `hero` | `DSC05250_2021_06_11_SUHAIL` | Full-frame hero | Complete |
| 2 | `stacked-01` | `DSC05445_2021_06_11_SUHAIL` + `DSC05442_2021_06_11_SUHAIL` | Stacked two | Complete |
| 3 | `full-frame-05217` | `DSC05217_2021_06_11_SUHAIL` | Full frame | Complete |
| 4 | `full-frame-05406` | `DSC05406_2021_06_11_SUHAIL` | Full frame | Complete |
| 5–6 | `panorama-05382` | `DSC05382_2021_06_11_SUHAIL` | Continuous two-slot panorama | Complete |
| 7 | `full-frame-05409` | `DSC05409_2021_06_11_SUHAIL` | Full frame | Complete |
| 8 | `stacked-02` | `DSC05417_2021_06_11_SUHAIL` + `DSC05419_2021_06_11_SUHAIL` | Stacked two | Complete |
| 9 | `full-frame-05421-a` | `DSC05421_2021_06_11_SUHAIL` | Full frame | Complete |
| 10 | `stacked-03` | `DSC05441_2021_06_11_SUHAIL` + `DSC05382_2021_06_11_SUHAIL` | Stacked two, reusing the previously authored 05382 crop intent | Complete |
| 11 | `full-frame-05382` | `DSC05382_2021_06_11_SUHAIL` | Full frame | Complete |
| 12 | `full-frame-05372` | `DSC05372_2021_06_11_SUHAIL` | Full frame | Complete |
| 13 | `full-frame-05421-b` | `DSC05421_2021_06_11_SUHAIL` | Repeated full frame | Complete |
| 14 | `dynamic-range-01` | `DSC05250_2021_06_11_SUHAIL` + `DSC05421_2021_06_11_SUHAIL` | Dynamic Range Comparison | Complete |
| 15 | `edit-comparison-01` | `DSC05250_2021_06_11_SUHAIL` + `DSC05421_2021_06_11_SUHAIL` | Edit Comparison | Complete |
| 16–17 | `panorama-05417` | `DSC05417_2021_06_11_SUHAIL` | Continuous two-slot panorama | Complete |
| 18 | `dynamic-range-02` | `DSC05445_2021_06_11_SUHAIL` + `DSC05417_2021_06_11_SUHAIL` | Dynamic Range Comparison | Complete |
| 19 | `edit-comparison-02` | `DSC05445_2021_06_11_SUHAIL` + `DSC05417_2021_06_11_SUHAIL` | Edit Comparison | Complete |
| 20 | `full-frame-05250-repeat` | `DSC05250_2021_06_11_SUHAIL` | Repeated full frame | Complete via the existing hero render intent |

The plan has 18 editorial items and exactly 20 delivered slots. The
project-owned JSON now contains this accepted order and resolves to exactly 20
delivery frames. WSP split both continuous surfaces, the operator reviewed the
20 final HDR JPEGs, and manual Instagram publication is recorded against the
accepted post checksum.

## Red Meridian example: Threads Package A

Threads is an independent 17-item, 17-frame 4500 × 8000 (9:16) package. It
shares stable source assets and editorial intent with Instagram but owns its
templates, placement transforms, frame topology, and publication evidence.
Three-image stacks use exact no-gutter parent rows of 2667, 2666, and 2667
pixels. Comparison imagery uses contain fit in the inspected taller templates;
landscape sources letterbox and portrait sources pillarbox unless an explicit
placement transform says otherwise.

| # | Photara item | Source(s) | Layout | Status |
| ---: | --- | --- | --- | --- |
| 1 | `hero` | DSC05250 | Full frame, authored 9:16 | Complete |
| 2 | `stacked-01` | DSC05445 + DSC05442 + DSC05441 | Stacked three | Complete |
| 3 | `full-frame-05217` | DSC05217 | Full frame, authored 9:16 | Complete |
| 4 | `full-frame-05406` | DSC05406 | Full frame, authored 9:16 | Complete |
| 5 | `full-frame-05382-a` | DSC05382 | Full frame, authored 9:16 | Complete |
| 6 | `full-frame-05409` | DSC05409 | Full frame, authored 9:16 | Complete |
| 7 | `stacked-02` | DSC05417 + DSC05419 + DSC05382 | Stacked three | Complete |
| 8 | `full-frame-05421-a` | DSC05421 | Full frame, authored 9:16 | Complete |
| 9 | `stacked-03` | DSC05382 + DSC05372 + DSC05441 | Stacked three | Complete |
| 10 | `full-frame-05372` | DSC05372 | Full frame, authored 9:16 | Complete |
| 11 | `full-frame-05421-b` | DSC05421 | Repeated full frame, independently authored | Complete |
| 12 | `dynamic-range-01` | DSC05250 + DSC05421 | 9:16 Dynamic Range Comparison | Complete |
| 13 | `edit-comparison-01` | DSC05250 + DSC05421 | 9:16 Edit Comparison | Complete |
| 14 | `full-frame-05417-rotated` | DSC05417 | Rotate 90° clockwise, then crop 9:16 | Complete |
| 15 | `full-frame-05445-rotated` | DSC05445 | Rotate 90° clockwise, then crop 9:16 | Complete |
| 16 | `dynamic-range-02` | DSC05445 + DSC05417 | 9:16 Dynamic Range Comparison | Complete |
| 17 | `edit-comparison-02` | DSC05445 + DSC05417 | 9:16 Edit Comparison | Complete |

All 17 PSBs were reviewed and exported through WSP. Manual Threads publication
is recorded against the exact Threads post checksum.

## Cloudinary exact-original backup boundary

For `0.0.9`, Cloudinary stores verified backup copies of the exact WSP HDR JPEG
originals: 20 Instagram files and 17 Threads files. It does not define social
or website order and is not yet a website asset schema. Photara records local
SHA-256 and byte evidence, provider identity and URL evidence, and optional
project/post/platform/item provenance. Website derivatives, thumbnails,
presentation order, Cloudinary-specific layouts, and Loomara integration remain
deferred until the website contract exists.

The project post carries platform, caption draft, accessibility text, teaching
notes, and publication intent. Photara's social purpose is both to showcase the
finished photography and to invite viewers to learn from the process; future
caption and tool-credit structures should support that without baking prose
into the layout engine. A future branded tools slide may include Photara once
its visual identity exists.

## Illustrative post structure

```json
{
  "schema_version": 1,
  "post": {
    "project": "red-meridian",
    "name": "package-a",
    "platform": "instagram",
    "state": "draft"
  },
  "items": [
    {
      "id": "hero-01",
      "template": "full-frame@1",
      "source": {
        "asset_id": "stable-photara-asset-id",
        "sdr_rendition_id": "stable-sdr-rendition-id",
        "hdr_rendition_id": "stable-hdr-rendition-id"
      },
      "placement": {
        "mode": "fill",
        "focal_point": [0.5, 0.5]
      }
    }
  ]
}
```

Separate Instagram and Threads post specifications may share editorial intent
and source assets, but each resolves its own template versions, crops,
transforms, frame topology, immutable snapshots, and delivery evidence.
Platform differences should be explicit rather than conditional surprises
hidden inside one post.

### Sylvan `0.1.0` proof

Sylvan proved the generalized contract with 10 Instagram frames and 14 Threads
frames, including three four-image grids on each platform. Its flattened
masters, authored crops, and automatic fits remained platform-independent
inputs while Instagram and Threads retained separate post-owned transforms.
The final project specifications were reordered to match the photographer's
numbered publication files before Cloudinary and publication evidence were
recorded. Those numbered filenames are the authoritative live sequence.

## Implementation invariants

- Applications never identify layout assets by filename alone.
- Repeating an asset creates a placement, not a duplicate asset.
- A frozen or published post snapshot is immutable.
- Rendering the same frozen snapshot is deterministic and idempotent.
- Template version, post snapshot, source rendition checksums, transform
  values, UXP report, WSP report, and final output checksums remain connected.
- HDR is always above SDR in the WSP handoff document.
- Non-HDR regions are pixel-identical between the HDR and SDR composites.
- Only explicitly declared masks may contain gain-map-producing differences.
- WSP splits continuous compositions; Photara owns their intent and order.
- Temporary outputs are removed only after verified durable evidence exists.
