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

### 4A. Minimum package registry and persistence foundation — complete

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

**Measured outcome:** exact package manifests validate, persist, and rebuild the
ordinary definition registry after reopen. The portable Project Document is the
single authoritative project/graph aggregate behind backend-neutral create,
load, and revision-checked replace operations. A minimal filesystem adapter
uses synchronized temporary files and atomic publication/replacement, while an
in-memory adapter supports tests and short-lived services. The real Layout
package, exact node pins, configuration, authored state, and unknown future
state round-trip through a brand-new store. No legacy database, normalized
graph copy, Stage 4B lifecycle, credential store, or runtime/evidence
persistence was introduced.

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

### 5. Project asset context and representations — complete

- Define media-general semantic asset identity independently of any file,
  provider, host application, media kind, or current location.
- Let one asset expose multiple independently identified related
  representations/renditions. In particular, paired HDR and SDR flattened TIFFs
  are two representations of one asset, not two unrelated assets.
- Define representation capabilities, immutable content fingerprints,
  availability, locator/binding separation, and explicit materialization
  requests/results. Moving a representation must not change asset or
  representation identity; changing its content must produce a new fingerprint.
- Add project-owned asset context with explicit lookup and `AssetSet` values.
  Asset ordering and membership used by a graph are declared typed input, while
  transient Gallery selection remains client state.
- Add a minimal local/project adapter for development fixtures and imported
  paired HDR/SDR flattened TIFFs. These stand in for representations that future
  Photoshop, Lightroom, Lureva, cloud, or other upstream nodes may produce.
- Keep every provider/host concept outside Core. Future upstream nodes create or
  replace ordinary assets/representations through the same contracts.

**Gate:** Core retains asset and representation identity across path changes,
represents paired HDR/SDR renditions under one asset, describes capabilities and
availability, detects changed content through fingerprints, materializes an
available project-local representation, and round-trips an explicit ordered
`AssetSet` without Gallery selection or Photoshop-specific state.

**Measured outcome:** the portable Project Document now owns semantic assets and
multiple independently identified representations with roles, capabilities,
SHA-256 content fingerprints, and portable project-resource or stable
runtime-resolution bindings. Availability, provider/machine locators, and
verified local materialization remain runtime-only. Output storage policy is
separate from identity and binding: placing layered PSBs beside RAWs and
flattened TIFFs in the project is a useful default, never a Core requirement.
The exact ordered
`photara.asset-set` typed value validates project membership and is used by
Layout's declared input port. A development adapter imports paired HDR/SDR TIFF
paths as one asset, preserves identities and fingerprints across path moves,
detects changed bytes, and refreshes the fingerprint without decoding TIFF or
generating proxies. Project JSON and the Stage 4A store round-trip this context;
proxy/cache/UI/provider concepts remain absent.

### 6. HDR/SDR-aware project proxy infrastructure — complete

#### 6A. Contracts and measured backend decision — complete

- Define backend-neutral project proxy request, profile, exact-profile,
  descriptor, and content-addressed cache-key contracts before selecting an
  imaging implementation.
- Key proxy identity by source representation fingerprint and every
  output-affecting profile input: sizing, resampling, orientation, color/intent,
  HDR/tone-map policy, depth, alpha, exact encoding recipe, and generator
  revision. Project, asset, representation, request, and consumer identity do
  not affect derived bytes.
- Benchmark ImageIO/Core Image, libvips, ImageMagick, and a viable Rust-native
  path against exact profiles and representative TIFFs for ICC/wide gamut,
  HDR/SDR, depth, orientation, memory, throughput, deployment, and portability.
- Record the fixture corpus, raw medians, correctness results, deployment
  assessment, and selected backend before production generation.

**Measured outcome:** Quasar benchmarks used deterministic 8000×5333,
high-entropy 179 MiB Display-P3 U16 SDR and 302 MiB linear-ACEScg F32 HDR TIFFs,
plus an orientation-6 fixture. The optimized ImageIO float-thumbnail/Core Image
path passed ICC, wide-gamut, orientation, F16, negative-value, and HDR-headroom
checks at median 0.45 s / 475 MiB for a 512 px SDR thumbnail and 1.09 s / 954 MiB
for a 2048 px HDR preview. It is selected for the first macOS backend and uses
no macOS 27 APIs. libvips remains the leading future portable/Windows candidate;
ImageMagick is not the default; the measured Rust `image` path failed the exact
color/HDR contract. Full results are in `docs/architecture/PROXIES.md`.

#### 6B. Production project proxy service — complete

- Build a project-scoped proxy service shared by nodes and UI consumers. Layout
  and Gallery request proxies; neither owns proxy generation or cache policy.
- Generate reusable color-described thumbnail and authoring-preview proxies
  from explicit source representations. Proxies are derived/cache data and
  never authoritative Project Document or node-authored state.
- Add request deduplication, content-addressed storage, atomic publication,
  descriptor verification, quotas, corruption recovery, and
  unmounted/remounted-source behavior behind the backend-neutral contracts.
- Deduplicate identical in-flight requests before they wait for deliberately
  bounded generation capacity. Set the initial bound from measured decoder
  memory and responsiveness rather than logical CPU count.
- Keep ImageIO/Core Image in the macOS adapter. No platform image, color, EDR,
  filesystem, or cache-backend object enters Core.

**Gate:** the measured backend decision is recorded; shared project services
produce responsive, color/HDR-described proxies whose cache identity changes
with source fingerprint or proxy/color policy; multiple Layout/UI consumers
reuse them; and deleting the cache cannot remove project, authored, or
evidentiary state.

**Measured outcome:** a new runtime-only `photara-proxy` crate provides one
project-scoped service over the Stage 6A contracts. It verifies content-addressed
objects and descriptors on every hit, publishes synchronized staging directories
atomically, evicts least-recently-used derived entries to a byte quota, removes
corrupt entries, and retries unavailable sources after remount. In-flight
deduplication precedes the generation limiter: eight concurrent identical test
requests materialized and generated once while six distinct requests never
exceeded a configured bound of two. The first macOS adapter runs the measured
ImageIO/Core Image path in a short-lived helper process built with Xcode 26.6
and no macOS 27 API. On the 42.7 MP HDR fixture, sampled aggregate RSS scaled
from a 658 MiB median for one helper to 1,316 MiB for two; the isolated Stage 6A
peak remains the more conservative 954 MiB per-job sizing datum. The initial
production default is therefore one generation at a time, explicitly
configurable and intentionally not derived from CPU count.

### 7. Built-in Layout node — complete

- Consume an explicit ordered `AssetSet` typed input. Layout never reads Gallery
  selection or ambient project UI context.
- Resolve visual source representations and proxies through project services.
  Layout does not generate or own proxies and is indifferent to whether an
  asset originated on disk, in Photoshop, Lightroom, Lureva, cloud storage, or
  another upstream node.
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

**Gate:** independent 3:4 and 9:16 Layout instances consume explicit AssetSets,
reuse project-scoped proxies for the same representations, retain independent
crops, survive save/reopen/undo, and resolve exactly without ambient Gallery or
provider-specific dependencies.

**Measured outcome:** `photara-layout-node` now owns a versioned authored-state
model for bundled and custom canvases, arbitrary ordered frames, independent
arrangements/decorations, one-cell/stacks/grids/custom normalized cells,
explicit repeated asset placement, Fit/Fill/Crop, focal alignment, and
quarter-turn rotation. Fixed-point normalized geometry resolves deterministically
to pixels and produces canonical state and plan digests. Semantic commands
return exact inverses and reject invalid state atomically. The ordinary node
runtime evaluates only authored state plus its explicit ordered `AssetSet` and
performs no I/O. A separate runtime-only project-service request obtains one
proxy per distinct placed asset; neither proxy descriptors nor cache paths enter
authored state or the semantic plan. The gate test gives independent 3:4 and
9:16 Layout nodes different crops, observes one generation plus one shared cache
hit, deletes the complete proxy cache, and reopens both Layouts byte-for-byte
from the portable project store.

### 8. Minimal dockable macOS workspace — complete

#### 8A. Production native facade — complete

- Replace the disposable NDJSON spike with a workspace-pinned UniFFI library
  facade before substantial SwiftUI work.
- Expose project create/open/save, immutable project/node/asset snapshots,
  structured diagnostics and command rejection, evaluation progress, and
  explicit cancellation handles without exposing Rust repositories or graph
  objects.
- Translate Layout authoring intent to exact Layout commands and commit the
  resulting authored state through revision-checked Core commands. Undo and
  redo must use the same Core command path.
- Compile and run the generated Swift bindings on Quasar with macOS 26.5.2,
  Xcode 26.6, and Swift 6.3.3. The facade has no SwiftUI/AppKit or macOS 27
  dependency.

**Measured outcome:** workspace-pinned UniFFI 0.32 generates Swift 6 bindings
from the real `photara-bridge` dynamic library. The Quasar verification creates,
inspects, edits, saves, and reopens a portable project; observes immutable,
typed Layout inspection DTOs; rejects a stale revision with a structured
diagnostic; applies
and undoes a crop through Core `SetAuthoredState` commands; streams evaluation
progress; and honors Swift-triggered cancellation. It also moves the Inspector,
hides Gallery, preserves node selection, and verifies that the graph digest is
unchanged.

#### 8B. First workspace vertical slice — complete

- Create the SwiftUI/AppKit application and document lifecycle.
- Model independently identified panels/surfaces separately from their current
  position. Workspace UI state—placement, splits, visibility, tabs, floating
  geometry, and selected preset—must not dirty project or graph state.
- Ship a useful default Layout Authoring preset, which may initially place
  Assets, Workspace/Graph, and Properties/Inspector in three regions without
  making left/center/right placement part of panel identity.
- Build only the project lifecycle, project-owned asset Gallery, primitive
  graph list, typed Layout Inspector, diagnostics, progress/cancellation,
  menus, resizing, visibility, movement, and restoration required by the first
  vertical slice. Defer polished docking, graph editing, and Stage 9 controls.
- Keep the graph representation intentionally simple. Polished wires, ports,
  groups, minimaps, macros, search, final visual language, advanced tabbing,
  floating, multi-display behavior, and workspace management do not block the
  first useful Layout workflow.
- Use AppKit/Metal selectively for high-performance graph/crop/HDR surfaces.
- Keep every semantic edit as a Core command.
- Keep Rust as the sole interpreter of opaque node-authored state. Native
  clients consume immutable presentation DTOs, never Layout's persisted JSON.
- Import selected local HDR/SDR TIFFs into project-relative resources as a
  development stand-in for future upstream nodes. Bind them through an explicit
  ordinary `AssetSet` source and revision-checked Core command.
- Return leased, verified proxy file references and backend-neutral descriptors
  across the facade, preserving color space, ICC identity when present,
  dynamic range, depth, orientation, and content fingerprints. Never move image
  pixel buffers through UniFFI.
- Treat Gallery strictly as a view over project-owned asset context and proxy
  services. Closing, docking, moving, filtering, or selecting in Gallery changes
  no project or graph semantics; drag/drop creates explicit commands or typed
  AssetSet bindings.

**Gate:** create/open/save/close/reopen a real project; import and explicitly
bind assets; display project Asset Context through shared proxies; inspect a
selected Layout through typed DTOs regardless of Inspector placement; obtain a
proxy-backed Layout preview; and prove that moving, closing, filtering, or
selecting Gallery changes neither project semantics nor graph digest unless an
explicit Core command is issued.

**Measured outcome:** the production UniFFI facade no longer exposes Layout
authored-state JSON. Rust validates and converts it into immutable canvas,
frame, cell, placement, crop/focal, rotation, and digest DTOs. A new ordinary
`Project Assets` source node produces the explicit ordered `AssetSet`; one
atomic Core batch creates source + Layout + connection. A project-level Core
command publishes a fingerprinted import, and another explicit graph batch adds
its identity to the `AssetSet` and assigns it to a Layout cell. The macOS shell
creates, closes, and reopens projects, copies paired local
TIFFs into project-relative resources, populates Gallery solely from Project
Asset Context, and holds Gallery selection/filtering in workspace state. Gallery
and Layout request the same bounded project proxy service and receive leased
verified file references with SDR/HDR and color descriptors rather than pixel
buffers. The Swift 6.3.3 Quasar harness generates real TIFFs and proves import,
binding, SDR thumbnail, HDR authoring preview, connected graph evaluation,
save/reopen, typed inspection, Inspector movement, Gallery hide/filter/select
digest invariance, and explicit-command-only semantic change using Xcode 26.6
without macOS 27 APIs.

### 9. Production visual Layout authoring — in progress

- Keep conventional Inspector controls and the visual Layout authoring surface
  as separate presentation concepts. The authoring surface may occupy most of
  a workspace or later become focused/detached; placement and size never enter
  Core semantics.
- Make asset assignment/reassignment, frame/cell structure changes,
  Fit/Fill/Crop, focal alignment, quarter-turn rotation, and crop commits
  revision-checked semantic Core commands with coherent undo/redo. A native
  gesture may keep transient presentation state while active, but commits one
  intentional command at its boundary rather than sending pointer movement.
- Build on the existing Layout semantics and explicit `AssetSet` input before
  expanding templates. Keep resolved geometry deterministic, authoritative
  state proxy-free, and Rust solely responsible for interpreting authored
  state into immutable presentation DTOs.
- Request thumbnails and authoring previews from the shared project proxy
  service, preserving HDR/SDR/color descriptions through preview selection.
- Support multiple independent Layout nodes.
- Harden unavailable storage, source replacement, stale revisions, cancellation,
  corrupted cache, and interrupted save recovery.

**Gate:** a real project can be authored visually without the old manual Layout
worksheet/crop-authoring process, including multiple independent Layout nodes,
explicit asset placement, frame/cell editing, Fit/Fill/Crop, rotation, crop
authoring, validation, resolved proxy-backed preview, save/reopen, and coherent
undo/redo.

**Current slice:** the facade exposes resolved normalized/pixel cell geometry
and semantic cell/structure edit enums. Exact forward/reverse Core graph
transactions make assignment, structure, content mode, focal/alignment, crop,
and rotation undoable. The macOS shell now has a separate movable Layout
authoring surface, explicit frame/cell selection, shared per-cell proxies,
frame/cell controls, nine-point alignment/focal controls, rotation, crop sizing,
and transient crop dragging that commits once on gesture end. Stage 9 remains
open for real-fixture workflow hardening against the complete gate. The
generated Swift gate now exercises assignment undo/redo, resolved geometry,
cell insertion/arrangement, focal Fill, rotation undo/redo, and two independent
Layout nodes through the production facade on Quasar.

The native first-look refinement now separates Graph, standard Inspector,
optional node Workspace, and project panels. Launch presents Create/Open/Recent
with native-only recent state and no database. The facade provides generic
typed port/connection/status/summary DTOs; Swift renders Project Assets and
Layout through one Inspector shell. A primitive spatial graph replaces the
engineering list, Layout Workspace opens explicitly, and Workspace → Restore
Default Workspace recovers presentation state without semantic mutation.

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
