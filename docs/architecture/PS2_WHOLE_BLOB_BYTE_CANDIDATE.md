# PS2 whole-Blob byte candidate

Status: **unfrozen additive storage proposal, pure disposable tests; no production codec, migration, or permanent format approval**. This closes the JSON-only placement subproof's missing existing managed-Blob case. It is not yet a complete HEAD-selected package or a replacement for the [single-HEAD route](PS2_SINGLE_HEAD_ROUTE_BYTE_CANDIDATE.md).

The [generator](proposals/ps2/blob/generate.py) independently emits [fixed canonical bytes](proposals/ps2/blob/linked.json). The private [test target](../../crates/photara-store/tests/whole_blob_candidate.rs) and [reader](../../crates/photara-store/tests/whole_blob_candidate/wire.rs) exercise them. The immutable source is the existing [D19 package golden](../fixtures/generation-two/d19-package-specimen.json), also checked by the production v1.1 package tests. Every supplied source file is compared to that golden and the actual v1.1 `validate_memory` reader verifies its complete original package, including its one managed Blob.

## Existing semantic bytes are preserved

The original `photara.project.managed-resource` v1 and managed `photara.project.representation-content` v2 canonical bytes are copied exactly, without schema renaming or semantic lowering. The managed resource still names the original Blob ObjectRef, media length/type, retention class and immutable resource/version identity. Its representation still binds that exact managed resource and content digest. The raw original Blob is the five bytes `photo`.

ObjectRef remains the existing `{kind,sha256,byte_length}` contract. SHA-256 hashes the raw Blob bytes without a new domain or framing prefix. Existing actual `ObjectRef::Ord` is tested to order Blob before Json even when their digest and length are identical. A JSON ObjectRef cannot masquerade as a selected Blob. The existing u64 decimal length accepts a length above u32 without introducing a new wire-wide large-object ceiling; this test does not allocate that large payload.

A second directly selected Blob is empty. Its SHA is the SHA-256 of zero bytes, and its raw extent is zero. This tests the storage boundary; it is not a claim that the legacy golden's authored representation was changed to an empty image.

## Explicit whole-allocation placement

The proposed capability is `photara.whole-blob-storage.v1`. A proposed `photara.storage.locator-leaf` v1 entry for a Blob has:

```text
{object: BlobObjectRef, membership: "blob",
 physical: {kind: "whole-blob", allocation_id: UUID,
            byte_length: DecimalU64, sha256: Digest}}
```

That physical reference covers an entire immutable raw allocation. It has no frame offset, frame header, arena-relative record digest, or internal chunks. No new binary framed-body tag is proposed. The allocation metadata and ownership claim explicitly declare `layout:"whole-blob"`. A framed reference, unknown extra offset, JSON membership or a packed allocation layout refuses; a reader does not infer storage layout from payload bytes.

The proposed `photara.storage.allocation-claim` v1 contains `allocation_id`, `arena:"data"`, `layout:"whole-blob"`, `owned_extent`, `authenticated_prefix:{byte_length,sha256}`, `sealed:true`, and `sealed_charge:ObjectRef`, in addition to schema/project/extensions. The prefix covers the whole Blob, including the empty Blob. Its exact sealed charge and local observation are selected independently.

`photara.storage.sealed-charge` v1 uses the proposed [accounting candidate](PS2_ACCOUNTING_SCALING_CANDIDATE.md) fields: allocation/arena/domain incarnation, measured extent, charged high-water, and an ObjectRef to immutable local observation. This component adds the explicitly proposed local-observation subject variant `{kind:"whole-blob",allocation_id,arena:"data"}`. The observation otherwise preserves observation ID, profile/incarnation, device/inode, measured extent and charged high-water. The accounting component's frozen reader supports only its tested pack/retained-file variants; a future combined reader must explicitly dispatch the full proposed union rather than silently treat raw Blobs as framed packs.

The enclosing `example.ps2.whole-blob-selection` is a bounded test projection with two logical Blob refs, one locator leaf, exact ownership/charge/observation refs, unchanged managed/representation refs and the local accounting bounds. Its arrays are not a proposed lifetime RootSet or global registry encoding. Its exact referenced control records and supplied physical allocation set contain neither missing nor extra members.

## Structural opening and strong audit are different

`structural()` checks canonical metadata, typed logical/physical dispatch, current allocation length, complete ownership, original selected local observation, and charge coverage. It **does not read/hash media content as part of ordinary structural open**. `audit()` explicitly performs full raw Blob digest verification, suitable for a full audit or capture proof.

The test corrupts one byte while keeping length and native identity unchanged: structural opening still succeeds, and the explicit full audit refuses. This prevents the specimen from implying that ordinary package open/autosave/root turnover rehashes large media. A same-byte native allocation replacement or incarnation mismatch refuses the local observation check; copied bytes do not automatically acquire the old local identity or charge authority.

The scope uses a synthetic, unqualified profile and injected original identity observations. Matching them is not a provider qualification or a durably saved claim. Device/inode observations are local accounting evidence, not portable managed-resource fields.

## Empty allocation and namespace accounting

The local test model rounds data charge to 4,096-byte units. The five-byte Blob has 4,096 data charge and the empty Blob has zero data extent/charge. **Zero raw extent does not mean allocation creation is free.** The selection additionally retains 4,096 bytes of namespace/creation allowance per Blob, a separate 4,096-byte directory allowance, and a 65,536-byte standing control pool. The ten actual immutable control records occupy 40,960 rounded bytes within that pool.

The selected modeled total is:

```text
4,096 data + 8,192 per-allocation namespace + 4,096 directory
+ 65,536 standing controls = 81,920 bytes
```

These are explicit conservative fixture allowances, not measured filesystem inode costs, refunded namespace credit, a new product default, or OS free-space observations. Removing or zeroing the empty-file allowance refuses. Checked arithmetic rejects total overflow. A future durable publisher must bind these allowances to its original admitted local profile and complete lifecycle; this byte component supplies no allocation-create, unlink or sync implementation.

The current raw allocation namespace is distinct from the original legacy package's reference-fixture paths. Those original files are immutable test input used to establish existing semantics, not an implicitly retained conversion source or an uncharged part of this projection's selected allocation total. A final conversion route must account for its retained original snapshot separately; a copied current Blob and retained original file are two actual physical allocations unless an explicit canonical allocation identity proof says otherwise.

## Evidence and remaining composition

Six tests pass, covering exact legacy source/record preservation, canonical byte hashes, full Blob ownership and empty allocation, explicit strong audit, original ObjectRef kind/order/u64 length, framed/raw dispatch refusal, missing ownership, native witness replacement, nonfree namespace accounting, overflow, required capability and optional-extension nonedges. Strict lint, formatting and exact generator reproduction are recorded in the [evidence manifest](verification/ps2-whole-blob-candidate.json).

The final proposed-coordinate package still needs one actual HEAD-selected closure combining framed JSON/locator/ownership storage, whole-Blob placement, scalable observation/conversion charges, factored resources, original-token phases and compatibility dispatch. This component does not claim that composition, a permanent reader, or universal storage qualification.

## Prototype parser and composition limits

The selected Blob list is bounded to sixteen entries. The fixture loader reads
supplied JSON/hex into memory and does not impose a separate metadata-body or
aggregate-input size cap; its control parser uses serde_json's default recursion
behavior plus exact canonical re-encoding. The standing-control check happens
after loading and does not substitute for an input parser bound. A final shared
reader needs explicit bounded canonical parsing before allocation. The u64
ObjectRef test establishes length representation only, not streaming I/O or a
measured multi-gigabyte capture. No product maximum follows from these probes.

The original D19 project identity differs from the route/resource/accounting
project identity. Its exact-byte compatibility proof remains a separate case.
An integrated specimen must either validate an independent D19 HEAD closure or
construct and identify genuinely new project-consistent metadata; rewriting a
ProjectId cannot be presented as preserving the original canonical bytes.
