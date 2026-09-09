# Native Graph Lab gesture verification

[VERIFICATION.md](VERIFICATION.md) records the latest completed acceptance run.

Run the full suite from the repository root:

```console
platform/macos/photara-graph-lab/verify-interactions.sh
```

It builds a separate optimized verification app containing the real Graph Lab
view, geometry, controller and AppKit event surface. It opens its own window and
uses a separate preferences domain with the author's saved visual payload as a
read-only fallback. It never presses Save Preferences or writes that payload.
The app exits nonzero on failure. It does not require XCTest or a Rust build.

## Acceptance matrix

`GraphInteractionChecks.swift` sends native NSEvents through the actual NSWindow
for left/middle button sequences, keyboard cancellation, selection and native
slider interaction. Slider dragging additionally posts public HID mouse events
because native SwiftUI controls consult physical button state. The verification
process needs Accessibility permission for that check; it restores the pointer
after each slider gesture. It checks both appearances, straight/curved paths, and zoom
55%, 100%, and 180%. It covers:

- Immediate port creation, empty drops, fanout, duplicates and occupied inputs.
- Valid/invalid rewires and merging an already-identical destination.
- Rejected output-to-output, unconnected input-to-input, same-node and
  incompatible-type drops, with the document unchanged.
- Every node's continuous endpoint positions and document commit boundary.
- Single/multiple cuts, then immediate pan, zoom, node movement and rewiring.
- Knot addition, movement, deletion, and edge deletion/cutting with knots.
- Escape, real window focus loss, mode change and resize during every drag kind.
- Late drag/up events after cancellation, mixed buttons and outside releases.
- Native contextual-menu commands, middle button, wheel, precise scrolling and
  the bottom native SwiftUI slider.
- DTO roundtrips, invalid documents and randomized swept node collisions.

AppKit has no public constructor for a magnification NSEvent. Pinch verification
uses a typed NSEvent test double through the production `magnify(with:)` responder.
Wheel and precise scrolling use public CGEvent construction. Physical trackpad
pinch, momentum feel, hardware latency and cursor feel still need a hands-on check;
the automated suite does not claim to simulate the trackpad hardware/driver.

## Random gesture sequences and independent oracle

`GraphRandomGestures.swift` shuffles the complete gesture vocabulary, then freely
mixes actions using a seeded SplitMix64 generator. The default is three seeds in
each appearance and path style. Every completed action is compared with
`GraphGestureOracle.swift`, an independent expected-state model. The oracle never
calls production connection operations, hit testing, collision resolution or
camera transforms. It maintains its own nodes, edges, knots, and camera, and uses
separate Bezier sampling to predict visible wire targets and knife intersections.

The 32-action vocabulary includes connecting, abandoning a wire after hovering a
valid input, duplicate creation, input replacement, rewiring/abandoning a rewire,
node movement, left/middle pan, wheel/pinch/precise scrolling, both slider limits,
routing knots, Delete, cutting, native context-menu operations, invalid
output-to-output/input-to-input attempts, and interruption by Escape, focus loss
or tool switching. Targeted lifecycle tests additionally cancel all drag kinds. Long
random sequences retain prior graph and camera state across actions.
When zoom/scroll moves a target outside the canvas or beneath the slider, a
recorded native middle-pan brings it back into view before the next gesture.
That prerequisite is checked against the oracle too.

All persistent connections must run from an output to a compatible input on a
different node. Dragging an already-connected input to another input remains a
valid rewire: it moves the destination of the original output-to-input edge.
It does not create an input-to-input connection.

After **each** action the suite compares topology, connection identity, routing,
node positions, camera pan/zoom, attached endpoint positions, active port beads,
and empty transient/knife/capture state. Fresh random UUIDs are accepted only
when the intended topology matches; the oracle then requires those IDs to stay
stable. Invisible/occluded knot or wire targets are recorded as explicit no-ops.
Default runs require every action category to have actually executed.

Run a longer replayable sequence:

```console
platform/macos/photara-graph-lab/verify-interactions.sh --random-only --seed 12648430 --steps 1000 --appearance dark --style curved
```

Omit the appearance/style filters to run the seed in both appearances and styles.
Reports are in `/tmp/photara-graph-verification`:

- `result.txt`: assertion count and failures.
- `random-SEED-APPEARANCE-STYLE.log`: exact replay command and ordered action
  parameters, prerequisites, no-ops and expected state after every step.
- `*.actual.json`: the persistent document at a random-sequence failure.
- Appearance screenshots: actual native compositor captures (ordinary AppKit
  view snapshots omit Canvas and native glass layers).

The random suite stops a failing sequence at its first mismatch and prints its
seed/step. Other seeds still run. Replaying with `--steps` set to the failure step
plus one reproduces the same action prefix. Test fixture IDs and initial values
are deterministic; no time-based random input affects a sequence.
