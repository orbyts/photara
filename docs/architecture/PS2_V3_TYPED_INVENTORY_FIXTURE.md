# PS2 actual-v3 typed inventory fixture

Disposable engineering evidence, 2026-09-18; implementation starts from
`46c9992`. This implements the approved direction—typed semantic and ownership
inventory in the existing HEAD-selected locator—without selecting permanent
wire. No production reader/writer, live packages, migration, GC, Asset Store,
project switching, real Graph/journal integration or qualified Accepted/Saved
behavior is added.

## Executable contract

Every physical locator location now declares `Semantic` or `Ownership`
membership. Disjoint logical-key ranges are checked by typed readers. The
high-bit ownership namespace and JSON shapes are disposable fixture choices,
not proposed permanent ID allocation or codec bytes.

Semantic traversal still starts at the original selected semantic root and
must cover exactly its semantic locator leaves. Ownership traversal starts at
a separate **logical commitment inside the same selected locator** and must
cover exactly its ownership leaves. Their union must equal the complete
locator: untyped extras, missing records, duplicate/extraneous leaves, wrong
membership and content corruption refuse. Ownership records cannot silently
become authored nodes or satisfy a semantic reference.

Ownership commitments form a persistent radix tree of bounded immutable nodes,
not a lifetime list in HEAD. Leaves describe selected allocation claims by
arena/pack key, device/inode, prefix extent/hash and sealed state. Branches
reference immutable ownership nodes. Path copying writes changed nodes and
locator paths; existing history is shared. Allocation-key duplicates, malformed
branch ordering, routing violations and duplicate/cyclic node references are
checked during explicit audit. A claim validates its bounded internal pack
prefix and exact local generation; it is not a portable external-media record.

HEAD carries active/recovery ownership commitments and a logical object counter.
The same commitments travel with all ten extra pin classes and original
continuations. They introduce no second physical selector or independent
ownership object store: all ownership bodies and tree nodes are ordinary
typed objects in the actual v3 packed arenas/locator.

Physical-only ownership publication and metadata relocation preserve the exact
authored semantic HEAD. Ordinary semantic checkpoints preserve the ownership
commitment, including recovery. A selected or pinned ownership claim
conservatively blocks retiring its exact arena/pack. This is retention safety,
**not an implemented claim-release/refund policy**.

## Reader, audit and recovery assurance

Ordinary structural probing reads the selected ownership root node in addition
to existing semantic headers. It does not enumerate the ownership tree or
hash all claimed packs. Accessed ownership paths validate namespace, membership,
content hash, physical prefix bounds and branch order. Explicit full audit
traverses both independent closures and verifies every claimed internal prefix.
No routine external-media reads or hashes are introduced.

The original admitted continuation contains the final active/recovery locator
roots, ownership commitments, object counter and arena extents. Before-HEAD
and after-HEAD cut tests reopen and complete through the same original token,
without changing semantic identity. Candidate ownership objects are audited
before continuation publication; a corrupt owner payload fails its digest and
retains the original hold and intent. A discovered planned-counter bug was
fixed: recovery validates newly planned ownership IDs against the counter in
the original continuation, not only the old selected counter.

The explicit fixture imported/read-only mode permits structural inspection but
blocks publication until successful full combined audit. Local reopen retains
the existing fixture assumption of a trusted owner. This flag is not a native
provenance, cross-process owner-lease or secure import implementation.

## What remains intentionally incomplete

This slice validates **selected ownership claims**, not a complete allocation
ledger. It does not assert that every live data, locator and control allocation
has a claim, nor add complete charge growth, refund, control-generation ownership
or filesystem qualification. The prior v3 high-water capacity accounting is
unchanged. Claim verification may read up to one 256 KiB internal pack prefix;
there are no external media reads.

The benchmarks select two claims (data pack 0 and metadata pack 0). They do not
measure a large ownership population, complete metadata self-accounting or
ownership claim deletion. In particular, the bounded metadata fixed-point
planner from the earlier probe is **not** integrated with an all-pack ownership
publication. Before-effect allocation preparation and arbitrary partial-record
failure recovery are not established by these typed-publication cuts; the
tested cuts occur after complete candidate payload writes. Existing reserve,
physical allocation assumptions, qualified flush and reader-proof limitations
remain unchanged. No production Saved or OS reservation guarantee follows.

Next connect automatic active-prefix/sealed ownership coverage, exact retirement
witnesses and refundable per-domain accounting to these typed roots and
continuations, including locator/control self-allocation. Then integrate the
genuine Core Graph journal/receipt path. Neither is completed here.

## Actual-v3 measurements

Both map baselines fully materialize 1,000/10,000 original operations. Each
installs two typed claims, then publishes one group each of 1, 8 and 32
additional operations using the actual v3 checkpoint path. The source builder
is bulk-built; its I/O is outside checkpoint counters. These are single samples,
not p50/p95 distributions. Logical operation/receipt bodies remain the v3
synthetic model, not genuine Core Graph commands.

| N | Map | Groups 1 / 8 / 32: process bytes | Groups 1 / 8 / 32: latency ms |
|---:|---|---|---|
| 1k | Radix | 44,961 / 107,296 / 240,842 | 141.7 / 127.0 / 119.1 |
| 1k | B-tree | 41,902 / 109,464 / 225,264 | 122.5 / 148.5 / 128.3 |
| 10k | Radix | 53,355 / 174,297 / 429,969 | 113.9 / 134.5 / 169.3 |
| 10k | B-tree | 49,812 / 147,450 / 383,686 | 110.4 / 119.6 / 124.9 |

Process bytes include data, locator metadata and control envelopes. Each group
uses 26–28 file/directory sync calls. At 10k, net observed regular-file allocated
deltas for groups 1/8/32 are 94,208/98,304/413,696 B radix and
32,768/176,128/323,584 B B-tree. APFS allocation is not proportional to encoded
bytes at every checkpoint. These `st_blocks` snapshots exclude directories,
internal filesystem metadata, snapshots, device write amplification and
unobserved peaks.

Each physical-only claim publication writes about 16.7–21.8 KiB process bytes,
allocates a net 4 KiB in this run, takes 90–134 ms and uses 26 sync calls. The
fixture retains conservative, explicit publication barriers; no barrier was
removed to improve timing. These costs are negative practical evidence for a
future batching/flush pass, not a snappy application implementation.

Fresh-reader structural open/probe takes 1.41–1.83 ms over all four cases,
reading 12 data objects and 44–50 locator pages; HEAD bytes are approximately
1.9 KiB including the repeated HEAD read. Oldest-operation retry takes
0.95–1.40 ms. Files are OS-cached, not disk-cold. Explicit audit takes
0.248–0.249 s at 1k versus 2.18–2.39 s at 10k, covering 2,381–25,070 objects
across independently selected active/recovery roots. That exhaustive cost is
not charged to ordinary open.

B-tree writes fewer bytes for all measured 10k groups; this small typed slice
does not select a permanent map. The isolated command takes 11.81 s wall with
27,312,128 B reported maximum RSS. An earlier run overlapping the full test
suite took 46.25 s and is retained separately, not used for the table.

## Verification and provenance

```sh
cargo test --release -p photara-store --example ps2_index_furnace typed_inventory
cargo test --release -p photara-store --example ps2_index_furnace
cargo clippy --release -p photara-store --example ps2_index_furnace --tests -- -D warnings
rustfmt --edition 2024 --check crates/photara-store/examples/ps2_index_furnace/placement_v3.rs
/usr/bin/time -l cargo run --release -p photara-store --example ps2_index_furnace -- placement-v3 typed-inventory 1000 10000 > /private/tmp/ps2-typed-inventory-isolated.jsonl 2> /private/tmp/ps2-typed-inventory-isolated.log
```

Six new typed tests pass; the full furnace suite passes **46 tests in 44.31 s**.
Strict Clippy and formatting pass. Tests cover typed claims across checkpoint
and reopen; before/after-HEAD same-hold recovery; content corruption retaining
evidence; forged/missing/extra typed locator leaves; all ten pin classes and
actual metadata compaction preserving semantic/ownership identities; and
explicit imported-read-only refusal before full audit. Faults are controlled
errors and disposable-file corruption, not real power-loss/ENOSPC injection.

Changes are limited to actual-v3 fixture hooks, new child
`placement_v3/typed_inventory.rs`, two compatibility field initializers in the
previous ownership probe, and this new report/raw artifact. The prior probe
still labels its experimental ownership leaves Semantic and its negative
recovery test continues to refuse them. Previous raw results remain untouched;
disposable fixture encodings changed, not production wire/golden bytes.

- [Isolated raw four-row output](verification/ps2-v3-typed-inventory.jsonl), 21,334 B, SHA-256 `85e266a2fcaf74c6fc06e8c7d32c5a92cbdb4e85554d148c4687880776d2e181`.
- v3 source SHA-256 `22db3da440d3891637c992328c2b69c6d81fb686d3cf200b55d565e803d7c22e`.
- Typed child source SHA-256 `5537ed6926d3f3d86c65fdef3bd9a3207a022e1f8df314c578980164988a0c9a`.
- Probe compatibility source SHA-256 `2b363485258fd4e99c7766facf11565e8f8506e9cd4bed6e02eff6f1cd495ed4`.
- Test log `/private/tmp/ps2-typed-inventory-tests.log`, SHA-256 `89de367d412c113d924429564d2e7838ebc83e88821127108e91ac5a56c3923e`.
- Earlier contended stdout `/private/tmp/ps2-typed-inventory-final.jsonl`, SHA-256 `61119124bf2d04b7a4ec21f9a3f8d951ece93137a61919db650b0c04e6d94032`.
