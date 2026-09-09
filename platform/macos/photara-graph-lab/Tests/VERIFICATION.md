# Interaction verification — 2026-09-09

`verify-interactions.sh` completed with **13,438 assertions and zero failures**.
The optimized normal app built successfully and was launched after verification.
`git diff --check` passed. No changes were staged, committed or pushed.

The native acceptance matrix ran in light and dark appearance, straight and
curved noodle styles, at 55%, 100%, and 180% zoom. It includes invalid direction,
same-node and incompatible-type drops; creation, fanout, replacement and rewire;
cutting and immediate subsequent gestures; routing knots; cancellation; native
menus; mixed buttons; and camera controls.

The random harness completed **2,160 steps**: 180 steps for each combination of
two appearances, two noodle styles, and seeds `22597503690035777`, `12648430`,
and `20260909`. All 32 action categories actually executed. Every completed step
and prerequisite was compared with the independent expected-state model.
Ineligible visible targets are explicit no-ops in the replay logs.

The same run checked 600 randomized swept node moves and DTO integrity. The
controller benchmark took 0.073 seconds for 10,000 drag updates plus attached
path derivation on this machine. That measures controller/path work, not native
frame rendering or hardware latency.

Native compositor captures were reviewed for the authored light/dark appearance,
connected/disconnected port beads, routing and the legitimate live wire preview.
The saved `graph-lab.visual-preferences.v1` payload remained byte-for-byte identical:
SHA256 `d62c94bf1c1fa2e5791178316809c13b21d4b53274c7c8917c6a5a1fbce97937`.
The four unrelated working-tree files named in the engineering handoff retained
their original checksums.

Reports and replay logs are in `/tmp/photara-graph-verification`; see
[README.md](README.md) for commands and event-generation details. Native slider
tracking was exercised with public HID events. Physical trackpad pinch, momentum,
latency and cursor feel still require a hands-on check; magnification was tested
through the production responder with a typed NSEvent double.
