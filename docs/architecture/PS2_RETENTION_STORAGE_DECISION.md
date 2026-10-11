# PS2 retention and storage decision — approved direction

Status: direction approved 2026-09-16, **not implemented**. This memo narrows the
[PS1 retention proposal](PS1_RETENTION_PROPOSAL.md); it does not select or reserve a
format version, required-feature identifier, migration number, or production limit.
The [PS0 contract](PROJECT_SESSION_DURABILITY.md) remains the current baseline.

**2026-09-17 clarification approved:** the
[resource storage and verification contract](RESOURCE_STORAGE_AND_VERIFICATION_CONTRACT.md)
separates package-resident closure from managed external byte backing, availability
from retention compliance, and capture identity from retention policy. Those
semantics govern the positive sealed-root fixtures; they add no production codec.

## Approved decision and remaining gates

Design a versioned sealed-checkpoint-root protocol for production autosave, with
explicit migration consent and deliberate loss of obsolete intermediate autosave
save points. Preserve all current authored content and explicitly retained history,
source snapshots, resource versions required by retention/pinning policy, evidence
and opaque extensions. Retain a
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
2. Preserve each required package-resident byte and each external backing obligation
   reachable from current authored state, retained history, source snapshots and
   evidence, including unknown optional fields. Descriptors alone neither embed
   external bytes nor pin them forever; retained history promising reconstruction
   must carry the necessary backing obligations. Only obsolete
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
   package closures, embedded blobs, journal, index, staging and filesystem space.
   External media described by a root is not duplicated/reserved for each
   checkpoint; managed publication needs its own Asset Store reserve, with shared
   physical-volume accounting when applicable. Before accepting
   a mutation, prove capacity for its entire bounded checkpoint and recovery work
   while retaining the old base. Reject before admission if that reserve cannot be
   met. A live closure that cannot fit requires backpressure and user-directed
   recovery/export, not deletion of authored content. Real workloads must determine
   the reserve and final trigger values before production approval.

An offline retained Asset Store does not invalidate package metadata or undo its
prior durable-publication/verification evidence. Report unavailability and current
observation uncertainty independently. Only evidence that an active obligation is
unmet establishes unsatisfied retention: for example confirmed loss/corruption
without enough remaining qualified copies, or definite failure to establish a
required backing. Apparent missing files and unknown publication outcomes first
require identity/binding reconciliation. Offline alone is not proven loss.

Captured versions are not automatically retained indefinitely. Authored history,
recovery roots, explicit policy, pending operations and other declared obligations
pin bytes. Once all pins end, backing may become eligible for conservative
retirement while useful identity/evidence survives. No cleanup algorithm is
approved here; the positive model must prove that neither obsolete root ancestry
nor descriptor-only provenance accidentally pins every historical media version.

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

## Acceptance-base turnover packet — proposed 2026-10-11, not frozen

This develops the approved direction and the user's latest-position clarification;
it does not reserve schema IDs/versions, authorize migration/deletion, or change
deployed bytes. Names below are **draft type labels**, not dispatch strings.
Unlike the original 1.1 motivation above, current v1_3 already has sealed
RootSet/StateRoot records and non-traversed predecessor commitments. The missing
piece is a finite acceptance/recovery base: `operations::audit_operations` still
enumerates lifetime receipts and recomputes both prefixes; repeatable originals
retain all predecessor controls and exact allocation prefixes.

### Exact proposed data delta

Reuse canonical JSON, checked Id/Digest/Decimal, existing JSON ObjectRef (`JRef`),
physical references, receipt v1 and accepted/journal-prefix hash domains. New
records reject unknown fields and require `extensions:{}`. All records carry
`project_id`, `library_id`, `bootstrap_sha256` matching the selected package.
The following field sets are in addition to those identity/schema/extension fields.

| Draft record | Exact proposed fields and typed references |
| --- | --- |
| `AcceptanceBase` | `checkpoint_id:Id`, `source_commit:{commit_id:Id,commit_sha256:Digest}`, `source_state:JRef`, `authored_revision:Decimal`, `authored_sha256:Digest`, `accepted:{through_ordinal:Decimal,prefix_sha256:Digest}`, `journal_inclusion:<existing StateRoot inclusion object>`, `receipts_by_id:JRef`, `receipts_by_ordinal:JRef`, `receipt_count:Decimal`. Source commit/state are **provenance commitments**, not traversal edges. Both receipt roots are the existing typed operation-ID/ordinal tree encodings, preserving exact receipt entries; they belong to the historical lookup class below. `receipt_count` equals accepted ordinal. No portable edge points into the device-local journal. |
| `CheckpointOperationIndex` | `base:JRef`, `accepted:<existing accepted object>`, `by_id:JRef`, `by_ordinal:JRef`. Base selects `AcceptanceBase`; both trees select only post-base receipts, using existing tree/receipt encodings. Suffix ordinals are contiguous from base ordinal+1; recompute existing acceptance/journal hashes starting at the base commitments. Validate receipt before/after continuity from the base authored coordinate to selected current authored state. |
| `JournalBase` (device-local) | `checkpoint:JRef`, `journal_id:Id`, `through:{record_id:Id,checksum:Digest,sequence:Decimal}`, `mutation:null|{record_id:Id,checksum:Digest}`, `accepted_frame:null|<existing AcceptedFrameV1>`, `coordinate:<existing PS3 Coordinate>`, `undo_roots:[<existing OperationTarget>]`, `archive:JRef`. `checkpoint` selects the package AcceptanceBase; `through` is a complete PHPSJ001 outer-record boundary; Mutation/frame and coordinate must equal independently verified inclusion there. Sorted unique undo roots preserve existing owner/action/patch evidence; this first slice removes only eligible completed move targets, preserving structural targets and redo dependencies. `archive` selects the exact archived journal segments and original recovery material below. |
| `HistoryArchive` (device-local) | `archive_id:Id`, `previous:null|JRef`, `segments:[{journal_id:Id,first_sequence:Decimal,last_sequence:Decimal,byte_length:Decimal,sha256:Digest}]`, `originals:JRef`, `receipts:JRef`. Segments sort by journal ID then sequence, have no overlap, and retain exact bytes; originals and receipts use authenticated inventories of exact existing records. `previous` is a historical lookup edge, not a current-state traversal edge. Archive location/pins are local registration data, never portable host paths. Original/receipt inventory refs resolve in this registered archive namespace; they do not inject journal bytes into portable authored closure. |

No generic replacement RootSet is needed. A new required reader/codec dispatch
would permit the existing StateRoot field layout to select
`CheckpointOperationIndex`, with the base's coordinate as the suffix starting
point. Existing RootSet active/recovery/pinned roles, per-role locator/ownership,
authored/history/resource roots and conversion evidence remain mandatory. The
root inventory rules must explicitly distinguish **current closure** from
**historical lookup edges**; existing inventories cannot silently omit old
receipt/control edges. Historical lookup pages and archived allocations remain
located, authenticated and fully charged; they are not free or deleted bytes.
The new admission dispatch binds the exact selected base and suffix, rather than
recursively retaining every completed pre-base original. Unresolved originals
remain full current recovery dependencies with their exact old inputs.

Receipt lookup checks suffix first, then the base's indexed archive. Return the
exact original receipt only after checking request digest, identity, index path
and actual receipt bytes; changed request reuse refuses. New admission proves
absence in both indexes before assigning the next ordinal. No dedupe expiry or
incarnation reset is proposed. The full reciprocal indexes/prefixes remain
available for explicit historical audit. Undo retirement never removes receipts.

### Turnover and original retry

Turnover is storage maintenance, not an authored mutation: it adds no operation
receipt, authored revision or user undo step. Persist one original maintenance
identity and exact `{old_head,old_commit,source_state,target_base,target_state,
target_head,target_commit,journal_base,archive,byte_manifest,reserve,planner}`
before effects. `old_head/commit` and `target_head/commit` use existing exact
objects; refs identify complete candidate bytes, `byte_manifest` uses the existing
admitted allocation/range/hash representation, and reserve/planner retain the
existing finite bounds/accounting rules. This is a proposed new maintenance
dispatch, not a reinterpretation of Graph IntentV1 or a fabricated no-op edit.

First adoption fully audits the source, exact accepted/journal inclusion, receipt
indexes and all retained dependencies. Later turnover extends an independently
verified base with its fully validated suffix. Only checkpointed, resolved move
undo targets may leave the useful-history roots; mixed structural transactions,
unknown commands, pinned snapshots and run/resource dependencies remain retained.
Latest authored bytes and opaque fields are copied exactly. Keep one complete
previous base/state and all reader/unresolved-operation pins. No production undo
budget is chosen, and this slice does not evict structural undo.

Write/sync candidate bytes and archive/index records, then verify the complete
candidate live/recovery closure and exact new inclusion. Atomically replace HEAD
under expected-old CAS and the existing qualified barriers. Append a local
turnover receipt binding `{maintenance_id,old_head,new_head,base,journal_base}`
only after selected verification/barriers; then durably select the journal base
for suffix replay. HEAD remains the package authority: old HEAD selects old replay;
new HEAD with missing receipt/base pointer reconstructs the same maintenance
outcome; an ahead pointer with old/unrelated HEAD freezes as rollback/conflict.
Unknown outcomes retain all bytes and original identity, reissue required barriers,
and never create a new maintenance attempt to escape the uncertainty. A new local
journal-base record/index selection must receive its own reviewed encoding; the
PHPSJ001 header and existing checkpoint bodies are not rewritten.

### Accounting, compatibility and the precise remaining boundary

Use existing sealed-charge/observation and retained-file accounting for immutable
archived allocations, charging each physical inode once even when shared by active,
recovery and history. Mixed live/archive packs cannot be released; moving live
objects out requires existing admitted generation enrollment and complete old/new
coexistence reserve. All old bytes stay charged in the first no-deletion slice.
No credit follows merely from changing inventory membership or closing a reader.
Existing quotas still refuse before admission; archival storage is not made bounded
by calling hot replay bounded. A later reclamation protocol is separate work.

**Recommended single review decision: snapshot recovery semantics.** After the
trusted writer fully audits the old path and durably publishes the original-bound
turnover, the new checkpoint becomes the recovery base. On future opens, require
complete verification of its current/previous/pinned content closures, current
accounting and pending suffix; do **not** require re-proving every pre-base
historical transition. Read old receipt/undo/run content through authenticated
index paths when requested. Keep the existing independently registered identity,
latest-selection/rollback reconciliation and current authorization checks. A
stored checkpoint never grants permission or emits a new Accepted acknowledgement;
only fresh authorized transactions do. Its prefix fields preserve commitments,
not a claim that an opener has re-executed historical validation.

This changes error detection deliberately: corruption or semantic inconsistency
confined to an unrequested, unpinned archive no longer blocks opening/saving a
sound current checkpoint; detect and report it when that history is requested or
explicitly audited. A bad current dependency, recovery root, dedupe lookup path,
requested original receipt or pending suffix still refuses that operation, never
substitutes a new ID or silently skips evidence. Coherent malicious replacement
is not prevented by JSON hashes; existing authority/custody/rollback assumptions
still apply. Initial adoption in this slice is only from the fully verified
registered old selection, not arbitrary imported checkpoint JSON. New import,
copy/rebind and real-library migration remain disabled pending their own reviewed
admission paths.

The alternative is to retain the old historical-validation claim at every cold
open, either paying a full archive audit or adding a separately trusted local
validation certificate with issuer/invalidation rules. **That is not recommended
for this latest-state slice**, and no certificate is proposed here. Approval of
snapshot recovery semantics is the precise remaining proof-obligation decision;
it avoids presenting unlimited historical replay as necessary to save the latest
positions. The field table is a review draft, not yet a frozen encoding packet.

Freeze exact new dispatch/feature/minimum-reader and local base-pointer
bytes only after resolving that narrow boundary and validating acyclic typed edges.
Old readers must refuse; old packages and retries remain intact, with opt-in
conversion and no silent migration. Use existing corpus tests for repeated move
turnovers, retained structural undo/run dependencies, exact old-ID retries,
corrupt archive/index paths, reader pins and every old/new/unknown publication cut.
Measure current/suffix bytes read, full-audit adoption versus subsequent checkpoint open,
save CPU/latency, peak/live memory and total charged archive growth separately.
Do not claim bounded lifetime storage, constant cost for arbitrary live content,
or solved cold replay before those results. This focuses the next review on a
verified acceptance base, rather than making the optional live custody cache the
product's retention architecture.
