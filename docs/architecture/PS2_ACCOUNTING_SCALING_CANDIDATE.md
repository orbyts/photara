# PS2 accounting scaling candidate

This new, disposable byte candidate tests the explicitly proposed observation and retained-file charge coordinates directly. It does not freeze a format, alter production readers, migrate existing evidence, qualify a filesystem or grant portable resource authority. The selected object is a typed accounting **subset** selector, not a complete HEAD/RootSet publication route.

The independent [Python generator](proposals/ps2/accounting/generate.py) produces [canonical vectors](proposals/ps2/accounting/linked-accounting.json). The [Rust verifier](../../crates/photara-store/tests/accounting_scaling_candidate/accounting.rs) parses their actual proposed schema IDs, exact field sets, versions and canonical frame bytes. It does not rename hypothetical records into historical codecs. The [integration tests](../../crates/photara-store/tests/accounting_scaling_candidate.rs) use coherent ancestor rehashes for structural/accounting negatives.

## Proposed shapes exercised

All records below have schema version 1, project_id and a namespaced extensions object. These are proposed coordinates, not a compatibility promise. Canonical decimal u64, lowercase SHA-256, nonnil canonical UUID and existing `QualifiedName` grammar apply.

| Coordinate | Fields beyond the common envelope |
|---|---|
| `photara.storage.local-observation` | observation_id, profile, incarnation, subject, physical, measured_extent, charged_high_water |
| `photara.storage.local-observation-leaf` | count, entries `{observation_id,observation:ObjectRef}` |
| `photara.storage.local-observation-branch` | count, children `{first,last,count,child:ObjectRef}` |
| `photara.storage.sealed-charge` | allocation_id, arena, domain_incarnation, measured_extent, charged_high_water, observation:ObjectRef |
| `photara.storage.charge-leaf` | count, charged_high_water, entries `{allocation_id,charge:ObjectRef,charged_high_water}` |
| `photara.storage.charge-branch` | count, charged_high_water, children `{first,last,count,charged_high_water,child:ObjectRef}` |
| `photara.package.retained-file-charge-leaf` | count, charged_high_water, entries `{key,conversion_source:ObjectRef,source_file,registered_charge,observation:ObjectRef}` |
| `photara.package.retained-file-charge-branch` | count, charged_high_water, children `{first,last,count,charged_high_water,child:ObjectRef}` |

Pack observation subject is `{kind:"pack",allocation_id,arena}`. Retained-file subject is `{kind:"retained-file",conversion_id,namespace:[components],path:[components]}`. Physical evidence is `{device,inode}` under the record's profile/incarnation. A charge-tree observation reference is an actual typed edge to the immutable observation record; it replaces the historical fixture's `{kind,observation_id}` observation placeholder. This is a new layout under a new proposed coordinate, not a rename-compatible old record.

Retained-file key is `{conversion_id,path:[components]}`. Ordering is canonical UUID text followed by componentwise UTF-8 lexicographic path order, not JSON rendering or concatenated pathname order. Paths reuse the actual `RelativeComponents` validation, including reserved names/control/trailing-character rules. `source_file` is the unchanged original `{byte_length,sha256}` commitment; `registered_charge` is separate. Identical bytes at distinct component paths remain distinct charge entries. Entries can share one original ConversionSource identity without merging file identity or charge.

The `example.ps2.accounting-scaling-ledger` selector has profile, incarnation, observation_root, retained_file_charge_root, sealed_charge_root, conversion_source and registered_charge. It selects roots rather than lifetime per-file observation arrays. Its total covers the three sealed allocations and retained conversion files only. Current tips, control standing pool, source-directory allowance, pins, original-token dispatch and refunds are outside this subset; the complete joined ledger must compose those obligations separately.

The whole-Blob agent adds a separate proposed observation subject variant. This accounting component explicitly dispatches only pack and retained-file; whole-blob is rejected here and must be tested by its owning component before the final shared union is specified.

## Original evidence and local scope

The candidate uses all 18 real retained files and three sealed pack allocations from the unchanged [joined fixture](proposals/ps2/joined/linked-joined.json). The joined corpus SHA is bound, original ConversionSource v1 canonical bytes are identical, and every original file commitment is checked against the supplied bytes. Sealed allocation evidence—including bytes, digest and scoped witness—and retained-file witness registry must exactly match that original fixture epoch. No old aggregate is reclassified as a refundable charge.

The illustrative profile rounds original file lengths to 4096-byte allocations. This is synthetic original evidence, not available filesystem space and not a rule allowing later high-water reduction. There are 21 selected observations: three pack and 18 retained-file observations. The charge subtotal is 90112 bytes (16384 sealed-pack bytes plus 73728 retained-file bytes). Observation records do not add a second charge merely because both an observation tree and a charge entry reference them.

`qualification:false` exists only on the corpus harness. No proposed observation record has a qualification bit that could grant writable or Saved authority. The verifier returns `scope_matches`, not `writable`. A profile/incarnation mismatch permits the tested metadata inspection but refuses the optional local-binding check; it never rewrites observations or automatically rebinds a copy. Matching strings are only an identity precondition. Trusted runtime qualification and real storage operations remain separate and unproven here. Portable resource/backing/representation records receive no device/inode fields.

## Sparse original sealed-charge proof

The source inclusion witness takes an independently authenticated original charge-tree root and source allocation key, exact canonical branch/leaf records only along that key path, the original sealed-charge record, and its original observation. All sibling descriptors remain inside the authenticated branch bytes. They are bounded summaries and hash commitments, **not edges requiring sibling bytes** in this proof.

The verifier checks exact root and each next child reference, unique path nodes, complete child field sets, sorted/disjoint ranges, counts and checked charge sums, selected child summary equality, exact terminal leaf membership, charge identity and observation reference/scope. Extra path records, sibling/root/charge/witness substitutions and structurally invalid summaries under a coherently rehashed root refuse. A trustworthy original root must come from the selected admission; this helper does not create that authority.

The fixture's complete charge tree has five nodes. The source path uses three nodes; adding its charge and observation requires 2692 canonical record bytes. The diagnostic corpus representation also stores parsed inputs and frame hex, so its sparse bundle is 12551 bytes; that redundant diagnostic container is not a proposed final witness encoding. No permanent inclusion-witness schema or layout is frozen here.

The post-source-removal test deletes every sealed/source file map, source witness map and global record map before verifying the standalone sparse proof. It succeeds using only the original root/path/charge/observation. That proves original registered membership without reopening source bytes. It does not prove source absence, retirement eligibility, pin release, directory barrier or once-only credit; the original-token retirement protocol must still establish those independently.

## Bounds, tests and limitations

These are fixture limits, not proposed lifetime ceilings: at most 1024 supplied records; canonical body at most 65536 bytes; parser depth 32, 256 members and 4096 array elements; tree recursion below 16 with fewer than 256 prior visited nodes; leaf width at most two and branch fanout 2–4; subtree count at most 4096. The sparse proof is bounded to 16 nodes and 65536 diagnostic bytes. Candidate frames use existing `PS2PKD01` tag 2 for accounting records and tag 1 for unchanged ConversionSource. Every fixed frame/body digest and exact boundary is checked.

Twelve tests pass. Coverage includes actual proposed versions/fields and opaque extension nonedges; independent observation/charge/retained tree traversal; original shared conversion identity; distinct equal-content path charges; exact original file/witness evidence; coherent undercharge/order/range/count/overflow/unknown-layout changes; portable path negatives; root/sibling/charge/witness swaps; sparse proof without the source or sibling records; profile/incarnation mismatch without rebind. Strict scoped Clippy and targeted rustfmt pass.

Commands: `cargo test --release -p photara-store --test accounting_scaling_candidate`; `cargo clippy --release -p photara-store --test accounting_scaling_candidate -- -D warnings`. Evidence is `verification/ps2-accounting-scaling-*`; copied logs normalize trailing blank lines only, preserving one terminal newline. No existing fixtures or production files were changed.
