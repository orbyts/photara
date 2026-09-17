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
be inferred from a lock test. The compiled interface still refuses profiles without
that assertion; no production adapter constructs one. Detecting a
change after publication does not undo a race already lost to an uncooperative
writer. The approved [writer admission direction](PS2_WRITER_ADMISSION_PROPOSAL.md)
separates registered in-place cooperative admission from storage qualification,
without requiring a managed root. All controlled writer surfaces use one Rust
authority; another direct process gets WriterBusy while the lifetime lease is held.
This revises the proposed contract, not the compiled policy or qualification result.

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
