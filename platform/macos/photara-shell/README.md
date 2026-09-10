# Shared application shell

Owns launcher, native toolbar, navigation, pane placement/disclosure, panel headers,
host empty states and contextual status. It accepts `ApplicationPresentation` and
semantic `ApplicationActions`; feature rendering belongs to the supplied panel closure.

`ApplicationShellAvailability` is typed capability policy. `WorkspaceModel` owns
client preferences and transient selection/disclosure. Every node can reveal the
shared Inspector; optional Layout authoring requires a capability. Session state
resets per project and never enters Project Documents or graph digests.

`ApplicationShellPreset` (also named `ApplicationShellPresentation`) validates the
versioned JSON in Resources. It holds bounded typography and dimensions, no colors
or behavior. Both Photara and [Shell Lab](../photara-shell-lab/README.md) compile these
exact files through `shared-ui-sources.sh`.

Build all hosts with `platform/macos/build-ui.sh`. Verify with the shared UI,
production UI and bridge scripts; run full Graph interactions after assembly changes.

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
