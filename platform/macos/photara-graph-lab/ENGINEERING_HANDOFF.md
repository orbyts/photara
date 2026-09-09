# Graph Lab interaction engineering handoff

## Objective

Graph Lab is visually close to the desired result. The next task is not another
small gesture patch. Review and refactor the complete graph interaction
architecture so node, port, noodle, cutting, panning, and zooming behavior is
deterministic, smooth, reusable, and ready to exchange document data with the
Rust core that will eventually own Photara's graph data and operations.

Limit implementation work to Graph Lab and the reusable Graph presentation
module. Continue directly from the current working tree. Preserve the accepted
visual design, current node/noodle/port appearance, and saved authoring
preferences exactly unless a rendering change is strictly required to correct
interaction behavior and is called out before making it.

## Repository safety

The working tree contains unrelated user work that must survive:

- `ROADMAP.md`
- `platform/macos/photara-theme/Sources/ThemeLabView.swift`
- `platform/macos/photara-theme/build-theme-lab.sh`
- `platform/macos/photara-theme/Sources/GlassTestScene.swift` (untracked)

Do not use a broad reset, `git restore .`, or otherwise stage or modify those
files. Graph Lab's current uncommitted work is in:

- `platform/macos/photara-graph-lab/README.md`
- `platform/macos/photara-graph-lab/Sources/GraphLabView.swift`
- `platform/macos/photara-graph/Sources/GraphPresentation.swift`

The last committed Graph Lab checkpoint is `2100257` (`perf: optimize graph
node dragging`). The current uncommitted Graph changes are intentional design
progress, but the interaction implementation is not an accepted engineering
baseline until the checklist below passes.

## Current blocking defect

Reproduction:

1. Start with Source/Assets connected to Transform/Input.
2. Hold `Y` and drag across the noodle to cut it.
3. Move the Source node or try to pull a new noodle from Assets.

Observed failures across recent implementations:

- A transient noodle remains attached to Source with a dangling free endpoint.
- A subsequent port drag pans the canvas instead of creating a noodle.
- In some revisions, cutting stops working entirely.
- Once the bad state occurs, a new noodle cannot reliably be pulled until the
  interaction state is reset or the app is relaunched.

Recent patches alternated between an AppKit local event monitor and SwiftUI
drag gestures. Cleanup hooks did not solve the underlying ownership conflict.
Inspect the event flow from first principles. Prefer one explicit interaction
state machine and one owner for each pointer sequence; do not continue layering
special-case resets onto the current behavior.

## Non-negotiable interaction requirements

### Interaction state and lifecycle

- At most one primary interaction is active: idle, node drag, canvas pan, wire
  drag/rewire, knife slice, or routing-knot drag.
- A pointer sequence has one owner from mouse-down through mouse-up/cancel.
- Transient state never survives its gesture, Escape, focus loss, mode change,
  or an invalid drop.
- Persistent noodles exist only as connections with two valid port endpoints.
  No completed action may leave a free/dangling endpoint.
- Port active/connected appearance is derived from persistent connections plus
  the one legitimate active wire gesture, never from stale presentation state.
- Switching tools must cancel the prior interaction predictably without
  poisoning the next gesture.

### Ports and connections

- Left-press and drag on an output immediately pulls a live noodle. There is no
  hold duration or timing trick.
- The visible small port may use a larger invisible hit target, but adjacent
  ports must remain unambiguous.
- Dropping near a compatible input connects simply and predictably.
- Dropping on empty canvas cancels the new noodle. Rewiring an existing input
  and dropping invalidly preserves the original connection.
- One output may fan out to multiple different inputs.
- An input accepts one incoming connection.
- The same source port and destination port pair must never be duplicated.
- Dragging a connected input rewires its existing connection.
- Moving any node updates every attached noodle continuously in the same frame;
  Source, Transform, Composite, and future nodes use exactly the same code path.

### Cutting and routing

- Holding `Y` shows the smaller downward-pointing knife cursor.
- One slice cuts every noodle intersected by the slice, not only the first.
- Cutting leaves both ports disconnected as appropriate, removes connected-bead
  state, and leaves no transient endpoint.
- Normal port creation, node dragging, and canvas panning work immediately after
  a cut without an extra click or relaunch.
- Straight and curved noodle styles remain available.
- A routing knot can be added, moved to bend a noodle, selected, and deleted.
- Deleting/cutting a connection also removes its routing-knot state.

### Node behavior

- Nodes cannot overlap other nodes.
- Node dragging is smooth and low latency.
- Selection uses the authored stroke; do not reintroduce a separate selected
  shadow or a blue selection border.
- The one-, three-, and six-port specimens are fixture data for the same reusable
  node implementation, not three special cases.

### Pan and zoom

- Zoom scales the entire graph consistently: nodes, text, ports, noodles, knots,
  spacing, and hit behavior.
- Support pointer-centered mouse-wheel zoom, trackpad pinch zoom, and the bottom
  zoom slider.
- Support middle-mouse panning with the hand cursor and native trackpad
  two-finger scrolling.
- Empty-canvas drag may pan, but it must never win a gesture that started on a
  node, port, noodle, or knot.
- Pan and zoom must remain smooth under the one-/three-/six-port fixture and
  should not introduce per-frame animation or unnecessary visual computation.

## Accepted visual direction to preserve

- Treat the currently running Graph Lab's appearance as the accepted visual
  baseline. This task is an engineering cleanup of behavior, not a redesign.
- Do not reset preferences or substitute new default colors. Existing light and
  dark authored values must load unchanged after the refactor.
- Nodes currently use the performant flat presentation; Liquid Glass is
  reserved for connected port beads and the floating zoom control.
- Disconnected ports are small, opaque, desaturated flat semantic dots.
- Connected ports gain a separate Liquid Glass bead; only that bead casts its
  authored active-port shadow. The semantic inner dot becomes brighter.
- No per-port or grouped recess treatment.
- Light and dark palettes have separately authored colors where contrast needs
  differ. Semantic port hues and geometry controls may be shared.
- Preferences, including all chosen colors, remain saved and load on the next
  iteration.
- Continue using Apple-provided SwiftUI/AppKit controls and effects. Performance
  and snappy interaction take priority over ornamental effects.

## Modularity and Rust DTO boundary

Separate persistent graph data from ephemeral UI interaction state.

The persistent, serializable graph representation should be expressible as
plain value types suitable for mapping to/from a Rust DTO, including at least:

- Stable node ID and node kind/type.
- Node position and any document-owned node properties.
- Stable port identity/direction/type.
- Stable connection ID, source port ID, destination port ID, and optional routing
  knot(s).
- Camera pan/zoom only if it is document-owned; otherwise keep it in UI state.

Do not put SwiftUI gesture objects, AppKit events, view geometry caches, selection,
hover, knife state, or transient wire endpoints in the DTO. Keep those in a
single UI interaction controller/state machine. The SwiftUI views should render
from persistent graph state plus a read-only interaction snapshot and send
semantic actions back to the controller.

New node definitions must automatically inherit rendering, geometry, collision,
port hit testing, connection tracking, and drag behavior. Avoid branching on
Source/Transform/Composite IDs in interaction logic.

## Required review and verification

Before editing, inspect `git status`, the Graph-only diff, recent history, and
the current event/gesture flow. Then exercise this matrix in both light and dark
appearance where relevant:

1. Pull Source/Assets to Transform/Input; invalid empty-canvas drop; reconnect.
2. Fan Source/Assets out to two distinct inputs.
3. Attempt the identical source/destination pair twice; verify one connection.
4. Replace an occupied input and rewire a connected input.
5. Move Source, Transform, and Composite separately while attached; endpoints
   remain glued and equally smooth.
6. Y-slice one noodle, then immediately create another from the same output.
7. Y-slice across multiple noodles; all crossed noodles disappear, no others do.
8. After every cut, immediately pan, zoom, move a node, and create/rewire a wire.
9. Add/move/delete a routing knot, then cut/delete its noodle.
10. Cancel wire, node, pan, knot, and knife interactions with Escape/focus loss.
11. Repeat interactions at minimum, 100%, and maximum zoom.
12. Confirm no dangling previews, duplicate edges, stuck glass beads, unexpected
    canvas pans, or interaction that requires timing knowledge.

Build with:

```sh
platform/macos/photara-graph-lab/build-graph-lab.sh
```

Run:

```sh
open -n "platform/macos/photara-graph-lab/.build/Photara Graph Lab.app"
```

The engineering pass is complete only when the matrix is verified, the Graph
Lab build succeeds, `git diff --check` succeeds, and unrelated working-tree
changes remain untouched. Document the final interaction model and any manual
verification that still requires the user.
