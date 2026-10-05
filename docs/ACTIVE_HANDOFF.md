# Active handoff

This is the sole current operational handoff. Historical handoffs and detailed
experiment reports are reference material only.

## Baseline

- Authoritative branch: `main`.
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

The delivery path is:

**PS2 production durability → PS3 one-project session/autosave → PS4 project
browse/reopen/switch → LL2a signed cross-Library acceptance.**

The approved PS2 checkpoint implementation and **conditional disposable-image
qualification** now pass. PS3 implementation is active: the user approved the bounded mutation/undo
journal amendment on 2026-10-05 for implementation and disposable validation in
[Project session and durability](architecture/PROJECT_SESSION_DURABILITY.md#ps3-record-kind-amendment--approved-2026-10-05).
General real-library storage admission remains disabled; this scoped result is
not a production release gate. LL1 remains parallel and cannot bypass PS3/PS4 or
authorize LL2 production mutation.

The shared PS3 implementation and native disposable autosave lab now pass:
32 affected Rust tests, strict Clippy, Swift projection/pipe checks, consecutive
edits, original retry, restart undo/redo, process interruption after Accepted,
clean remount, and actual native edit → close while saving → reopen at exact
Saved revision 10. All 50 frozen source/output hashes remain unchanged.
[Evidence and the next review scope](architecture/PROJECT_SESSION_DURABILITY.md#ps3-disposable-implementation-and-next-review--2026-10-05).

The successful private image `/private/tmp/photara-ps2-remount-cuc4usia` is cleanly
detached and retained. An earlier allocation-overrun attempt remains preserved in
`/private/tmp/photara-ps2-remount-n5522myf`. The correction registers measured
preallocated tips before the first admission, preserving existing high-water
semantics; no failed attempt was enlarged. These are fixture bounds.

**Next boundary: separately reviewed production session wiring.** The concrete
scope is in the session document: shared app/CLI lifecycle integration restricted
to controlled disposable projects, with real-library writes still disabled.
Do not call full PS3 complete or begin PS4/LL2a acceptance yet. Do not repeat the
approved journal-format review or unchanged passing suites.

### Execution priority — 2026-09-29

Reach PS2 → PS3 → PS4 → LL2a with minimum token use. Reuse committed evidence;
do not create another fixture family or repeat broad verification unless a
specific exit requirement lacks evidence. Finish the current Blob integration,
then consolidate one exact-wire review package and identify only concrete
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
  disposable autosave acceptance passes; separately reviewed production session
  wiring remains open (PS3).
- No safe real project browse/reopen/switch flow on that durability boundary
  (PS4).
- No production Library create/select/rename wiring or signed cross-Library
  acceptance (LL2a). Library removal is LL2b and remains later.
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

1. This file.
2. [Root roadmap — delivery path only](../ROADMAP.md#delivery-path-cross-library-acceptance-then-platform-ready-vertical-slice--2026-09-17).
3. [Project session and durability](architecture/PROJECT_SESSION_DURABILITY.md).
4. [PS2 production codec/publication contract](architecture/PS2_PRODUCTION_CODEC_AND_PUBLICATION_CONTRACT.md).
5. [PS2 consolidated engineering evidence](architecture/PS2_CONSOLIDATED_ENGINEERING_EVIDENCE.md).

For parallel LL1 work only, additionally read
[Library lifecycle](architecture/LIBRARY_LIFECYCLE.md),
[LL1 typed contract/schema delta](architecture/LL1_TYPED_CONTRACT_AND_SCHEMA_DELTA.md)
and the
[current protected-executor evidence entry](architecture/verification/LL1_PROTECTED_EXECUTOR_CONTRACT.md).
Follow their focused links only as needed.
