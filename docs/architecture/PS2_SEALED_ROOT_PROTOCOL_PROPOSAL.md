# PS2 sealed-root protocol — review-only proposal

Status: proposed protocol, not an approved production codec or implementation.
The [retention direction](PS2_RETENTION_STORAGE_DECISION.md) and
[cooperative admission direction](PS2_WRITER_ADMISSION_PROPOSAL.md) are approved.
This document selects no production format number, feature identifier, threshold,
migration, deletion or live-package operation.

The [resource/storage semantic boundary](RESOURCE_STORAGE_AND_VERIFICATION_CONTRACT.md)
was approved 2026-09-17 for the next positive fixtures. Its distinctions below
are approved semantics; exact root encoding, feature identifiers and production
implementation remain proposed.

## Recommendation and alternatives

Keep `HEAD.json` as the sole authoritative atomic dispatch point. It selects one
immutable, feature-gated root-set commit. Keep `manifest.json` byte-identical.

| Approach | Benefit and cost |
| --- | --- |
| Existing HEAD selects a new root-set commit — recommended | Reuses exact HEAD tokens, commit IDs/hashes and publication machinery. Requires explicit new commit semantics and equality checks against the selected state root. |
| New HEAD schema directly selects roots | Also permits one atomic switch and old-reader refusal, but changes the outer dispatch codec and existing HEAD integration without an established need. |

The current [1.1 reader](../../crates/photara-store/src/package/v1_1/reader.rs)
binds every commit to the exact bootstrap hash and walks to revision-1 genesis.
An optional marker cannot change that contract. Rewriting bootstrap and HEAD as
separate ordinary writes would introduce an unsafe two-file cutover.

## State roots and dispatch

Names below describe logical roles, not reserved wire fields or schema names.

A sealed state root binds portable Project/Library identity, bootstrap digest,
authored coordinate, authored/history references, exact inventory and checkpoint
inclusion evidence. Its complete state closure validates without fetching any
predecessor. Host incarnation, attachment identity and credentials are not authored
package data; device recovery bindings remain separate.

A root-set commit selects:

- One active sealed state root.
- One independently verifiable recovery root.
- Additional roots pinned by explicit history, unresolved recovery/undo or
  conversion-source retention.

Retain legacy-decodable commit header fields and declare the new required feature
and minimum-reader boundary. Old readers must refuse before interpreting truncated
ancestry as valid. The new validator requires the explicit root-set discriminator
and supported feature together; inconsistent or missing declarations refuse.
Legacy-shaped authored/history/inventory references must exactly match the active
state root. A passing optional marker alone never selects the new semantics.

Package revisions remain checked and monotonic. Compaction alone does not advance
authored revision. For the first positive fixture protocol, emit a self-contained
state root per checkpoint and coalesce edits in the journal; do not introduce a
second delta-chain format or infer production checkpoint timing from this choice.

A state root must not recursively require its former recovery roots. Otherwise
retaining one previous root retains every generation. Predecessor commitments
record provenance, not mandatory traversal or availability of discarded bytes.
Retiring an old dispatch record cannot invalidate a retained state root.

## Keep-set and validation

Validate each selected root independently, then require the retention inventory
to equal the union of its schema-defined dependencies. Account separately for
dispatch, inventory, journal/index and attempt-owned staging metadata.

Preserve current authored content, explicit history, captured source-authored and
Graph/context/input snapshots, retained resource-version records and required
embedded managed blobs, external-backing obligations, evidence/artifacts, opaque
extensions, and all unresolved recovery/undo references.
Current optional fields remain byte-preserved; reference-looking opaque JSON must
not be reinterpreted as filesystem edges. Unknown required semantics refuse.

Keep dependency classes explicit: required package-resident objects; authoritative
resource/version/location/backing/evidence records; and separately retained
external byte backings. External descriptors remain descriptors: retaining them
does not copy or hash their media at each checkpoint. Validate the exact package
closure independently of live external availability. A required embedded object
missing from that closure is package damage; a previously durably published and
verified retained backing on an offline store is unavailable with preserved
retention evidence, not automatically a broken promise. Confirmed loss/corruption
or definite failure to establish required backing makes retention unsatisfied only
when an active obligation cannot be met by the remaining qualified backings.
Ambiguous lookup and unknown publication outcomes require reconciliation.

Captured Version is immutable identity/evidence, not an indefinite retention pin.
Authored use/history promising reconstruction, recovery roots, explicit policy,
pending operations, active leases and other declared obligations pin backing.
After all obligations end it may become eligible for conservative retirement;
identity/provenance can remain without promising byte availability. Descriptor-only
history and predecessor commitments do not recursively pin all media versions.
Package-object retirement and Asset Store retirement are separate operations;
neither may infer permission to remove user-owned sources. A predecessor hash or
archive pointer alone is not a recoverable backing.

Only registered objects proven unnecessary to every retained or pending reference
can become retirement candidates. Age, filename or one stale reachability scan is
insufficient. Unknown files remain untouched. This bounds obsolete autosave
ancestry, not user-authored content growth; capacity exhaustion requires refusal
and recovery options, not deletion of authored content.

Checkpoint reserve covers package closure, journal/index, recovery and staging,
including newly embedded bytes. External-media descriptors do not reserve or
rehash their entire media payload at every root turnover. Asset publication has
its own target-store reserve; shared physical volumes need combined accounting
so concurrent reservations do not spend the same free space twice.

## Publication and recovery

Use the shared Rust authority and qualified cooperative lease from
[PS0](PROJECT_SESSION_DURABILITY.md). One finite accepted journal prefix supplies
the checkpoint target; later accepted work remains recoverable separately.

1. Persist an intent binding exact old HEAD, original write/commit IDs, new root-set
   bytes, retained-root manifest and journal inclusion evidence.
2. Publish immutable roots, dependencies and root-set commit no-replace. Complete
   required file/directory barriers and validate the entire candidate keep-set.
3. Recheck ownership, pins and exact old HEAD; atomically replace only HEAD.
4. Complete root-directory barriers, reopen and validate the selected package,
   then durably record its acknowledgement and recovery/dedupe transition.
5. Only afterward consider retirement. Serialize or revalidate eligibility against
   later mutations so newly retained references cannot race with cleanup.

| Interrupted state | Required result |
| --- | --- |
| Before dispatch | Old package remains selected; retain intent and staged objects. |
| Unknown dispatch, barrier or receipt | Freeze and reconcile original IDs; no fresh-ID retry or cleanup. |
| Exact intended dispatch | Verify inclusion and reconstruct missing acknowledgement. |
| Unrelated HEAD, changed identity or rollback | Preserve evidence and enter conflict/recovery; never overwrite automatically. |
| Interrupted retirement | Preserve complete active/recovery closures; resume only from verified eligibility evidence. |

Successful syscalls and process-exit tests do not establish power-loss durability.
The [storage qualification gate](PS2_MACOS_STORAGE_QUALIFICATION.md) remains open.
Managed external publication also crosses the package/store authority boundary:
persist intent, stage/verify and durably publish backing before committing its
descriptor/obligation. No single atomic rename spans the two. Reconcile unknown
outcomes under their original operation IDs and preserve recovery-critical staging.
See the [publication contract](RESOURCE_STORAGE_AND_VERIFICATION_CONTRACT.md#publication-relocation-and-capacity).

## Dedupe and optional conversion

Preserve operation-ID/intent-digest/outcome and original credential-free provenance
independently of package ancestry and undo eviction. Inclusion binds the exact
journal prefix and its resulting state, not merely a numeric sequence.
Revocation does not erase accepted intent or authorize duplicate execution.

Unconverted 1.1 packages use the existing reader and remain byte-identical on open.
Conversion is opt-in. Stage the new root set while retaining a complete original
1.1 representation: original HEAD/bootstrap and all required ancestry/objects.
Keep that conversion source separately pinned beyond the rolling recovery root
until its retirement policy is explicitly approved. Its exact representation and
capacity accounting must be specified before conversion is offered.

After the final HEAD switch, old readers refuse the active converted package.
This is not a promise that old readers can open its new state. Restoring older
content or exporting the original requires an explicit action preserving newer
evidence; never silently rewind HEAD to a superseded dispatch record.

## Acceptance and approval boundary

The existing [boundary fixtures](../../crates/photara-store/tests/package_planning/retention_boundary.rs)
prove unchanged legacy reads and negative reader-boundary behavior, not this
positive protocol. The [positive fixture evidence](PS2_POSITIVE_SEALED_ROOT_FIXTURES.md)
now models independent roots, complete keep-set equality,
multiple turnovers, exact dedupe/inclusion, interrupted conversion, and the
defined publication/retirement cuts, including capacity exhaustion and concurrent
work. This is fixture evidence, not production reader or durability qualification.

The approved [resource/storage fixture matrix](RESOURCE_STORAGE_AND_VERIFICATION_CONTRACT.md#positive-fixture-acceptance-and-deferred-work)
also requires zero external media reads on root turnover, synthetic 8 GB working
changes and weak-observation uncertainty, retained V1 versus edited V2, finite
capture pinning/retirement eligibility, offline versus proven retention loss,
embedded-object damage, placement fallback/refusal, slot retarget without history
retargeting, same-version verified relocation, cross-store recovery, independent/
shared-volume reserves, and package-only versus complete-backup claims. Use
fixture models/read counters rather than real media allocation or production GC.

No additional product decision blocked the clearly fixture-only positive model.
Before production codec implementation, approve the exact root/feature contract,
dispatch validation, conversion-source representation and retirement/rollback
semantics. Existing directional approval does not reserve a wire format or
authorize migration, deletion, production thresholds or live-package writes.
