# Resource storage, verification and retention — approved semantic boundary

Status: semantic direction approved 2026-09-17. This document consolidates the
storage/resource decisions and the retention clarifications accepted before
positive sealed-root fixtures. It is authoritative for that **target semantic
boundary**, including the short default-slot names below. It does not implement
a codec, schema, resolver, Asset Store publisher, watcher or cleanup algorithm.
Existing frozen contracts and serialized bytes retain their current meaning.

Read with [D19 compatibility](D19_CONTRACT_FREEZE.md),
[typed context](TYPED_CONTEXT_AND_EXPRESSIONS.md),
[storage locations](STORAGE_LOCATIONS_AND_HOST_BINDINGS.md), and the
[sealed-root proposal](PS2_SEALED_ROOT_PROTOCOL_PROPOSAL.md).

## Three storage roles

| Role | Authority and contents |
| --- | --- |
| Source locations | User/external-provider custody: RAW/XMP archives and other existing media referenced by Photara. There may be many locations. Reading a source does not take custody or grant sibling-write permission. |
| Project location | The selected `.photara` package, controlled by the package writer: authored Graphs/decisions, resource records, captured evidence, history and sealed-root metadata. Package journal/checkpoint state follows PS0; device recovery remains outside the package. |
| Managed asset locations | Qualified stores controlled through the resource publisher: retained PSBs, TIFFs, layouts, deliveries and immutable captured byte versions. There may be multiple stores/backings. |

These are roles rather than three mandatory directories. A package can occupy an
internal SSD while sources and assets occupy other volumes. Package identity is
independent of its path, display name and extension. Changing a creation or output
default changes future selection; it does not move a package or retarget an
already published resource. The package should remain relatively lightweight.

Each destination independently qualifies for the requested operation and claimed
durability. The proposed initial package-writer qualification scope is local APFS;
this does not qualify a NAS/provider Asset Store. A configurable location is not
a write guarantee, and production storage qualification remains open.

## One typed location and naming system

Use Library Storage Locations, stable Storage Slots and per-device Host Bindings;
introduce no parallel raw-path configuration or process-environment lookup.

| Initial slot spelling | Role |
| --- | --- |
| `$library.project_store` | Default parent destination considered when creating a new package. |
| `$library.asset_store` | Default destination considered when publishing managed large assets. |

These are approved initial slot names, not already implemented built-ins or new
reserved expression keywords. Each default role selects a stable `StorageSlotId`;
the spelling and display label are editor-facing names. Compilation/capture binds
slot identity, revision and the selected `StorageLocationId`. Renaming a slot
does not change identity, and retargeting a default does not retarget captured
references. New captures follow an explicit refresh/selection boundary. Apply
the existing same-scope name-collision rules, including collisions with variables;
do not infer a default role merely from a user-created slot's spelling.

The source grammar is `$library.<slot>`, such as `$library.raw_archive`. Earlier
`$library.storage.<slot>` examples were conceptual and disagree with the
[implemented grammar](../../crates/photara-core/src/context/expression.rs); they
are corrected rather than introduced as a second syntax. The initial names share
the ordinary typed-slot mechanism used by user-defined source archives.

`$project.root` remains the current package's capability-backed root, independent
of the creation default. `$project.artifacts` remains a publisher request target,
never a node-writable package directory; publication policy can select its backing.
`$asset` retains its explicitly asset-bound metadata meaning. Uppercase `$HOME`,
`$PICTURES` and the other closed HostPlaces remain host capabilities. No symbol
grants more rights than the operation's current authorization. Nodes normally use
typed publisher/resource APIs; expressions, CLI and agents use the same authority.

The former `~/Pictures/Photara/Projects` creation location is a configurable
first-run suggestion, not a hard-coded identity or storage rule. Product naming
changes must not require moving existing packages or changing logical identities.

## Resource, working file, capture and backing

A logical resource has stable identity, semantic purpose, custody, producer
provenance and explicit retention policy. A producing node does not own the bytes:
deleting a Photoshop node or removing an asset from a view does not authorize
deleting a retained master. File extensions do not define resource architecture.

Keep four concepts distinct:

1. **Working binding:** editable file/provider object and its current logical
   location. This can change independently of previously captured bytes.
2. **Working observation:** cheap, possibly stale identity/change/availability
   observations about that binding. It is not an immutable-byte guarantee.
3. **Captured Version:** immutable version identity, exact-byte evidence and
   length, provenance, and a capture receipt describing what was established.
4. **Version backing:** one or more locations holding those exact bytes, with
   publication, verification and retention evidence of their own.

A digest proves something about observed bytes; it is not a backup. Recovering
captured V1 after Photoshop overwrites working V2 requires independently protected
immutable V1 backing. An editable hardlink to the working file cannot provide it.
Capture evidence alone must not advertise future recoverability.

The target adds managed external backing: Photara custody need not imply bytes
inside the package. The existing [ManagedResourceSpec](../../crates/photara-core/src/contracts/resource.rs)
requires an embedded blob `ObjectRef`, and D19's external-resource revision binds
a logical coordinate into that revision. Neither is silently reinterpreted.
Managed external backing and mutable backing records require an additive reviewed
contract/feature boundary. Existing `external-output` semantics disclaim custody;
they cannot be reused to claim managed retention. Exact new type/wire names wait.

## Placement and node authority

Nodes declare semantic output kind, source relationships, mutability, retention
requirement, estimated bounds and required capabilities. The shared Rust publisher
resolves policy and supplies scoped staging. Built-in and third-party nodes use
the same flow; they do not build authoritative absolute paths or write HEAD,
manifests, commits, locks or internal objects.

Placement precedence, highest first:

1. Explicit operation choice.
2. Project kind-specific rule.
3. Project default.
4. Library kind-specific rule.
5. Library default.

Choices are **Asset Store**, **Prefer Beside Source**, **Require Beside Source**,
and **Explicit Qualified Location**. For example a working PSB may sit beside its
RAW while TIFFs and retained immutable PSB captures use an Asset Store. A qualified
versioned store beside the source can also retain captures; centralization is a
default, not a constraint. Human-readable working paths and internal immutable
layout can differ without changing resource identity.

Beside Source requires an identified source anchor and separately authorized
sibling-output capability. With an offline/read-only/unsupported/insufficiently
durable destination, Prefer can use only a preauthorized qualified fallback and
must record the actual selection and reason. Require pauses/refuses. If no such
fallback exists, Prefer also pauses/refuses. Neither silently weakens retention.

Content addressing is an optional internal layout, not visible path policy or
resource identity. Regenerable caches are disposable. Publication staging and
journals needed for recovery/dedupe are not disposable caches.

## Availability and retention are separate facts

Report package structural validity, current availability, last byte verification,
and retention compliance separately. The following are semantic states, not
reserved wire enum spellings:

| Evidence/state | Availability and retention interpretation |
| --- | --- |
| Qualified immutable backing durably published and verified; receipt retained | Retention has established supporting evidence under its declared qualification/failure model. The receipt records what was established at that time; it does not prove perpetual accessibility. |
| Same store temporarily offline/unmounted, denied or unreachable | Resource unavailable/offline or unobservable. Preserve publication evidence; current backing condition is unconfirmed. No retention breach follows merely from failed observation. The package remains valid. |
| File apparently missing, location ambiguous or binding changed | Reconciliation required. First resolve store/object identity and binding; a failed lookup on an uncertain mount is not proof of byte loss. |
| Confirmed deletion/loss or verified corruption of a required backing | An active retention obligation is unsatisfied when the remaining qualified backings cannot meet that obligation, such as its required replica count. One lost copy need not breach a policy met by other valid copies. |
| Publication definitely failed before required backing was established | The requested retention guarantee is not established; if an active obligation already requires it, that obligation is unsatisfied. Never report a successful retained publication. An unknown publication result instead stays pending reconciliation under its original ID. |
| Confirmed evidence that the backing no longer meets an explicit active policy requirement | Report that specific unmet requirement. Mere inability to recheck is uncertainty unless the policy expressly requires a freshness/online condition. No such condition is implied by ordinary retention. |
| No remaining retention/pinning obligation and retirement completed through the future conservative protocol | Legitimately retired bytes, not a broken retention promise. Preserve useful identity/evidence and retirement facts without claiming materializability. |

Retention is evaluated against the explicit obligation, not a universal promise
of always-online storage. A structurally sound package can describe an unavailable
resource or even record an unsatisfied retention obligation without having corrupt
package metadata. Missing a required embedded package object is separately package
damage/incompleteness.

## Capture is not indefinite retention

Captured Version establishes identity/evidence; **Retain indefinitely** is a
separate policy. Current authored use, history promising reconstruction, recovery
roots, explicit retention, pending operations, active leases and other declared
obligations can pin bytes. A historical descriptor or predecessor commitment
alone does not implicitly promise permanent byte availability.

When no authored history, recovery root, explicit policy, pending operation or
other obligation requires bytes, the captured version can become eligible for
conservative retirement. Eligibility is not deletion, and a stale reachability
scan is insufficient. Revalidate/serialize against new pins, unresolved outcomes
and concurrent work before any future retirement. Identity, digest, provenance
and retirement receipts may outlive the backing. Package-object collection and
Asset Store collection are distinct lifecycle operations; neither may delete
user-owned source files through node deletion or filename inference.

This leaves room to bound obsolete retained-media versions as well as obsolete
package ancestry. It does not promise bounded growth of authored or explicitly
retained content. Cleanup algorithms and production collection remain deferred.

## Cheap observation and strong verification

Normal metadata opening, browsing, Graph autosave, sealed-root turnover and
metadata reconciliation do not hash large external media. Working observations
can be unobserved, no-change-observed, possibly-changed or capture-in-progress,
with availability reported independently. File identity, length, change metadata,
provider generations and watchers can reveal possible change cheaply. Matching
size/mtime does not prove equality; missed watcher events create uncertainty.

An 8 GB working PSB save invalidates the prior assertion about working bytes
without forcing an immediate 8 GB read. Exact capture later may require a full
read. Strong verification belongs at a meaningful resource boundary: exact
capture, retained promotion, exact-byte consumption lacking verified immutable
backing, verified transfer/delivery, uncertain relocation or explicit integrity
audit. Reuse remains subject to the backing's qualified immutability and evidence;
ordinary checkpointing must not repeatedly verify external byte content.

Sequentially generated final output may be hashed during staging. Random-access
output may require a final pass. A stream digest does not establish persistence
or verify an independently copied destination. The existing publication reread
requirement stays in force until separately reviewed. Capture must refuse or
reconcile concurrently changing input rather than label mixed bytes as a stable
working version.

The [package schema](PROJECT_PACKAGE_SCHEMA.md) permits deferred large embedded
blob hashing conceptually, but the [current reader](../../crates/photara-store/src/package/reader.rs)
hashes reachable embedded blobs. Future structural opening, embedded-object
presence and full byte-integrity verification need separately reported guarantees.
This approval does not relax the current reader, current hashes or frozen fixtures.

## Publication, relocation and capacity

Package HEAD and an Asset Store backing cannot be published by one atomic rename.
The shared Rust authority persists an operation intent, stages/verifies output,
durably publishes qualified immutable backing, then commits the descriptor and
retention obligation to the package. Keep enough evidence to reconcile every
interruption between authorities. Unknown results use the original operation ID;
no fresh-ID retry, invented failure, premature cleanup or success receipt.

A remount changes device Host Binding. Relocating a captured version can change
its backing record without changing content version when exact-byte continuity
is established. An intentional byte edit creates a new version. Publish/verify
the new backing before considering retirement of the old; history, recovery and
pending work may still pin it. This backing indirection is an additive change to
D19's frozen coordinate-bound external revision, not a rewrite of old descriptors.

Checkpoint reserve accounts for package objects, journal/index, recovery roots,
staging and newly embedded work. Describing hundreds of gigabytes of managed
external media does not reserve those bytes again for every checkpoint. Asset
publication reserves capacity in its target store separately. If both occupy one
physical capacity domain, account for their combined concurrent reservations and
avoid double-counting the same free space. An insufficient reserve causes
backpressure/refusal, not loss of authored/retained content.

## Sealed roots, backup and synchronization

Each root's exact schema-defined package closure retains required embedded objects
plus authoritative resource/version identities, logical locations/backing records,
provenance, verification evidence and retention obligations. External byte backing
is a distinct dependency class; retaining its record does not embed/copy/hash its
media. Offline backing does not prevent validating package metadata. Roots/history
that promise reconstruction pin the corresponding backing obligations explicitly.
Predecessor provenance must not recursively pin every obsolete root or version.

A `.photara` directory copy is a **package-state backup**. A **complete project
backup/export** must also include the managed backings required by the selected
retained roots/obligations across all locations, with verification and recorded
scope. If required bytes are offline, report incomplete/pending rather than claim
a complete backup. Collecting user-owned RAWs is a separate explicit choice.
Copying preserves identity; creating a new independent project still requires
the explicit fork protocol. Cloud catalog synchronization does not imply media
upload, replication, retention compliance or availability on another computer.

## Positive fixture acceptance and deferred work

Next implement a disposable, fixture-only positive sealed-root model. Cover:

- Independent active/recovery roots, complete package keep-set equality, repeated
  turnover, dedupe/inclusion and interrupted publication/conversion boundaries.
- Zero external large-media reads during metadata/root turnover; synthetic 8 GB
  edits marked stale cheaply; same-size/same-mtime uncertainty and missed watchers.
- Retained V1 surviving working V2; capture without an indefinite pin; releasing
  all obligations permits retirement eligibility, while any history/recovery/
  pending/explicit pin blocks it and descriptor-only provenance does not leak pins.
- Previously verified offline backing preserving package validity and retention
  evidence; confirmed loss/corruption breaching only an unmet active obligation;
  ambiguous lookup reconciling; required embedded-object absence reporting damage.
- Qualified Beside Source fallback/refusal, explicit rights, policy precedence,
  slot rename/retarget without historical retargeting and role/slot-name separation.
- Verified relocation preserving content identity; uncertain relocation and
  cross-store publication using original operation IDs without unsafe cleanup.
- Independent package/store reserves and shared-volume accounting; package-only
  versus complete backup scope and offline incomplete-backup claims.

Fixture evidence must not be described as implemented production publication,
GC, provider support or durability qualification. Synthetic media sizes/read
counters can establish the boundary without allocating real 8 GB fixtures.

Approved now: the three roles, typed default slots, resource/working/capture/backing
split, placement precedence, publication and fallback semantics, independent
availability/retention, policy-governed version lifecycle, verification boundaries,
sealed-root dependency classes, capacity/backup scope and fixture expectations.

Deferred: exact storage directories, CAS/chunking, UI, watcher implementation,
NAS/provider write qualification, cleanup algorithms, migration/format/feature
identifiers and production thresholds. Additive production contracts and codecs
require their own reviewed implementation boundary; no existing bytes change
meaning by virtue of this documentation approval.
