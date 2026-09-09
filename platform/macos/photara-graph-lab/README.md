# Photara Graph Lab

Graph Lab is a small macOS developer utility for evaluating Photara's reusable
Graph presentation primitives without opening a project or building the Rust
bridge. It owns fixture data and temporary authoring controls only.

Node headers follow the reusable
[Graph Node Design Language](../photara-graph/NODE_DESIGN_LANGUAGE.md): a
free-standing 28-point outline icon centered against a tight title/category
stack. The icon alone receives an adaptive functional-category color; the title
and category label remain neutral. SVGs are cached native template images, so
node movement, pan, and zoom do not regenerate paths. Presentation metadata is
resolved separately from the graph document and interaction controller.

The current slice compares procedural major/minor background patterns, native
glass treatments, node corner radius, round versus pill ports, port edge
offset, and one stable node-shadow model. There are deliberately no
status indicators or Graph semantics.

Lines, Dots, and Crosses share aligned minor and major phases while the camera
pans or zooms. Graph Lab authors their colors, opacity, size or line width, and
major interval independently. Light and Dark each retain a complete authored
palette for the Graph canvas, minor and major grid, noodles, node glass tints,
port-bead tint, and node text. Semantic port hues remain shared concepts while
each appearance authors a native brightness adjustment. Dense minor marks fade
before they alias.

The scene uses one-, three-, and six-row node definitions with live port-to-port
connections. Pressing an output immediately starts a noodle. Dropping near a
compatible input connects; dropping on empty canvas cancels. Outputs fan out,
inputs accept one incoming connection, and identical pairs are idempotent.
Dragging a connected input rewires its existing connection; an invalid or
cancelled rewire preserves its identity and routing knot.

Hold `Y` and slice to cut all crossed noodles. Option-click a noodle to add a
routing knot. Drag from the knot to branch to another input; Option-drag moves
the shared point and all of its attached noodles. Click a noodle or knot and press Delete,
or use the native right-click/Control-click menu. Input-port menus list compatible
outputs from the document. Straight and curved noodle styles remain available.

Empty-canvas dragging, middle-mouse dragging, and precise two-finger scrolling
pan the graph. Mouse-wheel zoom and trackpad magnification preserve the pointer
anchor. The bottom slider zooms about the canvas center. All graph geometry,
text, ports, knots and hit targets use the same camera. The floating control
retains Apple's interactive Regular Liquid Glass capsule and native material
fallback for Reduce Transparency.

The translucent overview shows the entire graph and a viewport rectangle. It
fades in at zoom start and fades out after 650 ms of inactivity; held slider or
pinch gestures keep it visible until they end. The Overview section at the top
of the sidebar offers Show While Zooming (default), Always Show, and Never Show,
all four corners, a relative size control, and corner radius. Its default width
is 16% of the graph window, bounded to 144–360 native points; native display
scaling handles Retina resolution. The frame matches the graph canvas aspect
ratio and follows window reshaping immediately. Bottom placements clear the zoom controls.
It does not intercept graph input. Save Preferences persists all overview choices
alongside the existing palettes, with defaults for older preference payloads.

The narrow floating tool rail groups camera controls, selected-noodle actions,
and pinned native-node shortcuts. Its icons are flat while its container uses
Apple's native Liquid Glass, with the semantic material fallback when Reduce
Transparency is enabled. A top-center rail is horizontal; left and right rails
can align to the top, center, or bottom, switching to two columns only when their
vertical proposal is too short. The Graph Lab authoring sidebar exposes corner
radius, light/dark shadow opacity, blur, and vertical offset. The Settings window
contains user preferences for rail visibility/placement, overview behavior, and
knife cursor size. Hiding the rail does not disable keyboard, pointer,
contextual-menu, or zoom behavior. The passive gesture guide appears only while
the rail is hidden. When the rail is visible, native hover help describes each
tool and includes its available shortcut or pointer gesture. The command menu
shows graph gestures and shortcuts. Native node types can
be pinned or removed there, or removed from a shortcut's contextual menu.
Clicking a shortcut inserts an ordinary node DTO at an unoccupied position near
the viewport center; it does not create a separate node interaction path.

Selection remains one typed value: at most one node, noodle, or routing knot is
selected. Contextual rail actions therefore apply only to the current selection.
Add Knot appears for a selected direct noodle, Delete Knot for a selected routing
point, and Disconnect for a selected noodle.

The tool rail and overview are mutually exclusive overlays. If both request the
same occupied corner, the overview is presented in the opposite corner. A
left-bottom rail also moves the passive gesture hint away from its hit region.

A native floating tool palette sits at the leading edge of the canvas. It uses
the same controller commands as the canvas responder instead of duplicating graph
interaction state. Center and zoom commands are always available. The contextual
slot offers Add Routing Knot only for a selected direct noodle, Delete Routing
Knot only for a selected knot, and Disconnect only for a selected noodle. The
commands menu documents the corresponding native gestures and shortcuts. Only
one node, noodle, or routing knot can be selected at a time. When vertical space
is constrained, the palette falls back to a compact two-column layout.

The flat knife artwork is the shared SVG for the palette and custom cursor. Its
lower blade tip is the cut hotspot. The standard macOS Settings command opens a
Graph Tools pane with a 16–32 point cursor-size control; the default is 20 points.
The setting is persisted by macOS independently of graph documents and visual
palette exports.

One AppKit canvas responder captures each pointer sequence. Its interaction
controller owns node drag, pan, wire/rewire, knife and knot previews. SwiftUI
renders those previews and the document without graph drag gestures or delayed
mouse-up callbacks. Escape, tool changes, focus loss and viewport changes cancel
uncommitted motion. Node positions and every attached endpoint derive from the
same preview in the same frame. Axis sweeps from the last accepted position keep
the authored non-overlap gap, including large/coalesced moves and boundary rounding.
The authoring form does not observe per-frame pointer state.

Node rendering, port geometry and hit testing share reusable definitions. New
nodes require fixture/document data, not gesture code. Selection uses the saved
stroke, with no additional shadow state. Native glass remains on the connected
port beads and zoom control; the accepted flat node rendering and separate light
and dark palettes are preserved. Node-shadow blur and vertical offset are shared,
while opacity is authored per appearance.

The node silhouette has no port recesses. The Assets-to-Input fixture noodle
terminates beneath its ports. A disconnected port is only a small flat semantic
dot that remains fully opaque; its saturation, optional same-color stroke,
stroke width, and Light/Dark brightness can be authored. Connecting keeps that
dot at the same size and exact bead center, raises its separately authored
Light/Dark brightness, and adds the native glass bead beneath it. A separate
semantic-colored shadow belongs only to the connected bead and provides native
opacity, blur, and vertical-offset controls. There is no connection ring or
state animation. The node shadow is applied only to the node surface, so text
and ports never inherit or duplicate it.

The node body can independently use a native flat fill or native Liquid Glass.
Flat mode keeps the port beads as Liquid Glass and provides one node color plus
a selected-node stroke color for each of Light and Dark, independent of the
graph background. Stroke width is shared across appearances. Selection is
communicated by the stroke rather than a second shadow state.

Save Preferences stores the current appearance, both complete appearance
palettes, and the visual controls in Graph Lab's local, versioned `UserDefaults`
payload. It restores them on the next launch and migrates the earlier shared
palette into the appearance in which it was saved without losing the user's
choices. Camera pan/zoom, node positions, selection, and gesture state are not
saved. This developer convenience is intentionally separate from the future
Photara Lab import/export format and never enters a project document.

```console
platform/macos/photara-graph-lab/build-graph-lab.sh
open "platform/macos/photara-graph-lab/.build/Photara Graph Lab.app"
```

Shared primitives live in `platform/macos/photara-graph/Sources`. Production
Photara currently consumes the shared procedural background with its existing
defaults; further node integration waits for an accepted Graph Lab treatment.

## Interaction engineering and verification

[INTERACTION_MODEL.md](INTERACTION_MODEL.md) describes the event lifecycle and
serializable document boundary. [Tests/README.md](Tests/README.md) describes the
native acceptance matrix and seeded random gesture harness, including exact
failure replay. Run both with:

```console
platform/macos/photara-graph-lab/verify-interactions.sh
```

The verification app uses a separate preferences domain and reads the existing
visual payload without saving to the author's domain. Reports, compositor
screenshots, and seed traces are written to `/tmp/photara-graph-verification`.
