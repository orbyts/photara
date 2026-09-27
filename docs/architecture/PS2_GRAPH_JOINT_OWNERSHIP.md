# PS2 same-tip Graph publication with exact ownership and attributable settlement

Status: disposable engineering evidence, 2026-09-27. No production code, permanent
wire, live package, qualified storage profile, or production Accepted/Saved
acknowledgement is established.

## Integrated behavior

`ps2_index_furnace placement-v3 graph-journal joint` composes genuine Core Graph
commands and exact typed data/metadata ownership into one original-token packed
publication. The [previous Graph path](PS2_GRAPH_ORIGINAL_TOKEN_PACKED.md) remains
available without `joint`; its historical measurements and full-hold settlement
are unchanged. This report measures the new same-generation path.

A bounded source preview and a shared semantic-delta planner feed one joint
physical plan. The planner merges semantic locator changes with the two current
tip ownership updates. Both resulting active and recovery ownership trees cover
the newly generated data and metadata suffixes; recovery retains the previous
active Graph state. At most sixteen in-memory fixed-point iterations resolve the
claim extents, including the bytes storing those claims and their locator pages.
The original prefix digest and content edge remain distinct from the larger
owned allocation extent. Existing ownership claims are verified before their
prefix commitment is extended, so a corrupted prefix cannot be blessed.

The original liability precedes journal creation. It binds canonical group
bytes, original source identity/high-water, original data/metadata identity and
prefixes, the complete control selector, and the joint continuation. Partial
journal/source/data/metadata replay uses the existing original-token recipe.
No new ordinal or OperationId is allocated by retry. Genuine no-op and inverse
receipts, final authored state, both selected tip claims and the regenerated
original plan are checked before settlement. The packed HEAD is authoritative;
the source's own HEAD remains a disposable builder/oracle artifact.

The active tips carry no guessed self-referential allocation charge in their
claim payload. Instead, the selected ledger owns each tip's observed allocation
high-water. Settlement observes nonnegative tip growth and retained source growth,
consumes the same original hold, and selects the resulting ledger exactly once.
Package controls have a separate standing reservation; the source registry has
its own standing scratch reservation. Source staging is retained and never earns
retirement credit. Neither the reservation nor observed allocation is an
exclusive filesystem reservation or a free-space guarantee.

The final settlement selector is also recoverable. Its old intent preserves the
original Graph hold after the selected candidate consumes it. Recovery derives
the exact permitted published-old and ledger-completion selectors from that
hold, rechecks the original plan, state, receipts and ownership, then reconciles.
An altered charge or unrelated gate field preserves the evidence. Reopening and
finishing again leaves the settled selector unchanged.

## Verification and boundaries

The combined regression passed 90 index/placement tests and seven original Graph
furnace tests. Strict Clippy and formatting checks passed. Six joint-focused
tests cover:

- twenty-one journal/source/packed/publication/cleanup return cuts, for each map;
- two additional cuts before and after the final settlement HEAD replacement,
  each followed by repeated fresh-handle completion for both maps;
- same-token candidate ledger mutation and changed final-settlement charge;
- corrupted old owned prefix refusal;
- no-op and changed groups with exactly-once source/tip high-water settlement;
- data or metadata rollover refusal before a hold, journal, source or pack write,
  independently forced for both maps.

The final settlement tests also retain original receipt identity and validate
both closures after recovery. These are twenty-three named controlled return
cuts, not every syscall interruption, abrupt kill, remount or power-loss tests.
The two final cuts are a separate test from the original twenty-one-point matrix.

Full Graph replay, complete physical closure auditing and source auditing run
independently after measured clean completion. The completion path uses bounded
original-plan and selected-tip checks, not a claimed production lifetime full
audit. Interrupted fixture reconciliation may itself use the existing exhaustive
fixture audit. No ordinary external-media access is introduced.

Fresh-generation enrollment is the deliberate next boundary: this planner
refuses rollover without effects. It cannot yet continue a durable Graph group
by creating and enrolling a new pack under its original token. The separate
sealed-pack retirement ledger tests do not prove a full Graph growth, seal,
retire and credit lifecycle. All-class pin and imported-read-only fixtures remain
independent regression coverage, not a new Graph-specific complete lifecycle test.
Owner fencing, concurrent writers, asynchronous sessions, qualified durability,
production reader/writer implementation and permanent wire review remain open.

## Bounded release observations

The raw evidence contains baselines of 8 and 32 for both maps, followed by groups of 1, 8 and 32.
Baselines, attributable bootstrap and initial source enrollment are outside the
measured groups. The workload remains the fixed sixteen-node position/no-op/
inverse/two-node Batch sequence. This is same-tip engineering smoke evidence,
not a scaling comparison with the earlier baselines of 64 and 128 legacy run.

All write columns are successful process bytes, not device traffic. Journal
includes group/candidate/marker; source includes its retained objects and HEAD;
packed includes data, metadata and admission/publication/settlement controls.
The last two columns are separately observed, nonnegative allocation high-water
increases; their sum is the selected project charge increase. Allocation
rounding/preallocation makes these different from logical or process bytes.

| Baseline | Map | Group | Journal writes B | Source writes B | Packed writes B | Tip charge growth B | Source charge growth B |
|---:|---|---:|---:|---:|---:|---:|---:|
| 8 | Radix | 1 | 13,935 | 12,335 | 143,317 | 20,480 | 12,288 |
| 8 | Radix | 8 | 27,765 | 68,541 | 258,462 | 45,056 | 69,632 |
| 8 | Radix | 32 | 75,149 | 258,705 | 646,745 | 126,976 | 258,048 |
| 8 | Btree | 1 | 13,935 | 12,335 | 143,317 | 20,480 | 12,288 |
| 8 | Btree | 8 | 27,765 | 68,608 | 258,529 | 45,056 | 69,632 |
| 8 | Btree | 32 | 75,149 | 262,695 | 646,130 | 126,976 | 262,144 |
| 32 | Radix | 1 | 13,978 | 15,175 | 149,551 | 73,728 | 65,536 |
| 32 | Radix | 8 | 27,812 | 64,094 | 268,627 | 16,384 | 65,536 |
| 32 | Radix | 32 | 75,168 | 277,614 | 661,475 | 135,168 | 225,280 |
| 32 | Btree | 1 | 13,978 | 15,086 | 148,892 | 73,728 | 65,536 |
| 32 | Btree | 8 | 27,812 | 66,464 | 267,232 | 16,384 | 65,536 |
| 32 | Btree | 32 | 75,168 | 278,303 | 657,111 | 126,976 | 229,376 |

The standing package-control charge is 1,163,264 B. The separate standing source
scratch charge is 1,114,112 B. Both remain unchanged across all measured groups.
The source registry additionally carries its actual retained pack high-water;
that grows independently of the two package tip charges. Original group holds
are 19,365,888 / 19,431,424 / 19,628,032 B for group sizes 1/8/32. These deliberately
conservative admission bounds retain the earlier 16 MiB source work allowance;
settlement now consumes only the observed source/tip growth under that hold.
No source or filesystem credit results from unused admission headroom.

Each clean group performs 43 sync calls: 28 packed, 4 source and 11 journal. Group 32
writes 30,644–31,696 total process bytes per operation, including all three paths.
Measured single-observation planning/admission/completion sums are 797–1,042 ms.
The capture overlapped other verification; these values are not an isolated
latency comparison, distribution or qualified Saved latency. Group formation,
UI scheduling, baseline initialization and final exhaustive audits are excluded.
Large conservative holds, repeated original-hold control copies, bounded prefix
verification and 43 barriers remain negative practical evidence. Neither map is
selected for a permanent format by this run.

## Reproduction and provenance

```sh
cargo test --release -p photara-store --example ps2_index_furnace joint_graph
cargo test --release -p photara-store --example ps2_index_furnace joint_owned_graph
cargo run --release -p photara-store --example ps2_index_furnace -- placement-v3 graph-journal joint 8 32 > docs/architecture/verification/ps2-graph-joint-owned.jsonl
```

[Raw evidence](verification/ps2-graph-joint-owned.jsonl), SHA-256 `95352460ed7db5ee06d5418fbe570203c377288c77c54d7b25c0aa9848fa0a76`.

Source hashes at capture, relative to `crates/photara-store/examples/`:

| Source | SHA-256 |
|---|---|
| `ps2_index_furnace/placement_v3.rs` | `3f633d70afa0e3761460ee26bda9f6b50d1959dbd6313fadc0da5a71ee40c68f` |
| `ps2_index_furnace/placement_v3/graph_journal.rs` | `fb62e3b7b31178c5c6e0a23591356b41e32ec23fd900b3c769ea44644d5a1be4` |
| `ps2_index_furnace/placement_v3/typed_inventory/graph.rs` | `8701bc7f72050be480ee0cda6a3eaadd7da6711ee5486a654804466eed9b3f5f` |
| `ps2_index_furnace/placement_v3/typed_inventory/retirement_ledger.rs` | `64a59c539b0b3c768393c151f61e8fbf6bea96ab8ce3c0f282ffec2910b11c53` |
| `ps2_index_furnace/placement_v3/typed_inventory/recipe.rs` | `f3662f45c2504974c50c93d4c90fdd8694e91482468a21d26683cb7f52a3286e` |
| `ps2_index_furnace/placement_v3/typed_inventory/recipe/accounting.rs` | `7a5e803a8c5b352a6911dacd0fdb011e9461cee516a815e683318573a2aae1b0` |
