# Photara Graph Lab

Graph Lab is a small macOS developer utility for evaluating Photara's reusable
Graph presentation primitives without opening a project or building the Rust
bridge. It owns fixture data and temporary authoring controls only.

The current slice compares procedural major/minor background patterns, native
glass treatments, node corner radius, round versus pill ports, port edge
offset, and a single simple shadow model. There are deliberately no status
indicators or Graph semantics.

Lines, Dots, and Crosses share aligned minor and major phases while the camera
pans or zooms. Graph Lab authors their colors, opacity, size or line width, and
major interval independently; the Graph canvas color is authorable separately,
and dense minor marks fade before they alias.

The deterministic scene has one-, three-, and six-row specimens plus fixed
noodles beneath the nodes. Empty-canvas dragging pans the Graph, direct node
dragging moves only that node, and Center Scene restores the camera and fixture
positions. Lab fixture nodes maintain a small non-overlap gap while dragging;
this remains presentation behavior rather than Core semantics. The selected
node uses separately authored native glass treatment and tint settings. Ports
are independent native glass beads with their own
Regular/Clear treatment, tint color, and tint amount controls; Reduce
Transparency restores the solid semantic port treatment. Unselected and
selected node glass likewise author tint color and amount independently. A
small SDR color core remains above each bead
so fixture port kinds stay legible without participating in refraction. Apple
exposes Regular and Clear rather than a continuous blur/transparency value, so
Graph Lab does not present a fictitious optical slider.

Save Preferences stores the current appearance and visual authoring controls in
Graph Lab's local, versioned `UserDefaults` payload and restores them on the next
launch. Camera pan/zoom, node positions, selection, and gesture state are not
saved. This developer convenience is intentionally separate from the future
Photara Lab import/export format and never enters a project document.

```console
platform/macos/photara-graph-lab/build-graph-lab.sh
open "platform/macos/photara-graph-lab/.build/Photara Graph Lab.app"
```

Shared primitives live in `platform/macos/photara-graph/Sources`. Production
Photara currently consumes the shared procedural background with its existing
defaults; further node integration waits for an accepted Graph Lab treatment.
