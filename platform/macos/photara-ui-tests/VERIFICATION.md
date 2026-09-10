# Shared UI extraction acceptance

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
| `photara-ui-tests/verify-shared-ui.sh` | Passed; preset roundtrip/rejection, HDR float data/native range/geometry, retained image dimensions, workspace preference isolation, callback targets, 26 compositor captures |
| `photara-ui-tests/verify-production-ui.sh` | Passed; actual WorkspaceView/Graph/Gallery/Inspector/Layout assembly, typed adapter/Core edits, undo/redo, HDR proxy generation, binding, save, graph-digest viewing isolation |
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
