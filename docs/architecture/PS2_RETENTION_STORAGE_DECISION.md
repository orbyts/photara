# PS2 retention and storage decision — approved direction

Status: direction approved 2026-09-16, **not implemented**. This memo narrows the
[PS1 retention proposal](PS1_RETENTION_PROPOSAL.md); it does not select or reserve a
format version, required-feature identifier, migration number, or production limit.
The [PS0 contract](PROJECT_SESSION_DURABILITY.md) remains the current baseline.

## Approved decision and remaining gates

Design a versioned sealed-checkpoint-root protocol for production autosave, with
explicit migration consent and deliberate loss of obsolete intermediate autosave
save points. Preserve all current authored content and explicitly retained history,
source snapshots, resource versions, evidence and opaque extensions. Retain a
verified previous root for recovery; retain operation-ID dedupe independently of
undo and package ancestry. Require a reader that understands the new root contract;
older readers must refuse it. This is approval of retention/compatibility direction,
not approval to convert, prune, install, publish or edit a live package.

Continue disposable PS2 verification and retain existing read-only/explicit-save
routes until implementation and qualification pass. This approval selects no format
number, migration or production threshold. The experimental numbers below are
fixture inputs only. Multi-surface authority follows the
[shared Rust contract](PROJECT_SESSION_DURABILITY.md#shared-authority-authorization-and-concurrent-clients);
root turnover must preserve its dedupe/recovery and credential-free operation
provenance, not turn historical authorization into a reusable grant.

## What the evidence establishes

The current 1.1 reader follows and verifies the entire parent chain. The default
ceiling is 1,024 commits; the pure planner clamps caller budgets to that ceiling.
The existing ceiling fixture accepts 1,024 and refuses checkpoint 1,025 with all
original bytes intact. Removing ancestors or merely raising a limit does not solve
the format's indefinite-growth problem. See the [reader](../../crates/photara-store/src/package/v1_1/reader.rs)
and [planner fixtures](../../crates/photara-store/tests/package_planning/mod.rs).

The disposable measurement added in `06fb538` performed 128 tiny Graph-name changes,
one checkpoint per change, in debug mode with complete closure re-verification at
each iteration. The observed output was:

| New checkpoints | Package revision | Files | Total logical bytes |
| --- | --- | --- | --- |
| 1 | 2 | 42 | 40,655 |
| 32 | 33 | 166 | 283,913 |
| 64 | 65 | 294 | 535,049 |
| 128 | 129 | 550 | 1,037,439 |

Growth at 128 was 1,004,628 bytes; that debug run took about 66.9 seconds, including
full closure re-verification. These are synthetic logical-byte measurements, not
physical disk writes, realistic Graph-edit costs, release latency targets, or a
basis for extrapolating large-workload limits. The fixture is explicitly ignored in
routine tests. Single-RenameGraph semantic equality evidence in `69ad2db` is also
test-only and does not implement the complete typed journal contract.

## Recommended policy to develop and verify

1. Introduce a sealed root whose canonical record binds its complete current
   authored/history closure, predecessor commitment, journal inclusion coordinate
   and identity. The predecessor commitment records provenance; it is not a claim
   that discarded historical bytes remain available. Specify exact traversal and
   validation rules before choosing an encoding/version.
2. Preserve every byte reachable from current authored state, retained history,
   source snapshots and evidence, including unknown optional fields. Only obsolete
   autosave commit envelopes and objects proven unreachable from every retained
   root/history/undo/recovery reference become cleanup candidates. A timer, filename
   or ancestry age alone never authorizes deletion. This policy does not prune
   explicit user history or media to make room.
3. Keep the active sealed root and one independently verifiable previous root as
   rollback/recovery bases. Each must have its complete closure; neither may depend
   on deleted ancestry. Retain any additional root pinned by an unresolved intent
   or explicit history. Publish and verify the replacement root, then persist the
   package receipt and local recovery/dedupe index before retiring a superseded
   recovery generation. An unknown barrier result postpones all retirement.
4. Keep the complete operation-ID/request-digest index for the journal incarnation.
   Undo eviction never removes dedupe evidence. Use PS0's 64 MiB segment and 256 MiB
   uncheckpointed budgets as furnace inputs; its 100-group/32 MiB undo horizon
   remains a tuning proposal. A full dedupe index freezes admission until a reviewed
   continuation protocol exists; do not forget IDs or silently create an incarnation.
5. For the new-root furnace only, start testing compaction at 512 post-root commits
   or 50% of the existing JSON/object budget, whichever comes first (64 MiB aggregate
   JSON or 50,000 inventory objects). These are proposed experiment thresholds,
   **not validated production defaults**. Account separately for both retained
   closures, blobs, journal, index, staging and filesystem space. Before accepting
   a mutation, prove capacity for its entire bounded checkpoint and recovery work
   while retaining the old base. Reject before admission if that reserve cannot be
   met. A live closure that cannot fit requires backpressure and user-directed
   recovery/export, not deletion of authored content. Real workloads must determine
   the reserve and final trigger values before production approval.

## Alternatives not recommended

Keeping every ancestry record and raising 1,024 merely postpones exhaustion and
increases validation work. Longer autosave intervals reduce frequency but cannot
bound growth. Cutting the chain while still advertising 1.1 violates current reader
semantics. Automatic external archival moves correctness into an availability and
catalog-identity protocol that has not been designed; a hash or pointer is not an
archive. Calling device-journal-only state Saved would misrepresent the portable
package and does not satisfy PS0's package publication contract.

## Reader and migration implications

Old 1.1 files remain intact and readable until an explicit conversion is requested.
A future feature/minimum-reader boundary must make existing readers refuse new
root semantics before considering a shortened chain complete. No downgrade writer
may reinterpret a root as a legacy genesis commit. Conversion must bind the exact
old HEAD, identity, target root and pending journal, and preserve a verified recovery
base throughout interruption. Define where the original retained bytes live and
how they are accounted for before offering conversion. This memo authorizes no
copy, migration, format number, schema change or cleanup.

## Storage qualification is a separate gate

Recommend keeping the existing initial scope: specifically qualified local APFS;
provider-managed, network and unknown storage remain read-only. Qualify a versioned
profile tied to volume identity and supported platform behavior; requalify after
move/remount. Require safe descriptor-relative access, stable exclusive lifetime
lock, immutable no-replace publication, same-volume atomic HEAD replacement, and
verified file and directory barriers. APFS naming and a successful rename probe
are insufficient. Apple's [exclusive-renaming property](https://developer.apple.com/documentation/foundation/urlresourcevalues/volumesupportsexclusiverenaming)
reports support for `RENAME_EXCL`; it does not establish the rest of this profile.

Apple's [fsync manual](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/fsync.2.html)
warns that drive buffers can survive a successful `fsync` call and recommends
`F_FULLFSYNC` for stronger persistence/ordering. Apple's current
[disk-write guidance](https://developer.apple.com/documentation/xcode/reducing-disk-writes)
describes `F_FULLFSYNC` as best effort in its iOS discussion, including possible loss
on sudden power failure. Neither source justifies an unconditional macOS hardware
guarantee. Define the macOS profile's tested barriers and failure model explicitly;
do not substitute `sync_all`, process exit/reopen, capability booleans, or
`F_BARRIERFSYNC` for demonstrated persistence. Unsupported/failed full-sync or
directory barriers refuse acknowledgement; uncertain outcomes freeze and reconcile.

## Evidence still required before production PS3 autosave

- Canonical root/feature/reader specification, old-reader refusal, new-reader full
  closure validation, rollback/downgrade refusal, and interrupted opt-in conversion.
- Realistic large Graph/gesture/history/opaque-payload/blob workloads; multiple root
  cycles; logical and physical growth, release latency, and capacity reserve under
  journal/index/JSON/object/blob/disk exhaustion. Verify no retained content is lost.
- Complete typed mutation/patch agreement, undo boundaries and checkpoint inclusion;
  dedupe across restart, rotation, root change and undo eviction; no duplicate effect
  after an uncertain append or checkpoint receipt.
- Before/after/unknown faults at each write, full-file flush, no-replace publish,
  directory flush, HEAD replacement, receipt, index switch and owned-file cleanup.
  Independent old-or-new recovery must survive interrupted compaction without
  recursive deletion or loss of the original evidence.
- Qualified macOS adapter tests for competing writers, lock/root/parent/inode
  replacement, symlink/hardlink/case attacks, provider/network refusal, remount,
  disconnect and full-sync failures. Process termination tests establish process
  recovery only. Any stronger crash/power-loss claim needs separately scoped
  platform/storage fault evidence and explicit residual-risk wording.

These are production-autosave gates, not reasons to pause remaining authorized
disposable tests or a synthetic PS3 coordinator lab.
