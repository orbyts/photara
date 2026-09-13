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

Typography stores portable family intent (`display`, `rounded`, `serif`, or
`monospaced`), not an installed font name. macOS resolves those roles through native
system fonts; the shipped launcher default is SF Display. A future Windows shell can
map the same roles to its native families while consuming the same design decision.

The preset also owns the launcher hero icon-tile treatment and individual action
tints as paired Light/Dark sRGB values, plus portable tile, stroke, corner and glow
geometry. It separately stores symbol alignment within the tile, tile positioning,
launcher edge insets, hero-to-recents spacing and the opening content's vertical
position. Project Chrome geometry includes project-title typography, panel headers
and the status bar; their colors remain shared Theme roles.
The native toolbar keeps project identity and its optional thumbnail at the leading
edge, centers the application identity, and supplies semantic groups of operational
controls at the trailing edge. macOS owns its Liquid Glass, grouping, shadow, blur,
contrast, accessibility fallback and overflow; the Shell exposes no optical controls.
Launcher background material is stored as a semantic native role with an
adaptive tint, allowing macOS to use system frost and other clients to map the same
intent to their native backdrop material. Shell Lab saves an authoring draft in
`com.photara.shell-lab` preferences.
Its explicit Apply action places a validated development override in the
`com.photara.desktop` preferences domain; Photara live-reloads it. Removing the
override restores the bundled resource. Neither preference enters project state.

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
