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

The initial field sketch is superseded by the consolidated candidate byte contract
below; there is one proposed field/dispatch packet, not two competing formats.

### Accounting, compatibility and the precise remaining boundary

Use existing sealed-charge/observation and retained-file accounting for immutable
archived allocations, charging each physical inode once even when shared by active,
recovery and history. Mixed live/archive packs cannot be released; moving live
objects out requires existing admitted generation enrollment and complete old/new
coexistence reserve. All old bytes stay charged in the first no-deletion slice.
No credit follows merely from changing inventory membership or closing a reader.
Existing quotas still refuse before admission; archival storage is not made bounded
by calling hot replay bounded. A later reclamation protocol is separate work.

**Snapshot recovery semantics — approved by the user 2026-10-11.** After the
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
for this latest-state slice**, and no certificate is proposed here. The user approved
this proof-obligation change on 2026-10-11;
it avoids presenting unlimited historical replay as necessary to save the latest
positions. The consolidated field packet below remains a review draft, not frozen dispatch.

Freeze exact new dispatch/feature/minimum-reader and local base-pointer
bytes only after validating acyclic typed edges and the concrete encoding packet.
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

## Consolidated candidate byte contract — review draft, 2026-10-11

**Approved by the user on 2026-10-11 for Rust implementation and disposable
validation.** Snapshot-recovery semantics and the exact identifiers, encodings and
new dispatch below are now frozen within this packet's scope. Implementation and
qualification remain required; approval does not enable real-library writes,
conversion, deletion, expiry, new production limits or deployment. This section
supersedes the earlier field sketch where they differ. It is one bounded packet:
latest-position turnover, exact old-operation lookup, no deletion, no new commands,
no production thresholds, and no arbitrary import or real-library conversion.
Validation items at the end remain open; this is not qualification evidence.

### Dispatch and exact package records

Propose selected commit `minimum_reader:{major:1,minor:4}` and required feature
`photara.snapshot-recovery.v1`, retaining commit/head v1 bytes and field meanings.
The original bootstrap manifest and its hash do not change. Admission explicitly
checks the old manifest plus upgraded selected commit; an unsupported reader must
refuse before interpreting a root. All old v1/v2/v3 originals dispatch unchanged.
Proposed new names were searched against existing source/proposal identifiers;
version additions must dispatch by the complete `(id,version)` pair.

Use existing canonical JSON, Id, Digest, Decimal, JRef (including `kind:"json"`),
PhysicalRef, ReceiptV1 and accepted/journal-prefix domains. `Common` below means
exact `schema,project_id,library_id,bootstrap_sha256,extensions`, with matching
identities and `extensions:{}` for newly introduced records. Retained existing
objects preserve all optional bytes. No field implicitly denotes authority.

| Proposed schema | Exact fields beyond Common and reference meaning |
| --- | --- |
| `photara.package.acceptance-base` v1 | `checkpoint_id:Id,source_commit:{commit_id:Id,commit_sha256:Digest},source_state:JRef,authored_revision:Decimal,authored_sha256:Digest,accepted:{through_ordinal:Decimal,prefix_sha256:Digest},journal_inclusion:null\|<existing StateRoot inclusion>,receipts_by_id:JRef,receipts_by_ordinal:JRef,receipt_count:Decimal`. Source commit/state are provenance commitments, not traversal edges. Receipt roots select existing operation-ID/ordinal trees as historical lookup edges. Count equals accepted ordinal; zero acceptance has null inclusion. Nonzero inclusion's resulting coordinate equals the base authored coordinate. No edge points into a device-local journal. |
| `photara.package.operation-index` v3 | `base:JRef,accepted:<existing accepted object>,by_id:JRef,by_ordinal:JRef`. Base selects acceptance-base v1. Trees contain only the post-base suffix, using unchanged tree/receipt encodings. Both contain exactly the same entries, unique by operation ID, with contiguous ordinals from base ordinal+1. Replay existing prefix domains from the base, validating receipt before/after continuity and final selected coordinate. Empty suffix has the base coordinate, not ordinal zero. |
| `photara.package.state-root` v3 | Exact v2 field set; `operation_index` selects v3. Authored/history/resource-state and explicit pin semantics are unchanged. Inventory includes the base object and complete suffix closure, but not base receipt-tree descendants. Those are historical lookup edges. |
| `photara.package.root-set` v3 | Exact v2 fields plus `archive_placement:{locator:PhysicalRef,ownership:JRef}`. Existing placements continue to cover each distinct active/recovery/pinned StateRoot. Archive locator maps historical receipt/index objects with their existing semantic/ownership membership; its ownership claims cover their physical allocations. Global inventory includes current role closures and archive ownership metadata, not historical receipt/index descendants. Physical locator pages remain authenticated implementation edges, never inventory themselves. |
| `photara.storage.allocation-claim` v2 | Exact `schema,project_id,extensions,allocation_id,arena,layout,owned_extent,verification,sealed,sealed_charge`. `layout` is exactly `framed-json`; `verification` is exactly `selected-records`; other fields preserve v1 meaning. This explicitly replaces v1's whole `authenticated_prefix` obligation for this new dispatch only. Whole-Blob claims remain v1. |

For claim v2, freshly verify registered identity, safe handle/name binding, extent,
charge and sealed-charge agreement. Each current or requested frame must validate
actual offset/length, tag/header, canonical body and physical/logical hashes;
overlap, conflicting aliases and out-of-extent references refuse. Do not scan
unselected historical frames merely to claim whole-pack canonical validity: that
claim is no longer made. New writes still validate every admitted suffix byte,
including padding, before publication. An old claim v1 always retains its old
full-prefix obligation, even under an upgraded outer commit.

Historical receipt lookup searches suffix then base by-ID tree; authenticate each
accessed path and the exact receipt. Same ID/request returns the original receipt;
changed request refuses. Prove absence in both before assigning a new ordinal.
Full reciprocal coverage and old prefix validity are checked at initial adoption;
subsequent turnover folds the fully verified suffix into the base indexes, retaining
all entries. Old immutable subtrees may be reused by commitment. Scoped lookup is
not a claim that unvisited history was audited on this open. Existing page/depth/
entry/byte budgets apply to visited work and full-audit operations respectively;
cache exhaustion changes speed, not admission. No dedupe expiry/reset is introduced.

### Exact device-local base and selection

Propose these separate local records; none becomes a portable authored edge.
Each carries Common plus `device_id:Id,incarnation_id:Id` bound to registration.
`Link` is `{record_id:Id,checksum:Digest,sequence:Decimal}`. `ArchivedRecord` is
`{stream_id:Id,offset:Decimal,byte_length:Decimal,record:JRef,checksum:Digest}`;
its range contains one complete journal frame, whose payload is the exact JRef.

| Proposed schema | Additional exact fields |
| --- | --- |
| `photara.session.checkpoint-base` v1 | `checkpoint:JRef,package_head:<HeadV1>,package_commit:JRef,journal_id:Id,through:Link,mutation:null\|Link,accepted_frame:null\|<AcceptedFrameV1>,coordinate:<PS3 Coordinate>,undo_roots:[<existing OperationTarget>],lookup:JRef,archive:JRef`. Package checkpoint selects AcceptanceBase; other package commitments select the verified clean turnover result. Through is the old segment's complete outer-record boundary; Mutation/frame identify its actual accepted projection. Undo targets sort by operation ID, then Mutation link; no duplicates. |
| `photara.session.checkpoint-operation-lookup-leaf/branch` v1 | Reuse the existing operation-ID tree's exact `count,entries` or `count,children` grammar/ranges/order. Entry is `{operation_id:Id,request_sha256:Digest,receipt:JRef,local:null\|{mutation:ArchivedRecord,owner_sha256:Digest,action_sha256:Digest}}`. Null is allowed only for inherited receipts lacking PS3 Mutation evidence. Resolve exact old owner/action/receipt before a local retry; never manufacture undo ownership. |
| `photara.session.checkpoint-archive` v1 | `archive_id:Id,previous:null\|JRef,segments:[{stream_id:Id,journal_id:Id,header_sha256:Digest,byte_length:Decimal,sha256:Digest,first_sequence:Decimal,last_sequence:Decimal,last_checksum:Digest}]`. Sort by journal ID then first sequence, no overlap. Previous is a historical lookup edge. Exact files and safe pins are separately registered; no embedded host paths. Segment ranges are closed before turnover and never rewritten. |
| `photara.session.checkpoint-selection` v1 | `generation:Decimal,previous_sha256:null\|Digest,selected:{format:"PHPSJ001"\|"PHPSJ002",stream_id:Id,header_sha256:Digest,base:null\|JRef},pending:null\|JRef`. Base is null exactly for PHPSJ001. Pending selects the one original maintenance record. Generation increases by one per exact CAS replacement; previous digest is nonedge provenance. |

Propose `PHPSJ002` framing with the same 8-byte magic/LE-u32 length/SHA-256 chain
construction as PHPSJ001, using the new magic in its header hash domain. Header
has the exact PHPSJ001 field set, `format_version:2`, plus
`base:JRef,sequence_base:Decimal,previous_segment:{stream_id:Id,header_sha256:Digest,last:Link}`.
Use a fresh stream ID but preserve logical journal ID and incarnation. Base selects
the local checkpoint-base; sequence_base equals its through.sequence. Envelopes
retain the existing exact fields with `format_version:2`, and outer sequence starts
at sequence_base+1. Mutation logical sequence/acceptance ordinal continue from the
base; they are not reset or confused with outer sequence. Existing bodies are
unchanged. The sole new kind, `CheckpointBaseReceipt`, has exact body
`{maintenance_id:Id,original:JRef,journal_base:JRef,selected_head:<HeadV1>,selected_commit:JRef}`;
it is the first record and changes no authored/accepted coordinate. Its package
and base commitments must agree. Its envelope identity is regenerated from the
persisted original: `record_id` is the maintenance ID and `session_generation`
is the original-bound pending selector generation (old generation plus one).
No new retry input is generated during activation. It is appended only after fresh clean selection
verification and barriers; its own barriers precede selecting the new segment.

The local selector is one canonical file in the independently registered journal
namespace, atomically replaced under exact-old-byte CAS with file/directory/full
barriers. Its pathname is registrar-supplied, not an arbitrary caller path. On
first adoption, establish a selector for the existing PHPSJ001 header under that
lease; do not rewrite its header. A pending selector is persisted before package
effects. Unknown selector publication is reconciled against only exact old/new
bytes, never an arbitrary well-formed successor. Old segments remain retained.

### Acyclic original and maintenance phases

Propose `photara.storage.checkpoint-turnover-original` v1 with Common plus exact
`maintenance_id:Id,scope:<existing registered scope>,old:{head:<HeadV1>,commit:<CommitV1>,selector:JRef},target:{base:JRef,state:JRef,recovery:JRef,pinned_roots:JRef,archive_placement:{locator:PhysicalRef,ownership:JRef}},journal:{next_stream_id:Id,through:Link,lookup:JRef,archive:JRef},attempt:<existing Attempt field set>,writes:[Write]`.
Attempt preserves token/nonce, finite reserves/geometry and fixed IDs; its selectors
array has exactly four entries in the order below. `Write` is exactly
`{allocation_id:Id,arena:"data"\|"metadata",witness:<existing device/inode object>,original_end:Decimal,maximum_end:Decimal,target_end:Decimal,suffix_sha256:Digest}`,
sorted by allocation ID. Writes only use existing registered allocations; no new
enrollment, quota expansion or retirement is part of this slice. Target closure
bytes precede original construction and contain no reference back to it. The
original contains neither its own reference nor candidate HEAD/commit hashes.
Those selectors are regenerated deterministically from it, avoiding a digest cycle.

Propose `photara.storage.checkpoint-turnover-phase` v1 with Common plus
`maintenance_id:Id,original:JRef,stage:"reserved"\|"written"\|"published"\|"clean",consumed:Decimal,remaining:Decimal`.
Use existing hold/accounting-envelope machinery with explicit new original/phase
dispatch. Reserved selects old content and the finite original hold; written is
selected only after exact payload barriers; published selects new roots and
charges actual admitted growth once; clean selects the new roots and empty holds
only after cleanup barriers and zero remaining liability. Original and phases are
loose controls outside their own inventories; no phase refers to its containing
HEAD. The fixed four selector IDs and existing timestamp from the old commit make
regeneration deterministic; maintenance never changes authored revision or emits
an authored operation/undo receipt. Local pending still retains the original when
clean removes its package hold, until the new segment/receipt/selector is durable.

The proposed graph continuation dispatch is
`photara.codec.ps2-repeatable-admission-v4` / `photara.codec.ps2-repeatable-layout-v4`.
It retains v3 fields except that `payload_recipe` adds `checkpoint_base:JRef`
(the device-local `photara.session.checkpoint-base` v1, whose `checkpoint` edge
selects the package AcceptanceBase; it must match the selected PHPSJ002 header)
and
its retained-control inventory is limited to post-base originals plus unresolved
pins. Planner corridor entries keep their v3 fields except `original_sha256` is
replaced by `base_head_sha256:Digest`, equal to SHA-256 of exact canonical old HEAD.
Old snapshot/current/recovery/pinned objects are verified through their selected
frames; witness/old extent and every appended suffix byte remain exact retry
checks. Unread pre-base archive bytes are not a promised whole-prefix proof.
Turnover Attempt uses `photara.codec.ps2-checkpoint-turnover-layout-v1` with that
same corridor shape. Persist all original planner inputs once. No v1/v2/v3 record
is reinterpreted by this dispatch; persisted old retries keep their full old checks.

### Selection, pins, accounting and failure table

Turnover first drains a finite Accepted target and resolves every affected attempt.
First adoption audits the old path fully. Retire from undo selection only completed
pure-position edit/undo/redo chains whose entire command is set-node-position or
a batch solely of those commands. Keep all structural/mixed/unknown actions,
explicit history, run/resource dependencies and unresolved reader/recovery pins.
Position undo beyond the new base is unavailable; exact receipts remain retrievable.
Latest authored/opaque bytes are unchanged. No output/run codec is introduced.

| Observed durable state after interruption | Required action |
| --- | --- |
| Old selector, no pending original | Old reader/journal path remains authoritative. A bare unselected candidate is not adoption. |
| Pending original, old/reserved/written package | Revalidate old scope and exact original; resume its exact writes/stages or preserve/freeze. Never allocate replacement IDs. |
| New published/clean package, old segment selected | Reconcile the original, verify current/previous/pinned closure and barriers; regenerate the same local base/new segment/receipt. Do not replay included moves. |
| New segment receipt durable, selector uncertain | Compare exact old/new selector bytes; reissue barriers, finish the same CAS outcome. |
| New selector, package old/unrelated/missing | Freeze as rollback/replacement; preserve both domains. A local pointer cannot advance package HEAD. |
| Corrupt pending/current dependency or requested archived receipt | Refuse the affected action; do not truncate, skip, invent Saved or treat old ID as new. |

The first previous root may still be v2 with v1 claims and lifetime audit obligations;
it must retain those checks. Only a later turnover may rotate it to a complete
new-profile previous checkpoint, after new publication/receipt/base selection and
all old reader/unresolved pins release. Do not promise bounded first-adoption or
first-previous-root work. Ordinary previous roots are independently complete;
predecessor hashes are not substitutes for their content.

Archive and mixed live/archive allocations retain full existing ledger charges,
observations, extent and namespace accounting. Charge each inode once across roles;
never credit inventory removal, a closed pin or an obsolete original. Reserve
simultaneous old/new controls, payload, local base/index/segment and temporary
selector names before effects. Actual charge/extent above the original bound
freezes with the old hold; limits are not increased. Existing sealed-charge,
retained-file and Blob semantics remain unchanged. No-deletion turnover bounds
hot semantic work, **not total disk use or lifetime index metadata**.

### Readiness and bounded validation

The above is an approved field/dispatch packet, not a completed implementation
proof. The test-only candidate checks below make the listed subset reviewable;
they do not qualify a runtime implementation. Before enablement, prove exact
current versus archive inventory partition, physical archive placement ownership
and local lookup; implement nonempty suffix continuity/coverage; validate all four
phase selector outputs and local two-domain crash cuts; and size simultaneous
controls under existing limits. These are implementation validation obligations,
not new product-policy questions. The dependency-shape test is not proof of a
complete regenerated maintenance transaction. Any
contradiction found must amend this single packet before permanent dispatch.

Implement in order: pure candidate parser/typed-edge and canonical vectors using
the existing corpus; adoption plus repeated pure-position turnovers with exact
old/new retry/refusal; local segment/selector reconciliation and accounting faults;
then one existing disposable native qualification. Build-gated work counters must
separate full adoption, old-profile previous-root work, mature current/suffix/index
work and requested historical audit. Measure same-input CPU/latency, live/peak
memory, bytes read and full physical charge. Existing frozen vectors/reader refusal
and unsupported-command behavior stay covered. No native enablement, personal data
migration, deletion or production budget follows merely from accepting these names.

### Bounded candidate-vector check

Using only `proposals/ps2/integrated/linked.json` (file SHA-256
`6de4a73749c539ad98f3dfa1b68e2687473cf70ded8328a95b1c3575c6d43e4a`),
the proposed base takes the existing active StateRoot, its exact accepted/inclusion
coordinates and operation-index v2 roots, the exact bootstrap commit ID/hash,
and checkpoint ID `96000000-0000-4000-8000-000000009901`. All Common fields
come from that StateRoot; source_state is its existing JRef; authored_sha256 is
its authored JRef digest. Two existing-schema empty operation-ID/ordinal leaves
use the same Project, count `"0"`, entries `[]`, extensions `{}`. Index v3 selects
the resulting base plus those empty leaves and preserves the base accepted value.
Sorted-key, compact, UTF-8 canonical encoding through the existing corpus recipe
produced:

| Candidate | Bytes | SHA-256 |
| --- | ---: | --- |
| AcceptanceBase v1 | 1,424 | `8f6944120c4526f0a206489a9d8891d98bd50f27b1705d8d4b1ad42bd11a1d8e` |
| Existing empty operation-ID leaf v1 | 158 | `5913c637d24acc366d39a4c04c37716dffa4db028ebea70bba9e7b6b0b7313b4` |
| Existing empty operation-ordinal leaf v1 | 163 | `ce4323a8968a9ad3cac51a15d7c76c86a3286ea72493813184eb5a83ab67af32` |
| Empty-suffix operation-index v3 | 752 | `31d45052d27bded73f69aeea28337fce97e1df69ac81e3946ba7f2cb3e26bab1` |

This bounded check verified coordinate agreement, canonical bytes and the local
acyclic base→historical roots / suffix index→base graph. A source/proposal search
found no existing uses of the newly named schema prefixes, feature or PHPSJ002
magic outside this packet. It does **not** validate whole-package placement,
maintenance selectors, local crash recovery, physical accounting or performance;
those explicit implementation gates above still apply. No production parser or
writer was added, and no package/journal/fixture file was modified.


The isolated `repeatable::tests::checkpoint_candidate_tests` module additionally
pins these four vectors through Rust canonical encoding. It checks exact adoption
identity/coordinate/index commitments and empty-suffix schema/edges; uses the
existing frame decoder to refuse requested corruption, malformed canonical bodies,
overlap, wrong physical/logical identity and extent; and distinguishes corruption
in an unrequested former frame **inside the fully observed owned extent** from
corruption in that same frame when requested. Its explicit proposed typed-edge
graph excludes provenance commitments and rejects introduced maintenance/local-base
cycles. It provides no production dispatch, authority, charge reservation, complete
nonempty-index parser or filesystem barrier evidence.

## Narrow v4 commitment correction — pending approval, 2026-10-11

Implementation exposed two additional inherited whole-allocation commitments.
The approved packet replaces `planner.corridors[*].original_sha256`, but v3 also
stores that field in the admission payload recipe and finalization writes.
Keeping those hashes would reread unrelated archive bytes and make an exact v4
retry depend on corruption that the approved snapshot recovery model tolerates.
Silently changing what `original_sha256` means is not permitted.

Approve these two replacements **only for the new v4 dispatch**:

| Location | Exact correction |
| --- | --- |
| `photara.codec.ps2-repeatable-admission-v4` → `payload_recipe.allocations[*]` | Replace `original_sha256:Digest` with `base_head_sha256:Digest`. |
| `photara.storage.finalization-control` → `writes[*]`, only when `recipe_codec` is `photara.codec.ps2-repeatable-layout-v4` | Replace `original_sha256:Digest` with `base_head_sha256:Digest`. |

In both locations the value is SHA-256 of the exact canonical old HEAD already
committed by the original attempt, identical to the approved v4 corridor value.
Require the new field and reject the old field or both fields in these v4
objects. All other fields retain their exact meanings and encoding. In particular,
allocation identity/witness, bounds, frame count and appended-range digest remain
unchanged. Finalization `recipe_sha256` is regenerated using the existing rule
over the corrected write object before adding `recipe_sha256` itself.

Current/recovery/pinned records still receive their required verification;
retained v1 allocation claims retain their full legacy checks. Every appended
suffix byte and exact original retry input remains verified. Unrequested archive
bytes do not acquire a new whole-prefix integrity promise. Existing v1/v2/v3
codecs, hashes and golden bytes are unchanged, as are the approved turnover
format, Accepted/Saved barriers, accounting and registration limits.

Validation stays in the existing corpus: two consecutive v4 operations,
interruption/retry of the same original, changed HEAD/witness/extent/suffix refusal,
and continuation with unrequested archived corruption. No new fixture family,
real-library writes, conversion, deletion, expiry, deployment or production limits.
Dependent v4 emission/retry remains disabled until this correction is approved.
