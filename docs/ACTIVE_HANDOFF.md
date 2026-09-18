# Active handoff

## Current delivery direction — 2026-09-17

The [root roadmap](../ROADMAP.md#delivery-path-cross-library-acceptance-then-platform-ready-vertical-slice--2026-09-17)
now separates two milestones: first, a signed PS3/PS4/LL2a build that safely
autosaves, reopens and switches real projects and Libraries; second, a
platform-ready vertical slice (Gallery, asset/proxy paths, reference Node SDK
runtime, second-machine/full Fly route) before Layout becomes the first
product node. Keep existing Fly.io resources available; do not deprovision
them merely because local development remains possible. Publish verified
bounded checkpoints to `main` by normal fast-forward and keep the recoverable
local-only stash through the post-LL2a audit, not an automatic drop.

PS2 remains the active engineering gate. The consolidated disposable fixture
below does not approve a permanent wire or production writer. The
[LL1 typed-contract and unnumbered schema review](architecture/LL1_TYPED_CONTRACT_AND_SCHEMA_DELTA.md)
now has concrete R1–R5 recommendations pending review; project-browser
presentation can be prepared in parallel. The [synthetic activation fixture](architecture/verification/LL1_ACTIVATION_FIXTURE.md)
passes 12 injected tests but is not SQL, package durability or native acceptance.
Live LL2 switching must consume the Rust-owned
session/durability boundary. No numbered migration or production data change
is authorized by this sequencing update.

## LL1 actual PostgreSQL baseline — 2026-09-18

The [private-cluster actual-migration check](architecture/verification/LL1_ACTUAL_POSTGRES_BASELINE.md)
passes 30 behavioral cases and matches all 59 tables, 114 FKs, 122 enabled
triggers, 173 policies, 52 functions and 45 forced-RLS tables. All three
populated RESTRICT cycles remain undeletable under current retention guards;
API/control roles also lack DELETE. This establishes the protected-executor,
guard-exception and mandatory receipt/marker/inventory implementation boundary,
not successful full retirement. No production migration, role change or live
data access. The [actual SQLite baseline](architecture/verification/LL1_ACTUAL_SQLITE_BASELINE.md)
loads all 15 unchanged local migrations and passes 24 behavior cases plus four
logical-join checks over 79 tables, 137 FKs and 190 triggers. Its existing
guards also refuse aggregate retirement; PostgreSQL-style data-modifying
DELETE CTEs do not parse on SQLite. This rules out directly porting that
statement shape, not every unchanged-RESTRICT strategy.

## LL1 disposable protected-executor comparison — 2026-09-18

The [shared contract and comparison](architecture/verification/LL1_PROTECTED_EXECUTOR_CONTRACT.md)
links the [PostgreSQL 24-case](architecture/verification/LL1_PROTECTED_POSTGRES_EXECUTOR.md)
and [SQLite 21-case](architecture/verification/LL1_PROTECTED_SQLITE_EXECUTOR.md)
experiments. Both load all unchanged migrations and close their seeded mutual
cycles with existing `RESTRICT` actions; no `NO ACTION` migration is justified
by this evidence. Each verifies seeded rollback, terminal agreement, stale/retry
behavior and unrelated-row preservation. Both require test-only narrow deletion
guard exceptions and modeled terminal relations. Coverage is sparse relative to
the complete schema. Crucially, the PostgreSQL API SQL login can spoof owner
context through caller-set GUCs; the SQL fixture does **not** prove authenticated
authorization. Stop for security/physical-design review before any production
deletion, migration or LL2b wiring.

## PS2 bounded typed-recipe recovery prerequisite — 2026-09-18

The [disposable report](architecture/PS2_TYPED_RECIPE_RECOVERY_PREREQUISITE.md),
[final bounded data](architecture/verification/ps2-typed-recipe-bounded.jsonl)
and [rejected exhaustive path](architecture/verification/ps2-typed-recipe-exhaustive-negative.jsonl)
show exact original-token partial-record replay without routine lifetime scan.
All 53 furnace tests pass. The fixture HEAD cap is unchanged, but ~50 KB of
process writes, 28 syncs and a ~3 MB conservative hold for 4 KiB net allocation
are negative production-readiness evidence. Complete ownership/refund,
qualified barriers, genuine Graph/journal receipts and permanent recipe
placement remain PS2 gates; no Accepted/Saved or live writer claim.

## PS2 actual-v3 typed inventory fixture — 2026-09-18

The [disposable typed-inventory fixture](architecture/PS2_V3_TYPED_INVENTORY_FIXTURE.md)
and [four measurement rows](architecture/verification/ps2-v3-typed-inventory.jsonl)
implement separate exact semantic/ownership closures in the existing
HEAD-selected locator. Forty-six furnace tests, strict Clippy and formatting
pass. Controlled recovery, forged/missing/extra leaves and all-pin retention
are covered. Only two selected allocation claims were measured; complete
charge/refund, locator/control self-ownership, qualified barriers and genuine
Graph/journal integration remain PS2 gates. No production wire or Saved claim.

## LL1 scoped-cycle disposable comparison — 2026-09-18

The [private-cluster comparison](architecture/verification/LL1_SCOPED_CYCLE_COMPARISON.md)
passes 87 PostgreSQL cases. Both original RESTRICT edges refuse either
separate-statement deletion order, while two tested single-statement CTE
forms commit in the minimal fixture. Either one-edge deferred NO ACTION
variant permits its corresponding separate-statement order. No strategy is
selected: full production triggers, RLS, privileges, concurrency and facade
publication remain unproved. No numbered migration, Neon or live deletion.

## LL1 full-schema static audit — 2026-09-18

The [full inventory](architecture/verification/LL1_FULL_SCHEMA_STATIC_INVENTORY.md)
and [gap review](architecture/LL1_FULL_SCHEMA_GAPS.md) cover all 79/59 tables,
137/114 FK edges and 190/122 triggers. They expose a scoped PostgreSQL
receipt/batch RESTRICT cycle absent from the previous four-FK proposal.
Physical deletion order and any additional FK adjustment await explicit
review; no migration, SQL execution or live data change occurred.

## PS2 actual-v3 ownership closure gate — 2026-09-18

The [bounded actual-v3 probe](architecture/PS2_V3_OWNERSHIP_CLOSURE_GATE.md)
and [four raw rows](architecture/verification/ps2-v3-ownership-placement-probe.jsonl)
compare 1k/10k-operation placements and preserve the negative recovery result:
the current audit correctly rejects ownership leaves outside semantic closure
and retains the old selection/hold. Six new tests and all 40 furnace tests
pass. Select a typed semantic-plus-ownership closure within the existing
locator or a separate persistent ownership root under the same HEAD before
permanent wire or refundable v3 integration. Fixed upfront pack-capacity
admission is a separate tradeoff. No production writer or live conversion.

## LL1 disposable retirement constraints — 2026-09-18

The [isolated engine fixture](architecture/verification/LL1_RETIREMENT_CONSTRAINT_FIXTURE.md)
passes 38 SQLite and 40 private-cluster PostgreSQL cases for the narrow
current-name FK/permit proposal. It is not a production migration or
authorization proof; full-schema closure, role/RLS and retirement-facade
review remain gates. No Neon or installed service was touched.

## PS2 active-prefix ownership adapter — 2026-09-18

The [disposable adapter report](architecture/PS2_ACTIVE_PREFIX_OWNERSHIP_ADAPTER.md)
and [three raw rows](architecture/verification/ps2-prefix-ownership.jsonl)
cover same-inode prefix growth, original-hold retry, sealing and retirement
for data/locator/control-like packs. Sixteen release tests, strict Clippy and
formatting pass. It uses a forked bounded ownership model, not actual v3
locator pages or a Graph writer. Twenty-four syncs per group and substantial
write amplification are negative performance evidence. Integrate real v3
placement/liveness, qualified storage and Graph/journal receipts before wire
selection or production `Saved` claims.

## PS2 refundable allocation-credit prerequisite — 2026-09-18

The [standalone disposable report](architecture/PS2_ALLOCATION_OWNERSHIP_CREDIT_PREREQUISITE.md)
and [three raw rows](architecture/verification/ps2-allocation-credit.jsonl)
show exact retirement releasing a recorded project allocation charge only
after pin/root/unknown-outcome and physical-absence checks. Nine release
tests, strict Clippy and formatting pass. This never credits OS free space.
Extending active-pack charge growth, locator/control metadata ownership,
qualified availability and genuine Graph+v3 integration remain open. No
production reserve, `Saved` path or live package writer is established.

## PS2 genuine Graph/journal furnace — 2026-09-17

The [disposable report](architecture/PS2_GRAPH_JOURNAL_CHECKPOINT_FURNACE.md)
and [exact six-row data](architecture/verification/ps2-graph-authored-v2.jsonl)
add real Core Graph commands, original typed operation receipts, finite
journal groups, controlled interruption/recovery and old-ID lookup at
1k/10k/100k. Seven release tests, strict Clippy and formatting pass.
Radix writes fewer process bytes in this simple append-only pack at 100k;
the combined physical fixture below still favors B-tree in total physical
bytes. Keep both until the genuine Graph path is integrated with the physical
locator/liveness/reserve and qualified barrier model. No production `Saved`,
permanent wire, live package write or migration is established.

## PS2 consolidated packed engineering — 2026-09-17

The [consolidated evidence](architecture/PS2_CONSOLIDATED_ENGINEERING_EVIDENCE.md),
[final scale/economics data](architecture/verification/ps2-combined-packed-final.jsonl)
and [independent fault record](architecture/verification/ps2-combined-v3-independent-review.jsonl)
are ready as a disposable engineering checkpoint. Bounded append packs and
HEAD-selected locator roots avoid tail copying. The combined fixture covers
all twelve root/pin classes, exact work-bound liabilities, verified physical
continuation, candidate-local reclaim and no-effect unprofitable refusal.
Thirty-four release tests (15 independent actual-v3 checks), strict Clippy,
formatting and whitespace checks pass. The independent rollover/restart
counterexample is retained as regression evidence.

At 100k, packed total bytes/op at batch 32 are 20,303 radix versus 17,562
B-tree; B-tree is the provisional packed lead, radix the comparator. An
oversized 3.34 GB compaction reserve was rejected and reduced to 3.3–3.6 MB
modeled admission with no speculative deletion credit. The fixture still has
a nonrefundable high-water ledger and cannot qualify real APFS/quota free-space
progress. Actual disposable remount/abrupt-power trials and arbitrary-path
provider exclusion are unexecuted; macOS storage and production `Saved` remain
unqualified. Integration of the separate genuine Graph/journal evidence with
this physical path, refundable ownership accounting and shared Rust
multi-surface coordination precede any
permanent wire or writer approval. No live package, migration, production GC,
Asset Store, project switch, commit or push was performed in this review.

## PS2 batching/packing/qualified flush review — 2026-09-17

The [disposable review](architecture/PS2_BATCHING_PACKING_AND_FLUSH_REVIEW.md)
and [raw evidence](architecture/verification/ps2-batching-packing-flush.jsonl)
compare the ordinal sequence plus compressed radix map with the paged B-tree
under sealed and relocatable 1 MiB segments, 1/8/32-operation batches and
complete 1k/10k/100k histories. At 100k, the 32-operation batch wrote
7.20 KB/op (radix) versus 10.27 KB/op (B-tree) of fixture pack bytes, but
the current uncached relocation locator has large and unstable read/latency
cost. Whole-segment retirement reclaimed nothing; relocation at 100k radix
copied about 2.04 MB to save only 53 KB of current data. The full scale run
took 2,238 seconds wall; anomalous timing is preserved, not explained away.
Five release example tests, strict Clippy and formatting pass. Journal-only,
pack/HEAD and relocation cuts reconcile original synthetic operation IDs.

The recommendation is a **conditional logical** radix write-heavy lead,
not a permanent wire or physical-pack choice. `Accepted` requires a qualified
durable journal prefix; `Saved` requires selected package closure and original
checkpoint receipt for the latest authored revision. Production still needs
physical locator/dispatch and incremental-liveness decisions, real reserve
proof, macOS full-file/directory barrier qualification, authored Graph-journal
recovery, and process/power fault evidence. The synthetic fixture does not
authorize production reader/writer, migration, GC, Asset Store, live packages
or project switching. Candidate flat wire/goldens remain unfrozen.

## PS2 index/inventory scaling — disposable real-file review

The [scaling review](architecture/PS2_INDEX_INVENTORY_SCALING_REVIEW.md)
compares flat index/inventory, linked segments, LSM runs, paged trees and a
compressed OperationId map plus dense ordinal Merkle sequence over shared
immutable receipts. A disposable 1k–1M operation byte-work model shows the
flat rewrite problem and a bounded-page proxy, while explicitly **not**
claiming filesystem durability or acceptable physical write amplification.
The [real-file comparison](architecture/PS2_INDEX_INVENTORY_REAL_FILE_COMPARISON.md)
now tests complete 1k–1M disposable fixtures for compressed radix and paged
B-tree OperationId maps over the same ordinal history, receipt and pack.
Radix leads on incremental bytes, B-tree on bulk-built baseline size. Both
are dominated by an unoptimized ten-sync publication schedule. Structural
opening, old-ID retry, full audit and fault/recovery cases are measured, with
raw evidence retained. The user approved structural/read-only imported opening
versus explicit exhaustive audit and audit-gated writable admission, now
reflected in the publication contract. The next review is batching/flush,
storage layout/compaction, inventory edges and qualified durability before
revised permanent wire. No production reader/writer, live-package data, or
existing flat goldens changed.

## PS2 exact wire appendix and golden bytes — paused

The [production boundary](architecture/PS2_PRODUCTION_CODEC_AND_PUBLICATION_CONTRACT.md)
is approved as direction. The [candidate exact wire appendix](architecture/PS2_PRODUCTION_WIRE_APPENDIX.md)
and [19 golden canonical-byte vectors](architecture/proposals/ps2/sealed-wire-golden.json)
are not frozen; their flat index/inventory portions await a revised proposal
after the real-file scaling and physical-storage gates.
The selected commit's 1.2 minimum-reader floor is
separate from bootstrap, HEAD/commit schema, public app and filename versions.
Portable `working-binding` contains only logical location/coordinate state,
never host observations or grants. A test-only encoder check pins bytes; no
production reader, publication/recovery code, live writes or conversion was
added. Next: review real-file comparison and unresolved storage gates before
revising exact bytes or implementing production reader/synthetic publication. Shared Rust
session/autosave follows, before project switching.

## 2026-09-17 local main cleanup safeguard

The old local `main` checkout was fast-forwarded to published `305987d` after
preserving its 71-path pre-fast-forward onboarding state in a **local-only**
recoverable stash (`ce39772a4008c886265ac9a25485b2970d0f8332`). The checkout
is clean and matches `origin/main`. The stash is not part of GitHub history and
must not be dropped yet. The [roadmap housekeeping gate](../ROADMAP.md#housekeeping-after-real-project-browsing-and-switching)
requires comparing it with validated `main`, recovering any unique work, and
only then dropping it after real project browsing/switching acceptance.

## 2026-09-17 PS2 experimental sealed-root codec — disposable implementation

The [checkpoint](architecture/PS2_EXPERIMENTAL_SEALED_CODEC_CHECKPOINT.md)
records a test-only on-disk codec, reader, and recovery/reconciliation boundary.
`HEAD.json` remains the sole atomic dispatch; independently validated active
and recovery roots include authoritative resource/version/backing/provenance/
retention records. The actual old reader refuses the required feature. Original
1.1 bytes remain independently readable throughout the interrupted opt-in
conversion fixture. Synthetic 8 GB external media is never read during ordinary
root opening/turnover. Production code and live packages remain unchanged.

Verification: 24 focused tests and 149 full store tests passed, with four
intentionally ignored fixture generators/probes; strict Clippy, formatting and
whitespace checks passed. Next review exact production format/feature,
storage qualification and publication protocol; then shared Rust session/
autosave coordination before Swift project browsing/switching, followed by
Library lifecycle. No new architectural decision arose from this fixture.

## 2026-09-17 PS2 positive sealed-root fixtures — ready for review

The [fixture evidence](architecture/PS2_POSITIVE_SEALED_ROOT_FIXTURES.md)
records 19 new positive tests, 125 passing store tests and four pre-existing
ignored tests. Sixty-four root turnovers retain at most eight package IDs;
synthetic 8 GB external media has zero reads during ordinary metadata/open/
autosave/turnover cycles, including offline and ambiguous backing scenarios.
Nine root and six cross-store interruption cuts, exact dedupe, conversion-source
preservation, finite pins, placement, capacity and backup scope are covered.
The 1.1 negative reader-boundary tests remain unchanged and pass.

Only test-module registration and three test-only fixture files changed, plus
this handoff, roadmap and evidence report. No production package code, wire
format, migration, live package, Asset Store, GC, directory layout or thresholds
changed. The in-memory model is not physical durability or performance evidence.
No new architectural decision emerged. Next review the production root/feature/
dispatch contract and conversion/retirement/rollback specifics, then separately
qualify storage and publication. PS2 and PS3 remain incomplete. The fixture
checkpoint was committed and pushed to `main` as `f8c387b`; no installation or
live-data operation was performed.

## 2026-09-17 resource/storage contract — next PS2 gate

The reviewed direction is consolidated in
[Resource storage and verification](architecture/RESOURCE_STORAGE_AND_VERIFICATION_CONTRACT.md)
and sequenced in the [roadmap](../ROADMAP.md#resource-location-and-verification-contract--2026-09-17).
It distinguishes the transactional package from user-managed source locations
and configurable managed asset storage; mutable working bindings from immutable
captured versions and independently retained byte backings; availability from
retention compliance; and capture evidence from an indefinite retention promise.
Library storage slots and device Host Bindings remain the only location-variable
mechanism. Approved initial default spellings are `$library.project_store` and
`$library.asset_store`; `$project.root` and `$project.artifacts` keep their
existing meanings. The implementation accepts `$library.<slot>` while older
conceptual documentation used `$library.storage.<slot>`; the contract records
that reconciliation rather than adding another path language.

**Completed fixture step:** implemented disposable PS2 positive sealed-root fixtures under
the amended package-closure versus external-backing boundary. Cover independent
roots, exact keep-set, no large-media reads for metadata checkpoints, offline
versus confirmed loss, pinned versus retirement-eligible captured versions,
qualified placement and cross-store interruption. Preserve legacy 1.1 package
bytes and negative boundary fixtures. Review fixture results before choosing a
production codec, migration, managed-store publisher, asset cleanup or native
integration. The current reader still hashes embedded blobs on full validation;
the documented structural-open/deferred-hash distinction is not implemented.
No storage profile beyond the separately qualified initial scope, asset-store
writer, CLI/agent routing, live-data migration or deletion is implied by this
documentation checkpoint. The entries below retain earlier PS2 evidence.

## PS2 current checkpoint — disposable recovery only

Isolated branch `codex/ps2-durability-furnace` now includes child-process exits
inside checkpoint resume (`bc85ae1`), after every missing immutable publication
and replacement HEAD from every initial publication prefix. Fresh recovery keeps
the original persisted intent and IDs, validates old/new closure, and compares the
complete final file tree across repeated completion. The single-RenameGraph proof
(`69ad2db`) is now also wrapped in a fixture-only typed Mutation body shared with
the framing furnace. It rejects altered IDs/unknown fields before append; fresh
process recovery after an uncertain write reuses one unchanged frame and the same
checkpoint IDs/virtual bytes. Journal helpers were moved entirely into integration
tests. Store checks on macOS: 92 passed, 4 intentionally ignored; strict all-target
Clippy and formatting pass. The separately executed
[macOS probe](architecture/PS2_MACOS_PROBE_OBSERVATION.md) records successful
file/directory full-sync and exclusive-rename collision refusal on one local APFS
configuration. Namespace persistence ordering and provider exclusion remain
unproved. No production body schema or qualified profile is selected.

[Retention/storage direction](architecture/PS2_RETENTION_STORAGE_DECISION.md),
[macOS qualification](architecture/PS2_MACOS_STORAGE_QUALIFICATION.md) and
[writer admission](architecture/PS2_WRITER_ADMISSION_PROPOSAL.md) now distinguish
approved sealed-root retention and registered cooperative in-place editing from
unfinished implementation/qualification. No managed-root requirement, production
threshold, format number or migration is selected. The
[shared Rust contract](architecture/PROJECT_SESSION_DURABILITY.md#shared-authority-authorization-and-concurrent-clients)
and [typed proposals](architecture/proposals/ps0/CONTRACTS.md) cover all controlled
surfaces, authorization/provenance, owner epochs versus attachments, ordered dedupe,
finite flush and client lifecycle. Future voice/chat workflow proposals use that
same boundary. A GUI-held lifetime lease returns WriterBusy to a direct second
process; routing/handoff and agent-progress guarantees remain deferred.
The following compiled interface slice removes the impossible storage exclusion
assertion and requires opaque registered admission for every mutating `PackageIo`
method. No production constructor, registrar or writer exists; primitive policy
success does not authorize writes. Four synthetic tests check absent/mismatched
HEAD/incarnation/volume/owner/protocol bindings, and four compile-fail doctests check
forgery/raw-lock/default/profile bypasses. `WriterBusy` is typed; no actual GUI/agent
contention or progress claim is made. Current store checks: 96 regular tests plus
4 compile-fail doctests passed, 4 ignored; strict Clippy/fmt pass.
The subsequent [logical-client fixture](../crates/photara-store/tests/package_planning/authority.rs)
adds GUI/agent ordering with actual pure rename planning, private host-grant
fixtures, stale-coordinate refusal, original-ID dedupe, ordered reconnect/gap
behavior, finite virtual checkpoint prefixes, and detach isolation. A second logical
owner gets WriterBusy; no real OS contention, authorization service, durable append,
owner-turnover recovery or production coordinator is implemented. Current store
checks: 98 regular tests plus 4 compile-fail doctests passed, 4 ignored; strict
Clippy/fmt pass. See [evidence limits](architecture/proposals/ps0/VERIFICATION.md#completed-bounded-logical-client-evidence).
Full typed mutation/undo replay, platform barriers, the remaining I/O failure
matrix, retention and production integration are unfinished. No installed app,
live package, main-branch publication or power-loss qualification is implied.
Continue safe disposable work; qualification and production integration remain
separate gates, not a request to reapprove the accepted directions.

## PS1 accepted; PS2 next — 2026-09-16

[PS1 implementation/evidence](architecture/PS1_PURE_PACKAGE_PLANNER.md) starts from
published BR0 `5777fce04a9c1d88cbf249581efdb09055acb2f8`. It adds typed pure
commands/prepared receipts, deterministic immutable byte plans, shared virtual 1.1
validation, exact HEAD/manifest/incarnation comparison and interface-only I/O
contracts. Configured outer extensions and stable internal schemas remain intact.

**PS1 accepted for publication.** No filesystem edit writer,
journal, native autosave/session coordinator, project switch or migration is wired.
The [retention proposal](architecture/PS1_RETENTION_PROPOSAL.md) now has an
[approved direction](architecture/PS2_RETENTION_STORAGE_DECISION.md), not an
implementation: 1,024 commits remain a hard bound, with no history truncation
or indefinite-autosave claim. Exact 15-file inventory and tests are in the report.
PS2 is the disposable writer/journal/recovery furnace; PS3 is native session and
autosave integration. The BR0/PS0 entries below are historical review checkpoints.
After PS2–PS3, consolidate durable reference documents and retire superseded
checkpoint prose through reviewed, incremental cleanup; keep user and Node SDK/API
guides as the product matures.

## BR0 implementation review gate — 2026-09-16

[BR0 Brand Cutover / Rename Readiness](architecture/BR0_RENAME_READINESS.md) implements the configurable public
identity seam from published PS0 `6cfd0f12a9cc42c7e7ff28ded84d1dffd8975f26`.
The development identity remains Photara. Synthetic-brand/extension builds and
disposable creation/reopen checks exercise the cutover without live trust enrollment.
Known UI/default/extension leaks are addressed; internal identifiers remain stable.
Native synthetic GUI acceptance also passed: create `.jprtest`, quit/restart/reopen,
and open a completed legacy `.photara` fixture. Both package manifests stayed
identical; the isolated app is closed. Protected-file audit: 1,838 unchanged
entries. One Keychain WAL metadata change is disclosed without attribution in
the report. The 28-file change remains uncommitted for review.

**Awaiting Suhail review: no commit/push/install/deploy authorization.** BR0 must be
accepted before PS1 begins. Production branding/trust enrollment and controlled
local-directory migration remain separately reviewed work, not automatic rename
side effects. No durability writer, retention implementation, project switching,
LL1 or COV work is included. The PS0 entries below are historical publication gates.

## PS0 architecture review gate — 2026-09-15

[Project Session and Graph Durability PS0](architecture/PROJECT_SESSION_DURABILITY.md) is drafted against published
LL0 baseline `114ce43af1089c8bac7fa5c3010a440521027571`. It audits the legacy
explicit-save and UI1 read-only package routes and proposes journal, writer,
coordinator, failure recovery and review-only typed/schema contracts.

Order: **PS0 review → BR0 Brand Cutover / Rename Readiness → PS1 pure
writer/contracts → PS2 disposable persistence and
recovery furnace → PS3 native coordinator/autosave → PS4 Project-switch integration**.
Retention/current-reader compatibility is a production gate. No writer/session
wiring, migration numbering/execution, package mutation or real switching occurs
in PS0. Stop for Suhail review; no commit or push is authorized for this checkpoint.

**PS1 is blocked until BR0 passes.** Branding, website, domain, marketing name and
production trust enrollment stay deferred until the product is concrete. BR0 proves
public renaming and extension replacement are a bounded configuration cutover,
including synthetic-brand tests, legacy-extension reads and explicit security/local-
directory transition contracts. Stable internal compatibility IDs remain unchanged.
Known hard-coded naming leaks are BR0 inputs, not readiness claims; PS0 implements
none of this gate and authorizes no live trust enrollment.

LL0 remains accepted/published. LL1 remains independently gated; PS0 does not start
Library lifecycle implementation. COV and Gallery work remain separate. The older
LL0 publication authorization below applies only to that historical checkpoint.

## 2026-09-15 accepted LL0 checkpoint — next slice PS0

Suhail accepted [Library Lifecycle LL0](architecture/LIBRARY_LIFECYCLE.md), including the native Library/account
switcher. The bottom-left avatar/name trigger opens its menu **above** the row;
cloud Libraries are grouped by account, true locals under On This Mac, with one
native current checkmark. The sidebar contains only active-Library content.

LL0 contains the contract, lab-only fixtures and native verification evidence.
Create/select/rename/remove semantics include exact-name and final confirmation,
owner authorization, retry/reconciliation and catalog cleanup while files remain
untouched. Production lifecycle wiring and schema changes are not implemented.

The next distinct slice is **Project Session and Graph Durability PS0**, from the
clean published LL0 checkpoint. This publication does not begin PS0. **LL1 Library
Lifecycle typed contracts and unnumbered schema delta review remains separately
gated**; PS0 neither starts nor replaces that Library Lifecycle work.

This checkpoint authorizes the reviewed LL0 commit and a normal fast-forward push
only. No install/deploy, live database/package/account operation, migration numbering
or DDL. COV0/COV1/Gallery remain separate; the recorded COV0 ROADMAP conflict is not
merged here. UI1 remains accepted/published at `b0c2ba7`, with publication record
`913f3526f51eaa1aaf34ef83ead2860869465e85` as this checkpoint's verified parent.

Verification: 78 native captures (Light/Dark and narrow states), 22 menu placement/
action checks, 81 switcher context transitions; shared UI 119 captures, 256 scenario
transitions and 50 contrast pairs. Shell Lab builds. All 273 protected paths and
29 migrations match the parent. Accessibility metadata and native mouse/keyboard
checks pass; human VoiceOver speech/navigation remains explicitly manual.

## Historical checkpoint entries

## 2026-09-15 UI1 checkpoint published

Review was accepted and Git publication was explicitly authorized. Checkpoint
`b0c2ba7ab26968a7863d63add48a7980170347f8` — `feat(ui): create projects transactionally` — contains exactly the
accepted **78-file UI1 integration inventory**, with all staged and committed bytes
checked against the accepted hashes. Its parent is `c605475742f22581060ff009fcbcfd2a3acc4e36`.

`git push origin HEAD:refs/heads/main` completed as a normal fast-forward
(`c605475..b0c2ba7`, no force). At **2026-09-15T21:43:19+00:00**, local `HEAD`, local `origin/main`,
remote `refs/heads/main`, and remote default `HEAD` all resolved to the checkpoint;
the remote default remains `main`. The source worktree was clean after the push.

This documentation-only follow-up changes `ACTIVE_HANDOFF.md` and
`UI1_INTEGRATION_CHECKPOINT.md`. It records the completed checkpoint publication;
its own final commit ID and remote verification are retained in the publication
task's completion record, avoiding a self-referential evidence commit. The machine
inventory hashes describe the **checkpoint commit's tree**, before these two
publication-record edits. The [checkpoint report](architecture/UI1_INTEGRATION_CHECKPOINT.md)
records the retained raw-log whitespace exception and detailed evidence.

Graph acceptance remains **19,093 assertions / zero failures**, with all prior
verification and failed-run history retained. Publication performed no app
installation/launch, service deployment, live SQLite/Neon/Auth0/Keychain change,
COV0 merge, or Library Lifecycle implementation.

**Next separately gated step:** scope Library Lifecycle under the accepted roadmap.
COV0 documentation remains separate, with the recorded single ROADMAP conflict.
App/service rollout and live migration acceptance require separate authorization.

## 2026-09-15 UI1 integration checkpoint — pre-publication verification

The following records the accepted verification state before Git publication.

The uncommitted `/Users/suhail/.codex/worktrees/6569/photara` worktree is ready for
review and a separately authorized UI1 checkpoint commit. **No commit, push,
installation, publication or live-data change was performed.**

Final full Graph gate: **19,093 assertions / 0 failures**, exit 0, all 12 native
configurations and 12×180 seeded actions. It ran alone, with frozen source hashes,
no targeted/fault/preflight flags, no concurrent compiler/verifier, no rebuild and
no interactive approval after gate entry. The initial 17,469/12 failure, intermediate
19,090/1 full run, and two reported windowless replay attempts remain historical
failures, not passing evidence.

The harness now explicitly creates/identifies its native AppKit test window and
logs process/lifecycle entry before verification. A PID/token/window handshake and
external 15-second startup / 1,200-second execution watchdog prevent indefinite
windowless/stalled runs; diagnostics and cleanup address only the owned process.
Native menu selection timers cancel when tracking ends, and bounded native focus
acquisition/restoration checks active app, key window, first responder and actual
foreground PID. No production Graph, oracle, tolerance or original assertion was
weakened. Three native hit-tests now use AppKit's required superview coordinates;
an independent translated/flipped fixture rejects the original mistake. The focused
camera/overview check passes **176/0**. Native toolbar/sidebar/chrome remain OS-owned.

Fresh checks: exact failing Light/Straight seed **924/0**; all 12 context-menu
configurations **2,239/0**; eight real-process furnace scenarios **49 launcher
checks + 60 native assertions / 0 unexpected failures**; prompt-free signed native
permissions, shell/Python syntax and 40-file ownership guard. The furnace includes
cold launch, deliberate windowless/stalled startup, recovery, runtime timeout,
owned-child cleanup, and genuine foreground interruption by a separate process.
The historical randomized action33 failure remains unclassified beyond its
pre-tool-switch branch-ownership precondition; exact replay/full run now pass.

The [checkpoint report](architecture/UI1_INTEGRATION_CHECKPOINT.md) and
[machine-readable evidence](architecture/verification/ui1-integration-20260915.json)
contain the exact 78-file inventory, command/results and preservation checks.
All prior non-Graph integration gates retain their measured passes and unchanged
inputs. All four source worktrees and all existing source app bundle bytes/modes/
Team IDs are preserved. SQLite 0015/floors 5/5, PostgreSQL 0014/API 3 and package
1.1 remain unchanged. Live Keychain/login/service rollout and CXT4e remain separate.

**Next eligible step:** review this UI1 checkpoint; a commit still needs explicit
authorization. The accepted next implementation slice is separately scoped
**Library Lifecycle**. COV0 documentation may be reconciled separately after UI1
acceptance: retain both ROADMAP entries at its one conflict and refresh UI1/0015
references. No COV0/COV1/Gallery/browser/lifecycle implementation is included.

## 2026-09-15 initial UI1 integration checkpoint — historical Graph failure

This section records the initial failed checkpoint. The resolved verdict and current
inventory appear above.

Reviewable, uncommitted integration worktree:
`/Users/suhail/.codex/worktrees/6569/photara`, based on remote default `main` at
`c605475742f22581060ff009fcbcfd2a3acc4e36` (verified read-only).
**Not safe to commit as a verified UI1 checkpoint yet.** The clean sequential
full Graph run completed all 12 configurations and 12×180 seeded actions with
**17,469 assertions / 12 failures**, exit 1. Failures include routed-edge disconnect,
surviving/random branch ownership and native focus. Do not substitute the earlier
source worktree's passing run for this result or weaken any oracle/tolerance.

The [integration report](architecture/UI1_INTEGRATION_CHECKPOINT.md) contains the
exact **72-file inventory**, source overlaps/base/divergence, integration decisions,
commands/results, protected-bundle checks and remaining gates. Its
[machine-readable evidence](architecture/verification/ui1-integration-20260915.json)
includes full source-delta hashes and bundle file inventories. The next checkpoint
action is to diagnose Graph's failures and obtain a clean full sequential pass.

Passed: 289 Rust tests; all-target check/Clippy/fmt; schema/fixture/naming/ownership
and configuration guards; synthetic native authentication; 12 private PostgreSQL
tests plus native HTTP/recovery furnace; bridge; isolated signed build; production
UI (27 captures); shared UI (117 captures). SwiftPM manifest/module/cache/config
paths were additionally confined to the selected build root. Hostile inherited
production output/signing/cache settings created none of their forbidden paths.

All source files in main, `12ef`, `3e99` and `deb1` are unchanged. All 20 regular
files in each signed source app, and all 16 in the older ad-hoc Graph-context app,
retain exact bytes; bundle entries/modes and Team IDs are unchanged. The signed
`12ef` app validates under Team `524GTA93Q3`; the Dropbox checkout bundle separately
fails strict validation because of a FinderInfo xattr and was not repaired.
SQLite adds only 0015/floors 5/5; PostgreSQL remains 0014/API 3; all 28 existing
migration files and package format 1.1 are preserved. No live data, Keychain entry,
installed service or interactive app was changed. Keychain-write/live-login and
matching desktop/service rollout gates remain separate.

COV0 remains documentation-only and separate. Its dry three-way merge has **one
ROADMAP.md conflict**; the other overlapping documents merge cleanly. After UI1
acceptance, retain both entries and refresh UI1/0015 dependency wording. Do not
activate cover/Gallery migrations or implement COV0/COV1/browser/lifecycle work.
No commit, push, publication or installation occurred. Older checkpoint sections
below retain historical evidence and do not override this current failed gate.

## 2026-09-15 native verification harness hardening

Production UI verification now builds its input app, generated product identity,
bindings, Rust/helper outputs and caches under a fresh owned run directory. It
cannot replace the default signed interactive app; all 20 existing bundle file
hashes and Team ID `524GTA93Q3` remained unchanged. Snapshot sizing preserves real
macOS chrome/toolbars, checks both content dimensions across native layout turns,
and explicitly verifies the production Library toolbar is populated. Graph test
input drains pending layout and requires stable native focus; production Graph
sources, fixture data, oracle and original behavioral assertions remain unchanged.

Final gates: **18,086 Graph assertions / 0 failures**, all 12 configurations and
12×180 seeded actions; two production passes with **27 captures each**; shared UI
pass with **117 captures**, 225 transitions and native Opening/Create accessibility
probes. Configuration tests (7), ownership guard (37 files), shell syntax and
whitespace pass. Earlier runs exposed focus/native input failures; the final full
run passed after the stability precondition. A follow-on isolated wheel replay
stalled before producing lifecycle output and was stopped on request; the queued
knot replay did not run. Do not claim those additional replays passed. Both seeds
are covered by the successful final full matrix. No more gates are running.

See [UI1 hardening evidence](architecture/UI1_CREATE_PROJECT.md#2026-09-15-native-verification-hardening).
No commit, push, production app launch/installation, service change or live database
write was performed by this hardening task. The native-first design policy is
preserved; fixture geometry is test-only and no native chrome is imitated.

## 2026-09-14 UI1 real Create Project implementation

UI1 is implemented from `c605475` in the uncommitted working tree. The approved
shared Compact sheet and native Opening are preserved. Rust now owns the initial
Project/Graph package, durable SQLite creation/recovery and exact cloud command;
the existing local development Rust HTTP service transactionally registers and
projects it in PostgreSQL. No desktop database credentials or Fly resources were
introduced. Completion uses the existing safe saved-Graph shell, with legacy
single-document authoring actions unavailable for package projects.

The [implementation, recovery boundaries and measured checks](architecture/UI1_CREATE_PROJECT.md)
cover native destination selection, collision/permission/volume checks,
duplicate/concurrent/restart recovery, cancellation, package/catalog rollback and
lost cloud reply reconciliation. SQLite adds migration 0015 (local floor 5);
PostgreSQL stays at migration 14/API floor 3. The full Graph furnace passes 15,644
assertions with zero failures; original Graph source/oracles are unchanged.

**No commit, push, installation, deployment or live user/Neon change is authorized.**
Those actions require explicit publication approval after reviewing the local schema
floor and matching service endpoint. UI2/UI3 and CXT4e remain open. Library Lifecycle
is the next separately gated slice: exact Library-name retyping, current
ownership/authorization and impact counts; transactional catalog/reference cleanup
without database orphans; never deletion of packages or source/archive files.

The earlier visual-checkpoint status and its publication authorization below are
historical and do not authorize publication of this implementation.

## 2026-09-14 UI0 accepted; UI1 Compact visual checkpoint accepted

**Final checkpoint verification passes on Xcode 27:** Graph 15,644 assertions,
zero failures, all 12 configurations and 12×180 seeded actions; shared UI 225
transitions/50 contrast pairs/117 captures; production UI 25 captures; bridge,
Theme validation, Theme/Shell/Graph/production builds, signature checks, source
guard, formats and neutral raster checks. The 37-file Graph guard preserves every
original behavioral assertion/oracle and permits only the three approved production
Theme edits plus the explicitly authorized verification-harness adaptation.

The verifier now has stable development signing, a screen-capture purpose string,
confirmed AppKit launch readiness and real native-input/capture probes. The user's
approved app-specific Accessibility reset removed an old ad-hoc identity record;
manual authorization is complete and survives the final rebuild. No global TCC
reset, automated consent, production Graph behavior change or new UI1 creation
transaction was performed. The self-targeted AX diagnostic's error remains logged;
actual HID receipt and the original Graph matrix establish interaction readiness.

This checkpoint starts from `ef3fc00` on `codex/ui0-ui1-visual-checkpoint`.
The machine-readable evidence and approval images are linked below. Delivery is
an authorized normal fast-forward push only; never force.

Suhail approved UI0 and the rendered shared Create Project view, selecting **Compact**
as the shipped default. Balanced and Spacious remain Shell Lab comparison options.
The checkpoint starts from `ef3fc00` and preserves all existing UI0 edits plus the
reviewed native Opening and UI1 mock. The default is defined once by
`CreateProjectPresentation.shipped`; the shared view and Lab model consume it.

Opening uses native `windowBackgroundColor` and primary/secondary text. It is the
native exception before Photara's authored Foundation/Primary/Inset ladder begins.
Graph's background shares Foundation; its accepted nodes, ports, noodles, controls,
controller, geometry and presets remain unchanged apart from UI0's three exact
Theme-consumption edits. Test assertions and oracles are preserved; only authorized
verification launch/input plumbing and preflight were adapted. Theme Lab owns the palette, and Shell
Lab owns scenario-specific geometry plus the three Create Project comparisons.

`CreateProjectView` is shared production source displayed by Shell Lab. Its native
Choose/Cancel/Create buttons invoke fixture callbacks only in the preview. The real
production creation route is not connected to the new sheet. No real package,
database or cloud creation workflow has been implemented. Existing production/Core
behavior is preserved. Name trimming and nonempty enablement are presentation checks,
not complete path, collision, permission or transactional validation.

See the [accepted checkpoint, Xcode 27 verification and approval images](architecture/UI_LADDER_AND_AUTHORING_SEQUENCE.md#accepted-ui0-and-ui1-visual-checkpoint).
Compact Light/Dark captures are retained under `docs/architecture/mockups/ui1`;
the full verification run also captures Balanced and Spacious. The refreshed Opening
rasters retain native macOS appearance. All account/project test state is disposable.

Next: separately scope and implement UI1 creation wiring. Library lifecycle remains
a separate future slice: deletion transactionally removes catalog Project records
and references, never `.photara` packages or external archives. No live Neon/Auth0/
Keychain data or service deployment is part of this checkpoint. Commit and normal
fast-forward push are authorized only after the checkpoint checks pass.

## 2026-09-14 committed checkpoint and proposed UI0

The user confirms the live first-Mac Google/cloud flow works and supplied Light/Dark
opening captures. CXT4b–d, recovery, account controls and their test furnaces were
committed and pushed to `origin/main` as `45ddedf`. CXT4e second-Mac hydration and a
complete independently captured aggregate audit remain open.

The next proposed work is documented in
[UI ladder and authoring sequence](architecture/UI_LADDER_AND_AUTHORING_SEQUENCE.md).
Preserve Graph Lab. UI0 freezes a reusable three-step neutral ladder, rebuilds
Theme/Shell Lab ownership, removes the Opening content tint, renders real Light/Dark
production states and stops for approval. UI1 then designs and wires the first real
Create Project operation across Library catalog, initial Graph, package and cloud
projection. Do not begin UI1 implementation before its mockups are approved.

Updated 2026-09-13 for the account sidebar aesthetic cleanup.

The user confirmed live sign-in/sign-out and library selection work. Their next
request was visual: move Accounts to the bottom-left library sidebar, shrink the
avatar, and show a person icon with **Sign in** when signed out. The account entry
now sits under the library status in the sidebar; the project editor uses the
same lower-left control. Clicking the compact avatar/name opens the existing
account menu. Settings is accessible there, replacing the empty top-right gear.
The top-right account control is removed. There is no signed-in banner.

The photo is rendered into a circular **20-point** NSImage as well as constrained
in SwiftUI. This fixes native Menu label image extraction bypassing the former
view-only frame/clip and displaying the larger cached photo. A **512×320 raster**
fixture now exercises that path in the production shell, including Light/Dark,
first launch, signed out, returning sessions, and 760×560 / 1280×820 windows.

Signed build + production UI verification passed (exit 0), with visual inspection
of the account/footer and project views. Log: `/private/tmp/photara-sidebar-accounts-ui.log`.
Current UI evidence: `platform/macos/photara-app/.build/cxt4d-gates/sidebar-ui-passed.json`.
Only **4 UI/fixture files** differ from the previous full gate; its **101 other
source hashes** still match. Authentication, service and storage code is unchanged.
The full gate below is historical evidence for that unchanged functionality; it
was not rerun for this presentation-only change.

Current signed app: `/Users/suhail/.codex/worktrees/84ad/photara/platform/macos/photara-app/.build/app/Photara.app`.
Executable SHA-256: `a87e52ccbb977d31d90706ba579a9fed075ddeb7f0c4d9392c07271db2372661`.
No live login/logout, credential change, Neon write, service replacement, commit
or push occurred. Quit/reopen the app to see the layout change.

## Earlier Accounts and library-selection checkpoint

Updated 2026-09-13 for Accounts, explicit logout/rejoin and library selection.

The user confirmed that the previous fix completed live Google sign-in and
created a Neon record, and supplied the cloud-library opening screenshot. That
is user-reported live success; this task has not independently queried the new
production account or receipt. The earlier blocked live-attempt notes below are
historical, not the current status.

The new Accounts menu exposes profile name/email, a circular Google avatar when
available, and Sign Out. A deliberate sign-in now presents **Choose a library**,
including when only **My Library** is available. Normal launches restore the
selected session quietly without a prompt or signed-in label. The current service
contract exposes one owned default library; additional created/invited libraries,
billing and account settings remain future work, without placeholder actions.

Logout durably marks the session signed out, then clears the refresh token under
the existing attempt/refresh locks. The device secret and library/receipt evidence
remain intact. Rejoin verifies fresh Google issuer/subject, reuses the exact device
and original enrollment, reads current `/v1/session`, and waits for selection.
Access is fetched again after the selection before Rust applies it. No bootstrap,
new device or account/library creation occurs on rejoin. Wrong-account and cancelled
selection remain signed out. A failed Keychain cleanup cannot restore local session
authority and is retried before the next deliberate authorization.

Name/email/picture come only from a verified ID token and are presentation data.
The local display cache is scoped to environment and local library. Startup reads
that cache without Keychain/network access. Google avatar downloads happen after
successful explicit sign-in, allow only HTTPS googleusercontent.com subdomains,
refuse redirects/cookies/credentials, bound size/time and store a small raster
thumbnail. The existing signed-in user will get the new profile cache on the next
deliberate login. Missing profile/photo uses the circular person icon.

The complete Accounts gate run exited **0**:
`/private/tmp/photara-cxt4d-accounts-gates.log`. Its passing manifest records
**105** unchanged source hashes, independently rechecked with the executable.
Evidence: **94 + 58 + 64** native security/Auth0/presentation assertions,
**124** production-driver boundary assertions, **253** native service assertions
across **17** real TCP/Rust/PostgreSQL scenarios including account restart stages,
all **11** disposable PostgreSQL tests, **99** process assertions across **13**
scenarios, **10** installed-service checks with unchanged port ownership, **8**
signed Keychain checks, and **62** ordinary Rust tests. Bridge/product/schema/naming,
formatting, Clippy with warnings denied, shared UI, production UI and strict signing
checks passed. Light/Dark library-picker and account-control captures were visually
inspected. The shared session survives opening/project-view transitions.

Signed app: `/Users/suhail/.codex/worktrees/84ad/photara/platform/macos/photara-app/.build/app/Photara.app`.
Executable SHA-256: `b6a04b3637e0347635782a95fb614de5e645585193aa13f4c187e7f3f2a890eb`.
The user should quit/reopen this build for their manual Accounts → Sign Out →
Sign in again → My Library check. No live sign-in/logout, Neon write, service
replacement, commit or push was performed by this feature task. Existing dirty
work is preserved.

## Earlier terminal-expiry and passive-launch checkpoint

Updated 2026-09-13 for deliberate sign-in, passive launch and terminal expiry recovery.

Opening now reads only local Rust journal/binding metadata. It does not read
Keychain, start/refresh authentication, launch the service or open a browser.
The first local opening offers **Sign in with Google**. A returning cached cloud
connection opens quietly, with neither a sign-in prompt nor a **Signed in** label.
The prompt is hidden until the local read finishes to avoid a launch-time flash.
Saved signed-out/access-disabled bindings remain distinct; cached presentation
never establishes current online authority. This follows the user's latest
Notion-style preference and supersedes the earlier automatic recovery behavior.

An unfinished attempt presents **Sign in again**. That explicit click first
reconciles the exact retained operation. When authenticated operation lookup,
a proofless exact replay, and a final authenticated lookup confirm no receipt
and the original enrollment proof is unavailable/expired, Rust appends the
terminal disposition **abandoned-expired**. An additive local migration 0014
retains the original command, principal, operation, credential reference and any
later receipt. No deployed migration checksum is rewritten. Local reader/writer
floor 4 keeps old readers from misinterpreting this state.

The deliberate click may then perform exactly one new browser authorization.
The replacement has a new operation/proof and explicit lineage, and reuses the
same Library, Device and credential commitment. The same principal is mandatory.
The native attempt lock and Rust transaction guards serialize competing clicks.
Cancelling or crashing before dispatch keeps the shared original credential.
A late old receipt remains evidence and cannot change the active library selection.
Real Rust/PostgreSQL tests prove one Account/Identity/Device/default My Library
and membership even if the old operation commits before or after the replacement.
Two valid operation receipts in that race are retained, not treated as duplicate
accounts or libraries.

**Neon has tables:** prior deployed-database verification established 14 successful
service migrations and resolved the onboarding relations. Zero account/library
rows did not mean missing tables. The confirmed earlier defect was the service
rejecting Auth0's API plus issuer `/userinfo` audience pair. The corrected strict
policy and capability marker remain installed. See the historical evidence in
[CXT4d checkpoint](architecture/CXT4D_NATIVE_AUTHENTICATION_CHECKPOINT.md).

The live retained operation remains `35d65e1f-48bb-4fb0-ab06-b4ced18cd4f1`.
This task has not run another live Google sign-in, changed its live journal,
written directly to Neon, or claimed successful live cloud enrollment. The user
can choose the explicit sign-in action in the new signed build. All original
live evidence is preserved; the earlier prohibition on initiating another live
sign-in was respected throughout implementation and verification.

The complete `scripts/verify-cxt4d-gates.sh` run exited **0**. Log:
`/private/tmp/photara-cxt4d-terminal-gates.log`. The passing manifest at
`platform/macos/photara-app/.build/cxt4d-gates/passed.json` records **103**
unchanged source hashes; they and the executable hash were checked after completion.

Evidence: **94 + 52 + 56** native security/Auth0/presentation assertions,
**106** browser/journal assertions, **225** native service-flow assertions across
**16** real TCP/Rust/PostgreSQL scenarios (including restart stages), all **11**
executed disposable PostgreSQL tests, **99** process-furnace assertions across
**13** scenarios, **10** installed-service checks with unchanged port ownership,
**6** signed Keychain checks, and **61** ordinary Rust tests. Bridge execution,
product/schema/naming/whitespace, shared UI and signed production UI checks passed.
Separate Rust formatting and Clippy with warnings denied passed. Minimum opening
captures in Light/Dark were visually inspected; the fixture now prevents automatic
window expansion and asserts the requested 560-point content height.

Signed app: `/Users/suhail/.codex/worktrees/84ad/photara/platform/macos/photara-app/.build/app/Photara.app`.
Executable SHA-256: `86d7ca25a946b4932edf3fc32bb2c208465d1772eaad1375f3844117f4630de0`.
The app was built and signature-verified without starting a live Google sign-in.

Existing dirty CXT4b/c/d, graph and lab changes are preserved. No commit, push or
publication occurred.

## Earlier CXT4b-dev runtime checkpoint

Updated 2026-09-13. **CXT4b-dev runtime provisioning and live readiness are
complete on this development Mac.** The three Neon `main/neondb` logins
`photara_dev_api`, `photara_dev_control`, and `photara_dev_auth_read` each inherit
exactly their corresponding existing capability role. Live authentication/audits
prove no owner, Neon-elevated or cross-capability membership, privileged role
flags, object ownership, database CREATE permission or admin-option grants;
each login has a six-connection ceiling.

One non-sync macOS Keychain item stores the three certificate-verified URLs and
a stable random 32-byte cursor key. A native operator-only launcher retrieves the
bundle, strips inherited configuration and directly executes the development
service. Both `/health/ready` and `/health/live` returned HTTP 200 at
2026-09-13 19:36:54 UTC; restart readiness passed at 19:38:18 UTC. Postflight
Accounts/Libraries/defaults/receipts remain `0/0/0/0`; schema epoch/floor is `1/3`
with 14 successful migrations. The proof service is stopped, not installed as a
background daemon. No sign-in, seed, Auth0 change or Fly resource was created.

**Next bounded gate: CXT4d-dev native integration.** Consume the checked identity
descriptor for browser PKCE, user-token Keychain storage, loopback HTTP and local
Library reconciliation. CXT4e still owns first real sign-in and second-Mac
acceptance; secure provisioning of that second host remains outstanding. Read
[runtime checkpoint](architecture/CXT4B_SERVICE_CHECKPOINT.md#cxt4b-dev-runtime-provisioning-and-live-readiness)
and [exact host runbook](../crates/photara-service/deploy/README.md#host-local-keychain-launcher).

## Earlier CXT4b-dev configuration checkpoint

Updated 2026-09-13. **CXT4b-dev configuration implementation is verified; live
loopback readiness remains gated on host-only runtime credential provisioning.**
The checked `config/product-identity.json` feeds typed Rust and generated Swift,
the application title/support-directory seam and bundle/callback metadata.
Explicit development binds `127.0.0.1:8080` through the original service verifier,
pool/role/schema checks and router. No native sign-in or content worker is active.
Hosted channels remain unconfigured and fail closed. Read the
[exact files, evidence, limits and next gate](architecture/CXT4B_SERVICE_CHECKPOINT.md#cxt4b-dev-configuration-checkpoint)
and [operator profile](../crates/photara-service/deploy/README.md).

Next: separately provision three least-privilege runtime login credentials and a
stable cursor key into approved host-local secure operator storage for Neon `main`,
then prove loopback `/health/ready` without signing in or seeding user data. CXT4d
native PKCE/Keychain/bootstrap and CXT4e multi-Mac acceptance remain later gates.
The source/config seam is complete; real multi-Mac cloud use is not yet active.

## Earlier development-policy decision

Updated 2026-09-13. **The development-cloud and replaceable-product-identity
policy is approved; CXT4b-dev was selected next.** Suhail needs cloud Libraries to follow
him across development Macs, but does not want idle Fly compute before production.
Use the identical Rust/Axum contract at loopback on each explicitly configured
development Mac, with host-only secrets and Neon `main` as cloud source of truth.
Fly.io is retained for brief remote-acceptance deployments and permanent production;
destroy test Machines after evidence. Add one typed environment/product-identity
seam before native integration so `Photara`, bundle/callback/package/API coordinates
have one coordinated pre-release cutover. Read
[the authoritative policy](architecture/CXT4_DEVELOPMENT_CLOUD_AND_PRODUCT_IDENTITY.md).

Updated 2026-09-13. **Additive CXT4b migration 0014 is installed on Neon
`photara` / `main`; no user data was seeded.** Read-only preflight proved
`photara.service.g2`, epoch 1, minimum API 2, 13 ledger entries, zero Accounts,
and zero Libraries. The operator-only `photara-migrate` binary then installed
0014. Postflight proves minimum API 3, 14 successful ledger entries, and checksum
`adeadf906a1cd1e97fbc3b31d2dafe3f54435a10224c47ba8254cfa521f7d2ebf97f30a10c6c45e6116be5aad0f91dec`.
`account_defaults`, `device_credentials`, `onboarding_receipts`, and
`onboarding_challenges` resolve in their expected schemas. Accounts, Libraries,
defaults, and receipts are all still zero. `legacy-v0.1.x` was not opened or
changed.

Updated 2026-09-13. **The dedicated Auth0 development client/API boundary is
configured; no user has signed in.** The exact public tuple is:

- issuer `https://dev-nmturasdrkz7up27.us.auth0.com/`;
- API audience `urn:photara:api:development` (API id
  `6aa6e96c3595ecc7d9ad79f1`);
- Native public client `Photara macOS`, client id
  `CJ3hH2CUSkEk0vSbdvHMJLEqD4gYWGkW`;
- callback/logout return
  `com.photara.desktop://dev-nmturasdrkz7up27.us.auth0.com/macos/com.photara.desktop/callback`.

The Native client uses Authorization Code and Refresh Token grants, no implicit
grant, seven-day idle/thirty-day absolute rotating refresh tokens, and Google as
its only enabled connection. The API uses the Auth0 JWT profile, RS256,
ten-minute access tokens, offline access, per-app user authorization, no client
access, and the sole `photara:onboard` permission; the Native client has 1/1 of
that permission. Username/password was disabled for this client only. Existing
Chordrift clients/connections were not edited. No Auth0 user, Account, Device,
Library, credential, Neon row, or product sign-in was created.

Updated 2026-09-13. **CXT4b now has a selected Fly.io production/acceptance adapter
and verified runnable artifacts; billing is configured and live creation is
deliberately deferred.** Added
separate `photara-service` and operator-only `photara-migrate` binaries, a
non-root multi-stage Docker image, `.dockerignore`, and a valid Fly manifest with
managed HTTPS/readiness/concurrency policy. Release compilation, service tests,
strict Clippy, formatting and `fly config validate` pass. Fly CLI 0.4.102 is
installed and authenticated as the user's account. `fly apps create photara-api`
created nothing during the earlier billing prerequisite. Billing has since been
added, but no retry is authorized under the approved development profile. Auth0's
development resources and Neon migration 0014 have since been configured as
recorded above. No Fly app, runtime credential, user-data row or charge was
created. Do not create persistent Fly resources during CXT4b-dev; use a separately
authorized, short-lived remote-acceptance deployment and destroy its Machines.

Updated 2026-09-13. **The user accepted the CXT4c native opening-shell first
visual baseline and explicitly made minor sizing refinement non-gating.** Shared
production/Shell Lab source now uses `NavigationSplitView`, a native sidebar
`List`, unified toolbar, system accent/selection/material, a plain centered
`Photara` title, and leading-aligned local/cloud status. The opening UI remains
presentation-only: its Google action does not contact Auth0, the service, or Neon.
Both Shell Lab and the production app build successfully. Continue with CXT4b-dev
typed identity/environment and loopback-service wiring, then CXT4d; do not mistake
the rendered fake state for
working authentication.

Updated 2026-09-13. **CXT4b has a verified host-independent service checkpoint;
it is not deployed and its gate is not complete.** Started clean at approved
`14f77ee`; nothing is staged, committed or pushed. Read the
[exact implementation, inventory, tests and remaining inputs](architecture/CXT4B_SERVICE_CHECKPOINT.md).

Additive migration 0014/service floor 3, pinned RS256/JWKS verification, typed
HTTP onboarding/session routes, atomic bootstrap/replay, device logout/resume and
the password-disabled three-login operator template are implemented locally.
Ten disposable PostgreSQL suites pass; all prior migration checksums remain exact.
The live Neon floor is now 3 with 14 migrations and no user data created here.

**Deferred remote-deployment inputs:** the Fly app/HTTPS origin and encrypted runtime
secret injection; operator/deployment identity; confirmed release
signing access group and supported macOS range. The Auth0 development tuple is
now concrete above and no value was inferred from Chordrift. Host-specific
packaging, encrypted credential injection, provider settings, backup restoration
and a controlled live smoke test remain unperformed. Do not seed around onboarding.

Full offline Rust, strict Clippy/all-target checks, schema/naming/fixture guards,
native bridge and production UI pass. Serial Graph rerun passes 15,562 assertions;
the preceding overlapping native run's focus/menu failures remain documented.
All platform/Graph/local/package/fixture sources outside the bounded opening-shell
files are preserved. Real local SQLite was never opened.

## Historical CXT4a approval handoff

Updated 2026-09-13. **CXT4a onboarding/security contract and Chordrift reference
audit are approved and complete.** Read the
[approved implementation contract](architecture/CXT4A_ONBOARDING_SECURITY_CONTRACT.md).
This documentation-only slice starts from clean `main` at `c90274b` and remains
uncommitted/unpushed. The contract does not mark authentication or deployment done.

It specifies native PKCE, exact token verification, Keychain/refresh/logout,
bootstrap DTOs and transactions, additive persistence prerequisites, same-ID local
Library reconciliation, returning-user/collision handling, threat model and tests.
The user approved Google as the only initial Auth0 connection, the
`Sign in with Google` action, device credential, token policy, additive migration/
floor approach and empty-Library enrollment limit. Future providers such as Yahoo
remain additive and require explicit identity linking rather than email matching.
Exact Auth0 coordinates, bundle/signing inputs and host/secret store are concrete
CXT4b inputs, not unresolved CXT4a architecture.
Existing CXT3b startup only accepts local-only authority; CXT3c has no durable
Account default pointer or pre-Account bootstrap receipt. Those require bounded
CXT4b/d changes, not manual seeding or edits to deployed migration checksums.

**Historical gate:** separately select CXT4b and its concrete deployment inputs. The already-approved
[CXT4c native opening-shell boundary](architecture/CXT4_ONBOARDING_AND_OPENING.md#cxt4c--native-opening-library-shell)
is preserved intact and remains before real native Auth0 integration. No production
Rust/Swift/service code, provider state, credentials or database was changed.

**Verification before approval:** naming guard passes 362 files; schema guard passes 46 signatures
and 104 scoped FKs; fixture guard passes 12 canonical containers and 92 embedded
records; `git diff --check` passes. See the contract's exact six-file inventory
and preservation/verification limits. Runtime acceptance tests remain future work.

## Historical CXT3d handoff

Updated 2026-09-13. **CXT3d Neon Generation Two schema activation is complete.**
CXT3c was committed and pushed as `be1ed80`; CXT3d was committed and pushed as
`699395f`. Read the
[live topology, inventory, and onboarding boundary](architecture/CXT3D_NEON_ACTIVATION.md).

The existing Neon `photara` project was retained. Its empty primary/default
`production` branch is now `main`; populated `development` is now
`legacy-v0.1.x`. The latter remains unchanged with 34 public tables and its
20-entry legacy ledger. On `main`, four non-login capability roles and the exact
13 checked migrations are installed. The live inventory is 55 domain tables,
569 columns, 153 indexes, 115 triggers, 48 functions and 173 policies; all 45
RLS tables force RLS. All migration checksums match CXT3c, and a second migration
run accepted the deployed ledger. Accounts, Libraries, and Projects are all empty.

**Historical next step, now prepared above:** CXT4a, the Astra-led onboarding/security
contract and Chordrift Auth0 reference audit. Follow the exact bounded sequence in
[CXT4 onboarding and opening Library shell](architecture/CXT4_ONBOARDING_AND_OPENING.md):
CXT4a contract → CXT4b minimal cloud service → CXT4c native opening Library shell
→ CXT4d native Auth0 integration → CXT4e first-Account acceptance. Stop before
each gate. Do not manually seed around the product experience. No runtime login,
password, or service secret exists, and no privileged Neon credential may enter
the desktop app.

## Historical CXT3c handoff

Updated 2026-09-13. **CXT3c disposable PostgreSQL/service proof is complete.**
Started from clean synchronized `7e6d794`; committed and pushed as `be1ed80`.
Read [service routes, limits and the next Neon plan](architecture/CXT3C_SERVICE_RUNTIME.md),
[implementation and regression inventory](architecture/CXT3C_IMPLEMENTATION_INVENTORY.md)
and [measured PostgreSQL objects/checksums](architecture/CXT3C_POSTGRES_INVENTORY.json).

The new `photara-service` crate owns typed service controllers above Storexa,
13 executable migrations, separated unprivileged pools and fake identity/media/
sync ports. PostgreSQL 18.6 executed 697 migration statements: 55 domain tables,
569 columns, 153 indexes, 115 triggers, 48 functions and 173 RLS policies.
Five real PostgreSQL suites pass, including 678 access/privilege/sensitivity cells,
CAS/concurrency, last-manager and last-identity guards, rollback, checksums/floors,
pooled GUC cleanup, invitations, scoped receipts/media and lost-response recovery.
Every disposable PostgreSQL cluster is stopped; no service was deployed.

The full offline Rust suite passes 258 tests. Its nine ignored tests comprise
four existing fixture-generation checks plus the five PostgreSQL suites that the
explicit disposable runner executes successfully. All-target check, strict Clippy,
fmt, schema/naming/fixture guards, bridge and production UI pass. Graph Lab's
initial randomized prerequisite failure (15,050 assertions, one failure) is retained;
exact seed replay passed 548 assertions, then the full rerun passed 15,586/zero.
All 42 Graph/Graph Lab source files and all UI/Core/SDK/package/local runtime
sources remain byte-for-byte unchanged. The real CXT3b database still matches its
starting SHA-256 and size, 1,179,648 bytes; it was not opened or migrated here.

The authorized slice exercises online transport with an in-memory fake. It does
not activate the native SQLite online worker. Its closed content codecs expose
Library storage and per-Project catalog projections, not a complete backup of
all installed tables. Real identity/media adapters, native online activation and
package publication are not fabricated by this proof.

The subsequent CXT3d activation is recorded above. CXT3c itself did not contact
Neon or activate real Auth0/CloudKit/media adapters.

## Historical CXT3b handoff

Updated 2026-09-12. **CXT3b local runtime and app initialization are complete.**
Started from clean `415039e`; this slice is uncommitted and unpushed. Stop here.
Read [runtime/API boundaries and inspection commands](architecture/CXT3B_LOCAL_RUNTIME.md)
and [exact file/migration inventory](architecture/CXT3B_LOCAL_INVENTORY.md).

The real app startup entry point created/opened:
`/Users/suhail/Library/Application Support/Photara/State/photara-local-v2.sqlite`.
It contains one active `My Library`, explicit local controller, stable device ID,
12 migrations, reader/writer floor 2 and 70 domain tables. Read-only integrity
returns `ok`; foreign-key checks return no violations. No unknown/preexisting
user database was adopted or deleted.

CXT3b includes explicit local membership, restricted Project registration/access,
policy/grant CAS with immutable control audit, atomic last-manager/controller
transfer, classified storage/slots/host binding selection, checked Library
variables/expressions, device observations, context apply/recovery, Project-scoped
media and offline scoped variable intents. Project variables remain package-owned.
Online channel activation/sealing/dispatch/service snapshot installation stay in
CXT3c; package publication and publication receipts stay in L3. No PostgreSQL,
Neon, Auth0, CloudKit, v0.1.3 database or SMB/user source storage was written.

**Verification:** 258 offline Rust tests pass, four intentionally ignored;
all-target compilation, Clippy with warnings denied, fmt and whitespace pass.
Schema: 70 tables / 68 explicit indexes / 169 triggers / 310 statements. All six
baseline migration bodies, all 12 fixtures and all 42 Graph/Graph Lab files are
byte-for-byte preserved. Static checks cover 46 proposal signatures and 104 FKs;
fixture verification covers 12 canonical containers and 92 embedded records.
Bridge passes (revision 19; 3 progress / 2 cancellation callbacks). Shared UI
passes with 98 snapshots, production UI passes with 10 snapshots and explicit
startup assertions. Representative Light/Dark production images were inspected.

Dedicated Graph Lab: the first 15,430-assertion run had one randomized prerequisite
routing-knot failure at seed 20260909 dark/curved. Isolated replay passed 541
assertions; the full rerun passed **15,586 assertions / zero failures**. Graph source
was not edited to obtain the pass. Logs: `/private/tmp/photara-cxt3b-graph.log`,
`/private/tmp/photara-cxt3b-graph-replay.log`,
`/private/tmp/photara-cxt3b-graph-full-retry.log`.

The next gate is separately authorized CXT3c disposable PostgreSQL/service work.
No service deployment, fresh Neon environment, package publisher, commit or push
is authorized by this completion. Continue to preserve the Graph implementation.

## Historical CXT3a/rebaseline handoff

Updated 2026-09-12. **CXT3a and the separately authorized Library nomenclature
rebaseline are complete and verified.** Nothing is committed or pushed.

Read the [CXT3a reader/API record](architecture/CXT3A_PACKAGE_READER.md),
[rebaseline authority/evidence](architecture/LIBRARY_NOMENCLATURE_REBASELINE.md)
and [exact file/hash inventory](architecture/LIBRARY_REBASELINE_INVENTORY.md).
The [execution roadmap](ROADMAP_0_2_EXECUTION.md) is the current gate order.

Suhail superseded D19 R1 physical-name preservation: unshipped Generation Two uses
`Library`, `LibraryId`, `libraries`, `library_id` throughout Rust, package, SQLite,
PostgreSQL proposals and service vocabulary. There are no rename migrations,
aliases, shadow columns, dual writes or runtime naming adapters. The old v0.1.3
and live databases are untouched. An optional future historical importer is
non-gating; it has not been implemented.

**Verification:** 245 full offline Rust tests pass; all-target compilation, Clippy,
formatting and whitespace checks pass. Fresh SQLite baseline: 44 tables, 30 explicit
indexes, 95 triggers, 171 statements; integrity and FK checks pass. S3/S4 documentation
totals include examples (179/275); PostgreSQL baseline DDL is 263 statements and was
never executed. D19 static proposals retain 26/20 new tables, 139/431 statements,
46 checked signatures and 104 FKs. All 12 canonical fixture containers and 92 embedded
byte records pass Rust codec/hash verification. Naming checks, Swift/Rust bridge,
98 shared UI snapshots and 10 production snapshots pass; representative Light/Dark
screens were visually checked.

## Authority and exact next task

Library/account/catalog/access/device-binding records belong in local/cloud
databases. Authored Project graphs, nodes, connections, configuration, Work Surface
state, variables and asset/resource ledgers belong in `.photara`, with the designed
catalog/sync projections. Immutable runs/evidence follow package/cloud projection
rules. UI tokens are code/presets. Window geometry, panes, selection, zoom, scroll,
transient progress and unsaved edits are per-device preferences/session state.
The shell now calls that model `EditorSessionModel`; it is not Library data.

**Next separately authorize CXT3b:** use the clean Library baseline and accepted
D19 proposals to implement executable local SQLite repositories/facade and real
Generation Two app initialization. Prove fresh disposable migrations, floor refusal,
rollback/FKs, scoped permission/CAS behavior and local recovery with fake hosts.
Initialize local My Library explicitly; do not route through a legacy importer.
The current app's existing Library/UI regression is not a claim that D19 app
initialization or the new repositories are implemented.

Then CXT3c proves disposable PostgreSQL/RLS and scoped service behavior. Fresh
Generation Two Neon deployment requires its own authorization after those checks.
The minimum usable Project/UI/node vertical slice follows that deployment gate.
No CXT3b implementation, D19 migration execution, PostgreSQL/Neon execution,
service deployment, live database change, staging, commit or push occurred here.

## Historical CXT1b handoff — superseded next-step labels


Updated 2026-09-12. **CXT1b pure Rust context contracts are complete.** Suhail
separately selected this slice after CXT1a. Read the [implementation/API/grammar/
verification record](architecture/CXT1B_CONTEXT_CONTRACTS.md), then the
[accepted freeze](architecture/D19_CONTRACT_FREEZE.md) and
[bounded gates](architecture/D19_STATIC_SCHEMA_DELTA.md#fixture-delta-implementation-slices-and-gates).

The additive `photara_core::context` API implements explicit literal/expression/
template admission, bounded parser and ID-bound typed AST/interpreter, variable
CAS/precedence/cycles, immutable consent-checked captures, bounded metadata queries,
Run overrides, private device/frozen contexts, cache v2 and proposal/receipt planning.
Literal-only fields never become expression records; fenced containers remain
unsupported. Evaluated results retain privacy labels; no existing graph evaluator,
NodeSDK validator, package codec, database/host adapter or effect executor is changed.

**Verification passed offline: 159 tests**, comprising 57 new context integration
tests, three new serialization compile-fail tests and 99 retained CXT1a/Core/SDK/node
tests. New coverage includes a 441-case deterministic arithmetic matrix. The separate
[d19-context.json](fixtures/generation-two/d19-context.json) was generated and checked
by the unchanged Rust canonical encoder; previous fixture bytes remain exact.
Affected formatting, all-target library compilation, Clippy with `-D warnings`,
whitespace, Markdown links and pre-slice hash checks pass. Database suites were not
executed; retained node tests use disposable filesystem fixtures.

CXT1a source/tests/implementation record, six applied L2 migrations, all 14 CXT2
proposal files, ten pre-existing fixture files, Cargo files and unrelated dirty work
are preserved. The hash audit confirms 317 pre-existing files are byte-identical;
232 local Markdown links/anchors pass. Only the additive context module declaration
changes existing Rust.
No database/SQL, resolver, package reader/writer, Swift/UI, user Project/SMB, service,
staging/commit/push or release work occurred.

**Next eligible slice: separately select CXT3a.** Its package 1.1 reader/closure and
explicit compatibility mapping DTOs belong in disposable roots with no publisher.
CXT3b/c retain separate local/fake-host and service/RLS/sync proof gates. L3 remains
paused. Stop at CXT1b; do not automatically continue into those slices.

## CXT1b exact file inventory

New source files:

- `crates/photara-core/src/context/mod.rs`
- `crates/photara-core/src/context/value.rs`
- `crates/photara-core/src/context/expression.rs`
- `crates/photara-core/src/context/parser.rs`
- `crates/photara-core/src/context/interpreter.rs`
- `crates/photara-core/src/context/variable.rs`
- `crates/photara-core/src/context/snapshot.rs`
- `crates/photara-core/src/context/metadata.rs`
- `crates/photara-core/src/context/cache.rs`
- `crates/photara-core/src/context/proposal.rs`
- `crates/photara-core/tests/context.rs`

New documentation/data: `docs/architecture/CXT1B_CONTEXT_CONTRACTS.md` and
`docs/fixtures/generation-two/d19-context.json`.
Existing Rust edit: `crates/photara-core/src/lib.rs`, additive module declaration only.
Updated documentation: `README.md`, `ROADMAP.md`, `docs/ACTIVE_HANDOFF.md`,
`docs/ROADMAP_0_2_EXECUTION.md`, and architecture `README.md`, `CORE.md`, `ASSETS.md`,
`NODE_PACKAGES.md`, `STORAGE_LOCATIONS_AND_HOST_BINDINGS.md`,
`TYPED_CONTEXT_AND_EXPRESSIONS.md`, `GENERATION_TWO_FIXTURES.md`,
`D19_CONTRACT_FREEZE.md`, `D19_STATIC_SCHEMA_DELTA.md`, `SCHEMA_REVIEW.md`.
This slice adds 13 files and updates 15 pre-existing files. Earlier dirty-tree
changes in git status are not part of this inventory.

## Prior CXT1a completion — historical scope

The following records earlier completed slices. Their then-next-step statements
are historical; the CXT1b status and CXT3a gate above govern current work.

Updated 2026-09-12. **CXT1a pure Rust contracts are complete.** Suhail separately
selected this bounded implementation after accepting R1–R8 and CXT2. Read the
[implementation/API/verification record](architecture/CXT1A_CONTRACTS.md), then the
[accepted freeze](architecture/D19_CONTRACT_FREEZE.md) and
[bounded gates](architecture/D19_STATIC_SCHEMA_DELTA.md#fixture-delta-implementation-slices-and-gates).

Additive `photara_core::contracts` and `photara_node_sdk::v2` implement portable
IDs/adapters, Project access, resource rights/coordinates and nonserializable live
handles, immutable AssetSet v2 snapshots/pages/digests, complete manifest v2
validation and minimal response/opaque context coordinates. The one new
[d19-contracts.json](fixtures/generation-two/d19-contracts.json) is generated and
verified with the existing Rust canonical encoder in the reserved synthetic namespace.
No parser, expression evaluation, capture engine, cache v2 implementation,
package 1.1 codec or host/database adapter is included.

**Verification passed offline: 99 tests** (58 new integration, two compile-fail,
39 retained Core/SDK/node tests), plus explicit golden generation and the final
SDK rerun. Affected-crate formatting, whole-library all-target compilation and
Clippy with `-D warnings` pass. Database suites were not executed. The pre-slice
SHA-256 inventory confirms 302 pre-existing files remain byte-identical and preserves existing dirty work, v1 source/API/cache behavior,
Cargo files, all six L2 migrations, all 14 CXT2 proposal files and all nine
pre-existing fixture files. No database/service, user Project/SMB, UI,
staging/commit/push or release action occurred.

**Next eligible slice: separately select CXT1b.** Its bounded parser/AST/types,
dependencies/snapshots/cache/proposals remain unstarted. CXT3a/b/c retain their
separate package/local/service proof gates. L3 remains paused. Stop at CXT1a;
there is no automatic migration, package publication or live-service continuation.

## CXT1a exact file inventory

New Rust files:

- `crates/photara-core/src/contracts/{mod,ids,schema,access,resource,asset_set,dto}.rs`
- `crates/photara-core/tests/contracts.rs`
- `crates/photara-node-sdk/src/v2/{mod,types,validation}.rs`
- `crates/photara-node-sdk/tests/contracts_v2.rs`

Existing Rust changes: only additive module declarations in
`crates/photara-core/src/lib.rs` and `crates/photara-node-sdk/src/lib.rs`.
New documents/data: `docs/architecture/CXT1A_CONTRACTS.md` and
`docs/fixtures/generation-two/d19-contracts.json`.
Updated documentation: `README.md`, `ROADMAP.md`, `docs/ACTIVE_HANDOFF.md`,
`docs/ROADMAP_0_2_EXECUTION.md`, and architecture `README.md`, `CORE.md`, `ASSETS.md`,
`NODE_PACKAGES.md`, `STORAGE_LOCATIONS_AND_HOST_BINDINGS.md`,
`TYPED_CONTEXT_AND_EXPRESSIONS.md`, `GENERATION_TWO_FIXTURES.md`,
`D19_CONTRACT_FREEZE.md`, `D19_STATIC_SCHEMA_DELTA.md`, `SCHEMA_REVIEW.md`.
This slice adds 14 files and updates 16 pre-existing files; earlier dirty-tree
changes shown by git status are not part of this inventory.

## Prior CXT2 acceptance — historical scope

The following records the completed CXT2 slice before CXT1a was selected. Its
then-next-step and no-source-change statements are historical evidence only.

Updated 2026-09-12. **Suhail accepted R1–R8 as proposed; CXT2 acceptance is complete.**
Read the [accepted contract freeze](architecture/D19_CONTRACT_FREEZE.md),
[static schema delta](architecture/D19_STATIC_SCHEMA_DELTA.md), and
[inert DDL/inventory](architecture/proposals/d19-cxt2/README.md).
The primary review found no blocking inconsistency; approval was explicitly
recorded for all eight decisions on 2026-09-12. Only CXT2 acceptance was selected.

CXT2 translated the approved signatures into eleven `.proposal.sql` files outside
runtime migration directories: SQLite 0007–0012 and PostgreSQL 0008–0012. The
service's unexecuted 0007 privileges reservation is preserved. Added relations
remain **26 local (44 → 70)** and **20 service (35 → 55)**, plus four service
upload-session columns. These are proposed counts, not installed schemas.

The proposal includes concrete constraints, FK indexes, retention/CAS/lifecycle
guards, service access helpers and replacement RLS/grants. Its
[responsibility ledger](architecture/proposals/d19-cxt2/RESPONSIBILITIES.md) names
exact Rust/repository/controller checks that SQL cannot prove, including local
commit-level last-manager checks, access-generation aggregation, canonical typed
bytes, consent, scoped sync installs and package evidence.

**Next eligible slice: separately select CXT1a pure Rust**—IDs, access masks,
portable resource contracts, AssetSet v2 and complete manifest v2 validation.
CXT1b and CXT3a/b/c follow their own bounded gates. CXT1/CXT3/L3 have not begun;
Neon or any live service is not the next step. L3 remains paused.

No runtime/UI/source, applied/local migration, baseline service SQL, fixture
byte/hash, manifest or package specimen changed. No database, service, user
Project or SMB storage was opened. No staging, commit or push occurred. Existing
dirty-tree changes remain intact. The prior documentation-only review evidence
below is retained as history, followed by this acceptance slice's verification.

**S7 D1–D17 approved; bounded L1 and L2 complete.** After L1,
the user explicitly authorized L2 local SQLite implementation and fresh disposable
database tests. No existing user database, service, SMB storage, UI or live Project
was opened/changed. Publication/locks, cloud/auth, staging, commits and release
remain outside this authorization.

## Resume here

Canonical repository:
`/Users/suhail/Library/CloudStorage/Dropbox/matrix/crates/photara`.
Verify the path/branch/status before editing: the task environment may open another
worktree. Read in order:

1. This handoff and [execution roadmap](ROADMAP_0_2_EXECUTION.md), then
   [D19](architecture/LIBRARY_AND_NODE_WORK_SURFACES.md) and
   [revised D18](architecture/TYPED_CONTEXT_AND_EXPRESSIONS.md), followed by
   [storage locations and host bindings](architecture/STORAGE_LOCATIONS_AND_HOST_BINDINGS.md),
   then [exact R1–R8 contracts](architecture/D19_CONTRACT_FREEZE.md) and
   [static delta](architecture/D19_STATIC_SCHEMA_DELTA.md).
2. [Approved product architecture](architecture/GENERATION_TWO_PRODUCT_ARCHITECTURE.md)
   and [S7 decision record](architecture/SCHEMA_REVIEW.md).
3. [L1 implementation boundary](architecture/PROJECT_PACKAGE_CODEC.md).
   Then [L2 implementation boundary](architecture/LOCAL_LIBRARY_IMPLEMENTATION.md).
4. [Logical model](architecture/LOGICAL_DATA_MODEL.md),
   [package format](architecture/PROJECT_PACKAGE_SCHEMA.md),
   [local SQLite](architecture/LOCAL_SQLITE_SCHEMA.md),
   [service PostgreSQL](architecture/SERVICE_POSTGRESQL_SCHEMA.md),
   [synchronization](architecture/SYNCHRONIZATION_CONTRACT.md),
   [fixtures](architecture/GENERATION_TWO_FIXTURES.md), and
   [social profiles/export](architecture/SOCIAL_PROFILES_AND_LIBRARY_EXPORT.md).
5. [Storexa integration](architecture/STOREXA_INTEGRATION.md),
   [current project documents](architecture/PROJECT_DOCUMENTS.md),
   [persistence](architecture/PERSISTENCE.md), and
   [Library architecture](LIBRARY_ARCHITECTURE.md).
6. [Core](architecture/CORE.md), [assets](architecture/ASSETS.md),
   [Node packages](architecture/NODE_PACKAGES.md), and
   [native clients](architecture/NATIVE_CLIENTS.md). For presentation work read
   [Shared UI](../platform/macos/SHARED_UI.md) and
   [design language](../platform/macos/DESIGN_LANGUAGE.md).

[ROADMAP](../ROADMAP.md), [CODEX_HANDOFF](CODEX_HANDOFF.md) and
[FRAME_LIBRARY_HANDOFF](FRAME_LIBRARY_HANDOFF.md) retain historical/operator
context; their older next-work sections do not override this scope.

## Locked decisions

- Native macOS presentation over portable Rust; Graph authoring composition hosts modular
  Inspector and optional Node Work Surfaces. UI implementation still requires
  raster mockup approval; schema approval is not UI approval.
- General versioned Node packages own behavior. No privileged Layout/provider
  cases in Core/bridge. Layout and Gallery are D19's proposed built-ins; LrC/Lr/Ps are
  planned free first-party downloads. Node Store commerce remains future work.
- Stable identity is independent of paths. Package authority owns authored
  Project metadata, typed Library snapshots/assignments, a private graph/run asset
  identity/provenance/artifact ledger, multiple named Graphs and immutable run/attempt/effect/evidence records.
  Catalogs are projections, never a second source of package authority.
- Library Storage Locations are portable logical identities; per-device Host
  Bindings resolve them to macOS, Windows, Linux or provider resources. Packages
  and synchronized Library data contain no absolute paths, bookmarks or secrets.
  External sources, managed Project resources, external artifacts and disposable
  cache are distinct storage classes. Variables resolve typed handles and never
  hide AssetSet membership or workflow dataflow.
- Library is the durable catalog/collaboration boundary, local-first and typed: People with multiple roles and
  relationships, client Organizations, unique LocationKinds and concrete
  Locations with a required kind. Account/Auth0 identity is never a Person.
  Scene is not a separate target domain record; a friendly UI label remains a
  presentation choice. Current Scene code is transitional, not silently removed.
- People/Locations/Project catalog management belongs to the app/Library. Node
  Work Surfaces embed host-owned pickers/Browsers under declared permissions;
  inline creation uses Library commands. Authoring visibility does not grant runtime
  access. Project-only collaborators receive bounded assigned snapshots. Package/SMB
  access remains separate device authorization.
- Graph inputs are explicit typed ports plus declared frozen context. Read/source,
  enrichment and effect/output are distinct; MetadataPatch never mutates originals
  by itself. Stable hierarchical categories/tags are discovery-only metadata.
- Kind claims use the approved Unicode 16.0.0 policy and explicit Beach/beaches
  seed groups. Narrow atomic merge/claim transfer preserves Library/key
  uniqueness. The full normalizer/merge runtime is not implemented by L1.
- Auth0/API/Neon is the cloud trust boundary; desktop has no privileged Neon
  credentials. CloudKit is a deferred explicit record-sync adapter, not SQL.
- Storexa 0.2.0 is published from sibling commit `22c4270`, with explicit SQLite
  and PostgreSQL SQLx types. Photara retains entities, SQL/migrations, package
  publication, authorization and sync policy. L1 uses no database adapter.
- D16 reserves typed manual-first social profiles. Stable bound subjects are
  Library-unique including tombstones; no ordinary reassignment or identity
  proof. Mutable/reusable handles never silently merge owners. Provider adapters,
  Instagram lookup and consent/expiry-aware fetched avatars remain optional.
- D17 reserves future logical checksummed/optionally encrypted Library export:
  separate Project backups, safe root hints/rebind, dry-run/isolated restore,
  no credentials, device IDs, absolute paths, bookmarks or sync/recovery state.
  Export/import is non-gating and unimplemented.
- Clean generation two only. Existing Neon and `v0.1.3` are optional reference/
  salvage. Legacy import is never a schema, implementation or release gate.

## Completed bounded L1

Additive `photara-store::package` provides strict raw JSON validation before
Value construction, canonical-json.v1 checking, versioned typed control/authored/
Graph envelopes, safe descriptor-relative no-follow reads, managed-byte hashing,
HEAD/parent/bootstrap/inventory closure and focused typed authored/history checks.
Original canonical bytes and optional unknown data are retained. No writer API
exists. Current one-JSON ProjectDocument import/export and Core are unchanged.

The S6 33-file inert archive is materialized only in fresh temporary roots.
All 25 focused tests and 29 retained Core/NodeSDK/store tests passed; selected doc
tests passed. Store all-target Clippy with warnings denied and whole-library
all-target compilation passed offline. No database, service, user Project or SMB
storage was opened. Full Unicode graph-name uniqueness, embedded manifest support,
application integration, conversion, publication and recovery remain outside this
bounded reader. See [precise limitations](architecture/PROJECT_PACKAGE_CODEC.md).

## Completed L2 and next exact slice

`photara-library::gen2::LocalLibraryStore` now implements the separate family and
six ordered migrations (44 tables), typed Library CRUD/CAS/tombstones, exact
Unicode-16 Kind claims, local changes and catalog/device/verified-observation
foundations. Storexa 0.2/SQLx 0.9 is adopted privately; existing v1 APIs are intact.
The necessary rusqlite compatibility pin is 0.39.0/libsqlite3-sys 0.37.0. All five
retained Library tests and 16 new L2 tests pass; selected regression total is 75,
with doc tests, library Clippy `-D warnings` and whole-library compile passing.
See [L2 scope and limits](architecture/LOCAL_LIBRARY_IMPLEMENTATION.md).

**Next: separately select CXT3a.** R1–R8, CXT2 inert DDL and CXT1a/b pure contracts
are complete. CXT3a package readers, CXT3b disposable local migrations/repositories
and CXT3c disposable service/RLS/fake transport each require their own scope. L3 waits for accepted
contracts and required conformance. See the companion's exact slices and gates.
S2–S6 remain baseline evidence: S3 is 44 tables/S4 35; all six L2 migration
checksums and pre-existing inert fixture bytes are unchanged. The proposed totals are neither
installed schema counts nor runtime test evidence.
Resolve remaining L1
writer-readiness gaps before publishing any package. L2b Kind merge/claim transfer
remains unexposed: its exact affected-root/CAS/rebind/retirement/promotion proof must
pass before merge or cloud reconciliation is offered. No automatic L3, SMB/user
storage, cloud, UI, staging/commit or release authorization is implied.

## Design evidence still awaiting runtime proof

S3's 44 SQLite tables/179 statements now install through L2's six migrations;
selected local FK/trigger/concurrency tests pass. S4 has 35 PostgreSQL tables/275
statements, RLS and service functions and remains unexecuted. Full merge/sync,
service privilege/concurrency and publication tests remain future work.
S5 defines sealed commands/receipts, offline chains, ordered feeds, bounded reset,
conflict/rebase, media staging and privacy; no API/Auth0/object-store was contacted.
S6 has 51 scenario specifications and 12 crash points; passing L1 does not mark
their database/service/SMB/import families passed. S2's publication and recovery
protocol is approved design, not an fsync/rename/SMB guarantee.

## Working tree warning

The canonical tree already contains substantial uncommitted docs, shell/library
presentation, Inspector, lab, build configuration and UI verification work.
These belong to the user/ongoing work and were preserved. The planning docs and
L1 sources are also uncommitted. Inspect live status; do not reset, restore, clean,
stage or commit unrelated changes. Storexa's separate release is already complete.

## Task and model allocation

Use Astra with high reasoning for architecture/heavy Rust/persistence and Sol for
bounded UI/module slices after contracts and scope approval. This does not itself
authorize spawning tasks/agents. Give each slice a concrete boundary and gate.

## Completion checklist

- [x] Architecture and S1 logical model accepted; Storexa 0.2.0 published.
- [x] S2–S6 designs, inert fixtures and static SQL/JSON/hash/link checks prepared.
- [x] S7 D1–D17 approval recorded on 2026-09-11.
- [x] Bounded L1 read-only implementation/test scope selected and passed.
- [x] Actual changes, limits and next gate recorded.
- [x] L2 database scope separately authorized; implementation and temporary tests passed.
- [x] D18 concept inclusion and source syntax requested; documentation/inert addendum prepared.
- [x] D19 conceptual direction approved and documentation amendment prepared.
- [x] D19 consistency reviewed; exact logical/package/NodeSDK freeze candidate prepared.
- [x] Static physical/package/sync/DTO/fixture delta specified as inert documentation.
- [x] Suhail accepted R1–R8 as proposed, 2026-09-12; CXT2 acceptance only selected.
- [x] CXT2 inert SQL, exact inventory and static checks complete; no execution.
- [x] CXT1a separately selected and completed; pure contracts and golden fixture verified.
- [x] CXT1b separately selected and complete; pure context/golden/regression verified.
- [ ] CXT3a separately selected, followed by CXT3b/c proof before L3.
- [ ] Remaining migration/service/publication/recovery scenarios pass before
  their corresponding release claims.

## CXT2 acceptance verification — historical, 2026-09-12

This acceptance slice adds **14 files** (11 inert SQL proposals and three proposal
README/inventory/responsibility documents) and updates **27 Markdown files**.
The exact paths are listed below; the earlier review's delta is separate history.

| Proposal | Statements | New tables | Explicit indexes | Triggers | Functions | New policies | Old policies removed |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| SQLite 0007–0012 | 139 | 26 | 38 | 74 | 0 | 0 | 0 |
| PostgreSQL 0008–0012 | 431 | 20 | 33 | 42 | 27 | 173 | 25 |

The service total includes 44 ALTER TABLE statements (40 enable/force-RLS, three
forward-reference constraints and the upload-column extension), privilege changes
and the inert API-floor update. Local floor changes likewise remain inert text.
No generated proposal is referenced by a runtime migration runner.

Checks passed: **46 exact relation-column/nullability signatures**, **104 scoped
FK target/key/index checks**, lexical SQL statement/delimiter/quote checks,
identifier uniqueness/length checks, explicit replacement of all 25 old policies,
enable/force RLS on all 20 service additions, **319 local Markdown links/anchors**,
changed-file whitespace and `git diff --check`. These checks read SQL as text;
no database, interpreter, compiler or service was started. No installed pglast,
sqlglot or sqlparse was available in the checked Python environments; grammar,
SQL type/name resolution and runtime behavior remain unverified CXT3 gates.

All **236 pre-existing non-Markdown files** retain their pre-slice SHA-256.
All six applied/local migration files, all nine fixture-directory files and the
original complete S2/S3/S4/S5 baseline bodies remain byte-identical. The inventory
records protected-file hashes. Existing SQLx checksums, package specimen, fixture
hashes, runtime/Rust/Swift/UI, manifests and unrelated dirty files were preserved.

SQL cannot by itself prove local commit-level manager/aggregate invariants,
expected-revision intent, canonical/typed/privacy agreement, complete controller
lock ordering, bootstrap/cursor correctness, identity/token checks or package
publication evidence. The responsibility ledger states the exact future checks.
No CXT1a implementation, CXT3 database work or L3 publication was begun. No staging,
commit or push occurred. **Next: separately select CXT1a pure Rust, not Neon.**

Exact changed files for this acceptance slice:

```text
README.md
ROADMAP.md
docs/ACTIVE_HANDOFF.md
docs/ROADMAP_0_2_EXECUTION.md
docs/architecture/CORE.md
docs/architecture/D19_CONTRACT_FREEZE.md
docs/architecture/D19_STATIC_SCHEMA_DELTA.md
docs/architecture/GENERATION_TWO_FIXTURES.md
docs/architecture/GENERATION_TWO_PRODUCT_ARCHITECTURE.md
docs/architecture/LIBRARY_AND_NODE_WORK_SURFACES.md
docs/architecture/LOCAL_LIBRARY_IMPLEMENTATION.md
docs/architecture/LOCAL_SQLITE_SCHEMA.md
docs/architecture/LOGICAL_DATA_MODEL.md
docs/architecture/NATIVE_CLIENTS.md
docs/architecture/NODE_PACKAGES.md
docs/architecture/PERSISTENCE.md
docs/architecture/PROJECT_DOCUMENTS.md
docs/architecture/PROJECT_PACKAGE_CODEC.md
docs/architecture/PROJECT_PACKAGE_SCHEMA.md
docs/architecture/README.md
docs/architecture/SCHEMA_REVIEW.md
docs/architecture/SERVICE_POSTGRESQL_SCHEMA.md
docs/architecture/SOCIAL_PROFILES_AND_LIBRARY_EXPORT.md
docs/architecture/STORAGE_LOCATIONS_AND_HOST_BINDINGS.md
docs/architecture/STOREXA_INTEGRATION.md
docs/architecture/SYNCHRONIZATION_CONTRACT.md
docs/architecture/TYPED_CONTEXT_AND_EXPRESSIONS.md
docs/architecture/proposals/d19-cxt2/INVENTORY.md
docs/architecture/proposals/d19-cxt2/README.md
docs/architecture/proposals/d19-cxt2/RESPONSIBILITIES.md
docs/architecture/proposals/d19-cxt2/postgresql/0008_library_project_access.proposal.sql
docs/architecture/proposals/d19-cxt2/postgresql/0009_storage_locations.proposal.sql
docs/architecture/proposals/d19-cxt2/postgresql/0010_library_context.proposal.sql
docs/architecture/proposals/d19-cxt2/postgresql/0011_scoped_sync.proposal.sql
docs/architecture/proposals/d19-cxt2/postgresql/0012_d19_access_guards.proposal.sql
docs/architecture/proposals/d19-cxt2/sqlite/0007_library_project_access.proposal.sql
docs/architecture/proposals/d19-cxt2/sqlite/0008_storage_locations_bindings.proposal.sql
docs/architecture/proposals/d19-cxt2/sqlite/0009_library_context.proposal.sql
docs/architecture/proposals/d19-cxt2/sqlite/0010_context_apply_recovery.proposal.sql
docs/architecture/proposals/d19-cxt2/sqlite/0011_scoped_sync.proposal.sql
docs/architecture/proposals/d19-cxt2/sqlite/0012_d19_guards_and_floor.proposal.sql
```

## Prior documentation review verification — 2026-09-12

Passed for this slice: **292 local Markdown links/anchors**, all changed-file
whitespace including untracked Markdown, Project action-mask arithmetic, exact
relation inventories (14 common + 12 local-only; 14 common + 6 service-only),
26 proposed package schema rows, eight unchanged fixture JSON parses, and
`git diff --check`. The six migration files and all nine fixture-directory files
(including README) are byte-identical to the task-start baseline. Hash comparison
also proves all **236 existing non-Markdown files** unchanged. Reconstructing the
pre-notice S2/S3/S4/S5 documents matches their original full-file hashes, so their
SQL, JSON and protocol baseline text is preserved exactly.

No new SQL grammar/runtime/RLS/codec/interpreter test was run: the proposal uses
inert relation signatures, not executable DDL. No Rust build or database test was
needed or authorized for this documentation-only slice. The commands below are
historical L1/L2 checks, not new runtime evidence.

Exact task delta: **two new review documents and 26 updated Markdown files**.
New files: `docs/architecture/D19_CONTRACT_FREEZE.md` and
`docs/architecture/D19_STATIC_SCHEMA_DELTA.md`. Updated files:

```text
README.md
ROADMAP.md
docs/ACTIVE_HANDOFF.md
docs/ROADMAP_0_2_EXECUTION.md
docs/architecture/ASSETS.md
docs/architecture/CORE.md
docs/architecture/GENERATION_TWO_FIXTURES.md
docs/architecture/GENERATION_TWO_PRODUCT_ARCHITECTURE.md
docs/architecture/LIBRARY_AND_NODE_WORK_SURFACES.md
docs/architecture/LOCAL_LIBRARY_IMPLEMENTATION.md
docs/architecture/LOCAL_SQLITE_SCHEMA.md
docs/architecture/LOGICAL_DATA_MODEL.md
docs/architecture/NATIVE_CLIENTS.md
docs/architecture/NODE_PACKAGES.md
docs/architecture/PERSISTENCE.md
docs/architecture/PROJECT_DOCUMENTS.md
docs/architecture/PROJECT_PACKAGE_CODEC.md
docs/architecture/PROJECT_PACKAGE_SCHEMA.md
docs/architecture/README.md
docs/architecture/SCHEMA_REVIEW.md
docs/architecture/SERVICE_POSTGRESQL_SCHEMA.md
docs/architecture/SOCIAL_PROFILES_AND_LIBRARY_EXPORT.md
docs/architecture/STORAGE_LOCATIONS_AND_HOST_BINDINGS.md
docs/architecture/STOREXA_INTEGRATION.md
docs/architecture/SYNCHRONIZATION_CONTRACT.md
docs/architecture/TYPED_CONTEXT_AND_EXPRESSIONS.md
```

Canonical pages now point to exact decisions instead of leaving the freeze
unspecified; older historical baseline text remains labeled and preserved.
Unrelated existing dirty changes are not included in this task's change claims.
Git's full diff includes prior work and omits untracked file content; do not use
its aggregate diff-stat as this review's change inventory.

## Verification commands

Run from canonical repository. L1 checks actually run offline:

```sh
git status --short
git diff --check
cargo fmt -p photara-store -- --check
cargo fmt -p photara-library -- --check
cargo test --offline -p photara-library -p photara-store -p photara-core -p photara-node-sdk
cargo clippy --offline --workspace --all-targets -- -D warnings
cargo check --offline --workspace --all-targets
```

Untracked files need explicit inspection; ordinary git diff omits them. Whole-
library tests were not needed for this bounded slice; selected Library tests
now run with L2's temporary-database authority. Compilation is not service or
package-publication verification.
## PS2 physical dispatch/liveness/reserve/barrier gate — 2026-09-17

The [disposable review](architecture/PS2_PHYSICAL_DISPATCH_LIVENESS_RESERVE_BARRIERS.md)
records six 1k/10k/100k real-file placement fixtures, their [raw
measurements](architecture/verification/ps2-physical-dispatch-liveness-reserve.jsonl),
and 344 local-APFS process-fault scenarios. A HEAD-bound, generation-keyed
locator and persistent 32-step liveness queue showed bounded lookup/selection
and exact old/candidate placement retention. The tested physical layout is
**not acceptable for continuous authoring**: at 100k, radix physical-only
publication wrote 222–233 KB metadata plus 250–287 KB copied tail per small
logical append; B-tree amplified more. The metadata bootstrap and old tail
generations are not reclaimable in bounded work. Liveness covers only the
fixture's active/recovery/queued pins, not all product obligations.

The APFS matrix observed 2,994 successful `fsync` and 2,875 `F_FULLFSYNC`
calls across 344 passing scenarios, with 284 old and 60 candidate HEAD
reopens. These are process-cut observations, not a production-qualified
file/directory/power-loss profile. Physical reserve liabilities proved safe
refusal under injected exhaustion, not exclusive OS capacity or guaranteed
progress. Eight release example tests, the ordinary package suite, strict
Clippy, Rustfmt and whitespace checks passed. No permanent wire, production
reader/writer, live package, migration, GC, Asset Store or project switch was
changed. No commit/push. Main remains separate and clean.

**Next gate:** non-tail-copy bounded physical placement, reclaimable bootstrap
metadata, all pins/leases, integrated admission/reconciliation and exact
qualified-storage barriers. Radix remains a conditional *logical* lead;
B-tree remains a comparison. Do not freeze wire or claim production latency.
