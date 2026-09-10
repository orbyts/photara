# Photara Shell Lab

A native two-window shell authoring app: **Photara — Shell Preview** contains the
exact production `ApplicationShell`, Graph, Gallery, Inspector and optional Layout
views; **Shell Authoring** contains scenarios, Light/Dark, sizing and visual controls.
No Core, bridge, database, source folders or network are used. Fixtures are generated
in memory; workspace preferences are not read or persisted. Bundled presets/theme
are read-only resources; Export uses the native save panel.

```sh
platform/macos/photara-shell-lab/build-shell-lab.sh
open 'platform/macos/photara-shell-lab/.build/Photara Shell Lab.app'
```

Scenarios cover opening with/without recents, collapsed/expanded recents, empty
project, first node, cleared selection, assets, Layout capability, reviewable
content, evaluation/cancel, diagnostics, compact, saved and syncing. Add Node opens
a fixture catalog; Create/Open/Save/Run/Cancel update fixture state. Feature editing
callbacks are recorded in controls; production behavior remains in the app adapters.
Review currently exposes the shared Gallery for reviewing available images; a
specialized review workflow remains future work.

Use live controls for title font/size/weight, hero icon-tile visibility, symbol/tile/
stroke/glow colors, tile and symbol size, stroke width, corner and glow geometry,
symbol vertical alignment, tile position, individual launcher-button tints,
hero-to-recents spacing, launcher edge insets and opening vertical position, pane
ideal widths, header/status heights and toolbar identity width. The launcher
background can use the Theme surface or a
native Ultra Thin, Thin, Regular or Thick frosted material with its own tint. Light
and Dark colors are authored independently. Export to
`platform/macos/photara-shell/Resources/photara-application-presentation-v1.json`,
review the diff and rebuild Photara. Schema v1 rejects unknown versions and invalid
or nonfinite dimensions. Font choices are portable design roles: macOS maps Display,
Rounded, Serif and Monospaced to SF Display, SF Rounded, New York and SF Mono.
Other native clients map those same roles to their platform families. Behavior remains
typed Swift. The general application palette remains Theme-owned; the explicitly
authored launcher hero and action tints travel with the Shell preset.

Project Chrome controls author the real project-title typography and visibility,
panel-header geometry, semantic divider thickness, and status-bar typography and
spacing. The fixture project name (initially `Coastal Studies`) is preview-only;
production always supplies the open project's real name. Chrome color pickers edit
the shared Theme roles for workspace, panel, elevated header/status surfaces,
dividers, text and semantic status colors. Native macOS window material and traffic
lights remain system-managed.

Shell Lab automatically retains the current draft in its `com.photara.shell-lab`
preferences. **Apply to Photara** writes a validated development override to the
`com.photara.desktop` preferences domain and a validated shared Theme override; a
running development build polls both and updates without a rebuild. **Remove Photara
Override** returns the app to its bundled Shell and Theme. These are local developer preferences and never enter a
Project Document. Exporting and promoting the JSON resource remains the explicit
step that changes the shipped default for all users and future platform builds.
Graph's source, preset and interactions are unchanged.

For an independent task, edit shell-owned source or this preset, run
`platform/macos/build-ui.sh`, `photara-ui-tests/verify-shared-ui.sh`,
`photara-ui-tests/verify-production-ui.sh`, `photara-app/verify-bridge.sh` and the full
`photara-graph-lab/verify-interactions.sh` (paths relative to `platform/macos`).
Handoff the commit, preset/source changes and verification results. Merge and
rebuild assembles the change; no feature views are copied into production.

Ownership: every node has an Inspector and may contribute its own optional work
surface. The Layout surface is owned by the Layout node (`photara-layout`). Shell
Lab previews that exact surface but does not author its internals. A future Layout
Node Lab can independently author it; no new node-specific lab is created here.

The toolbar is driven by `[NodeWorkSurfacePresentation]`, not a fixed Layout tab.
Each descriptor carries the node ID, contribution ID, title and node-provided icon.
The host supplies the node-owned renderer. Multiple nodes have distinct buttons;
removing the active node returns to Graph. Production registers supported surfaces
in `ProductionWorkSurfaceRegistry`; Layout is its first entry. The saved client pane
key remains `layoutAuthoring` for compatibility, while its Swift identity is now the
generic `nodeWorkSurface`.

Application Surfaces controls now author semantic/custom canvas and module fills,
canvas material, gutters, outer/content insets, corner radius, optional border,
elevation, active emphasis, elevated/flat headers, compact breakpoint and a
separate bottom/status surface. They use the same draft, Apply and Export workflow.
Use the top-right shortcuts or Workspace to reveal Library fixtures. Every module
has an icon and can close/reopen, including Graph and the optional Work Surface.
Graph's accepted shared sources, preset and interactions are untouched.
