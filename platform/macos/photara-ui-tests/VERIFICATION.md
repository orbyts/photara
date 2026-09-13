# Shell integration acceptance — 2026-09-10

Work is integrated directly on `main`, including the five reviewed launcher/roadmap
handoff files. The historical extraction report below describes its earlier branch.

- `platform/macos/build-ui.sh`: Graph, Gallery, Inspector, Shell Lab and Photara built.
- Shell Lab independently built and opened its native 1440×900 Preview and 360×760
  Authoring windows. Its executable links SwiftUI/AppKit without `photara_bridge`.
- Shared UI checks: shell preset validation/roundtrip/rejection, typed availability,
  first-node selection, stable no-selection Inspector, explicit Gallery/diagnostics,
  project-session reset, Layout/Review capability, catalog handoff, cancellation,
  existing Gallery/Inspector/HDR contracts and 54 Light/Dark compositor captures.
- Production UI checks: launcher and compact empty states, real Core actions,
  undo/redo, HDR preview, binding/save and graph digest isolation passed.
- Bridge verification: passed at graph revision 19, with evaluation and cancellation.
- Full unchanged Graph suite: **15,562 assertions, 0 failures** in the isolated rerun.

Visual inspection covered both appearances, launcher/recents, empty Graph,
Inspector no-selection, compact pane sizing and production Graph assembly. Snapshot
review caught and corrected a vertically centered panel header and compact width
caps. A Swift compiler method-reference conversion crash was avoided with an
explicit binding closure. Neither fix changed feature renderers or Graph tests.

An initial full Graph run reported native focus failures and a branch-menu failure;
its result was not accepted as green. The rerun uses the exact unchanged verification
binary, with other native test windows closed and no concurrent native UI tests.

Ownership: every node has an Inspector and may provide a node-owned work surface.
Layout's surface stays in `photara-layout`. Production dispatch uses a node-contribution registry, with `photara.layout.work-surface`
as the first registered renderer. Toolbar icons come from the contributing node;
other contributions cannot accidentally route to Layout. A future Layout Node Lab, additional node
surfaces, a specialized review workflow, cloud sync and physical HDR luminance checks
remain separate work. Shell Lab's Review scenario hosts the shared Gallery; its
sync status is a deterministic fixture, not a new sync implementation.

---

# Historical shared UI extraction acceptance

Verified on the supplied macOS/Xcode host from `codex/shared-ui-labs`, based on
Graph/HDR commit `4631c67`. Graph renderer, interaction source, preset, test
sources and both pre-existing verification scripts remain unchanged.

| Command / check | Result |
| --- | --- |
| `platform/macos/build-ui.sh` | All three labs and Photara built successfully |
| `photara-gallery-lab/build-gallery-lab.sh` | Independently built, including final retained-dimensions adapter boundary |
| `photara-inspector-lab/build-inspector-lab.sh` | Independently built |
| `photara-theme/build-theme-lab.sh` | Existing Theme host and inherited Glass experiment built successfully |
| `photara-graph-lab/verify-interactions.sh` | Full unchanged suite: **15,562 assertions, 0 failures** |
| `photara-app/verify-bridge.sh` | Passed; graph revision 19; evaluation and cancellation callbacks verified |
| `photara-ui-tests/verify-shared-ui.sh` | Passed; preset roundtrip/rejection, HDR float data/native range/geometry, retained image dimensions, editor preference isolation, callback targets, 26 compositor captures |
| `photara-ui-tests/verify-production-ui.sh` | Passed; actual EditorSessionView/Graph/Gallery/Inspector/Layout assembly, typed adapter/Core edits, undo/redo, HDR proxy generation, binding, save, graph-digest viewing isolation |
| `cargo test -p photara-bridge` | 7 passed, 0 failed |
| `cargo clippy -p photara-bridge --all-targets -- -D warnings` | Passed |
| Actual application launches | Graph Lab, Gallery Lab, Inspector Lab and Photara each opened independently |
| Visual inspection | Gallery photo/square states in both appearances, Disk/Layout Inspector and isolated frame/cell section, real app launcher, production Graph/Layout composition |

Paths in the table after the first row are relative to `platform/macos`.
Rust checks used the existing app-local Cargo target directory. Final shared and
production check executables were built with `--build-only`, then run sequentially
to keep their native windows separate from Graph's physical-input suite.

An earlier direct Graph run was interrupted by native focus/pointer failures.
A LaunchServices run lacked Accessibility/screen-capture permission. The final
run through the documented script passed with all assertions intact. No Graph
behavior changes were made to accommodate the tests.

The production harness initially omitted Layout's required explicit AssetSet
input, then incorrectly assumed an internal project's recent entry had an
external document path. These were fixture/reporting errors, corrected in the
harness. The production semantic contract was retained.

Native compositor captures are under `.build/snapshots` and
`.build/production-snapshots`. Actual HDR panel luminance, physical trackpad feel,
and broader Disk/NAS behavior remain hardware/behavior follow-ups; the tests
verify native HDR policy and extended-range data, not display brightness.

Inherited work remains deliberately uncommitted: `ROADMAP.md`,
`photara-theme/Sources/ThemeLabView.swift`, and the untracked
`photara-theme/Sources/GlassTestScene.swift`. The inherited Theme build addition
was integrated into the refactor's optional experiment-source support. Theme
roles and the Glass source were preserved.
