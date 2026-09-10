# Shared native UI and component labs

Photara is the integration surface. Graph Lab, Gallery Lab and Inspector Lab
are independently runnable authoring hosts around the **same source files**
that Photara compiles. There is intentionally no separate Main App UI Lab.
This is source-level modularization, matching the existing Graph pattern;
these directories are not independent Swift binary frameworks.

| Boundary | Shared ownership | Host responsibility |
| --- | --- | --- |
| `photara-ui-foundation` + `photara-theme` | Native theme environment/resolver, Apple HDR image view, lightweight native inspection and preview values | Map bridge values and retain proxy references |
| `photara-graph` | Existing canvas, nodes, event surface, interaction controller, authored preset | Graph Lab fixtures; production bridge adapter |
| `photara-gallery` | Photo/square grids, cards, activity badges, selection treatment, filter, sizing, full-image sheet, authored preset | Assets/images, selection/filter bindings, open/assign/request callbacks |
| `photara-inspector` | Assembled Inspector and individually previewable identity, Disk, ports, parameters, frame/cell, evaluation and diagnostic sections | Immutable typed inspection values and semantic action callbacks |
| `photara-layout` | Optional Layout workspace, canvas and cells, transient crop gesture | Resolved Layout inspection, proxy images, selection state, request and edit callbacks |
| `photara-shell` | Application chrome/launcher/status, three split regions, panel headers/placement/visibility, client workspace preferences | Project summary, application actions, feature composition |
| `photara-app` | Thin `WorkspaceView` composition and production adapters | Core, bridge, persistence, file dialogs, source grants, evaluation, proxy ownership |
| `photara-lab-support` | Deterministic fixtures and lab appearance/export helpers | Compiled only by labs and verification, never Photara |

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
4. Inspector hierarchy/spacing and shell region sizing are shared code constants;
   colors remain semantic theme roles. There is no artificial Inspector or shell
   preset duplicating theme values or constants.
5. Gallery filter, current grid style/size, selection and focused image are
   disposable native viewing state. Workspace panel visibility and placement
   retain the existing `photara.workspace.layout-authoring.v1` UserDefaults
   payload. Theme and Graph user preferences retain their current domains.
   None of these presentation settings enter Project Documents or graph digests.

Production is not hot-patched from lab-local experiments: source/preset promotion
and rebuild are explicit. Theme development overrides retain their existing live
reload mechanism.

## Universal Inspector, optional node authoring

Every selected node has a standard Inspector for identity, package/definition
contract, typed ports, evaluation and diagnostics, including missing packages.
Node-specific parameter sections are optional. A dedicated authoring pane is
also optional and independent of Inspector: Layout is the existing example.
`NodeInspection` contains native projections, not node-authored JSON, and
`InspectorActions`/`LayoutActions` carry explicit intents to the production
adapter. The adapter resolves the current node and submits the existing
revision-checked Core command.

Future node packages should contribute typed inspection/authoring contracts
through the bridge's definition/contribution mechanism. Do not turn Inspector
into a Swift parser for package JSON, assume every node has Layout state, or
make Core own native view policy. A general registry for additional native
node-specific workspaces remains a later behavior feature.

## Compatibility and boundaries

`photara-app/Sources/ThemeStore.swift` and `WorkspaceModel.swift` are source
symlinks to their new owners. They keep the existing Graph and bridge
verification scripts unchanged. Main and feature builds compile the canonical
shared files once. `HDRImageView.swift` moved unchanged to UI foundation:
Gallery/Layout request `.constrainedHigh`; focused Gallery uses `.high` through
`NSImageView.preferredImageDynamicRange`.

Gallery continues the accepted design and selection/layout behavior. Its context
assignment now uses the same target-availability flag as the footer button.
Core, Project Documents, bridge declarations, proxy generation and evaluation
implementations have not changed. Existing Gallery scope/metadata filtering,
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
Inspector callback targets, and render 26 Gallery/Inspector states. The production
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

One command assembles the three labs and production:

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
