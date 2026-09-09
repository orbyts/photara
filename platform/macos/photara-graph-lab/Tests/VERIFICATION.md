# Interaction verification — 2026-09-09

`verify-interactions.sh` completed with **15,562 assertions and zero failures**.
The optimized normal app built successfully and was launched after verification.
`git diff --check` passed. No changes were staged, committed or pushed.

The native acceptance matrix ran in light and dark appearance, straight and
curved noodle styles, at 55%, 100%, and 180% zoom. It includes invalid direction,
same-node and incompatible-type drops; creation, fanout, replacement and rewire;
cutting and immediate subsequent gestures; routing knots; cancellation; native
menus; mixed buttons; and camera controls.

Routing-point checks cover branching, shared-point movement, duplicate and
invalid drops, cancellation, cutting individual branches or their common trunk,
and retaining the surviving branch's geometry. Overlapping noodles explicitly
select the topmost edge in every appearance/style/zoom configuration.

Overview checks cover all three visibility policies, fade timing, held and
renewed zoom sessions, all four corners, size and rounding, and live graph bounds.
The harness resizes the actual native window to wide, tall and square shapes,
checking both preview and viewport-outline aspect ratios against canvas bounds.
Resize during active gestures also checks cancellation and subsequent input.

The random harness completed **2,160 steps**: 180 steps for each combination of
two appearances, two noodle styles, and seeds `22597503690035777`, `12648430`,
and `20260909`. All **46 action categories** actually executed, including **48
native window resizes** followed by further gestures. Every completed step
and prerequisite was compared with the independent expected-state model.
Ineligible visible targets are explicit no-ops in the replay logs.

The same run checked 600 randomized swept node moves and DTO integrity. The
controller benchmark took 0.0814 seconds for 10,000 drag updates plus attached
path derivation on this machine. A separate optimized, no-window comparison
against `c022768` measured approximately 33 ms for both versions (nine-run
medians, repeated twice, within about 1%). These measure controller/path work,
not native frame rendering or hardware latency.

Native compositor captures were reviewed for the authored light/dark appearance,
connected/disconnected port beads, routing, live wire preview and overview corner
placements. Legacy knot DTO migration and old/new preference serialization were
checked, including preservation of every existing palette field.
The floating tool rail was verified as native chrome over (rather than part of)
the event surface. Random pan targets explicitly avoid its control hit region.
Pinned native shortcuts insert a unique, collision-free ordinary node DTO and
preserve document validation. The final build also includes the global Settings
toggle that hides the rail without modifying the graph controller.
The saved `graph-lab.visual-preferences.v1` payload remained byte-for-byte identical:
SHA256 `d62c94bf1c1fa2e5791178316809c13b21d4b53274c7c8917c6a5a1fbce97937`.
The four unrelated working-tree files named in the engineering handoff retained
their protected checksums. Node presentation, fixtures, icon resources and the
node design-language document remain unchanged.

Reports and replay logs are in `/tmp/photara-graph-verification`; see
[README.md](README.md) for commands and event-generation details. Native slider
tracking was exercised with public HID events. Physical trackpad pinch, momentum,
latency and cursor feel still require a hands-on check; magnification was tested
through the production responder with a typed NSEvent double.
