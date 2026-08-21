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
