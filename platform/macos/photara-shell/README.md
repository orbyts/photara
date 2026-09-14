# Shared application shell

Owns launcher, native toolbar, navigation, pane placement/disclosure, panel headers,
host empty states and contextual status. It accepts `ApplicationPresentation` and
semantic `ApplicationActions`; feature rendering belongs to the supplied panel closure.

`ApplicationShellAvailability` is typed capability policy. `EditorSessionModel` owns
client preferences and transient selection/disclosure. Every node can reveal the
shared Inspector; optional Layout authoring requires a capability. Session state
resets per project and never enters Project Documents or graph digests.

`ApplicationShellPreset` (also named `ApplicationShellPresentation`) validates the
versioned JSON in Resources. It holds bounded typography, dimensions, and launcher-
specific adaptive colors, but no availability or project behavior. Both Photara and
[Shell Lab](../photara-shell-lab/README.md) compile these
exact files through `shared-ui-sources.sh`.

The accepted Opening uses native `OpeningLibraryView` window/text colors.
The authored ladder begins in project content.
Its sidebar, titlebar, toolbar, controls, selection geometry and system accent remain
macOS-owned. Retired launcher typography, colors/material, tile/glow and positioning
fields remain readable in schema-1 presets for compatibility. The active Opening
route does not use them and Shell Lab no longer authors them.

Shell Lab owns only shared geometry/behavior and scenario fixtures. Its draft and
explicit Apply/Remove/Export actions affect Shell only. Theme Lab owns the paired
palette; all hosts read it through the shared resolver. These preferences never
enter project state. The toolbar supplies project identity, application identity
and semantic command groups while macOS owns optics and accessibility behavior.

The accepted UI1 `CreateProjectView` defaults to Compact through
`CreateProjectPresentation.shipped`. Shell Lab also previews Balanced and Spacious.
The new sheet is not connected to the production creation route; its Lab callbacks
are disposable fixture actions. Real destination selection and atomic package,
catalog, initial Graph and cloud creation remain a later wiring task.

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

The `frame` preset wraps each module in one borderless rounded surface with visible
canvas gutters and an integrated icon/header. Theme owns the application base,
module base, optional inset-content surface and selection tint. New Library modules and Account are peers.
Graph and the optional Work Surface have independent visibility; neither's
renderer owns Shell clipping or insets. Existing saved placements migrate by
adding only missing identities, and Restore Editor remains recoverable.
