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

The critical path is the fastest architecture-sound route to useful native
Layout authoring: establish the general contracts that would be expensive to
retrofit, then implement only the production depth required by that vertical
slice. Bundling a package is distribution policy, not a distinct Core node
kind.

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
  graph, node instance, port, and connection IDs; add evaluation, artifact, and
  receipt IDs when their records are introduced.
- Keep package release versions, node-definition versions, value-type versions,
  and persisted schema/state versions as separate concepts.
- Define version compatibility and canonical serialization/digest rules.
- Define typed value registry, codecs, validation, cardinality, and optional
  converter registration.
- Reserve hierarchical graph paths for future subgraphs without implementing
  nested evaluation.

**Gate:** arbitrary example nodes connect or fail through general typed rules;
Core contains no Layout, source, host, destination, Adobe, or media-kind branch.

### 1A. Portable project and node-graph documents

- Define a small, versioned, human-inspectable JSON `ProjectDocument` as the
  portable authoritative project contract before persistent graphs proliferate.
- Reuse the ordinary `GraphDocument` payload so project JSON straightforwardly
  contains `nodes[]`, `connections[]`, exact package/definition pins, generic
  configuration, and generic authored state.
- Define a separate standalone `NodeGraphDocument` export using the same graph
  payload and package requirements, so users can trivially share how nodes are
  configured and connected without sharing project identity, resource
  inventory, runtime state, caches, or workspace layout.
- Keep project identity/revision, optional human metadata, exact package release
  requirements, and semantically identified project-relative resources in the
  project wrapper. Paths describe locations, never asset identity.
- Exclude secrets, credentials, absolute machine paths, environment values,
  runtime/evaluation state, progress/cancellation, caches/proxies, temporary
  artifacts, and docking/window state.
- Validate structure before acceptance, preserve unknown fields and opaque
  versioned node-owned state where practical, and use canonical serialization
  for semantic digests independently of pretty JSON whitespace.
- Keep future reusable workflow templates additive: they may derive from the
  same graph vocabulary while omitting project-specific bindings, but do not
  freeze or build a template product now.

**Gate:** media-agnostic and Layout graphs round-trip through project JSON and
standalone graph JSON without semantic loss; package/definition/schema identity
and unknown node state survive; relative paths cannot escape the project root;
and neither document requires runtime, cache, secret, or workspace UI state.

### 2. Graph commands and evaluation model

- Add graph documents, node instances, connections, configuration, authored
  state, and unknown-field preservation through the portable document contract.
- Add optimistic revisions and semantic commands suitable for undo/redo.
- Define validate, plan, evaluate/execute, progress, cancellation, retry,
  effect/idempotency, dirty propagation, and diagnostic lifecycles.
- Add deterministic evaluation keys separating inputs, environment, code,
  schema, and authored-state versions.
- Implement only the evaluator depth needed by the active vertical slice while
  keeping command, progress, cancellation, diagnostic, artifact, evidence, and
  receipt contracts general. Sophisticated scheduling, nested execution, broad
  retry orchestration, optimization, and remote execution are not `0.2.0`
  gates unless a first-party node actually requires them.

**Gate:** a CLI/test harness builds, edits, validates, serializes through the
portable project/graph contract, reloads, and evaluates a small media-agnostic
graph deterministically.

### 3. Early native-client facade spike — complete

- Compare UniFFI, a small C ABI, and IPC/XPC where isolation is valuable.
- Exercise one real Core command, one immutable DTO, request/revision identity,
  a revision conflict or other structured error, a progress stream, and
  cancellation from a minimal Swift harness.
- Avoid copying the Rust object graph or large pixel buffers into Swift.
- Treat this as an interoperability and facade-shape test, not the beginning of
  the production GUI.

**Gate:** the bridge choice and command boundary are measured and documented
before persistence, asset/proxy, and Layout APIs make the facade expensive to
change.

**Measured outcome:** a disposable Foundation/NDJSON Swift harness passed on
Quasar with macOS 26.5.2, Xcode 26.6, and Swift 6.3.3. It exercised portable
Project/Node Graph JSON, applied/rejected commands, correlated progress, and
cooperative cancellation against real Core code. UniFFI is the preferred
production in-process facade; handwritten C ABI is not justified, and IPC/XPC
is reserved for boundaries that need process isolation. The bridge remains
free of macOS 27 APIs; later SDK-27 UI experiments belong above it.

### 4A. Minimum package registry and persistence foundation

- Define a manifest/descriptor contract and package/definition registry that
  registers bundled packages through the ordinary node-package path.
- Persist the exact package release, definition identity, and definition
  version pinned by every node instance. Keep those distinct from manifest,
  configuration, authored-state, and private-state schema versions.
- Define backend-neutral Core repositories and transactional unit-of-work
  boundaries for the Layout vertical slice.
- Persist the portable Project Document as the authoritative project/graph
  representation, with backend indexes or normalized records remaining derived
  implementation details. Add clean persistence for package registrations,
  project asset references, evaluations/evidence where required, and
  namespaced node state that intentionally lives outside the portable document.
- Preserve unknown or newer state where practical so save/reopen does not erase
  data merely because the current application cannot interpret it.
- Keep credentials behind scoped host handles.
- Use the existing PostgreSQL/Storexa experience when useful, but keep the
  repository boundary backend-neutral.

**Gate:** `photara-layout` registers through the ordinary registry; Core has no
Layout dependency; a graph pins the exact Layout package/definition versions;
graph and Layout authored state save and reopen; revision-safe writes work; and
a brand-new store has no dependency on the `v0.1.0` database.

### 4B. Full package distribution lifecycle — after the first Layout Inspector

- Add remote and local-development installation, compatible update, rollback,
  disable/uninstall, retained-state lifecycle, and explicit destructive state
  deletion.
- Add dependency resolution, trust/signing, permissions, revocation,
  publishing, private distribution, and official/community store and account
  experiences.
- Preserve inexpensive Stage 4A invariants—namespaced identity, independent
  versions, declared capabilities, migration boundaries, and sensible
  missing-package behavior—so this lifecycle does not require a special path
  for bundled packages.

**Gate:** Stage 4B is not a `0.2.0` release gate. Implement an individual piece
early only when the Layout vertical slice genuinely requires it.

### 5. Asset context and representations

- Define media-general `AssetRef`, representation, capability, fingerprint,
  availability, and materialization contracts.
- Add an initial local/project asset adapter without making paths identity.

**Gate:** Core can retain asset identity across representation and path changes,
describe availability/capabilities, and materialize a local project
representation without making gallery selection an implicit graph input.

### 6. Visual proxy pipeline

- Measure thumbnail and authoring-preview backends using representative SDR,
  wide-gamut, HDR, rotated, portrait, landscape, and large sources.
- Implement content-addressed proxy caching, request deduplication, quotas,
  corruption recovery, source-change invalidation, and unmounted/remounted
  storage behavior.

**Gate:** responsive, color-described proxies can be requested through Core;
cache deletion cannot remove authored or evidentiary state.

### 7. Built-in Layout node

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

### 8. Minimal dockable macOS workspace

- Create the SwiftUI/AppKit application and document lifecycle.
- Model independently identified panels/surfaces separately from their current
  position. Workspace UI state—placement, splits, visibility, tabs, floating
  geometry, and selected preset—must not dirty project or graph state.
- Ship a useful default Layout Authoring preset, which may initially place
  Assets, Workspace/Graph, and Properties/Inspector in three regions without
  making left/center/right placement part of panel identity.
- Build the minimum project navigation, asset gallery, graph/workspace
  placeholder, real Properties/Inspector framework, diagnostics, progress,
  cancellation, accessibility, menus, keyboard focus, drag/drop, undo,
  resizing, visibility, practical rearrangement, and restoration needed by the
  first vertical slice.
- Keep the graph representation intentionally simple. Polished wires, ports,
  groups, minimaps, macros, search, final visual language, advanced tabbing,
  floating, multi-display behavior, and workspace management do not block the
  first useful Layout workflow.
- Use AppKit/Metal selectively for high-performance graph/crop/HDR surfaces.
- Keep every semantic edit as a Core command.

**Gate:** the app creates, edits, saves, closes, and reopens a graph through the
same facade exercised by Rust tests; selecting a Layout node presents its real
Inspector regardless of that panel's placement.

### 9. Production Layout inspector

- Add frame sequence editing, cell/template selection, proxy assignment,
  direct Fit/Fill/Crop controls, rotation, crop authoring, validation, and
  resolved preview. The Inspector may occupy most of a workspace or become a
  focused/detached authoring surface without changing Core commands.
- Support multiple independent Layout nodes.
- Harden unavailable storage, source replacement, stale revisions, cancellation,
  corrupted cache, and interrupted save recovery.

**Gate:** the application replaces manual layout worksheets for a real project
and is useful enough for daily authoring.

### 10. 0.2.0 stabilization

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
- Full package download, update, rollback, disable/uninstall, publishing, and
  official/community/private distribution lifecycle from Stage 4B.
- Windows-native client over the portable application facade.

## Explicitly not blocking 0.2.0

- Lightroom or Photoshop integration
- legacy database import
- publication or cloud-provider nodes
- final brand, icons, website, or marketplace
- complete graph visual language
- complete package distribution/store lifecycle
- advanced docking, floating, multi-display, and workspace management
- macros/subgraphs
- third-party loading
- Windows implementation
