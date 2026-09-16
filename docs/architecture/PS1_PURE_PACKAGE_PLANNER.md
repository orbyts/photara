# PS1 — Pure Project package planner/contracts

Status: accepted as a bounded planner checkpoint; **no installation or deployment**.
Baseline: published clean BR0 `5777fce04a9c1d88cbf249581efdb09055acb2f8`.
Normative inputs: [PS0](PROJECT_SESSION_DURABILITY.md),
[contracts](proposals/ps0/CONTRACTS.md), [verification](proposals/ps0/VERIFICATION.md).
BR0 is published; its exact reviewed checkpoint and Keychain metadata disclosure
remain unchanged. This slice performs no live package, local catalog, Neon, Keychain,
Auth0, service, photograph or archive access. Other worktrees are untouched.

## Implemented boundary

`package::planning` accepts an immutable, bounded in-memory package, verifies it
with the same 1.1 validator as directory reads, and produces a deterministic
virtual checkpoint. No planner function accepts a filesystem locator, reads a clock,
allocates random IDs, publishes bytes or acknowledges durable storage. Test fixtures
alone materialize candidates into new temporary directories for the existing reader.
The legacy reader and UI1 creation publication route remain separate and unchanged.

The private `VerifiedClosure`, `HeadToken`, `CheckpointPlan`, `PlannedFile` and
`PreparedReceipt` fields prevent callers from forging validated plans/receipts.
The receipt proves preparation only: it has no durable marker, journal sequence or
Saved status. `WriteId` and `IncarnationId` are distinct nonnil UUID types; existing
typed operation/commit IDs are reused. Revisions are checked decimal counters, with
Core Graph revisions converted only at the new contract boundary. Core semantic
Graph digest, exact raw Graph payload digest, and saved-envelope digest are distinct.
The raw payload digest preserves the distinction when a Core decoder ignores an
unknown nested optional field.

Typed version-1 mutations cover Project title/description, Graph name/metadata
revision, and existing Core Graph commands using explicit pure definition/value
registries. `AddNode` (including inside a Batch) is refused because creating its
1.1 context and contract records needs a separate reviewed mapping. Legacy
ProjectDocument asset/extension operations, undo/redo, session generations and
multi-transaction journal batching are not implicitly mapped. PS2/PS3 must extend
these contracts explicitly; they must not serialize a lossy legacy aggregate.
Unknown command versions, missing manifests, node value schemas that disagree with
pinned manifests, unsupported package schemas and lossy Core round trips refuse
editing. Existing reader inspection may still preserve such data read-only.

A changed transaction advances authored revision once. A Core Batch advances Graph
revision once; Graph renaming advances metadata revision separately. A checkpoint
advances package revision once. Unchanged semantic state returns `Unchanged`, with
no commit or revision advance. The caller supplies timestamp, operation, write and
commit identities; identical requests and IDs yield identical canonical bytes.
Same operation ID with altered input has a different request digest. Persistent
operation dedupe is a PS2 requirement, **not a stateless planner guarantee**.

## Closure, preservation and CAS

The planner patches original parsed canonical records rather than reconstructing
them from legacy DTOs. Untouched immutable files, all prior commits, the bootstrap
and historical closures remain exactly byte-identical. Optional fields in authored,
Graph, graph-link, inventory, commit/schema and HEAD records survive. A Core Graph
mapping must reproduce its original canonical payload exactly before it can edit;
otherwise it refuses. Node-owned payload changes still use Core semantic validation
and must retain the pinned manifest schema. No provider/effect is run.

The current commit's inventory is its exact schema-directed reachable
**authored/history closure**, not a flat union of every prior inventory. History may
itself retain old authored/Graph snapshots. All ancestors keep their own inventories
and closures; current 1.1 full-chain validation runs over the complete virtual
candidate before any plan is returned. Required features/minimum reader are retained;
no format upgrade is hidden in planning.

New object names hash their exact canonical bytes. Existing names must match exact
bytes, length and SHA-256; commits and write IDs cannot be rebound. Immutable output
files have a deterministic ordered manifest. Only virtual HEAD is replaced.
The token binds exact original HEAD bytes, their digest, exact manifest bytes/digest,
package revision and local incarnation. Matching numeric revision is insufficient.
Changed optional HEAD bytes, manifest bytes, stale tokens or incarnation refuse.
This is a pure comparison contract; it does not claim hardware CAS or detect an
unreported inode replacement. PS2 must check pinned parent/root/volume/lock identities
and compare under a qualified exclusive lifetime lease.

BR0 naming remains an outer admission boundary: callers supply validated configured
write extension and read aliases. A read alias does not grant write permission.
Until an explicit filename cutover is reconciled, alias opening stays read-only.
Changing admitted public extensions produces identical planned package bytes.
Internal `photara.*` IDs, canonical codec, HEAD/commit/object paths and schema
versions are retained. No configuration or native/bridge entry point is changed.

## Resource and I/O contracts

`MemoryPackage` clamps budgets to current reader ceilings and charges **all supplied
files**, even unreachable retained objects/commits, before verification. This is
conservatively stricter than directory reachability validation. It rejects unsafe
relative names and mismatched hash-named bytes. Object/commit counts, per/aggregate
JSON, per/aggregate blob limits and canonical parser depth/member/array limits apply.
Typed request depth/size admission precedes canonical encoding and recursive Core
Batch application. Candidate validation is all-or-nothing; failure does not mutate
the verified input. Sharing immutable byte buffers avoids copying retained blobs.

The interface-only `PackageIo` specifies opaque pins/leases, storage qualification,
stable lock, recheck, temporary creation/write, full-file flush, immutable no-replace
publication, directory flush, HEAD replacement, verification and attempt-owned
cleanup. Every call distinguishes pre-effect failure from unknown post-effect
outcome. There is **no implementation or caller** of this trait in PS1. A capability
policy accepts only a versioned local APFS profile with every required guarantee;
provider-managed, network and unqualified storage refuse. Flags are future adapter
assertions, not evidence of actual hardware qualification. No storage probing occurs.

The full 1,024-commit ceiling fixture passes; planning 1,025 refuses without deleting
anything. [Retention proposal](PS1_RETENTION_PROPOSAL.md) remains a required review
gate before production autosave. No indefinite-autosave, compaction, GC or power-loss
claim is made.

## Verification and evidence

- Core/store/bridge tests: **200 passed, 0 failed, 3 explicit fixture generators
  ignored**, including all sixteen PS1 cases.
- Sixteen new PS1 fixtures exercise deterministic multi-checkpoint ancestry,
  current directory-reader compatibility, raw/semantic/envelope coordinates,
  history/blob retention, opaque fields, exact HEAD/manifest/incarnation conflicts,
  altered operation digests, immutable collisions, extension neutrality/read-only
  aliases, unchanged plans, schema/version refusal, lossy mapping refusal, unsafe
  paths/hash mismatches, resource exhaustion, overflow, capability refusal and the
  actual 1,024-commit ceiling.
- Strict Clippy on Core/store/library/bridge, all targets: **passed**.
  Formatting/whitespace, nine descriptor tests and BR0 source/config guards: **passed**.
  Exact inventory, empty staging, BR0 HEAD and report-relative links: **passed**.
- Existing tracked golden fixtures, config, migrations, native/bridge source and
  production entry points remain byte-identical to BR0.

Raw logs are local disposable evidence, not committed artifacts:
`/private/tmp/photara-ps1-final-tests.log`,
`/private/tmp/photara-ps1-final-clippy.log`,
`/private/tmp/photara-ps1-envelope-test.log`.
Earlier fixture attempts correctly failed reader integrity because the negative
fixture changed graph/configuration bytes without updating pinned history digests.
The fixture was corrected to reach the intended lossy-mapping refusal. Initial
strict Clippy findings (documentation, boxed command variant, scope/formatting)
were fixed; no test suppression or production workaround was added.

## Exact inventory

15 files; no staged changes, migrations, dependencies or generated artifacts:

1. `ROADMAP.md`
2. `docs/ACTIVE_HANDOFF.md`
3. `docs/architecture/PS1_PURE_PACKAGE_PLANNER.md`
4. `docs/architecture/PS1_RETENTION_PROPOSAL.md`
5. `crates/photara-store/src/package/mod.rs`
6. `crates/photara-store/src/package/reader.rs`
7. `crates/photara-store/src/package/memory.rs`
8. `crates/photara-store/src/package/v1_1/mod.rs`
9. `crates/photara-store/src/package/v1_1/reader.rs`
10. `crates/photara-store/src/package/planning/mod.rs`
11. `crates/photara-store/src/package/planning/contracts.rs`
12. `crates/photara-store/src/package/planning/commands.rs`
13. `crates/photara-store/src/package/planning/io.rs`
14. `crates/photara-store/tests/package_v1_1.rs`
15. `crates/photara-store/tests/package_planning/mod.rs`

PS2 filesystem writer/journal/recovery is next. PS3 native coordinator/autosave,
PS4 switching, live conversion and deployment are not included here.
