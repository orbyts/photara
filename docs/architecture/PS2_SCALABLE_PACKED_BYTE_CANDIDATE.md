# PS2 packed byte and independent closure candidate

Status: **unfrozen, disposable pure-test proposal, 2026-09-27**. This is a
concrete byte experiment for the [scalable review candidate](PS2_SCALABLE_WIRE_REVIEW_CANDIDATE.md),
using the [earlier appendix](PS2_PRODUCTION_WIRE_APPENDIX.md)'s canonical JSON,
ObjectRef and accepted-prefix meanings. All new storage IDs, versions, frame
magic, sizes and reader floor below are proposed for review. No production
reader/writer, compatibility promise, migration, automatic rebinding or storage
qualification is added.

The [standalone test](../../crates/photara-store/tests/scalable_wire_candidate.rs)
and its [private verifier](../../crates/photara-store/tests/scalable_wire_candidate/wire.rs)
operate entirely on byte arrays. The authored/history leaves are explicitly
named **closure probes**, not production authored schemas or proof of Core
Graph semantic validation. The operation root contains only the empty accepted
prefix; real mutation/receipt/index vectors remain separate work. Within that
scope the fixture is fully linked: every required reference names actual exact
bytes, including both independent locators, their implementation pages, all
ownership records and selected bootstrap/accounting records.

## Bootstrap and capability dispatch

`manifest.json`, `HEAD.json` and the outer `photara.package.commit` v1 envelope
keep the earlier meanings. HEAD alone selects the exact commit ID/digest; the
bootstrap identity is the SHA-256 of the original manifest bytes. No second
manifest or accounting HEAD selects packs. The pure-test corpus supplies these
three files as exact byte arrays.

The proposed extra required capability is
`example.ps2.scalable-storage.draft-v1`. The corpus also requires
`photara.sealed-roots.v1`, the exact experimental RootSet discriminator
`scalable-draft`, and a proposed minimum reader coordinate 1.3. None of these
spellings is reserved. Feature/floor checks precede any packed lookup; the
proposed floor does not by itself imply support for the capability. A flat
inventory target cannot be interpreted as the new typed inventory root.

RootSet contains inline `placement` with exactly:

```text
{ generation:Decimal,
  active_locator:PhysicalRef, recovery_locator:PhysicalRef,
  active_ownership:PhysicalRef, recovery_ownership:PhysicalRef,
  accounting:ObjectRef }
```

The four direct roots bootstrap packed lookup without locating themselves
through an uninitialized locator. RootSet active/recovery ObjectRefs are then
resolved through their respective locator. The accounting reference, its one
sealed-charge dependency, and RootSet's union inventory are directly
bootstrapped canonical JSON objects, modeled as
`objects/json/<sha256>.json`; their exact ObjectRefs remain commit-selected.
They never supply an alternative selected root.

This bounded loose charge is a bootstrap specimen, **not** a lifetime one-file-
per-charge layout proposal. The scalable design still requires typed owned
charge storage and bounded direct accounting roots. Selecting additional pins,
conversion sources or managed resources requires their full separate typed
closures; this initial vector explicitly selects none.

## Exact framing and references

The candidate allocation namespace maps a canonical nonnil lowercase UUID to
`allocations/<allocation_id>.pack`. Arena is an independently checked `data` or
`metadata` tag; a UUID has one arena throughout the selected closure. A UUID is
not a native path, inode, device identity or writable admission authority.

Every allocation is an exact concatenation of frames, with no unframed trailer:

| Offset | Width | Exact candidate meaning |
| --- | ---: | --- |
| 0 | 8 | ASCII `PS2PKD01` |
| 8 | 1 | Tag: 0 padding, 1 semantic JSON, 2 ownership JSON, 3 locator JSON |
| 9 | 1 | Flags, exactly zero |
| 10 | 2 | Reserved bytes, exactly zero |
| 12 | 4 | Unsigned little-endian payload byte length |
| 16 | stated length | Payload |

Tags 1 and 2 occur only in data allocations; tag 3 only in metadata allocations.
Their payload is Photara canonical JSON UTF-8, without BOM/trailing newline.
Unknown tags, nonzero reserved bytes, truncated header/body, overflow,
noncanonical JSON and duplicate JSON keys refuse. The pure verifier bounds each
allocation/JSON value at 1 MiB and traversal at 128 physical reads; these are
review-fixture bounds, not production limits.

Tag 0 is a proposed **explicit padding frame** with a nonempty all-zero payload.
It can occur in either arena, creates no edge and cannot be a PhysicalRef target.
Its entire frame contributes to actual file length, owned extent and synthetic
charge. Malformed or truncated padding refuses even when no selected record
points into it. Padding is not a free reservation or an allocation policy.

`PhysicalRef` has exactly these fields, without extensions:

```text
{ allocation_id:Id, arena:"data"|"metadata", offset:Decimal,
  byte_length:Decimal, record_sha256:Digest }
```

Offset is the start of the **complete frame**. Byte length and SHA-256 cover that
complete frame, including its header. The offset must be an actual frame
boundary; an interior slice cannot masquerade as another frame. A complete
frame may be shared by references, but no differently bounded overlapping
reference can validate.

The existing JSON ObjectRef remains exactly
`{kind:"json",sha256:Digest,byte_length:Decimal}`. Its digest and length cover
only the exact canonical JSON payload, with no new digest domain. A locator
entry proves **both** the logical payload ObjectRef and the complete framed
PhysicalRef. Prefix hashes cover the exact raw allocation prefix, including any
frames/padding in that prefix. Neither a prefix digest nor an owned extent is
misrepresented as a digest of the entire growable allocation.

All Decimal values are canonical unsigned u64 strings; `"00"`, signs, fractions
and overflow refuse. Schema versions and the outer reader coordinate retain
JSON integer representation. UUIDs and digests are lowercase canonical values.
The empty accepted prefix retains the existing domain
`photara.package.accepted-prefix.v1` and the exact project/library/bootstrap/
ordinal inputs from the earlier appendix. Storage experimentation does not
change accepted-operation identity.

## Typed nodes

Every new named record has exact common fields
`schema:{id,version:1}`, `project_id:Id`, and `extensions:{...}`. New schema IDs
are `example.ps2.<suffix>` below. Unknown required fields or schema tags refuse.
Namespaced optional extension values are retained as canonical payload bytes,
never traversed for references even when they look exactly like ObjectRefs.

| Suffix | Exact additional fields and validation |
| --- | --- |
| `locator-leaf` | `count:Decimal, entries:[{object:ObjectRef,membership:"semantic"\|"ownership",physical:PhysicalRef}]`. Strictly unique/sorted ObjectRef keys. |
| `locator-branch` | `count:Decimal, children:[{first:ObjectRef,last:ObjectRef,count:Decimal,child:PhysicalRef}]`. At least two children; recursively checked exact first/last/count, disjoint ordered ranges, total count. Children target locator frames. |
| `inventory-root` | `count:Decimal, entries:[ObjectRef]`. Exact sorted/unique set and count; this initial schema is a bounded leaf root. Scalable inventory branches are not implemented by this vector. |
| `ownership-claim` | `allocation_id:Id,arena,owned_extent:Decimal,authenticated_prefix:{byte_length:Decimal,sha256:Digest},sealed:bool,sealed_charge:null\|ObjectRef`. Exact allocation identity/extent, prefix at most extent; sealed means full prefix and registered external charge; growable means no sealed charge and an exact current-tip registration. |
| `ownership-branch` | `count:Decimal, children:[{allocation_id:Id,node:PhysicalRef}]`. At least two children; separator equals the recursively checked child's minimum allocation key. Flattened keys are unique/sorted, total leaf count exact. Children target ownership frames. |
| `sealed-charge` | `allocation_id:Id,arena,domain_incarnation:Id,measured_extent:Decimal,charged_high_water:Decimal,observation:{kind:"synthetic-vector",observation_id:Id}`. Unique allocation/domain charge, high-water at least extent; identity/extent equal each claim. Observation is explicitly synthetic, not a local filesystem observation or qualification. |
| `physical-accounting` | `domain_incarnation:Id,standing_control:Decimal,tips:[{allocation_id:Id,arena,owned_extent:Decimal,charged_high_water:Decimal}],sealed_charges:[ObjectRef],unresolved:[],total_charge:Decimal`. Tips strictly sorted/unique; tips and sealed generations disjoint. Exact arithmetic counts every shared allocation once. |
| `probe-leaf` | `label:String,links:[ObjectRef]`. Only links are edges; this explicitly experimental semantic probe is not a production authored record. |
| `history-probe` | `retained:[ObjectRef]`. Exact semantic edges; empty in this specimen. |
| `operation-index-root` | `accepted:{through_ordinal:Decimal,prefix_sha256:Digest},operation_ids:[],ordinals:[]`. Only ordinal zero is supported here, with recomputed existing prefix formula. |

ObjectRef key order in these all-JSON vectors is digest then **numeric** byte
length, consistent with the earlier appendix's full Blob-before-Json ordering.
Blob payloads are outside this initial specimen rather than silently accepted.
Ownership keys are canonical allocation IDs, in lexical UUID order. This
explicitly avoids reusing the furnace's unrelated numeric locator key space.

`state-root` uses every StateRoot field from the earlier appendix: library and
bootstrap identity, root ID, authored revision, authored/history/resource/index
references, accepted coordinate, journal inclusion, predecessor, inventory and
extensions. Here resource state and journal inclusion are null; predecessor is
a digest/revision commitment that creates no edge. The experimental schema ID
selects the typed inventory/index targets explicitly. `root-set` likewise keeps
its earlier fields, changes kind/schema explicitly, and adds placement. Outer
authored/history/inventory equal the active root exactly; RootSet operation index
equals active's. The corpus does not claim a general authored schema validator.

## Three independent closure equalities

This makes explicit a distinction left ambiguous by the earlier scalable draft's
phrase “logical inventory equals the typed semantic/ownership closure.” It does
**not** silently add physical implementation objects to the earlier StateRoot
semantic inventory:

1. StateRoot semantic inventory equals the transitive typed semantic edges from
   its authored/history/resource/index roots. It excludes StateRoot and its own
   inventory to avoid cycles. RootSet union inventory equals both semantic
   closures plus their StateRoots/inventories and separately selected accounting/
   charge dependencies; it excludes itself and the containing commit/HEAD.
2. Each locator's semantic membership is exactly that StateRoot closure plus
   StateRoot and inventory. Its ownership membership is exactly the ownership
   nodes reached from the direct ownership root. Extra unreachable ownership
   locator entries refuse. Ownership descriptors cannot pass as semantic leaves.
3. Physical ownership equals the allocation set needed by semantic records,
   ownership nodes **and all locator pages**. Every referenced frame fits the
   claimed extent. Missing a metadata allocation's claim refuses even though
   all semantic records remain readable. Direct locator pages do not need
   entries in their own locator and do not enter a self-referential inventory.

The positive fixture uses one 770-byte sealed allocation shared by both roots
and separate 16,384-byte data and metadata tips for each root. Ownership nodes
live in their respective data tips, including the claim for that very tip;
locator pages live in the corresponding owned metadata tip. Growable claims
have the empty authenticated prefix, while every selected record/page has its
own exact framed digest. Sealed claims authenticate their entire sealed prefix.

The fixed 16 KiB final extents close with padding frames. An exploratory exact-
length fixed point oscillated when changing claim digests reordered locator
ranges with different decimal-length widths. This corpus therefore proves a
finite explicit layout, **not** a universally convergent self-sizing algorithm.
It does not change the existing furnace layout.

Active/recovery have different authored records, StateRoots, inventories,
locators and ownership nodes. Each verifies after all nonshared allocations of
the other root are removed. A whole-package verification then correctly refuses
because its other required root is absent. Optional extension and predecessor
commitments require no invented retained allocation.

The synthetic charge is 4,096 B for the sealed allocation, 16,384 B for each of
four final tips, and a 65,536 B bounded bootstrap allowance: total 135,168 B.
The verifier separately enumerates the selected HEAD, commit, manifest, accounting,
sealed charge and RootSet inventory: **5,805 actual bytes**, all within that
allowance. It deduplicates direct loose references by exact ObjectRef. This is a
snapshot byte bound only; no simultaneous temporary publication files, qualified
barrier, disk reservation, refund, maintenance progress or writable admission is
claimed. Sharing active/recovery does not multiply the sealed charge.

## Fixed vectors, negatives and reproduction

The [linked corpus](proposals/ps2/scalable/linked-vectors.json) records deliberately
non-key-sorted input values, literal expected canonical UTF-8, exact byte lengths,
SHA-256, complete frame hex/tag/length/hash, packed allocation bytes and named
expected logical closures. A shared pool deduplicates identical specimens. The
[generator](proposals/ps2/scalable/generate.py) uses only Python standard-library
JSON sorting/UTF-8, explicit little-endian framing and hashlib. It never calls
the Rust codec. Rust compares against those committed literal bytes; expected
bytes are not generated by the serializer under test.

Fourteen linked scenarios include one positive and thirteen semantic refusal
cases. Negative ancestors and HEAD hashes are regenerated independently, so the
tests reach the intended closure/type checks rather than failing merely on an
outer stale hash. Cases cover missing capability, flat/scalable confusion,
missing metadata ownership, wrong membership, inconsistent shared charge,
dangling placement, optional data promoted into a real edge, recovery borrowing
the wrong active object, wrong bootstrap, targeting padding, extra ownership
locator entries, unknown required fields and noncanonical decimal offsets.
Additional direct byte tests cover missing locator allocation, changed frame
bytes/reserved flags, malformed/unknown-tag/truncated padding, duplicate JSON
keys, noncanonical whitespace, incompatible readers and independent recovery.

```sh
python3 docs/architecture/proposals/ps2/scalable/generate.py
cargo test -p photara-store --test scalable_wire_candidate
cargo clippy -p photara-store --test scalable_wire_candidate -- -D warnings
```

The [checkpoint evidence](verification/ps2-scalable-linked-byte-candidate-20260927.json)
records source hashes, independent reproduction and passing test/Clippy output.
The current eight tests and strict Clippy pass. Candidate byte stability is
reproducibility evidence, not a compatibility freeze. Real operation receipt
indexes, scalable inventory branches, scalable owned charge trees, pins/resources,
publication/recovery mutation and qualified local profiles remain separate
implementation and review work.
