# Photara Inspector Lab

Independent host for `../photara-inspector`, with the exact sections Photara
uses. It requires no live bridge, project, source folder or provider.

Follow the shared [`DESIGN_LANGUAGE.md`](../DESIGN_LANGUAGE.md). This lab owns Inspector
content, sections, fields, native controls and Inspector states. Shell/Theme own its
single module base, title, selection tint, shared geometry and top-level glass. Metadata
sections use hierarchy and spacing instead of feature-owned cards or outer frames.

```sh
platform/macos/photara-inspector-lab/build-inspector-lab.sh
open 'platform/macos/photara-inspector-lab/.build/Photara Inspector Lab.app'
platform/macos/photara-ui-tests/verify-shared-ui.sh
```

Choose Disk, scanning Disk, Layout, single-cell Layout, missing package,
diagnostics, no selection, Graph hidden or no editable settings. Preview the assembled Inspector or an isolated
section; change appearance, evaluation label, enabled state, frame and cell.
Resize the divider to inspect constrained widths. Semantic callbacks are
recorded in the sidebar. The fixture adapter also reflects cell mode/crop/rotation
and scan-state changes; it is not a substitute implementation of Core's node
semantics. Structural commands are recorded only.

Hierarchy, spacing and controls are shared Swift sources; colors use semantic
theme roles. The Inspector preset owns its empty-state copy, SF Symbols, typography,
spacing, vertical position and maximum text width. Lab drafts are saved automatically.
**Apply to Photara** installs a validated local development override that the running
production Inspector reloads; Remove returns to the bundled default. There is no
second Inspector implementation. Lab selections and scenario controls are transient.

Every node has a contract Inspector. Node-specific authoring workspaces are
optional; Layout's authoring canvas lives separately in `../photara-layout`.
See [shared architecture and handoff](../SHARED_UI.md).

For the next Inspector task: own `photara-inspector/Sources` and this lab's
host/fixtures. Keep bridge translation in `InspectionAdapter.swift`. Changes to
`NodeInspection` must remain typed and be verified with the production adapter.
Run this lab and `../photara-ui-tests/verify-production-ui.sh` before handoff.
Finish with the commit, contract changes if any, and verification results;
`platform/macos/build-ui.sh` assembles the result into Photara.
