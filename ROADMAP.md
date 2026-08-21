# Roadmap to Photara 0.2.0

## Product target

`0.2.0` is the first daily-usable generation-two application, not an
incremental extension of an earlier CLI. It ships on macOS with a native
SwiftUI/AppKit interface over a portable Rust Core. Future Windows support uses
a separate native shell over the same semantic application contracts.

The product model is Houdini-like procedural authoring for creative workflows:
versioned node instances, typed ports and values, parameter and rich inspectors,
explicit evaluation, dirty propagation, caches, artifacts, receipts, and
eventual subgraphs and marketplace packages.

Layout is the first serious built-in node and drives the initial implementation.
It is not a special engine node kind. It lives in the `photara.layout`
namespace, implements the ordinary node-package contract, and is installed by
default with the application.

## Non-negotiable architecture

1. Core is portable Rust with no SwiftUI, AppKit, Windows UI, Adobe, provider,
   or database-backend types in semantic contracts.
2. Platform clients invoke a narrow versioned application facade using
   immutable DTOs, revision tokens, structured diagnostics, progress, and
   cancellation.
3. Swift owns macOS presentation and interaction. Core owns graph commands,
   validation, revisions, evaluation, and authored-state semantics.
4. Nodes are independently namespaced and versioned. Built-in and downloadable
   packages use the same definition, capability, state, and migration contracts.
5. One Core-owned state service holds shared identity, graphs, values,
   evaluations, artifacts, and receipts. Nodes receive namespaced private state,
   never ambient SQL access.
6. Configuration, authored state, input values, runtime environment, derived
   outputs, cache, and evidence remain distinct.
7. Asset identity is independent of file path and media kind. Representations
   expose capabilities and fingerprints.
8. UI context such as the gallery is never an invisible graph dependency.
9. Human-authored state and evidence are never disposable cache entries.
10. Future video, VFX, ML, and provider nodes must require no new fundamental
    evaluator variants.

## Implementation sequence

### 0. Repository foundation — complete

- Start a clean Cargo workspace.
- Establish separate Core, node SDK, store, bridge, and Layout packages.
- Retain only generation-two documentation on the active branch.

### 1. Core identity and value contracts

- Finalize namespaced package, definition, value type, schema, capability,
  graph, node instance, port, connection, evaluation, artifact, and receipt IDs.
- Define version compatibility and canonical serialization/digest rules.
- Define typed value registry, codecs, validation, cardinality, and optional
  converter registration.
- Reserve hierarchical graph paths for future subgraphs without implementing
  nested evaluation.

**Gate:** arbitrary example nodes connect or fail through general typed rules;
Core contains no Layout, source, host, destination, Adobe, or media-kind branch.

### 2. Graph commands and evaluation model

- Add graph documents, node instances, connections, configuration, authored
  state, and unknown-field preservation.
- Add optimistic revisions and semantic commands suitable for undo/redo.
- Define validate, plan, evaluate/execute, progress, cancellation, retry,
  effect/idempotency, dirty propagation, and diagnostic lifecycles.
- Add deterministic evaluation keys separating inputs, environment, code,
  schema, and authored-state versions.

**Gate:** a CLI/test harness builds, edits, validates, saves, reloads, and
evaluates a small media-agnostic graph deterministically.

### 3. Early native-client facade spike

- Compare UniFFI, a small C ABI, and IPC/XPC where isolation is valuable.
- Exercise one real Core command, one immutable DTO, a revision conflict or
  other structured error, a progress stream, and cancellation from a minimal
  Swift harness.
- Avoid copying the Rust object graph or large pixel buffers into Swift.
- Treat this as an interoperability and facade-shape test, not the beginning of
  the production GUI.

**Gate:** the bridge choice and command boundary are measured and documented
before persistence, asset/proxy, and Layout APIs make the facade expensive to
change.

### 4. Shared persistence and node installation

- Define Core repositories and transactional unit-of-work boundaries.
- Add a clean state-store schema for workspaces, node packages/installations,
  graphs/revisions, node instances, connections, typed values, evaluations,
  artifacts, receipts, and namespaced node state.
- Keep credentials behind scoped host handles.
- Implement package install, compatible update, rollback, disable/uninstall,
  retained state, and explicit destructive deletion semantics.
- Use the existing PostgreSQL/Storexa experience when useful, but keep the
  repository boundary backend-neutral.

**Gate:** a brand-new store boots without external legacy data; restart and
revision-conflict tests preserve authoritative state exactly.

### 5. Asset context and visual proxies

- Define media-general `AssetRef`, representation, capability, fingerprint,
  availability, and materialization contracts.
- Add an initial local/project asset adapter without making paths identity.
- Measure thumbnail and authoring-preview backends using representative SDR,
  wide-gamut, HDR, rotated, portrait, landscape, and large sources.
- Implement content-addressed proxy caching, request deduplication, quotas,
  corruption recovery, source-change invalidation, and unmounted/remounted
  storage behavior.

**Gate:** responsive, color-described proxies can be requested through Core;
cache deletion cannot remove authored or evidentiary state.

### 6. Built-in Layout node

- Define versioned output canvas profiles: bundled ratios and custom positive
  dimensions/aspects.
- Support arbitrary positive ordered frame counts.
- Separate frames from their cell arrangements and decorations.
- Support bundled one-cell, stacks, uniform grids, and custom normalized cells.
- Assign explicit asset references by drag/drop semantics; repeated use is
  valid.
- Support Fit (`contain`), Fill (`fill`), and user-authored Crop (`crop`) per
  cell, plus focal alignment and quarter-turn rotation.
- Resolve deterministic normalized and pixel geometry without host knowledge.
- Add rich authoring commands, validation, diagnostics, save/reopen, undo/redo,
  and exact digest behavior.
- Keep later justified, masonry, packed/mosaic, treemap, constraint, and
  aesthetic optimization strategies version-additive behind the same plan type.

**Gate:** independent 3:4 and 9:16 Layout instances can use the same assets,
retain independent crops, survive save/reopen/undo, and resolve exactly.

### 7. macOS application shell

- Create the SwiftUI/AppKit application and document lifecycle.
- Build project/workspace navigation, asset gallery, graph canvas placeholder,
  Properties/Inspector framework, diagnostics, progress, cancellation,
  accessibility, menus, keyboard focus, drag/drop, and undo integration.
- Use AppKit/Metal selectively for high-performance graph/crop/HDR surfaces.
- Keep every semantic edit as a Core command.

**Gate:** the app creates, edits, saves, closes, and reopens a graph through the
same facade exercised by Rust tests.

### 8. Production Layout inspector

- Add frame sequence editing, cell/template selection, proxy assignment,
  direct Fit/Fill/Crop controls, rotation, crop authoring, validation, and
  resolved preview.
- Support multiple independent Layout nodes.
- Harden unavailable storage, source replacement, stale revisions, cancellation,
  corrupted cache, and interrupted save recovery.

**Gate:** the application replaces manual layout worksheets for a real project
and is useful enough for daily authoring.

### 9. 0.2.0 stabilization

- Exercise several real projects and refine workflow, performance, diagnostics,
  recovery, and native interaction.
- Document install, backup, update, rollback, crash recovery, and the explicit
  supported contract.
- Choose release branding only if it is ready; code-name presentation is valid.

**Release gate:** a new user can install the Mac application, create a clean
workspace, load assets, visually author and persist useful Layout nodes, and
recover safely from interruption without external legacy state.

## After 0.2.0

- Photoshop, Lightroom Classic, Lightroom Desktop/Cloud, Cloudinary, delivery,
  metadata, ML, and other independently installable nodes.
- Selective reuse of proven historical Rust, Lua, PSJS, validation, and tests
  through the new package/host/receipt contracts.
- Explicit legacy-data importer only after new identity and evidence contracts
  can represent the source faithfully.
- Polished graph language, subgraphs/macros, third-party isolation, signing,
  permissions, marketplace, and public SDK stabilization.
- Windows-native client over the portable application facade.

## Explicitly not blocking 0.2.0

- Lightroom or Photoshop integration
- legacy database import
- publication or cloud-provider nodes
- final brand, icons, website, or marketplace
- complete graph visual language
- macros/subgraphs
- third-party loading
- Windows implementation
