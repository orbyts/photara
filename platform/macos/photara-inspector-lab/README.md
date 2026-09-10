# Photara Inspector Lab

Independent host for `../photara-inspector`, with the exact sections Photara
uses. It requires no live bridge, project, source folder or provider.

```sh
platform/macos/photara-inspector-lab/build-inspector-lab.sh
open 'platform/macos/photara-inspector-lab/.build/Photara Inspector Lab.app'
platform/macos/photara-ui-tests/verify-shared-ui.sh
```

Choose Disk, scanning Disk, Layout, single-cell Layout, missing package,
diagnostics or no selection. Preview the assembled Inspector or an isolated
section; change appearance, evaluation label, enabled state, frame and cell.
Resize the divider to inspect constrained widths. Semantic callbacks are
recorded in the sidebar. The fixture adapter also reflects cell mode/crop/rotation
and scan-state changes; it is not a substitute implementation of Core's node
semantics. Structural commands are recorded only.

Hierarchy, spacing and controls are shared Swift sources; colors use semantic
theme roles. There is no second Inspector implementation and no redundant
Inspector preset. Change the shared component and rebuild this lab and Photara.
Lab selections and scenario controls are transient.

Every node has a contract Inspector. Node-specific authoring workspaces are
optional; Layout's authoring canvas lives separately in `../photara-layout`.
See [shared architecture and handoff](../SHARED_UI.md).

For the next Inspector task: own `photara-inspector/Sources` and this lab's
host/fixtures. Keep bridge translation in `InspectionAdapter.swift`. Changes to
`NodeInspection` must remain typed and be verified with the production adapter.
Run this lab and `../photara-ui-tests/verify-production-ui.sh` before handoff.
Finish with the commit, contract changes if any, and verification results;
`platform/macos/build-ui.sh` assembles the result into Photara.
