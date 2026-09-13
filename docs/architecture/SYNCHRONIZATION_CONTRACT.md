# Synchronization and API contract

**Current baseline — 2026-09-12:** the user superseded D19 R1 physical-name
preservation because Generation Two is unshipped. Domain, package and SQL names
below use Library consistently. This is a clean baseline rewrite, with no rename
migration, alias, shadow column or live database change. See the
[rebaseline authority and evidence](LIBRARY_NOMENCLATURE_REBASELINE.md) and
[current execution order](../ROADMAP_0_2_EXECUTION.md). CXT3a is complete; local
D19 execution/app initialization and PostgreSQL/RLS remain separate CXT3b/c gates.

## D18 extension — pending contract/schema review

[D18](TYPED_CONTEXT_AND_EXPRESSIONS.md) proposes versioned allowlisted Library
variable commands/poststates and permitted immutable captures. Existing sealed
bytes, local/server revision separation, receipt idempotency, offline chains,
tombstone reservation and explicit conflict/rebase rules still apply. No new
command is accepted merely because it fits `extensions`; CXT2 must freeze its
schema/capability negotiation and CXT3 must prove local boundaries. No D18 API or
DDL is implemented here. Expression ASTs are data, never service-executed code.

Library authorization covers definition/value reads and transitive dependency
resolution; a portable Project's capture conveys no live Library grant. SecretRefs,
provider tokens, device/root bindings and raw secret digests never sync/export.
Personal/restricted data require explicit allowed projections; omitted required
dependencies remain visibly unavailable. D17 imports report/rebind typed variable
IDs and AST dependencies transactionally without reactivating grants. Local
Set Variable application and later server acceptance/conflict are separate facts;
an offline receipt cannot assert cloud acceptance. No cross-authority atomic
Library-plus-package write or CloudKit evaluator is implied.

Status: S5 proposal prepared for review, 2026-09-11. This defines a proposed
version-one protocol and reconciles S2–S4; it is not an implemented API, deployed
service, migration or approval. S6 supplies executable fixtures; S7 is the user's
separate schema/policy approval gate.

Read with [S1 logical model](LOGICAL_DATA_MODEL.md),
[S2 package schema](PROJECT_PACKAGE_SCHEMA.md),
[S3 local SQLite](LOCAL_SQLITE_SCHEMA.md),
[S4 service PostgreSQL](SERVICE_POSTGRESQL_SCHEMA.md), and
[Storexa boundary](STOREXA_INTEGRATION.md). This is clean generation two.
Existing Neon/v0.1.3 import is optional later salvage, never a schema or release
gate. No service connection or database operation was used to prepare this slice.

## V1 decisions and authority

- Sync replicates typed Library records and explicit discovery metadata, not
  SQL rows, a global file archive, authored Graphs or live package contents.
- Local edits commit durably before network access. The server accepts/rejects
  versioned typed commands against per-record expected server revisions.
- One command is one atomic multi-record server batch. Whole-command conflicts
  are explicit; v1 performs no automatic field merge or last-write-wins.
- Receipt acknowledgement and ordered-feed application are different milestones.
  An acknowledgement never jumps the feed cursor or erases later local edits.
- V1 feed pages contain **exactly one complete batch**; no-change replies contain
  no page and do not advance the cursor. This makes immutable page identity
  unambiguous for S3's single pending inbox.
- V1 uses bounded single-response snapshots, not an open transaction held across
  interactive page requests. Oversized Libraries return an explicit unsupported
  snapshot-size error until an export protocol is separately designed.
- Recommend explicit atomic Location Kind claim transfer for v1. Retired IDs and
  term provenance remain; one Library/concept key has exactly one current
  owner. This replaces the earlier S3/S4 physical reservation limitation.
- One Photara Cloud target per Library. CloudKit is a later independent sync
  adapter, not another writer to the same target and not a SQL/HTTP translation.

Project saves/runs remain S2 package operations. Neither a server receipt nor a
catalog observation proves that a package HEAD was published or an external
node effect occurred. Media bytes and package publication are separate recovery
boundaries; there is no cross-store ACID transaction.

## Endpoint and authentication boundary

All paths below are proposed relative paths, not a live hostname. Environment
selection is an explicit developer/user configuration: On This Mac or a named
Photara Cloud environment. Selecting an environment cannot retarget an existing
Library's pending queue to a different account/service silently. Endpoint
allowlists and TLS validation prevent sending a bearer token to an arbitrary
URL or redirected host.

The API validates the Auth0 issuer, audience, signature, expiry and authentication
policy. Exact issuer/subject maps to AccountId; emails and People do not.
AccountId/IdentityId and action permissions come from that validation, never from
request JSON. Registered DeviceId is attribution, not authentication. Each
Library endpoint rechecks current Account/identity/membership; possession of a
cursor, object digest, receipt ID or package is not permission.

The service uses the S4 authorization helper and transaction-local scope before
domain SQL. Owners/admins/editors can write Library/catalog under their defined
roles; viewers read. Membership/owner/billing controls use their separate
controller and audit path. Developer/paid entitlements do not confer membership.
Offline clients may continue authorized local work from their existing data,
but cached permissions cannot authorize a new server operation.

| Method/path | Contract |
| --- | --- |
| GET /v1/capabilities | Authenticated protocol/codec/features, supported normalizers and size limits |
| POST /v1/libraries/claim | Explicit Account-scoped idempotent claim of a new local LibraryId; first owner + stream |
| GET /v1/libraries/{library}/access | Current membership, Library state and bounded entitlement/cache revisions |
| POST /v1/libraries/{library}/mutations | One sealed typed command; final immutable receipt or transport/protocol error |
| GET /v1/libraries/{library}/mutations/{mutation} | Authorized receipt lookup; absence is not proof an in-flight request cannot commit |
| GET /v1/libraries/{library}/changes?cursor={opaque} | Exactly the next complete batch, or 204 with no cursor change |
| POST /v1/libraries/{library}/snapshots | Bounded full snapshot with epoch/high-water cursor |
| POST /v1/libraries/{library}/acknowledgements | Monotonic acknowledgement of a locally committed cursor |
| POST /v1/libraries/{library}/media/uploads | Authorized staging upload session for declared digest/size/type |
| POST /v1/libraries/{library}/media/uploads/{upload}/complete | Verify uploaded bytes and finalize immutable object availability |
| GET /v1/libraries/{library}/media/{digest}/download | Reauthorize access and issue a short-lived download capability |

Identity linking, invitations and plan administration use separate authenticated
control contracts, not Library mutation kinds or client-authored membership rows.
Claim uses an Account-scoped ClaimId and immutable request/response receipt; it
must never turn an existing LibraryId collision into ownership. A retry of a
successful claim returns that Account's exact prior result. Joining uses an
already authorized Library identity, not claim or an email-based identity
merge. Local-only mode never fabricates a service Account/membership.

## Canonical bytes, versions and limits

Protocol media type: application/vnd.photara.sync+json; version=1.
The protocol field is photara.sync.v1; records and commands additionally carry
their own schema/version identifiers. Canonical codec is the S2
photara.canonical-json.v1 contract, **not** an assertion of RFC 8785 compatibility.

Require UTF-8, no BOM, no duplicate object keys, compact JSON, deterministic
lexicographic key ordering and the frozen serde_json string/number behavior.
Arrays retain semantic order; set-valued fields have a prescribed sorted,
duplicate-free order before serialization. Hashing never normalizes user text.
Term normalization is a separate typed operation with a pinned policy version.
Reject malformed Unicode, non-finite/unrepresentable numbers and unknown required
features before applying anything. S6 must freeze the complete byte vectors,
including non-ASCII keys and floating-point edge cases; ordinary JSON.stringify
is not the cross-language specification.

UUIDs are lowercase hyphenated strings; hashes are 64 lowercase hex characters.
All new counters, byte lengths, per-record revision components and sequence
components are canonical decimal strings, never JSON numbers. Schema versions
and bounded structural ordinals are JSON integers. Instants use UTC RFC 3339,
Z and exactly three fractional digits; service writers store that same precision.
Calendar schedules retain S2 tagged date/zoned-interval forms.

Photara-Body-SHA256 carries the SHA-256 of the exact decompressed canonical body
bytes; it is not included inside the bytes it hashes. Persist and replay those
bytes, not a newly formatted equivalent. A digest detects accidental corruption
and identifies content; it is not authentication or authorization. Transport
headers such as trace IDs/replay indicators are outside the immutable body.
Neither PostgreSQL JSONB rendering nor a SQLite reconstruction is a canonical
serializer.

Proposed negotiated ceilings (servers may advertise lower, not silently higher):

| Object | V1 maximum |
| --- | --- |
| Mutation request / individual typed post-state | 1 MiB each |
| Changed roots in one command | 1,000 |
| Canonical server batch | 3 MiB, narrower than S4's storage ceiling |
| Receipt / single-batch feed response | 4 MiB each |
| Complete snapshot | 16 MiB and 10,000 aggregate records |
| String keys / labels / descriptions | Per-record schema bounds frozen by S6 |

Bounds apply after decompression and before allocation-heavy validation. Reject
an atomic operation that exceeds limits; do not silently split a merge that
needs one transaction. Snapshot responses are assembled under a bounded
repeatable-read transaction, committed before the response is sent. Exceeding
size/time limits returns snapshot_too_large or retryable busy—not partial data
labelled complete. Large-Library snapshot export is a later explicitly scoped
feature; S7 must approve this initial product limit.

## Identity and revision vocabulary

| Value | Meaning and comparison |
| --- | --- |
| Local revision | Positive SQLite aggregate revision; local CAS only |
| Server revision | Opaque sr1:<positive-decimal> token for one Library/kind/ID; compare expected equality |
| MutationId | One immutable command intent/request identity scoped to its Library |
| ClaimId | Account-scoped creation intent, separate from Library MutationId |
| Stream epoch | Server Library feed generation; not a record revision |
| Batch sequence | Commit order within an epoch; never a per-record revision |
| Cursor | Opaque authenticated position in that Library/epoch |
| Project/Graph revision, CommitId/checksum | Package/Graph provenance only; never a Library precondition |

The client may parse its own local revision, but does not order arbitrary opaque
server tokens. The sr1 adapter validates canonical decimal values in
1..9223372036854775807 and converts only at the repository boundary. The service
increments each changed root exactly once. A created server record starts at 1
regardless of how many local edits preceded upload.

Create commands carry the immutable original entity-created-at fact from local
creation; the service validates and preserves it rather than replacing it with
upload time. Server acceptance time lives in receipts/audit and server updated_at.
Local updated_at and server updated_at are informational clocks, not merge
preconditions; owned-field comparisons exclude revision/delivery metadata.

A local working root may be revision 8 while sync_object_state records a server
base that corresponds to local revision 5. That is normal pending work, not a
request to overwrite revision 8 with revision 5. Keep base state, working state,
immutable local changes and delivery state distinct.

## Typed mutation envelope

A sealed request contains exactly:

| Field | Rule |
| --- | --- |
| protocol, codec | Fixed negotiated v1 identifiers |
| library_id, stream_epoch | Must match the route and current target; epoch prevents replay into an unknown reset generation |
| mutation_id, device_id, created_at | Stable intent/attribution facts, preserved through retries |
| required_features | Sorted unique namespaced feature identifiers |
| preconditions | Sorted unique (entity kind, ID) expectations for every affected existing/new root |
| command | One registered typed command with closed owned fields |
| attachments | Typed media descriptors and/or catalog observations required by this command |
| extensions | Namespaced optional data; no undeclared behavioral requirements |

Preconditions are either {mode:"absent"} or
{mode:"revision",server_revision:"sr1:42"}. Absence is an explicit expectation,
not a missing key or wildcard. Wire preconditions never contain predecessor
MutationIds; the local scheduler resolves those only after the preceding result
has been applied through the ordered feed. The declared affected-root set must
match the service-computed set under its lock; newly introduced dependents yield
affected_set_changed instead of silently editing unlisted records.

V1 command families are typed create/replace/tombstone/merge operations for
Person, Organization, relationship, Location Kind, Location, StorageRoot,
ProjectCatalog and ProjectLocator. catalog.observe.v1 appends a derived
observation while advancing the catalog root. kind.reconcile_collision.v1 is
the explicit collision-resolution operation described below. Library claiming,
membership, billing and media-upload authorization are separate controllers.
No generic table name, SQL, JSON Patch interpreter, executable script, Graph
operation or node effect is accepted in a sync command.

A replace command owns only that aggregate's declared fields, including its
label/capability/term children. It cannot change identity, owner, server revision,
created timestamp or lifecycle through an arbitrary payload. Lifecycle changes
have separate typed commands. The server revalidates canonical keys and aliases
against its pinned normalizer; mismatched client-derived keys are rejected, not
silently repaired into another meaning.

Illustrative typed request (readable formatting; not itself canonical bytes):

```json
{
  "protocol": "photara.sync.v1",
  "codec": "photara.canonical-json.v1",
  "library_id": "10000000-0000-4000-8000-000000000001",
  "stream_epoch": "20000000-0000-4000-8000-000000000001",
  "mutation_id": "30000000-0000-4000-8000-000000000001",
  "device_id": "40000000-0000-4000-8000-000000000001",
  "created_at": "2026-09-11T12:00:00.000Z",
  "required_features": [],
  "preconditions": [
    {
      "entity": {"kind": "person", "id": "50000000-0000-4000-8000-000000000001"},
      "expect": {"mode": "revision", "server_revision": "sr1:7"}
    }
  ],
  "command": {
    "type": "photara.person.replace.v1",
    "entity_id": "50000000-0000-4000-8000-000000000001",
    "record_schema": 1,
    "value": {
      "display_name": "Alex",
      "description": "",
      "aliases": [],
      "capabilities": ["photara.photographer", "photara.stylist"],
      "labels": [],
      "thumbnail": null,
      "extensions": {}
    }
  },
  "attachments": {"media": [], "catalog_observations": []},
  "extensions": {}
}
```

The ASCII-only example above has a reference canonical length of 837 bytes and
SHA-256 f219f8c0bf4ccd85235f24bbb3e291ecf7aafdf0c8e28413e286255201e4dc42.
This was checked statically; S6 must match it with the actual Rust codec and
extend coverage to the non-ASCII/number edge cases before freezing the codec.

## Local acceptance, sealing and offline chains

A local command commits its typed root/children, expected-local-revision CAS,
immutable local command/change records and an eligible outbox entry in one
SQLite transaction. It can succeed with no network. A new cloud target
association first resolves claim/join and its initial snapshot; edits made
before association remain durable local intent, not assumed server history.

For each affected root, store either the known server baseline, an explicit
absent expectation, or the preceding pending local command. Dependencies form a
DAG; a multi-root command waits for every predecessor. Commands with overlapping
roots preserve order. Independent roots can proceed, although the first server
implementation serializes accepted Library writes.

Before sealing, ensure required media is remotely verified, resolve all
predecessor results through the feed, and validate that the command still
expresses the user's intent on that base. Then persist the exact wire bytes and
hash once. After sealing, no field—including timestamp, epoch, precondition,
device or optional extension—may change under that MutationId. Signing a new
Auth0 access token changes the HTTP header, not the body.

An unsealed command can be discarded/replaced explicitly. Preserve its immutable
intent and record a local disposition plus replacement link; do not claim a
server rejection. A sealed request, even if its response is unknown, must first
reach an authoritative final receipt by replay/lookup. V1 has no cancellation
endpoint that can safely outrun an already queued network request. Undo after
acceptance is a new CAS command against the accepted state.

An offline chain A -> B behaves as follows:

1. A and B have already produced local working revisions; B is not yet sealed.
2. Submit A unchanged until a final result is known.
3. If accepted, mark delivery acknowledged but leave the feed cursor/base alone.
4. Apply A's ordered batch when reached; update the server base without erasing
   B's local working overlay. Then seal B with A's resulting server revision.
5. If A conflicts/rejects, block B. Rebase/discard requires a recorded resolution,
   new MutationIds and a rebuilt dependency chain. Never secretly retarget B.

A transient sign-out, connection loss or app restart does not delete the chain.
A different authenticated Account cannot inherit a previously sealed request
without an explicit reassociation/recovery decision.

## Receipts and idempotency

The receipt's canonical body has protocol, Library/MutationId, request digest,
outcome, per-root result versions/digests, and either accepted {epoch,sequence,
batch_sha256} or a typed error/conflict result. Accepted receipts do not need to
duplicate the whole batch. No volatile replay flag/server-now field is inside
the immutable body. Header-only replay diagnostics may vary.

The service validates current authorization, locks the Library and checks
(library,mutation_id) before execution. Identical actor/request bytes return
the stored result. Different bytes/actor return idempotency_conflict without
leaking the previous body. Existing authorized receipts may still be returned
after an epoch change; an unknown receipt plus mismatched epoch never executes.

Receipt persistence is atomic with accepted roots, attachments, ordered changes
and stream increment. Rejected/conflicting commands persist only their final
receipt; transient infrastructure/auth failures are not invented domain
rejections. There is no committed processing receipt. The client durably stores
the verified response bytes/hash separately from its immutable local intent,
then updates outbox delivery state. Receipt storage is not a substitute for
feed application or proof of a package effect.

| HTTP/result | Durable domain result? | Client behavior |
| --- | --- | --- |
| 200 accepted receipt | Yes | Store receipt, acknowledge delivery, drain ordered feed |
| 409 conflict receipt | Yes | Store exact conflict; block affected descendants |
| 422 rejected receipt | Yes | Record validation failure; user correction creates new intent |
| 409 idempotency_conflict | No new result | Stop; preserve both local evidence and diagnostics |
| 401 authentication required | No | Pause target, refresh/re-authenticate; same sealed body |
| 403 access revoked | No | Stop uploads/downloads; retain local work; do not impersonate another Account |
| 404 receipt absent | No | Not proof of cancellation; replay same request to settle an unknown outcome |
| 410 snapshot_required / epoch mismatch | No new command result | Preserve pending work; reconcile receipts, then snapshot flow |
| 413 size limit | No | Do not split atomic operations implicitly |
| 429, retryable 5xx, timeout/disconnect | Unknown or none | Bounded backoff; same MutationId/bytes; respect Retry-After |
| Digest/schema mismatch | No local application | Quarantine intact response; report protocol failure |

Non-success responses declare whether they contain a stored receipt. Do not
infer that every HTTP 409/422 was persisted. Retryable transport envelopes and
error trace IDs are not immutable receipts. No exactly-once network delivery is
claimed; idempotent server acceptance is the guarantee.

## Feed, inbox and acknowledgement

A feed batch has immutable Library/epoch/sequence, originating MutationId,
ordered root changes and typed attachments. Each change has ordinal, entity
kind/ID, server revision, change kind, record schema, canonical typed post-state
and digest. The post-state includes the complete aggregate child sets; it never
means "join this historical ID to the latest row."

The feed wrapper contains protocol, codec, Library/epoch, cursor_before,
cursor_after and exactly one complete batch. It excludes live high-water,
server-now and has-more flags that could change bytes for the same position.
If no next batch exists, return 204 and do not persist an empty inbox row.

Cursors are opaque authenticated encodings of version, Library, epoch,
sequence and signing-key identifier. Decimal sequence zero denotes a snapshot/
initial high-water with no applied batch. The service validates scope, bounds,
signature and current access. For a request cursor, its next cursor uses the same
supported signing-key generation, giving deterministic replay for that chain.
Retiring that key yields snapshot_required, not a newly signed different body
masquerading as the same page. Signing-key retention/rotation is operational
policy; cursor tokens confer no access on their own.

Identity of an inbox item is (target,epoch,sequence), independent of its cursor
string or local InboxId. Repeated identity requires the same canonical batch
digest. A cursor token is not a primary semantic batch identity. This is an
explicit S3 refinement.

Receive one complete response, verify protocol/bytes/digests/identity, then
persist it before application. In one local write transaction:

1. Check exact expected cursor position and all typed schemas/features.
2. Validate media descriptors and catalog attachments; stage necessary typed
   rows before roots with FKs.
3. Apply the entire batch or none. Update server-base state and any conflict-free
   working roots with normal local revision increments.
4. Preserve local pending overlays; correlate an own-command echo with the
   existing intent instead of applying that intent a second time.
5. Record remote-origin local changes only where working state actually changes,
   mark inbox applied and advance cursor atomically. Never enqueue remote echo.
6. Only after commit send the server acknowledgement.

A remote batch's wire MutationId is provenance, not necessarily the SQLite
mutation primary key. Allocate a distinct local ingest ID for unrelated remote
commands and retain the wire origin; this avoids collisions across Libraries
and with an existing own-command intent. Correlate own echoes using target,
Library, actor/device, MutationId and receipt digest, not UUID coincidence.

If receipt arrives before feed, it cannot skip earlier batches. If feed arrives
first, verify its own-command receipt/origin before resolving the local outbox;
do not create another local edit. A known older/equal server post-state never
regresses base or working state. A conflicting same-revision/different-digest
post-state is corruption/protocol error. Descendants seal only after the relevant
predecessor base has reached this ordered application point.

A page with unsupported required schema/features remains intact and unapplied.
A page with local conflicts is quarantined as a whole; the client does not apply
unrelated roots from that batch and advance past the conflict. One pending inbox
item bounds the v1 state machine. Other already known independent outbound
commands may proceed, but user-facing freshness must show the blocked inbox.

## Conflict and rebase rules

Compare base, local working intent and incoming server state using typed values.
V1 never resolves same-field or structural conflicts by wall-clock timestamp.
A semantic no-op (equal typed values) may be recognized without manufacturing a
new root edit; it still records the server-base/cursor transition.

| Case | V1 resolution |
| --- | --- |
| No pending local work for root | Apply server state; normal local revision; no outbound echo |
| Own accepted predecessor with unsealed descendants | Advance base; retain/replay known local overlay; descendants remain conditional |
| Remote update overlaps unsealed local intent | Quarantine; user chooses remote, rebased local intent, or explicit typed merge |
| Remote update overlaps sealed unknown request | Settle that request first; do not discard or rewrite it |
| Remote tombstone vs local edit | Preserve intent; explicit discard or new identity/approved merge; no resurrection |
| Merge/delete changes dependent-root set | Reject stale plan; recompute full impact and ask for a new command |
| Same concept claimed by another Kind ID | Explicit collision reconciliation below; never insert a duplicate key |
| Membership/entitlement lost | Preserve local work; pause gated transport; permission is not a merge policy |

A resolution is itself durable: record the conflict, base/local/remote evidence,
chosen action and any replacement MutationId. Choosing remote marks unsealed
superseded work as such and applies the remote base; choosing local creates a
new command against the currently observed server revision. Existing immutable
requests/history are never edited. Rebasing several pending intents produces a
new dependency chain; state exactly which intents changed and which user-owned
values will be retained.

Local changes concurrent with planning are caught by local revision CAS. Never
hold a SQLite transaction while waiting for user input, server response or media.
A later server conflict can still occur after a user approves a rebase; it
creates another explicit conflict, not permission to overwrite.

## Location Kind v1: atomic claim transfer with retained provenance

Recommend **supporting** canonical promotion and concurrent offline-creation
reconciliation in v1, with this narrow operation—not arbitrary term reassignment.

The current owner of a (LibraryId,normalized term key) is unique across
canonical and alias terms. An active/tombstoned Kind owns its current claims;
a merged Kind owns none and retains a redirect plus immutable retirement-term
snapshot. That snapshot records its pre-retirement canonical descriptor and
term keys/spellings/policy version. It is provenance, not another current claim.

For merge A -> B, the command captures expected revisions and all affected
Locations/relationships, then atomically:

1. Validates same Library, active B, no cycle and the precise dependent set.
2. Rebinds live references explicitly, capturing affected root revisions.
3. Captures A's retirement terms and marks A merged -> B.
4. Transfers A's existing term claim rows to B; the library/key PK never changes.
5. Optionally promotes one now-owned term as B's canonical key/display.
6. Advances each affected root once and publishes one complete batch.

There is never a committed gap in reservation or duplicate claim. Intermediate
in-transaction states are allowed only by deferred FKs. A reader sees all old
ownership or all new ownership. Source canonical/term history and package
snapshots stay unchanged; future merges may extend the redirect chain without
rewriting historical terms. Tombstoned terms remain reserved; v1 has no free
reassignment, unmerge or resurrection command.

Physical reconciliation in both proposals uses two generated columns on Kind:

- claim_owner_id = KindId unless state is merged, otherwise NULL; unique with
  LibraryId. Current term owner FK references this eligible-owner pair,
  deferred until commit, so a merged Kind cannot retain any current claims.
- required_canonical_key = canonical_key unless state is merged, otherwise NULL.
  The canonical same-owner FK uses this field; merged historical descriptors no
  longer force a source-owned live claim.

retirement_terms is a typed immutable snapshot for retired rows, NULL while
active. Current term owner may change only during a validated merge from its
old owner's direct redirect to an active target. Key, Library and policy stay
immutable. Canonical/alias normalization remains the same pinned policy;
Beach/beach/beaches cannot become separate concepts in either backend.

If offline client A creates Kind A while the server already owns that concept
as Kind B, A's create receives a final concept_claim_conflict. The user may choose
Reuse B (with explicit alias additions if needed). The replacement
kind.reconcile_collision.v1 command expects A absent on the server and B at its
known revision, records A as a merged historical alias identity -> B, and
validates its retired terms against B's claims/explicit approved additions.
It does not first create another active owner of the concept. This preserves
references to the never-published local A identity without pretending its failed
create was accepted. Dependent local intents are explicitly rewritten/rebased
to live B; existing package snapshots remain readable and are not silently edited.

If the server already knows A, use ordinary merge with A's expected revision.
A generic "upsert kind by spelling" is never allowed. Same spelling with truly
different meaning requires explicit disambiguated concept terms, not bypassing
the shared unique key. This recommendation is pending S7 approval; S6 must prove
both physical schemas against the same collision/promotion fixtures.

## Snapshot bootstrap and reset

A snapshot response contains protocol/codec, SnapshotId, LibraryId, epoch,
high-water cursor, complete typed current records (including tombstones/merged
identities and retirement terms), media descriptors, catalog/locator metadata
and opted-in bounded observation attachments. It contains no device bindings,
local recovery intents, pending commands, provider grants or package bytes.
A complete inventory/digest covers exactly those records and attachments.
SnapshotId identifies one response/export, not a reusable live query session.

The server reads records and stream position from the same repeatable-read
snapshot, builds the bounded canonical response and ends the SQL transaction
before sending. A failed/oversized response is never a partially usable snapshot.
A retry may yield another SnapshotId/high-water; each response is internally
complete. V1 has no paginated snapshot continuation and no automatic retention
pruning; a larger export protocol needs an explicit later design.

The client stages the complete verified snapshot durably before installation.
It records target, epoch/high-water, payload digest and installation state; an
interrupted install is restartable. Installation is a single local CAS/repository
transaction after typed validation and conflict planning, not destructive
database replacement. Preserve device paths/bookmarks, catalog device selection,
local media/recovery, unsynced commands, immutable history and package snapshots.

For pending work, settle all sealed unknown outcomes first where possible.
Unsealed work is preserved as explicit intent, compared to the snapshot base
and rebased only through the conflict workflow. Concurrent local changes abort
the install plan and require replanning. Never label local pending rows "missing
remotely" and delete them. A previously synced identity unexpectedly absent from
a purported complete snapshot is lost_history_conflict in v1; preserve it and
investigate rather than inventing a remote tombstone.

After successful installation, advance the local base/cursor to the snapshot
high-water atomically and continue from the next feed batch. Send an
acknowledgement only after commit. Old inbox history remains identifiable by
epoch; it is not reapplied into the new snapshot.

Ordinary v1 operation does not reset epochs or prune history. Disaster restore,
key retirement or a later retention policy can require a snapshot. If server
history/receipts may have been lost, an old sealed command with no surviving
receipt is quarantined as historical_outcome_unknown; its old epoch prohibits
blind replay into the new generation. Recovery requires explicit evidence and
user/service coordination. A reset is not permission to retry a possible
external effect or erase unsynced data.

## Media, catalog, devices and privacy

Media bytes never appear in mutation/feed/snapshot bodies. Their typed descriptor
contains LibraryId, SHA-256, byte length, MIME type and optional dimensions.
A command advertising newly uploaded media waits for verified remote
availability; receiving clients may retain metadata while bytes are pending.

Uploads use a private staging key/session, expected digest/length and expiry.
Verify staged bytes before publishing an immutable final object. A still-valid
client upload URL must never be able to overwrite the final object after
verification; staging and final write capabilities are distinct. Repeated
finalization compares exact descriptors. Unknown finalization outcomes query the
same upload session, not blindly create another final object. Object-store
failure does not claim a SQL rollback erased remote bytes; retain cleanup/
reconciliation facts. Signed URLs are short-lived responses, never portable
fields or stored credential strings.

Catalog reports are typed attachments to catalog commands, not an alternate
Project document. They include ProjectId, locator, CommitId/checksum, package
revision, index schema, bounded title/counts/Graph summaries and chosen Library
snapshot summaries. The API derives reporter Account/device attribution.
Identical provenance/index identity requires identical projection digest;
disagreement is a report conflict, not proof the service can repair a package.

V1 cloud catalog observation upload is explicit. Default discovery sync carries
logical root/locator and minimal title/count/provenance; richer party/location
snapshots require explicit sharing within that Library. The projection sharing
profile is encoded as minimal or library-rich in the typed
observation attachment; minimal rejects nonempty party/location snapshots.
Cross-Library records snapshot details are omitted from cloud catalog reports in v1; the local
package retains them. No absolute paths, bookmarks, signed provider URLs,
credentials or local availability/selection fields enter attachments.
Schema allowlists reject structured secret/device fields; they cannot promise
to detect every secret a user deliberately types into a free-text description.

Revocation stops future server read/write/download operations and invalidates
new media capabilities according to their bounded lifetime. Already downloaded
bytes and offline packages cannot be remotely unlearned. Sign-out pauses sync,
clears tokens/security-scoped grants according to host policy, and preserves
local data and pending intent. Explicit local data removal is a separate user
action with backup/recovery consequences. An Account switch does not silently
rebind a queue or share another Account's history.

A missing/unmounted/duplicated package is a local locator condition, never a
Library tombstone or remote Project deletion. A catalog-hide command only changes
discovery preference. Package move/copy/fork/rebind semantics remain S2, and
durable cross-store recovery remains S3. Never infer package authority from the
latest server timestamp, largest package revision or successful API receipt.

## Failures, versions and alternate adapters

Retry transient delivery using bounded exponential backoff with jitter and
server guidance; stop on access loss, protocol corruption, unsupported required
schema or a durable conflict. Cancelled network work leaves a sealed request
unknown until its receipt is settled. Do not hold local/remote SQL locks while
waiting on network, UI or filesystem effects. Reconcile source evidence before
compensation; metadata acceptance is not external-effect evidence.

Protocol/record/command/codec versions are separate. Unknown required features
or major schemas pause application and preserve raw bytes. Unknown optional
namespaced extensions round-trip unchanged but cannot alter behavior. A client
that cannot preserve required fields becomes read-only for that record; it does
not replace the server record with its older reduced shape. Server changes to
canonical serialization require another codec/version, not a dependency upgrade
that silently changes signatures/digests.

Normalization-policy upgrades require shared vectors and collision reporting,
then an explicit Library-wide migration. They are not a background preference
toggle. Version negotiation never lets an old client write keys using a
different policy. Batch schema/canonical bytes remain immutable once published.

Storexa 0.2 supplies explicit SQLite/PostgreSQL lifecycle and transactions.
Photara owns HTTP, Auth0 policy, envelopes, SQL repositories, conflict resolution
and package recovery. Nodes receive scoped capabilities, not bearer tokens or
database handles. Selecting a cloud backend does not change node meaning.

CloudKit may later map the same typed aggregate intent/identity/provenance to its
own records, change tokens, conflicts and partial failures. It cannot impersonate
a Photara Cloud cursor/receipt, provide this service's atomic multi-root batch by
assumption, or implement SQL transactions. Enabling it requires a separate target
identity, capability matrix and conformance tests. V1 does not dual-write one
Library to Photara Cloud and CloudKit; migration between targets is explicit,
not a change of enum while pending outbox items exist.

## Required physical reconciliation and local durability

S5 refines the earlier physical proposals; their initial DDL was not a complete
implementation of this protocol. These are documentation-level schema changes,
not executed migrations:

| Store | Required durable representation |
| --- | --- |
| S3/S4 Kind | Generated eligible owner/canonical reference, immutable retirement terms, narrowly guarded claim-owner transfer |
| S3 inbox | Explicit epoch + batch sequence uniqueness, independent of opaque cursor token |
| S3 remote_mutation_receipts | Exact own-command server receipt bytes/hash, outcome and accepted batch reference |
| S3 local_mutation_dispositions | Append-only discard/supersession decision and replacement intent link |
| S3 outbox | Distinct superseded/discarded states for unsealed local intent; never fabricate server rejection |
| S3 sync_snapshot_installs | Durable complete snapshot bytes/hash, target/epoch/cursor and install state |
| S4 library_claim_receipts | Account-scoped claim idempotency independent of an already existing Library |
| S4 media_upload_sessions | Private staging-session identity/key, expected descriptor, expiry and verified completion |

Receipt/inbox/snapshot payloads are immutable once received; only their local
processing state changes. They must survive restart and cache eviction. A
snapshot is bounded metadata, not a media-byte archive. Durable local receipts
can be removed only under an approved retention policy; an acknowledged outbox
timestamp alone is insufficient to reconstruct an offline dependency result.

Local disposition guards reject discarding an unresolved sealed request.
Definitively rejected/conflicting sealed intents retain their original outcome
even when a replacement is authored; accepted intents are undone through a new
command, not relabelled superseded. Snapshot install and conflict resolution
remain typed repository transactions with expected local revisions.

The generated Kind columns constrain final transaction state; transfer guards
only allow an old owner that is now merged directly into the new active owner.
Retirement snapshots are preserved across future merges. These precise
amendments must parse and pass shared runtime fixtures before S7; no independently
shipping local/service interpretation is permitted.

## S7 social-profile and export additions

[D16/D17](SOCIAL_PROFILES_AND_LIBRARY_EXPORT.md) add `social-profile` as a typed
Library records root with create/replace/tombstone commands, ordinary expected
revisions, accepted post-states, receipts, whole-batch feed and snapshot handling.
Negotiate required feature `photara.social-profiles.v1`; old clients must stop
atomically on unsupported records, never drop profiles while acknowledging a
batch. Existing no-profile canonical vectors remain byte-identical. Profile
schema version 1 includes one typed Person/Organization owner, exact optional
provider subject namespace/ID, mutable display facts and bounded safe provenance.
Manual handles/URLs work offline without OAuth or automatic lookup.

Profile edits do not implicitly edit their parent. Person/Organization retirement
or merge explicitly includes every affected active ProfileId/revision and retires
it; bound subjects remain reserved and cannot be cloned onto the target. Manual
unbound successors require new IDs/provenance in the same atomic command. A future
bound-subject transfer requires separate policy approval. Validate parent availability
and Library-wide scoped subject uniqueness under the Library lock. A cross-owner
bound subject match rejects the command; manual handle matches warn, never merge
identities. Imported/provider-authorized provenance grants no access.

Never replicate tokens, private provider connection identifiers, signed avatar
URLs or raw provider responses. Provider refresh/consent and avatar promotion are
explicit revision-checked operations. Cloud avatar attachment requires rights for
that transfer; cache-only/expired/disallowed bytes stay out. Project snapshots
carry selected profile display facts through ordinary typed party assignments,
not social credentials/subject identifiers or independently writable profiles.
Minimal catalog reports gain no social fields automatically; rich same-Library
sharing still needs opt-in. Provider deletion/expiry must invalidate caches and
applicable bytes according to reviewed policy; tombstone/history retention cannot
be cited as permission to retain prohibited personal data forever.

Future portable Library export/import is **not sync reset**. Export omits queues,
receipts/cursors/epochs/leases, recovery state, Accounts and device bindings.
Import is dry-run then isolated/transactional; revisions are source provenance,
not fresh server preconditions. Restored provider provenance requires fresh access
before refresh. Cloud association/upload needs current authorization and explicit
reconciliation, never blind replay or a second writer cloned from stale state.
No export endpoint/encryption/runtime work is required for initial implementation.

## S6 fixture and acceptance specification

The next artifact should turn these cases into named, deterministic fixtures
with input bytes, expected digest/typed state, expected error and crash boundary.
No live account, real database or cloud deployment is required to specify them.

1. Canonical bytes: ASCII/non-ASCII keys, escaping, duplicate keys, number bounds,
   negative zero, decimal counters, timestamp precision and optional extensions.
   Rust/macOS encoders must match frozen byte vectors; reject misleading JSONB/
   JSON.stringify equivalence.
2. Local-first command: commit with network unavailable, restart, preserve exact
   local state/intent; stale local revision rolls back roots/children/outbox.
3. Offline chain: A accepted, B remains an overlay; receipt first and feed first
   produce the same final state. B seals only after ordered A application.
4. Idempotency: same request retried after lost commit response, different body/
   actor conflict, receipt missing while original request is still queued, epoch
   mismatch and unknown historical outcome.
5. Multi-root merge: complete expected set, concurrent new dependent rejection,
   full rollback, whole-batch bounds and no automatic split.
6. Kind transfer: Beach/beach/beaches in either creation order, canonical/alias
   collision, source retirement snapshot, all claims moved, canonical promotion,
   chain A->B->C, tombstone reservation, wrong target/policy/Library rejection.
7. Offline Kind collision: failed create A vs accepted B, explicit reconcile
   creates merged historical A without a second active owner, dependent intents
   rebase and package snapshots preserve original IDs.
8. Feed identity: exactly one immutable batch per page, 204 no insertion/cursor
   change, stable bytes on replay, token-key retirement, sequence uniqueness,
   out-of-order/duplicate and same-revision/different-digest rejection.
9. Conflict quarantine: no partial root application/cursor advancement, own echo
   idempotence, sealed unknown work protected, explicit remote/local resolution
   and immutable supersession history.
10. Snapshot: records/high-water from one snapshot, bounds rejection, truncated
    payload, unsupported schema, stage/restart/install, local edit during planning,
    pending overlays, missing-known-identity conflict, device/recovery preservation.
11. Revocation: Account/identity/member changes during requests, editor->viewer,
    sign-out/re-authentication, different Account/environment, short-lived media
    capability residual access and no claim of remote offline-byte erasure.
12. Media: staging URL cannot overwrite finalized object; size/hash mismatch,
    expired upload, completion retry, unknown outcome, authorized descriptor-only
    sync and missing local bytes.
13. Catalog: minimal vs opted-in rich projection, cross-Library redaction,
    structured path/bookmark rejection, duplicated package IDs, reported-vs-
    verified provenance and no Graph/run/package mutation.
14. Version evolution: unknown required features stop atomically, optional
    extensions round-trip, old writers refuse lossy saves, policy upgrade collision
    report and old cursor/new epoch reset.
15. Cross-backend conformance: same IDs, semantic normalization, lifecycle,
    relationships, term transfers, retirement terms and mutation results under
    SQLite/PostgreSQL; Storexa lifecycle/rollback/redaction without SQL in nodes.

Crash-injection boundaries must include: after local intent commit; after sealing
but before send; after server commit but before response; after receipt persistence;
after inbox receipt but before apply; before/after cursor commit; after staging
snapshot but before install; after media staging upload but before finalization.
Each fixture states what may safely retry and what remains unknown.

## S7 decisions and v1 recommendation

Recommend approving v1 only with:

- Atomic Kind claim transfer/retirement provenance and explicit offline collision
  reconciliation; no arbitrary term stealing, duplicate concepts or resurrection.
- One-batch feeds, separate receipt/base milestones, no automatic conflict merge,
  sealed-request immutability and explicit local supersession.
- Bounded full snapshots with honest oversize failure; no hidden pagination/
  destructive reset, and preservation of pending local intent.
- Initial retention of receipts, tombstones, term provenance and change history.
  Operational size limits and a later erasure/retention protocol remain explicit
  product/security work; no indefinite-storage promise is inferred.
- Explicit cloud catalog sharing and no cross-Library rich snapshot upload in
  v1; no silent Account/environment retargeting or CloudKit dual-write.
- The negotiated limits, normalizer/codec vectors, role/revocation rules and
  media staging/finalization safety validated by S6.

These are recommendations, not recorded user approval. S7 may narrow features or
request another representation, but cannot relax uniqueness, durable intent,
authorization, package authority or truthful unknown-outcome handling.

## S5 acceptance checklist

- [x] Versioned endpoint, envelope, canonical-byte and digest contracts.
- [x] Local/server revisions, atomic commands, offline chains and idempotent receipts.
- [x] Ordered feed/inbox, bounded snapshots, reset and conflict/rebase behavior.
- [x] V1 Kind merge/promotion/collision recommendation without duplicate concepts.
- [x] Media, catalog, device/privacy, failure and version-evolution boundaries.
- [x] Required S3/S4 physical refinements identified and reconciled as proposal text.
- [x] S6 fixture cases and S7 decisions are explicit.
- [ ] S6 byte vectors and runtime physical-schema/conformance fixtures verified.
- [ ] User approves protocol/schema/policy choices at S7 before implementation.

Next: [S6 generation-two fixtures](../ROADMAP_0_2_EXECUTION.md). No service,
database, API implementation, migration deployment or UI restructuring is
authorized by this proposal.
