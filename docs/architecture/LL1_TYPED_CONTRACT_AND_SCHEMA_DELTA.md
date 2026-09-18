# LL1 typed contract and unnumbered schema delta

Status: **review-only readiness draft, 2026-09-17**. This document prepares LL1
while PS2 continues. [LL0](LIBRARY_LIFECYCLE.md) is the accepted behavior contract;
the signatures and schema changes below are proposals, not implemented APIs.
No migration ordinal is reserved, no DDL is executed, and no production lifecycle
command, live-data change, or rollout is authorized by this draft.
**LL1 ends at typed-contract and unnumbered-schema review. LL2 is the separate
production lifecycle implementation, wiring, and acceptance gate.**

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
Physical names, codec bounds, index design, grants, and exact floor values remain
review work. Local ID encoding follows existing BLOB16 rules; service IDs follow
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
then negotiate new reader/writer/API floors before exposing removal. Floor numbers
are deliberately unassigned. Old writers must fail closed before the new admission
protocol is active; startup cannot recreate a removed default or replay an archived
binding. Existing data is not migrated by this review.

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

Proposed coordination rule for review: the local selected-Library pointer and
GUI project-activation intent have one coordinated durable activation outcome,
with a recovery capsule until it commits. No two independent successful pointer
writes may leave visible Library A with active Project B. Recheck actor, access,
projection, request and attachment generations at final activation. Exact storage
sharing/receipt mapping must be agreed with PS3/PS4; no PS2 file or protocol is
changed here. Voluntary removal with active attachments must first settle them
through their owning session workflow or remain blocked; removal does not flush
or close packages itself.

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

Open design decisions for the next review are: (1) physical detached-evidence and
guarded-retirement design, including every immutable payload exception; (2) exact
aggregate-generation/admission coverage and account/Library lock ordering;
(3) inventory codec, watermark and account-privacy policy; (4) coordinated
selection/activation receipt storage with PS3/PS4; and (5) exact compatibility
floors, bounded codecs and migration/rollout plan. Accepted LL0 names, ownership,
two confirmations, protected defaults, database cleanup, and untouched files are
not reopened. LL1 review approval precedes any numbered migration or implementation;
LL2 authorization and PS3/PS4 acceptance remain distinct prerequisites for production
lifecycle wiring and the signed two-Library/two-project acceptance.
