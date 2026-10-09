# PS0 — Project Session and Graph Durability

Status: architecture contract; PS2 sealed-root retention and registered cooperative
in-place admission directions approved 2026-09-16. Multi-surface contract below is
transport-neutral; production implementation and storage qualification remain gated.
Historical audit baseline: clean
`114ce43af1089c8bac7fa5c3010a440521027571` (HEAD and local origin/main).
No production implementation, migration reservation, package conversion or switch
is authorized by this document. LL0 is accepted/published; LL1 is independent.

Companion: [review contracts and schema deltas](proposals/ps0/CONTRACTS.md),
[acceptance furnace and evidence](proposals/ps0/VERIFICATION.md).

## Scope and audit

The following are implementation findings at the baseline, not descriptions of an
already implemented autosave system. Paths are repository-relative.

| Layer / source | Observed behavior | Reuse / replacement boundary |
| --- | --- | --- |
| `crates/photara-core/src/command.rs` | Immutable GraphCommandEnvelope, identity/revision validation, Batch, one Graph revision increment per accepted command | Reuse pure validation/application; wrap in durable operation protocol |
| `crates/photara-core/src/project_command.rs` | Project operations validate but do not advance durable ProjectRevision | Reuse semantic operations; do not confuse this revision with journal sequence or package revision |
| `crates/photara-bridge/src/production.rs` — ProjectSessionState, apply_core_command, save | Mutex protects one in-process aggregate; applied commands replace memory and set dirty. Undo/redo are memory vectors. save increments ProjectRevision and calls whole-document replace | Replace persistence/session ownership and acknowledgement boundary. Existing applied=true does not mean durable; snapshot construction can fail after memory application |
| `crates/photara-store/src/lib.rs` — FileSystemStateStore | Legacy `<id>.photara-project.json` whole-document CAS under create-new lock file; temporary file sync then rename; no directory sync in replace; stale lock recovery absent | Keep legacy route explicitly isolated. Do not adapt this whole-file writer into package autosave or claim power-loss durability |
| `crates/photara-store/src/package/v1_1/reader.rs`, `reader.rs`, `json.rs` | Canonical strict JSON, descriptor-relative Unix reads, no-follow/hardlink checks, exact hashes, closure inventory, complete parent chain and HEAD recheck; non-Unix safe reads unsupported | Reuse validation and byte codec, broaden platform adapter separately |
| `crates/photara-store/src/package/creation.rs`, `creation/filesystem.rs` | UI1 assembles initial 1.1 package, pins private stage, syncs files/directories, validates and publishes no-replace | Reuse codecs and path-pin pattern. Materialize truncates files only inside reserved private creation stage: never use it on published packages |
| `crates/photara-bridge/src/creation.rs`, UI1 creation journal | Creation identity/reconciliation, not an edit journal | Keep creation operation namespace/state machine separate |
| `platform/macos/photara-app/Sources/AppModelCreation.swift` | Reopen requires matching complete creation-journal path; advances creation, then sets createdProject after closeProject | Created-package route is read-only after creation, not a general editable package session |
| `ApplicationAdapter.swift` | Created-package presentation sets `canAuthorProject: createdProject == nil`; Saved package label describes initial package, not editing autosave | Preserve read-only gate until durability acceptance |
| `AppModel.swift` — commitGraphMutation, save, closeProject, openRecent | Graph mutations call memory bridge; Save is explicit. Close clears session, cancels evaluation without a flush barrier. OpenRecent assigns new context without durability transaction | Replace entry-point orchestration only after PS2. Route all opens/new completion/close through coordinator; no direct assignments bypassing it |
| `PhotaraMacApp.swift` | Cmd-S is Save, enabled only for legacy project; URL opening directly enters creation route; no termination durability gate | Native Save Now and host lifecycle adapter required |
| `ProductionGraphView.swift`, `photara-shell/Sources/EditorSessionModel.swift` | Canvas interaction/presentation, local panel preferences; not persistence authority | Preserve Graph UI. Project-keyed view state via coordinator, not authored Graph bytes |
| `crates/photara-bridge/src/local_state.rs`, Library local store/service | Separate SQLite initialization and cloud/local catalog projection | No new graph-byte authority; no live initialization in PS0 |

The exact gap is **an existing package 1.1 reader/validator plus initial UI1 creation
writer, with no published-package edit writer, durable Graph journal, or unified
Project session coordinator**. Legacy explicit saving does not close this gap.

## Product and authority invariants

1. Graph stays the primary center surface. Projects/People/Locations are one-click
   Library navigation. Library switching remains the less frequent LL0 avatar menu.
2. A Gallery single click selects/previews. Activating the already open Project
   returns to its Graph idempotently, without warning, reload or revision advance.
3. Other-project double click uses native confirmation titled `Switch to “<target>”?`
   with message `Photara will close “<current>” and open “<target>.” Changes to
   “<current>” have been saved automatically.` Buttons: **Cancel / Switch Project**.
   No suppression preference initially. Escape cancels; native default action and
   keyboard focus are explicit and tested.
4. That past-tense message must be true: before presenting it, finish pending input
   and establish a verified save barrier. If saving fails, show Save Failed in the
   current Project instead of this confirmation. Confirmation does not replace the
   mandatory post-confirmation freeze/flush barrier. Block editor mutation while the
   sheet is present; recheck revision/run state when accepting. This freezes the
   switching attachment, not other clients. With concurrent clients, the dialog
   must describe its verified save barrier without implying that later shared
   changes are saved; refresh or withhold the claim when it would be false.
5. One Rust mutation authority orders writes for each registered package incarnation.
   GUI, CLI, headless, scripting, automation and AI agents are legitimate clients of
   that same protocol. Old owner/client callbacks cannot mutate a new attachment.
   No cloud round trip is required to save already authorized local edits.
6. Never label memory-only state Saved. Never silently overwrite a changed HEAD.

| Data | Authority / placement |
| --- | --- |
| Nodes, ports, connections, parameters, authored positions, portable context | `.photara` authored closure |
| Pending authored operations, recovery and unpublished checkpoints | Device-local durable journal; recovery overlay until package publication |
| Active Project/Graph, viewport, selection, panel layout | Device-local session state, keyed by Library/Project/Graph and device |
| Library catalog, Project access and cloud membership | Cloud authoritative for cloud Libraries, locally projected; true local Library authority remains LL0 |
| Evaluation observations, run state and effects | Separate runtime/evidence protocol; never replay an effect as a Graph command |
| Proxies, thumbnails and derived caches | Regenerable local caches |

Neon is **not the authored Graph byte store** in this bounded design. Stable
Project/Library/Graph IDs in package and catalog must agree at open. A path is a
locator, not identity. Cloud outage does not invalidate a verified package save;
access denial is surfaced separately and must not erase pending work. Creating or
transferring catalog ownership is UI1/LL1 work, not a side effect of autosave.

### Shared authority, authorization and concurrent clients

The per-package Rust authority owns admission, exact expected-coordinate checks,
semantic planning, ordered journal append, operation dedupe and package publication.
Calling the same Rust library in two processes does not itself share that authority.
Clients never edit package files directly. Distinguish a package owner epoch from
client attachment identity/generation and GUI activation generation; identify a
package by registered incarnation and pinned identity, not its path spelling.

Host authorization supplies a validated principal, acting application/automation,
granting authority, package/action scope, and current grant validity/revocation
policy. Rust admission enforces that context; caller-provided actor strings or
possession of a path/operation ID are not authority. Persist bounded, credential-free
operation provenance (actor and grant references, effective scope and policy
decision) alongside accepted-operation evidence. Credentials, access tokens,
security-scoped bookmarks, prompts and unrestricted host paths remain outside the
journal/package. Historical provenance explains acceptance; it never grants future
access. Reauthorization and receipt lookup must not disclose another principal's
work or duplicate an already accepted operation after revocation.

Concurrent clients are ordered, not silently merged. If both submit against the
same authored coordinate, one may succeed and the other receives a stale-coordinate
conflict unless it is the same operation retry. Preserve operation IDs and canonical
request identity across retries/restarts; different intent with the same ID refuses.
Resolve authorized duplicate lookup before treating its old expected coordinate as
a new stale mutation. Ordered observations provide reconnect snapshots and explicit
gap recovery; local speculative UI is never an acknowledged shared state.

The lifetime OS lease remains exclusive. A second direct process gets WriterBusy
while the GUI (or any other owner) holds it: safety does not establish liveness.
Routing to the owner or coordinated handoff is deferred; no agent-progress promise
through another process's ownership exists yet. Never steal a live lease by timeout.
A future handoff must resolve journal/intent state, fence the old owner, transfer
authority without overlap and revalidate/recover before new admission. Changing to
transaction-scoped direct leasing would require a separately specified lifecycle,
not just replacing the lock call. Transport/ABI and owner-process choice remain open.

Forward compatibility: voice/chat may later propose and preview entire workflow
changes. Accepted commands use this same Rust authorization, exact-revision,
journal, dedupe, publication and recovery boundary, never direct package edits.
Conversational planning semantics and interaction/approval UX are deferred; this
PS2 contract neither implements an agent planner nor grants it blanket authority.

## Brand independence and BR0 rename readiness invariant

Branding, website, domain, marketing name and production trust enrollment are
deferred until the product is concrete. At that point public renaming must be a
bounded, tested configuration cutover rather than a broad refactor. **BR0 — Brand
Cutover / Rename Readiness** is mandatory immediately after PS0 and before PS1;
persistence/session expansion must not begin until BR0 passes. Readiness can be
proved with a synthetic identity before choosing the final public brand. BR0 is
not implemented or accepted by this documentation amendment.

Public/product identity must come from `config/product-identity.json` or its typed
generated configuration: display/short name, executable/product name, bundle ID,
document UTI and display type, default project package extension, Auth0 callback
scheme/audience, Keychain service, user agent, Application Support/cache roots,
service/public URLs and all other user-visible product names. Missing fields or
implicit derivations must become explicit validated generator contracts in BR0.
No package/session/writer code may hard-code `Photara` or `.photara` as public
identity, root or filename policy. The extension is an outer filename convention,
independent of the internal package format. Here `.photara` denotes the current
configured extension or legacy compatibility fixture, not a future writer constant.
The confirmation text above uses the configured display name in place of today's
`Photara`; all other accepted wording and behavior remain unchanged.

After cutover, new package creation, Save As and any filename publication write only
the new configured extension. Keep a reviewed legacy-extension read alias for
`.photara` unless a later explicit clean-break decision removes it. Opening a legacy
alias must not rename it automatically. If it is subsequently made writable, require
an explicit, safely reconciled filename cutover to the configured extension before
publication; read-only legacy open remains available. Extension-only rename never
rewrites package contents, IDs, HEAD, revisions or object bytes. Locator updates
must retain package identity and recovery binding. Internal fixed paths such as
`HEAD.json` and `commits/` are format conventions, not public branding.

Stable internal compatibility identifiers are separate: persisted format IDs,
node/value/schema IDs, migration history, operation IDs and already-deployed
PostgreSQL schema names must not be mass-renamed. Existing `photara.*` protocol
identifiers and persisted preference aliases are not automatically public leaks.
Changing any internal namespace requires a separate migration/alias contract.
Session/journal identity uses stable UUIDs; paths use configured roots and UUID
components, never display names or branded strings. Synthetic public renaming must
leave durable internal identifiers and package bytes compatible.

Security cutover may legitimately provision new production Auth0, Apple signing/
entitlement and service trust coordinates. Reauthentication or a controlled local-
directory migration may be necessary and expected. BR0 must specify trust validation,
Keychain access policy, callback enrollment, old/new directory ownership, interrupted
migration reconciliation and rollback; text replacement cannot substitute for that
coordinated cutover. No silent credential copying, account reassociation or live
provisioning is authorized by PS0. A final production cutover requires its own
review even after synthetic readiness passes.

Known baseline leaks / BR0 inputs (source inspection; not an exhaustive readiness claim):

| Source | Observed dependency to address in BR0 |
| --- | --- |
| `platform/macos/photara-shell/Sources/CreateProjectView.swift:16–19` | `~/Pictures/Photara/Projects` destination and `.photara` package-name suffix |
| `crates/photara-core/src/creation.rs:35` | Filename-length limit assumes `title.len() + ".photara".len()`; validate configured extension byte budget |
| `crates/photara-store/src/package/creation/filesystem.rs:145,189` | Published `.photara` filename and branded creation-stage prefix; preserve reconciliation for already recorded stages |
| `platform/macos/photara-app/Sources/AppModelLibrary.swift:61` | Application Support fallback appends `Photara` despite configured primary route |
| `platform/macos/photara-app/Sources/AppModel.swift:71–72,236` | Branded initial destination and hard-coded recent-package extension routing |
| `platform/macos/photara-app/Sources/ApplicationAdapter.swift:6` and `platform/macos/photara-shell/Sources/ApplicationShellPreset.swift:230` | `Photara Project` and `Photara` UI title defaults |
| `platform/macos/photara-app/Resources/Info.plist` and `platform/macos/photara-ui-foundation/Sources/ReleaseConfiguration.swift` | Static product/executable strings and incomplete explicit identity fields require auditing generated output, not just source templates |

The current descriptor already carries many public coordinates; that is a reusable
foundation, not proof of rename readiness. Keep the current implementation untouched
in PS0. BR0 acceptance is specified in the companion verification document.

## Autosave and revision model

Keep distinct owner epoch, attachment/activation generation, journal sequence,
each Graph revision, authored revision and package revision/HEAD digest. Graph commands increment
the affected Graph revision; an accepted authored transaction increments authored
revision once; a package checkpoint increments package revision once and may include
many journal transactions. Revisions are checked unsigned integers with overflow
refusal, never wall-clock timestamps. Existing legacy ProjectRevision is not
silently reinterpreted. Unchanged flush is a no-op.

Structural edits immediately enter a serialized validate → append → durability
barrier → acknowledge pipeline. Only acknowledged journal records may become the
committed UI model. Speculative previews are visibly pending. Journal failure
rejects the edit or retains a clearly unsaved draft and freezes further commits.

Movement previews coalesce in memory; commit at gesture end. During long gestures,
checkpoint latest absolute positions at most every 1 second under healthy storage,
one undo group per gesture. If a checkpoint cannot complete, freeze gesture commits
and show failure; elapsed time is not a durability guarantee. Text/parameters use
300 ms trailing debounce, a 1-second maximum pending interval, and immediate focus-
loss flush. Invalid text remains a draft, blocks close/switch until corrected or
explicitly discarded; it is not silently serialized as a valid parameter.

Package checkpoints run after 1 second idle or 5 seconds of sustained accepted
edits, with one in flight and bounded backpressure. These are proposed PS3 tuning
values, not guarantees about a failing disk. Close, switch, sleep, termination and
Cmd-S **Save Now** drain gestures/debounce, sync journal, checkpoint package, then
verify exact revision and digest. Newer accepted work means an earlier save receipt
cannot set Saved. Journal-durable/package-pending remains **Saving…**; **Saved**
requires current authored state and the verified durable package receipt to match.
**Save Failed** includes retry/recovery action and the last known durable coordinate.

A client flush captures a finite accepted journal sequence after submitting its
pending input. The returned receipt covers that target (or a later included prefix),
not an unbounded wait for every client's future work. Later mutations may leave the
shared status Saving while the caller's barrier succeeds. Close/switch drains and
freezes the relevant attachment, not unrelated clients; no receipt promises success
or latency on failed storage. Bounded queues/backpressure apply to all surfaces.

Undo/redo submit new durable semantic transactions with new operation IDs and
preconditions; never rewind HEAD. Persist undo group boundaries and before/after
patches. A recovery checkpoint in a gesture is not another user undo step. Redo
branch invalidation is journaled with the edit that invalidates it. Unknown node
schemas or command versions prohibit editing/replay, but preserve bytes.
Bind groups to their originating attachment/principal and exact target transaction;
never implicitly undo another client's edit. Cross-client undo policy/UI is deferred.

## Package publication protocol

This is append-oriented content-addressed publication, not a whole-package rewrite.
Reuse `photara.canonical-json.v1`: UTF-8, compact recursively key-sorted JSON using
Core's pinned codec; arrays keep schema order. No Unicode or float normalization
outside that codec. Strict parser rejects duplicate keys, invalid integers and
noncanonical bytes. SHA-256 hashes the exact canonical bytes; lengths are exact
bytes, hashes lowercase hex, package counters canonical decimal strings. Core
semantic digest and saved-graph envelope digest are named separately. Retain original
opaque optional fields/bytes. Refuse editing a closure whose meaning cannot be
preserved by the writer; never round-trip it through a lossy legacy aggregate.

1. Open and pin package root and parent by handles and volume/file identity; validate
   manifest, current HEAD and closure, IDs, feature floor and storage capability.
   Pin manifest digest and exact HEAD bytes/hash as CAS token. Acquire lifetime
   exclusive OS lock on a stable `.writer-lock` inode; do not unlink the lock file.
   All cooperating writers use that lock and revalidate after acquisition. Ownership
   metadata is diagnostic (device/process/session nonce), not a timed lease allowing
   lock theft. OS process death releases the lock. Cross-device lease expiry alone
   never authorizes a write. Refuse an ambiguous lock or changed lock inode.
2. Prepare immutable bytes and complete candidate closure in memory/bounded staging.
   Allocate write_id and commit_id once; record checkpoint intent durably locally
   before package I/O. Parent points to exact old commit and digest. New inventory
   equals reachable authored/history closure, sorted by existing ObjectRef ordering;
   carry history and unaffected objects forward. Manifest remains unchanged.
3. Descriptor-relative create-exclusive `.ps-write-<write_id>-<nonce>.tmp` beside
   each destination, mode 0600, directories 0700 subject to existing access policy.
   Reject symlinks/reparse points in every component, nonregular files, hardlinks,
   traversal, case aliases, mount/root replacement and unsafe inherited permissions.
   Never chmod an existing package or follow a substituted path. Windows adapter
   requires equivalent handle-relative safety; unsupported platforms refuse writes.
4. Write complete new objects, sync each file with qualified platform durability
   primitive, then publish each immutable name **no-replace**, sync its parent.
   Existing hash-named object is reusable only after exact length/hash verification;
   collision with different bytes is integrity failure. Commit is
   `commits/<commit_id>.json`, also no-replace and synced. No object/commit is mutated.
5. Validate candidate using the same reader rules (a virtual candidate HEAD over
   pinned data), including old ancestry, before publication. Recheck root/manifest,
   lock and exact old HEAD under lock. Atomic rename is not hardware CAS: this is
   compare-under-exclusive-lock among registered cooperating writers. Arbitrary
   noncooperating changes are outside the guarantee, not a filesystem capability
   asserted to be excluded. Detected conflicts refuse; unknown outcomes retain
   evidence and reconcile. Watcher events are
   hints; polling/revalidation and pre/post-write checks are authoritative.
6. Write and sync temporary canonical HEAD in package root; atomically replace only
   HEAD.json on the same volume, sync root, then reopen and validate published HEAD,
   revision, digest and closure. No Saved receipt until these barriers pass. File
   sync alone is insufficient; the adapter must qualify its file/directory barrier
   ordering and explicitly bounded failure model, including macOS full-sync support.
   Successful fsync/F_FULLFSYNC calls or process-exit tests are not an unconditional
   power-loss guarantee.
7. Append and sync local checkpoint receipt before trimming any recovery records.
   If any publication/flush/receipt outcome is unknown, retain intent and journal,
   block further writes and reconcile by exact write_id/commit/digest, not retry with
   fresh IDs. Cleanup only registered temporary inodes belonging to this attempt;
   unknown files are retained and reported, never recursively removed.

Objects published before HEAD can be unreachable but valid. They are tracked by the
intent manifest, not unaccounted orphan data. PS1 performs no garbage collection.
Prior commits stay immutable. Deleting ancestors would break today's full-chain
reader. Defaults are 1,024 commits, 100,000 inventory objects, 128 MiB aggregate JSON,
16 MiB individual JSON, depth 64, 4,096 object members, 100,000 array elements, 1 GiB
per blob and 4 GiB total blobs. Apply existing validator budgets to the entire
candidate before accepting it. Backpressure before exceeding limits; never create
an unreadable HEAD. **Production autosave is gated on a separately reviewed bounded
retention/compaction and reader-compatibility policy**; the 1,024-commit ceiling is
not a usable indefinite autosave policy. The approved
[sealed-root direction](PS2_RETENTION_STORAGE_DECISION.md) preserves current authored
content, explicitly retained history, source snapshots/resource versions, evidence,
opaque extensions and recovery/dedupe while allowing obsolete intermediate autosave
ancestry to compact. Exact reader compatibility, encoding, migration and retirement
proofs remain unimplemented; no format number or production threshold is selected.
Existing 1.1 ancestry cannot be truncated under its current reader contract.

### Storage capability policy

Writable initial scope: qualified local APFS with functioning exclusive lock,
no-replace publication, atomic same-volume HEAD replacement, safe handles and
verified file/directory/full-flush barriers. APFS name alone is insufficient; deny
known provider-managed locations and uncertain storage. Record capability profile
version and volume identity; requalify after remount/move. Disposable qualification
belongs to PS2, never probes inside a user's package.

Approved admission is registered cooperative editing in place at a user-selected
qualified path; no managed-root requirement or implicit copy/move/conversion.
Registration binds identity, access policy and cooperative ownership separately
from storage qualification. See [writer admission](PS2_WRITER_ADMISSION_PROPOSAL.md).
The compiled interface now separates primitive capability policy from opaque
`RegisteredCooperativeLease` admission. The impossible exclusion flag is removed;
every mutating I/O method requires the opaque lease, which has no production
constructor. Exact binding checks alone do not mint authority. Real registration,
authorization, qualified adapter and multi-client ownership remain unimplemented.

SMB/NAS, cloud-sync/File Provider folders and unqualified local filesystems are
**read-only** initially. A successful rename test cannot prove remote power-loss
or distributed-lock guarantees. No silent journal-only editable mode labelled
Saved. Offer a separately authorized copy to a qualified local destination in a
future slice; do not move/copy automatically or create two catalog identities.
Future NAS support requires a provider-specific protocol and failure furnace,
including disconnect and competing clients. External sources/archives and sibling
packages are never writer targets.

## Durable journal and reconciliation

Use a separate per-device local journal store, not the existing live catalog DB or
UI1 creation journal. Proposed format is a framed append-only file per package
incarnation, with a rebuildable device-local index. No DDL is required for this
format. Header: magic `PHPSJ001`, journal UUID, device UUID, Project/Library IDs,
package incarnation UUID, pinned manifest hash, base HEAD/commit/revision and header
SHA-256. Header itself is length-framed canonical JSON. Each record is
`u32-le payload_length | canonical payload | 32-byte SHA256(previous_checksum ||
length_bytes || payload)`. First previous_checksum is header checksum. Maximum
payload 16 MiB; sequence starts at 1, strictly consecutive. Partial writes never
produce a successful append receipt. Sync file and directory on creation/rotation;
use qualified local durability barriers per acknowledged append.

Every payload contains format_version, sequence, record_id, session_generation,
project_id, incarnation_id, kind and body. Mutation body: operation_id, request
hash, expected authored revision and Graph revisions/digests, resulting coordinates,
versioned semantic command, canonical before/after changed-record patches, undo
group_id and boundary, base HEAD coordinate. Patches preserve unknown fields and
permit replay without running node providers; validate semantic command and patch
agreement before initial acknowledgement. No secrets, host paths, evaluation output
or source bytes in command payloads. Host package locator/pin is a separate local
binding, not portable data.

Record kinds and transitions:

| Kind | Durable effect |
| --- | --- |
| Mutation | Validated authored transaction becomes recoverable; duplicate operation ID + same request returns original receipt, different request rejects |
| UndoBoundary | Closes a gesture/group without changing authored revision; a recovered open group is closed at its last durable checkpoint |
| CheckpointIntent | Binds journal through-sequence, resulting digest, expected HEAD, new write/commit IDs, byte manifest and candidate HEAD before publication |
| CheckpointReceipt | Exact verified package commit includes that through-sequence; permits future compaction |
| SessionBarrier | Records frozen revision/sequence and reason; never substitutes for package receipt |
| RecoveryDecision | Records validated reconciliation outcome and preserved fork identity; no implicit merge |

An acknowledged Mutation is committed intent even if the process dies before UI
acknowledgement. Retrying the same ID is idempotent; operation lookup persists
across restarts and compaction. A failed/unknown append must be reconciled before
accepting later operations. Bounded journal: 64 MiB segment, 256 MiB uncheckpointed
budget; rotate before frame would cross boundary. Stop accepting before budget
exhaustion, preserving existing data. Segment headers chain prior final checksum.
Compaction creates a new synced segment containing recovery base, operation receipt
index and retained undo groups, atomically switches a synced local index, and only
then removes known old segments. Preserve old segments through unknown outcomes.
Undo horizon proposal: last 100 groups / 32 MiB; evict only fully package-checkpointed
old groups, clearly disable unavailable undo. Operation ID dedupe for the journal
incarnation is not evicted with undo; if its bounded index fills, freeze for reviewed
rotation/recovery policy rather than forget IDs. These limits gate production tuning.

Recovery runs before enabling mutations:

- HEAD equals base: validate continuous record prefix, replay each mutation once,
  check expected and resulting digests, resume same checkpoint intent.
- HEAD equals intended candidate or verified descendant containing that exact
  commit/write_id: verify ancestry and digest, reconstruct missing receipt; replay
  only suffix not already included. Numeric revision equality alone proves nothing.
- HEAD ahead with an unrelated commit, divergent digest, or no provable inclusion:
  freeze for conflict resolution; retain local branch. Never silently merge.
- HEAD behind a recorded receipt: treat as rollback/replacement, retain journal;
  no automatic HEAD advance. Require explicit recovery selection after validation.
- Incomplete final frame: preserve raw journal and salvage only verified prefix.
  A checksum mismatch even at tail is corruption, not presumed truncation. Interior
  gap, checksum mismatch or unsupported record version quarantines the affected
  suffix and blocks automatic replay past it. Never skip a bad record.
- Missing package/changed root identity: preserve device journal and current model;
  request locate/recovery. Same path/new inode is not the same incarnation. A moved
  package can be rebound only after identity, manifest, HEAD/ancestry and lock checks.

### First journal-storage implementation — approved 2026-10-03

The user approved this exact **checkpoint-record subset** on 2026-10-03 for
implementation and disposable validation, not the whole PS3 mutation/undo journal
or real-library enablement. The earlier 16/64/256 MiB and 100-group/32 MiB values remain
proposals; use explicit disposable registration budgets for record bytes, aggregate
bytes, records and parser work. Refuse before growth exceeds any bound. No rotation,
compaction, expiry, deletion, automatic conversion or incarnation reset is included.

**Exact approved v1 bytes.** A file is `ASCII("PHPSJ001") || LE32(len(H)) || H || C0`
followed by zero or more `LE32(len(Pi)) || Pi || Ci` frames. `H` and `Pi` are exact
Photara canonical UTF-8 JSON with no newline; lengths count bytes, and zero lengths
refuse. Checksums are **32 raw bytes**, not hex text:
`C0 = SHA256(ASCII("PHPSJ001") || LE32(len(H)) || H)` and
`Ci = SHA256(C(i-1) || LE32(len(Pi)) || Pi)`. Validate checked arithmetic and the
registered byte/work bounds before allocation. A checksum covers the entire
canonical envelope, including kind, identities and body. Duplicate JSON keys,
noncanonical encodings and trailing non-frame bytes never become valid records.

The exact header fields are `{format_version:1, stream_id:Id, journal_id:Id,
device_id:Id, project_id:Id, library_id:Id, incarnation_id:Id,
bootstrap_sha256:Digest, base_head:HeadV1, base_package_revision:Decimal}`.
`HeadV1` is the unchanged package HEAD object. IDs are canonical nonnil UUIDs,
digests lowercase SHA-256 hex, and Decimal is the existing canonical unsigned
u64 string. `stream_id` identifies this physical device-local file;
`journal_id` preserves the logical accepted-journal identity. Header identity and
base HEAD must match independently validated package/local binding inputs. No
header field is proof of registration or writer permission.

The exact envelope is `{format_version:1, sequence:Decimal, record_id:Id,
session_generation:Decimal, project_id:Id, incarnation_id:Id, kind:String,
body:Object}`. Physical `sequence` starts at `"1"` and increments without gaps;
record IDs are unique. Project/incarnation equal the header; generation is a
positive owner-session generation supplied by the registrar. Unknown version,
kind or field at either layer refuses automatic append/replay. The first decoder
supports **only** these exact bodies:

| Kind | Exact body and required validation |
| --- | --- |
| `CheckpointIntent` | `{original:O, semantic_intent:IntentV1, operation_receipt:ReceiptV1, accepted_frame:AcceptedFrameV1}`. These are unchanged existing typed objects: original admission, canonical semantic intent, portable operation receipt and accepted-journal frame. Dispatch O by its supported original codec; verify its exact request commitments, old package/registration, original reserve, planner inputs and replayed Core result through the shared repeatable verifier. Every receipt/frame operation ID, request digest, journal ID/sequence and receipt digest must agree. Persist this exact record before the attempt's first package effect. Unsupported original codecs refuse; opaque JSON is not an execution plan. |
| `CheckpointReceipt` | `{intent_record_id:Id, intent_record_checksum:Digest, original_sha256:Digest, operation_receipt:ReceiptV1, selected_head:HeadV1, selected_commit:CommitV1}`. The referenced earlier intent's record ID and raw checksum (hex here) must match; hash its exact O bytes and preserve its exact original receipt. HEAD/commit must be the original plan's verified completed selector with matching digest, revision, authored coordinate and journal inclusion. Append only after shared full closure verification and the required package barriers; establish this journal record's own qualified barriers before returning completion. |

The frozen `AcceptedFrameV1` remains a **nested unchanged object**, with its
original logical sequence and journal-prefix meaning. It is not the outer storage
frame, and its sequence must never be overwritten by the outer consecutive
checkpoint-record sequence. Neither record kind newly acknowledges a mutation.
`Mutation`, `UndoBoundary`, `SessionBarrier` and `RecoveryDecision` are unsupported
in this first subset; their full typed bodies and logical/physical sequence mapping
must be specified before enabling PS3 acknowledgements. This proposal does not
claim generic patch replay, undo support, or a complete Accepted journal.

Use a separately pinned device-local journal namespace under the configured local
storage root, keyed by device/package incarnation and stream ID. Its path, native
pins, owner epoch, qualification reports and active-stream selection belong to
local registration, not portable authored objects or a caller boolean. One held
cooperative authority serializes appends. Do not create a replacement stream when
one is missing or corrupt, or discover authority from an occupied filename.
Account this namespace, its receipt bytes and coexistence independently from the
package; qualify both domains even if they share a volume.

On uncertain append, preserve bytes and resolve the exact original record before
any later append. A complete matching record is re-barriered before completion;
matching bytes alone do not prove durability. An incomplete tail preserves the
verified prefix and blocks further appends in this subset (no implicit truncate).
A checksum mismatch is corruption, including at the tail. Package retry restores
persisted original planner inputs, selectors, intent and receipt through
[`restore_plan`](../../crates/photara-store/src/package/v1_3/repeatable/restart.rs),
not current fixture/default inputs. Exact retry returns the original matching
receipt; changed bytes under the same record/operation identity refuse. Stored
receipt JSON alone cannot mint `Saved`: live qualification, original-ID inclusion
and current accepted-coordinate checks remain mandatory.

The approval is limited to implementing these storage bytes/two typed kinds
plus the separate [conditional native-profile direction](PS2_MACOS_STORAGE_QUALIFICATION.md#first-native-profile-and-admission-direction--approved-2026-10-03).
It does not approve the remaining PS3 record kinds, production size/undo limits,
retention changes, migrations, provider-unknown admission or real-library writes.


### PS3 record-kind amendment — approved 2026-10-05

The user approved this exact bounded amendment on 2026-10-05 for implementation
and disposable validation, separately from the checkpoint-subset approval. Keep `PHPSJ001`, its exact header,
envelope/checksums, existing checkpoint bodies, intent/receipt encodings and all
old golden bytes unchanged. Older decoders refuse these unsupported kinds. The
new decoder must still refuse unknown kinds/versions/fields. This amendment adds
no retention expiry, production size/undo limits, rotation, deletion, automatic
conversion, real-library admission or cross-client undo authority. The approved
first disposable PS3 scope is **single-operation edits and durable single-operation
undo**. Multi-transaction gesture grouping and its periodic gesture checkpoints
remain deferred; do not expose those paths as implemented PS0 behavior.

Common exact types: `Link={record_id:Id,record_checksum:Digest}` identifies an
**earlier verified frame in this stream**, including its raw checksum encoded as
hex. `Owner={attachment_id:Id,attachment_generation:Decimal,principal:Principal}`
uses the existing checked Principal encoding; it records historical attachment
ownership, never grants authority. `Coordinate` is the unchanged full PS1
AuthoredCoordinate. `RecordBytes={reference:JRef,value:Object}` carries an exact
canonical JSON object whose hash/length match its JRef. Arrays of these records
sort by `(sha256,numeric byte_length)` and reject duplicate references. `Patch={before:JRef,
after:JRef,removed:[RecordBytes],added:[RecordBytes]}` names the before/after AuthoredProject
roots and carries the exact set differences of their schema-defined authored
closures, including changed graph/context records; unchanged dependencies resolve
from the verified base/prefix. No arbitrary JSON-pointer patch language is added.
`OperationTarget={kind:"operation",operation_id:Id,mutation:Link}` names one exact
earlier Mutation; its operation ID must equal that record's frozen intent/receipt.
This is local undo evidence, not a portable receipt group or a new intent variant.

| Kind | Exact approved body |
| --- | --- |
| `Mutation` | `{semantic_intent:IntentV1,operation_receipt:ReceiptV1,accepted_frame:AcceptedFrameV1,owner:Owner,base_head:HeadV1,result:Coordinate,patch:Patch,action:Action,redo_invalidated:[Id]}`. `Action` is exactly `{kind:"edit"}` or `{kind:"undo",target:OperationTarget}` or `{kind:"redo",target:OperationTarget,undo:OperationTarget}`. The redo target is the original edit and undo names the exact intervening undo Mutation. `redo_invalidated` is the sorted unique set of this owner's previously redoable original operation IDs invalidated by this transaction. |
| `UndoBoundary` | `{owner:Owner,target:OperationTarget,reason:"single-operation"}`. Confirms the already complete single-operation undo unit without changing authored state or joining operations. A durable single Mutation is undoable without this optional redundant marker; recovery never invents a group or requires appending a missing marker to recover that edit. |
| `SessionBarrier` | `{owner:Owner,reason:Reason,through:Link|null,accepted_frame:AcceptedFrameV1|null,target:Coordinate}` where Reason is `flush`, `save-now`, `close`, `switch`, `sleep`, or `terminate`. `through` identifies the frozen final Mutation, not the SessionBarrier's outer sequence. Both nullable fields are null exactly when this stream has no Mutation; then target equals the inherited completed-checkpoint coordinate defined below, including its accepted prefix. |
| `RecoveryDecision` | `{owner:Owner,verified_through:Link|null,observed:{head:HeadV1,commit:CommitV1}|null,outcome:Outcome}`. Outcome is exactly `{kind:"prefix-replay",through_mutation:Link|null,result:Coordinate}`, `{kind:"checkpoint-reconciled",intent:Link,receipt:Link}`, or `{kind:"conflict-preserved",preserved_branch_id:Id,reason:Reason}` with Reason `unrelated-head`, `rollback`, `missing-package`, or `identity-changed`. The branch ID names preserved local recovery evidence, not a new package, stream or incarnation. |

**Mutation validation and acknowledgement.** Verify current authorization and
owner attachment separately. Replay the exact supported typed command against the
authenticated preceding authored state through Core/PS1; require its complete
result coordinate and canonical closure differences to equal `result`/`patch`.
Validate both patch sides, all schema-defined dependencies, unknown optional-byte
preservation, and receipt/intent/frame/request identity agreement. Do not trust a
patch because its hashes match, execute providers, or admit currently unsupported
commands. The base HEAD is the verified package checkpoint backing this prefix;
intent.expected is the immediately preceding accepted authored coordinate, which
may be newer than that HEAD. No-op edits retain equal before/after roots and empty
patch arrays while preserving normal operation dedupe.

The accepted frame's logical sequence advances once per Mutation, never for the
other three kinds or a repeated CheckpointIntent containing that same frame.
Checkpoint inclusion must resolve the actual continuous accepted prefix; matching
numeric sequence alone is insufficient. Generate every operation/command,
record and recovery-branch ID and supplied timestamp once before its first append;
retain exact bytes across unknown outcomes. Only a verified, qualified-barriered
Mutation yields Accepted. Duplicate operations return the original receipt and
ownership/action metadata; a changed request or an attempt to replace its stored
ownership/action metadata refuses without changing that original operation. Stored provenance never authorizes a retry by itself.

**Undo and recovery rules.** Preserve the frozen supported portable grammar:
`IntentV1.boundary` is exactly `single`, intent and ReceiptV1 `undo_group_id` are
exactly null. Refuse begin/continue/end and nonnull groups; no existing portable
validator is widened. New durable undo units are keyed by original operation ID
plus the exact Mutation link above. Historical null-group receipts without this
Mutation's owner/patch evidence do not acquire invented undo membership.
Before undo/redo, validate the target operation and owner, current command
preconditions, and the affected values against the target's recorded result.
Derive an inverse/forward command from preserved before/after values and run it
through Core as a **new** single/null transaction with new operation/command IDs
and newly computed revisions/digests. Never replay an old revision or apply inverse
bytes directly. Support only inverses expressible in the reviewed command subset;
unknown inverses, changed preconditions or another owner's target refuse without
effects. Recovered ownership requires separately validated attachment continuation;
never rewrite the recorded owner. Redo invalidation must equal the derived
owner-local undo state and be durable in the same Mutation. Preserve retained
patch dependencies and operation dedupe; no eviction rule is introduced.

For a prefix with no Mutation, the inherited base is the latest earlier
CheckpointReceipt whose original intent, selected package closure, exact operation
inclusion and required durability evidence independently verify; use the verified
header base only if no such completed checkpoint exists. A checkpoint-only stream
may therefore advance this base without inventing a Mutation. A SessionBarrier
with null `through`/`accepted_frame`, or prefix-replay RecoveryDecision with null
`through_mutation`, must use that inherited coordinate and its exact accepted
prefix, not reset to the header. An unverified receipt cannot advance it.

SessionBarrier captures a finite accepted target and is not Saved evidence or a
package receipt. RecoveryDecision records an independently validated outcome: exact
HEAD/commit and inclusion proofs still govern replay/receipt reconstruction.
`verified_through` excludes the decision itself; no decision can bless a corrupt
suffix, authorize rebinding/merge, discard a branch or replace missing original
planner inputs. If a damaged tail blocks appending, preserve it and report recovery
without manufacturing a decision in another stream. Explicit recovery selection
and conflict-resolution policy remain deferred, as in PS0.

Implementation review must include Core/patch disagreement, changed unknown optional
bytes, duplicate operation with altered local action, wrong-owner undo, stale inverse,
single-operation undo after restart, redo invalidation, logical/outer sequence
separation, forged recovery inclusion and barrier coverage by an older checkpoint.
The only added dispatch is these four local journal kinds and their exact bodies;
old readers continue to refuse them, and portable single/null intent/receipt
meaning and golden bytes remain unchanged. Gesture grouping requires a later
explicit additive codec/dispatch proposal; it is not hidden in this amendment.

### PS3 disposable implementation and next review — 2026-10-05

The approved four-kind amendment is implemented. Core derives and validates
patches; restart preserves original receipts, owner-scoped single-operation
undo/redo and finite checkpoint coverage. Unsupported inverses refuse.
Conflict-preserved records decode structurally but cannot be appended as semantic
evidence without independent host proof; the host freezes and preserves instead.
The old checkpoint vector remains unchanged. New six-record vector:
`75b9ccd52e7adcc4599c1abdd9879fae46e094ce4a71ed404ee4d490285443ef`.

[Machine evidence](verification/ps3-disposable-session.json) records 32 passing
Rust tests (the native opt-in run separately), strict Clippy, Swift 6 build and
projection/short-pipe checks, and all 50 unchanged frozen hashes. The private
native sequence passed two edits, undo/redo, exact original retry, changed-request
refusal, process interruption after Accepted, restart/checkpoint and clean remount.
Actual native UI then saved a move from x=61 to x=77, kept Close pending through
the acknowledgement/checkpoint, and reopened **Saved revision 10, x=77, y=14**.
A second normal close exited 0. Both test images and logs are retained/detached.
Clean remount and child-process interruption are not power-loss proof.

The lab is a separate ad-hoc app with a private cfg(test) Rust child. It introduces
no real-library registrar or production write route. Reopen the retained test on
this Mac from the repository root with:

```bash
python3 platform/macos/photara-graph-lab/run-disposable-autosave.py \
  --controller /private/tmp/photara-repeatable-controller.py \
  --controller-sha256 c7e2433d7a3f4dcfc8f84e5c14fcdff3ddb2b334fe681d32ea5aad837118cdda \
  --resume /private/tmp/photara-ps2-remount-cuc4usia
```

**Approved 2026-10-07: PS3 production session wiring, restricted to disposable
admission.** The user approved the following exact scope. Implement the shared app and CLI/headless session integration,
authority/lease checks, truthful native statuses, final flush and failure/recovery
lifecycle using this reviewed contract. Exercise only explicitly controlled
disposable projects; keep real-library/provider-managed/unknown storage read-only.
Prepare the integrated disposable hands-on build and acceptance evidence before
advancing to PS4. This authorizes no new record meanings, grouped gestures,
retention policy, product limits, real-library writes, migration, conversion,
deletion, deployment or Library/project switching. Any additional format or
security decision remains a separate boundary. PS4 and LL2a retain their order.
This is the existing PS0 requirement below: “synthetic multi-client/session lab
first, then separately reviewed production session wiring.” The approved bounded integration is now verified as recorded below; general
real-library enablement and broader authoring coverage remain outside it.
Do not ask again for this bounded integration approval.

### PS3 controlled app/CLI integration — verified 2026-10-07

The default-off `controlled-disposable` feature exposes the existing Rust authority
through typed UniFFI requests and `photara-controlled-session`. It opens only an
already registered controller-owned image; it cannot provision or admit a general
path. The same lease, journal, original retry and checkpoint checks serve both
clients. A second writer refuses; post-open mutation/lifecycle requests require
the current owner epoch and attachment generation. CLI EOF before explicit verified
Close returns failure; dropping a handle never claims Saved.

The isolated `Photara Disposable.app` uses the actual app entry and existing Graph
canvas, branching before normal AppModel/Library initialization. It has a separate
bundle ID, no normal document/URL handlers, and development/ad-hoc signing only.
Unsupported authoring remains unavailable. Swift projects Rust evidence and retains
the pending edit; the App-level termination delegate drains finite work before
quitting and cancels quit on failure. Sleep flush and wake revalidation share that
coordinator. The original standalone lab remains available.

[Evidence](verification/ps3-integrated-session.json): malformed/general-path
refusals, controlled strict Clippy, default-feature-off compile, Swift lifecycle
and FFI checks pass. Real AppKit termination tests cover pending edits and failed
flush/retry. Native CLI testing passes exclusive writer, stale/omitted binding
refusal, edit/checkpoint/original retry and verified close. Actual Graph drag then
Quit while pending recovered **Saved revision 6, x=40,029, y=20,014** in both CLI
and the integrated app. The initial native test exposed and fixed a Scene-level
termination adaptor that allowed an unacknowledged edit to be abandoned; its
regression exercises the actual App entry/delegate. Generated-binding/library
checksums are now exercised during the controlled build.

The qualified scope is still the explicit disposable profile. An older image
refused changed device pins after remount; all 40 files remained unchanged. No
registration was rebound. A fresh instance of the same prepaid fixture supplied
integration acceptance and is detached/retained at
`/private/tmp/photara-ps2-remount-o812boh1`. Frozen fixture/output hashes remain
unchanged; of the 50 prior packet entries, only the store Cargo manifest changed
(to add the default-off feature). Existing PS2 evidence is reused, not reclassified
as production qualification. No real-library writes, migration, deployment or
project/Library switching occurred.

Build the integrated artifact with
`platform/macos/photara-app/build-controlled-disposable.sh`. The existing
`run-disposable-autosave.py` launcher accepts `--integrated-app`, with the same
explicit controller digest and fresh registration/resume arguments above. Resume
never rewrites registration: changed pins refuse and preserve the image.

### PS4 local activation mapping — approved 2026-10-08

The user approved the exact format reviewed at `a776a53` on 2026-10-08,
with the confirmation-order reconciliation below. Implement preparation, confirmation, rollback and activation
between explicitly registered disposable Projects in the same Library, through the
shared Rust coordinator. It approves neither Library switching nor real-path
admission, migration, deployment, new package/journal meanings or capsule disposal.
The [PS0 inventory](proposals/ps0/CONTRACTS.md#unnumbered-local-schema-deltas-review-only)
and [switch cases](proposals/ps0/VERIFICATION.md#switch-and-native-acceptance) remain
the behavioral contract. This is a concrete first mapping, not PS4 acceptance.

**Confirmation-order reconciliation — approved behavior preserved.** PS0's
past-tense “Changes have been saved automatically” remains unchanged. Preparation
finishes pending input and establishes a verified save barrier before showing the
sheet; failure shows Save Failed on the current Project instead. Block GUI mutation
while the sheet is present. Confirmation still requires the separate post-confirmation
freeze/flush barrier and revision/run/attachment recheck. Cancellation releases no
source ownership and does not undo already accepted autosave work. An older saved
receipt alone cannot justify the message. This corrects the proposal's contrary
pre-dialog prose without changing any approved record fields or interaction.
Do not ask again for PS4 format/scope approval. PS4 acceptance remains pending the
real native switching and failure-recovery gates; LL2a is separate.

**Publication unit.** Choose one atomic canonical snapshot per registered
`(device_id, workspace_slot_id)`, rather than SQLite: this bounded slice needs one
slot and its receipt published together, and can reuse the qualified file/barrier/
rename adapter without assuming that package qualification covers SQLite/WAL.
The registrar separately pins a controller-owned local activation directory and
stable cooperative lock; no filename or stored record grants access. Its explicit
test budget covers old snapshot, candidate, retained records, directory and
coexistence before effects. Capacity refusal retains current; no expiry,
compaction, deletion or production count/size limit is introduced.

The exact approved file is canonical UTF-8 JSON, no trailing newline:
`{format:"photara.local.activation-snapshot",version:1,body:B,body_sha256:Digest}`.
Use the existing canonical JSON encoder; the digest is SHA-256 of canonical `B`,
not of its enclosing file. `B` is exactly
`{device_id:Id,workspace_slot_id:Id,revision:Decimal,request_generation:Decimal,
committed_generation:Decimal,authority_scope_sha256:Digest,
active:Ref|null,pending:Ref|null,records:[Record]}`.
IDs/digests/decimal integers use the approved journal spellings. A `Record` is
exactly `{id:Id,kind:String,version:1,body:Object}`; a `Ref` is
`{id:Id,sha256:Digest}` over that complete canonical record. Records sort by ID,
are unique, remain immutable, and all references resolve in this snapshot.
Unknown snapshot/authority-record format, version, kind or fields, contradictory
IDs, broken references or snapshot checksum refuse recovery; they are not preference fallback. All IDs
are generated once and persisted before their first externally visible effect.

The aliases below refer to storage/coordinator values, **not** the reduced native
presentation DTOs. `SessionBinding` is exactly
`{project_id:Id,incarnation_id:Id,owner_epoch:Id,owner:Owner}` from
[`session::SessionBinding`](../../crates/photara-store/src/package/v1_3/repeatable/session.rs),
with unchanged PS3 `Owner`/Principal spellings. `AcceptedCoordinate` is exactly
`{coordinate:Coordinate,mutation:Link|null,accepted_frame:AcceptedFrameV1|null}`
from that module's `AcceptedCoordinate`; `Coordinate` is the full PS1
AuthoredCoordinate validated by
[`journal_session::coordinate`](../../crates/photara-store/src/package/v1_3/repeatable/journal_session.rs),
including its complete per-Graph digest/revision map. `Link` retains
`record_checksum`, not the presentation alias `checksum`. `CoreGraph` means the
exact canonical `graph` JSON member of the authenticated
[`SavedGraph`](../../crates/photara-store/src/package/types.rs), validated by the
existing [SavedGraph record validator](../../crates/photara-store/src/package/records.rs)
and [`photara_core::GraphDocument`](../../crates/photara-core/src/graph.rs).
Preserve its original fields and optional bytes; do not reconstruct it from the
native position-node list. `CheckpointIntentRecord` and `CheckpointReceiptRecord`
are exactly `{value:JournalEnvelopeV1,record_checksum:Digest}`, copying
[`JournalRecord::value()` and `checksum()`](../../crates/photara-store/src/package/v1_3/repeatable/journal.rs).
`value` is the unchanged complete PHPSJ001 envelope specified above and must have
the corresponding checkpoint kind; it is not only the record body. Resolve and
verify the checksum against the original registered journal before using it as
Saved evidence. `HeadV1`,
`CommitV1` and the nested intent/receipt/frame keep their existing checked package
and PS1 encodings. `ProjectTarget` abbreviates exactly
`{library_id:Id,project_id:Id,incarnation_id:Id,graph_id:Id,registration_sha256:Digest}`.

| Kind | Exact approved body / authority |
| --- | --- |
| `SessionView` | `{device_id,library_id,project_id,graph_id,pan_x:i64,pan_y:i64,zoom_ppm:Decimal,selected_node_ids:[Id],visible_panels:[String]}`. Pan uses existing Graph milliunits; zoom is positive millionths. Node IDs sort uniquely and must resolve; panels use existing editor IDs. Invalid optional view content/version selects deterministic defaults (zero pan, zoom `"1000000"`, empty selection, Graph panel), persisted as a new valid view before activation. It never changes authored bytes or selects a Project. |
| `SavedProof` | `{binding:SessionBinding,accepted:AcceptedCoordinate,checkpoint_intent:CheckpointIntentRecord,checkpoint_receipt:CheckpointReceiptRecord,registration_sha256:Digest}`. Embed the unchanged complete PHPSJ001 records, not only an accepted operation receipt or revision. Shared verification checks original admission, journal coverage, selected HEAD/commit and the proof's recorded covered coordinate. It may be retained as historical checkpoint evidence; it represents current Saved only when that coordinate equals the current accepted target and live qualification/barriers are verified. Stored bytes alone cannot mint Saved or a lease. |
| `RollbackCapsule` | `{source_active:Ref,source_saved:Ref,view:Ref,graph_snapshot:CoreGraph,registration_sha256:Digest}`. Capture the actual verified source Graph and view before detach; Graph bytes are an inert read-only recovery display, never write/reacquisition authority. Reacquisition uses the registrar and rechecks the exact source identity. |
| `ConfirmationEvidence` | `{activation_id:Id,device_id,workspace_slot_id,request_generation,expected_committed_generation,authority_scope_sha256,source_active:Ref,source_binding:SessionBinding,accepted:AcceptedCoordinate,prior_saved:Ref|null,target:ProjectTarget}`. Captures the original source identity/current Accepted coordinate, the existing last-verified SavedProof and target/request shown for confirmation. `prior_saved` binds the verified pre-dialog checkpoint proof. PS0 preparation finishes pending input and establishes current Saved before displaying the past-tense confirmation; an older historical checkpoint or null cannot justify that message. The nullable format is retained, but preparation must refuse to present confirmation without current proof. This preserves PS0's saved-receipt binding without authorizing detach. Its copied fields must exactly match the referring ActivationIntent; it has no reference back to that intent, avoiding a hash cycle. |
| `ActivationIntent` | `{device_id,workspace_slot_id,request_generation,expected_committed_generation,authority_scope_sha256,source_active:Ref|null,target:ProjectTarget,source_attachment:SessionBinding|null,confirmation_basis:Ref|null}`. Record ID is the activation ID. Null source/basis is allowed only for a genuinely empty slot. Confirmation binds this original intent plus its non-authorizing ConfirmationEvidence, including the prior checkpoint receipt and current Accepted coordinate. Target or request identity cannot be replaced once preparation starts; stale source attachment/request authority invalidates confirmation. A dirty current session must reach verified Saved before the approved confirmation is presented; failure retains the source and shows Save Failed instead. |
| `ActivationProgress` | `{intent:Ref,stage:Preparing|Confirmed|Frozen|SourceSaved|TargetReady|Refused,source_saved:Ref|null,capsule:Ref|null,target_open:Ref|null}`. `pending` selects the latest validated stage. Only after confirmation and freeze does SourceSaved require a fresh SavedProof covering the frozen finite accepted prefix, independently rechecking the pre-dialog checkpoint proof and covering any newer accepted work; capsule durability precedes detach. A genuinely empty source skips source save/detach with null proofs. No progress record changes the active pointer. |
| `TargetOpenProof` | `{binding:SessionBinding,registration_sha256:Digest,selected_head:HeadV1,selected_commit:CommitV1,accepted:AcceptedCoordinate,graph_id:Id,view:Ref}`. Resolve through the actual target session after recovery, closure/identity checks and view restoration. This is target-open evidence, not the source flush proof. Refresh live access/lease/attachment evidence before publication; no stored access generation is a reusable grant. |
| `ActiveSession` | `{device_id,workspace_slot_id,committed_generation,authority_scope_sha256,library_id,project_id,incarnation_id,graph_id,activation_id,view:Ref,target_open:Ref}`. Describes the committed selection; restart must reacquire authority. It is not a persistent live-writer grant. |
| `ActivationReceipt` | `{intent:Ref,outcome:Activated|RetainedCurrent|RetainedReadOnlyRecovery,old_committed_generation,new_committed_generation,source_saved:Ref|null,capsule:Ref|null,target_open:Ref|null,active:Ref|null}`. Activated increments committed generation exactly once and binds the exact new ActiveSession. Refused outcomes preserve generation and active identity; retain the source capsule for explicit recovery. |

Under the stable local lock, admission compares authority scope and expected
committed generation, advances request generation once, and refuses an existing
pending preparation. Each snapshot replacement advances `revision` by one and
compares the exact prior snapshot digest. The successful replacement includes the
original activation receipt, new active pointer and cleared pending pointer
**together**. Prepare, dialogs, package IO and restoration occur outside that
publication; freeze/recheck prevents a stale callback from publishing. Same-Project
activation is idempotent. Failure before publication retains/reacquires current or
its read-only capsule; visible title/Graph never changes early.

Use exclusive candidate creation, complete-file barrier, atomic replace and
registered-directory barrier. On an unknown result, inspect only the exact old or
candidate bytes, reestablish barriers and query the original activation ID; never
issue a replacement activation. Before a committed receipt restart selects source;
afterward it selects target or explicit recovery with the source capsule retained.
Keep all original receipts and capsules in this first bounded implementation,
including after successful establishment; their later disposal remains unapproved.
Growth must refuse before effects when the registered snapshot/coexistence budget
cannot fit. This finite snapshot codec makes no lifetime scalability claim.

Minimum acceptance: real local snapshot publication with every before/after barrier
and rename cut; exact retry/CAS/stale scope; canceled/double activation; failed
source flush, target open/restore, local publication and source reacquisition;
view fallback isolated from authority corruption; original source/target evidence
separation; native title/Graph preservation and restart on both sides of commit.
No durable snapshot, native dialog or passing model test alone closes PS4. LL2a's
later SQL selection integration remains a separate reviewed mapping; this amendment
does not assign its migration numbers or make a cross-store atomicity claim.

## Coordinator and native lifecycle

A Rust per-package authority owns ordered mutation admission, journal, package
lease, checkpoint scheduling and flush for every supported client surface. Client
activation/lifecycle is a separate coordinator responsibility, not ownership of all
work by the GUI. Swift MainActor projects state; UniFFI transports native typed
requests/receipts without becoming a second persistence protocol. Other transports
remain undecided. No AppKit types in Rust. Callbacks bind owner epoch and attachment
generation and cannot apply to a replaced attachment. All clients of an incarnation
share ordering; second independent ownership is refused.

GUI attachment switch states: Current → Confirmation → Frozen → Flushing → Verified →
Released → OpeningTarget → RestoringTarget → Current(target). Only the final step
updates visible Project identity and persisted active-session pointer. Keep current
Graph visibly present but frozen during the transaction. Store a rollback capsule
with verified snapshot, package identity/HEAD, view state and reacquisition data
before detaching. Detaching does not discard this capsule or another client's work.
Package ownership release, if needed, requires its separate authority transition.

Save failure transitions back to Current(current), with Save Failed and retained
work. Target validation/open/view restoration failure discards target provisional
session, then reopens/reacquires current at its saved coordinate. If current is now
missing, externally changed or locked, retain its verified Graph on screen in
read-only recovery state with Retry/Locate; never present an empty surface or falsely
claim restored editability. No concurrent current and target editable attachments
for this GUI switch transaction; other authorized clients/projects are not forbidden.
A failed local active-pointer write also rolls activation back. Crash during switch
recovers current until a durable target activation receipt; after receipt it opens
target or presents explicit recovery with current rollback capsule available.

An evaluation owned by the switching attachment blocks switching and asks
**Stop Run and Switch / Cancel**.
Cancellation request is not completion: await terminal run state and outstanding
callbacks/effect bookkeeping. Failure or unknown stop outcome retains current.
Recheck at freeze; this slice adds no background/detached evaluation facility.
Switching the GUI cannot implicitly cancel another client's independently owned
job or revoke its authority. Such job lifecycle/ownership must be specified before
that facility is enabled. Same-project
activation does not stop evaluation. Switch/close/termination requests serialize;
a second target cannot replace an in-flight transaction.

Use native macOS alert/sheet, commands and standard status text. Termination adapter
uses deferred application termination and replies only after barrier; failure
cancels termination and presents Retry, with explicit discard a future separately
reviewed action. Close/window close routes through same coordinator. Sleep notification
requests immediate flush but correctness relies on already synced journal; sudden
sleep/process kill/power loss cannot depend on SwiftUI scene callbacks. Wake checks
storage identity/HEAD before unfreezing. OS-forced termination recovers journal on
next launch. Disk full/permission loss/disconnect freezes admission, preserves
unacknowledged draft visibly, and never downgrades Saved evidence. Retry revalidates
identity/capabilities and resolves unknown outcomes first.

Session state persistence failure is separate from authored save failure: authored
Saved can remain true, but close/switch barrier reports local-state failure when
required restore state cannot be recorded. Accessibility exposes status and error
text, progress without repeated announcements, keyboard cancellation and focus
restoration. No custom chrome, Gallery or Graph changes in PS0. A synthetic lab is
not needed to review this contract; PS3 must validate native presentation before
production wiring is accepted.

## Failure taxonomy and rollout gates

Typed failures: Validation, UnsupportedFormat, UnsupportedStorage, ResourceLimit,
RevisionConflict, ExternalChange, WriterBusy, IdentityChanged, JournalCorrupt,
JournalAppendUnknown, PackagePublishUnknown, DiskFull, PermissionLost, Unavailable,
VerificationFailed, RuntimeBusy, StopUnknown, TargetOpenFailed, RestoreDegraded,
SessionStateFailed. Include phase, retryability and last proven durable coordinates;
redact path/content/credentials. Unknown outcomes require reconciliation, not blind
retry. Failures never authorize deletion of package history or recovery records.

- **PS0 (historical checkpoint):** review architecture, typed proposal, schema deltas,
  furnace and compatibility blockers. Its original stop-before-implementation gate
  is not a prohibition on later authorized isolated PS2 contract/test slices.
- **BR0 — Brand Cutover / Rename Readiness:** mandatory after PS0, before PS1.
  Close known public-identity leaks, validate generated configuration and synthetic
  rename/extension compatibility, and review security/directory cutover contracts.
  Final brand/trust enrollment stays deferred; PS1 is blocked until readiness passes.
- **PS1 — pure writer/contracts (requires BR0 acceptance):** typed commands/receipts, deterministic byte plans,
  virtual candidate validation, capability/IO interfaces; disposable tests only.
  Acceptance: current-reader compatibility, opaque preservation and CAS fixtures.
- **PS2 — disposable persistence/recovery furnace:** implement qualified local
  adapter and journal behind test-only entry points; inject every boundary, prove
  replay/unknown outcomes, resource ceilings and restoration. Review retention/
  compaction compatibility before production gate. No live package migration.
- **PS3 — shared authority and native autosave:** synthetic multi-client/session lab
  first, then separately reviewed production session wiring; all entry points, lifecycle and
  truthful statuses. Preserve legacy route unless explicit conversion approved.
- **PS4 — Project switching integration:** only after PS3 durability acceptance,
  integrate accepted confirmation and failure restoration; Gallery implementation
  remains its own scope. LL1 is parallel Library contract/schema/security review;
  production Library switching/lifecycle is LL2 and does not piggyback on PS4.

Each slice requires its own review/authorization; labels follow the existing PS0
roadmap convention and do not reserve migration ordinals. Existing package 1.0 and
legacy documents stay supported on their existing read routes; conversion is not an
autosave side effect. Existing 1.1 UI1 packages remain read-only until exact closure
editing and reader compatibility pass. No hidden upgrade or catalog reassociation.
COV work, Graph rendering, live SQLite/Neon/Auth0/Keychain, archives, photographs,
services and app installation remain outside this checkpoint.

## PS4 disposable native implementation verified — 2026-10-08

The approved same-Library registered-disposable implementation now passes the
required native switching and failure/recovery gates. Shared Rust owns the frozen
activation snapshot, immutable references, original activation IDs, SavedProof
validation, rollback capsules, generation checks and durable atomic publication.
Authored Graph state remains distinct from device-local activation state.

The actual signed app drains pending input and verifies the current source Saved
before showing PS0's past-tense confirmation, then separately freezes/flushes and
rechecks after confirmation. Cancel preserves the source; target failure restores
it; failed source reacquisition shows its full verified Graph read-only. Explicit
Retry restores editing after fresh admission. Quit is canceled while that recovery
is unresolved. Successful activation exposes the target only after durable commit.

[Evidence](verification/ps4-disposable-switching.json) records 13 native matrix
cases, five additional guards, restart on both sides of activation commit, actual
native confirmation/switch/recovery observations, 36 passing affected Rust tests,
strict Clippy, Swift/AppKit lifecycle checks and real generated FFI validation.
The final UI restored B at Saved5, x=40,013, y=20,014; both private images are
cleanly detached and retained. The evidence distinguishes intermediate test builds
from the final signed artifact rather than claiming one executable ran every case.

PS4's approved disposable acceptance scope is complete. This is not cross-Library
or real-library enablement. Conditional storage assumptions and fixture budgets
remain unchanged; no power-loss proof, deletion, conversion, migration, production
writes or deployment is implied. LL2a is the next separate milestone. Its
[2026-10-09 approved local-first packet](verification/LL2A_FIELD_AUTHORITY_REVIEW.md#2026-10-09-recommended-decision--fresh-local-sql-slots)
resolves the publication, target/scope and create-authority decisions for fresh disposable local Libraries only.


## LL2a approved fresh-local implementation — 2026-10-09

The normal native shell and avatar menu now use the shared durable session for
two explicitly registered disposable local Libraries, one Project each. A fresh
SQLite slot publishes exact v2 selection bytes and derived IDs atomically;
immutable v1/package/journal meanings remain unchanged. Current stored local
controller and grant checks run at the actual authority boundary. Unknown commit
outcomes recover the original receipt after durable read reconciliation.

[Evidence](verification/ll2a-disposable-local-acceptance.json) records the native
matrix, final SQL commit cuts, affected tests, and actual signed development-app
Create/Rename, autosave, pending Quit/relaunch, Library switching and Close
Project/reopen. Human review is next. The registered images remain mounted for
repeatable testing; the pinned launcher is listed in ACTIVE_HANDOFF.md.
This is conditional disposable qualification, not power-loss proof or personal
Library enablement. No migration, deployment, removal, cloud lifecycle, format
conversion, expiry or new production resource limit is authorized.
