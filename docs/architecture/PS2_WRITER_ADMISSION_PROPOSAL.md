# PS2 writer admission — cooperative ownership proposal

Status: review-only. No compiled contract or production admission changes.
This proposes replacing the unsupported exclusion assertion in
[PackageIo/CapabilityProfile](../../crates/photara-store/src/package/planning/io.rs),
not weakening a check in the existing implementation. The present policy continues
to refuse profiles lacking `excludes_uncooperative_writers`; no production adapter
constructs a qualified profile.

## Audit conclusion

Filesystem capability and writer cooperation are different facts. Apple's
[flock contract](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/flock.2.html)
provides advisory coordination: another process may write without acquiring the
lock. Apple's [file-presenter documentation](https://developer.apple.com/documentation/foundation/nsfilepresenter)
also excludes direct low-level writes from its notifications. Neither API proves
that arbitrary third-party writers cannot access a package. A scan for running
processes, no recent watcher event, a private directory mode, or an app sandbox is
not such a proof either. Same-user software or privileged actors can remain outside
the protocol. Atomic rename cannot close the compare/publication race with them.

The existing workflows provide no stronger admission evidence:

- [Native creation](../../platform/macos/photara-app/Sources/AppModelCreation.swift)
  lets users select destinations. It pins and creates a package; selection and a
  security-scoped resource grant do not assert sole-writer ownership. Completed
  packages remain read-only through
  [ApplicationAdapter](../../platform/macos/photara-app/Sources/ApplicationAdapter.swift).
- [Creation publication](../../crates/photara-store/src/package/creation/filesystem.rs)
  locks its reserved private stage with `.creation-lock` and publishes no-replace.
  That stage lock is not a lifetime lock on the resulting published package.
- [Legacy store replacement](../../crates/photara-store/src/lib.rs) protects its
  separate whole-document route with its own lock/revision protocol. Neither that
  route nor the bridge's in-process mutex participates in a package `.writer-lock`
  protocol. They cannot be advertised as cooperating package editors.

## Recommended admission model

Separate qualified storage primitives from an explicit **registered, cooperative
writer assumption**. Preserve the intended workflow: the user chooses a project
package and edits it at that path. A selected package on qualified local APFS can
be admitted in place after validation, local binding registration and lease
acquisition. Its parent need not be an app-managed root. Network, provider-managed,
unqualified and ambiguous storage stay read-only under the existing PS0 policy.
This is an explicit scope of guarantee, not proof that third-party software is
physically prevented from writing the selected package.

Require exact pinned root/parent/volume/lock identity, valid access policy, positive
package binding registration, provider-policy admission, a compatible cooperating
writer protocol and a held lease. App-controlled rename/move/delete/import tools
must participate too. Missing storage/binding facts refuse admission. Folder names, extensions,
mode 0700, and ownership UID are insufficient on their own. No probing or user
checkbox can turn unknown external writers into a measured filesystem capability.
Do not require an impossible proof that no unknown process exists; admit only
under the stated cooperative guarantee, and refuse known incompatible writers.

In-place admission records the selected package's Project/Library identity,
incarnation, exact manifest/HEAD, locator and volume/root pins in device-local
binding state. It then acquires the stable writer lease and revalidates before
editable activation. The user's open/edit request supplies the intended target;
registration records that choice, not an assertion that they controlled every
other process. No package copy, relocation, catalog identity change or format
conversion is implicit. Reopening a copy, changed locator or replaced inode must
reconcile identity and pending journal rather than automatically inheriting a
binding. The still-undefined registration/rebind recovery contract must be tested
before implementation. Current creation/reopen and legacy-save behavior stay
unchanged by this proposal.

The guarantee would be: cooperating writers serialize; the app refuses detected
identity/content conflicts and preserves recovery evidence; durability receipts
describe the exact verified checkpoint under the qualified storage and cooperative
writer assumptions. Arbitrary noncooperating modifications are outside that
guarantee. They can be undetected in the last comparison/rename gap or overwrite
data later. Do not claim universal lost-update prevention, detection of every race,
mandatory locking, or permanent preservation merely because a receipt once existed.

## Proposed typed boundary (not compiled or a wire format)

```rust
struct StorageQualification { /* private verified primitive/profile evidence */ }
struct RegisteredCooperativeLease {
    // Private: storage qualification, registered package, parent/root/volume/lock
    // pins, package incarnation, protocol compatibility, held OS lock,
    // expected exact manifest/HEAD and the cooperative-policy identity.
}
enum WriteAdmission {
    ReadOnly(AdmissionReason),
    RegisteredCooperative(RegisteredCooperativeLease),
}
fn admit_writer(locator: &LocalLocator,
                policy: &CooperativeAdmissionPolicy) -> Result<WriteAdmission, SessionFailure>;
fn publish(lease: &mut RegisteredCooperativeLease,
           plan: &CheckpointPlan) -> PublishOutcome;
```

On approval, remove `excludes_uncooperative_writers` from *storage capability*
assertions and make publication require this separate opaque admission/lease.
Keep safe handles, exclusive cooperating lock, no-replace publication, atomic HEAD
replacement and file/directory barriers as independently qualified requirements.
No caller-provided boolean grants registered admission. No new profile/format number
is selected here; the existing version-1 interface remains unchanged pending review.

## Conservative failure behavior

A detected pre-publication conflict stops the attempt; it never overwrites the
observed HEAD or silently replans/merges. Preserve intent and any prior immutable
publications. A detection after a possibly effective write is an unknown outcome,
not a claim that the attempt did nothing. Freeze further mutations and reconcile
original write/commit IDs against exact bytes and independent closure validation.
An unrelated valid HEAD is still conflict; never restore the old HEAD over it.
Unknown observer provenance is not proof of a harmless change.

Publish Saved only after the complete qualified protocol and receipt barriers.
If later observation invalidates the admitted binding or shows a different HEAD,
invalidate current writable/Saved state while retaining the historical receipt and
pending journal. Watchers accelerate checks; they cannot make the guarantee apply
to unseen writers. Do not auto-delete an unknown file or suspected writer's lock.

## Viable alternative and its cost

Managed-root-only authoring can reduce accidental overlap with unrelated tools by
restricting registration to an app-controlled location and participating app
routes. It has the same residual same-user/privileged-writer limitation; directory
ownership does not establish mandatory exclusion. A chosen package is writable
in place only if already inside an admitted managed root. Other chosen paths would
remain read-only, so this alternative changes the expected workflow.

Editing an external selection under that alternative would require an explicitly
authorized copy/adoption into managed storage, with a chosen destination, capacity
check, exact validated bytes, interruption recovery, one active catalog locator and
a separately reconciled incarnation/journal binding. Preserve the original without
silently activating two writable copies. Do not infer permission to move/delete the
source or perform format conversion. Those data-movement and identity semantics
need a separate proposal and user consent; this memo does not authorize them.
Recommend in-place registration instead because it preserves user-selected paths
without overstating the protection offered by managed directories.

## Verification and decision gate

Before implementation, revise the normative PS0 wording to scope its no-lost-update
promise to cooperating writers. Then test all own writer/move/delete routes against
the same lease; independent-process contention/death; missing registration and
wrong protocol; in-place admission at user-selected local paths; provider/unknown
storage refusal; changed permission and every pin;
root moves and journal rebind; stale/changed HEAD before and after publication.
Include a deliberately noncooperating writer that demonstrates the remaining race;
that test documents the limit and must not be reported as successful exclusion.
Repeat the [storage failure matrix](PS2_MACOS_STORAGE_QUALIFICATION.md) under the
admitted model; storage barriers and retention remain independent release gates.

**Approval needed before production contract changes:** accept explicitly
cooperative concurrency guarantees with registered in-place editing at user-chosen
qualified local paths, acknowledging that arbitrary noncooperating writers can
race outside the protocol. Alternatively choose managed-root-only admission with
the path/copy tradeoff above. Neither promises exclusion of arbitrary writers.
This changes the scope of PS0's no-lost-update claim; it must not be introduced as
a naming cleanup. If neither model is acceptable, retain read-only package opening
while a different ownership protocol is designed. The decision does not block
remaining disposable PS2 tests or authorize live admission, copying or migration.
