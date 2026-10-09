# LL2a field, authority and session dependency review

Status: **UNAPPROVED concrete local-first review packet, 2026-10-09**. The original mapping below
is retained; the dated amendment identifies where approved PS4 now supersedes
its assumptions. No schema, codec,
production endpoint, local pointer or default is changed. This narrows the
[LL1 readiness audit](LL1_READINESS_AUDIT_20260927.md) to the first cross-Library
acceptance; guarded removal remains LL2b.

Inputs are the accepted [LL0 behavior](../LIBRARY_LIFECYCLE.md),
[typed command proposal](../LL1_TYPED_CONTRACT_AND_SCHEMA_DELTA.md#proposed-commands-results-and-durable-state),
[current R4 recommendation](../LL1_TYPED_CONTRACT_AND_SCHEMA_DELTA.md#r4--one-device-activation-receipt-with-ps3ps4),
[physical proposal](../LL1_PHYSICAL_SCHEMA_AND_PRIVILEGE_REVIEW.md), and accepted
[PS0 coordinator behavior](../PROJECT_SESSION_DURABILITY.md#coordinator-and-native-lifecycle).
The original tables map semantics; the final dated proposal supplies unapproved exact fields.

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

## 2026-10-08 narrow amendment — UNAPPROVED

This is a proposed **Library** create/rename packet, not a new Project creation
protocol. UI1 Project creation and its receipts remain unchanged. The completed
PS4 disposable same-Library acceptance supplies the source-save, rollback,
original-ID and target-open behavior. It does **not** approve cross-Library access,
SQL activation publication or general filesystem admission. No code, SQL, live
database, migration ordinal, endpoint or rollout is authorized here.

### Exact proposed create/rename wire

These spellings are review candidates. Use `photara.canonical-json.v1`, canonical
nonnil UUIDs, lowercase SHA-256 and canonical decimal strings. Counters fit signed
SQL bigint; positive revisions start at `"1"`. Reject duplicate/unknown fields,
unknown variants and overflow. Each request/receipt is at most the already
proposed 65,536 canonical bytes; names retain LL0's trim-once, 1–128 UTF-8-byte,
no-controls rule. No extension object or credentials occur in these records.

| Value | Exact proposed members |
| --- | --- |
| `Authority` local | `{kind:"local",database_id:Id,epoch:Id}` |
| `Authority` cloud | `{kind:"cloud",environment_id:String,epoch:Id}`; environment uses the existing 1–255-byte registered identifier, never a caller-selected URL. |
| `Principal` | `{kind:"local"|"account",id:Id}`; local authority requires local principal, cloud requires account. Verified actor/device comes from the existing authority boundary, not this claim. |
| `CreateLibrary` request | `{schema:"photara.library-lifecycle.v1",kind:"create",authority:Authority,principal:Principal,operation_id:Id,initiating_device_id:Id,library_id:Id,name:LibraryName}` |
| `RenameLibrary` request | Same exact common fields with `kind:"rename"`, plus `{expected_revision:Decimal,name:LibraryName}`. `library_id` is the existing target. |
| Terminal receipt | `{schema:"photara.library-lifecycle.v1",kind:"receipt",authority:Authority,principal:Principal,operation_id:Id,initiating_device_id:Id,library_id:Id,action:"create"|"rename",request_sha256:Digest,committed_at_ms:Decimal,result:Result}` |
| `Created` result | `{kind:"created",revision:"1"}`. No selection, package binding or default reassignment follows. |
| `Renamed` result | `{kind:"renamed",previous_revision:Decimal,revision:Decimal}`; previous equals the request CAS and revision is exactly previous+1. |
| Definitive rejection | `{kind:"rejected",reason:"revision-conflict"|"library-id-unavailable"|"authority-denied"}`. These are authenticated, authoritative terminal decisions only; no name, current hidden revision or existence detail. |

Proposed request digest: SHA-256 of ASCII
`photara.library-lifecycle.request.v1`, one NUL byte, then the exact canonical
request. Proposed receipt digest uses ASCII
`photara.library-lifecycle.receipt.v1`, one NUL byte, then canonical receipt.
Neither digest is a signature or authorization token. These domains are new
review choices; existing onboarding, Project, PS1 and PHPSJ001 domains do not change.

Idempotency key is `(authority,principal,operation_id)`, **not device ID**. Persist
canonical request before dispatch. Authenticate before original-receipt lookup;
same key/different hash refuses, same hash returns original immutable receipt even
after later rename. A newly eligible device may retrieve the initiating principal's
receipt; it cannot execute a changed request. Malformed/unsupported requests and
unauthenticated calls produce protocol/auth errors, not fabricated durable receipts.
Transport failure is unknown, never terminal rejection. Query the original key/hash;
NotFound does not cancel a concurrent dispatch. Fresh device/session credentials
are external to immutable request bytes.

In one authority transaction, create checks current entitlement/controller,
nonreuse and current admitted contract defaults, then creates Library, contract,
owner/controller, empty stream where applicable, inventory event and receipt.
Rename checks current owner/controller plus revision, then changes name/revision,
inventory event and receipt atomically. Duplicate display names remain allowed.
Neither operation receives a package-open capability. The request deliberately has
no caller-selected term policy, billing plan or owner grant: their authoritative
source must be pinned in the reviewed executor, not silently copied from bootstrap.

### Current floor and smallest schema review delta

Source rechecked 2026-10-08: local has 15 generation-two migrations, latest
[`0015_project_creation.sql`](../../../crates/photara-library/migrations/generation_two/0015_project_creation.sql)
sets 5/5; the [activated local opener](../../../crates/photara-library/src/gen2/mod.rs)
requires 5/5. Service has 14 migrations; latest
[`0014_onboarding.sql`](../../../crates/photara-service/migrations/postgres/0014_onboarding.sql)
sets API 3 and [runtime admission](../../../crates/photara-service/src/runtime.rs)
requires 3. Schema families remain `photara.local.g2` / `photara.service.g2`, epoch
1. Therefore LL1's proposed **6/6, API 4, lifecycle capability** is still numerically
available, not installed or reserved. No package reader floor changes from this
SQL/control proposal. Negotiate only implemented command variants; a lifecycle
capability must not imply that LL2b Remove is enabled.

The [physical draft](../LL1_PHYSICAL_SCHEMA_AND_PRIVILEGE_REVIEW.md#sqlite-relation-delta)
already defines bounded `lifecycle_authorities`, `lifecycle_intents` and immutable
`lifecycle_receipts`, including operation keys and no live Library FK on evidence.
For LL2a, review exact create/rename CHECK variants, original-hash uniqueness,
owner/controller authority and account inventory publication against those shapes.
Local intent must precede cloud dispatch; cloud receipt and local projection are
separate transactions, with visible projection-pending recovery. Local-only
creation/rename uses one SQLite authority transaction. Preserve onboarding and
UI1 rows/bytes; do not repurpose `project_creation_intents` or mutate defaults.

Service work requires explicit create/rename/query facade signatures, authenticated
principal routing and exact grants/RLS for new private receipt/inventory rows.
The draft's removal SECURITY DEFINER executor, retirement permits, FK rebuilds,
delete-guard exceptions and aggregate deletion are **outside LL2a**. No broad
privilege grant is justified by create/rename. Full generated DDL/grants and
fresh/upgrade checks remain required before schema acceptance; this document is
not executable DDL.

### SQL selection versus approved PS4 publication

The earlier R4 recommendation here assumes SQLite publishes activation. Approved
[PS4](../PROJECT_SESSION_DURABILITY.md#ps4-local-activation-mapping--approved-2026-10-08)
instead publishes exact snapshot bytes by file replacement. **Do not implement
both as independently authoritative pointers.** Package/journal durability also
does not qualify a SQLite database, WAL or its directory.

Smallest proposed SQL option: retain PS4's canonical record graph and validator,
but install one protected SQLite slot row keyed by `(device_id,workspace_slot_id)`
containing `revision`, `snapshot_canonical`, `snapshot_sha256`, plus the selected
authority/principal/Library/optional Project columns needed for indexed access.
Every indexed column must be derived from the selected validated record graph;
callers cannot update it separately. One short transaction compares old snapshot
hash/revision, checks current Library/access/projection and request/attachment
generations, and publishes exact candidate bytes, derived selection and original
receipt **together**. A separate receipt index, if used, is a checked projection of
retained immutable records. No second file-active pointer is written. This is a
new proposed storage mapping, not a transparent reuse of PS4's file authority.

Preparation, pre-dialog Saved, confirmation, mandatory second flush, durable
capsule and provisional target opening remain outside the SQL transaction and
preserve the approved order. The source stays selected until commit. Unknown
COMMIT is resolved by reading exact old/candidate snapshot and original receipt
through the same registered database; old permits original retry, candidate
requires target reopen or explicit recovery, and neither may silently roll back a
committed selection. Other bytes/conflicting generations refuse. Missing selected
state never becomes an empty slot. SQLite IO uncertainty must reestablish its
reviewed durability/registration evidence before exposing Activated.

An existing PS4 file snapshot cannot be silently imported or superseded. The
smallest disposable SQL acceptance starts a fresh, independently registered slot;
migrating an existing file-authoritative slot needs a separately reviewed exclusive
handoff/restart rule. Retaining PS4 file authority and making SQL only a recoverable
projection is an alternative, but would amend R4 and require exact projection
reconciliation; this packet does not choose it by accident.

## 2026-10-09 recommended decision — fresh local SQL slots

This recommendation supersedes the open alternatives in the 2026-10-08 mapping
for the initial local-first slice; those paragraphs remain review history.

**Recommendation, not implementation approval:** choose SQLite as the sole
selection publisher for a fresh registered slot. Implement two genuine local
Libraries, one disposable Project each, under the current verified local
controller. Use the **normal signed app shell and avatar Library menu**, with the
same Rust durable session as PS3/PS4. A separate controlled scene or manual
`project.save()`/context-clear route does not satisfy this acceptance. Signing is
not cloud authorization. Cloud create/rename/selection, account changes, existing
slot migration and real-library writes are outside this initial packet.

This recommendation resolves the publication alternative above in favor of R4.
Frozen PS4 file snapshot v1, its reader and all package/journal bytes remain
unchanged. No v1 file is imported, overwritten or simultaneously maintained as an
active pointer. The SQL slot is independently registered before its first use.

### Proposed exact selection snapshot v2

Use the existing canonical JSON/hash/ref/UUID conventions and bounded parser.
The envelope is exactly `{format:"photara.local.activation-snapshot",version:2,
body:B,body_sha256:Digest}`. `B` is exactly
`{device_id,workspace_slot_id,revision,request_generation,committed_generation,
slot_scope_sha256,active:Ref|null,pending:Ref|null,records:[Record]}`.
`slot_scope_sha256` binds the immutable independently registered database/slot
scope, **not the selected Library**. Changing Library changes selected records,
never this registration hash. Records retain `{id,kind,version,body}`, unique ID
sorting, complete Ref resolution and immutable bytes. Counters fit signed SQL
bigint. Unknown fields/versions refuse. No numeric product limit is introduced:
parser, retained-record and database/coexistence budgets are explicitly registered
for the disposable instance and checked before effects, without growth or pruning.

Aliases below are exact field-set definitions, not permission to accept additional
members. `Authority`/`Principal` retain the proposed lifecycle definitions above;
this initial selection decoder admits only local authority for the registered
`database_id`/epoch and the verified local principal. Cloud/account variants refuse
before preparation. Existing v1 `SessionView`, `SavedProof` and `TargetOpenProof`
records keep their exact bytes/meaning. All other kinds in this table are **v2**;
no old authority record is reinterpreted.

| Type / record | Exact proposed body |
| --- | --- |
| `Context` (inline) | `{authority:Authority,principal:Principal,library_id:Id}`. Source context is read from selected state, never supplied as a replacement by the request. |
| `Target` Library (inline) | `{kind:"library",context:Context}`. No remembered Project, Graph or fake target SavedProof. |
| `Target` Project (inline) | `{kind:"project",context:Context,project:ProjectTargetV1}`. `project.library_id` must equal `context.library_id`; all original registration/Graph identity checks apply. |
| `LocalContextProof` | `{context:Context,library_revision:Decimal,contract_revision:Decimal,authorization_generation:Decimal}`. Values come from actual active Library and local-only contract rows under transaction. Historical observation only; publication/reopen checks live controller equality again. |
| `ConfirmationEvidence` | `{activation_id,device_id,workspace_slot_id,request_generation,expected_committed_generation,slot_scope_sha256,source_active:Ref,source_binding:SessionBinding,accepted:AcceptedCoordinate,prior_saved:Ref,target:Target}`. Exact current pre-dialog Saved is mandatory. |
| `ActivationIntent` | `{device_id,workspace_slot_id,request_generation,expected_committed_generation,slot_scope_sha256,source_active:Ref|null,target:Target,source_attachment:SessionBinding|null,confirmation_basis:Ref|null}`. ID is original activation ID. Null attachment means the actual source has no Project; it does not bypass a source Project barrier. |
| `RollbackCapsule` | `{source_active:Ref,source_saved:Ref,view:Ref,graph_snapshot:CoreGraph,registration_sha256:Digest}`; same full Graph preservation as PS4, but source points to v2 ActiveSelection. |
| `SelectionReady` | `{intent:Ref,context_proof:Ref,project_open:Ref|null}`. Project targets require actual v1 TargetOpenProof and fresh Project access/registration/lease checks; Library-only targets require null. |
| `ActivationProgress` | `{intent:Ref,stage:Preparing|Confirmed|Frozen|SourceSaved|TargetReady|Refused,source_saved:Ref|null,capsule:Ref|null,target_ready:Ref|null}`. References remain monotone through the same original attempt; no stage publishes selection. |
| `ActiveSelection` | `{device_id,workspace_slot_id,committed_generation,slot_scope_sha256,activation_id,target:Target,ready:Ref}`. Project view resolves through ready→project_open→view; Library target has no Project view. |
| `ActivationReceipt` | `{intent:Ref,outcome:Activated|RetainedCurrent|RetainedReadOnlyRecovery,old_committed_generation,new_committed_generation,source_saved:Ref|null,capsule:Ref|null,target_ready:Ref|null,active:Ref|null}`. Activated increments once; retained outcomes preserve selection/generation. “Activated” includes Library-only selection; it does not grant package authority. |

Context access is **not** authorized by these stored versions. Reuse live
`local_authority` for the exact target controller, and existing `project_authority`
for the requested Project access, active ownership/association and package
registration. Recheck actual grant/policy state inside final SQL publication;
reacquire through the registrar on reopen. No caller eligibility boolean, scalar
generation or saved proof can supply authority. Same selected Library is a no-op
that leaves its current Project open; same exact Project is also idempotent.

### Transition and interaction rule

Project→Project follows PS4 exactly: resolve pending work; pre-dialog current
Saved; block editing through confirmation; then Confirmed→Frozen→SourceSaved→
TargetReady→atomic activation. Both source barriers, exact attachment/accepted
recheck and durable full capsule remain required. Stale repeated prepare refuses
its original confirmation instead of rebinding to a new owner epoch.

Library menu selection obeys LL0: **no generic Library Save or clean-switch
confirmation**. With a source Project, drain pending work and reach verified Saved;
unsaved/run resolution uses its existing owner, and failure retains source. Then
Preparing→Frozen→SourceSaved→TargetReady, with no ConfirmationEvidence. The
post-freeze finite barrier/recheck and capsule still precede detach. The target
kind supplies this stage distinction; it is not a caller “skip confirmation” flag.
With an empty or Library-only source, there is no Project barrier/capsule:
Preparing→TargetReady after live target validation. These no-Project stages also
apply when opening a Project from a selected Library-only context. No automatic
remembered-Project activation follows Library selection.

In all routes, TargetReady requires validated/restored target and source dependency
checks; no visible title/Library/Graph changes early. Before commit, cancel/error
recovers old selection or its explicit read-only capsule. After commit, recover
target or explicit recovery, never rewind to source. Preserve every immutable
receipt/capsule; unknown outcome queries original ID. Unknown IDs cannot reacquire
or rotate ownership. A terminal ID with changed target conflicts. Dirty work and
failed Close retain the existing PS3 Retry behavior; no discard/expiry policy.

### Exact fresh SQLite publication proposal

Add one STRICT relation `local_activation_slots` with the following columns;
this is an **unnumbered schema specification**, not executed DDL:

| Columns | Type / constraint |
| --- | --- |
| `device_id`, `workspace_slot_id` | BLOB16 nonnil; composite PK; device FK to registered `local_device(device_id)` RESTRICT. |
| `slot_scope_sha256` | BLOB32, immutable and equal independent registration. |
| `revision`, `request_generation`, `committed_generation` | INTEGER >=0; equal exact parsed B values; overflow refuses. |
| `snapshot_canonical`, `snapshot_sha256` | BLOB canonical complete v2 envelope and BLOB32 SHA-256 of those complete bytes. Body digest is separately validated. Byte bound comes from registered fixture limits. |
| `active_library_id`, `active_project_id` | Nullable BLOB16; null/null only when active=null; Project implies Library. Exact checked projection of active.target, never separately writable. FK Library→libraries RESTRICT; paired Project FK→project_ownership(library_id,project_id) RESTRICT. |

No separate mutable activation-receipt or scoped selection row is needed: complete
immutable receipts and authority/principal live in the canonical graph. The slot
PK deliberately excludes principal so two authority-scoped rows cannot both claim
the GUI. Existing `onboarding_library_selection` remains bootstrap evidence,
not a second current pointer; the new normal-app route reads only this slot.
Registration provisions exact empty v2 bytes once. Missing selected row is an
error, never implicit initialization. No ordinary delete/reset API is exposed.

Use a pinned local SQLite connection with foreign keys enabled; for the first
fresh disposable proof recommend DELETE rollback journal, `synchronous=FULL` and
macOS full-fsync enabled and checked. This avoids assuming the existing package
adapter qualifies WAL. Register database/directory/lock identity and explicit
coexistence allowance separately; refuse unsupported storage or settings. No
SQLite/native global durability theorem is claimed by selecting these pragmas.

Rust owns `BEGIN IMMEDIATE`; compare exact old revision/hash and slot registration,
validate the full v2 transition and current controller/Project authority, then
replace canonical snapshot and both derived IDs in **one UPDATE/COMMIT**. All
package IO, UI and provisional target open occur outside this short transaction.
A database writer cannot bypass the shared Rust facade to publish a session; SQL
columns/triggers are integrity checks, not an OS security boundary. No remote
service transaction participates and no file-active pointer is updated.

On COMMIT error or lost reply, retain the exact attempted candidate and original
ID. Reopen through the pinned registrar and compare selected bytes: exact old
means no selection change; exact candidate means reestablish durable database
state and recover its original outcome; different bytes refuse. Process restart
reads selected state and original receipts, then independently opens the selected
context. An interrupted rollback journal must be recovered by SQLite before
selection is read. Qualification must exercise this actual database boundary;
never report Activated merely because UPDATE or COMMIT returned success.

### Local create/rename authority and remaining cloud boundary

The initial local create policy proposed for **explicit approval** is: authenticate
the current controller of the verified bootstrap-default **local-only** Library
in this registered database, under the same write transaction; assign that exact
principal as the new local-only Library controller. Read the stored current
controller, not the deterministic bootstrap principal (controller transfer already
exists). If the default is no longer local-only or actor no longer controls it,
refuse; cloud identity cannot supply or change this grant. New Library name follows
LL0, ID comes from original request, revision is1, term policy version1 and existing
contract version/schema/auth-generation/revision are1. No default is reassigned.
Reuse current admitted normalization-policy row1, not arbitrary caller policy.
Rename requires the target's current controller and exact revision CAS; it never
changes controller or contract authorization. Source:
[`local.rs`](../../../crates/photara-library/src/gen2/local.rs) `ensure_default_library`,
`local_authority` and controller transfer; existing `put_library` is not an
authoritative replacement because it has no actor/atomic contract+receipt contract.

Add only the local lifecycle authority/intent/receipt relations specified above,
restricted to create/rename, plus this slot relation. Create/rename preserve their
original request/hash and publish Library/contract changes+receipt atomically;
no cloud stream or account inventory row is fabricated for a local Library.
Fresh local schema floor6/6 is proposed; historical migrations/checksums and
existing database upgrades remain untouched. No PostgreSQL DDL or API-floor change
is needed for this local-first acceptance. Service API4 remains a future proposal.

Actual service `claim_library` has no additional-Library create-entitlement check;
existing capability lookup presupposes a Library. Do not invent a capability key,
paid entitlement, free quota or eligibility default. Cloud create and local↔cloud
selection remain refused in this slice. Their eligibility/authority mapping is a
separate concrete policy gate, not a reason to block the recommended local proof.

### One bounded approval and readiness handoff

Proposed approval text:

> Approve the exact proposed v2 selection records and fresh SQLite publication
> mapping above, plus local additional-Library creation by the current verified
> bootstrap-default local controller. Implement and qualify this in the normal
> signed app shell using two registered disposable local Libraries and one Project
> each, with shared Rust autosave/activation, avatar create/select/rename and exact
> original retry. Preserve PS4 v1 and all existing package/journal bytes. No cloud
> lifecycle policy, real-library writes, existing-database/slot migration, removal,
> capsule disposal or new production size/count limit is included.

After approval: implement the bounded codec/fresh schema/facade and normal-shell
bridge; run focused actual SQL rollback/unknown-commit, CAS/authority and original
retry tests; then signed native two-Library acceptance. Exercise avatar creation,
rename, Library-only selection and explicit Project open, edit/autosave, failed
save/target-open/publication recovery, close/quit and exact relaunch. Reuse prior
PS3/PS4 behavior evidence rather than another storage furnace. Root then hands the
user the signed normal-shell app and its two disposable Projects for autosave
review. Until this passes, the existing separate controlled scene is not that
handoff. Review remains unapproved; this document changes no code or database.
