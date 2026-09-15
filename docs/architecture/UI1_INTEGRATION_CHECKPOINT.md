# UI1 integration checkpoint — 2026-09-15

Safe to commit as a verified UI1 checkpoint after review and separate authorization. The final full Graph gate passes **19,093 assertions / 0 failures**; no commit was performed.

Reviewable, uncommitted worktree: `/Users/suhail/.codex/worktrees/6569/photara`.
No commit, push, installation, service replacement or live-data operation occurred.
The [machine-readable report](verification/ui1-integration-20260915.json) records
source hashes, exact inventories, commands, outcomes, bundle preservation and gates.
Historical 2026-09-14/15 evidence imported from `12ef` remains historical.

## Base and reconciliation

`git ls-remote --symref origin HEAD` confirmed default branch `main` at
`c605475742f22581060ff009fcbcfd2a3acc4e36`. This fresh task worktree and UI1's
`12ef` worktree both started at that commit. No fetch, cherry-pick, reset, stash,
commit or source-worktree mutation was needed. The initial remote read was denied
inside the sandbox; the read-only host-access retry confirmed the tip.

| Input | HEAD | Divergence from remote main (unique / behind) | Dirty paths | Decision |
| --- | --- | --- | --- | --- |
| Main Dropbox checkout | `14f77ee` | 0 / 3 | 71 | Preserve in place; retain accepted remote onboarding/recovery history |
| UI1 `12ef` | `c605475` | 0 / 0 | 70 | Import exact source, schema, documentation and retained evidence delta |
| Graph context `3e99` | `4631c67` | 3 / 13 | 4 | Preserve separate ROADMAP/Theme Lab glass experiment; import none |
| COV0 `deb1` | `c605475` | 0 / 0 | 9 | Documentation proposal remains separate; import none |

UI1 has **40 modified tracked files and 30 new files**: 43 source/schema/configuration,
seven documentation and 20 retained evidence files. Ignored app/Graph/Foundation/UI
`.build` trees and `scripts/__pycache__` were not copied. Of the main checkout's 71
dirty paths, 38 already equal accepted main and 33 differ; the report lists each.
Those residual edits remain intact in that checkout, not reapplied over accepted
cloud recovery/account/native UI work. The source snapshot inventories all tracked
and nonignored untracked files, including unrelated user changes.

UI1 overlaps 20 main dirty paths, one Graph-context path (`ROADMAP.md`), and three
COV0 planning documents. Exact intersections and per-source SHA-256 are in the
report. Integration adds no product behavior beyond UI1. During the initial integration,
the extra executable changes were in `build-app.sh` and `verify-production-ui.sh`: SwiftPM manifest/Clang
module caches and SwiftPM cache/configuration/security directories now follow the
owned build root. `--scratch-path` alone had left manifest compilation targeting
a user-level compiler cache. The verifier also overrides inherited module-cache
settings before building. No oracle, tolerance, fixture seed or production Graph
source changed during integration.

## Contracts and schema

- Preserve signed native onboarding, immutable creation IDs, the empty saved
  `Graph 1`, canonical package closure, local catalog/projection/binding transaction,
  cloud registration/grant/projection/receipt transaction, duplicate suppression,
  cancellation, restart and lost-response reconciliation.
- Actual main has SQLite migrations 0001–0014 and reader/writer floors 4/4. UI1 adds
  **0015_project_creation.sql**, reader/writer floors **5/5**, and one retained intent
  table. All 14 earlier migration files remain byte-identical to main.
- PostgreSQL remains at **14 migrations / API floor 3**. All 14 migration files
  remain byte-identical. Package format remains **1.1**. No proposed cover/Gallery
  migration is activated or allocated.
- The disposable native SQLite fixture has 79 domain tables, 768 columns, 221 FK
  column entries, 190 triggers, 15 ledger entries, floors 5/5, successful integrity
  and no FK violations. There is exactly one completed creation, catalog record,
  Graph projection and device binding. Index count is 183 excluding the SQLx ledger
  index, or 184 including it; this is the same schema as the historical report.
- Native macOS 27 toolbar/sidebar/chrome remain OS-owned. Snapshot-only sizing and
  Graph harness layout/focus preconditions remain in test hosts. Light/Dark created
  project captures were visually inspected; the native toolbar and saved Graph
  surface are present. Existing Graph hint/zoom overlay layout is unchanged.

## Initial integration verification (retained historical results)

All builds and native gates ran sequentially. Rust used a clean private target;
production UI allocated its own fresh target and complete input bundle. No source
build products were copied. Logs and measured inventories are under
`/private/tmp/photara-ui1-integration-20260915`; exact command arrays and log hashes
are retained in the report.

| Gate | Exit | Result |
| --- | --- | --- |
| fmt | 0 | Rust formatting |
| fixtures | 0 | 12 canonical containers / 92 embedded byte records |
| schema | 0 | 46 relation signatures / 104 FK checks |
| naming | 0 | generation-two naming guard |
| ownership | 0 | 37 Graph ownership files; retained behavioral oracles |
| configuration | 0 | 7 product-configuration tests |
| rust | 0 | 289 tests passed; 12 PostgreSQL tests separately executed; 4 fixture generators ignored |
| check | 0 | workspace, all targets |
| clippy | 0 | workspace, all targets, warnings denied |
| authentication | 0 | 94 security + 58 network + 64 Opening assertions; synthetic/in-memory credentials |
| postgres | 0 | 12 PostgreSQL tests plus real native HTTP creation/recovery furnace; 59 tables / 14 ledger entries |
| bridge | 0 | UniFFI verification, Core/Graph/progress/cancellation bridge |
| signed-build | 0 | isolated Apple Development bundle, strict signature valid, Team 524GTA93Q3 |
| production-ui | 0 | 27 captures; creation/restart/cancel/duplicate and native toolbar assertions; hostile inherited settings ignored |
| shared-ui | 0 | 225 transitions / 50 contrast pairs / 117 captures and external native accessibility probes |
| graph | 1 | 17,469 assertions / 12 failures; all 12 configurations and 12×180 seeded actions completed |

The clean Graph gate completed with **17,469 assertions / 12 failures**, exit 1.
There were three original-edge disconnect failures, two surviving-branch failures,
one randomized branch-source/origin failure, and six native-focus assertions.
Failures occur in Light straight 0.55/1.0, Light curved 0.55, and the Light random
sequences. Permissions, HID and screen-capture preflight passed. Some menu
failure diagnostics record `focus=false/false`; that does not establish that all
behavioral failures are environmental. No compiler or other verifier was running
alongside Graph. The exact log, all failed messages and 12 completed seed/style/
appearance combinations are retained in the report. No replay, tolerance change,
or production Graph change was used to mask the result. This initial run was a real failed gate. The follow-up below resolves the blocker
with explicit host/focus hardening, corrected native hit-tests and a fresh full pass; the initial failure is
preserved in machine-readable evidence, not overwritten or reclassified.

Initial failures are retained honestly: the fixture guard ran before its codec
binary existed in the clean target, then passed after the explicit codec build.
Sandboxed authentication exited 133 because in-memory RSA key creation was denied;
private PostgreSQL exited 1 on `shmget`; bridge exited 1 because Swift manifest
compilation could not write its default module cache. Each passed on the scoped
host-access retry. Shell syntax and final whitespace checks also pass.

Production's hostile-environment run set invalid channel/profile values and five
forbidden app/Rust/generated/Clang/SwiftPM output paths. All five remained absent;
the integration worktree's default interactive `.build/app` remained absent.
Rust output, helper scratch, generated identities/bindings, module caches and
SwiftPM cache/configuration/security paths were under the verifier's fresh run root.

## Graph host/focus correction and final verification

### Changes and diagnosis

The SwiftUI `WindowGroup` relied on its scene `.task` to log readiness and start
verification. Controlled external replays found live but windowless macOS 27
processes that never reached that task. Test-only `GraphVerificationHost.swift`
now uses explicit `NSApplication`/delegate ownership of a named `NSWindow` hosting
the unchanged `GraphLabView`. It logs process entry, lifecycle and real event-
surface readiness; the furnace also asserts a populated native toolbar.

`launch-verification.py` directly owns the exact signed child executable, avoiding
an `open -W` process that can wait indefinitely without identifying its child.
Every run gets a fresh nonce and directory. Atomic process/ready records must match
that PID, nonce and window identity; success also requires the child's real exit
status to match its completion file. Startup is bounded to 15 seconds, execution
to 1,200 seconds. On timeout, bounded process state/sample diagnostics are retained
and TERM→KILL cleanup addresses only the owned child. There are no hidden retries.
The permission-only shell helper uses the same launcher.

Native menu selection previously left a delayed event-tracking timer alive after
tracking ended. The timer could act during another menu session. It now belongs
to one observed tracking instance and is invalidated on end/uninstall; menu
tracking itself has a deadline. Async focus acquisition/restoration brackets
native menu input, requiring three settled native turns with app active, test
window key, canvas first responder and the verifier's PID actually foreground.
Failed menu actions are never retried or treated as passes. Original gesture,
branch, oracle and tolerance assertions remain; only async plumbing changes their
call sites. Action33 of the historical random seed failed before its Y/tool-switch
key input; the unique cause is not established merely because replay now passes.
Failure-only diagnostics would identify its event/focus/wire state on recurrence.

The first full run after host/focus hardening completed **19,090 assertions / one
failure**, solely the zoom pointer-priority assertion. Its actual native zoom drag
passed. Read-only diagnosis found that three test calls passed receiver-local
points to [AppKit NSView.hitTest](https://developer.apple.com/documentation/appkit/nsview/hittest%28_%3A%29),
which requires the receiver's **superview** coordinates. The explicit flipped
hosting view exposed this test adapter error: diagnostics show the old y≈903
point selects the canvas while parent y=34 selects the control's hosting view.

A shared test-only adapter corrects the slider and both overview hit-tests. All
three original predicates, points and messages remain. Two independent native
fixture assertions exercise a translated/flipped hierarchy, and a new nonnil
slider assertion strengthens the original check. The targeted camera/overview
run passes **176 assertions / zero failures**, including native zoom
input. The failed full run remains embedded in the evidence.

### Measured gates, sequential and isolated

| Gate | Result |
| --- | --- |
| Cold signed launch | 3 assertions / 0 failures, native toolbar populated |
| Signed native permissions/capabilities | Accessibility, actual HID, ScreenCaptureKit and CGWindow probes pass; no permission dialogs |
| Launch/focus furnace | 8 real-process scenarios; 49 launcher checks + 60 native assertions / 0 unexpected failures |
| Exact Light/Straight seed 22597503690035777 | 180 original actions, 924 assertions / 0 failures |
| Native camera/overview target | 176 assertions / 0 failures, including independent hit-test fixture |
| Context-menu matrix | All 12 original matrix/branch configurations, 2,239 assertions / 0 failures |
| **Final full Graph** | **19,093 assertions / 0 failures**, all 12 configurations and 12×180 seeded actions; exit 0 |
| Ownership/retention | 40 Graph source/preset/test files, 49 frozen files; production/oracles/seeds/fixtures preserved |
| Guard regression | 23 in-memory mutation cases (11 host/focus + 12 hit-test) reject weakening/unapproved source changes |
| Launcher protocol checks | 46 non-GUI checks, separately labeled; not a substitute for real-process furnace |

Furnace cases: cold launch; deliberate windowless startup and recovery; deliberately
blocked-main-thread startup and recovery; ready-then-stalled runtime timeout and
recovery; genuine foreground interruption by another isolated native process,
restoration and native right/control-click menu delivery. The three injected
failures are expected timeout outcomes, individually recorded with bounded cleanup;
none is described as successful verification. All eight scenario contracts pass.

An initial compile failed at one missed async menu call and was corrected before
runtime checks; its log is retained. The intermediate 19,090/1 full run justified
the specific native hit-test adapter correction described above. After that change,
the focused camera check, cold launch, permissions, furnace, exact seed and context
matrix all passed before one new full run on the frozen signed binary. No production
change, oracle relaxation or seed change occurred. No concurrent build/verifier
was present and no interactive approval occurred during that final run.

The previous Rust/schema/fixture/native-authentication/PostgreSQL/bridge/shared/
production UI passes are retained with unchanged non-Graph code. Their source
hash comparison, fresh harness guard/build/syntax checks and full native Graph pass
are the proportionate affected gates; unrelated Rust/product UI suites were not
rerun merely for documentation and test-host changes.

Full command arrays, per-run logs/launch reports, assertion totals, all attempted
outcomes and source/bundle preservation are embedded or hashed in the
[machine report](verification/ui1-integration-20260915.json). Working evidence:
`/private/tmp/photara-graph-host-20260915`.

### Reproduce without rebuilding during the native gate

```sh
platform/macos/photara-graph-lab/verify-interactions.sh --build-only
platform/macos/photara-graph-lab/verify-permissions.sh --permissions-only
python3 platform/macos/photara-graph-lab/verify-launch-furnace.py --app \
  'platform/macos/photara-graph-lab/.build/verification/Graph Lab Verification.app'
python3 platform/macos/photara-graph-lab/launch-verification.py --app \
  'platform/macos/photara-graph-lab/.build/verification/Graph Lab Verification.app' -- \
  --random-only --seed 22597503690035777 --steps 180 --appearance light --style straight
PHOTARA_GRAPH_CONTEXT_MATRIX_ONLY=1 python3 platform/macos/photara-graph-lab/launch-verification.py --app \
  'platform/macos/photara-graph-lab/.build/verification/Graph Lab Verification.app'
# Final full gate: no host fault or targeted-only environment flags.
env -u PHOTARA_GRAPH_HOST_FAULT -u PHOTARA_GRAPH_CONTEXT_MATRIX_ONLY -u PHOTARA_GRAPH_PREFLIGHT_ONLY -u PHOTARA_GRAPH_CAMERA_ONLY \
  python3 platform/macos/photara-graph-lab/launch-verification.py --app \
  'platform/macos/photara-graph-lab/.build/verification/Graph Lab Verification.app'
```

No permission-request command belongs inside a final run. Prepare signing/TCC first;
the existing `--request-permissions` mode remains an explicit operator action only.
Next: review/authorize the UI1 checkpoint; then separately scope Library Lifecycle
per the accepted execution roadmap. COV0 docs can follow with the single documented
ROADMAP conflict and refreshed dependency wording; COV1/Gallery runtime remains gated.

## Protected apps, source inputs and remaining gates

Full before/after inventories compare every regular-file byte by SHA-256, entry
names, symlink targets and permission modes in all three existing source bundles.
Both signed bundles have 20 regular files; the old Graph-context ad-hoc bundle has
16. Team IDs are compared separately. The authoritative signed `12ef` app and the
new isolated signed bundle pass strict signature verification with Team
`524GTA93Q3` and the expected private desktop Keychain access group.

The older Dropbox checkout's signed bundle retains Team `524GTA93Q3` but fails
host-access strict verification because of a `com.apple.FinderInfo` attribute on
the bundle. The sandboxed initial signature check also failed (trust unavailable).
This task does not repair or re-sign that input. Byte preservation and signature
validity are separate facts; do not report all existing source apps as valid.

No live SQLite, Neon, Auth0, project package, installed service or Keychain entry
was read for mutation or changed. Actual Google sign-in, the live service's new
endpoint, and second-Mac hydration were not exercised. The real HTTP fixture uses
private PostgreSQL and file-backed synthetic credentials. Signed Keychain runtime
and service-process furnaces insert/delete synthetic Keychain entries, so they
were intentionally not run under this task's no-Keychain-mutation restriction.
Read-only signature/entitlement checks and synthetic authentication tests are the
current evidence. Runtime Keychain and matching desktop/service publication remain
separate acceptance gates. No deployment-readiness claim follows from a safe
checkpoint commit; launching the new desktop against live state upgrades it to
local floor 5 and requires separately authorized rollout.

## COV0 follow-on

COV0 is **not conflict-free**. A read-only `git apply --check` and disposable
three-way `git merge-file -p` find one textual conflict at the top of `ROADMAP.md`:
keep both the UI1 implementation entry and the separate COV0 proposal entry.
`docs/ROADMAP_0_2_EXECUTION.md` and
`docs/architecture/UI_LADDER_AND_AUTHORING_SEQUENCE.md` merge cleanly, as do the
five remaining tracked COV0 documents; its ninth path is the new proposal itself.
The subsequent documentation merge must refresh references to UI1 being only in
`12ef` and to this checkout lacking 0015. Preserve COV0's inert status and Library
Lifecycle ordering. Future migration allocation must inspect the eventual merged
baseline: local 0015 is occupied by UI1, service latest remains 0014. No cover,
Gallery, browser or lifecycle implementation is included here.

## Exact integration changed-file inventory

The original 70-file UI1 delta, two checkpoint report paths, and six additional
Graph harness/launcher paths total **78** changed files. Graph follow-up edits ten
runtime/guard files (six new delta paths plus four paths already in UI1) and the
existing handoff/implementation/evidence documents. No generated build product is
included. Source hashes and all input-worktree inventories remain in the report.

```text
ROADMAP.md
crates/photara-bridge/src/creation.rs
crates/photara-bridge/src/lib.rs
crates/photara-core/src/creation.rs
crates/photara-core/src/lib.rs
crates/photara-library/migrations/generation_two/0015_project_creation.sql
crates/photara-library/src/gen2/catalog.rs
crates/photara-library/src/gen2/creation.rs
crates/photara-library/src/gen2/local.rs
crates/photara-library/src/gen2/mod.rs
crates/photara-library/src/gen2/onboarding.rs
crates/photara-library/src/gen2/tests.rs
crates/photara-service/src/access.rs
crates/photara-service/src/creation.rs
crates/photara-service/src/http.rs
crates/photara-service/src/http/native_furnace.rs
crates/photara-service/src/lib.rs
crates/photara-service/src/onboarding.rs
crates/photara-service/src/onboarding_pgtests.rs
crates/photara-service/src/sync.rs
crates/photara-store/src/package/creation.rs
crates/photara-store/src/package/creation/filesystem.rs
crates/photara-store/src/package/mod.rs
crates/photara-store/tests/package_creation.rs
docs/ACTIVE_HANDOFF.md
docs/ROADMAP_0_2_EXECUTION.md
docs/architecture/UI1_CREATE_PROJECT.md
docs/architecture/UI1_INTEGRATION_CHECKPOINT.md
docs/architecture/UI_LADDER_AND_AUTHORING_SEQUENCE.md
docs/architecture/mockups/ui1/created-dark.png
docs/architecture/mockups/ui1/created-light.png
docs/architecture/verification/ui1-20260914/authentication.log
docs/architecture/verification/ui1-20260914/bridge.log
docs/architecture/verification/ui1-20260914/build.log
docs/architecture/verification/ui1-20260914/clippy.log
docs/architecture/verification/ui1-20260914/configuration.log
docs/architecture/verification/ui1-20260914/fixtures.log
docs/architecture/verification/ui1-20260914/graph.log
docs/architecture/verification/ui1-20260914/naming.log
docs/architecture/verification/ui1-20260914/ownership.log
docs/architecture/verification/ui1-20260914/postgres.log
docs/architecture/verification/ui1-20260914/process.log
docs/architecture/verification/ui1-20260914/production-ui.log
docs/architecture/verification/ui1-20260914/rasters.json
docs/architecture/verification/ui1-20260914/rust.log
docs/architecture/verification/ui1-20260914/schema.log
docs/architecture/verification/ui1-20260914/shared-ui.log
docs/architecture/verification/ui1-20260914/swift-format.log
docs/architecture/verification/ui1-creation-20260914.json
docs/architecture/verification/ui1-integration-20260915.json
platform/macos/photara-app/README.md
platform/macos/photara-app/Sources/AppModel.swift
platform/macos/photara-app/Sources/AppModelCreation.swift
platform/macos/photara-app/Sources/ApplicationAdapter.swift
platform/macos/photara-app/Sources/EditorSessionView.swift
platform/macos/photara-app/Sources/PhotaraMacApp.swift
platform/macos/photara-app/Sources/ProductionOpeningCloudDriver.swift
platform/macos/photara-app/Tests/ServiceFurnace/NativeServiceFlow.swift
platform/macos/photara-app/build-app.sh
platform/macos/photara-graph-lab/Tests/GraphBranchOverviewChecks.swift
platform/macos/photara-graph-lab/Tests/GraphInteractionChecks.swift
platform/macos/photara-graph-lab/Tests/GraphNativeInput.swift
platform/macos/photara-graph-lab/Tests/GraphRandomGestures.swift
platform/macos/photara-graph-lab/Tests/GraphVerificationHost.swift
platform/macos/photara-graph-lab/launch-verification.py
platform/macos/photara-graph-lab/verify-interactions.sh
platform/macos/photara-graph-lab/verify-launch-furnace.py
platform/macos/photara-graph-lab/verify-permissions.sh
platform/macos/photara-shell/Sources/ApplicationPresentation.swift
platform/macos/photara-shell/Sources/ApplicationShell.swift
platform/macos/photara-shell/Sources/CreateProjectView.swift
platform/macos/photara-ui-tests/Capture.swift
platform/macos/photara-ui-tests/ProductionUIChecks.swift
platform/macos/photara-ui-tests/README.md
platform/macos/photara-ui-tests/verify-production-ui.sh
platform/macos/shared-ui-sources.sh
scripts/generate_product_configuration.py
scripts/verify_ui0_contract.py
```
