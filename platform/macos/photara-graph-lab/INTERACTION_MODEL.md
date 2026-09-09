# Graph interaction model

## Ownership and lifecycle

`PhotaraGraphEventView` is the sole pointer responder for the graph. The nodes,
ports, noodles and knots have no SwiftUI gestures. The bottom zoom slider and
sidebar remain native SwiftUI controls above/outside the event surface.

At mouse-down, the responder captures the button and the controller chooses one
interaction. Only that button's drags and release are accepted until the sequence
ends. A second button cannot steal the sequence. Cancellation makes the controller
idle, but the responder consumes the original sequence's remaining events; those
events cannot recreate a preview. Focus loss clears capture because AppKit may
never deliver a matching release. A later drag alone can never begin an interaction.

The only local event monitor forwards an unmodified Y key over the canvas when
the canvas is not yet first responder and no text editor owns focus. It neither
monitors nor consumes pointer events. All normal keys, pointer events, wheel and
magnification events use the native responder. There are no delayed cleanup tasks.

| State | Mouse movement | Valid release | Cancellation |
| --- | --- | --- | --- |
| Node drag | Collision-constrained position preview | Commit position | Discard preview |
| Wire | Free endpoint and nearest compatible input | Connect/replace or rewire atomically | Preserve document |
| Knife | Accumulate every edge touched by each slice segment | Remove crossed connections | Restore all edges |
| Routing knot | Knot position preview | Commit knot position | Discard preview |
| Canvas pan | Camera delta from down position | Retain camera | Restore initial camera |
| Pressed node content/empty input/noodle | Retain ownership without panning | End sequence | End sequence |

Escape, tool changes, focus loss, viewport changes and document replacement
cancel the current interaction. Releasing Y before releasing the pointer cancels
a slice. Wheel/trackpad camera updates are ignored during a captured pointer
sequence, preventing geometry from moving under a wire, knife or node drag.

## Picking and geometry

Picking happens once at mouse-down, in this order:

1. Nearest port in its invisible target.
2. Node body, which occludes knots and wires beneath it.
3. Routing knot.
4. Noodle stroke.
5. Empty canvas.

Port targets expand horizontally in screen space at low zoom, while their
vertical extent stays below half the port-row spacing. Adjacent ports therefore
remain distinct. Dropping uses a wider compatible-input target. The release
position is authoritative; no previous hover target is used as a fallback.

Rendering, picking, snapping and cutting share `PhotaraGraphGeometry` paths and
port positions. Node rows derive from stable port definitions. Node and endpoint
rendering read the same controller preview, with no separate offset cache.

The knife caches stroked paths once at the start of a slice and tests each
pointer segment at at most one screen-pixel intervals. Its stroke tolerance
covers those sample intervals. Every intersected connection is included, even
when the pointer moves across several wires in one coalesced event. Deleted
connections own their knots, so removing an edge also removes its routing state.

## Document boundary

`GraphDocument.swift` imports Foundation only. The Codable document contains:

- Nodes: opaque stable ID, kind, title, subtitle, position and port definitions.
- Ports: stable key, input/output direction, data type and label. Array order
  determines rows; identity never depends on a row index or display label.
- Connections: stable ID, source and destination port IDs, optional routing knot.

Graph validation rejects missing/invalid endpoints, incompatible types, duplicate
node/port/connection identities, multiple incoming edges and nonfinite positions.
Every connection must run from an output to an input on a different node;
output-to-output, input-to-input and same-node edges are rejected both by the UI
and the document transaction. Dragging a connected input moves the destination
of its existing output-to-input edge.
A single connection transaction handles duplicate creation, replacing an occupied
input and rewiring while preserving the reconnected edge's ID and knot.

The document does not contain AppKit events, SwiftUI state, gestures, hover,
selection, camera, geometry caches or transient endpoints. Camera and visual
preferences are Lab UI state. The existing legacy saved-knot preference is still
read/written for compatibility, but its in-memory owner is its connection.

## Rendering and performance

`GraphLabView` retains authoring controls and the unchanged versioned preference
payload. `GraphLabCanvas` isolates observable pointer/camera state from that form.
Nodes use the accepted SwiftUI surfaces, semantic dots and connected glass beads.
One synchronous Canvas draws all noodles and knots. Pointer updates disable
animation; no physics, timers, or extra selection shadows participate in dragging.
The build uses Swift optimization. Cursor rectangles are invalidated only when
the cursor actually changes, not on every drag sample.

Interaction code contains no Source/Transform/Composite branches. Those names
and initial positions occur only in Lab fixtures and authoring reset behavior.
