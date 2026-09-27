# PS2 joined settled byte candidate — engineering review only

This specimen selects one complete settled snapshot through actual
`HEAD.json → photara.package.commit v1 → RootSet → StateRoot` bytes. It composes
real Graph operations, scalable inventory/index/charge branches, resource metadata,
original conversion-source retention, packed placement and allocation accounting.
It replaces projection-only equality with traversal of the selected framed bytes.

This is an **experimental engineering checkpoint**, not the final proposed
permanent wire spelling or a production reader. Its `example.ps2.*` coordinates
are incompatible with historical fixtures where fields differ; no rename or
automatic rebinding is implied. The next review layer must give exact unfrozen
`photara.*` schema/capability coordinates and independently canonicalized bytes.

## Reproducible evidence

- [Generator](proposals/ps2/joined/generate.py).
- [Positive bytes](proposals/ps2/joined/linked-joined.json), SHA-256
  `d5079e905db8181d6cd60a0b2548e4b1ddbd30e00bb1f9e1d34891e4b996f09e`.
- [Coherent negative byte deltas](proposals/ps2/joined/joined-negatives.json),
  SHA-256 `012ac6b18eeaba864a8740a7122ec24f152529020988016343e29d627aefbfd6`.
- Integration target `crates/photara-store/tests/scalable_joined_candidate.rs`;
  private reader `scalable_joined_candidate/joined.rs`.

The Python standard-library generator independently fixes 169 canonical records,
seven allocation byte streams, the original 18 conversion files and bootstrap
bytes. Regeneration is byte-identical. The negative corpus stores exact changed
byte spans plus new hashes/bootstrap bytes; applying it reconstructs fully
rehash-consistent alternate snapshots without repeating padding megabytes.

Twelve tests pass; strict scoped Clippy passes. The actual unchanged v1.1 reader
rejects this outer commit with `UnsupportedFeature` before packed lookup, with no
packed files supplied to that legacy reader.

The subsequent route work extracts a private shared reader seam accepting
already validated selected references and accounting. The original settled
entry point still checks its original bootstrap, empty tickets, null
predecessors and exact allocation/control sets. Its fixed byte corpora are
unchanged. A separate [refactor verification](verification/ps2-joined-route-seam-verification-20260927.json)
records the updated source hash, twelve passing regression tests, strict
Clippy and targeted formatting; the original checkpoint evidence remains a
record of its original sources. This extraction alone proves no transition.

## One selected bootstrap and typed closures

The original operation corpus manifest bytes are unchanged. Both outer commit
`bootstrap_sha256` and RootSet bootstrap identity equal their SHA-256. HEAD
selects the complete canonical commit digest and ID. The existing v1 outer
fields, timestamp and active authored/history/inventory projection are checked.
This settled fixture has package revision 1, no parent and null StateRoot
predecessors: it does not fabricate prior commit or prior StateRoot provenance.

The proposed reader floor is 1.3. In addition to the original required features,
the commit requires `photara.sealed-roots.v1`, `photara.resource-backings.v1`,
`example.ps2.scalable-storage.draft-v1`,
`example.ps2.typed-branches.draft-v1`, and
`example.ps2.owned-accounting.draft-v1`. This bounded reader checks the exact
known feature set before packed reads. Unknown features, wrong floor or missing
capabilities do not dispatch into another layout.

RootSet and StateRoot use the established experimental `example.ps2.root-set`
and `example.ps2.state-root` field shapes from the packed specimen. There is no
parallel phase-state selector. RootSet selects active/recovery, their operation
index, exact union inventory, conversion source and one placement:

```text
placement = {
  generation,
  active_locator: PhysicalRef, recovery_locator: PhysicalRef,
  active_ownership: PhysicalRef, recovery_ownership: PhysicalRef,
  accounting: ObjectRef
}
```

The direct locator and ownership roots bootstrap lookup without locating
themselves. Accounting is one directly selected loose canonical object; every
charge record, charge tree page, StateRoot, semantic inventory page, operation
index page and conversion descriptor is packed. Locator implementation pages
remain direct physical edges and do not index themselves.

Active has the three real operation records and recovery their first two. The
original operation verifier proves exact original receipts, accepted prefix,
authored results and journal coordinates; the joined reader resolves those
objects from its own selected locator and checks the same bytes. It walks the
new ID/ordinal branches, preserving original reciprocal entries. The source
oracle checks expected fixed semantics and never supplies a missing-object
fallback. Before/after authored digests remain nonedges.

Each StateRoot selects its own ResourceState, with its actual root UUID as the
retention-obligation origin. The shared resource validator reads a record pool
constructed exclusively from the selected physical resolver. Every returned
resource edge must have semantic frame membership. The two resource states share
captured-version/capture evidence while retaining their own backing revision,
publication evidence and obligation. No new captured-version→backing edge exists.
No independent representation edge is invented in the immutable authored Graph.

Resource metadata structural readability remains separate from retention
satisfaction. The proof exposes the synthetic profile consistency result. A
mismatched external qualification profile gives `resource_supported=false` while
the structurally valid snapshot remains readable. This does not confirm external
media, qualified durability or a Saved result.

The active semantic inventory has 33 entries and recovery 28, each exactly equal
to its typed semantic closure. Inventory implementation nodes are physically kept
but excluded from their own represented set. RootSet's union inventory includes
both complete state closures, directly selected accounting, charge dependencies
and ConversionSource. Its own pages, ownership implementation nodes and locator
pages are excluded from that represented set. Each role's locator membership is
exactly its state closure plus global inventory/charge/conversion pages and its
ownership nodes: 131 active entries, 124 recovery entries.

Recovery succeeds with both active-only allocations removed. This checks the
complete recovery state/placement closure independently; the full package check
still requires all allocations registered by its current accounting root.

## Supplied bytes, ownership and original local witnesses

Three shared sealed data packs contain 2,709, 5,585 and 2,984 bytes of actual
selected semantic frames. Unlike the branch-only specimen, their full bytes are
present and checked. Four role-specific data/metadata tips each have a fixed
131,072-byte extent, with explicit exact zero padding. Padding is owned and
charged, is never referenceable, and is not a production allocation policy.

Every selected semantic/ownership ObjectRef resolves to the expected frame tag
and exact canonical body; PhysicalRefs check the whole frame hash, offset and
length. The parser checks all framing/padding, and arenas must be exactly data or
metadata. Per-role ownership covers all and only allocations required by that
role's complete resolver, including the ownership nodes themselves and locator
pages. Sealed claims authenticate their complete extent and point to the canonical
registered charge; growing claims correspond to original registered tips.

The local model is explicitly synthetic and unqualified. Original witnesses bind
profile, incarnation, allocation ID, device and inode; portable IDs or hashes do
not grant mutation authority. Each current tip carries its selected original
witness. The accounting root also carries a bounded three-entry original sealed
observation registry; each packed charge's observation ID resolves exactly once
to that witness. Byte-identical sealed or growing inode substitutions refuse.

The existing sealed-charge shape remains exact, including observation
`{kind:"synthetic-vector",observation_id:UUID}` with no extra fields. The new
registry avoids weakening that schema or treating current matching bytes as
permission to inherit an old registered charge. The final scalable proposal must
replace the bounded inline registry with a typed observation tree or a precisely
specified trusted local registry; this array is not a lifetime per-file layout.

## Selected accounting tree and retained conversion files

The new experimental `example.ps2.accounting-tree` version 1 has the common
schema/project/extensions fields and exactly:

```text
profile, incarnation, standing_control,
tips: [{witness,arena,extent,registered_charge}],
sealed_charge_tree: ObjectRef,
sealed_observations: [{observation_id,witness}],
tickets: [],
retained_conversion: {
  source: ObjectRef,
  files: [{path,logical_bytes,registered_charge,witness}],
  logical_bytes,registered_charge,directory_control
},
total_charge
```

The packed owned-charge tree is multi-level and keyed by allocation ID. It uses
the branch specimen's exact count/range/checked-sum rules. Its three charges are
4,096, 8,192 and 4,096 bytes. Every charge's extent and arena match supplied sealed
bytes; every tip's original witness, extent and rounded registered high-water
match its supplied allocation. Shared active/recovery allocations are counted
once. No ticket or unresolved hold exists in this settled snapshot.

The ConversionSource descriptor binds the exact original snapshot file table and
all 18 files: 15 legacy package internals plus three unknown optional files. The
actual v1.1 reader validates the internal snapshot. The retained snapshot's exact
manifest bytes and descriptor bootstrap hash must equal the selected original
manifest; a separately valid alternate legacy snapshot cannot substitute for it.
Snapshot paths live under the descriptor's canonical conversion namespace,
disjoint from the allocation namespace.

Each distinct retained snapshot path has its own original synthetic local witness
and charge, even if two paths contain equal bytes. The selected row includes
`{profile,incarnation,namespace,path,device,inode}`; supplied current witness
metadata must equal it exactly. A byte-identical retained-file replacement
therefore cannot inherit the old row's registered charge. These bounded rows
model one immutable source snapshot and are not a proposed unbounded inline
retention ledger.

| Selected charge component | Bytes |
|---|---:|
| Three supplied sealed packs, rounded synthetic high-water | 16,384 |
| Four 131,072-byte tips | 524,288 |
| Eighteen retained conversion file charges | 73,728 |
| Explicit bounded conversion directory/control allowance | 16,384 |
| Standing package control allowance | 131,072 |
| **Total** | **761,856** |

The retained files contain 9,794 logical bytes; their per-file registered charges
are separate from that logical total. The canonical retained file/witness table
occupies 7,940 bytes within the 16,384-byte directory/control allowance. Actual
manifest+HEAD+commit+accounting bytes total 14,233 within the 131,072-byte standing
allowance. No transient publication controls are modeled in this settled vector.
All additions are checked; no filesystem-free-space credit or local qualification
is inferred from these synthetic values.

## Refusal evidence and remaining route

The twelve tests cover exact selected bytes and complete allocation accounting,
independent recovery, unchanged legacy refusal, outer bootstrap/timestamp/arena
validation, missing sealed bytes, original tip/sealed/retained-file identity,
resource-profile uncertainty, coherent resource/conversion wrong frame membership,
coherent alternate source bootstrap, changed active projection/capability/pin
selection, a coherently reselected undercharged tip and conversion charge refund.
Named error assertions distinguish the targeted checks from stale outer hashes.

The pin case is an explicit refusal of nonempty pins in this bounded specimen;
it is not support for general historical pin traversal. The two wrong-membership
and alternate-bootstrap variants are generated independently with rebuilt frames,
locators, ownership prefixes and HEAD commitments.

This checkpoint is one settled snapshot. It does **not** yet select admission,
phase and consumed reserve under this same accounting root during a transition.
The next integrated route must use old → Graph-published → released → credited
snapshots with these same complete schemas and actual proposed frames. The
immutable original admission must bind the exact old base and a finite target
semantic/physical plan projection, excluding the mutable accounting selector.
Publication accounting must atomically select that original admission, phase and
measured consumed charge; a separate external phase-order list cannot authorize
it. In particular, admission→full target RootSet→accounting→admission is an
invalid cycle and is not introduced here.

Commands:

```sh
python3 docs/architecture/proposals/ps2/joined/generate.py
cargo test -p photara-store --test scalable_joined_candidate
cargo clippy -p photara-store --test scalable_joined_candidate -- -D warnings
```
