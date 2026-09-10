# Shared native UI and component labs

Photara is the integration surface. Graph Lab, Gallery Lab, Inspector Lab and Shell Lab
are independently runnable authoring hosts around the **same source files**
that Photara compiles. Shell Lab authors shared shell composition with deterministic fixtures in a separate controls window; Photara remains the production integration app. There is no duplicate Main App UI Lab.
This is source-level modularization, matching the existing Graph pattern;
these directories are not independent Swift binary frameworks.

| Boundary | Shared ownership | Host responsibility |
| --- | --- | --- |
| `photara-ui-foundation` + `photara-theme` | Native theme environment/resolver, Apple HDR image view, lightweight native inspection and preview values | Map bridge values and retain proxy references |
| `photara-graph` | Existing canvas, nodes, event surface, interaction controller, authored preset | Graph Lab fixtures; production bridge adapter |
| `photara-gallery` | Photo/square grids, cards, activity badges, selection treatment, filter, sizing, full-image sheet, authored visual and empty-state preset | Assets/images, source/result context, selection/filter bindings and semantic callbacks |
| `photara-inspector` | Assembled Inspector, authored empty-state preset, and individually previewable identity, Disk, ports, parameters, frame/cell, evaluation and diagnostic sections | Immutable typed inspection values and semantic action callbacks |
| `photara-layout` | Layout-node-owned optional work surface, canvas and cells, transient crop gesture | Resolved Layout inspection, proxy images, selection state, request and edit callbacks |
| `photara-people` | User/studio People and Clients browser/editor | Immutable Library projections and semantic create/edit/select actions |
| `photara-locations` | Hierarchical Locations and sub-locations browser/editor | Immutable Library projections and semantic create/edit/select actions |
| `photara-scenes` | Reusable Scene browser/editor | Immutable Library projections and semantic create/edit/select actions |
| `photara-project-info` | Project assignments for clients, people, locations, and scene occurrences | Portable project references plus Library lookup/assignment actions |
| `photara-shell` | Application chrome/launcher/status, capability-driven split regions, panel headers/placement/visibility, client workspace preferences | Project summary, application actions, feature composition |
| `photara-app` | Thin `WorkspaceView` composition and production adapters | Core, bridge, persistence, file dialogs, source grants, evaluation, proxy ownership |
| `photara-lab-support` | Deterministic fixtures and lab appearance/export helpers | Compiled only by labs and verification, never Photara |

### User Library and project context

The user-level Library is distinct from both the visual asset Gallery and the
node Catalog. Its shared production UI is divided by workflow into separately
authorable People, Locations, Scenes, and Project Info modules. Each has its own
lab and compiles the same source used by Photara. Clients begin as a category in
People because client contacts and organizations share the same discovery and
assignment workflow; that boundary can be split later without changing stable
record identity.

Each module is an independently identified workspace surface. Users may close,
restore, dock, tab, or move it; those choices are native workspace preferences
and never project semantics. Project Info composes assignments from the global
Library and stores portable project references with display/revision snapshots.
It does not duplicate the Library records.

The shared modules consume immutable presentation values and emit semantic
actions. The backend-neutral `photara-library` Rust service owns record
identity, validation, revisions, tombstones, queries, and local SQLite
persistence. Every mode retains this offline local working copy. Photara Cloud
and future iCloud options add synchronization through host-owned adapters;
native views and node packages never receive SQL or credentials. See
[`docs/LIBRARY_ARCHITECTURE.md`](../../docs/LIBRARY_ARCHITECTURE.md).

These module and lab boundaries are now pre-Layout groundwork. Their first
slice should establish contracts, empty/loading/error states, close/restore,
and On This Mac storage; full cloud subscription and legacy migration do not
block visual authoring.

### Application surface frame

Shell Lab owns the Spotify-reference composition language: an application
canvas behind independent rounded, filled module surfaces separated by gutters,
with each surface carrying its own header and scrolling region. It authors
semantic Light/Dark canvas and surface roles, gutter, inset, corner radius,
border/elevation, active emphasis, compact stacking, and status treatment.
Feature labs own the content within those surfaces. This is a layout and
hierarchy reference, not a copy of Spotify's branding or dark-only palette.

The title bar remains native and system-managed. Account, People, Locations,
and Scenes shortcuts may reveal or focus their corresponding module from the
top-right application area. Project Info is a peer workspace module; optional
node Work Surfaces remain opt-in node contributions.

`shared-ui-sources.sh` is the production assembly manifest. Feature labs compile
only foundation, theme, their feature and their lab fixtures. They do not build
Rust, link the bridge, create a project, access NAS, or require a provider.

## Build and run

From the repository root, on macOS with the current Xcode toolchain:

```sh
platform/macos/photara-graph-lab/build-graph-lab.sh
open 'platform/macos/photara-graph-lab/.build/Photara Graph Lab.app'
platform/macos/photara-gallery-lab/build-gallery-lab.sh
open 'platform/macos/photara-gallery-lab/.build/Photara Gallery Lab.app'
platform/macos/photara-inspector-lab/build-inspector-lab.sh
open 'platform/macos/photara-inspector-lab/.build/Photara Inspector Lab.app'
platform/macos/photara-shell-lab/build-shell-lab.sh
open 'platform/macos/photara-shell-lab/.build/Photara Shell Lab.app'
platform/macos/photara-app/build-app.sh
open 'platform/macos/photara-app/.build/app/Photara.app'
```

The existing Theme Lab and the author's Glass experiment remain separate theme
tooling, preserved by this refactor. Its production preview consumes this same
assembly. `build-theme-lab.sh` includes `GlassTestScene.swift` when present so
uncommitted theme experiments are supported without becoming dependencies of
a clean checkout.

## Authoring and defaults

1. Change shared feature code to change the component everywhere on rebuild.
2. Experiment with visual controls inside a lab. Its live authoring state is local.
3. Gallery's **Export Shipped Preset** writes a validated, versioned JSON file.
   To promote it, save/copy it to
   `photara-gallery/Resources/photara-gallery-presentation-v1.json`, review the
   diff, and rebuild Gallery Lab and Photara. Both decode that exact resource.
   Graph retains its existing Graph Lab export/preset workflow unchanged.
4. Shell Lab exports validated `photara-shell/Resources/photara-application-presentation-v1.json`.
   Promote that file and rebuild to share launcher typography, hero icon-tile
   treatment, launcher action tints, spacing, pane sizes and chrome dimensions.
   The Lab automatically saves its draft and can apply a validated development
   override that Photara live-reloads. Availability remains typed Swift. Inspector
   hierarchy stays in its component; the general application palette stays in Theme.
5. Gallery filter, current grid style/size, selection and focused image are
   disposable native viewing state. Workspace panel visibility and placement
   retain the existing `photara.workspace.layout-authoring.v1` UserDefaults
   payload. Theme and Graph user preferences retain their current domains.
   None of these presentation settings enter Project Documents or graph digests.

Production is not hot-patched from ordinary lab-local experiments. Theme and Shell
have explicit development-override actions with live reload; source/preset promotion
and rebuild remain the deliberate path to shipped defaults.

## Universal Inspector, optional node authoring

Every selected node has a standard Inspector for identity, package/definition
contract, typed ports, evaluation and diagnostics, including missing packages.
Node-specific parameter sections are optional. A dedicated authoring pane is
also optional and independent of Inspector: Layout is the existing example.
The work surface belongs to its node, not to the application shell or Inspector.
`photara-layout` is the Layout node's native work-surface implementation; the shell
only hosts it and manages placement and capability-driven access. Each node's
surface can be authored separately. A dedicated Layout Node Lab can be added
later, consuming these same sources; it is not part of this shell task.
`NodeInspection` contains native projections, not node-authored JSON, and
`InspectorActions`/`LayoutActions` carry explicit intents to the production
adapter. The adapter resolves the current node and submits the existing
revision-checked Core command.

Future node packages should contribute typed inspection/authoring contracts
through the bridge's definition/contribution mechanism. Do not turn Inspector
into a Swift parser for package JSON, assume every node has Layout state, or
make Core own native view policy. `NodeWorkSurfacePresentation` carries each contributing node's identity, title,
icon resource and contribution ID. The shell renders one toolbar button per node
and invokes the supplied host renderer. `ProductionWorkSurfaceRegistry` registers
node-owned native renderers; Layout is the first entry. Future node surfaces add
an entry there and their own module/lab, without changing shell navigation. Nodes
without a work surface still have an Inspector; unsupported native contributions
do not advertise dead toolbar buttons.

## Compatibility and boundaries

`photara-app/Sources/ThemeStore.swift` and `WorkspaceModel.swift` are source
symlinks to their new owners. They keep the existing Graph and bridge
verification scripts unchanged. Main and feature builds compile the canonical
shared files once. `HDRImageView.swift` moved unchanged to UI foundation:
Gallery/Layout request `.constrainedHigh`; focused Gallery uses `.high` through
`NSImageView.preferredImageDynamicRange`.

Gallery continues the accepted design and selection/layout behavior. Its context
assignment now uses the same target-availability flag as the footer button.
Library integration adds an independent facade and portable Project Info extension.
Graph, proxy generation and evaluation implementations remain unchanged. Existing Gallery scope/metadata filtering,
capability-driven action discovery, scalable paging/virtualization and further
Layout behaviors are separate roadmap work, not introduced by this extraction.

## Verification

```sh
platform/macos/photara-graph-lab/verify-interactions.sh
platform/macos/photara-app/verify-bridge.sh
platform/macos/photara-ui-tests/verify-shared-ui.sh
platform/macos/photara-ui-tests/verify-production-ui.sh
```

The shared checks validate preset decoding/round trips, extended-range float
fixtures, native HDR policy and fit/fill geometry, workspace preference isolation,
Inspector callback targets, and render 54 Gallery/Inspector/Shell states. The production
checks compose the real `WorkspaceView`, use an isolated Core project, import an
HDR/SDR pair, exercise semantic edits and undo/redo, save, and check that viewing
Graph/Layout leaves the graph digest unchanged. Snapshots live under
`photara-ui-tests/.build`. Verification executables are test harnesses, not extra
product/lab apps. Graph's native test suite requires an uninterrupted foreground
session and Accessibility permission for physical slider gestures. HDR luminance
and display tone mapping still require a human check on an HDR-capable display;
screenshots cannot establish actual panel brightness.

## Separate-task handoff and assembly

Start each design task from the integrated shared-UI branch (or its merged
successor), preferably in its own worktree. Keep Graph edits in `photara-graph`
and Graph Lab; Gallery edits in `photara-gallery` and Gallery Lab; Inspector
edits in `photara-inspector` and Inspector Lab. Read this document and the
feature's lab README first. Shared foundation/contract changes require checking
the other consumers and should be called out in that task's handoff.

One command assembles the eight labs and production:

```sh
platform/macos/build-ui.sh
```

There is no copying a lab view into production and no manual source-file wiring:
the feature source directories are compiled directly. Preset exports are the
only deliberate promotion step for live authoring controls. Each completed lab
task should hand off its commit, shared source/preset paths, build/test results,
and any visible behavior changes. Before merging, run its lab build and
`platform/macos/photara-ui-tests/verify-production-ui.sh`; for Graph changes also
run the full existing Graph interaction suite. A merge plus rebuild is the
production assembly step. This task creates no additional Codex tasks; the user
can start the independent lab tasks when ready.

The original Disk/main-functionality task can resume against this structure:
edit production adapters/models and bridge/Core as required by behavior, while
leaving renderer ownership in the shared component directories. Keep all node
inspectors universal and dedicated node authoring surfaces optional.

Shell tasks own `photara-shell`, its presentation preset, and `photara-shell-lab`.
Read [Shell Lab](photara-shell-lab/README.md) before authoring. New projects reveal Project Info for Library assignment; an otherwise empty workspace shows
Graph alone. Inspector stays disclosed after first selection; Gallery requires
assets, an asset-producing context, or an explicit request. Layout navigation
requires Layout capability; Review requires reviewable content. Diagnostics use
no space until requested. Compact windows stack disclosed regions. All this is
client state and never changes Core digests.

## Module frame and Library labs

People, Locations, Scenes and Project Info now have build scripts at
`photara-<module>-lab/build-<module>-lab.sh` (`project-info` keeps the hyphen).
Their app bundles are `.build/Photara People Lab.app`, `Photara Locations Lab.app`,
`Photara Scenes Lab.app` and `Photara Project Info Lab.app` within each lab directory.
Each README documents its exact build and run commands. Shared browser/editor
chrome and thumbnail primitives live in `photara-library-ui`; feature field
editors remain in their feature directory. Fixtures and lab controls remain in
`photara-lab-support`, never in production.

Every surface has an icon and close control. Workspace toolbar/menu toggles
restore Graph, Assets, Inspector, Work Surface, People, Locations, Scenes,
Project Info, Diagnostics and Library & Sync. Graph and a node Work Surface can
coexist. Old placement payloads gain new hidden module identities without losing
existing positions. Restoring the default recovers Graph and progressive
disclosure. Native split resizing remains; advanced tabs/floating geometry are
future work. If more than two panels share a region, the region scrolls and a
shortcut scrolls to its revealed module.

Shell preset `frame` owns semantic/custom Light/Dark canvas and module fills,
canvas material, gutter, outer/content inset, radius, border, elevation, active
emphasis, header treatment, compact breakpoint and separate bottom/status shape.
The native title bar stays system-managed. Shell Lab's existing automatic draft,
Apply to Photara, Remove Override and validated export paths include these fields.
Production polls overrides every 500 ms. Promote the JSON resource and rebuild
for shipped defaults; ordinary Library lab fixture edits remain local.

See [Library architecture](../../docs/LIBRARY_ARCHITECTURE.md) for actual SQLite,
thumbnail and portable project-reference behavior and deferred cloud work.
