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

### Extension and boundary-drift guardrail

The architecture is successful only if a materially new node normally arrives
as an exact package definition plus a host-registered runtime, declared typed
ports/values, scoped capabilities, and optional Inspector/Workspace
contributions. Adding a provider, editor, automation, compute, or future
scripting node must not require a new Core node kind, definition-ID switch in
the bridge, platform semantics in the Project Document, ambient database or
filesystem access, or a rewrite of the native application shell.

Before accepting a feature or bug fix that crosses an existing boundary, apply
this extension test:

> Could an independently versioned package add this behavior through the
> existing semantic contracts, exact runtime registration, scoped host
> services, and optional presentation contributions?

If the answer is no, stop and identify the missing general contract. Extend the
narrowest owning layer deliberately and compatibly; do not make the active
first-party node a privileged special case. This is a design review trigger,
not permission to prebuild speculative subsystems. The current stage gate
remains the priority, and future depth is implemented only when a concrete
vertical slice requires it.

Executable/code nodes are a future example of this rule. They remain ordinary
node packages over the existing evaluation, progress, cancellation, typed
value, diagnostic, and exact-runtime boundaries. A future scripting host must
add explicit language/runtime identity, implementation fingerprints, resource
limits, isolation, and scoped filesystem/network/secret/project capabilities;
it must never grant ambient process, SQL, credential, or machine access. That
is after `0.2.0` and does not justify implementing a scripting engine now.

Diagnostics follow the same ownership rule. Validation and execution
diagnostics are structured and node-addressable. Runtime diagnostic history is
runtime state keyed by evaluation identity, source graph revision, and node;
it does not enter authored Project Documents or graph digests. Native clients
render those diagnostics in the selected node's Inspector and other
presentation surfaces without becoming their semantic owner. Optional future
source spans, recovery actions, and log references must extend the diagnostic
contract compatibly.

### Future asset discovery and query guardrail

Photara must eventually support Lightroom/Capture One-style smart collections,
provider selections, and very large local, NAS, catalog, or cloud archives
without turning filenames into asset identity or copying an archive listing
into a Project Document. This is an after-`0.2.0` product capability, but work
before then must preserve the following route:

- Provider/source nodes expose semantic assets and normalized portable metadata
  where available. Standard fields such as capture time, camera, lens, rating,
  label, keywords, dimensions, media kind, and representation capabilities use
  typed common vocabulary; provider-specific fields remain namespaced.
- Selection-import nodes may emit external selection evidence such as Pixieset
  CSV keys. A separate explicit match/join node resolves that evidence against
  an upstream `AssetSet`; filenames and provider keys are matching evidence,
  never universal asset identity.
- Query/filter authored state is a portable typed expression, not SQL or a
  provider API payload. Intersection/AND, union/OR, difference/NOT, symmetric
  difference/XOR, query, sort, group, and match remain ordinary independently
  versioned node packages over typed ports.
- Runtime query planning pushes supported predicates into the source provider
  or a Core-owned derived index before materializing results. Unsupported
  predicates may be evaluated by Photara after provider filtering. Results are
  incremental, cancellable, paged, and capable of reporting partial/updating
  status; the bridge and Gallery must not require one enormous in-memory array.
- Local/NAS metadata indexes are rebuildable device/runtime data, logically
  scoped by provider/source through the Core-owned state service. They are not
  authoritative project state and generally do not sync. Saved queries and
  user preferences may later sync through explicit user-scoped services.
- Selecting a graph node may make Gallery present a visualizable `AssetSet`
  output from that node. Gallery exposes an explicit scope choice between that
  selected-node result and the entire Project Asset Context, then allows a
  disposable quick filter or metadata-expression filter inside the chosen
  scope. These are native workspace viewing lenses only: graph selection,
  Gallery scope/filter expressions, and Gallery item selection never change the
  Project Document or graph digest. Connecting, assigning, capturing, or
  promoting the expression to a Query node requires an explicit revision-checked
  Core command.

Do not implement the generalized index/query subsystem for `0.2.0`. The current
Disk and Gallery work should retain stable asset identity, incremental discovery,
virtualized presentation, cancellation, and explicit semantic commands so the
future capability does not require replacing their ownership boundaries.

### Future project-variable guardrail

Projects may eventually define typed, named values for reuse by node
configuration and portable expressions. A reference such as
`$PROJECT_PATH/masters` expresses a location relative to the currently opened
project; it must never serialize one machine's absolute project path or grant a
node ambient filesystem access. The exact expression syntax is not frozen by
this roadmap, but the ownership rules are:

- Project variables are authored Core state with stable names, declared types,
  validation, revision-checked commands, undo/redo, deterministic resolution,
  and appropriate dirty propagation. They are distinct from process environment
  variables, user preferences, node-private state, and runtime provider data.
- Read-only built-ins such as project identity or project-relative location are
  supplied by the application host through explicit portable evaluation
  context. Device paths, security-scoped bookmarks, credentials, and mounts
  remain runtime bindings behind scoped capabilities.
- User-defined variables may hold portable scalar/configuration values and
  relative resource references. Secrets and credentials use host secret handles,
  never Project Document values or expression interpolation.
- Unknown variables, type mismatches, cycles, unavailable bindings, and attempts
  to escape an authorized root produce structured diagnostics rather than
  falling back to ambient machine state.
- Native clients may provide a project-variable editor, but Core remains the
  parser, validator, resolver, and semantic owner. Windows and macOS resolve the
  same portable references through their own scoped host adapters.

Do not build a general expression language or variable editor during the current
Graph visual slice. Preserve generic node configuration and evaluation context
so the smallest concrete cross-node workflow can introduce this contract later
without adding path substitution to Swift or individual node packages.

### Planned node catalog

This catalog is the product inventory for the node browser, not a promise that
every listed node ships in `0.2.0`. The implementation sequence and its gates
remain authoritative for release timing. A node's **category** describes what it
does; its **provenance** describes who supplies it. “Built-in,” “Photara,”
“Adobe,” and “third-party” are therefore badges or filters, not categories.

The Tab node browser groups nodes by functional category and searches titles,
aliases, providers, and keywords. First-party nodes use a category-colored icon
with a neutral title. Provider nodes keep a neutral title and may use the
provider's recognizable icon color. Exact colors are adaptive native UI theme
roles and presentation preferences; they are not serialized graph semantics.
The normative icon construction, header sizing, and provenance rules live in
[`platform/macos/photara-graph/NODE_DESIGN_LANGUAGE.md`](platform/macos/photara-graph/NODE_DESIGN_LANGUAGE.md).

| Functional category | Native icon role | Intended contents |
| --- | --- | --- |
| Sources & Import | teal | Bring assets, selections, or external records into a graph. |
| Selection & Logic | purple | Filter, combine, compare, order, and group asset sets. |
| Transform | orange | Change geometry, orientation, dimensions, or representation. |
| Application Actions | pink | Invoke or exchange work with external creative applications. |
| Layout & Composition | blue | Arrange, mask, and composite visual content. |
| AI & Automation | indigo | Learned, generative, and automated workflow operations. |
| Metadata & Organization | yellow | Read or author descriptive and organizational data. |
| Output & Delivery | green | Render, export, publish, or deliver results. |
| Utilities & Control | gray | Inspect, annotate, route, cache, or control a workflow. |

#### Bundled first-party nodes

These use the same package and runtime contracts as downloadable nodes. “Now”
means the node is already part of the active vertical slice; “planned” records
an intended product node; “candidate” preserves a useful direction whose exact
definition and port contract still require a concrete workflow.

| Node | Category | Purpose | Horizon |
| --- | --- | --- | --- |
| Project Assets | Sources & Import | Emit the project's explicit ordered asset context. | Now |
| Disk Folder (`photara.disk.folder`) | Sources & Import | Discover assets from an authorized local or NAS folder incrementally. | Now |
| Query / Filter | Selection & Logic | Evaluate a portable typed metadata expression over an `AssetSet`. | Planned after `0.2.0` |
| Union / Append | Selection & Logic | Combine asset sets while defining stable identity and ordering behavior. | Planned after `0.2.0` |
| Intersection | Selection & Logic | Keep assets present in every input set. | Planned after `0.2.0` |
| Difference | Selection & Logic | Remove assets found in another set. | Planned after `0.2.0` |
| Symmetric Difference / XOR | Selection & Logic | Keep assets present in only one of two sets. | Planned after `0.2.0` |
| Match / Join | Selection & Logic | Resolve external selection evidence against semantic asset identity. | Planned after `0.2.0` |
| Sort | Selection & Logic | Produce a deterministically ordered asset set from typed keys. | Planned after `0.2.0` |
| Group | Selection & Logic | Partition assets by typed metadata keys without changing identity. | Planned after `0.2.0` |
| Rotate | Transform | Apply a non-destructive orientation transform. | Candidate |
| Crop | Transform | Apply a non-destructive crop independent of Layout cell framing. | Candidate |
| Resize / Scale | Transform | Produce a representation with explicit target dimensions or scale. | Candidate |
| Flip / Orientation | Transform | Apply explicit horizontal, vertical, or normalized orientation. | Candidate |
| Layout (`photara.layout`) | Layout & Composition | Resolve ordered assets into authored pages, cells, and output geometry. | Now; production UI in progress |
| Composite | Layout & Composition | Combine typed visual inputs using explicit order and blend semantics. | Candidate |
| Mask | Layout & Composition | Supply or apply a typed mask without implicit UI state. | Candidate |
| Read / Inspect Metadata | Metadata & Organization | Expose normalized and namespaced metadata for inspection or downstream logic. | Candidate |
| Set Metadata | Metadata & Organization | Author explicit metadata changes with receipts and diagnostics. | Planned after `0.2.0` |
| Keywords | Metadata & Organization | Add, remove, or normalize keywords as authored metadata operations. | Candidate |
| Rating / Color Label | Metadata & Organization | Apply portable rating and label changes where providers support them. | Candidate |
| Rename | Metadata & Organization | Plan deterministic representation renames without changing asset identity. | Candidate |
| XMP Sidecar Read / Write | Metadata & Organization | Exchange supported develop and metadata values through explicit sidecar artifacts. | Candidate |
| Render / Export | Output & Delivery | Materialize a typed result using an explicit format and destination contract. | Required boundary for `0.2.0`; exact node split TBD |
| Disk Delivery | Output & Delivery | Deliver rendered artifacts into an authorized filesystem destination. | Candidate pending the `0.2.0` export decision |
| Inspect / Preview | Utilities & Control | Surface a value, artifact, or diagnostic without changing graph semantics. | Candidate |
| Note | Utilities & Control | Add human-authored graph documentation with no evaluation behavior. | Candidate |
| Cache | Utilities & Control | Declare an explicit reusable evaluation boundary without making cache authoritative. | Candidate |
| Project Value | Utilities & Control | Emit a typed project variable once the Core-owned variable contract exists. | Planned after `0.2.0` |

Photara is a workflow automation and orchestration tool, not a replacement raw
developer or pixel editor. Authoring and updating metadata remains directly in
scope for Photara and its bundled nodes. The first-party Transform nodes above
cover narrow, deterministic workflow operations such as geometry and output
preparation; they do not imply a native Lightroom-, Capture One-, or
Photoshop-style adjustment engine.

A node may make photographic or pixel adjustments. That behavior belongs to
the independently versioned node package, its registered runtime, or an
external application/service—not to Photara Core or the native graph UI. Core
orchestrates typed inputs and outputs, capabilities, evaluation, artifacts,
diagnostics, provenance, and durable receipts without interpreting the
adjustment itself. This allows Lightroom, Capture One, Photoshop, Lureva,
Stable Diffusion, and future first- or third-party adjustment nodes to
participate without turning Photara into an editor.

Knots/reroutes are graph-routing primitives rather than evaluation nodes. They
belong to the shared graph interaction model and remain distinct from the
Utilities & Control packages above. Likewise, Graph Lab's current Source,
Transform, and Composite specimen labels exercise one-, three-, and six-port
presentation and interaction; they do not register product nodes by themselves.

#### Planned provider and extension nodes

Provider packages appear in the same functional categories as first-party
nodes. A package may contribute more than one node when source, edit, and
delivery operations have different capabilities or side effects.

| Provider or extension | Functional placement | Intended nodes |
| --- | --- | --- |
| Adobe Lightroom Classic | Sources & Import; Application Actions | Catalog/collection source, selection exchange, and explicit develop/XMP round trips performed by Lightroom. |
| Adobe Lightroom Desktop/Cloud | Sources & Import; Application Actions | Cloud-library source and supported Lightroom actions behind scoped authorization. |
| Adobe Photoshop | Application Actions | External edit with explicit input/output artifacts and receipts. |
| Capture One | Sources & Import; Application Actions | Catalog/session source, selection exchange, and supported Capture One actions. |
| Pixieset | Sources & Import | CSV selection import whose evidence is resolved by a separate Match / Join node. |
| Lureva | AI & Automation | Learn from editing examples and create or update XMP sidecars; optionally apply them to supplied DNG representations. |
| Stable Diffusion | AI & Automation | Generative or image-to-image operations with explicit model, prompt, seed, and artifact provenance. |
| Cloudinary | Output & Delivery | Upload and delivery operations with scoped credentials and durable receipts. |
| Dropbox, Google Drive, Box, and iCloud/File Provider | Sources & Import; Output & Delivery | Independently branded source and destination nodes over ordinary semantic assets. |
| Photos / PhotoKit and studio DAMs | Sources & Import | Authorized provider sources with stable provider identity and scoped materialization. |

Before implementation, each catalog entry still needs an exact package and
definition identity, typed ports, authored configuration/state boundary,
capabilities, deterministic behavior, diagnostics, and migration policy. New
ideas should be added to this inventory first, then promoted into the staged
implementation plan only when a concrete vertical slice owns them.

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

**Internal architecture checkpoint:** preserve the current Stage 9 behavior
while making ownership explicit before further substantial UI iteration. The
application host maps exact installed definition coordinates to runtimes;
`photara-disk` owns authorized-folder enumeration, revision observation,
fingerprinting, and reconciliation preparation; `photara-layout` remains
semantic and proxy-agnostic; and the production bridge/native shell are split
along evaluation, materialization, Gallery state, Gallery behavior, and Gallery
view surfaces. Portable schemas, project compatibility, proxy contracts, and
the production UniFFI facade remain unchanged.

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
- Keep local-file presentation responsive with a runtime-only native thumbnail
  fast path. Layout replaces its placeholder with a small verified
  HDR-preserving interaction proxy; preview resolution never limits normalized
  authored crop geometry.
- Publish provider assets after cheap discovery, progressively retain/show the
  best available preview, and verify source bytes in the background. Revision
  evidence must distinguish provider/file observations from verified content;
  preview freshness and progress remain runtime presentation state.
- Support multiple independent Layout nodes.
- Harden unavailable storage, source replacement, stale revisions, cancellation,
  corrupted cache, and interrupted save recovery.
- Expose an immutable available-definition catalog through the application
  facade. Exact package definitions contribute hierarchical category paths,
  search metadata, independent brand identity, package-owned neutral icon
  resources, generic Inspector contribution hints, and optional rich Workspace
  capabilities without becoming Core evaluator variants. Native clients resolve
  those resources into platform skins rather than hard-coding definition
  identities. Graph's
  `Tab` interaction renders this catalog rather than hard-coding node cases in
  Swift.
- Let each exact definition advertise a neutral default activation. The first
  macOS mappings open Disk's granted folder in Finder and focus Layout's
  existing authoring Workspace without changing project semantics.
- Add `photara.disk.folder` as the second ordinary bundled node and first
  live-data source. It emits explicit `photara.asset-set`, uses a stable portable
  folder-binding identity, and keeps macOS security-scoped bookmarks, absolute
  paths, availability, and materialized locations in native/runtime binding
  state. Disk receives a compact custom Inspector contribution but no canvas
  Workspace.
- Exercise Stage 9 against a real authorized folder and real project data.
  Folder scanning/reconciliation must publish asset-context and node-membership
  changes through an explicit revision-checked Core command; it cannot mutate
  project semantics merely because a directory changed.
- Iterate native density, border, material, background, shape, typography, and
  interaction using measured visual references while preserving independent
  panel identity and optional node Workspaces.
- Keep UI palette iteration behind one portable semantic theme document with
  paired Light/Dark sRGB values. The bounded developer Theme Lab must compile
  the same parser/resolver and production native views as Photara against an
  isolated fixture, validate/export the document, and live-reload only native
  presentation state. Do not maintain a second hand-authored UI preview.
  Theme choice and node color roles never enter Core, a Project Document, or a
  graph digest; node catalog taxonomy remains independent from color role.

#### Remaining Stage 9 UI sequence

Work through these slices in order. Each slice is a usable first draft whose
architecture can support later refinement; `0.2.0` does not require the final
marketplace-era visual language.

1. **Graph nodes and wiring.** Complete a working first draft of spatial nodes,
   typed input/output ports, connection creation/removal, selection, movement,
   camera interaction, and readable connection feedback. Derive node height and
   port rows from definition-provided typed ports rather than hard-coded node
   kinds. Keep node movement and graph edits distinct: transient drag is native
   presentation state and commits an intentional Core command at the gesture
   boundary. Preserve revision checking, undo/redo, graph zoom, minimap, theme,
   and the package catalog. Advanced routing, groups, macros, and final graph
   polish remain after `0.2.0`.
2. **Finish the Gallery design draft.** Retain Photo Grid and Square Grid, then
   add the minimum full-image proxy-backed view. Remove Layout-specific actions
   when there is no explicit compatible Layout target; contextual actions must
   derive from the active capability/command context rather than assuming the
   bundled Layout node. Make the status footer report meaningful discovery,
   verification, proxy, stale-preview, and failure state. Let users choose the
   compact primary label from available presentation metadata (for example file
   name, camera, lens, capture time, or another supported field). Add an explicit
   Project/Selected Node viewing scope and allow quick text plus metadata-
   expression filtering within it. Scope, filter expression, primary label,
   selection, and full-image presentation remain disposable workspace state;
   making a reusable semantic result requires an explicit Query node or Core
   command.
3. **Inspector design draft.** Establish the production first-draft hierarchy
   for identity, typed ports and connections, parameters, status, diagnostics,
   progress, and node-contributed controls. Inspector content follows the
   selected exact definition through immutable presentation DTOs and semantic
   commands; it must not become a Swift interpretation of node-authored JSON or
   assume every node owns a canvas Workspace.
4. **Usable Layout authoring.** Complete the real Layout Workspace and controls
   required by the Stage 9 gate: multiple Layout nodes, explicit asset placement,
   frame/cell structure, Fit/Fill/Crop, focal alignment, rotation, crop commits,
   resolved proxy-backed preview, validation, save/reopen, and coherent
   undo/redo. Layout remains an ordinary node with explicit `AssetSet` input and
   project-owned proxies.

Completing these four slices closes **Stage 9**, not a product version numbered
`0.9.0`.

**Gate:** a real project can be authored visually without the old manual Layout
worksheet/crop-authoring process, including multiple independent Layout nodes,
explicit asset placement, frame/cell editing, Fit/Fill/Crop, rotation, crop
authoring, validation, resolved proxy-backed preview, save/reopen, and coherent
undo/redo. The same project can add a Disk node from a hierarchical `Tab`
catalog, grant/rebind a folder through native permission UI, explicitly scan
real supported files into project Asset Context, connect its `AssetSet` to
Layout, and reopen with portable semantics intact even when the machine-local
folder grant is unavailable.

**Current slice:** the facade exposes resolved normalized/pixel cell geometry
and semantic cell/structure edit enums. Exact forward/reverse Core graph
transactions make assignment, structure, content mode, focal/alignment, crop,
and rotation undoable. The macOS shell now has a separate movable Layout
authoring surface, explicit frame/cell selection, shared per-cell proxies,
frame/cell controls, nine-point alignment/focal controls, rotation, crop sizing,
and transient crop dragging that commits once on gesture end. Stage 9 remains
open for real-fixture workflow hardening against the complete gate. Disk scans
now run outside the main actor, Gallery uses Quick Look for immediate local
thumbnails, and Layout upgrades its immediate native image to a 1K-default F16
HDR-preserving verified proxy. Definition-owned double-click activation opens
Disk in Finder or focuses Layout's Workspace. `Tab` is captured at window level
while Graph is visible and opens the catalog at the current graph pointer (or
center before the pointer enters). Gallery opens a runtime representation in
the user's default native viewer on double-click. Disk rebind clears its old
membership immediately, and completed scans atomically replace only that
provider's assets while retaining unrelated project context. The generated
Disk path now publishes a metadata-only observation pass before streaming file
bytes at utility priority. Gallery retains any stale image while progressing
through the cheapest measured provider/native/profile path. The generated
Swift gate now exercises assignment undo/redo, resolved geometry,
cell insertion/arrangement, focal Fill, rotation undo/redo, and two independent
Layout nodes through the production facade on Quasar.

The accepted Graph Lab node-and-wiring draft is now promoted into one shared
macOS graph canvas and node renderer used by both Graph Lab and Photara. The
production adapter consumes immutable bridge DTOs, preserves package-authored
SVG icons and accent colors, and commits connection, disconnection, routing,
and node-position commands only at gesture boundaries. Selection uses the
authored neutral node fill with a perimeter stroke; camera, overview, catalog,
and tool-rail choices remain native workspace preferences. This completes the
first Graph nodes-and-wiring slice; Stage 9 remains open for Gallery, Inspector,
and Layout refinement.

Real 60 MP Photoshop TIFF testing supersedes part of that first attempt:
Quick Look exceeded 60 seconds for one 761 MiB 32-bit float LZW TIFF, while the
bounded 384 px F16 ImageIO helper returned in 2.91 seconds. TIFF Gallery requests
now use that tiny HDR profile directly from revalidated file-observation
evidence. Initially requested previews finish before whole-byte verification,
so hashing cannot block first pixels; verified revisions promote existing
previews without redundant generation.

A follow-up four-process sample on the same Quasar workload returned each
384 px job in 2.60–2.66 seconds at about 733 MiB peak process footprint. The
native client therefore chooses an explicit memory-tiered project generation
limit: four on machines with at least 64 GiB physical memory, two with at least
24 GiB, and one below that, with a device override constrained to 1–4. Core
Image's default context is GPU-capable, but the measurements indicate that
ImageIO/LZW decode and source-memory traffic dominate this source class;
parallelism improves Gallery fill rate rather than one file's decode latency.

Gallery now reserves fixed preview geometry before pixels arrive and offers a
compact photo-first crop grid plus an aspect-preserving detail grid with small
filenames. This prevents progressive metadata/image arrival from changing row
height or overlapping neighboring cells. Disk discovery accepts a broad still
image extension set; actual preview decoding remains a platform/provider
capability, and unsupported encodings surface a failed preview instead of being
silently omitted. The folder permission panel intentionally dims individual
files because the Disk node grants a folder, not a single file.

The usable Gallery refinement now provides two explicit presentations. Photo
Grid begins with square placeholders, then uses known proxy/native dimensions
to form tightly justified rows with two-point gutters and minimal corner radii.
Square Grid retains stable square cells, compact filenames, and portable
representation-format pills such as TIFF or DNG. A full-image viewer is enabled
only when the project proxy exists; double-click continues to open the runtime
source in the system default application. The toolbar toggle is implemented;
the proposed `G` cycling shortcut is reserved for the later shortcut pass.

All supported still-image requests now converge on the shared project cache.
Quick Look may win the initial display race, but generation continues through
the bounded project service so a close/reopen can validate and return the same
cache object. A service-reopen test proves `CacheHit` with zero backend calls.
Cache files, aspect ratios derived from them, Gallery mode, selection, and the
full-image presentation remain disposable/client state and never enter the
project document or graph digest.

Live Red Meridian testing found two 8599×5733, 32-bit float TIFFs whose embedded
profile is `P3D65 PQ Display Full 12-16-0-1`, while neighboring files use
Display P3 Linear. The initial helper correctly refused to label that unknown
profile as ordinary Display P3. Generator version 2 adds an explicit
color-managed conversion into Apple's extended-linear Display P3 HDR proxy
space and retains fingerprinted embedded-ICC handling for other safe cases.
After the `whisk` NAS remounted, both exact sources produced verified 384×256
F16 proxies tagged Display P3 Linear; the SDR-named source completed in 5.20
seconds on Quasar. Both files carry the same P3/PQ profile, reinforcing that a
Disk node must not infer dynamic range or pair membership from filenames.

Gallery live resize no longer calls UniFFI descriptors or opens proxy files from
Swift view bodies. Descriptor, decoded `NSImage`, and aspect ratio are populated
once when the proxy request completes; ImageIO now eagerly decodes cached proxy
pixels off the main actor before Swift publishes the small `NSImage`. A 76–220
point slider controls Photo Grid row height and Square Grid cell width and stays
enabled without an asset or Layout selection. The Graph retains keyboard focus
for `Tab` without drawing SwiftUI's lagging canvas-wide focus effect. Square Grid
now follows the reference more closely with filename and a two-point-radius
format label beneath the thumbnail. Proxy failures expose their actual
structured message through the cell status help instead of leaving an
unexplained placeholder.

A local-SSD follow-up separates backend time from network latency. The 60.2 MP
`_SUH5024…HDR.TIF` measured 2.64 seconds through Photara versus 2.63 seconds
through Quick Look's full thumbnail. The 152.7 MP `_SUH5076…HDR.TIF` measured
6.56 versus 6.52 seconds. Quick Look's low-quality request failed after 2.77
and 6.69 seconds respectively, so TIFF continues to bypass that nonproductive
rung. Eagerly mapping and decoding all 40 existing proxies for the active
project measured 22.5 ms total and 5.3 ms to the first image. Generator version
2 intentionally changed cache identity once; subsequent project reopens reuse
the fast cache path.

The native first-look refinement now separates Graph, standard Inspector,
optional node Workspace, and project panels. Launch presents Create/Open/Recent
with native-only recent state and no database. The facade provides generic
typed port/connection/status/summary DTOs; Swift renders Project Assets and
Layout through one Inspector shell. A primitive spatial graph replaces the
engineering list, Layout Workspace opens explicitly, and Workspace → Restore
Default Workspace recovers presentation state without semantic mutation.
The Graph camera uses a viewport-sized procedural grid whose spacing follows
zoom, supports drag/trackpad pan plus pinch/mouse-wheel zoom, and offers an
optional bottom-left node/connection/viewport overview. Camera and overview
state remain native workspace presentation only.

The developer Theme Lab is the bounded palette-authoring side tool for the next
UI pass. A versioned JSON document carries identical semantic slots for paired
Light/Dark sRGB palettes, including status text and durable node color roles.
Photara and the Lab compile the same Swift parser/resolver; the Lab previews
the actual production Workspace, Layout Inspector, Graph, Gallery, and controls
against an isolated fixture project. A validated development override live-reloads in
Photara while retaining the last valid palette. Built-in definitions now
request `node.native` through presentation metadata and keep literal accents as
compatibility fallbacks. Themes remain client preference state.

### 10. Scoped runtime persistence and 0.2.0 stabilization

- Introduce the first database-backed implementation of the Core-owned state
  service only after the Stage 9 Disk/live-project workflow proves a concrete
  runtime-state need. Preserve the portable Project Document as authored
  authority.
- Define explicit state scopes: syncable `user + exact definition` libraries
  and preferences; private `project + node instance` operational state;
  device-only grants, credentials, and paths; and portable authored Project
  Document state. Saving a reusable preset and applying it to a project are
  distinct semantic operations.
- Give exact definitions and node instances logically isolated, versioned state
  namespaces and narrowly scoped host APIs. Nodes never receive connection
  strings, arbitrary SQL, another node's namespace, or ambient access to the
  Core database. Cross-machine state is an authenticated host sync capability,
  not direct network/database authority for node code.
- Define migration, quota, backup, corruption recovery, deletion, package
  disable/uninstall, and missing-definition behavior for scoped node state
  before relying on it for production work.
- Exercise several real projects and refine workflow, performance, diagnostics,
  recovery, and native interaction.
- Validate the complete Graph → Gallery → Inspector → Layout workflow with live
  local and NAS data at realistic scale. A Layout result must be usable output,
  not only an on-screen worksheet: its resolved plan/artifact must survive
  save/reopen, expose actionable diagnostics, and be consumable by the explicit
  `0.2.0` export/delivery boundary selected for the release slice.
- Document install, backup, update, rollback, crash recovery, and the explicit
  supported contract.
- Choose release branding only if it is ready; code-name presentation is valid.

**Release gate:** a new user can install the Mac application, create a clean
workspace, load assets, visually author and persist useful Layout nodes, and
recover safely from interruption without external legacy state.

## After 0.2.0

- Restore the generation-one reusable production libraries as versioned,
  user-owned domain records: **People** (including the `model` role, aliases,
  and social identities), **Clients**, **Locations**, and **Scenes**. Projects
  assign stable library identities and retain the portable snapshots needed to
  remain intelligible offline or when a global record later changes.
- Add a separately authorable native **Library** module (`photara-library`) and
  **Library Lab** (`photara-library-lab`), parallel to Gallery, Inspector, and
  Graph. It owns browsing, searching, and editing those global records and
  project-assignment controls; Core-owned services own identity, validation,
  revisions, and persistence. Keep this library distinct from both the visual
  asset Gallery and the node Catalog.
- Support both local-only and authenticated cloud-synchronized library modes
  behind the same backend-neutral state-service contracts. The local adapter is
  fully usable offline; a PostgreSQL service such as Neon may implement the
  cloud adapter, but Neon, SQL, and connection strings never enter Core domain
  contracts or node packages. Define versioned schemas, incremental sync,
  conflicts, tombstones, account ownership, export, backup, and recovery before
  making cloud state authoritative for user libraries.
- Portable metadata-query expressions and ordinary selection nodes for
  intersection/AND, union/OR, difference/NOT, XOR, match/join, sort, and group.
  Add provider query pushdown, incremental/paged result contracts, and the first
  Core-owned rebuildable per-device metadata index only through concrete large-
  library vertical slices.
- Selection-import nodes such as a Pixieset CSV reader, followed by explicit
  matching against assets from Disk, Lightroom, cloud, or other providers.
- Gallery viewing of a selected node's visualizable asset output as disposable
  workspace context, with an explicit whole-project/selected-node scope and
  disposable metadata-expression filter. Explicit commands remain required to
  connect, capture, or promote that expression into a Query node.
- Typed project variables and portable expression references, including a
  host-resolved project-relative root comparable to `$PROJECT_PATH`. Add them
  through Core-owned authored state and scoped runtime binding resolution—not
  Swift string substitution, process environment variables, or serialized
  absolute paths.
- Photoshop, Lightroom Classic, Lightroom Desktop/Cloud, Cloudinary, delivery,
  metadata, ML, and other independently installable nodes.
- Independently branded asset-provider nodes for Dropbox, Google Drive, Box,
  iCloud/File Provider, Photos/PhotoKit, and studio DAMs. Each emits ordinary
  semantic assets while authorization and remote materialization remain scoped
  host capabilities.
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
