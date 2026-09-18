# LL1 / PS3 / PS4 activation compatibility review

Status: **bounded static review, 2026-09-18; proposal, not acceptance**.
Inspected source at `952c22ce6530ec31a3955f2bd65f7dae35bb777c`. No migration,
DDL execution, production wiring, permanent wire format or runtime durability
qualification is supplied. LL2a remains gated on the mappings and tests below.

Inputs: accepted [PS0 session/switch behavior](PROJECT_SESSION_DURABILITY.md#coordinator-and-native-lifecycle),
its [review-only typed contracts](proposals/ps0/CONTRACTS.md),
[LL1 R4](LL1_TYPED_CONTRACT_AND_SCHEMA_DELTA.md#r4--one-device-activation-receipt-with-ps3ps4),
the [physical proposal](LL1_PHYSICAL_SCHEMA_AND_PRIVILEGE_REVIEW.md), and
the [PS3 synthetic tests](../../crates/photara-store/tests/ps3_shared_coordinator.rs)
and [model](../../crates/photara-store/tests/ps3_shared_coordinator/model.rs).
The existing PS4 contract is the switching section of PS0, not a demonstrated
production coordinator. The PS0 companion's storage shapes remain proposals.

## Finding and one wording correction

One local transaction publishing Library and active Project is compatible with
PS0's atomic activation receipt/active-session requirement. It chooses PS0's local
database alternative, not a second package writer or cross-store transaction.
Freeze is attachment-local; accepted operations and other clients' jobs survive;
failure retains current or explicit recovery. No accepted product behavior needs
to change to realize that direction. The current LL1 physical shape is **not yet
sufficient** to implement it safely.

LL1 R4 currently allows target supersession until freeze. PS0's transaction already
includes Confirmation, and its confirmation token binds target identity and the
pre-dialog saved receipt. Allowing replacement during that stage conflicts with
PS0's serialized switch/close/termination rule. **Proposed LL1 wording correction
for review:** only discovery before activation preparation begins may supersede.
Once preparation/confirmation begins, serialize or reject another target until
the current transaction has a terminal outcome; cancellation must settle before
another preparation starts. A queued request is revalidated, not pre-authorized.
This preserves accepted PS0; allowing in-flight replacement instead would require
an explicit product/recovery-contract decision. This report does not silently
amend R4 or mark its corrected wording accepted.

## Required authority and evidence mapping

| Boundary | Required mapping before LL2a; current gap |
| --- | --- |
| Per-package writer vs GUI activation | Rust package authority retains lease, admission, journal and publication. The client activation coordinator owns one GUI slot's switch. Detaching that GUI cannot call owner-wide exit or stop another client's job. Library/principal selection is not a writer capability. |
| Authority scope and GUI slot | Physical `device_activation` is keyed by authority/principal/device, but PS0 proposes `ActiveSession(device,workspace slot)`. Multiple scoped rows must not each claim the one visible slot. Recommend a stable local GUI activation-slot ID, a per-slot serialized/CAS publication authority, and explicit binding to its authority/principal-scoped selection. Publish slot binding, selection and receipt in the same transaction. Preserve inactive scoped selections only as history/preferences, never simultaneous visible authority. Exact relation/key placement is an unresolved schema decision; do not assume one account forever, or introduce multi-window product behavior. |
| Active Graph and restored view | Map PS0's active Graph and versioned Project/Graph-keyed view to the same committed activation identity. The physical row currently omits Graph/view identity. Require a validated restored-view record/reference in that local publication unit, or a reviewed deterministic safe-default recovery rule. It must never independently select a Project from another activation. Missing selection IDs may use PS0's safe defaults without changing authored bytes. |
| Source save barrier | Physical `barrier_commit_id` + `barrier_sha256` cannot substitute for a PS0 `SavedReceipt`. Bind owner ID/generation, Library/Project/incarnation/bootstrap digest, finite through-sequence, authored revision/digest and Graph coordinates, complete HEAD commit/digest/HEAD-bytes digest/package revision, and qualified capability profile. Persist validated bounded evidence or a versioned immutable reference resolving that exact evidence after restart. A hash without its verified object is insufficient. Codec and storage ownership need review; this report chooses no wire format. |
| Source vs target | Save evidence covers the **source** attachment even when the target is only a Library with no Project. Target opening supplies separate verified identity/HEAD and restoration evidence. First activation without a source Project can explicitly have no source barrier; absence must not be inferred from nullable target Project. Source and target identities cannot share an ambiguous barrier field. |
| Owner/attachment/access/request generations | Keep package owner ID/generation, attachment ID/generation, grant validity/access generation, activation request generation and slot committed generation distinct. Physical `owner_epoch Id` must resolve the full PS0 owner coordinate; one actor counter is not proof of current package grant validity. New owner or new incarnation invalidates stale callbacks, even if IDs or paths otherwise match. |
| Capsule | PS3/PS4 owns bounded device-local capsule bytes, package identity/verified source snapshot, view and reacquisition data. LL1 holds only validated identity/checksum/reference. Capsule durability must precede source detachment. Retain it across activation receipt commit until successful next session establishment; after committed-target failure, explicit target recovery must still have the current-source rollback capsule available. Commit alone is not authorization to dispose it. |
| Commit and recovery | Target is prepared/restored provisionally, not concurrently editable with current. Recheck identity, access and generations, then atomically publish slot/Library/Project/view/receipt. Only after success may UI expose target as active/editable. Before commit recover current or read-only recovery; after commit recover target or explicit recovery, never silently roll back pointer. A failed or unknown response queries the original activation identity before retry. |
| Removal/access loss | Fencing and local cleanup invalidate the matching visible slot and source/target-dependent intents under the same publication serialization. Historical receipts are not current access. Removal cannot delete/read capsule/package files or make retained revoked work editable. A missing capsule after cleanup/restore is explicit recovery, not fresh authority. Source-dependency closure must dereference prior activation before clearing its pointer. |

One transaction means only **local session publication**. No SQLite transaction
spans dialogs, file IO, package leases, cloud calls or target restoration. Prepare
those outside it, then validate bounded evidence and generations during publication.
Unknown package outcomes are reconciled by their package authority first; an
activation receipt cannot upgrade `Accepted` or journal-only durability to `Saved`.

The pre-confirmation save barrier and mandatory post-confirmation frozen flush are
distinct. A finite covered prefix remains valid if another client subsequently
edits; global status may still be Saving. Do not require global writer quiescence
or falsely claim later shared changes are saved in the confirmation. Refresh or
withhold misleading past-tense text as accepted PS0 requires.

Library selection retains LL0 behavior: clean Library switching has no generic
Save confirmation and does not auto-open a remembered Project. Same-current
Library/Project activation stays idempotent; absence of a Project in a Library
selection request must not accidentally become an implicit close of an already
active same-Library Project. Distinguish no-op selection from an explicitly
authorized transition when defining the request contract. Project switching keeps
PS0's native confirmation; sharing a coordinator does not merge the two UX rules.

## Synthetic evidence: useful but not activation acceptance

PS3's 13 test functions model one in-memory owner, logical GUI/headless/agent
attachments, injected append/publication outcomes and a toy `(sequence,value,digest)`
coordinate. Relevant existing checks are
`all_surfaces_share_one_order_queue_and_finite_flush`,
`freeze_is_attachment_local_and_flush_drains_that_clients_pending_input`,
`stop_request_is_not_terminal_completion_or_permission_to_freeze`,
`old_attachment_and_owner_callbacks_never_mutate_replacement_context`, and
`lifetime_lease_requires_verified_drain_for_clean_owner_exit`.

They do not model two packages, Library selection, GUI slots, Graph/view restore,
confirmation, capsules, activation receipts, local SQL atomicity or real locks.
`Saved { covered: Coordinate }` and memory-surviving vectors are deliberately not
production `SavedReceipt` or durable restart evidence. This audit inspected tests;
it did not execute them or extend the fixture. No PS3/PS4 acceptance is inferred.

## Proposed disposable fixtures and pass conditions

All fixture effects below must be fake/injected until separately authorized;
database atomicity and hardware durability need their later dedicated acceptance.

| Fixture | Required observable result |
| --- | --- |
| Pre-preparation A→B→C vs open B confirmation | Discovery B may be superseded; once B preparation starts, C is rejected/queued. B token never activates C. Cancel B settles before C preparation; late B callbacks cannot affect C. |
| Full barrier substitution matrix | Wrong owner, incarnation, bootstrap, capability profile, prefix, authored/Graph digest, HEAD bytes or commit fails before detach/publication. Equal numeric revision or matching commit UUID alone never passes. Missing evidence object behind a hash gives explicit recovery. |
| Two barriers and finite prefix | Pre-dialog save succeeds, then independent agent adds work; GUI cannot imply that later work is saved. Confirm freezes/drains only GUI and verifies its captured finite prefix. Another client continues; global Saving does not itself invalidate a valid covered prefix. Save failure/unknown retains current. |
| Library-only destination | Dirty source Project moving to Library-only target still requires source save/stop/restore-state success. No-source first selection is explicit. Clean Library switch has no extra generic dialog; same-current selection is a no-op, no Project detach or revision advance. |
| Slot/principal arbitration | Two account/local-authority requests contend for the same GUI slot. Exactly one generation/receipt wins; no visible Library A/Project B. Switching principal invalidates late callbacks without deleting the other principal's inactive preference. Different GUI slots remain unenabled unless explicitly accepted. |
| Capsule crash cuts | Fail before capsule durability: do not detach. Crash after detach/before commit: reopen current or retained read-only recovery. Crash after commit/before target establishment: recover target with source capsule retained. Only successful establishment and recovery-owner validation can permit capsule disposal. |
| Publication/restore cuts | Fail target identity/open/view or pointer transaction: no target editable attachment and old committed pointer survives. Lost commit reply recovers same receipt; changed request under same ID refuses. Graph/view matches the winning activation or validated safe default. |
| Owner vs attachment isolation | GUI switch never owner-exits while agent work remains; pending stop is not terminal. Grant revocation/owner replacement between prepare and CAS prevents activation without dropping journal evidence or cancelling unrelated jobs. |
| Removal race | Removal before CAS fences/fails activation; removal after commit fences committed target and clears live slot in cleanup transaction. Source-dependent intents cannot outlive removed source authority. File-effect spy records zero removal-origin reads/deletes of capsules/packages/source files; retained memory is recovery-only. |

## Decision gates

Before LL2a schema acceptance: review the R4 supersession wording correction;
agree GUI-slot arbitration/Graph-view mapping; specify versioned complete source
barrier and distinct target evidence; bind capsule lifetime and generations to
PS3/PS4 ownership. These are unresolved contract/schema mappings, not permission
to infer them from the toy model. Choosing an active-pointer authority outside the
same local transaction, replacing in-flight confirmation, weakening capsule
retention or changing LL0 confirmation/file-isolation behavior requires explicit
architecture/product approval. No such change is recommended here.

Then authorize the disposable cross-slice fixtures and review their evidence.
PS3 production durability, PS4 native switching, LL2 schema/implementation and
rollout remain separate gates; this report closes only the bounded static audit.
