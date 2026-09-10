# Graph task integration handoff

The existing Graph source, presets, lab and complete verification suite remain
the reference implementation. Gallery and Inspector now follow its shared-view
pattern; see [shared UI architecture](../SHARED_UI.md).

Own Graph rendering/interaction in `../photara-graph`, and authoring controls and
fixtures here. Production supplies bridge data/actions in
`../photara-app/Sources/ProductionGraphView.swift` and `GraphAdapter.swift`.
Do not copy a lab view into production. Preserve neutral selected node fills,
selection strokes, package-authored icon colors, and the versioned preset flow.

For each separate Graph task, run `build-graph-lab.sh`, the full
`verify-interactions.sh`, and `../photara-ui-tests/verify-production-ui.sh`.
Keep the native verification window foreground without mouse/keyboard activity.
Hand off the commit, shared source/preset changes and verification results.
`platform/macos/build-ui.sh` reassembles Graph Lab, Gallery Lab, Inspector Lab
and Photara. No separate Main App UI Lab is required.
