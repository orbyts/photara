# LL1 typed contract and unnumbered schema delta

Status: **review-only readiness draft, 2026-09-17**. This document prepares LL1
while PS2 continues. [LL0](LIBRARY_LIFECYCLE.md) is the accepted behavior contract;
the signatures and schema changes below are proposals, not implemented APIs.
No migration ordinal is reserved, no DDL is executed, and no production lifecycle
command, live-data change, or rollout is authorized by this draft.
**LL1 ends at typed-contract and unnumbered-schema review. LL2 is the separate
production lifecycle implementation, wiring, and acceptance gate.**

The [physical schema and privilege review](LL1_PHYSICAL_SCHEMA_AND_PRIVILEGE_REVIEW.md)
maps R1–R5 onto proposed relation keys, exact FK changes, retirement guards,
privilege boundaries and explicit remaining proof/approval gates. It executes no DDL.

Read LL0 first, then [Project Session durability](PROJECT_SESSION_DURABILITY.md),
[local schema](LOCAL_SQLITE_SCHEMA.md), [service schema](SERVICE_POSTGRESQL_SCHEMA.md),
and [onboarding security](CXT4A_ONBOARDING_SECURITY_CONTRACT.md). The current
[delivery sequence](../ROADMAP_0_2_EXECUTION.md#2026-09-17-current-delivery-gate--ps2-to-ll2-and-platform-readiness)
allows contract readiness in parallel; LL2 production Library switching/lifecycle
still depends on PS3 session durability and PS4 safe activation.

## Scope and fixed outcomes

Create an additional empty Library, select an accessible Library, rename by
revision, or remove the entire database aggregate. Removal is neither membership
revocation nor hiding a Library. It deletes live and tombstoned owned rows and
catalog references atomically within each authority database, retaining only
explicitly detached historical evidence and the terminal anti-resurrection marker.

**Project packages, source photographs, archives, NAS/cloud objects, shared media
bytes, and cache files remain untouched.** The lifecycle removal coordinator
receives no filesystem, bookmark-resolution, package-open, source-scan, object
delete, or garbage-collection capability. Database paths and bindings are removed
without resolving them. Independent session Save/Close recovery remains owned by
PS3/PS4; it is never an implicit step or compensation inside Remove Library.

Cloud Libraries use service authority, including online account/device and owner
checks. Cached membership does not authorize a lifecycle write. Local Libraries
use the verified database/local-principal controller. No name/path-derived owner,
automatic local/cloud conversion, default reassignment, or account merge exists.
Defaults for **any** account, the bootstrap local default, and the last active
owned Library in the authority domain are protected. Active billing and known
unresolved package/context operations block user-initiated removal.

## Typed vocabulary and authority

Names below describe semantic types, not frozen Rust module paths or transport
endpoints. Reuse existing typed identity/revision/hash primitives where their
semantics match; do not represent every token as an interchangeable string.

| Type | Required meaning |
| --- | --- |
| `LibraryTarget` | `Cloud { service_environment, library_id }` or `Local { database_id, library_id }`; explicit authority prevents accidentally routing a cloud projection as local. |
| `VerifiedLifecycleActor` | Host/service-resolved Account + active Device + authenticated session generation, or verified local Principal + Device + Database. Never trusted from caller-asserted owner fields. |
| `LifecycleOperationId`, `CanonicalRequestHash` | Stable operation identity and domain/version-separated hash of immutable canonical request bytes. Identity scope includes authority and initiating principal. Same ID with changed bytes is rejected. |
| `LibraryName` | Create/rename trims leading/trailing Unicode whitespace once; 1–128 UTF-8 bytes; no control characters. Preserve case and Unicode sequence; duplicate display names allowed. |
| `ExactNameConfirmation` | Raw typed UTF-8 bytes, compared to reviewed authoritative name without trimming, folding, or Unicode normalization. Not a `LibraryName` constructor. |
| `LibraryRevision`, `AggregateMutationGeneration`, `AuthorizationGeneration` | Distinct CAS coordinates for name/state, the full affected aggregate, and current access. Neither timestamps nor selection generations substitute for them. |
| `SelectionRequestGeneration`, `CommittedSelectionGeneration` | Latest requested target versus current accepted UI context. Cancellation invalidates requests without changing the committed selection. |
| `RemovalReviewToken` | Opaque service/local-authority token binding operation, actor/device, target, name/revision, aggregate/access generations, impact digest, and five-minute expiry. UI cannot forge counts or extend expiry. |
| `SessionQuiescenceEvidence` | PS3/PS4-owned bounded observation of attachment/request generation and known blockers. An admission precondition, never Library authorization or proof about other offline devices. |
| `HistoricalLibraryId` | Scalar historical identity accepted only by evidence readers; no Library FK, catalog lookup, binding hydration, or mutation authority. |

Routine errors/logs contain codes and safe IDs, not names, typed confirmation,
canonical personal-data payloads, paths, tokens, or credentials. Canonical request
bytes containing reviewed names may be held in protected pending intent storage;
after removal replace them with the accepted minimal hash/result evidence. Exact
authentication receipts requiring original bytes use the separate archive below.

## Proposed commands, results, and durable state

| Command/query | Inputs and result | Owning boundary |
| --- | --- | --- |
| `CreateLibrary` | Operation envelope, explicit authority, canonical name and proposed new Library ID. Returns durable `Created` receipt or typed rejection. Reused/terminal IDs rejected. | Local Library + controller + receipt, or service Library + contract + owner membership + empty stream + receipt, commit together. Cloud creation requires current online account/device and service-side create entitlement at dispatch; cached membership or a prior entitlement observation is insufficient. No project/package/root binding. |
| `RequestLibrarySelection` | Target, current committed generation, new request generation, session evidence when relevant. Returns `Selected`, `Cancelled`, `Unavailable`, `AccessDenied`, or `RecoveryRequired`. | Device/session operation; no remote lifecycle mutation or default change. Persist only after access/projection/session admission succeeds. |
| `RenameLibrary` | Operation envelope, target, expected Library revision, canonical new name. Returns `Renamed` receipt with resulting revision or CAS/auth rejection. | Owner/controller check, rename, revision, inventory event, and receipt in one authority transaction. |
| `PrepareLibraryRemoval` | Target, preallocated operation ID, current actor context and known session blockers. Returns `Blocked` reasons or `RemovalImpact` + bound review token. | Read-only aggregate impact under a consistent snapshot; token issuance is not deletion or final consent. No file access. |
| `CommitLibraryRemoval` | Immutable envelope plus reviewed name/revision, aggregate/access generations, token/digest, exact typed bytes, and explicit final-consent version. Returns detached `Removed` receipt or definitive rejection. | Current authority recheck and aggregate deletion in one transaction, with marker, receipt, and account invalidations. |
| `QueryLifecycleOperation` | Exact authority/principal/operation identity and request hash. Returns terminal receipt, `InFlight`, or `NotFound`. | Current actor authentication required; former Library membership is not required to retrieve one's own receipt. `NotFound` is not proof a concurrent request cannot commit. |
| `ReconcileLibraryInventory` | Authenticated account/device, inventory epoch/cursor, bounded page request. Returns typed available/changed/removed/revoked items and a consistent watermark. | Channel survives Library stream deletion; local apply/fence/cursor progress commit together. |

Create/rename/remove durably record operation ID and canonical request before
dispatch. A draft is not a dispatched command. Creation does not switch selection
until a confirmed receipt and readable projection exist; “Open Library” explicitly
selects it. Cloud drafts may remain editable offline but cannot dispatch or become
local. Local create/rename/remove can operate offline with verified local authority.

Terminal results distinguish `Created`, `Renamed`, `Removed`, and definitive
`Rejected { reason }`. Progress separately distinguishes prepared, dispatching,
outcome-unknown, committed, projecting/reconciling, and ready. A remote removal
receipt with failed local cleanup is **removed in cloud, local cleanup pending**,
not a failed cloud deletion or a restored Library. `Removed` is never inferred
from a transport error, empty page, denial, or missing local row.

Rejections include invalid name, revision/impact stale, token expired, confirmation
mismatch, final consent absent, authority changed/denied, default/last protected,
billing/pending-work blocker, incompatible client floor, and operation-hash conflict.
Changing the request after a definitive rejection requires a new operation and
review. Unknown results retry/query the same immutable operation. A committed
receipt wins over later token expiry, after current actor/device authentication.

Receipt access belongs to the initiating account/local principal, including a
freshly authenticated eligible device after the original device was revoked.
It must not require the removed membership. A new execution still requires the
original bound device/token and current ownership; recovery on a new device is
receipt retrieval, not reauthorization to execute changed bytes.

## Confirmation and concurrency

Removal has two gates: exact-name typing, then a native final destructive dialog.
Typing an exact match or pressing Enter in the text field cannot dispatch. No
prefilled confirmation. Show the authoritative reference text and explicit mismatch
reason. The final consent is bound to the reviewed impact, not a reusable UI boolean.
Validation occurs again in the authority, not only Swift.

The impact includes category counts for all rows, including revoked/tombstoned
rows, protected default/last flags and known blockers. Its digest binds canonical
ordered affected IDs/counts internally; the response must not reveal hidden project
contents. Do not claim knowledge of unsent work on other Macs. Any relevant rename,
membership/default/aggregate change expires the review, clears typing, and requires
both confirmations again. A timeout preserves unknown state instead of clearing
the request and minting a replacement.

Every writer, including registration, membership/invitation acceptance, streams,
uploads and context changes, must participate in the same Library admission lock
and aggregate generation. Lock relevant account/default coordinates in a fixed
order so concurrent creates/removes cannot both pass last-Library checks. The
service uses the LL0 serializable unit and rechecks its reviewed snapshot after
serialization retry. SQLite uses one write transaction. No writer may publish
after terminal removal or resurrect a Library through replay/bootstrap.

Cancellation before dispatch changes no catalog rows; a local operation disposition
may record cancellation. Once dispatching, closing UI is not cancellation, and
recovery continues. Do not compensate with package rollback, filesystem work,
Auth0 logout, Keychain changes, or remote object deletion.

## Unnumbered schema signatures

These are logical relation signatures and invariant deltas, **not executable DDL**.
Physical DDL, indexes and grants remain review work; the concrete recommendations
below fix proposed codec bounds, relation responsibilities and compatibility floors.
Local ID encoding follows existing BLOB16 rules; service IDs follow
UUID rules. Existing migration files/checksums remain unchanged.

| Proposed relation | Scope/key and required facts | Constraints and retention |
| --- | --- | --- |
| Lifecycle operation intent | Authority + principal + operation; kind, target, immutable canonical request/hash, dispatch state, timestamps. | Local protected durable command storage; unique operation identity. No mutable replacement request. Remove payload after detached terminal evidence exists; retain unrelated pending intents. |
| Detached lifecycle receipt | Authority epoch + principal + operation; request hash, historical Library ID, action/result, initiating device, commit time, reviewed revision, bounded impact counts. | No FK to deleted aggregate; unique immutable terminal result. No package paths, bookmarks, membership payloads, tokens, or stream contents. No time-based expiry in LL1. |
| Terminal Library identity | Authority epoch + historical Library ID; removal operation and terminal version/sequence. | Nonreusable identity for the epoch; immutable; all create/import/bootstrap/replay entry points consult it. Not a Library tombstone or discoverable catalog stub. |
| Account lifecycle inventory/event | Account + sequence within explicit inventory epoch; historical Library ID and typed event with authorized minimal projection. | Service transaction commits invalidation with removal. Consistent watermark pagination; only previously entitled accounts get appropriate invalidation. Survives Library streams. |
| Local inventory position | Authority + account + device; epoch, last atomically applied position, snapshot watermark/completion. | Never infer deletion from an incomplete snapshot. Cursor and dispositions/cleanup advance together or remain pending. |
| Library selection | Database + device + typed account/local-principal scope; optional live Library ID, committed generation, accepted request/activation identity. | Separate from immutable defaults and onboarding singleton. Live target checked; selected removal clears pointer in cleanup transaction. No name fallback. |
| Detached operation disposition | Authority/principal + prior operation ID/hash; historical Library ID, terminal `LibraryRemoved`, source receipt/marker identity. | Prevents queued mutation or completed creation from replaying after cleanup; no command payload, destination/stage path, or live Library FK. |
| Historical authentication/replay archive | Original evidence identity, codec, exact canonical bytes/hash, authenticated principal scope, historical IDs, terminal-use restriction. | Only when retaining original signed/authentication bytes is required; reader cannot hydrate Library/bindings. No live aggregate FK. Reject any candidate bytes containing forbidden paths/secrets until an approved preservation design exists. |

Existing Library authority/contract records require explicit controller/default
protection and aggregate/admission coordinates where absent. Do not hide lifecycle
state in `extensions_json`, treat a cached membership as a controller, or reuse
`put_library` tombstoning as removal. This draft does not decide final table splitting.

### Guarded retirement, not disabled integrity

The source baseline has 15 local migrations and 14 service migrations. Local
[onboarding](../../crates/photara-library/migrations/generation_two/0013_onboarding.sql),
[dispositions](../../crates/photara-library/migrations/generation_two/0014_onboarding_dispositions.sql),
and [UI1 creation](../../crates/photara-library/migrations/generation_two/0015_project_creation.sql)
retain immutable/nondeletable Library-bound records. Service
[onboarding/defaults](../../crates/photara-service/migrations/postgres/0014_onboarding.sql)
and [scoped receipts](../../crates/photara-service/migrations/postgres/0011_scoped_sync.sql)
also require coordinated retention changes. Existing guards cannot simply be bypassed.

Propose a narrow retirement transaction whose admission proves current authority,
reviewed impact, exact operation/hash, cleared blockers, and complete detached
evidence. Only matched aggregate rows gain a guarded terminal transition; ordinary
UPDATE/DELETE remains forbidden. The physical design must prove a caller cannot
forge a retirement flag or use an unrestricted session setting to disable guards.
Retire onboarding replacement/disposition chains as a closed set, and close deferred
receipt/batch cycles without leaving one side alive. FKs and immutability checks
remain enabled. PostgreSQL runtime gets only the reviewed service boundary, not
general destructive privileges; SQLite raw SQL is not a supported lifecycle API.

Fresh and upgrade paths must preserve replay/authentication bytes and UI1 recovery,
then negotiate new reader/writer/API floors before exposing removal. The floor
recommendation below is separate from migration numbering. Old writers must fail
closed before the new admission protocol is active; startup cannot recreate a
removed default or replay an archived binding. Existing data is not migrated by
this review.

## Enumerated table dispositions

The following inventory comes from current migration source, not a live database.
Every listed table's **affected rows only** receive the stated disposition. Retain
other Libraries/accounts and shared metadata unchanged. Grouping does not replace
an independent runtime FK/logical-reference closure check. Future tables must be
classified before removal can be enabled.

`D` = delete complete owned rows; `T` = first preserve allowed detached terminal
evidence, then delete owned rows; `R` = retain unrelated/shared/account state;
`B` = block if protected/unresolved, otherwise apply the named disposition.

### SQLite

| Tables | Disposition |
| --- | --- |
| `schema_metadata`, `normalization_policies`, `local_device`, `account_cache` | R; no database/device/account deletion. |
| `libraries`, `library_contract_state` | B protected default/last; D roots after closure. |
| `membership_cache` | D all membership projections for target. |
| `library_media`, `media_local_state`, `project_media_links`, `media_transfers` | D database references; zero media/cache-file deletion or GC. |
| `people`, `organizations`, `social_profiles`, `person_capabilities`, `person_labels`, `organization_labels`, `person_organization_relationships`, `location_kinds`, `location_kind_terms`, `locations` | D including tombstones and claims. |
| `project_catalog`, `project_ownership`, `project_access_policies`, `project_access_grants`, `project_invitations` | D catalog/access aggregate, including hidden/pending rows. |
| `storage_roots`, `project_locators`, `device_root_bindings`, `device_project_bindings`, `storage_location_specs`, `storage_slots`, `storage_slot_names`, `host_bindings`, `host_binding_selections`, `legacy_external_resource_resolutions` | D owned/logically linked database bindings; never resolve paths or opaque grants. |
| `project_observations`, `project_party_projection`, `project_location_projection`, `project_graph_projection` | D indexed projections; package authority unchanged. |
| `library_variables`, `library_variable_values`, `library_variable_names`, `library_expressions`, `library_expression_dependencies`, `device_context_snapshots` | D owned context and snapshots, including logical Library/Project references. |
| `mutations`, `local_changes`, `mutation_baselines`, `sync_object_state`, `sync_outbox`, `sync_inbox`, `sync_cursors`, `sync_conflicts`, `sync_snapshot_installs`, `sync_targets` | T operation identities when needed, then D full payload/queue/channel closure. |
| `remote_mutation_receipts`, `local_mutation_dispositions` | T immutable replay/terminal evidence as required; D live Library scope. |
| `scoped_sync_channels`, `scoped_sync_operations`, `scoped_sync_operation_roots`, `scoped_sync_base`, `scoped_sync_inbox`, `scoped_sync_snapshot_installs` | T pending operation dispositions then D all channel/base/payload rows. |
| `operation_intents`, `operation_events`, `recovery_items`, `recovery_publications`, `context_apply_intents`, `context_apply_receipts` | B unresolved user-initiated removal; otherwise T then D. Remote-removal fencing follows the separate rule below; retained package/recovery files are untouched. |
| `onboarding_session`, `onboarding_credential_references` | R authentication/session/credential identity; no sign-out or secret-store call. |
| `onboarding_intents`, `onboarding_receipts`, `onboarding_dispositions`, `onboarding_replacements` | T authenticated/replay evidence through guarded closed-chain retirement; D target-bound rows. Unresolved local work blocks voluntary removal. |
| `library_cloud_bindings`, `onboarding_library_selection` | D target binding/old selection after preserving necessary receipt evidence. New scoped selection cleared/reconciled atomically. |
| `project_creation_intents` | B unresolved voluntary removal; T completed/cancelled identity/result, then D destination/stage/request data. Remote removal creates minimal terminal disposition, never cancels staging on disk. |

### PostgreSQL

Qualified names are explicit because identity/private/public domains have different
privileges. No table is classified merely by the presence of a `library_id` column.

| Tables | Disposition |
| --- | --- |
| `photara.schema_metadata`, `photara.normalization_policies` | R. |
| `photara_identity.accounts`, `photara_identity.account_identities`, `photara_identity.devices`, `photara_identity.device_credentials`, `photara_private.account_developer_grants` | R account/device authority. |
| `photara_identity.account_defaults` | B if **any** row targets the Library; R all defaults, no reassignment. |
| `photara.libraries`, `photara.library_contract_state` | B default/last/billing/pending policy; D after dependent closure. |
| `photara_identity.memberships`, `photara_identity.project_access_grants`, `photara_identity.project_invitations`, `photara_private.project_invitation_secrets` | D all active and inactive rows; invalidate invitation authority in transaction. |
| `photara.people`, `photara.organizations`, `photara.social_profiles`, `photara.person_capabilities`, `photara.person_labels`, `photara.organization_labels`, `photara.person_organization_relationships`, `photara.location_kinds`, `photara.location_kind_terms`, `photara.locations` | D complete typed aggregate. |
| `photara.storage_roots`, `photara.project_catalog`, `photara.project_locators`, `photara.package_observations`, `photara.project_ownership`, `photara.project_access_policies`, `photara.storage_location_specs`, `photara.storage_slots`, `photara.storage_slot_names` | D catalog/reference closure, no package/object access. |
| `photara.library_variables`, `photara.library_variable_values`, `photara.library_variable_names`, `photara.library_expressions`, `photara.library_expression_dependencies` | D owned context; package snapshots untouched. |
| `photara.library_media`, `photara.project_media_links`, `photara_private.media_objects`, `photara_private.media_upload_sessions` | D Library-owned database rows. Current `media_objects` has a Library FK: its row cannot be retained as a “shared object” exception. Actual object bytes remain; no cleanup-triggered remote GC. |
| `photara_private.library_streams`, `photara.library_change_batches`, `photara.library_changes`, `photara_private.sync_clients`, `photara_private.scoped_streams`, `photara_private.scoped_change_batches`, `photara_private.scoped_changes`, `photara_private.scoped_sync_clients` | D all streams/batches/payloads after coordinated receipt detachment. |
| `photara_private.library_claim_receipts`, `photara_private.mutation_receipts`, `photara_private.scoped_command_receipts`, `photara_private.security_audit` | T required immutable/account-scoped evidence; D live aggregate references with receipt/batch closure. |
| `photara_private.library_subscriptions`, `photara_private.library_entitlement_grants` | B active billing obligation; T permitted historical accounting evidence, then D Library-owned rows; no billing-provider command. |
| `photara_private.billing_events` | R account/provider evidence only after classifying embedded Library identifiers as historical; any live authority/reference in payload requires a reviewed detached representation before removal. |
| `photara_private.onboarding_receipts`, `photara_private.onboarding_challenges` | R account/identity-scoped exact authentication evidence and existing challenge retention; embedded removed IDs become historical-only to every replay reader. No new binding hydration. |

Migration ledgers and SQLite engine support tables remain intact. “T” is not
permission to retain an arbitrary JSON copy of the deleted aggregate. Enumerate
each archived payload shape, classify embedded IDs, prove path/secret exclusion,
and preserve exact authentication bytes only where required. If these constraints
cannot both be met, removal remains blocked until that retention case is reviewed.

## Inventory, remote removal, and session dependency

Cloud receipt/marker and affected-account lifecycle invalidations commit with
deletion. Local reconciliation must read this account-level channel **before**
dispatching queued Library work, after reconnect, restored backup, or sign-in.
An explicit removal or complete authoritative inventory can fence an absent
Library; interrupted pagination cannot. Access revocation is distinct from removal
and does not permit claiming an aggregate was deleted.

Once removal is observed, fence writes/observers first. In one local transaction,
persist terminal marker/dispositions, delete owned projections and pending command
payloads, clear affected selection, and advance inventory progress. Keep account
credentials and unrelated Libraries. Offline machines may retain old cache until
reconnection; this design makes no immediate-remote-erasure claim.

Remote removal can supersede local pending UI1/context work that would have blocked
voluntary local removal. Preserve minimal detached operation/hash/result evidence,
leave package staging and recovery files untouched, and show recovery requiring
the owning workflow. Never replay, export, reassign, or re-register automatically.
In-memory unsaved work stays frozen for an explicit recovery choice. A receipt or
old package carrying the historical Library UUID cannot recreate authority.

| Dependency | LL1 readiness can specify now | Required before LL2 production wiring |
| --- | --- | --- |
| PS2 | No guessed `Saved` status or assumed journal durability. | Production writer/codec/storage acceptance; disposable evidence alone is insufficient. |
| PS3 | Session admission/freeze/flush/terminal-run/cancellation result types and generation handoff. | Rust-owned session and truthful save/recovery result. Failed/unknown save or stop retains current context. |
| PS4 | Selection activation receipt and rollback capsule interface. | Target validation/restoration and durable active-pointer commit; failed activation restores old view or explicit read-only recovery. |
| LL1 → LL2 | LL1 reviews account/local scoped selection, authority/impact/receipt/inventory and cleanup contracts. | LL2 requires separately approved schema, authorized implementation and disposable local/service/multi-device furnaces, then accepted native views wired through the coordinator. |

A Library switch uses the PS3/PS4 coordinator, never direct assignment to a Swift
Library/Project property. Only unsaved work triggers existing save/close/cancel
resolution; a clean Library switch has no generic Save action or confirmation.
An owned active evaluation requires the PS contract's Stop Run and Switch/Cancel
and terminal completion. Independent clients' work is not implicitly cancelled.
Concurrent A→B→C requests cannot let late B results overwrite C.

The recommendation below gives the local selected-Library pointer and GUI
project-activation intent one coordinated durable activation outcome, with a
recovery capsule until it commits. No two independent successful pointer writes
may leave visible Library A with active Project B. PS3/PS4 must accept that mapping
before implementation; no PS2 file or protocol is changed here. Voluntary removal
with active attachments first settles them through their owning session workflow
or remains blocked; removal does not flush or close packages itself.

## Concrete recommendations for the five LL1 decisions

These recommendations resolve the design direction for review. They do not mark
LL1 approved or authorize DDL, implementation, deployment, or live deletion.

### R1 — detached evidence and narrow retirement

Use separate logical relations for `lifecycle_intents`, `lifecycle_receipts`,
`removed_library_identities`, `retired_operation_dispositions`, and
`historical_authentication_evidence`. Do not put all five in a generic archive
JSON table. Their semantic keys and retention follow the signatures above;
retired IDs are scalars with explicit historical types, never nullable live FKs
that ordinary repositories can later fill in.

The retirement unit stages a typed disposition manifest in the same transaction:
target Library, initiating operation/hash, reviewed generations, exact source
record identities/digests, allowed output evidence identities/digests, and
disposition kind. It validates complete dependency closure, copies permitted
evidence, deletes matched children/cycles and roots, inserts final receipt/marker
and invalidations, checks postconditions, then commits. A manifest is internal
transaction admission, not a new durable copy of all deleted data. Failed
postconditions roll back the whole database unit. No visible “half retired” state.

PostgreSQL should expose one narrow control-role retirement operation with a
fixed qualified search path and no caller-selected table names/dynamic SQL.
Runtime clients cannot directly insert retirement permits or delete protected
tables. Trigger exceptions must match the current transaction, admitted operation,
Library and exact manifest row; a caller-set GUC/boolean alone is not admission.
Authentication/authorization still belongs to the verified service boundary;
an `actor_id` parameter by itself proves nothing. Role and privilege tests must
demonstrate that ordinary API/control entry points cannot mint the exception.

SQLite should use the private typed repository's single write transaction with
the same matched-manifest guards and deferred closure where needed. Ordinary
repository calls cannot enter retirement. This does not claim security against
a process already able to rewrite the local database file or issue unrestricted
SQL; that remains the existing trusted-host boundary. Do not disable foreign
keys, drop guards at runtime, or expose a public raw-SQL retirement flag.

Apply this exception allowlist; unknown codecs/payload shapes block voluntary
removal rather than expanding retention automatically:

| Existing evidence | Recommended terminal representation |
| --- | --- |
| Completed/cancelled UI1 creation and package/context operation rows | Operation ID, actor scope, request hash, historical Library/Project IDs when required, terminal result and retirement receipt reference. Drop destination/stage fields and command/recovery payload. Keep files untouched. Unresolved work follows LL0's blocker/remote-fence rules. |
| Old claim/mutation/scoped receipt and disposition rows | Preserve original request/response hashes, authenticated actor/device, operation identity, original terminal outcome and historical coordinates; add `LibraryRemoved` applicability disposition. Do not return old content-bearing response bytes or stream payload. Former success remains historical success, never a new permission to apply it. |
| Security audit | Retain allowlisted action/time/actor/target IDs and original details digest. Drop arbitrary details after checking for required accounting/authentication evidence; any unhandled retention obligation blocks and requires explicit review. |
| Local exact onboarding receipt | Archive only the validated original receipt bytes/hash and required principal/operation binding. Do not archive the whole intent, access projection, credential reference, challenge/nonce, request name, or replacement chain payload. Preserve chain identity/hash/result in detached dispositions. |
| Service onboarding receipts/challenges already account-scoped | Keep existing immutable bytes and existing challenge-retention policy in their current account domain; do not copy them into a new Library archive. Every replay reader checks terminal identity before proposing Library projection. |
| Billing history | Leave independent immutable provider events under their existing account/provider policy; classify embedded Library IDs as historical. For Library-owned links retain only an approved accounting disposition, then delete links. Active obligations still block. No provider cancellation or new billing retention rule. |

The inspected [service onboarding types](../../crates/photara-service/src/onboarding.rs)
and [local counterpart](../../crates/photara-library/src/gen2/onboarding.rs) show
that exact bootstrap receipts include membership/stream IDs and a device
credential **digest**, not the secret. These are the narrow LL0 immutable-byte
exception: preserve the original bytes where required for receipt verification,
but expose them only through account-scoped evidence readers. Do not treat the
embedded membership/stream IDs as live links, import the old access projection,
or use the digest as a credential. The normal lifecycle receipt contains none
of these authentication fields.

**Approval boundary:** LL0 already permits exact authentication evidence and
minimal terminal history. It does not authorize indefinite retention of arbitrary
domain payloads, paths, secrets, or new billing obligations. If a real legacy
receipt cannot satisfy the allowlist while preserving required authentication,
stop that removal and present the exact codec/field conflict for user/security
review. This draft does not choose “keep everything” or destroy required evidence.

### R2 — aggregate admission, generation and lock order

Use three independent equality tokens: Library name/state revision, Library-wide
aggregate generation, and the existing authorization generation. An effective
database write to any row included in impact, blockers or permissions advances
the relevant Library aggregate generation in that transaction. A generation may
advance more than once for a multi-row command; it is not a per-command revision
or a wall clock. Use checked positive signed-64-bit counters locally and on the
service, transported as canonical decimal strings; overflow fails closed.

Cover every disposition-matrix table's affected Library: typed children and
tombstones, catalog/projection/binding changes, grants/invitations, stream/client
cursors, media/upload sessions, receipts, context/recovery rows, billing links,
default/controller state and queue state. A writer touching multiple Libraries
declares them all. Terminal receipt lookup, read-only impact preparation, its
ephemeral review token, account inventory delivery acknowledgments, and session
selection are outside the aggregate generation unless they change an actual
impact/blocker fact. Otherwise preparation would invalidate itself. Package
authored edits are not SQL aggregate changes; active session/pending-operation
admission is rechecked separately, without reading packages from removal.

Recommend this common service order for **every** affected writer:

1. Resolve the full lock set from an untrusted preliminary snapshot, with no
   mutation. Include actor, affected owners/default accounts, membership/grant
   targets, and inventory recipients needed by the operation.
2. Acquire any required canonical authentication-principal locks first; then
   Account locks in UUID byte order; then their identity/device/credential/default
   rows in fixed table-and-key order. All ownership/default create/remove paths
   use those account locks, including creation of a new Library ID.
3. Acquire Library admission rows in UUID byte order, then Project policy rows,
   operation/dedupe rows, and remaining affected children in fixed table/key order.
   No account lock may be acquired after a Library lock.
4. Re-read the closure and lock-set membership. If discovery added an account or
   Library, roll back and restart with the enlarged set; never acquire it out of
   order. Check current identity/device, owner/controller, entitlement, default
   for every affected account, last-owned count, and exact reviewed generations.
5. Publish mutation/generation/receipt/inventory in the same serializable unit.
   Serialization retry retains operation identity but must reject a stale review;
   no retry automatically renews confirmation or bypasses a blocker.

SQLite serializes the matching validation and closure through `BEGIN IMMEDIATE`;
there is no distributed lock shared with the service. Existing service
[access](../../crates/photara-service/src/access.rs) and
[onboarding](../../crates/photara-service/src/onboarding.rs) have their own locking
paths today. LL2 must audit and adapt all of them before enabling deletion; this
recommendation does not assert they already satisfy the complete order.

Receipt recovery authenticates the actor and serializes on the operation identity
without requiring deleted Library locks/membership. A fresh execution locks its
live Library before the operation key; duplicate recovery of an existing terminal
receipt never subsequently takes a Library lock. An in-flight/missing result
returns to the normal ordered execution path with the same bytes. This prevents
recovery from introducing an operation→Library inversion.

The conservative contract can invalidate a five-minute review during background
sync/media activity. LL2 should test that consequence and report stale-impact
honestly. Excluding real impact changes or holding a long write lock throughout
human confirmation would change accepted LL0 behavior and needs a new decision;
neither is selected here.

### R3 — inventory codec, watermark and privacy

Recommend a dedicated `photara.library-lifecycle.v1` envelope using the existing
`photara.canonical-json.v1` rules, strict discriminated variants, canonical UUIDs,
lowercase SHA-256, decimal counters, and no unknown mandatory fields. It is a
control/inventory protocol, not a new Library content stream. Add account-owned
immutable `library_inventory_events`, a materialized current inventory, and
device-local inventory positions; the current view is derived from the event log.

Each account inventory has an epoch and monotonically ordered sequence. Library
create/rename/access changes/removal emit the necessary per-account events in the
same authority transaction as their effects. At rollout, seed a versioned current
inventory snapshot from verified current authority, explicitly marked as baseline;
do not invent historical lifecycle actions. Retain events and terminal identity
evidence without time-based pruning for this bounded release. A later compaction
policy must preserve old-device/backup recovery before it can replace that rule.

Full inventory pages describe the state **as of one captured high-water H**, using
immutable event history plus the baseline. Page by stable Library UUID key, with
an opaque service cursor binding account, environment, device, epoch, H, last key,
codec and authorization/privacy generation. Do not hold a database transaction
open across network requests. Each page checks current authenticated account/device
and privacy generation; a removal/revocation/access-policy change invalidates the
snapshot before exposing now-forbidden fields. Restart at a new watermark without
inferring absence from the interrupted snapshot. Name-only changes after H may be
delivered in the next delta rather than rewriting an earlier page.

On complete inventory, the client may fence a formerly visible ID absent from
the accessible set, but absence means `AccessUnavailable`, **not** `Removed`.
Only an explicit authenticated removed event/terminal proof permits deletion
reconciliation and installing a removal marker. Delta application and local cursor
advance are atomic; reject duplicate sequence with different bytes, gaps, wrong
epoch/account and mismatched snapshot continuation. An epoch change forces a full
inventory before queued work resumes, preserving pending local work until a typed
disposition is known.

| Recipient/event | Permitted response |
| --- | --- |
| Currently authorized accessible Library | ID, display name, authority mode, membership role/access summary, revisions, lifecycle availability and default protection relevant to that account. No project contents, paths or member list. |
| Active member at removal | Historical Library ID, removed event/version and account-specific invalidation. No impact counts, other recipients or actor details. |
| Revoked account | Its own `AccessRevoked`/unavailable result with already-known ID; no subsequent removed/name updates or newly learned catalog facts. |
| Initiating remover querying its receipt | Its authenticated scoped terminal receipt and previously reviewed bounded counts; no broader inventory access follows. |
| Unrelated account / guessed Library ID | Uniform access-unavailable response; no existence oracle. |

Receipt recovery is separate from inventory: former members cannot query the
owner's operation receipt. Tombstone identity does not make an ID discoverable.
Routine logs remain code/operation oriented, without names or confirmation text.

### R4 — one device activation receipt with PS3/PS4

Recommend one protected local SQLite activation transaction, owned by the Rust
session coordinator, to publish both scoped Library selection and the GUI's active
Project pointer. Add a logical `activation_intent` (recovery workflow) and immutable
`activation_receipt` (committed outcome), keyed by database/device/principal and
activation operation. The target contains Library ID and optional Project ID;
selecting a Library alone does not silently open a remembered Project. A Project
activation requires matching Library ownership/access and exact package identity.

The intent binds prior committed selection, latest request generation, actor/access
generation, attachment/owner epoch, target identity, expected pointer revisions,
and PS3's verified barrier coordinate plus opaque recovery-capsule reference. PS3
owns any device-local capsule bytes/paths and package lease; LL1 stores only the
typed reference and required identity/checksum. This is a session protocol record,
never a package-authored record or cloud inventory item.

The coordinator first resolves pending input/run outcomes and prepares/restores
the target through PS3/PS4. It then rechecks authority, projection, target identity,
latest request and attachment generations, and atomically inserts the activation
receipt, advances committed generation and updates **both** pointers. The UI
projects that committed receipt; no separate Swift preference write completes
selection. Frozen current content and its verified rollback capsule remain until
this transaction succeeds. Other client attachments retain their independent jobs.

Before receipt commit, crash/cancel/target failure recovers the old activation or
PS4's explicit read-only recovery if reacquisition fails. After receipt commit,
restart recovers the target or explicit target recovery; it never silently rewinds
the pointer to old state. A lost receipt response queries the original activation
ID. Pointer-write failure leaves old committed selection intact. Capsule disposal
occurs only through the existing session recovery owner after it is safe.

Library selection requests may supersede an earlier request **before** PS4 freezes
and starts its activation transaction. After freeze, serialize/reject another target
until completion/cancel; do not replace the in-flight rollback capsule. Thus late
B cannot overwrite C, while PS0's rule against replacing an active switch remains
intact. Same-target selection/activation remains idempotent.

Remote Library removal fences matching session attachments before cleanup. Cleanup
invalidates associated activation intents and clears live pointers atomically with
terminal dispositions; minimal historical activation receipts survive without live
Library FKs or capsule/binding references. It does not read/delete capsules or packages. Retained unsaved memory
is visible only as explicit recovery, not an editable revoked Library. If removal
or access change races target preparation, final activation CAS fails; if observed
after commit, the normal invalidation path fences the committed target.

This recommendation chooses a shared **local session transaction**, not cross-store
ACID or a transport. PS3/PS4 owners must review it before accepting their persistence
shape. If they choose an active-pointer authority outside this SQLite transaction,
stop integration for an explicit recovery-protocol decision; do not silently fall
back to two independent pointer writes.

### R5 — compatibility, bounded codecs and rollout

For the inspected baseline only, recommend local reader/writer floors **6/6**
(current 5/5), service minimum API **4** (current 3), and mandatory lifecycle
capability `photara.library-lifecycle.v1`. These are proposed protocol/schema floors,
**not migration ordinals or reservations**. Retain current schema-family/epoch and
canonical codec unless physical review proves an incompatible family change is
needed. Re-evaluate floor values if another slice advances the baseline first;
never edit an already applied migration or fabricate its checksum ledger.

| Envelope or field | Proposed hard bound |
| --- | --- |
| Create/rename/remove/query command | 64 KiB canonical bytes; strict exact fields for its variant. |
| Library name / exact typed confirmation | Name is 1–128 UTF-8 bytes after the specified creation trim; confirmation is at most 128 raw bytes, never normalized. |
| Removal token | Opaque 32-byte random value represented canonically; authority stores its digest and bound review facts. Five-minute expiry stays fixed; token is not a bearer authorization. |
| Impact/receipt | 64 KiB; closed category enum (maximum 64 categories), decimal nonnegative counts; no full affected-ID list on wire. Internal closure digest is computed incrementally in deterministic order. |
| Inventory page | At most 100 entries, 2 KiB per entry and 256 KiB total canonical page; byte limit can end a page earlier. Positive progress or explicit error, never a looping empty continuation. |
| Inventory continuation | Opaque cursor up to 2 KiB, authority-bound as in R3; no client-generated cursor claims. |
| Detached authentication evidence | Existing known onboarding codec's 64 KiB bound and exact bytes; no generic larger archive fallback. |

Enforce size limits before allocation/canonical decoding, reject duplicate keys,
unknown variants, invalid decimal/UUID/hash encodings and counter overflow. Exact
confirmation may never be silently truncated. Large valid aggregate deletion uses
bounded-memory enumeration in one atomic transaction; it does not split committed
deletion over pages. Failure to finish within the admitted transaction budget rolls
back and reports a retryable capacity/timeout failure, not partial completion.
A background/chunked externally visible removal would require a new LL0 decision.

Recommended rollout sequence, still separately gated:

1. Accept LL1 recommendations and review physical unnumbered deltas, trigger/FK
   closure, privilege changes, receipt codecs and upgrade fixtures. Retain the
   full 79-table local/59-table service baseline classification; rerun inventory
   at implementation time and fail on unclassified additions.
2. Authorize LL2 implementation on disposable databases only. Add new migration
   files after physical approval; prove upgrade/fresh parity, original evidence
   verification, global writer-lock/generation coverage and session fakes. Keep
   lifecycle endpoints/actions disabled.
3. Complete PS3/PS4 acceptance and wire the accepted LL0 native flow. Prove signed
   two-Library/two-project behavior, failure recovery and zero removal-origin file
   effects on isolated targets. A synthetic coordinator fixture is not acceptance.
4. Under separate rollout authorization, quiesce affected service writers, verify
   backup/recovery, install reviewed schema and all participating writer code,
   verify roles/floors, seed inventory and then enable lifecycle capability.
   Do not serve an old writer alongside deletion-capable code. Local migration
   likewise runs exclusively before normal startup workers/selection hydration.
5. Negotiate capability/floor before dispatch and inventory-before-replay on every
   returning device. Old clients refuse mutation/activation; no automatic local
   fallback, bootstrap or Library recreation. An older client that cannot present
   a terminal receipt must upgrade to recover it; retain evidence meanwhile.

Before any accepted lifecycle writes, deployment can remain disabled or restore
the verified pre-cutover state under its approved recovery procedure. After a
committed removal, do not restore a stale backup as live authority or roll back to
old readers: it could resurrect the Library. Prefer forward repair; any database
restore must preserve/reconcile all terminal identities and accepted operation
evidence before admitting clients. Cross-epoch disaster recovery needs a separately
reviewed fence/reconciliation plan, not a reset of the marker ledger.

## Additional verification for R1–R5

Add these cases to LL2's disposable furnaces; no tests or migrations are executed
by this documentation change:

- R1: original known onboarding receipt bytes/hash still verify; archived receipt
  cannot rehydrate membership/stream; unknown or path-bearing payload blocks;
  forged/out-of-transaction retirement permit fails; failure before each evidence,
  cycle, root and marker step rolls back all rows.
- R2: enumerate each write path against generation coverage; concurrent last-owned
  removals and membership/default changes; incomplete lock-set restart; stable lock
  ordering across onboarding/access/media/registration; receipt recovery after root
  deletion without operation→Library lock inversion; overflow refusal.
- R3: seed, page interruption, delta gap/duplicate mismatch, wrong account/device/
  epoch, concurrent rename, revoke/removal between pages, authoritative absence
  versus explicit removal, stale-backup resnapshot, and guessed-ID privacy.
- R4: faults before/after one activation commit; lost response; late B after C;
  in-flight target serialization; save/run-stop failure; PS4 reacquisition failure;
  removal during preparation and after commit; no mixed Library/Project pointers.
- R5: exact byte boundaries including multibyte names, oversized/duplicate-key and
  unknown-codec refusal, old-client floor failure before any replay, fresh/upgrade
  checksum parity, disabled-feature rollout and forward recovery after deletion.

## Review and acceptance checklist

| Gate | Independent proof required; not claimed by this draft |
| --- | --- |
| Contract | Canonical name limits vs raw confirmation; same-ID/hash outcomes; owner/controller enforcement; current service-side cloud create entitlement (including denied/expired cases and original-result retry); no default change or cloud-offline dispatch. |
| Schema closure | Compare every source/runtime table, FK, trigger, logical UUID and canonical-payload reference against the disposition inventory; reject unknown/unclassified tables. |
| Local integrity | Populated active/tombstoned/revoked fixtures; wrong principal denied; zero owned rows/orphans; FK/integrity checks; unrelated row content unchanged. |
| Cloud integrity | Disposable service/PG; owner success and all weaker roles denied; account/default/last/billing races; receipt/batch cycles; no post-delete stream writer. |
| Retention | Original required auth bytes still verify; detached readers cannot hydrate authority; guarded deletion unavailable outside admitted operation; no sensitive payload leakage. |
| Retry/recovery | Crash at each durable boundary; lost commit reply; concurrent duplicate; altered bytes; expired token after success; NotFound while request still in flight; new-device receipt recovery. |
| Removal review | Five-minute expiry, name/aggregate/access mutation invalidation, both gates, Unicode byte mismatch, cancel-before-dispatch and unknown-after-dispatch. |
| Multi-device | Offline commands + remote removal; inventory before upload; interrupted page/old backup/stale onboarding response; terminal marker prevents resurrection. |
| Session | PS3/PS4 accepted first; clean/dirty/running/error switching; pointer-write failure; A→B→C race; retained unsaved work after removal; no mixed Library/Project activation. |
| File isolation | Injected filesystem/package/object APIs record zero removal-origin calls; independent before/after canary byte/name/metadata manifests unchanged, including symlinks, caches and staged packages. |
| Upgrade/native | Explicit new floors and old-writer refusal; UI1/auth recovery preserved; accepted LL0 native menu and both confirmation gates; signed two-Library/two-project acceptance remains a later gate. |

R1–R5 now provide concrete contract recommendations for the five previously open
design items. **Approval remains pending** for those recommendations and their
physical schema/privilege proof. No runtime correctness is claimed by document
checks. Accepted LL0 names, ownership, two confirmations, protected defaults,
database cleanup, and untouched files are not reopened.

Before leaving LL1, review the evidence allowlist/guarded retirement, writer-lock
coverage, inventory privacy and bounds, and PS3/PS4 activation mapping; confirm
floor values against the then-current baseline. Any required arbitrary payload
retention, weakened impact invalidation, distributed pointer publication, chunked
visible deletion, or cross-epoch restore policy is a new decision and remains
unselected. LL1 approval precedes numbered migration authoring or implementation.
LL2 authorization, PS3/PS4 acceptance, disposable furnaces, and separately approved
rollout remain distinct gates before production lifecycle wiring/live removal and
signed two-Library/two-project acceptance.
