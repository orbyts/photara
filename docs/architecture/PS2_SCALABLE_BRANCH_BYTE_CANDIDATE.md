# PS2 typed branch byte candidate — review only

This new, bounded specimen supplies exact proposed multi-level semantic
inventory, operation index and owned-charge records. It extends the historical
[operation candidate](PS2_SCALABLE_OPERATION_BYTE_CANDIDATE.md) without changing
its bytes or adding a production reader. All new schema IDs, capability spelling,
limits and layout choices remain experimental. This is not a wire freeze.

The positive corpus is independently assembled by Python's standard library;
Rust canonical serialization and schema-directed traversal are checked against
those fixed bytes. Regenerating does not invoke the Rust implementation.

- Generator: [branches/generate.py](proposals/ps2/branches/generate.py).
- Fixed corpus: [linked-branches.json](proposals/ps2/branches/linked-branches.json).
- Corpus SHA-256: `872093f479640079f95e3e97052469b8de910e07f26987e8126dbe32198c258a`.
- Linked operation corpus SHA-256:
  `b18f3d5960bff213c35ea12d9f2e32791eb83cc904046d9f47ed21f93ada2421`.
- Pure integration target: `crates/photara-store/tests/scalable_branch_candidate.rs`;
  private traversal: `scalable_branch_candidate/trees.rs`.

## Proposed dispatch and schemas

The fixture projection uses `kind: "typed-branches-draft"`, proposed reader floor
1.3 and both required capabilities, in lexical order:
`example.ps2.scalable-storage.draft-v1` and
`example.ps2.typed-branches.draft-v1`. Missing, duplicate or unknown required
capabilities refuse; neither the reader floor alone nor a flat selector enables
these records. This finite reader recognizes only those two capabilities.

Each new record has `{schema:{id,version:1},project_id,extensions}` and the fields
below. Schema IDs have prefix `example.ps2.`. Decimal values are canonical unsigned
64-bit strings; ObjectRefs retain the existing kind, SHA-256 and byte-length
fields. Unknown required fields or versions refuse. Optional extension contents
are not edges.

| Record | Proposed fields and typed edges |
|---|---|
| `inventory-leaf` | `count`, `entries:[ObjectRef]`, sorted by `(sha256,numeric byte_length)` |
| `inventory-branch` | `count`, `children:[{first:ObjectRef,last:ObjectRef,count,child:ObjectRef}]`; child targets only inventory leaf/branch |
| `operation-id-leaf` | Existing operation candidate's unchanged `count,entries` shape |
| `operation-id-branch` | `count`, `children:[{first:UUID,last:UUID,count,child:ObjectRef}]`; child targets only ID leaf/branch |
| `operation-ordinal-leaf` | Existing operation candidate's unchanged `count,entries` shape |
| `operation-ordinal-branch` | `count`, `children:[{first:Decimal,last:Decimal,count,child:ObjectRef}]`; ordinal comparison is numeric |
| `owned-charge-leaf` | `count`, `charged_high_water`, `entries:[{allocation_id,charge:ObjectRef,charged_high_water}]`, ordered by allocation UUID |
| `owned-charge-branch` | `count`, `charged_high_water`, `children:[{first:UUID,last:UUID,count,child:ObjectRef,charged_high_water}]` |
| `sealed-charge` | Existing packed candidate fields: allocation/arena/domain identity, measured extent, charged high water and explicitly synthetic observation |

The operation-index-tree root retains its earlier fields. Its `by_id` and
`by_ordinal` now dispatch by the explicit leaf/branch schema, under the new
required capability. This does not reinterpret historical flat arrays or alias
an old inventory schema. The earlier bounded `inventory-root` with an inline
array remains a historical specimen; these branch roots are distinct typed nodes.

Every branch's child summary must equal the independently traversed subtree's
first key, last key, count and, for charges, exact sum. Flattened keys are strictly
increasing and unique. Count and charge sums use checked arithmetic. Distinct
paths in one tree cannot name the same node. Identical subtrees shared by active
and recovery are legal: each root is validated with its own traversal set, then
physical membership is unioned once.

This specimen bounds leaves to 1–3 entries, branches to 2–4 children, a tree to
256 structural nodes and depth below 16, and flattened cardinality to 4,096.
The generator uses fanout two. A singleton remainder is promoted without a unary
branch, so terminal depths need not be uniform. These are probe bounds and a
proposed range-tree shape, not a production balance or fanout policy.

## Actual coverage and physical ownership

The active role contains three real operation receipts; recovery contains their
first two, with the original selected authored state and accepted/journal
coordinates. The existing operation verifier derives both semantic closures from
actual Core/PS1 evidence. The new verifier replaces only their old leaf-index
structures, traverses every new index and requires identical original entries,
receipts, root identity and accepted prefix. Original semantic objects must be
present and byte-identical in the new pack; an external oracle is used to check
expected semantics, never as a missing-object read fallback.

The active semantic inventory has 25 members and recovery 20. These include
operation index implementation records because they are typed semantic
ObjectRef edges. Inventory structural pages are excluded from their own
represented set, avoiding self-inventory cycles, but included in exact locator
membership and the physical keep set. Charge records and charge tree nodes have
ownership membership. Direct locator implementation records are physical edges,
not objects indexed by themselves.

There are 75 packed objects. The data allocation has 48,979 bytes of nonpadding
frames in a 65,536-byte owned extent. A separate 65,536-byte metadata allocation
contains the 26,415-byte canonical locator body, its 16-byte frame header and
padding. The existing candidate framing parser checks every frame and all zero
padding; the direct locator PhysicalRef checks exact frame offset, length and
hash. ObjectRefs check exact canonical bodies and typed frame membership.

The sealed-charge tree stores six sealed allocation observations in tagged
ownership frames, with six unique 4,096-byte charges. Current data and metadata
tip observations each cover their complete 65,536-byte extent, including the
charge tree and locator themselves. Total allocation charge is exactly
155,648 bytes, with shared allocations counted once. There is no loose lifetime
file per charge and no attempt to measure a charge record recursively through
its own sealed extent.

These observations are synthetic fixed inputs. The six sealed allocations'
content bytes are not supplied by this branch-only corpus; this validates exact
charge-tree coverage against the explicit observation table, not a sealed-content
or retirement proof. The selector and generator corpus are test scaffolding and
are not included in that allocation total. No standing control reserve, physical
filesystem observation, publication guarantee or complete package reserve is
claimed here. Full selected ownership/claim and control accounting belong in the
joined phase/bootstrap candidate.

## Verification and refusal evidence

Nine tests pass. The byte test checks every fixed record's canonical UTF-8 and
SHA-256, every selected framed reference and exact combined locator membership.
Positive traversal includes four-level inventory and charge trees and a
three-level active ID/ordinal tree. Recovery succeeds after all active-only
objects are removed from the candidate store; active then refuses.

Negative tests cover wrong child boundaries/counts/order, numeric count overflow,
a coherent two-entry charge leaf adding `u64::MAX + 1`, top-level charge overflow,
duplicate node paths, duplicate ordinal entries, missing inventory coverage,
structurally valid but altered original operation entries, missing charge objects,
wrong membership, extra unreachable locator entries, absent storage ownership,
inconsistent observation/extent/total, unknown fields/version/capability,
flat/tree confusion, and optional reference-shaped extension values as nonedges.
The summary-prepass and leaf overflow tests assert the exact
`tree aggregate overflow` result.

A content-addressed self-cycle cannot be constructed with valid finite bytes
without defeating its digest commitment. The cycle corruption test therefore
checks that replacing a committed node with a self-reference refuses at exact
byte identity before traversal. The independent per-tree visited-node guard is
exercised by a validly rehashed duplicate-subtree alias. This does not claim an
actual SHA-256 fixed-point cycle vector.

Negative edits are rehashed with the Rust candidate helpers where appropriate,
so structural rejection is not merely an unchanged ancestor hash. Positive
expected bytes remain independently fixed by the Python generator.

Commands:

```sh
python3 docs/architecture/proposals/ps2/branches/generate.py
cargo test -p photara-store --test scalable_branch_candidate
cargo clippy -p photara-store --test scalable_branch_candidate -- -D warnings
```

## Composition boundary

The `selector` is a fixture projection, not a second authoritative package HEAD.
This group proves tree formats and their physical bytes, not complete bootstrap,
resource/conversion dispatch or original-token phase publication. Its traversal
returns exact entries and structural node identities for composition into one
HEAD → commit → RootSet → StateRoot candidate, alongside the operation, resource
and phase groups. That joined candidate must traverse the actual selected bytes,
include every required allocation in ownership and reconcile one accounting root;
merely hash-linking these projection files will not establish complete closure.
