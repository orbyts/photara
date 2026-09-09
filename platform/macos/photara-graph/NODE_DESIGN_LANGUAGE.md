# Photara Graph Node Design Language

This document defines the reusable visual language for Photara graph nodes.
Graph Lab is the reference implementation. These rules are presentation only:
they do not enter a Project Document, graph digest, evaluation contract, or the
future Rust bridge DTO.

## Header hierarchy

- Use a free-standing, flat outline icon. Do not add a tile, badge, glass,
  gradient, shadow, or background shape behind a first-party icon.
- Render the icon in a 28-point optical size inside a 32-point layout frame.
  Center that frame against the complete title/subtitle stack.
- Keep 9 points between the icon frame and text.
- Keep the title neutral and semibold. Keep the subtitle neutral and secondary.
  Use 1 point between the title and subtitle so they read as one identity block.
- The subtitle names the functional category, not the package provider.
- Category color belongs to the icon only. It must not color the title, node
  outline, ports, or noodle.

## Icon construction

- Author SVG source on a `24 × 24` view box.
- Use a nominal 2-point stroke with round caps and round joins.
- Prefer one recognizable silhouette and no more than two supporting shapes.
- Use `fill="none"` for the standard first-party treatment. A necessary small
  semantic mark may be solid, but decorative fills are not allowed.
- Use black strokes in source SVGs and load them as template images. SwiftUI
  supplies the adaptive category color at runtime.
- Do not embed category colors, light/dark variants, gradients, raster images,
  text, glass, shadows, or animation in first-party SVGs.
- Keep important geometry inside a roughly 2-point optical inset. Adjust for
  optical balance rather than mechanically filling the view box.

## Category and provenance

Category describes what a node does. Provenance describes who supplies it.
Built-in, Photara, Adobe, and third-party are badges or search filters—not node
categories.

| Category | Native adaptive color | Examples |
| --- | --- | --- |
| Sources & Import | system teal | Project Assets, Disk Folder, provider sources |
| Selection & Logic | system purple | Query, Union, Intersection, Match |
| Transform | system orange | Rotate, Crop, Resize, Flip |
| Application Actions | system pink | Photoshop Edit, Lightroom Action, Capture One Action |
| Layout & Composition | system blue | Layout, Composite, Mask |
| AI & Automation | system indigo | Lureva, Stable Diffusion, automation packages |
| Metadata & Organization | system yellow | Set Metadata, Keywords, Rating, Rename, XMP |
| Output & Delivery | system green | Render, Export, Disk Delivery, Cloudinary |
| Utilities & Control | system gray | Inspect, Note, Cache, Project Value |

Authoring and updating metadata are directly in Photara's scope. A node may
also make photographic or pixel adjustments, but that implementation belongs
to the node package, its registered runtime, or an external application—not to
Photara Core or the native graph UI.

Provider nodes stay in the appropriate functional category. They may use
official provider artwork and brand color only when the artwork and usage are
authorized. Do not redraw or approximate third-party trademarks. Fall back to
the category-colored Photara outline style when official artwork is unavailable.

## Runtime and performance

- Load SVG resources once as cached template `NSImage` values.
- Recolor through native template rendering; do not regenerate paths during
  node movement, pan, or zoom.
- Render icons, fonts, node dimensions, spacing, ports, strokes, and shadows at
  their final zoom-aware sizes. Do not enlarge a completed node subtree with a
  whole-view scale transform: native text and vector-backed images must be
  rasterized at the current display size so they remain sharp when zoomed in.
- Resolve presentation metadata from a package/catalog adapter keyed by exact
  node definition. Do not branch gesture, geometry, or interaction code by node
  kind.
- A missing icon must fall back to a native neutral placeholder without changing
  node layout or interaction geometry.

## Current Graph Lab specimens

Graph Lab uses Disk Folder, Rotate, and Layout as representative one-, three-,
and six-row specimens. Their IDs and port arrangements remain deterministic
test fixtures; their category and icon metadata are resolved separately from
the document so future nodes inherit the same renderer and interaction model.

## Saved starter icon set

| Resource | Intended node |
| --- | --- |
| `node-disk-folder.svg` | Disk Folder |
| `node-query.svg` | Query / Filter |
| `node-union.svg` | Union / Append |
| `node-rotate.svg` | Rotate |
| `node-layout.svg` | Layout |
| `node-metadata.svg` | Read or Set Metadata |
| `node-export.svg` | Render / Export |
| `node-ai-workflow.svg` | Generic AI workflow when no authorized provider mark exists |

Add a node-specific SVG only when its silhouette remains recognizable at the
standard optical size. Reuse an existing icon when multiple definitions express
the same action; category and exact definition metadata still distinguish them.
