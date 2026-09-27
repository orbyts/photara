# LL2a field, authority and session dependency review

Status: **documentation-only mapping for review, 2026-09-27**. No schema, codec,
production endpoint, local pointer or default is changed. This narrows the
[LL1 readiness audit](LL1_READINESS_AUDIT_20260927.md) to the first cross-Library
acceptance; guarded removal remains LL2b.

Inputs are the accepted [LL0 behavior](../LIBRARY_LIFECYCLE.md),
[typed command proposal](../LL1_TYPED_CONTRACT_AND_SCHEMA_DELTA.md#proposed-commands-results-and-durable-state),
[current R4 recommendation](../LL1_TYPED_CONTRACT_AND_SCHEMA_DELTA.md#r4--one-device-activation-receipt-with-ps3ps4),
[physical proposal](../LL1_PHYSICAL_SCHEMA_AND_PRIVILEGE_REVIEW.md), and accepted
[PS0 coordinator behavior](../PROJECT_SESSION_DURABILITY.md#coordinator-and-native-lifecycle).
This table maps their semantics; it does not allocate new schema fields.

## Commands and durable ownership

| Command or boundary | Required identity and authority | Durable owner and retry | Dependency before LL2a implementation |
| --- | --- | --- | --- |
| Create Library | Explicit cloud environment or local database; verified initiating account/local principal and device; proposed new Library ID; canonical name; immutable operation/request | Service transaction creates Library/contract/owner membership/empty stream/receipt, or local transaction creates Library/controller/receipt. Persist exact request before dispatch; lost reply queries the original operation. Cloud create entitlement is checked at service dispatch. | Review concrete create envelope/receipt and physical delta. Existing removal fixtures do not supply create entitlement. No Project/package binding or selection change is implied by creation. |
| Rename Library | Explicit authority/Library; current owner or local controller; expected Library revision; canonical new name; immutable operation/request | Rename/revision/inventory event/original receipt share the authority transaction. Revision conflict is a fresh draft; unknown outcome retains original request. | Review concrete rename envelope/receipt and CAS/admission mapping. Names trim once, contain 1–128 UTF-8 bytes and no controls; preserve case and Unicode sequence. IDs, package paths and defaults remain unchanged. |
| Select Library | Accessible target, expected committed selection generation, new request generation, current actor/access and session context | Device-local session coordinator owns selection publication. Selection has no cloud lifecycle write and does not change defaults. Same-current selection is idempotent and must not implicitly close its active Project. | PS3/PS4 source resolution and target admission; reviewed slot/selection/activation schema. Clean Library selection has no generic Save confirmation and does not auto-open a remembered Project. |
| Activate Project | Exact target Library/Project/package identity, current access, request/attachment/owner coordinates and validated target Graph/view | Rust activation coordinator publishes the local activation result only after preparation and final rechecks; package authority remains responsible for journal/lease/flush. | Qualified PS2 Saved evidence through PS3, then PS4 open/restore and native failure behavior. A local selection transaction cannot upgrade journal-only Accepted to Saved. |
| Query original outcome | Exact authority/principal/operation and request hash; fresh eligible actor authentication | Immutable original result, including recovery from a newly eligible device where the command contract permits it. Changed bytes under the same ID conflict. | Review bounded lifecycle codec and recovery routing. A transport error or concurrent NotFound is not proof no command can commit. |

The existing physical draft's `lifecycle_intents` and `lifecycle_receipts`
represent the lifecycle request/result roles. Its `activation_intents`,
`activation_receipts` and `device_activation` represent local session roles.
These are unapproved proposed relations. Their independent identity scopes must
not be merged merely because each uses an operation ID or revision.

## Remaining activation mapping decisions

| Proposed physical gap | Required reviewed mapping | Preserved behavior |
| --- | --- | --- |
| Scoped `device_activation` can have multiple principal rows | One visible GUI slot needs a serialized/CAS publication owner and a binding to the selected authority/principal scope. Review exact keys/relations; do not infer a one-account lifetime or add multi-window behavior. | Only one winning slot activation; inactive scoped preferences are not simultaneously visible authority. |
| Graph/view absent from the proposed active row | Bind restored Graph and versioned Project/Graph view to the same activation result, or specify the already permitted deterministic safe default with validated identity. | View state cannot independently select another Project or mutate authored bytes. |
| Two abbreviated barrier columns | Resolve a complete bounded PS3 SavedReceipt or an immutable typed reference to its exact evidence: owner, incarnation/bootstrap, finite covered prefix, authored/Graph coordinates, HEAD bytes/commit/package revision and qualified profile. | Matching UUID/revision or an unresolved checksum is insufficient. Pre-confirmation and frozen post-confirmation flush are distinct; unrelated client work need not globally quiesce. |
| Ambiguous source/target evidence | Represent source save evidence separately from target identity/open/restoration evidence; explicitly represent first activation with no source. | Library-only target can still require source Project flush; nullable target Project never means no source barrier. |
| Collapsed owner/access/request counters | Distinguish owner and attachment generations, current grant/access generation, requested activation and committed slot generation. | Revoked or stale callbacks cannot mutate a replacement attachment even if IDs/paths match. |
| Capsule reference without reviewed lifetime | PS3/PS4 owns bounded capsule bytes and recovery authority; LL1 stores only validated identity/checksum/reference. Durability precedes detach; retain through successful next-session establishment. | Receipt commit alone never authorizes capsule disposal; before commit recover current, afterward recover target or explicit recovery with source capsule retained. |

Preparation, dialogs, package IO, leases, cloud access and restoration happen
outside the short local publication transaction. That transaction rechecks the
bounded evidence and generations and commits slot/Library/Project/view/receipt
together. Only then does the UI expose the target as active/editable. A lost local
reply queries the original activation identity; it does not issue a fresh switch.

Current R4 already permits supersession only during discovery before preparation
or confirmation. Once preparation starts, serialize or reject another target until
the request settles. The older compatibility review's contrary description of R4
is historical. No new approval is needed to keep the accepted PS0 serialization
rule; the unresolved work is its concrete schema/evidence mapping.

## Review boundary and next step

Review the exact mappings above with the PS3/PS4 persistence proposal, current
schema/API floors and bounded codecs before selecting physical DDL or implementing
production lifecycle routes. The older proposed numerical floors are tied to their
inspected baseline and must be rechecked; this document reserves none.

A useful later disposable test would exercise the reviewed local publication unit
with real SQL transactions, injected package evidence and restart/query cuts.
Repeating the existing in-memory activation model would not prove that boundary.
Production PS2 → PS3 → PS4 still precedes wiring and the specified signed
cross-Library acceptance. No current finding requires expanding that acceptance to
Library deletion, another Mac or additional Projects per Library.
