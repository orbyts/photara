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

Use live controls for title size/weight, hero/icon size, spacing, pane ideal widths,
header/status heights and toolbar identity width. Export to
`platform/macos/photara-shell/Resources/photara-application-presentation-v1.json`,
review the diff and rebuild Photara. Schema v1 rejects unknown versions and invalid
or nonfinite dimensions. Behavior remains typed Swift and colors remain Theme-owned.
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
