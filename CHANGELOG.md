# Changelog

## Unreleased

- Reset the active repository for generation two while retaining the official
  historical release in Git.
- Establish a `0.2.0-alpha.0` Rust workspace with separate Core, node SDK,
  persistence, native-client bridge, and independently namespaced built-in
  Layout packages.
- Replace historical operator documentation with the focused roadmap to the
  macOS SwiftUI/AppKit application over the portable Rust Core.
- Refine the `0.2.0` critical path around a minimum package/persistence
  foundation, an early interoperability spike, and a flexible native workspace
  leading to the real Layout Inspector.
- Add distinct package, definition, value-type, and schema versions; generic
  typed-value and port compatibility contracts; identified graph connections;
  exact definition pins; and deterministic canonical JSON/SHA-256 digests.
- Add revision-checked semantic graph commands, exact definition resolution,
  structured validation errors/diagnostics, deterministic topological
  evaluation, progress, cooperative cancellation, and per-node dirty keys.
- Tighten in-memory compare-and-swap persistence and add immutable command,
  error, graph-snapshot, and evaluation-progress DTOs for the native bridge.
- Add validated portable Project Documents with exact package requirements,
  embedded authored graphs, project-relative resources, canonical digests, and
  explicit exclusion of runtime/cache/secret/workspace state.
- Add standalone Node Graph Documents so configured node topology can be shared
  independently of project identity and resource inventory.
- Prove the native-client facade shape with a disposable Swift 6.3/Foundation
  harness on Quasar: real applied/rejected commands, portable Project and Node
  Graph documents, correlated evaluation progress, and cooperative cancellation.
- Select UniFFI as the preferred production in-process Swift bridge direction,
  reserving IPC/XPC for isolation and keeping bridge infrastructure independent
  of macOS 27 APIs.
- Add validated exact package manifests and an ordinary package/definition
  registry used identically by bundled packages and persisted registrations.
- Replace graph-only persistence with backend-neutral whole-project and package
  manifest repositories, plus in-memory and atomic filesystem adapters.
- Prove the Stage 4A gate with the real Layout package: exact pins,
  configuration, authored and unknown state, durable save/reopen, stale-writer
  rejection, and a brand-new store with no legacy database dependency.
- Refine Stages 5–9 around project-owned assets, shared HDR/SDR-aware derived
  proxies, explicit Layout AssetSet input, semantic-free Gallery state, and a
  proxy-consuming production Inspector.
- Add portable project asset context with semantic asset/representation IDs,
  rendition roles, capabilities, SHA-256 fingerprints, project-resource
  bindings, runtime availability/materialization, and typed ordered AssetSets.
- Add a local paired HDR/SDR TIFF development adapter that preserves identity
  across path moves, detects changed bytes, refreshes fingerprints, and performs
  no Stage 6 decoding, color, proxy, or backend work.
- Add portable runtime-resolution handles for provider/external representation
  locations while keeping machine locators and configurable output placement
  outside semantic asset identity.
- Add backend-neutral proxy request, profile, descriptor, and cache-key
  contracts covering resampling, ICC/color intent, HDR/tone-map policy, bit
  depth, alpha, and exact encoder/generator revisions.
- Add a reproducible large-TIFF backend harness and record the Quasar decision:
  ImageIO/Core Image for the first macOS proxy backend, libvips retained as the
  leading portable candidate, and no default ImageMagick or incomplete
  Rust-native color path.
- Add the project-scoped production proxy service with pre-scheduling in-flight
  deduplication, explicitly bounded generation, content-addressed verified
  objects, atomic publication, quota eviction, corruption recovery, remount
  retry, and derived-only cache clearing.
- Add the process-isolated ImageIO/Core Image macOS helper for measured SDR PNG
  thumbnails and F16 HDR TIFF previews, plus a one-versus-two job RSS harness
  that establishes an initial one-generation concurrency policy independent of
  CPU count.
- Clarify that asset capabilities should describe consumer abilities across
  still image, video and audio representations, while TIFF, AVIF, EXR, ProRes
  and similar formats or containers remain representation metadata.
- Add versioned Layout authored state and deterministic plans covering bundled
  and custom canvases, ordered frames, arrangements, decorations, explicit
  asset placement, Fit/Fill/Crop, focal alignment, and quarter turns.
- Add exact inverse-producing Layout commands, structured validation and
  diagnostics, canonical digests, and an ordinary semantic node runtime driven
  solely by authored state and explicit ordered `AssetSet` input.
- Add the narrow project visual-proxy service boundary and prove that two
  independent Layouts reuse a shared derived proxy while their authored state,
  plans, crops, save/reopen, and undo remain intact after complete cache
  deletion.
- Replace the disposable bridge transport with a workspace-pinned UniFFI 0.32
  facade exposing durable project sessions, immutable project/node/asset DTOs,
  structured semantic command responses and diagnostics, evaluation progress,
  and explicit cooperative cancellation.
- Verify the generated bindings with Swift 6.3.3 and Xcode 26.6 on macOS 26.5.2,
  including create/save/reopen, stale command rejection, Layout crop and undo
  through Core commands, progress callbacks, and Swift-triggered cancellation.
- Begin the native workspace with independently identified movable panels in
  three resizable regions, a deliberately primitive node list, project-backed
  Gallery and diagnostics views, and the first Layout Inspector controls.
- Replace Layout authored-state JSON at the Swift boundary with immutable typed
  canvas/frame/cell inspection DTOs interpreted exclusively by Rust.
- Add an ordinary explicit `Project Assets` source node and atomic Core command
  batches for source/Layout connection and explicit asset-to-cell binding.
- Complete the Stage 8 vertical slice with paired project-resource TIFF import,
  create/save/close/reopen lifecycle, Project Asset Context Gallery, leased
  verified SDR/HDR proxy references, and a proxy-backed Layout preview.
- Prove from generated Swift 6 bindings on Quasar that Gallery filter,
  selection, visibility, and Inspector placement do not change graph semantics,
  while explicit binding does and survives save/reopen and evaluation.
- Begin Stage 9 with resolved normalized/pixel Layout inspection geometry,
  semantic frame/cell and Fit/Fill/Crop/focal/rotation bridge commands, and
  exact transaction-based undo/redo that includes explicit asset reassignment.
- Add a separate movable visual Layout authoring surface with independent
  frame/cell selection, shared per-cell HDR/SDR proxy references, structure and
  conventional Inspector controls, and transient crop dragging that commits a
  single Core command at the gesture boundary.
- Replace implicit launch/reopen behavior with a native Create/Open/Recent
  project launcher backed by portable project documents, filesystem storage,
  and client-only recent state—without adding a cloud database.
- Add generic node inspection DTOs for typed ports, connections, status,
  workspace/icon hints, and node-produced summaries; render Project Assets and
  Layout through one standard Inspector shell.
- Replace the engineering node list with a primitive spatial graph canvas and
  explicit optional Layout Workspace activation, while preserving movable
  project panels and adding Restore Default Workspace.
- Move node brand identity, neutral package icon resources, hierarchical
  catalog paths, search terms, Inspector contributions, and optional Workspace
  contributions onto exact versioned node definitions; expose the visible
  catalog through immutable UniFFI DTOs instead of Swift node-type switches.
- Add the ordinary independently branded `photara.disk.folder` package. Its
  portable state retains only a stable folder-binding identity, scan policy,
  and accepted `AssetSet`; macOS paths and security-scoped bookmarks remain
  device-only runtime bindings.
- Add explicit Disk scan/reconciliation, stable file-derived asset and
  representation identities, runtime-resolved proxy materialization, and typed
  Disk Inspector controls for folder grant/rebind, scan, and explicit Layout
  connection. Reopen preserves semantics even when the device grant is absent.
- Add a native Quick Look fast path for responsive local Gallery/Layout images,
  move Disk fingerprinting and verified proxy generation off the main actor,
  and use a 1K-default F16 HDR-preserving Layout authoring profile with
  constrained HDR presentation.
- Add definition-owned default activation metadata: double-clicking Disk opens
  its granted Finder folder, while Layout focuses its existing Workspace.
- Make bare `Tab` reliably open the node catalog at the graph pointer, with the
  graph center as its initial fallback; retain the toolbar add button.
- Open Gallery assets in the user's macOS default application on double-click,
  while keeping cell assignment explicit.
- Reconcile provider membership atomically and clear a Disk node's old assets
  immediately after a successful folder rebind, retaining unrelated project
  assets while the new scan runs.
- Refine future persistence into explicit portable-project,
  user-plus-definition sync, project-plus-instance runtime, and device-only
  scopes behind one Core-owned state service rather than per-node databases.
- Publish Disk assets after a cheap metadata-only discovery pass, distinguish
  observed from content-verified representation revisions, and verify bytes at
  utility priority without blocking first Gallery visibility.
- Progress Gallery through Quick Look's low-quality and full native thumbnails,
  retain an older image while refreshing, expose loading/updating state, and
  allow the shared production proxy fallback only after byte verification.
- Formalize a 256–512 px Gallery-first tiny tier and a 1K-default,
  device-configurable color-described HDR-capable Layout authoring tier, with
  the tiny/stale image retained until the bounded authoring request completes.
- Route giant TIFF Gallery previews through the bounded 384 px F16
  `ImageIO`/Core Image path after Quick Look exceeded 60 seconds on a
  representative 761 MiB source; the same source measured 2.91 seconds through
  the selected helper.
- Revalidate cheap file observations for disposable preview generation and
  postpone whole-folder SHA-256 work until initially requested previews finish,
  preventing verification from blocking first pixels.
- Stabilize Gallery geometry with fixed preview slots so progressive images do
  not resize or overlap neighboring rows, and add separate photo-first and
  filename-bearing detail grids with smaller metadata typography.
- Expand Disk still-image discovery across common camera RAW, TIFF, JPEG, PNG,
  HEIF, AVIF, JPEG XL, PSD/PSB, EXR, WebP, GIF, and BMP extensions while
  keeping decoder availability a runtime capability rather than a Core rule.
- Measure four simultaneous 384 px HDR TIFF jobs on Quasar at about 2.6 seconds
  and 733 MiB peak footprint each, then select an explicit memory-tiered
  generation limit of 4 jobs on 64 GiB-plus machines, 2 on 24 GiB-plus, and 1
  below 24 GiB. The limit remains device-configurable and capped at four.
