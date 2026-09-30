# Selected Blob shared API patch contract

**Unfrozen private-test implementation contract.** The [preparation checkpoint](PS2_SELECTED_BLOB_PREPARATION.md) remains unchanged. The resulting [shared candidate and evidence](PS2_SELECTED_BLOB_SHARED_CANDIDATE.md) now implement this design through one physical/root driver; exact Rust signatures supersede the planning signatures below (the Blob map stores allocation IDs directly because its key already carries the full ObjectRef). Existing Graph and transition entry points remain preserved. No production interface or permanent wire is introduced.

## Preserve JSON APIs; add mixed identities only where needed

Keep `wire::Key = (String,u64)`, `key(Value)->Result<Key>`, `Resolver.objects:BTreeMap<Key,(Value,u8)>`, accounting keys, decoder maps, `RoleProof.semantic` and `Proof.global` **JSON-only**. Do not encode a kind prefix into a digest string or reinterpret those existing sets as mixed identities.

New [keys.rs](../../crates/photara-store/tests/selected_blob_candidate/keys.rs) implements `MixedObjectKey(ObjectRef)` with original typed ObjectRef ordering, strict parsing, `from_json_key`, fallible `json_key`, `object_ref`, and canonical value conversion. JSON zero-length references still refuse; empty Blob and u64 lengths remain representable. Blob-to-JSON conversion refuses even when hash/length equal a JSON key. Its `TreeKey::{Object,Id,Ordinal}` replaces formatted comparison strings inside the existing walker; selected tree kind fixes the variant, so cross-domain enum ordering confers no wire meaning.

The shared patch adds:

```text
Resolver.blobs: BTreeMap<MixedObjectKey, BlobLocation>
BlobLocation = { allocation_id:String, reference:ObjectRef }
Resolver::blob(Value) -> Result<&BlobLocation>       // Blob only
Resolver::mixed_keys() -> Result<BTreeSet<MixedObjectKey>>
RoleProof.blob_semantic: BTreeSet<MixedObjectKey>    // Blob entries only
Proof.blob_global: BTreeSet<MixedObjectKey>         // Blob entries only
```

JSON `get`/`any` and `.objects` keep their old meaning. Inventory entry/bound comparisons and locator object comparisons use mixed keys. Root-placement keys remain JSON StateRoot refs and explicitly require checked JSON conversion. Tree implementation-node/path sets remain JSON. Exact mixed role/global closure is the disjoint union of checked conversions of the existing JSON sets and the new Blob sets. The existing admission decoder still sees only JSON; its public wrappers reject a nonempty raw set until its codec explicitly supports that case.

## Private raw-provider boundary

New [provider.rs](../../crates/photara-store/tests/selected_blob_candidate/provider.rs) supplies:

```text
RawDescription = { extent:u64, device:u64, inode:u64 }
trait RawMetadata {
  allocation_ids() -> Result<BTreeSet<String>>;
  describe(allocation_id) -> Result<RawDescription>;
}
trait RawAudit: RawMetadata {
  audit(ObjectRef, allocation_id, original_description) -> Result<()>;
}
observe_blob(&dyn RawMetadata, ObjectRef, allocation_id, original_description)
  -> Result<RawDescription>
```

Structural verification receives **only `&dyn RawMetadata`**. No trait method returns raw bytes. The [fixture adapter](../../crates/photara-store/tests/selected_blob_candidate/media_adapter.rs) owns the existing private `Media` and derives its exact ID set from the same bounded allocation input; callers cannot supply a smaller registration-name list. `RawAudit` is provided only to the explicit audit entry, which consumes the verified selected mapping. The test counter still distinguishes zero structural reads/hashes from explicit five-byte audit.

Preserve `World.allocations` as the framed map used by the current recipe decoder. Add `World.raw_descriptors:BTreeMap<String,RawDescriptor>` without raw Vecs; `RawDescriptor={arena:"data",layout:"whole-blob"}` is exact dispatch metadata. A new `load_with_raw` partitions explicit allocation layouts, parses/bounds metadata, and requires raw IDs to equal the provider's IDs; old `load/load_transition` continue to reject raw input. Combined raw/framed count and extent limits are checked, not separate allowances that double the existing cap. A Blob cannot silently fall back to `Allocation.bytes`.

`physical()` stays framed JSON only. The existing `Resolver::new` adds a `membership:"blob"` arm that validates exact whole-Blob PhysicalRef fields, reference length/hash commitments, data layout and original registered description without hashing media. All other memberships require JSON refs. Across selected roles, one raw allocation ID must map to one exact Blob ObjectRef; sharing that same identity is allowed. Ownership adds an explicit `whole-blob` claim arm: whole extent/hash commitment equals that logical Blob, sealed charge and selected original observation agree, and raw allocation IDs participate in the existing exact per-root/global owned set. Framed authenticated-prefix hashing remains unchanged.

## Package context and semantic hooks

Add a private immutable context built before opening the candidate:

```text
PackageIdentity = { project_id, library_id, bootstrap_sha256,
                    original_manifest_bytes }
PackageContext<'a> = {
  identity: PackageIdentity,
  expected_required_features: BTreeSet<String>,
  registration: &'a accounting::Evidence,
  semantics: SemanticMode<'a>,
  raw: &'a dyn RawMetadata
}
SemanticMode<'a> = GraphOriginal | PreservedLegacy(&'a dyn RootSemantic)
trait RootSemantic {
  authored_history(state:&Value, resolver:&Resolver)
    -> Result<SemanticMembers>;
}
SemanticMembers = { json:BTreeSet<Key>, blobs:BTreeSet<MixedObjectKey> }
```

Context fields have private constructors for the existing Graph fixture and validated D19 fixture. They are not deserialized from the selected commit. D19 identity/immutable semantic authority comes from `TrustedLegacy::d19`; original accounting registration comes from the separately supplied synthetic registration epoch. All selected metadata must match context identity and exact original manifest bytes. Required features are the compiled candidate union of original bootstrap features and that candidate's supported additions; ResourcePolicy extras remain explicit additions. The selected feature list never chooses a more permissive context. Actual canonical codec and reader floor checks still run.

Registration retains `accounting::Evidence`'s project/profile/incarnation, allocation witnesses/charges, original ConversionSource bytes and retained-file evidence. `evidence(w,conversion)` becomes context-bound observation checking: compare supplied framed metadata and raw provider descriptions to original registration before calling the unchanged generic ledger validator. Never manufacture original witnesses/highwaters from the selected ledger. Null conversion requires no source files, no original conversion evidence, canonical empty retained-charge tree and zero retained-directory charge.

Keep Graph `authored_closure`, original receipt oracle, prepared Core output and transition predecessor checks intact under `GraphOriginal`. For `PreservedLegacy`, call the adapter on **actual selected StateRoot refs**, using authenticated JSON resolution and structural Blob mapping; compare its returned membership to the selected inventory. Shared code, not the adapter, validates StateRoot schema/project/library/bootstrap/root ID, authored revision and independent root placement.

Split the current `operations()` into shared typed index validation plus the existing nonempty Graph path. The preserved-legacy mode supports **only** zero accepted operations: both ID/ordinal trees must be canonical empty selectors, accepted count zero, accepted-prefix hash recomputed from actual project/library/bootstrap using the unchanged domain, and journal inclusion null. Nonzero operations refuse this mode. Do not return early before semantic/ownership/inventory checks. Null ResourceState is legal only when the managed-backing feature is absent; otherwise keep ResourcePolicy validation unchanged. Empty pin/evidence trees have exact schemas/counts and do not skip root or global validation.

## Entry-point and edit map

Keep `verify`, `verify_with`, `verify_transition`, `verify_selected_operation`, `recovery` and `recovery_with` signatures unchanged; wrappers build the existing Graph context and no-raw provider. Add `verify_context(w,context,resource_policy)` and `recovery_context(...)`. Both delegate to the same `verify_inner`/recovery driver. Thread context through bootstrap/schema identity validation, framed trees, resolver, role validation, ownership and selected accounting. Existing free `schema` keeps its Graph-default behavior for external helpers; internal generic paths use `schema_in(context,...)`. Existing Transition/OperationSelection structures and original-token authority checks remain intact, with explicit no-raw mode guard.

Expected edits after handoff: shared `integrated_wire_candidate/wire.rs` contains the substantive patch; new helper modules are included once and re-exported as needed. Shared accounting should need no schema change for this settled D19 case. The D19 target supplies context/provider adapters. One origin unit-test `Resolver` literal needs an empty Blob map or a JSON-only constructor. Admission decoder/admission code should require no key/map rewrites; rerun all its original tests. If that expectation fails, stop and revise the API rather than changing `Key` transitively.

Gate order: helper tests → unchanged settled/origin/admission/phase regressions → shared reader accepts the D19 corpus → coherent mixed-key/raw-layout/ownership/global-union negatives → counter-enforced structural/audit tests through that shared entry. New helper tests alone do not close full HEAD, converted Blob snapshot or Graph-with-media transition gaps. The three utility tests are in [selected_blob_api_candidate.rs](../../crates/photara-store/tests/selected_blob_api_candidate.rs); original preparation sources and evidence stay unchanged.
