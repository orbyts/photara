# PS2 actual-v3 ownership closure gate

Disposable engineering evidence, 2026-09-18; continuation from `9e1c6c0`.
The full refundable ownership adapter is **not integrated**. This slice runs
the actual v3 placement/checkpoint implementation, compares two ownership
accounting/placement models, and demonstrates the precise contract gate before
proceeding. It does not change the existing audit invariant, authoritative
selector, permanent wire, production writer, live packages, or Accepted/Saved
qualification. Graph/journal integration remains deferred.

## Actual boundary exposed

The existing locator is not a generic bag of physical objects: `audit_root`
requires its leaves to equal the selected semantic closure exactly, with no
extraneous or duplicate leaves. Thus simply inserting reserved-key ownership
descriptors into the same locator is rejected by correct existing behavior.

The new controlled test builds real ownership bodies and real locator pages,
reserves their modeled physical work, persists the exact final roots/extents in
the original v3 continuation, writes/barriers the candidate, and interrupts
before HEAD replacement. On fresh-handle recovery, both the intact descriptor
and a deliberately corrupt descriptor case refuse with
`extraneous or duplicate locator leaf`. The old selection, original liability
and intent remain. **This is not a discovered defect in approved v3 behavior**:
the attempted additional inventory category is outside its current contract.
The fixture does not bypass or weaken that check to claim integration.

This is now an explicit closure/format review gate. Two directions preserve one
HEAD authority but have different proof obligations:

| Direction | Benefit | Required contract and engineering |
|---|---|---|
| Typed semantic + ownership inventory in the existing locator | Reuses path-copying, physical relocation and pack compaction | Typed membership/namespace; independent semantic and physical ownership closure; exact coverage and corruption validation in audit and recovery; ownership roots must not change authored identity |
| Separate persistent ownership root selected by the same HEAD | Leaves exact semantic-locator closure unchanged | Another root, not another selector; bounded ownership-page placement/reclamation; active/recovery/all-pin retention; bootstrap/metadata self-accounting; joint reserve and continuation coverage |

Neither is selected here. A flat lifetime ownership map in HEAD is rejected as
a scaling shortcut. Fixed per-pack upfront charging changes admission policy,
not the need for authenticated exact-generation retirement evidence.

## Bounded metadata closure planning

The probe uses actual v3 `multi`, encoded locator pages and `WritePlan`, not the
earlier synthetic role/ordinal adapter. It plans both active and recovery root
paths and shared metadata pages. A descriptor insertion can close a metadata
pack, which in turn needs its own sealed ownership descriptor. A deterministic
near-full-tail case proves that a single insertion pass misses this obligation.

A no-effect fixed-point planner adds the newly closed pack descriptors and
replans until ownership extents/layout stabilize. It caps work at eight rounds
and 64 descriptors. The forced boundary test requires at least two rounds,
includes the extra metadata pack in the exact continuation, and includes all
descriptor/locator bytes in the modeled physical bound. A one-round budget
refuses without writes or selection changes.

The bounded experiment refuses if one arena would create **and seal another
new pack** in the same plan: that pack lacks a pre-effect inode witness. This
is an explicit incomplete case, not an inferred physical identity. A broader
implementation must model allocation-generation witnesses and original
continuation coverage, or split admission before effects. For closed packs
added by the feedback planner, charge uses the full 256 KiB pack cap, not a
claim that future physical allocation has been measured. Active tip ownership
in HEAD, exact refunds, namespace/control-generation ownership and authoritative
per-domain ledger updates are not implemented by the probe.

## Comparison method and limits

The ordinary benchmark executes actual v3 checkpoints at 1,000 and 10,000
fully materialized original receipts for both logical maps. One group each of
1, 8 and 32 further operations is selected; these are single observations,
not latency distributions. The source builder is bulk-built and its I/O is
outside checkpoint counters. The physical fixtures use existing packed arenas,
locator cache, all-class gate and v3 original liabilities unchanged.

An **in-memory observation model**, never an authoritative durable ledger,
records device/inode, selected extent and monotonically observed
`max(st_blocks * 512, length)` per actual data/metadata pack. Bootstrap scans
all packs; subsequent updates inspect only the bounded touched range. It
compares this charge with charging 256 KiB at each new pack's creation.
Neither includes the fixed control allowance or directory/APFS internals.
Neither creates real free-space reservation or filesystem availability credit.

No-write descriptor previews are alternatives, not extra measured writes:
incremental descriptors for touched units versus immutable sealed descriptors
with active tips proposed for HEAD. They use real encoding and locator paths,
but are not accumulated into a persistent ownership index over successive
benchmark groups. Their bytes exclude new ownership control/HEAD changes and
are **not complete integrated write amplification**. The bounded closure
estimate is also reported separately. Reserved high-bit keys are experimental
probe inputs, not a proposed permanent namespace or semantic ID allocation.

## Measured results

After the three groups, pack-only ownership comparison is:

| Starting operations | Map | Observed incremental pack charge | Fixed pack-capacity charge | Difference |
|---:|---|---:|---:|---:|
| 1,000 | Radix | 1,642,496 B | 2,097,152 B | 454,656 B |
| 1,000 | B-tree | 1,548,288 B | 1,835,008 B | 286,720 B |
| 10,000 | Radix | 13,721,600 B | 14,155,776 B | 434,176 B |
| 10,000 | B-tree | 12,832,768 B | 13,107,200 B | 274,432 B |

Fixed charging avoids per-prefix charge growth within already owned packs but
requires another 262,144 B per new pack before effects. At 10k, group 32 creates
two packs for either map: fixed new liability is 524,288 B, versus observed
pack growth of 405,504 B radix / 385,024 B B-tree. These are not guaranteed
allocator bounds. Existing v3's separate high-water charged totals still grow
to 15,106,048 / 14,184,448 B respectively; the probe does not replace them or
claim it has refunded physical retirement.

Actual v3 checkpoint measurements (process bytes include data, metadata and
control envelopes):

| N | Map | Groups 1 / 8 / 32: process bytes | Groups 1 / 8 / 32: latency ms |
|---:|---|---|---|
| 1k | Radix | 38,634 / 100,061 / 231,972 | 94.2 / 104.9 / 150.7 |
| 1k | B-tree | 35,590 / 102,493 / 217,113 | 90.0 / 99.1 / 109.9 |
| 10k | Radix | 47,089 / 166,387 / 418,623 | 93.1 / 105.1 / 138.9 |
| 10k | B-tree | 43,492 / 140,218 / 374,350 | 150.0 / 111.0 / 119.8 |

There are 26–28 sync calls/group. These are unoptimized actual v3 costs, not
an application latency endorsement. B-tree writes fewer bytes for all three
10k groups, but one-shot timings do not justify a permanent map selection.
No barriers were removed to improve the result artificially.

At 10k the incremental descriptor + both-locator-root preview adds about
2.5–5.1 KiB encoded bytes/group; sealed-only previews add 0–2.5 KiB, excluding
active-tip HEAD cost. Ordinary cases converge in one closure round; forced
near-tail tests expose the extra round and descriptor. The contrast must not
be read as a proven durable integration saving.

Fresh-reader structural open plus probe takes 1.18–1.82 ms across four cases;
oldest-operation retry takes 0.97–2.02 ms. These are OS-cached files, not
disk-cold. Raw read counters are retained. Observed net regular-file allocation
deltas are separate from process bytes; snapshot sampling excludes filesystem
metadata, snapshots, device writes and unobserved peaks. The command takes
10.77 s wall including 5.29 s compilation; maximum reported RSS is 27,377,664 B.

## Verification and artifacts

```sh
cargo test --release -p photara-store --example ps2_index_furnace ownership_probe
cargo test --release -p photara-store --example ps2_index_furnace
cargo clippy --release -p photara-store --example ps2_index_furnace --tests -- -D warnings
rustfmt --edition 2024 --check crates/photara-store/examples/ps2_index_furnace/placement_v3.rs
/usr/bin/time -l cargo run --release -p photara-store --example ps2_index_furnace -- placement-v3 ownership-probe 1000 10000 > /private/tmp/ps2-v3-ownership-probe-final.jsonl 2> /private/tmp/ps2-v3-ownership-probe-final.log
```

Six new tests pass; the complete furnace suite passes **40 tests in 38.08 s**.
Strict Clippy and formatting pass. The six cover no-effect real-page estimates,
metadata self-allocation, bounded closure/continuation reserve coverage,
no-effect closure exhaustion, observer charge idempotency, and real pre-HEAD
recovery refusal preserving the original liability for both intact/corrupt
experimental ownership records. They do not prove a functioning ownership
inventory, arbitrary mid-record recovery, actual kernel ENOSPC, power loss,
qualified free-space probes or reader leases.

Only code changes are the new child
`examples/ps2_index_furnace/placement_v3/ownership_probe.rs` and six lines of
module/CLI dispatch in the actual v3 file. Existing audit, reserve, publication,
compaction and recovery functions are unchanged. No earlier raw evidence is
rewritten. This report and its raw JSONL are new.

- [Raw four-row output](verification/ps2-v3-ownership-placement-probe.jsonl), 41,527 B, SHA-256 `ff3cf152b326b1b5e3a727432a9e57f579a26a6047361ea47797afbdfd113d69`.
- v3 source SHA-256 `e603dca341794dcc174ae5fe33b7f507c95c21b873b09dc62e2f0d334e5f77a9`.
- New child source SHA-256 `b7038dcbdfe7a8fbdb3eea8f5a8dd2d6e127cb9b82222fb45e7560e1812d8ad8`.
- Test log `/private/tmp/ps2-v3-ownership-tests.log`, SHA-256 `ec062e51f50f2cec1f8f048a5a45fbecf438003fa254ef935694066c293b492b`.

## Required next decision

Review the physical ownership/inventory closure contract: typed membership in
the existing locator, or a distinct persistent ownership root under the same
HEAD. Both need exact retirement witnesses, active/recovery/pin preservation,
bounded metadata self-accounting, control-generation ownership, and original
hold/continuation coverage. Decide whether the fixed sealed-pack capacity
trade-off is acceptable separately from root layout. Do not freeze permanent
wire or claim the Graph + refundable v3 backend is complete from this probe.
