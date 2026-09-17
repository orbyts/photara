# PS2 production codec and publication contract — review proposal

Status: **proposed for explicit review, not a production format freeze**.
This is the next boundary after the [approved disposable semantics](PS2_SEALED_ROOT_PROTOCOL_PROPOSAL.md)
and [on-disk experimental evidence](PS2_EXPERIMENTAL_SEALED_CODEC_CHECKPOINT.md).
It selects candidate permanent identifiers and validation rules for review.
Nothing in this document enables live conversion, a production writer, deletion,
an Asset Store, project switching, or a writable storage profile.

## Decision requested

Approve or amend one additive package-reader capability for sealed roots and
one additive resource-backing capability. Keep the current bootstrap and
`HEAD.json` envelopes byte-identical. A selected commit carries the new
required features, minimum reader, and typed root-set record. The existing 1.1
reader refuses the new feature before traversing legacy ancestry; unconverted
packages continue down their existing reader path without a rewrite.

These identifiers are stable internal protocol names, not the future public
brand, app name, or package filename extension:

| Wire role | Candidate permanent value |
| --- | --- |
| HEAD schema | Existing `photara.package.head` version 1; unchanged |
| Outer commit schema | Existing `photara.package.commit` version 1; unchanged |
| Canonical JSON | Existing `photara.canonical-json.v1`; unchanged |
| Sealed-root required feature | `photara.sealed-roots.v1` |
| Required commit member | `root_set` |
| Root-set schema and discriminator | `photara.package.root-set` version 1, `kind: "sealed"` |
| State-root schema | `photara.package.state-root` version 1 |
| Managed-backing required feature | `photara.resource-backings.v1` |
| Selected commit minimum reader | `{ "major": 1, "minor": 2 }` |

Bootstrap format and commit minimum reader are already separate coordinates.
The v1.1 reader checks required commit features before its 1.1 reader-floor and
legacy ancestry walk. A 1.2-capable reader must require the sealed feature,
supported root-set schema/discriminator, and floor **together**. If new
managed-backing records are referenced, it must also require the backing
feature. Missing, inconsistent, unknown or downgraded declarations refuse;
an optional `root_set` field alone never activates these semantics. Retained
bootstrap-required features remain required. No fallback to a different commit,
newest timestamp, or older valid root is permitted after a selected-head error.

## Dispatch and independently valid roots

`HEAD.json` stays the sole atomic dispatch point: exact ProjectId, CommitId and
commit-byte digest select one immutable root-set commit. Its legacy-shaped
authored, history and inventory references must equal the active root's
corresponding references. The new reader validates package identity, exact
bootstrap hash, canonical bytes, commit ID/hash, required features, floor,
root-set schema, selected roots and inventories under explicit budgets.

The root set selects one active and one independently verifiable recovery state
root, plus explicitly pinned roots, durable operation evidence and an optional
separately pinned original-conversion source. Each state root binds Project and
Library identity, bootstrap digest, authored revision, authored/history and
resource-state references, journal inclusion commitment and exact
schema-defined package-object closure. Active/recovery roots validate without
the other root or any predecessor object. The root-set keep-set is the exact
union of selected roots and their closures plus independent operation and
conversion-source objects; dispatch, journal and staging overhead are counted
separately. Opaque optional JSON is preserved but does not create dependency
edges. Unknown required schema or absent package object refuses.

Package revision advances monotonically for every newly selected commit;
compaction alone does not advance authored revision. An outer parent/predecessor
commitment records provenance and publication succession under the new feature,
but does **not** require recursive predecessor availability. The new reader must
not accidentally reuse the legacy genesis/parent-chain rule. Retaining a root
never recursively pins all former recovery roots. A previously accepted
operation cannot disappear from durable dedupe evidence merely because its
authored state left undo/history or a rolling recovery root.
The active root's accepted-operation prefix must extend the recovery root's
accepted prefix exactly. The durable operation index maps an original operation
ID and canonical request digest to its original outcome/receipt; the same ID
with a different digest refuses, while the same ID and digest returns that
recorded result without reapplying the mutation.

## Resource metadata is not media placement

The [approved resource contract](RESOURCE_STORAGE_AND_VERIFICATION_CONTRACT.md)
requires new typed, authoritative records. The existing embedded
`ManagedResourceSpec` cannot silently become an external managed backing, and
`ExternalResourceRevision`/`external-output` cannot acquire Photara custody by
reinterpretation. Candidate v1 record schemas are:

| Schema ID | Role |
| --- | --- |
| `photara.resource.state` | Root-selected resource/version/backing/obligation relationships |
| `photara.resource.identity` | Stable logical identity, purpose and custody |
| `photara.resource.working-binding` | Revisable working coordinate and cheap observation |
| `photara.resource.captured-version` | Immutable exact digest/length, capture and provenance evidence |
| `photara.resource.backing` | Version-specific backing identity/revision, logical StorageLocationId, portable locator and publication evidence |
| `photara.resource.retention-obligation` | Explicit policy/pin and requirements, including replica requirements when present |
| `photara.resource.publication-evidence` | Original operation, verified bytes, selected destination and qualification model |

The captured version does **not** own a permanent pointer to the current
backing or retention policy. A verified relocation can change root-selected
backing/obligation records without changing the captured version; a byte edit
creates a new version. Histories promising reconstruction explicitly pin the
needed version/backing obligation; provenance-only history does not implicitly
pin every prior media version. New authored references to managed external
backing use `photara.project.representation-content` **v3** with a new
`managed-captured` binding; existing v2 `managed` and `external` meanings remain
unchanged. The initial 1.2 reader may validate that v3 form. Existing context
`ResourceValue`, variable, expression and snapshot encodings **must reject** a
captured-managed descriptor until their own additive typed contract and feature
are reviewed and implemented. A side table alone is insufficient to make old
authored or node-value references mean something new.

Package opening validates these records, cross-links and package-resident
closure, not external media bytes. Host paths, credentials, live access grants
and device Host Bindings are not portable authored records. Previously verified
retained media on an offline store is unavailable/unconfirmed, not automatically
lost or a retention breach. Confirmed loss/corruption can leave metadata
structurally valid while an active retention obligation is unsatisfied. Normal
open, browse, autosave and turnover perform zero large external-media reads or
whole-file hashes solely to detect change; exact capture or explicit strong
verification is a separate operation.

## Conversion source and compatibility

Conversion is explicit opt-in, never automatic on open. Before changing HEAD,
create a separately pinned, independently reopenable original-package snapshot
under a reserved internal namespace such as
`conversion-sources/<conversion-id>/package/`. Record exact original HEAD and
bootstrap bytes and an exact portable path/digest/length manifest for **every**
original regular file, including unknown optional bytes. Refuse unsafe entries
and occupied namespace paths; do not follow symlinks or silently exclude
unknown files. Define and test the pre-operation exclusions for the cooperative
writer lock and attempt-owned staging. The original snapshot must pass the
actual legacy reader and publication barriers before HEAD advances.
An unknown or conflicting occupied entry refuses conversion. A partial snapshot
registered to the **same** persisted conversion ID and exact original manifest
is instead resumed or reconciled under those IDs; interruption must not turn a
legitimate original attempt into an unrecoverable collision.

The initial conservative implementation copies package-resident bytes once
and reserves that capacity; it does not copy user-owned source files or large
external managed backings. A future deduplicated conversion representation
would need independent byte-preservation and reopening proof before replacing
this choice. Every interruption retains the complete original representation.
The conversion source is pinned independently of active/recovery turnover and
has **no automatic release** in this first production contract. Exporting the
original or restoring old authored content is a separate action that preserves
newer operation evidence; neither silently rewinds HEAD.

## Publication, reconciliation and capacity

One shared Rust authority owns admission, accepted mutation ordering,
journaling, immutable publication, HEAD replacement and recovery for GUI, CLI,
headless, scripting and agent clients. A client may request an edit; it cannot
mint writer admission or mutate package internals. The exact routing mechanism
when another Photara surface owns the lease remains deferred. A user-selected
package can be edited in place only under a qualified storage profile and
registered cooperative lease bound to exact parent/root/volume/lock and
manifest/HEAD/incarnation coordinates. Network, File Provider, changed mount,
unknown or ambiguous targets refuse production writes. A path under Dropbox,
even if backed by APFS, is not admitted merely by its filesystem label.

1. Persist intent with exact old HEAD/bootstrap/incarnation, original operation,
   request and write/commit IDs, candidate bytes, selected roots, reserve and
   exact finite journal inclusion (journal identity, prefix digest and resulting
   authored state).
2. Publish immutable dependencies, roots, operation/dedupe evidence and commit
   without replacement; complete qualified file and directory barriers.
3. Validate the full candidate independently, then recheck lease, identity,
   pins and exact old HEAD. Only then atomically replace `HEAD.json` on the same
   qualified volume.
4. Complete the qualified HEAD/device/directory barriers, reopen the exact
   selected HEAD and full closure, and durably record acknowledgement and
   recovery/dedupe transition. `Saved` refers only to this verified receipt
   **when its included authored revision and digest equal the current accepted
   session state**. A valid older-prefix receipt cannot label later pending
   edits as saved.
5. Retain all unknown-outcome evidence. Old HEAD means pending; exact candidate
   requires original-ID barrier/inclusion reconciliation; unrelated HEAD or
   rollback after receipt freezes in conflict. Never retry with fresh IDs or
   infer “not performed” from a later failure.

The separate managed Asset Store publication persists intent, stages/verifies
and durably publishes the backing **before** package descriptor/obligation
publication. There is no cross-store atomic rename. Unknown cross-store results
retain original operation identity and backing evidence for reconciliation.

Reserve finite capacity before accepting bounded work: union of active,
recovery, pinned and conversion package closure; journal/dedupe/index/staging;
embedded bytes and safety margin. Reserve Asset Store work separately and
combine reservations when package and store share a physical capacity domain.
No experimental fixture threshold becomes a production default. Exhaustion
causes backpressure/refusal with preserved recovery state; dedupe exhaustion
cannot silently forget IDs or reset an incarnation. This contract identifies
retirement **eligibility only**. No package-object deletion, Asset Store GC,
conversion-source release, or user-source deletion is authorized.

## Required verification before production enablement

- New reader positive/negative tests for exact IDs, floor, bootstrap preservation,
  feature/discriminator disagreement, old-reader refusal, active/recovery closure,
  inventory surplus/omission, resource cross-links, offline backing and zero
  routine external-media reads.
- Conversion tests at every publication cut, exact original inventory (including
  unknown bytes), legacy-reader reopening, insufficient reserve and occupied/
  unsafe namespace refusal. Repeated conversion cannot mint new identities.
- Process and fault-injection furnace for every file/directory barrier, no-replace
  collision, candidate validation, HEAD comparison/replacement, reopen and
  receipt. Unknown outcomes must reconcile original IDs without losing either
  selected old or verified new closure.
- A separately reviewed macOS storage-qualification artifact for supported
  versions/configurations and the registered cooperative-admission policy.
  Syscall success, advisory-lock contention and process-crash tests are not
  power-loss proof. Do not enable production writes or a `Saved` claim before
  the stated profile and failure model are qualified.

After contract approval, complete and review the field-level wire appendix
below, then implement the production reader and synthetic publication/recovery.
Storage-profile qualification and the shared Rust session/autosave coordinator
follow. Real project browsing/switching and Library lifecycle wiring wait for
that durability boundary.

## Field-level wire completion required before codec code

The identifiers and invariant rules above are proposed for product review;
they are **not yet an exact serializable wire specification**. Before changing
the production reader, the implementation patch must include a reviewed typed
schema/closure appendix and golden canonical bytes for:

- RootSet, StateRoot, independent operation index/receipt and their exact
  reference edges, ordering and inventory equality. Define the accepted-prefix
  relation and package acceptance ordinal separately from journal sequence and
  authored revision.
- The operation-intent digest domain and exact canonical semantic fields,
  excluding transient attachment/owner credentials; typed credential-free
  principal, actor, grant and scope provenance. Reuse the PS0 authority model,
  but do not promote its still-proposed arbitrary wire spelling by accident.
- ConversionSource's exact namespace, path inventory and pre-operation
  exclusions for the stable cooperative lock and attempt-owned staging; resume
  exact matching partial snapshots under the persisted conversion ID.
- Managed resource-state/version/backing/obligation/evidence schemas, checked
  qualification and pin types, and `representation-content` v3 binding. Do not
  accept captured-managed values in existing context ResourceValue forms.

This is an engineering specification gate within the approved semantics, not
permission to choose a different retention promise, a weaker Saved definition,
or a production threshold. If it exposes a new product/security tradeoff, stop
for explicit review before code. The experimental `example.fixture.*` encoding
is not that appendix and must never be promoted by string substitution.

## Approval boundary

Review the candidate stable IDs, 1.2 floor and unchanged HEAD/outer commit,
new managed-backing record/reference model, complete original conversion copy,
independent operation evidence and ordered publication/reconciliation rules as
one contract. It authorizes completing the typed wire appendix and synthetic
reader/recovery work after that appendix passes review, not inventing it inside
a live writer. This review does **not** approve production writes, migration,
retirement, automatic conversion, an Asset Store, numeric thresholds or a
GUI/CLI/agent routing model. Changing those promises later requires a separate
decision. Synthetic shared-Rust session work can proceed without qualifying a
live writable profile.
