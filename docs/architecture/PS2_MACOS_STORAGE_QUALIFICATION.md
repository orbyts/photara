# PS2 macOS local-APFS qualification plan

Status: review-only test plan; **no storage profile is qualified by this document**.
Inputs: [PS0 publication protocol](PROJECT_SESSION_DURABILITY.md),
[PackageIo interface](../../crates/photara-store/src/package/planning/io.rs), and the
[approved retention/storage direction](PS2_RETENTION_STORAGE_DECISION.md). Retention
implementation is independent of the disposable work described here. No production
writer, live package access, migration, installation or power-loss experiment is
authorized by this plan.

## Scope and admission

Develop one adapter for explicitly supported macOS/SDK versions and local APFS
storage configurations. Record the OS build, adapter revision, filesystem/mount
facts, volume identity, device class, barrier operations, tests and failure model
in its qualification report. Bind runtime qualification to the pinned volume and
root; move, remount, identity change or unknown capability invalidates admission.
Network, File Provider/cloud-managed, unsupported and ambiguous storage refuse
writes. Being outside one familiar cloud folder is not proof of provider exclusion.
Specify and test the provider-detection policy before enabling a production profile.

The existing `CapabilityProfile` flags are a policy checklist, not measurements or
an unforgeable qualification result. A future adapter must own their construction
and retain evidence for every assertion. Do not set all flags from an APFS name,
successful `fsync`, successful rename, or an environment variable.

## Requirements mapped to disposable evidence

| Interface/flag | Required implementation property | Minimum refusal/race tests |
| --- | --- | --- |
| `pin`, `safe_handles` | Retain owned descriptors for parent, root and relevant child directories. Walk components without following links; bind filesystem/device/inode identities to opened handles. Validate regular files, single link count, component spelling and permissions before mutation. | Substitute parent/root/child with a symlink or another inode at each boundary; try traversal, case aliases, FIFO/device/directory-as-file and hardlinked destination. Sentinel bytes outside the fixture target must stay unchanged. |
| `lock`, `exclusive_lifetime_lock` | Hold a nonblocking exclusive lock on one stable `.writer-lock` regular inode for the writable session. Pin/check its identity and keep its descriptor private. Never unlink it, steal by timeout, or use PID text as ownership. | Two separately launched processes cannot both enter publication. Reopen after holder death; replace/unlink the lock inode and require refusal. Exercise duplicate descriptor and child-process inheritance so cleanup cannot unlock a live owner's lease. |
| `recheck` | Under the lease, compare parent/root/volume/lock identities, exact manifest and HEAD bytes, and package incarnation. Recheck before publication and verify after it. | Same numeric revision with different bytes; identical bytes at a replaced inode; stale incarnation; replacement between comparison and publication. No new write/commit IDs on retry. |
| `create_temporary`, `write_all` | Create-exclusive beside its final destination through pinned descriptors; private mode subject to access policy; register attempt-owned inode before later cleanup. Write complete canonical bytes with short-write handling. | Occupied name, symlink, wrong inode/type, permission/quota/space failure, interrupted/partial writes. No existing object is truncated or adopted by name. |
| `publish_no_replace`, `immutable_no_replace` | Publish only a fully written/flushed temporary with descriptor-relative exclusive rename. Existing immutable data is reusable only on exact length/hash/byte agreement under safe access. | Two publishers race for one name: one wins; loser cannot replace it. Test equal and unequal pre-existing bytes, symlink and directory collisions. |
| `replace_head`, `atomic_same_volume_head` | Create and flush a new HEAD beside the old HEAD; compare under lease, then replace exactly `HEAD.json` on the same filesystem. Never delete HEAD first or fall back to cross-volume copying. | Concurrent readers observe whole old/new HEAD bytes, never a missing/partial record. Refuse different filesystem and stale token. Verify candidate closure independently after replacement. |
| `full_flush`, `full_file_flush` | Use a documented, platform-qualified full-file persistence operation on every newly published object, commit, HEAD and journal record. Record success/error and ordering. | Inject unsupported operation, interrupt, I/O failure and unknown completion. Plain `sync_all` is not an acceptable silent fallback. |
| `flush_directory`, `directory_flush` | Establish the exact supported directory-entry persistence barrier after creation/publication/replacement, including newly created ancestors. Qualify its relationship to the file/device barrier. | Fault every parent/root flush; exercise fresh subdirectories and lost/reordered metadata. Successful file flushing alone cannot mark this flag true. |
| `verify_candidate`, cleanup | Read through pinned safe handles and validate complete closure with current reader rules. Delete only registered temporary names still bound to their owned inode, under the same ownership assumptions. | Changed bytes, detached root, substitute cleanup target, unknown temporary and interrupted cleanup. Preserve evidence and unknown files; no recursive deletion. |

Apple's [open manual](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/open.2.html)
describes create-exclusive and no-follow behavior. Checking only the final component
does not establish safety of an entire path; the component walk and handle checks
above are adapter requirements. Apple's [APFS APIs guide](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/APFS_Guide/ToolsandAPIs/ToolsandAPIs.html)
lists descriptor-relative `renameatx_np`; its
[exclusive-renaming capability](https://developer.apple.com/documentation/foundation/urlresourcevalues/volumesupportsexclusiverenaming)
specifically reports `RENAME_EXCL` support. Compile/runtime support and all failure
behavior must still be tested for the supported target. The existing creation
adapter offers implementation references but does not supply edit-writer qualification.

## Ownership and atomicity limitations

Apple's [flock manual](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/flock.2.html)
defines advisory locking: other processes can ignore it. Duplicated/fork-inherited
descriptors also share the lock. Therefore `excludes_uncooperative_writers` cannot
be inferred from a lock test. The compiled interface removes that assertion and
requires opaque registered admission for every mutation; no production path can
construct it or a qualified adapter. Detecting a
change after publication does not undo a race already lost to an uncooperative
writer. The approved [writer admission direction](PS2_WRITER_ADMISSION_PROPOSAL.md)
separates registered in-place cooperative admission from storage qualification,
without requiring a managed root. All controlled writer surfaces use one Rust
authority; another direct process gets WriterBusy while the lifetime lease is held.
The interface separation does not qualify storage or implement admission/routing.

The [disposable lease fixture](../../crates/photara-store/tests/package_planning/macos_lease.rs)
observes actual independent-process contention: a contender receives typed
WriterBusy while the holder lives; after holder termination, a fresh process
acquires and verifies unchanged inode coordinates, manifest/HEAD and full closure.
Per-mode completion markers and bounded waits guard against skipped or hung child
tests. This establishes only the exercised advisory-lock behavior. It does not
qualify path/lock substitution, inherited descriptors, noncooperators, providers,
power-loss durability or production ownership/routing.

Apple's [rename manual](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/rename.2.html)
documents replacement and same-filesystem behavior. Atomic replacement is not
compare-and-swap and does not independently prove persistence of the referenced
objects. Preserve the compare-under-lease, immutable-first, barrier and verification
sequence even when concurrent-read tests pass.

## Barrier qualification remains open

Apple's [fsync manual](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/fsync.2.html)
explains that successful `fsync` can leave data buffered on the drive and points to
`F_FULLFSYNC` for stronger flushing. The current
[disk-write guidance](https://developer.apple.com/documentation/xcode/reducing-disk-writes)
describes `F_FULLFSYNC` as best effort in its iOS discussion. Neither establishes
an unconditional macOS hardware guarantee or the full multi-file/directory
publication protocol here.

A [disposable observation](PS2_MACOS_PROBE_OBSERVATION.md) now records successful
file and directory `F_FULLFSYNC`, exclusive-rename collision refusal and candidate
validation on one local APFS configuration. Its current installed SDK manual also
documents persistence of previously fsynced data on the same device after full-sync.
This is useful additional platform evidence; the full namespace/failure protocol
and provider/admission policy remain unqualified.

Before qualification, document the chosen macOS file and directory operations,
supported descriptors/filesystems, their required ordering, error interpretation
and published platform guarantees. In particular, this plan does **not** assume
that observed directory-call success alone proves namespace persistence, or that
full-sync of an arbitrary file covers other devices or unflushed entries. The
documented same-device guarantee and directory publication semantics must be tied
to the actual protocol and targeted fault tests. Unsupported directory or full-file
barriers block the profile; do not downgrade to process-recovery-only Saved status.

## Failure furnace and acceptance artifact

Run only inside a fresh private temporary root containing synthetic packages,
journal and sibling sentinels. Independently spawned children receive only the
fixture paths/handles they need. A probe may report observed capabilities but must
not install an adapter or enable a writable profile. Do not fill a real volume to
test ENOSPC; inject failures. Mount/remount, destructive power interruption and
hardware fault testing need a separately designated disposable environment and
explicit authorization.

At every `WritePhase`, inject refusal before effect, failure after effect, lost
completion, and process termination before/after the syscall. Include separate
checkpoints around directory barriers, candidate validation and local receipt.
Only classify `NotPerformed` when the specific operation is proven not to have
affected storage; prior attempt effects can still exist. Otherwise report
`OutcomeUnknown`, retain intent and bytes, freeze later writes, and reconcile the
original identities. A failure after HEAD replacement must never become a fresh
publication attempt with new IDs or an unverified Saved receipt.

The acceptance artifact must contain the ordered operation trace, injected event,
exit/error, observed HEAD, reader-verified old/new closure, retained intent/journal,
cleanup inventory and acknowledgment status for every case. An independent opener
must classify exact old HEAD as pending, exact candidate as published only after
required validation/barrier reconciliation, and unrelated/invalid HEAD as frozen.
Data races that violate ownership are refusal evidence, not successful qualification.

Keep three results distinct: syscall/capability observations; deterministic process
crash recovery; and platform/storage durability evidence under a stated fault model.
The first two cannot establish the third. Device firmware, hardware faults and
sudden power loss remain residual risks; report tested configurations and limits
instead of advertising universal APFS durability.

No user decision is needed to continue safe disposable interface tests. Before
production enablement, review the ownership/provider policy, exact barrier evidence
and remaining guarantees; this document does not satisfy that gate.

## First native profile and admission direction — approved 2026-10-03

The user approved this qualification direction and conditional platform/failure-model
reliance on 2026-10-03 for implementation and disposable validation. It is not a
qualified profile, a new portable format or permission
for real-library writes. It uses the [shared-writer clean-remount result](PS2_MACOS_CLEAN_REMOUNT.md#shared-repeatable-rust-path--2026-10-03)
and existing failure/lease evidence; it does not request another fixture family or
repeat those runs. The immediate implementation target is testable autosave on
explicit disposable packages. Production `Accepted`/`Saved` constructors remain
unavailable until the profile and the following engineering conditions are met.

**Approved direction:** permit qualification against the stated macOS platform
persistence contract and bounded failure model below, without treating an abrupt
power experiment as a prerequisite for implementing PS3. This accepts a platform
assumption; it does not assert that clean detach proved power-loss durability.
This approval does not admit a user library, enable deployment, or override an
unknown provider classification. The profile remains unqualified until required
barrier ordering, identity binding, provider classification and configuration-specific
evidence pass qualification. Abrupt-power and hardware-fault experiments remain unauthorized.

| Proposed rule | Concrete interpretation and remaining engineering |
| --- | --- |
| Supported configuration | Start with the observed macOS 27.0.1 / 26A434 adapter build. Bind a report to exact OS/adapter/policy versions, persistent volume UUID, mount epoch, filesystem facts, device/hardware mapping and directory pins. The tested APFS image is an experiment configuration, **not evidence qualifying arbitrary internal/external APFS devices**. Each supported production configuration needs its own recorded mapping and review. Unknown mappings, remount/move or lost event continuity invalidate the lease; no automatic device/inode rebinding. |
| Persistence assumption | Assume the supported kernel/filesystem/device stack honors the documented same-device full-sync contract and atomic same-filesystem rename. Use `fsync` then `F_FULLFSYNC` on written files, and the qualified directory barrier after every new name/rename and newly created ancestor; issue the required final same-device full-sync after namespace barriers. Explicitly review that sequence against the installed SDK contract before declaring the profile qualified. Successful calls alone are insufficient evidence for the namespace guarantee. Refuse unsupported barriers; never substitute plain `sync_all`. Firmware that lies about flush completion, hardware destruction, filesystem/kernel defects and noncooperating writes remain outside the guarantee. This is a conditional platform guarantee, not a weaker process-only meaning of `Saved`. |
| Ownership | Keep the already approved registered cooperative in-place model. The registrar binds host authorization, exact Project/Library/incarnation, manifest/HEAD, parent/root/volume/lock pins, access policy and protocol digest, then holds the stable nonblocking lifetime lock. All Photara-controlled writers/moves/deletes participate. `WriterBusy` is a refusal; no timeout stealing. Private mode, a chooser grant or a raw lock never constructs admission. No managed-root requirement is added to user-selected packages. |
| Provider policy | Preserve **managed or unknown ⇒ read-only** for both package and journal roots. Initial disposable trials may use their separately owned image/controller provenance solely as experiment scope; they cannot mint general provider-exclusion evidence. Production host code must produce version-bound positive evidence tied to the complete path/volume/mount facts. Known File Provider/cloud roots, provider metadata or API uncertainty refuse. “Outside Dropbox,” false iCloud status and absent watcher events are not positive evidence. If available platform APIs cannot establish the proposed positive scope, stop at this evidence gap; admitting unknown paths would require a separate policy change, not a checkbox or silent classifier relaxation. |
| Journal/receipt namespace | Implement PS0's **separate device-local journal store**, under the existing configured local-data root, keyed by device and package incarnation; do not put it in the live catalog or creation journal. Pin/register it independently and qualify its own volume. The package may be on another qualified volume. Bind the journal header to the original manifest/incarnation/base and persist the original checkpoint receipt there. The remount trial's image evidence directory remains historical experiment evidence; it is not this production namespace. No migration, root rename or new journal wire is implied. |
| Native charge | Keep project admission charge distinct from free-space telemetry. Establish a versioned conservative native observation rule covering logical extents, reported allocated storage, rounding, inode/directory metadata, controls, staging and journal coexistence. Never use compressed/sparse/cloned allocation savings as unproved credit. Carry each unit's charged high-water monotonically until the existing exact retirement/absence/barrier proof permits release. Quantify and preflight the worst simultaneous control/directory/journal allowance before effects; the trial's 262144-byte standing allowance and 4096-byte rounding are **fixture parameters, not a qualified universal APFS bound**. If the bound cannot be established or retained, refuse admission. Neither a capacity query nor successful reservation accounting reserves future physical APFS space. |
| Two durability domains | Charge package and local-journal work separately, include both reservations before accepting an operation, and coordinate shared-volume capacity without double credit. Journal rotation/header/index/receipt bytes and their parent directories must be included. Reissue barriers for exact observed original-bound progress after an uncertain result, including an already selected clean HEAD. Any uncertain domain freezes later mutation and preserves the original operation; no fresh planner inputs or IDs. |

The implementation seams are concrete: [`CapabilityProfile`/opaque lease](../../crates/photara-store/src/package/planning/io.rs),
the [version-bound dry-run path classifier](../../crates/photara-store/tests/package_planning/macos_path_classifier.rs)
(its only positive provider evidence is explicitly synthetic), and
[`RepeatableIo`](../../crates/photara-store/src/package/v1_3/repeatable/execute.rs).
The native adapter is still `cfg(test)`: neither its test principal nor its private
registration file is a production registrar. The profile must cover the journal
adapter as well as packed package publication before either acknowledgement is
exposed. `Accepted` remains the exact operation's durable journal result; `Saved`
remains the independently verified selected package plus durable original receipt
for the current accepted revision. An older checkpoint cannot mark newer edits saved.

**Already authorized engineering:** implement the pinned namespace/registration
state machine, host fact collection and fail-closed classifier, journal accounting,
barrier/error plumbing, and PS3 scheduling against the shared Rust authority in
explicit disposable storage; validate new behavior with focused tests. No new
approval is needed merely to implement these conservative rules or to preserve
read-only fallback. Provider evidence, native charge bounds and final registrar
construction are concrete unfinished engineering, not facts supplied by approval.

**Approved scope, remaining evidence gate:** the explicit platform/failure-model
reliance and supported-profile direction are approved; required evidence must still
pass before marking a native profile qualified. Any proposal
to admit provider-unknown paths, weaken `Accepted`/`Saved`, auto-rebind copies/moves,
change retention, or enable real-library writes/deployment remains separate and is
not requested here. Power interruption would also need separate authorization;
the present proposal adds no such experiment. Qualification review can reject the
profile if the required platform evidence is insufficient, even after its policy
direction is approved.

## Conditional disposable qualification — 2026-10-03

The approved PHPSJ001 implementation passed the existing controlled-image trial:
persisted intent → partial package append → clean remount → original retry →
clean remount → completed retry → independent closure/receipt verification.
The image is detached and retained. The exact configuration was macOS 27.0.1
build 26A434, APFS volume `10d00d7d-c3e6-4628-b9b6-df7a8325987f`, under the private
controller root `/private/tmp/photara-ps2-remount-vt9tvk2o`. See the
[qualification evidence](verification/ps2-checkpoint-journal-qualification.json).

The host assessment independently checks the image/mount association, boot and
volume identities, controller generations and pinned handles. Its positive scope
is only this explicitly owned disposable environment. Real-library provider
exclusion and production registration are not inferred from it.

Native checks compare logical extents and reported allocated blocks for every
known package/source/control/journal inode, including namespace entries, against
registered charges. Exact planned coexistence fits the original standing pool;
the pool remains charged and observations repeat after effects/retry. The final
standing footprint is 126,976 bytes within the unchanged 262,144-byte fixture
allowance. The checkpoint reserves 36,864 bytes and observes 32,768 allocated
bytes; its separate namespace/control allowance is 16,384 bytes. These are
fixture registrations, not production defaults or physical-space reservations.
Overruns, ENOSPC and uncertain barriers refuse/freeze while retaining original
intent; no expected deletion or compression saving funds later work.

The barrier review confirms file fsync/full-sync before namespace barriers,
same-device full-sync after namespace changes, exact HEAD replacement, and
reissued file **and directory** barriers for matching preexisting controls.
Intent precedes package effects; the completed checkpoint wrapper follows full
selected-closure validation and package barriers, then its own journal barriers.
There is no weaker fallback and no power-loss claim.

This closes the **configuration-scoped disposable storage qualification** for
the approved checkpoint implementation. It supports progression toward disposable
PS3 autosave after the separately unapproved mutation/undo journal amendment.
It does not qualify arbitrary APFS paths or enable production Accepted/Saved
constructors, real-library writes or deployment. The native harness's general
`qualified`/`saved_claim` flags remain false for that reason. The remaining real-path
provider, authorization and local-root registrar work stays gated from enablement.
