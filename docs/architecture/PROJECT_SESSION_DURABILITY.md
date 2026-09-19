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
