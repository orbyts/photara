# Active handoff

This is the sole current operational handoff. Historical handoffs and detailed
experiment reports are reference material only.

## Baseline

- Authoritative branch: `main`.
- SSH publication recovered on 2026-10-10 without changing authentication or
  remotes. Verified checkpoint `e78d10c` and its local ancestors were pushed
  normally to `origin/main` (previously `79c2684`). Preserve that history.
- Historical production-code baseline before shared PS2 implementation:
  `f91f8de1546896267bac13adeac00c1659133232`. Shared `package::v1_3` reader
  components now follow the narrow 2026-10-03 approval; production writes remain disabled.
- The current `main` commit containing this file is the handoff publication;
  resolve it with `git rev-parse HEAD`. Remote synchronization was restored with
  explicit user approval: verified history through `6290b74` was pushed normally
  to `github.com/orbyts/photara` on `origin/main`. Preserve that history; do not reset
  to the historical remote baseline or repeat the old access troubleshooting.
- The working tree must be clean. Do not modify the unrelated dirty
  `codex/promote-graph-lab` worktree.
- Preserve stash `ce39772a4008c886265ac9a25485b2970d0f8332` until the post-LL2a
  audit. Do not apply or drop it during PS2–LL2a work.

## Critical path and active gate

**Current work — efficient checkpoint recovery:** the approved two-field v4
commitment correction is implemented and verified in local commit `3015282`.
Shared v4 save/retry and cold-reopen tests pass, with on-demand archived
dedupe and conservative position-only undo retirement at turnover. Next are
native adapter/dispatch and measured save/reopen/resource-use gates. The running review app has not been replaced;
this is not hands-on performance acceptance.

The [two-field v4 correction](architecture/PS2_RETENTION_STORAGE_DECISION.md#narrow-v4-commitment-correction--pending-approval-2026-10-11)
was approved by Suhail on **2026-10-11**. Both v4-only inherited commitments now
use `base_head_sha256`; old codecs and all other checks retain their meanings.
Do not ask again. Native/session runtime dispatch remains gated on the remaining
implementation and qualification. On 2026-10-15, normal SSH publication recovered:
verified local history through `680a2e5` was pushed to `origin/main` from
`ce9aa39`, without authentication or remote changes. Earlier publication failures
below are historical; preserve the published history.

The independent activation slice now passes: registered local-selector protocol
with five post-effect cuts, exact retry and third-selector refusal; PHPSJ002
seeded Core edit/retry/undo; selected-frame loading that tolerates unrequested
archive corruption but refuses changed selected frames/tighter bounds. Ten
focused tests pass (`/private/tmp/ps2-checkpoint-activation-final.log`), frozen v1
golden passes (`/private/tmp/ps2-checkpoint-activation-v1-golden.log`), strict store
tests pass (`/private/tmp/ps2-checkpoint-activation-clippy.log`). This is an
in-memory adapter with no native implementation/effects or Saved grant.
**V4 connected gate passes:** both approved HEAD-commitment fields are implemented.
Two consecutive operations (edit then undo) produce independently verified clean
packages. Exact original/suffix regeneration tolerates unrequested archived
receipt corruption; old/both field names, changed HEAD/witness/extent/suffix and
altered candidates refuse. Shared-executor partial-append and post-publication
unknown-outcome retries pass. Archive ownership updates use existing sparse
locator batches, preserve full charges and keep archive-only objects out of live
role locators. Thirteen focused tests including frozen v1/v2/v3 chains pass
(`/private/tmp/ps2-v4-connected-final.log`); strict store tests pass
(`/private/tmp/ps2-v4-connected-clippy.log`). The earlier generated-reader failure
is resolved, and the temporary pure-v4 gate is removed. No native adapter or
session runtime dispatch is enabled by this result.
**Cold-reopen gate passes:** registered recovery checks selector lineage,
original-bound BaseReceipt and selected AcceptanceBase, reconstructs Core from
the current independently verified package, and replays only pending semantic
edits. Requested archive lookups authenticate bounded paths/frames and cross-check
portable receipts. Two consecutive saves reopen correctly; an interruption after
Intent + Accepted Mutation resumes and completes using the reopened capability.
Original retry and changed-owner refusal pass. Turnover retires only entirely
proven single-position chains; a Batch retains its full dependency chain. This
is not new configuration/add/delete/connect command support. Latest acceptance
links remain distinct from retained undo order. Included PHPSJ002 suffix undo
remains available until its next turnover.
Thirteen focused and compatibility cases pass in 8.46s
(`/private/tmp/ps2-checkpoint-reopen-final.log`); strict store tests pass in 5.38s
(`/private/tmp/ps2-checkpoint-reopen-clippy.log`). The wider existing session
suite also passes 21/21 in 10.51s
(`/private/tmp/ps2-checkpoint-reopen-session-regression.log`). Existing codecs remain unchanged.
**Repeated turnover passes:** StateRoot3/PHPSJ002 now folds the authenticated
prior receipt/local lookup metadata with the current suffix, links the retained
archive descriptor, and activates a second checkpoint. Fresh cold reopen preserves
latest coordinate/through/frame and retires the completed pure-position undo
chain. Old retained controls move to archive placement, retaining all bytes and
charges. Six focused checks pass in 5.53s
(`/private/tmp/ps2-checkpoint-rotation-final.log`), with ten snapshot/journal
compatibility checks in 0.43s (`/private/tmp/ps2-checkpoint-rotation-compat.log`).
Strict store checks pass in 29.48s
(`/private/tmp/ps2-checkpoint-rotation-clippy.log`).
This implementation still rebuilds historical index metadata at turnover, so it
is a correctness bridge, not constant-cost maintenance or performance acceptance.
**Missing-receipt recovery passes (2026-10-15):** fresh exact clean-plan
verification retains the Original reference privately, including through exact
reader/compact caches with accounting. Recovery validates selected local custody
and prepares the missing original-bound receipt, then verifies recovery against
the repaired journal. This only prepares bytes: native package rebarriers and
journal append/rebarriers remain mandatory before Saved. The existing connected
test covers published-package/no-receipt, exact-byte completion/retry, generic or
wrong Original evidence, missing Mutation, tighter bounds and conflicting receipt
identity. A new receipt refuses to advance past newer accepted work; exact
existing-receipt retry preserves the pending suffix. Final connected check passes
in 6.29s (`/private/tmp/ps2-receipt-reconcile-pending-retry.log`), all 21 session
regressions pass in 10.43s (`/private/tmp/ps2-receipt-reconcile-final.log`), ten
compatibility checks pass (`/private/tmp/ps2-receipt-reconcile-compat.log`), and
strict store checks pass in 4.24s (`/private/tmp/ps2-receipt-reconcile-clippy.log`).
No wire changes.
Remaining: native selected local custody/turnover executor, PHPSJ002 SessionIo
dispatch, and disposable performance qualification. Existing stored
bytes/charges are retained; no deletion, native effects or app replacement.
Do not claim indefinitely bounded recovery or seamless app performance yet.

**Approved snapshot implementation — package reader integration:** shared Rust
now parses/adopts AcceptanceBase, audits a complete nonempty post-base suffix,
looks up exact historical receipts through reciprocal authenticated index paths,
checks selected-record allocation/ownership claims, and structurally decodes exact
reader1.4/feature/RootSet-v3 combinations. Legacy1.3 routes retain their behavior.
Full1.4 settled/recovery verification now connects archive placement, exact
role/global inventory union and observed allocation ownership. Four complete
package cases pass against the existing integrated corpus with unchanged extents
and charges: both recovery profiles, requested/unrequested archive corruption,
archive allocation fencing, current-state corruption and changed identity.
Retained v1 allocation claims preserve full framing/prefix checks, including in
archive placement. A settled proof is not a native Saved receipt or admission.
No native writes or app replacement. First-slice validation: snapshot selector6,
ownership1 (three corpus trees, v1/v2), frozen reader/recovery5 and candidate
vectors3 pass; strict store check passes. Logs
`/private/tmp/ps2-snapshot-{reader-tests,ownership-tests,legacy-reader-tests,candidate-vectors,reader-clippy}.log`.
Package integration log: `/private/tmp/ps2-snapshot-package-tests.log` (4 pass);
compatibility `/private/tmp/ps2-snapshot-archive-legacy-reader.log` (5 pass) and
strict store tests `/private/tmp/ps2-snapshot-archive-clippy.log` pass.
Verified commits through `92a19b0` are local; SSH publication retries currently
fail without an authentication/remote change. Continue local implementation.
Next: approved turnover, PHPSJ002/local recovery, v4 admission and disposable gates.
Do not ask for the approved format again or claim performance acceptance yet.

The next implementation slice adds exact PHPSJ002 framing, local base/selector/
archive records, and first-turnover history prepared from a verified drained
session. It binds the old full audit to the selected clean snapshot and rebuilds
the live Core coordinate using the registered incarnation. New journal semantic
dispatch still refuses until publication/recovery is connected. No native format
enablement or v4 continuation yet. Final focused gate passes
9/9 (`/private/tmp/ps2-turnover-focused-final.log`), including exact selected-base
binding; strict store tests pass (`/private/tmp/ps2-turnover-clippy-final.log`).
Maintenance Original/layout and four deterministic package stages are now
implemented. Actual in-memory append preserves every old prefix; published/clean
packages pass the full reader and clean binds the unchanged Core coordinate.
Exact original retry reproduces payload/selectors; changed commitments/candidates
and undersized existing budgets refuse. Archive-only allocations retain full
charge without being overclaimed by active roles. Ten focused tests pass
(`/private/tmp/ps2-turnover-stages-focused.log`), the frozen original golden passes
(`/private/tmp/ps2-turnover-stages-v1.log`), and strict store tests pass
(`/private/tmp/ps2-turnover-stages-clippy.log`).
Next concrete work: durable selector custody, PHPSJ002 semantic recovery and v4
continuation, including interruption/retry. These are compiled in-memory stages,
not native syscall qualification or measured app performance.

**Latest 2026-10-11 checkpoint:** exact-prefix journal decode reuse now removes
repeated canonical parsing and record re-encoding during preflight. It still
freshly verifies actual frame bytes, checksums, ordering, IDs, work limits and
current package/session proofs. Same Saved32 capture: 16 preflight pairs improve
989→544 ms (about 28 ms/pair), decoded bytes 68,770,264→23,051,904. Nineteen session
and two PHPSJ001 tests plus controlled/default strict checks pass. This modest
improvement does not warrant another native fixture/app rebuild; the existing
review app below remains the last native-validated executable.

**Current priority — latest state and bounded useful history (user clarified):**
Save exact current node positions, graph structure/settings and required run/output
evidence; do not retain every intermediate position as permanent user history.
Separate useful semantic undo, run/evidence retention and operation dedupe from
obsolete autosave ancestry. The user reaffirmed efficiency, low resource use and
snappy interaction as priorities. This refines the already approved sealed-root
direction; do not ask for that product direction again.
See [latest-state checkpoint proposal](architecture/PROJECT_SESSION_DURABILITY.md#latest-state-useful-history-and-sealed-checkpoints--2026-10-11-proposal).
The [acceptance-base turnover packet](architecture/PS2_RETENTION_STORAGE_DECISION.md#acceptance-base-turnover-packet--proposed-2026-10-11-not-frozen)
is now drafted. v1_3 already has sealed roots; the missing change is its acceptance
base, suffix replay and indexed historical receipt traversal. The user approved
**snapshot recovery semantics** on 2026-10-11: fully audit at turnover, then
validate complete current/previous/pinned closure and pending suffix; validate
archived history through authenticated lookup when requested. Unrequested,
unpinned archive corruption no longer blocks sound current state. Current/recovery
and requested dedupe/history corruption still refuses. No checksum is treated as
issuer authentication; current authority, custody and rollback checks remain.
This integrity/traversal decision is approved; do not request it again. The
[consolidated candidate byte packet](architecture/PS2_RETENTION_STORAGE_DECISION.md#consolidated-candidate-byte-contract--review-draft-2026-10-11)
was approved on 2026-10-11 for Rust implementation/disposable validation. It freezes
reader1.4/feature dispatch, acceptance base and suffix index,
archive placement, selected-record allocation claims, PHPSJ002/local selector,
acyclic turnover and v4 continuation corridors. Old formats retain exact meanings.
The four documented canonical vectors were independently reproduced against the
existing integrated corpus and pinned by Rust tests. Three isolated candidate tests
pass (exact adoption/empty suffix, requested-vs-unrequested frame corruption,
identity/extent/overlap refusals and explicit dependency-cycle negatives); strict
store lib/tests lint passes. Logs: `/private/tmp/ps2-checkpoint-candidate-tests.log`
and `/private/tmp/ps2-checkpoint-candidate-clippy.log`. The user has approved the
permanent-format packet; do not request that approval again. Shared Rust
reader implementation has begun; no native format is enabled yet. Complete
nonempty suffix/placement/accounting, actual phase/crash and native performance
validation remain implementation gates. Do not present the packet as qualification
or deletion authorization. The exact new IDs/versions/framing/dispatch in the
packet are approved; changes beyond them still require the documented boundary.
Shared reader code and tests now implement the first fail-closed slice; no native data changed. Existing data remains
untouched. User direction does not authorize
real-library conversion, deletion, arbitrary undo/output expiry or numeric limits.
The earlier [custody reuse proposal](architecture/PS2_MACOS_STORAGE_QUALIFICATION.md#live-session-immutable-prefix-custody-reuse--pending-approval)
is **deferred, not the critical path or the pending user decision**: it cannot
bound cold replay or obsolete ancestry. No custody change has been implemented.

**Current approved work (2026-10-11):** grouped-full-sync qualification, shared
allocation ownership and compact journal memo correction are published through
`b48e9ac` (normal SSH push verified). The memo correction is implemented: replacing duplicated Original values with exact
commitments reduces Saved32 metadata from 8,452,393 to 3,170,194 bytes under the
unchanged 8 MiB subbudget. Fresh Original equality and all prefix/context checks
remain required. Repeated verification improves 11.74 seconds → 54–58 ms with no
history replay; 17 session tests and controlled/default strict checks pass.
However final native B saves still take 5.07/5.21 seconds, peak RSS 1,176,780,800
bytes, and cold open 20.28 seconds. Exact out/back and normal close (0.376 seconds)
pass. This is not performance acceptance. Continue read-only capture optimization
of new-operation work; do not rebuild/hand over a stale or performance-ready app.
B is now Saved34 at x13/y14 and has consumed its prepaid tip corridors. Preserve
it; no further B authoring benchmarks. Existing registered Review A remains
available for a later bounded native/UI trial once a concrete improvement passes.
No new fixture limits, format, weaker validation or production writes are approved.
The subsequent packed-body optimization removes duplicate canonical encoding:
24 full candidate proofs on identical Saved32 bytes improve 10.938→7.435 seconds
(32%). Eighteen session/corpus tests, five reader/recovery tests and both strict
configurations pass. The provided PhysicalRecords method is additive, with the
existing generic-provider fallback unchanged. Exact physical/canonical/logical
checks remain. Native A out/back and exact reopen pass: saves 1.10/1.16 seconds,
normal close 65 ms, backend peak RSS 127,860,736 bytes. A has much shorter history;
this is not a same-input comparison to B or acceptance of history scaling.
The ad-hoc app build, ABI/refusal and compile-gated autosave tests pass. Native UI
drag/return and Cmd-Q drain pass (PID33010 exit0); exact reopen verifies A Saved8
at x13/y14. A bounded **node-move interaction review** build is ready:
`/private/tmp/photara-ll2a-packed-review-20261011/Open Photara Autosave.app`.
Use existing Project A; B is full and retained. Durable add/delete/connect and
full LL2a performance/human acceptance remain unfinished. Do not use old launchers.
Next engineering target: repeated hypothetical-journal preparation and cold replay,
using immutable captures before more native storage-consuming tests.

**Active gate — LL2a autosave performance remains unaccepted.** The v3 delta-index
format and narrow inventory-root version-2 correction were approved on 2026-10-10.
Implementation, exact v1/v2/v3 vectors, 15 session regressions and 5 reader tests, retry/context guards,
and controlled/default strict checks pass. New disposable attempts use v3; stored
v1/v2 originals keep exact original recovery dispatch. See the
[approved packet](architecture/PS2_REPEATABLE_WRITER_AMENDMENT.md#follow-on-review--inventory-and-locator-batch-overlays-approved-2026-10-10).
Do not request the same approval again.

**Verified pre-grouping baseline:** completed-history prefix reuse cuts total submit/save
from 8.47/9.43 seconds to 4.47/4.60 seconds on the same grown disposable Project.
Exact Saved and return-to-original pass; normal close takes 0.368 seconds. Exact reopen also passes (14.57 seconds). Peak
RSS is 915,210,240 bytes, and live RSS after work is about 721 MiB. This is not the requested low-resource seamless autosave experience.
Latest native evidence supersedes earlier measurements above: B Saved34, exact
return-to-original and normal close pass, but native latency/resource acceptance
remains open. The release CLI SHA and phase logs are in the existing
[acceptance evidence](architecture/verification/ll2a-disposable-local-acceptance.json).
Use immutable capture profiling before spending more disposable storage.

**Why v3 was necessary:** the earlier v1 move refused before admission because its
growing locator exceeded the unchanged 262,144-byte fixture corridor. V2's path-copy
inventory changes amplified into 547,413 locator bytes on the second captured
operation. V3 balanced inventory/locator delta indexes fit the same corridor;
old format meanings, history, charges, geometry and bounds remain unchanged.
The new logical `photara.package.inventory-overlay` root alone uses version 2;
the existing selected-control version 1 remains unchanged. Other new overlay
schemas use version 1. These size/correctness passes do not imply performance
acceptance. Do not raise caps, erase history, or relaunch stale app builds.

**2026-10-10 save-backlog correction active.** User reports prolonged Saving and
Quit waiting, and authorizes combining redundant moves. Combine only consecutive
same-node **unsent** drafts; never discard submitted/Accepted originals or their
undo/retry evidence. A 1,000-drop test covers coalescing, exact retry and close.
Quit now exposes its save wait and can cancel termination before close dispatch.
The prior build drained and quit normally (exit 0), preserving queued edits. A
native sample identified repeated immutable historical plan compilation inside
admission verification (100% CPU, about 1.57 GiB RSS). Compact per-Host plan reuse
and completed-journal metadata are under native performance validation; current
authority/closure checks and permanent formats remain unchanged.
The first native candidate was rejected: two coalesced operations plus normal
Quit took 226 seconds. An inspection-local audited locator index passes existing
reader/refusal/recovery tests but the second native candidate still exceeds 35
seconds. Next measured bottleneck: original checkpoint reconstruction repeatedly
parses historical objects to recover a small control set. Do not declare autosave
performance accepted from the two-operation fixture tests alone.

**Disposable capacity boundary:** the indexed native run (PID 62015) reached
Save Failed after its outward test move saved; its return-to-start draft remains
visible and unacknowledged. All seven packs reached their immutable prepaid
4 MiB corridors. Do not retry in a loop, force-quit, rebind, raise the existing
ceiling, or claim this draft Saved. Exact pending-position note is
`/private/tmp/photara-ps2-remount-34iyzxfo/ll2a-indexed-pending-position.json`.
User approved fresh 8 MiB prepaid-tip disposable fixtures on 2026-10-10,
retaining old files and the pending-position record. The exact old process was
closed with SIGTERM under that approval; this was not a successful save. Existing
registrations remain intact. Fresh fixtures are ready: `/private/tmp/photara-ps2-remount-mjwcvi6f` (A) and
`/private/tmp/photara-ps2-remount-ei2h61e0` (B). Identities/config paths are in
`/private/tmp/photara-ll2a-fresh-8deecae5-2425-4fc0-85c0-9ae0d2d14176.json`.
Native two-operation save times are 3.63/4.61 seconds; verified close is 0.56
seconds, peak CLI RSS about 200 MiB. UI out-and-return drag plus normal Quit
passed (PID 71768 exit 0); app is stopped, images remain mounted. Latency is
still not accepted. Subsequent same-fixture backend runs retain more history: single
compilation, Apple Silicon SHA acceleration and per-pass prefix hashing preserve
exact bytes/retry checks but latest saves remain 5.36/6.35 seconds with about
777 MiB peak RSS. All benchmark writers closed normally at original x=13/y=14.
Compact journal/semantic/candidate verification reuse now passes focused tests, but
integrated native saves regressed to 19.83/15.53 seconds (~861 MiB peak RSS).
The remaining profile points to repeated original-frame decoding in NEW plan
compilation. Compare against one fixed read-only snapshot before/after; no more
growing-history benchmark loop. Preserve full frame/audit validation and exact plan
bytes. Fresh B has 14 prepaid corridors left. All writers closed normally at
x=13/y=14; app is stopped. Do not hand over a new build as performance-ready.

Latest source correction adds bounded exact-input reader/reconstruction memoization
with fresh allocation observations/prefix digests; no authority or Saved result is
reused. Warm two-operation replay is 76 ms in the focused regression; five session
tests pass. These sources are newer than both rejected native builds. Native
performance/human acceptance remains open. Proposed fresh test setup is 8 MiB
per prepaid tip within existing reader limits; preserve both old images and the
pending-position note. See the existing LL2a evidence JSON for exact provenance.

**Human review finding — 2026-10-09:** LL2a interaction acceptance remains open.
The user observed a dropped node snap back to its acknowledged position, then
jump to the draft after admission. Fix immediate draft rendering and background
interaction; investigate save latency without weakening Accepted/Saved or barriers.
The recorded durability tests remain valid; they did not establish acceptable
interactive autosave behavior. The correction now uses an immediate volatile
position overlay and exact FIFO originals, with continued dragging during
background checkpoints. Focused delay/failure/retry/close/selection checks pass.
Native rapid-drag verification passes; human re-review remains required.

The user additionally requested compile-gated incremental autosave coverage.
Every `build-app.sh` invocation (including `build-local-libraries.sh`) now runs
`verify-autosave-interactions.sh`: real controller/model regressions and the
shared Rust session tests. Strict full acceptance is:
`zsh platform/macos/photara-app/verify-autosave-interactions.sh --full-autosave`.
It intentionally fails for missing durable add-node, delete-node and native
connection support. Controller-local insertion/connection tests and synthetic
session IO are labeled; neither claims native persistence. New authoring command
and definition/asset mappings require their own implementation/contract review.


The delivery path is:

**PS2 production durability → PS3 one-project session/autosave → PS4 project
browse/reopen/switch → LL2a signed cross-Library acceptance.**

**PS2–PS4 approved disposable scopes pass; LL2a interaction corrections are active after human review.**
General real-library storage admission remains disabled. LL1 remains parallel.
Do not rerun historical PS2/PS3 suites without a concrete regression or expand the
approved fixture scope.

**PS4 native switching verified — 2026-10-08.** Shared Rust implements the approved
activation snapshot, immutable references, original IDs, SavedProof/capsule
validation, generation checks, atomic publication and restart reconciliation.
The default-off native app uses registered disposable Projects in one Library.
PS0's current source Saved barrier runs before the past-tense confirmation;
post-confirmation freeze/flush/recheck remains separate.

All 13 native matrix cases and five additional refusal/retry guards pass, including
process interruption on both sides of activation commit. The actual signed app
passed pending drag → verified Saved confirmation → Cancel, failed target open,
switch A/B, read-only Graph recovery, canceled Quit and explicit Retry. Final B
state is Saved revision 5, x=40,013, y=20,014. A is Saved4, x=13, y=14.
The final affected Rust suite passed 36 tests (one opt-in ignored), strict Clippy,
default-off compile, Swift/AppKit lifecycle and actual generated FFI checks.
[Exact evidence and validation provenance](architecture/verification/ps4-disposable-switching.json).
This satisfies the user-required native PS4 gates within the approved disposable
scope; it does not enable real libraries or cross-Library switching.

Both private images are cleanly detached and retained:
`/private/tmp/photara-ps2-remount-sysgwzuu` and
`/private/tmp/photara-ps2-remount-fi7qydft`. Their original fixture budgets were not
increased. Keep all retained files; do not rebind identity pins. The controlled
app is stopped and was never installed/deployed.

**LL2a implementation approved — 2026-10-09.** The existing
[2026-10-09 review packet](architecture/verification/LL2A_FIELD_AUTHORITY_REVIEW.md#2026-10-09-recommended-decision--fresh-local-sql-slots)
approves two genuine disposable local Libraries/one Project each in the normal
signed shell, exact v2 Library/Project selection records, one fresh SQLite slot
publication transaction, and new-Library controller assignment from the verified
current bootstrap-default local controller. It includes one bounded approval text.
Cloud lifecycle/selection, existing database/slot migration and real-library writes
remain excluded. The user explicitly approved the packet at `2794db8`; do not
ask again for its security/format decisions. SQLite disposable qualification and final-commit failure/restart cases now pass
within the exact registered conditional profile; see the LL2a evidence below.

**LL2a native local implementation — 2026-10-09.** The normal
`ApplicationShell` and avatar menu now use the shared Rust session for the two
registered disposable local Libraries. Fresh SQLite is the sole activation
publisher; verified live controller/grant checks protect create/rename and
selection. The original legacy/personal application route remains unchanged.
The [acceptance evidence](architecture/verification/ll2a-disposable-local-acceptance.json)
records affected Rust/SQL/Swift checks, nine native cases and four final SQL
activation-commit failure/unknown/process-cut cases. Native UI verification
covers Create, Rename, cross-Library selection, pending autosave Quit/relaunch,
and Close Project/reopen. Human review remains the next gate.

The development app is `/private/tmp/photara-ll2a-responsive-final/Photara Local Libraries.app`;
use only the pinned launcher
`/private/tmp/photara-ll2a-responsive-final/Open Photara Autosave.app`.
The earlier bundle/launcher remain retained but have the rejected snap-back UI.
The two private images at `/private/tmp/photara-ps2-remount-34iyzxfo` and
`/private/tmp/photara-ps2-remount-b5oucref` remain mounted intentionally for that
review. Quit does not detach them. Do not rebind pins, discard files, install the
app, or admit personal Libraries. Operations can take tens of seconds or longer
in this disposable profile; preserve truthful pending/Saved state and barriers.
The native Rename presentation bug found during validation was fixed and retested.

Cloud Library creation/rename/selection, existing database migration and real
Library enablement remain excluded. Broader authoring, cloud account policy,
LL2b removal and the platform-ready vertical slice remain later work.

Remote synchronization succeeded on 2026-10-09: a normal push to the existing
SSH `origin/main` advanced GitHub from `6290b74` through `8b7b381`, including the
previously unverified PS4/LL2a commits. Authentication and remotes were unchanged.
Normal verified pushes remain authorized; do not reset history.

### Execution priority — 2026-09-29

Reach PS2 → PS3 → PS4 → LL2a with minimum token use. Reuse committed evidence;
do not create another fixture family or repeat broad verification unless a
specific exit requirement lacks evidence. Finish the LL2a field/authority review and identify only concrete
remaining blockers. Each further patch must close a named roadmap requirement.
Run focused checks once, with affected regression checks after shared changes.
Keep updates to completed gates, material findings or required decisions.
Do not expand LL1 removal, UI polish, second-device acceptance or later platform
work before LL2a. These efficiency constraints do not waive permanent-format,
security, production or storage-qualification boundaries.

The bounded byte-candidate work is now complete: the final shared suite passes
**100 tests**, strict Clippy and targeted formatting, with all 50 source/log
hashes independently verified. See the [single current review packet](architecture/PS2_WIRE_REVIEW_READINESS.md#current-consolidated-review-packet--2026-09-29)
for the exact common records, same-tip/single-birth/pending variants, whole-Blob
and retained-conversion evidence, limits and concrete format decision.

**Permanent-format boundary approved 2026-10-03.** The user froze the exact
reviewed codec/schema/coordinate variants and compatibility/refusal behavior at
`9b7facf` for shared Rust implementation. See the [recorded approval scope](architecture/PS2_WIRE_REVIEW_READINESS.md#permanent-format-approval--2026-10-03).
Implement shared Rust reader/writer/recovery/admission, then separately qualify
the disposable storage environment, followed by PS3 → PS4 → LL2a. Do not ask
again for this format approval or expand proof work without a concrete contract
gap exposed by implementation.

The approval does not authorize real-library writes, migration, deletion,
deployment, automatic conversion, expiry, new product limits or weaker durability.
Fixture bounds remain distinct from production limits. Clean-remount permission
does not authorize abrupt interruption. The conditional disposable qualification
below permits PS3 lab work; general real-library admission remains gated.

### Concrete implementation boundary — 2026-10-03

Shared `package::v1_3` now composes complete settled active/recovery/retained
closures for the supported reviewed variants, including metadata-only Blob and
retained-source accounting. The additive repeatable Graph path implements actual
Core replay, pre-effect capacity/identity checks, seven selected O/P/F stages,
exact append retry, barrier reissue, original receipt dedupe and bounded restart
from persisted prior originals. The prior in-memory-plan dependency is removed.
Two consecutive operations (Graph update then no-op) and interrupted restart of
both pass; exact byte fingerprints are pinned in the [amendment record](architecture/PS2_REPEATABLE_WRITER_AMENDMENT.md#exact-additive-vectors).

The disposable fixture explicitly registers a 262,144-byte standing-control
allowance before its first admission; the old 131,072-byte specimen correctly
refuses the second operation's 151,552-byte coexistence requirement. This is not a
production default or an automatic increase. Old corpus files/codecs remain unchanged.

This is a bounded shared implementation, not general production enablement.
The repeatable mutation planner currently supports framed JSON Graph packages;
whole-Blob mutation, newborn/retirement execution and pending-origin operation
authority are not supplied by this path. Imported/unsupported variants must refuse
writable admission. There is no production registrar/qualified native adapter or
production Accepted/Saved token. The private native test adapter remains cfg(test).
Verification: 145 tests passed across library, frozen codec/reader and legacy
package targets, plus strict library/tests Clippy, targeted Rustfmt and diff checks
([manifest](architecture/verification/ps2-shared-repeatable-regression.json)); 11 preexisting tests and the explicitly invoked native trial are
opt-in. The private [shared clean-remount observation](architecture/PS2_MACOS_CLEAN_REMOUNT.md#shared-repeatable-rust-path--2026-10-03)
passed partial append → remount → original retry → remount → persisted-plan
retry/verification. Its image is detached and retained with logs. All 50 prior
frozen source/output hashes remain unchanged. Do not repeat these unchanged runs.

The earlier clean-remount observation alone did not establish native storage
qualification. The approved implementation and scoped qualification below now
supersede that blocker for the designated disposable configuration only. Do not
infer general real-path provider exclusion or production writer authority, enable
real-library Saved, or bypass PS3/PS4/LL2a production-wiring review.
Approved power-interruption permission remains **absent**.

### Approved checkpoint implementation and scoped qualification — 2026-10-03

Shared PHPSJ001 now validates exact framing/checksums, strict header/envelopes,
CheckpointIntent/CheckpointReceipt, unchanged nested accepted frames, sequential
completed checkpoints and original retries. Torn tails freeze without truncation;
corrupt/unknown records refuse. Completed second-operation recovery now finds
digest-bound prior controls retained in complete packed frames, under existing
scan/work bounds. Four-record exact byte fingerprint:
`788ad5be74841fdb8e58457d952ad60e72266400eb3813286b6cf4f10bd180b1`.

The native adapter persists intent before package effects, independently checks
the owned image/configuration and pinned identities, preflights journal/control
coexistence, measures allocated storage, and reissues file/directory barriers on
uncertain exact-byte retries. The new partial append → remount → retry → remount
→ completed retry/verification passed; its image is detached and retained.
The [qualification record](architecture/PS2_MACOS_STORAGE_QUALIFICATION.md#conditional-disposable-qualification--2026-10-03)
and [machine evidence](architecture/verification/ps2-checkpoint-journal-qualification.json)
define the exact scope. The existing standing allowance remains 262,144 bytes;
the final measured standing footprint is 126,976 bytes. These are fixture values,
not production limits or a promise of physical free-space reservation.

All 28 library tests passed (one native opt-in invoked separately); strict Clippy,
scoped Rustfmt and diff checks passed. The 50 frozen source/output hashes remain
unchanged. Do not repeat this unchanged evidence.
The PS3 record-kind amendment in the existing session document was approved on
2026-10-05: implement Mutation, single-operation undo/redo, finite SessionBarrier
and evidence-only RecoveryDecision within its exact boundaries. Preserve old
portable single/null grammar and golden bytes. No grouped gestures, real-library
writes, migration, conversion, rotation/compaction/expiry or deployment follows.
Implement disposable PS3 autosave before PS4 and LL2a; do not ask again for this
record-kind approval.

The native evidence accounting preflight now combines selected-control peak with
all supplied retained/prospective journal/receipt allocations and explicit
namespace charge under the original standing byte/count bounds. It preserves
charged high-water and refuses duplicate identities or insufficient bounds; it
does not qualify a native charge rule or mint admission. The remaining review
must distinguish the frozen logical accepted-journal frame from the separately approved
device-local journal container and checkpoint acknowledgement. See the
[native profile direction](architecture/PS2_MACOS_STORAGE_QUALIFICATION.md#first-native-profile-and-admission-direction--approved-2026-10-03)
and [journal contract](architecture/PROJECT_SESSION_DURABILITY.md#durable-journal-and-reconciliation).
Neither the old proposed journal size/undo limits nor native scratch filenames
are approved production format or retention policy.
The [exact checkpoint-journal subset proposal](architecture/PROJECT_SESSION_DURABILITY.md#first-journal-storage-implementation--approved-2026-10-03)
specifies the exact framing and two record bodies; the user approved both it and
the conditional macOS profile direction on 2026-10-03 for implementation and
disposable validation. Preserve nested AcceptedFrameV1 and original bindings.
The four bounded PS3 record kinds were subsequently approved on 2026-10-05.
Production limits, rotation/compaction/expiry, automatic conversion and
real-library enablement remain unapproved. Managed or
unknown provider storage stays read-only. No weaker barriers or acknowledgements,
APFS-only qualification, abrupt-power or hardware-fault experiments are authorized.
Do not ask again for these approved format/profile decisions; finish the actual
qualification evidence, then PS3 → PS4 → LL2a.
Accounting validation: five affected repeatable tests passed (one existing native
opt-in ignored), including unchanged exact vectors; strict Clippy, scoped Rustfmt
and diff checks passed. No additional remount or broad fixture run was needed.

Implementation exposed a **repeatability gap in the frozen specimen recipes**:
the next active root/association IDs and allocation roles are hardcoded. A second
operation would reuse a root ID still held by recovery. Preserve old codec replay;
do not change their deterministic recipe silently. The [bounded additive amendment](architecture/PS2_REPEATABLE_WRITER_AMENDMENT.md)
was explicitly approved on 2026-10-03 for bounded additive implementation and
exact-byte validation. Implement the repeatable admission/layout variants with
persisted planner choices, preserve all frozen codecs and golden bytes, and prove
two consecutive operations plus interruption/retry of the same original attempt.
Do not ask again for that amendment or add a fixture family. Finish the shared
writer/admission path, then disposable storage qualification, then PS3 → PS4 → LL2a.

## Approved semantics — do not reopen without contradictory evidence

- Normal authoring uses continuous autosave; users do not need to save
  explicitly. `Command-S` may remain **Save Now**.
- Speculative or in-memory state is not committed. `Accepted` requires the
  exact journal group to pass its qualified durability barrier. `Saved`
  requires the matching selected package checkpoint/`HEAD.json` and original
  receipt to be durably verified for the current accepted authored revision.
- `HEAD.json` is the sole authoritative atomic selector. Active and recovery
  roots are independently verifiable. Structural opening is distinct from a
  full integrity audit; an imported, unaudited package may be inspected
  read-only but is not admitted for writing.
- The `.photara` package owns authored Graph state. Cloud is authoritative for
  account, Library membership and catalog coordination; local SQL is a
  projection/session/cache. Device-only observations and paths are not portable
  package state.
- Resource identity is separate from physical backing and placement. Ordinary
  open, autosave, validation and root turnover must not read or strongly hash
  large external media. Strong media hashing occurs only at explicit capture or
  verification boundaries.
- GUI, CLI/headless, scripts and future agents are peer Photara-controlled
  writers using the shared Rust admission, lease, journal, dedupe, publication
  and recovery protocol. The GUI is not a privileged writer.
- User-selected project locations are permitted only through qualified storage
  profiles; a Photara-managed project root is not mandatory. Storage roles use
  the existing logical location/host-binding architecture.
- Keep native host UI as the default: SwiftUI/AppKit on macOS and native Windows
  UI later. Custom treatment is an explicit exception.
- Photara remains the current identity. BR0 made branding, extension and roots
  configuration-driven, but a real rename still requires a coordinated trust,
  directory and compatibility cutover.
- Keep the existing Fly.io resources available for the controlled remote
  vertical slice. Do not deprovision them in this sequence.

## Proven or implemented

### PS2 disposable evidence

- Bounded packs and HEAD-selected locator roots avoid lifetime tail copying.
- Active/recovery selection, twelve pin classes, bounded interruption recovery,
  conservative capacity admission and exact old/candidate reconciliation have
  disposable real-file evidence.
- The packed physical comparison provisionally favors B-tree plus ordinal
  sequence; radix remains the required comparator. Neither is frozen wire.
- Genuine Graph commands, original receipts and exact ownership suffix coverage
  now share a same-tip packed publication and original token. Independent
  active/recovery closure checks and interrupted journal/source/packed/cleanup/
  final-settlement replay run for both maps.
- An [attributable ledger](architecture/PS2_ATTRIBUTABLE_RETIREMENT_LEDGER.md)
  charges bounded standing controls once, records
  source/pack growth without shrinking credit, and transfers a sealed allocation
  charge through typed claim removal, original-token unlink authorization,
  directory barrier and fresh absence before project-only credit. Current
  retirement evidence now also covers live semantic, ownership and locator
  relocation. All pin obligations and independent root closures remain checked.
- [Fresh-generation enrollment](architecture/PS2_FRESH_GENERATION_ENROLLMENT.md)
  selects the original hold before creation and promotes at most eight witnessed
  empty packs under that token. Ten focused tests and the 100-index/seven-Graph
  regression pass. Empty/partial unbound stages fence. The subsequent
  [witnessed Graph rollover](architecture/PS2_GRAPH_WITNESSED_ROLLOVER.md)
  integrates birth, payload, born-and-sealed generations and exact charge
  finalization under the original token. Eight main and seven independent
  adversarial tests pass. The combined regression passes all 126 index/placement
  tests and seven original Graph tests, plus strict example Clippy and targeted
  formatting.
- [Live-owned relocation](architecture/PS2_LIVE_OWNED_RELOCATION.md) moves both
  roots before exact selected unlink authorization and once-only project credit.
  Runtime Graph-born/sealed sources retain original receipts across relocation.
  The subsequent [compact retirement proof](architecture/PS2_COMPACT_RETIREMENT_COMMITMENTS.md)
  removes full-plan serialization from the unchanged 48 KiB cap and independently
  verifies complete destination bytes after authorized source unlink. Both maps
  now demonstrate repeated net-positive runtime reclamation with 32-record groups
  under a fixed budget. Eight-record runs remain net negative; retained state
  eventually causes safe capacity refusal. This is bounded evidence, not a
  lifetime plateau, general maintenance policy or permanent format.
- A [scalable wire review candidate](architecture/PS2_SCALABLE_WIRE_REVIEW_CANDIDATE.md)
  describes typed closure, placement and accounting records and required linked
  vectors. Its [first linked byte specimen](architecture/PS2_SCALABLE_PACKED_BYTE_CANDIDATE.md)
  passes eight tests across fourteen bootstrap/closure scenarios, with exact
  independent canonical bytes and recovery closure. The subsequent
  [real-operation specimen](architecture/PS2_SCALABLE_OPERATION_BYTE_CANDIDATE.md)
  passes seven tests for actual Core/PS1 mutation, authored no-op, original retry,
  typed receipt indexes and independent recovery. The
  [multi-level tree specimen](architecture/PS2_SCALABLE_BRANCH_BYTE_CANDIDATE.md)
  adds nine tests for typed inventory/operation/charge branches and exact physical
  bytes. A [resource/conversion specimen](architecture/PS2_RESOURCE_CONVERSION_BYTE_CANDIDATE.md)
  passes twelve tests across 26 linked scenarios, preserving all eighteen original
  snapshot files and checking same-attempt copy registration. The
  [canonical phase subproof](architecture/PS2_CANONICAL_PHASE_SUBPROOF.md) adds
  thirteen tests for original reserves, birth witnesses, actual suffix integrity
  and once-only credit. Its external phase records remain component evidence.
  The [joined settled snapshot](architecture/PS2_JOINED_SETTLED_BYTE_CANDIDATE.md)
  now passes twelve tests selecting real operation/resource/tree/conversion and
  physical-accounting bytes through one HEAD. The subsequent
  [single-HEAD transition route](architecture/PS2_SINGLE_HEAD_ROUTE_BYTE_CANDIDATE.md)
  passes nineteen tests for selected original admission/phase/C, actual Graph
  operation four, retirement, independent recovery and original retry. Its
  simultaneous controls exactly fill the original 131,072-byte pool; the small
  retirement has a 520,192-byte net charge increase. Inline original witnesses,
  full tiny charge proofs and v1 resource-origin rewrites remain bounded
  specimens. Scalable replacements and exact proposed permanent coordinates
  remain engineering work, tracked in the
  [wire readiness checklist](architecture/PS2_WIRE_REVIEW_READINESS.md).
- New component evidence: [factored resource selections](architecture/PS2_FACTORED_RESOURCE_BYTE_CANDIDATE.md)
  pass nine tests, and [scalable accounting](architecture/PS2_ACCOUNTING_SCALING_CANDIDATE.md)
  passes twelve. Both canonical generators reproduce exact bytes; independent
  parent runs pass. These are component selectors, not completed whole-HEAD
  composition or permanent-format approval. The integration checklists map
  [retention-origin authority](architecture/PS2_RETENTION_ORIGIN_EVIDENCE_PROPOSAL.md)
  and [newborn/original replay](architecture/PS2_NEWBORN_REPLAY_INTEGRATION_CHECKLIST.md)
  without changing release policy or claiming completed wire dispatch.
- The [whole-Blob compatibility component](architecture/PS2_WHOLE_BLOB_BYTE_CANDIDATE.md)
  passes six tests with exact legacy managed/v2 bytes, explicit raw-allocation
  ownership and separate strong audit. It remains a separate project-scoped
  component; full HEAD composition and bounded shared parsing are open.
  The [selected Blob composition plan](architecture/PS2_SELECTED_BLOB_COMPOSITION_PLAN.md)
  keeps the legacy D19 project identity and bytes, with a standalone selected
  layout first. Media-bearing ConversionSource composition also needs separate
  structural-open and strong-audit paths; the current retained-file verifier
  hashes every source file and cannot prove no-routine-media-hash behavior.
  [Blob preparation](architecture/PS2_SELECTED_BLOB_PREPARATION.md) now passes ten
  adapter/accounting tests with unchanged D19 metadata and instrumented
  structural/audit separation. Its linked corpus still lacks shared full-HEAD
  verification and selects no ConversionSource.
- The [final-coordinate integration plan](architecture/PS2_FINAL_COORDINATE_INTEGRATION_PLAN.md)
  and [newborn/original field proposal](architecture/PS2_NEWBORN_ORIGINAL_FIELD_PROPOSAL.md)
  are concrete unfrozen drafts. The [integrated settled coordinate candidate](architecture/PS2_INTEGRATED_SETTLED_COORDINATE_CANDIDATE.md)
  now selects active, recovery and nonempty retained-history roots with exact
  independent ownership, scalable resource/accounting trees and 2,076,672
  synthetic registered bytes. This starts a new synthetic physical registration
  epoch while preserving original semantic/source bytes. All-origin evidence,
  Blob placement and original newborn/transition integration remain next;
  neither this settled checkpoint nor the drafts are format approval.
  The next bounded code slices are [selected origin evidence](architecture/PS2_SELECTED_ORIGIN_FIELD_PROPOSAL.md)
  and an [admission-only successor](architecture/PS2_SELECTED_TRANSITION_IMPLEMENTATION_PLAN.md)
  preserving that new epoch. Pending-origin support depends on the actual
  selected original/phase reader; payload/newborn/retirement follow admission.
  The [shared evidence-validation seam](architecture/PS2_SELECTED_EVIDENCE_VALIDATION_SEAM.md)
  preserves all 17 settled and nine factored tests and every original vector;
  new selected-origin and admission targets remain in progress.
  A separate [prepared-successor seam](architecture/PS2_PREPARED_SUCCESSOR_VALIDATION_SEAM.md)
  authenticates original/candidate selectors and accepts only caller-supplied
  Core-prepared authored additions; admission and replay proofs remain separate.
  The [selected-origin byte candidate](architecture/PS2_SELECTED_ORIGIN_BYTE_CANDIDATE.md)
  now passes 16 tests with 19 exact coherent refusals. Four origins resolve
  through actual selected evidence, including independent recovery with a
  strict subset of a shared policy. Its recovery rotation is two static valid
  states, not an authorized transition or release; pending remains unsupported.
  The [selected admission candidate](architecture/PS2_SELECTED_ADMISSION_CANDIDATE.md)
  passes 16 tests for an actual control-only successor, exact original O/P hold,
  real Core preparation and independent complete append regeneration. Original
  R is 1,900,544; planned growth is 1,835,008; peak modeled controls are 118,784
  within 131,072. That admission checkpoint leaves operation four unaccepted.
  The subsequent [selected phase route](architecture/PS2_SELECTED_PHASE_BYTE_CANDIDATE.md)
  passes 22 tests and an independent parent run through seven actual selected
  stages, original-prefix Core replay, 18 selector cuts and once-only cleanup.
  The same original R and control bounds hold; published accounting is checked
  through the shared reader, while recovery verifies only its supplied closure.
  Barrier flags remain modeled. Selected newborn/born-and-sealed rollover,
  pending origins, retirement and full Blob composition remain engineering work.
- The local macOS syscall/fault matrix and three explicitly authorized disposable
  [APFS clean-remount cases](architecture/PS2_MACOS_CLEAN_REMOUNT.md) provide
  evidence. Abrupt-power, provider-path and production `Saved` qualification
  remain absent; clean detach may flush outstanding writes.

The current synthesis and exact measurements are in
[PS2 consolidated engineering evidence](architecture/PS2_CONSOLIDATED_ENGINEERING_EVIDENCE.md).

The [shared Blob checkpoint](architecture/PS2_SELECTED_BLOB_SHARED_CANDIDATE.md)
now verifies the original D19 project through the common HEAD/physical/accounting
reader. Eighteen focused tests and 71 affected regression tests pass, with strict
lint. Structural access reads no current media; explicit audit catches corruption.
This closes standalone Blob composition, not retained-Blob conversion or mutation.

### LL1 parallel evidence

- Keep the existing `RESTRICT` constraints. Disposable PostgreSQL and SQLite
  executors implement the same protected deletion semantics with engine-specific
  transaction strategies; current evidence does not justify `NO ACTION`.
- The service-authority model uses a dedicated authority boundary and exact,
  one-use transaction/backend-bound grants. Caller-set owner context alone is
  not authority.
- PostgreSQL dense coverage includes all 48 reviewed owned tables. SQLite
  coverage populates all 79 source tables, retires 73 target tables, retains six
  designated tables and publishes detached evidence in the fixture.
- A [46-case fresh-device receipt fixture](architecture/verification/LL1_FRESH_DEVICE_RECEIPT.md)
  proves exact terminal retrieval after original-device revocation without
  granting the replacement device permission to execute the original request.
  A separate [Rust credential-seam fixture](architecture/verification/LL1_RUST_CREDENTIAL_SEAM.md)
  now exercises signed HTTP tokens, database credential/revision checks and two
  synchronized expiry/revocation lock waits. The subsequent
  [combined signed HTTP-to-SQL fixture](architecture/verification/LL1_HTTP_SQL_ADAPTER.md)
  executes the protected SQL transaction and retrieves exact original receipts,
  with fresh credential checks, three post-lock deadline gates, ordinary-role
  denials and exact rollback snapshots. Its
  [lifecycle extension](architecture/verification/LL1_HTTP_SQL_LIFECYCLE.md)
  proves three private cancellation/connection-loss rollback cases after the
  original transaction and locks end, account-lock serialization of a concurrent
  receipt lookup, and original receipt recovery after a known commit loses its
  in-process response. A subsequent
  [private COMMIT relay](architecture/verification/LL1_COMMIT_ACK_RELAY.md)
  proves both rollback before COMMIT is forwarded and a committed original
  receipt after the server acknowledgement is withheld. A generic client commit
  error remains ambiguous. These remain test routers and disposable
  overlays; production authority custody and lifecycle integration remain open.
- A separate [raw-authority commit guard](architecture/verification/LL1_AUTHORITY_COMMIT_GUARD.md)
  passes 27 disposable cases, rejecting surviving grant/permit work at commit
  without deleting immutable request/receipt evidence. The unchanged historical
  raw fixture still demonstrates the original orphan limitation. The optional
  [signed HTTP composition](architecture/verification/LL1_HTTP_COMMIT_GUARD_COMPOSITION.md)
  now passes all three adapter/lifecycle/COMMIT-relay modes with and without
  the guard, preserving exact catalogs, authority privileges and original receipts.

The current entry is
[LL1 protected executor contract](architecture/verification/LL1_PROTECTED_EXECUTOR_CONTRACT.md),
with focused authorization, coverage and service-handoff links. The
[current readiness audit](architecture/verification/LL1_READINESS_AUDIT_20260927.md)
and [LL2a field/authority table](architecture/verification/LL2A_FIELD_AUTHORITY_REVIEW.md)
separate create/select/rename and activation mapping from later LL2b removal.
These are disposable proofs and review documents, not production deletion.

## Unimplemented or unproven

- Narrow PS2 wire approval and bounded shared reader/repeatable writer exist;
  no enabled production adapter, live conversion, migration, production GC,
  Asset Store or production `Accepted`/`Saved` path.
- No qualified production barrier path. Disposable Graph rollover and live
  relocation are integrated with bounded useful fixed-budget reclamation;
  arbitrary-scale maintenance, qualified local binding and production admission
  remain unproven.
- Shared Rust session/undo/barrier code exists behind private admission. Native
  disposable autosave and approved controlled app/CLI integration pass (PS3);
  general real-library admission remains disabled.
- PS4 registered-disposable browse/reopen/switch passes; general real-Project
  admission remains disabled.
- LL2a fresh disposable local create/select/rename and native cross-Library
  wiring exist. Human review is next; production/cloud Library lifecycle and
  real-library admission remain disabled. Removal is LL2b and remains later.
- LL1 production Rust/OIDC-to-SQL authority handoff, canonical evidence/replay
  codecs, real TCP-disconnect handling, public unknown-outcome policy and
  remaining lifecycle/concurrency states remain absent. The combined handoff,
  selected cancellation/connection-loss paths and two exact COMMIT cuts have
  disposable fixture evidence.
- No production deletion, live-data migration, deployment or public launch.

## Dependency order and human acceptance

1. Complete PS2 integration evidence, storage qualification, wire review/freeze
   and shared Rust reader/writer/recovery/admission.
2. PS3: one-project production session and autosave with truthful
   `Saving…`/`Saved`/failure state, final flush and crash/relaunch recovery.
3. PS4: minimal native project browser, reopen and safe switch; flush and verify
   the current project before target activation and restore it on target failure.
4. Finish the parallel LL1 review/security prerequisites needed by LL2.
5. LL2a: wire Library create/select/rename and run the signed, real
   two-Library/two-project acceptance on `main`.

LL2a is the next major Suhail test. In the signed app, he must be able to make
ordinary authored edits without manually saving, observe truthful saving state,
quit/relaunch and recover the exact state, browse/reopen/safely switch projects,
switch through the intended two-Library/two-project matrix, and verify each
project restores correctly. Preserve unknown/failure context. This consolidates
the existing PS3, PS4 and LL2a roadmap criteria; it does not add Library removal,
two projects per Library, second-Mac acceptance or final Gallery polish.

After LL2a passes, audit the preserved stash against validated `main` and ask
Suhail before dropping it. The broader platform-ready vertical slice remains a
later gate before Layout becomes the first product node.

## Parallel work

- LL1 contract/schema/authorization review may proceed without production DDL,
  deletion or live data.
- A minimal native project-browser presentation fixture may proceed, but must
  not simulate safe switching before PS3/PS4 durability exists.
- Final Gallery aesthetics, branding, website, store, LLC and public launch are
  later work and do not change this critical path.

## Prohibited claims and actions

- Do not call disposable fixture implementations production or extend their
  conditional storage qualification beyond the exact configuration and failure
  model recorded above. Disposable `Saved` requires that qualified evidence;
  it does not enable real-library writes. Preserve each recorded wire approval's
  exact scope.
- Do not patch Swift `Browse Projects`/`closeProject()` into apparent safe
  switching before the Rust session/durability boundary.
- Do not perform live package writes/conversion, numbered migration, production
  Library deletion, GC/retirement, deployment, live-data mutation or Fly.io
  deprovisioning without the later gate.
- Do not change `RESTRICT` to `NO ACTION` unless new full-schema evidence shows
  an independent necessity.
- Do not force-push, discard stashes, or modify unrelated worktrees.

## Stop conditions

Iterate autonomously when a disposable implementation is slow, fails, or needs
another engineering design within the approved semantics. Stop and ask Suhail
before changing an approved durability/`Saved`, compatibility/wire, resource
retention/custody, security/authorization, destructive file/data, or visible
lifecycle policy; before freezing a permanent format; or before production/live
migration, deployment or deletion. Also stop if evidence requires a schema or
constraint change, or if LL2a acceptance would materially differ from the
roadmap definition above.

Use an Astra task for major architecture, production-code or broad repository
changes. Keep routine bounded work moving without asking for approval unless a
stop condition above is reached.

## Minimal reading order

1. This file (current gate above; historical evidence below is reference only).
2. [Root roadmap — delivery path only](../ROADMAP.md#delivery-path-cross-library-acceptance-then-platform-ready-vertical-slice--2026-09-17).
3. [Project session and durability](architecture/PROJECT_SESSION_DURABILITY.md).
4. [LL2a field/authority amendment](architecture/verification/LL2A_FIELD_AUTHORITY_REVIEW.md#2026-10-09-recommended-decision--fresh-local-sql-slots).
5. Follow only the focused LL1/schema links needed by those unresolved mappings.
   PS2 format and qualification documents are reference material unless a concrete
   implementation gap requires them.

For parallel LL1 work only, additionally read
[Library lifecycle](architecture/LIBRARY_LIFECYCLE.md),
[LL1 typed contract/schema delta](architecture/LL1_TYPED_CONTRACT_AND_SCHEMA_DELTA.md)
and the
[current protected-executor evidence entry](architecture/verification/LL1_PROTECTED_EXECUTOR_CONTRACT.md).
Follow their focused links only as needed.
