# PS0 review-only contracts and unnumbered schema deltas

Proposals only; these signatures are not compiled APIs, migration inputs or wire
compatibility promises. Normative behavior: [PS0](../../PROJECT_SESSION_DURABILITY.md).
No new production module or database table is created by this checkpoint.

## Rust-facing boundary

Reuse Core ProjectId/GraphId/GraphRevision, CommandId and package Sha256Hex,
PackageUuid/DecimalU64 through validated constructors. New IDs are distinct UUID
newtypes, not interchangeable strings. Counters serialize as decimal strings at
wire boundaries. Types below describe required payloads; Bytes means bounded
canonical bytes, not arbitrary unvalidated JSON.

```rust
struct OwnerEpoch { owner_id: OwnerId, generation: u64 }
struct AttachmentToken {
    attachment_id: AttachmentId,
    generation: u64,
    owner: OwnerEpoch,
}
// Constructed only by the trusted host admission boundary, not by wire decoding.
struct AuthorizedContext { /* principal, grantor, grant, scope, validity policy */ }
// Bounded non-secret references; historical evidence, never an access credential.
struct OperationProvenance {
    principal: PrincipalRef,
    actor: ActorRef,
    grantor: PrincipalRef,
    grant: GrantRef,
    effective_scope: ActionScope,
    policy_decision: AuthorizationDecisionRef,
}
struct PackageIdentity {
    project_id: ProjectId,
    library_id: LibraryId,
    incarnation_id: IncarnationId, // local identity, never authored data
    bootstrap_digest: Sha256Hex,
}
struct HeadCoordinate {
    commit_id: PackageUuid,
    commit_digest: Sha256Hex,
    head_bytes_digest: Sha256Hex,
    package_revision: DecimalU64,
}
struct AuthoredCoordinate {
    revision: DecimalU64,
    authored_digest: Sha256Hex,
    graphs: BTreeMap<GraphId, GraphCoordinate>,
}
struct GraphCoordinate { revision: GraphRevision, digest: Sha256Hex }
struct MutationRequest {
    attachment: AttachmentToken,
    operation_id: CommandId,
    expected: AuthoredCoordinate,
    undo_group: UndoGroupId,
    boundary: GroupBoundary, // Begin, Continue, End, Single
    command: AuthoredCommand,
}
enum AuthoredCommand {
    Graph(GraphCommandEnvelope),
    Project(ProjectCommandEnvelope),
    // Adapters must preserve all package records; legacy ProjectDocument alone
    // cannot represent the full 1.1 authored closure.
    Undo { group: UndoGroupId },
    Redo { group: UndoGroupId },
}
struct MutationReceipt {
    operation_id: CommandId,
    request_digest: Sha256Hex, // canonical intent digest, excluding transient attachment
    journal_sequence: u64,
    result: AuthoredCoordinate,
    provenance: OperationProvenance,
    durability: JournalDurable, // unforgeable marker from IO barrier
}
struct FlushRequest {
    attachment: AttachmentToken,
    through_sequence: u64, // finite accepted prefix captured after draining client input
    reason: FlushReason,
}
enum FlushReason { Idle, SaveNow, FocusLoss, Detach, Switch, Sleep, Terminate, ClientBarrier }
struct SavedReceipt {
    owner: OwnerEpoch,
    identity: PackageIdentity,
    through_sequence: u64,
    authored: AuthoredCoordinate,
    head: HeadCoordinate,
    capability_profile: CapabilityProfileId,
}
enum SaveState {
    Saving { accepted_sequence: u64, checkpoint_sequence: u64 },
    Saved(SavedReceipt),
    Failed { failure: SessionFailure, last_saved: Option<SavedReceipt> },
}
struct CheckpointPlan {
    write_id: WriteId,
    identity: PackageIdentity,
    expected_head: HeadCoordinate,
    candidate_head: HeadCoordinate,
    through_sequence: u64,
    authored: AuthoredCoordinate,
    immutable_files: Vec<PlannedFile>, // validated relative name, hash, length, bytes
    head_bytes: Bytes,
}
enum PublishOutcome {
    Verified(SavedReceipt),
    NotPublished(SessionFailure),
    Unknown { write_id: WriteId, phase: WritePhase },
}
enum ReconcileOutcome {
    Resume { next_sequence: u64 },
    AlreadyPublished(SavedReceipt),
    Conflict { package: HeadCoordinate, pending: AuthoredCoordinate },
    Quarantined { valid_through: u64, failure: SessionFailure },
}
```

`AuthoredCommand::Project` only accepts operations with a reviewed package mapping;
unsupported legacy extension semantics return Validation, not a lossy conversion.
Admission requires registered cooperative lease, qualified storage, validated host
authorization, live attachment/owner epoch, exact expected authored/Graph coordinates,
bounded command and no applicable freeze. No independent caller may publish while
another owner holds the package. All receipt constructors stay private.

GUI, CLI, headless, scripting, automation and AI agents use the same Rust authority.
Operation identity binds canonical semantic intent, not transient transport or owner
epoch; retry after reattachment may resolve the original operation under fresh
authorization. Provenance records original acceptance and is not overwritten by a
retrying actor. The implementation must define a bounded canonical intent digest
separate from the request's transient attachment envelope; the fixture/PS1 request
encoding is not promoted to this protocol. Same ID/different intent refuses;
authorized original-receipt lookup precedes stale-coordinate rejection. Reject
unauthorized lookup without leaking existence or content. Revocation never erases
accepted intent or permits its duplicate execution. Unknown outcomes reconcile
before later admission. Undo groups belong to their originating client/principal;
cross-client undo is not implicitly authorized.

```rust
trait PackagePlanner {
    fn plan(&self, verified: &VerifiedClosure, changes: &DurableChanges,
            ids: CheckpointIds) -> Result<CheckpointPlan, SessionFailure>;
}
trait PackageWriter {
    fn acquire(&self, locator: &LocalLocator,
               expected: &PackageIdentity,
               authorization: &AuthorizedContext)
        -> Result<RegisteredCooperativeLease, SessionFailure>;
    fn publish(&self, lease: &mut RegisteredCooperativeLease,
               plan: &CheckpointPlan) -> PublishOutcome;
    fn reconcile(&self, intent: &CheckpointIntent) -> ReconcileOutcome;
}
trait RecoveryJournal {
    fn append_mutation(&mut self, prepared: PreparedMutation)
        -> Result<MutationReceipt, AppendFailure>; // Unknown distinct from rejected
    fn append_checkpoint_intent(&mut self, plan: &CheckpointPlan)
        -> Result<IntentReceipt, AppendFailure>;
    fn acknowledge_checkpoint(&mut self, saved: &SavedReceipt)
        -> Result<(), AppendFailure>;
    fn recover(&mut self, package: &VerifiedClosure) -> ReconcileOutcome;
    fn compact(&mut self, through: &SavedReceipt) -> Result<(), SessionFailure>;
}
trait ProjectSessionCoordinator {
    fn attach(&self, target: ProjectTarget, authorization: AuthorizedContext)
        -> Task<AttachmentSnapshot>;
    fn submit(&self, authorization: &AuthorizedContext, request: MutationRequest)
        -> Task<MutationReceipt>;
    fn lookup_operation(&self, authorization: &AuthorizedContext,
                        attachment: AttachmentToken, operation: CommandId)
        -> Task<OperationOutcome>;
    fn flush(&self, authorization: &AuthorizedContext, request: FlushRequest)
        -> Task<SavedReceipt>;
    fn detach(&self, attachment: AttachmentToken) -> Task<DetachOutcome>;
    fn observe(&self, authorization: &AuthorizedContext, attachment: AttachmentToken,
               after_event: Option<u64>) -> EventStream<SessionEvent>;
}
// Per-client native activation, not the lifetime of all per-package clients/jobs.
trait ClientActivationCoordinator {
    fn prepare_activation(&self, target: ProjectTarget) -> Task<ActivationPreparation>;
    fn confirm_activation(&self, confirmation: ConfirmationToken)
        -> Task<ActivationOutcome>;
}
```

`Task`/`EventStream` describe async semantics, not a selected runtime/UniFFI ABI.
ConfirmationToken binds source attachment generation, target identity and pre-dialog
saved receipt; cancel has no release side effect. ActivationPreparation is
SameProject, ConfirmationRequired, StopRunRequired or Failed. StopRunRequired needs
explicit user intent and terminal run receipt before confirmation/freeze proceeds.
ActivationOutcome is Activated, RetainedCurrent, or RetainedReadOnlyRecovery, never
an empty-success result. Events carry monotonically increasing event sequence and
owner epoch and attachment generation; Swift discards stale events. Reconnection
returns an exact snapshot/cursor or explicit gap requiring resynchronization.
Runtime cancellation is a neutral
host capability, not an AppKit selector. `SessionFailure` carries typed category,
phase, retry class, last proven receipt and redacted diagnostic code.

Platform IO seam must expose pinned directory/file handles, identity checks,
exclusive lifetime lock, create-exclusive, immutable no-replace publication,
full file flush, directory flush, atomic HEAD replacement, safe unlink-by-owned-pin
and capability qualification. Fault injector wraps each operation and reports
before/after/error/unknown. Domain code receives no OS-specific descriptor numbers.

`RegisteredCooperativeLease` is opaque evidence of validated registration, access,
storage qualification and held stable OS lock. Storage does not assert exclusion of
arbitrary noncooperating writers. Chosen qualified paths are admitted in place;
no managed-root requirement. The compiled interface now requires an opaque
`RegisteredCooperativeLease` for every mutation and separates primitive policy from
admission; no production constructor, registrar or adapter is implemented. Its
exact expectation check is necessary, not authorization or OS lock revalidation.
Competing direct acquisition returns WriterBusy while any owner holds the lifetime
lease. This ensures safety, not agent progress through GUI ownership. Routing or
coordinated handoff, owner location, IPC/ABI and fairness implementation are deferred;
never steal a lock by elapsed time or enable overlapping ownership.

Detach freezes/drains only that client's accepted work and pending-input contract;
it does not drop other attachments, cancel their jobs or revoke package ownership.
Flush is satisfied by a verified receipt including its finite target sequence, even
if later accepted work keeps global status Saving. Owner release requires separate
drain/reconciliation and fencing; it is not equivalent to window close. Authorization
is revalidated at admission under its policy; accepted durable operations remain
recoverable after client disconnect. Host credentials/bookmarks stay outside
canonical mutation/provenance evidence. Voice/chat workflow proposals may later use
this same boundary; planning and approval UX are not specified in PS2.

## Brand-neutral configuration boundary (BR0 prerequisite)

These are review-only additions to the proposed boundary, not implemented types.
BR0 must pass before PS1 can implement session/writer contracts. Public values are
validated/generated from `config/product-identity.json`, including any added fields;
callers cannot supply unrelated fallback literals or derive durable IDs from names.

```rust
struct PackageNamingPolicy {
    write_extension: ValidatedExtension,
    legacy_read_extensions: BTreeSet<ValidatedExtension>,
    document_uti: ConfiguredDocumentUti,
    document_display_type: String,
}
struct SessionStorageRoots {
    application_support: ConfiguredRoot,
    cache: ConfiguredRoot,
    journal: ConfiguredRoot,
}
struct PublicProductConfiguration {
    display_name: String,
    short_name: String,
    product_name: String,
    executable_name: String,
    bundle_id: String,
    package_naming: PackageNamingPolicy,
    roots: SessionStorageRoots,
    trust: GeneratedTrustCoordinates, // callback/audience/Keychain/Apple/service
    user_agent: String,
    urls: GeneratedPublicUrls,
}
```

Extension validation rejects separators, traversal, empty/ambiguous suffixes and
unsafe aliases; filename byte limits use the actual configured suffix and target
filesystem constraints. Alias matching admits inspection, never bypasses package
validation or grants write capability. Writer planning consumes the naming policy;
legacy-alias packages remain read-only until an explicitly reconciled outer-name
cutover. Format codecs do not consume product naming configuration. Session and
journal paths append UUID components beneath configured roots; display names never
supply identity/path components. Directory relocation preserves incarnation IDs and
operation dedupe, with an explicit interrupted-cutover recovery contract.

No schema delta mass-renames internal `photara.*` identifiers, migration history,
operation IDs or deployed PostgreSQL schema names. Configured document UTI and
public trust names are distinct from persisted format/schema identifiers. Retain
reviewed legacy read/registration aliases; internal namespace changes need their
own migration/alias proposal. BR0's security enrollment and sign-in/directory
transition contract is distinct from package-byte compatibility and may require
new production trust coordinates. No DDL, live enrollment or credential migration
is authorized here.

## Unnumbered local schema deltas (review only)

The framed journal is the source of truth. A disposable/rebuildable index may later
use SQLite; this is a logical schema inventory, **not executable DDL**. Do not add
these to the production migration registry or assign migration ordinals.

| Logical record | Key / fields / constraints |
| --- | --- |
| PackageBinding | incarnation ID; Project/Library IDs; locator reference; volume/root identity; manifest digest; capability profile. Path not unique identity. Host-only permissions |
| JournalSegment | journal ID + segment sequence; predecessor checksum; base HEAD; frame range; owned file pin; checksum; state sealed/active/quarantined. Consecutive sequence |
| OperationReceipt | journal ID + operation ID unique; canonical intent digest; sequence; before/after coordinates; originating undo group; credential-free actor/grant/scope evidence. Different intent forbidden; no reusable credentials |
| CheckpointAttempt | write ID unique; immutable commit ID; expected/candidate HEAD; through-sequence; byte manifest; phase planned/unknown/verified. Cannot change identity on retry |
| SessionView | device + Library + Project + Graph; schema version; viewport/selection/panels. Selection must resolve against opened Graph; missing IDs dropped with safe defaults |
| ActivationIntent | activation ID; attachment/generation; current/target identities; current saved receipt; rollback view; stage; target receipt. Single in-flight activation per client activation coordinator |
| ActiveSession | device + workspace slot; active Project/Graph; activation ID. Change only after target verified and view restoration ready |

Activation receipt and ActiveSession update are one durable local transaction (or
one atomic file snapshot); rollback capsule persists until successful next session
establishment. UI preference corruption falls back to defaults without modifying
Graph bytes. Journal corruption cannot be treated as a preference fallback.

**Package format delta:** none required for initial checkpoint protocol: existing
commit parent, package_revision, write_id, authored/history/inventory references and
HEAD suffice. Journal coordinates remain local. No extra required feature is
silently added. Retention beyond current reader bounds requires a separate format/
reader contract, not deleting ancestry behind 1.1's back. The approved
[sealed-root direction](../../PS2_RETENTION_STORAGE_DECISION.md) preserves retained
closure/recovery/dedupe and operation provenance across root turnover. No encoding,
feature/version number, migration or production threshold is chosen here.

**Cloud/PostgreSQL delta:** none. No Graph-byte column/table, catalog revision bump
per gesture, ownership mutation or new Neon authority. **Existing SQLite delta:**
none executed/reserved; selection of new isolated journal/index storage must be
reviewed in PS1/PS2. Creation receipts, CXT context recovery, sync outbox and runtime
effect receipts remain distinct protocols and cannot be reused as Graph operations.
