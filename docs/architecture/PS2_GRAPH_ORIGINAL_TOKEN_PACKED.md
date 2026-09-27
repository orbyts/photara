# PS2 genuine Graph journal on the original-token packed path

Status: disposable engineering evidence, 2026-09-27. No permanent wire,
production reader/writer, live package, qualified storage profile, or production
`Accepted`/`Saved` acknowledgement is established here.

## Integrated behavior

`ps2_index_furnace placement-v3 graph-journal` now uses the same Core Graph
command model as the earlier `ps2_graph_furnace`. The model was extracted into
one module; it was not replaced with a second synthetic command implementation.
Original typed requests, before/after coordinates, no-op outcomes, inverse
annotations, original OperationIds, request digests and dense ordinals are
stored in the existing logical receipts. The selected authored leaf carries the
actual Graph state. Optional fixture fields are omitted for the older synthetic
workloads, preserving their previous encoded object bytes.

The Graph path exercises the actual-v3 bounded data/metadata packs, locator
publication, independently retained active/recovery states, original liability
and original-token partial-record replay. Both radix and B-tree remain runnable.

A group contains at most 32 operations. Before effects, a bounded in-memory
source preview derives the exact semantic candidate and packed write plan.
Individual source objects retain the 64 KiB cap, source preview memory is capped
at 8 MiB, and original canonical journal bytes are capped at 128 KiB. Physical
replay inherits the finite recipe arena-span bound. A refusal during planning or
capacity admission writes neither the journal nor semantic/packed payloads.

The durable original liability precedes journal creation. It binds the complete
canonical group bytes, source-tail digest and length, original data/metadata
inode and prefix commitments, exact semantic target and packed continuation,
and the pre-effect control selector. Journal/source/packed work shares the same
modeled capacity domain; the disposable source's initial allocation is included
at bootstrap. Reopen derives the same plan and checks its continuation against
the original hold before replay. The shared recipe implementation verifies
matching partial frames/payloads and writes only missing suffix bytes. The
shared control-generation checker derives the exact permitted old/candidate
selectors, including pins, domains, charges and token sequence. Matching roots
and an original token alone are insufficient.

Only after the journal file and directory barriers does the path persist source
checkpoint objects and replay packed data/metadata. It then selects the packed
checkpoint and verifies every original receipt plus the final authored state.
The checkpoint marker binds the exact prefix, ordinal, authored coordinate and
semantic selection. Cleanup can resume after each journal/candidate/marker
unlink. The same original hold remains through cleanup and is settled only
after exact original receipts and state are verified. A missing or partial
journal can be reconstructed from the original hold. A conflicting journal,
source tail, physical suffix or control selector preserves the unresolved hold
and evidence.

The no-op test preserves the authored coordinate while extending the accepted
operation prefix. This tests receipt continuity; it does not change the approved
production UI Saved policy. The inverse remains an ordinary new command/revision,
not revision rollback or a proposed Undo wire. The workload remains the earlier
fixed 16-node position/no-op/inverse/two-node Batch workload; there are no external
media reads by construction.

## Verification scope

The Graph integration adds seven focused tests, including 21 interruption points
for **each** map:

- after original hold admission and before journal creation;
- partial journal, complete journal bytes, file barrier, directory barrier;
- partial source tail and complete source candidate;
- before packed effects, offsets 1/12/13/4096 in packed data, after data, and one
  byte into metadata;
- before/after packed HEAD replacement;
- checkpoint marker, journal unlink, journal cleanup barrier, candidate unlink,
  and marker unlink.

Fresh handles preserve the original token, replay its matching remaining bytes,
return every original receipt without a new ordinal, and audit active/recovery
Graph replay. The other tests cover no-op coordinate continuity, malformed
requests/duplicates before effects, insufficient capacity with no effects,
conflicting canonical journal replacement with matching partial-journal
recovery, altered domain limits under the same original token/roots, and
conflicting partial packed payloads. The original seven Graph-furnace tests
retain the independent PS1 planner check. Full-index auditing now also replays
real Graph receipts when a typed authored state is present.

These are controlled return/error cuts with fresh handles, not abrupt process
kill, remount or power-loss qualification of this integrated Graph path. The
separate macOS probe evidence does not qualify this path by association.

The final [combined regression](verification/ps2-integrated-regression-20260927.json)
passed all 76 index/placement tests and all seven original Graph-furnace tests.
Strict Clippy passed for both example test targets.

## Bounded release observations

macOS 27.0 build 26A428, arm64, Rust 1.95.0. Baselines of 64 and 128 operations
are bulk-built and copied before the measured groups. Each row below is **one**
clean group, in group-size order 1, 8, 32. This is engineering smoke evidence,
not scaling or latency-distribution evidence. All bytes are successful process
writes, not device traffic.

The journal column includes the group, candidate and marker files. Source
includes the disposable semantic staging pack and source HEAD writes. Packed
includes data, metadata and all admission/publication/settlement control writes,
including copies of the original journal bytes inside the held control record.
Thus the comparison does not hide source staging or journal costs.

| Baseline | Map | Group | Journal B | Source B | Packed B | Total B/op |
|---:|---|---:|---:|---:|---:|---:|
| 64 | Radix | 1 | 14,007 | 14,101 | 129,531 | 157,639 |
| 64 | B-tree | 1 | 14,007 | 15,662 | 128,876 | 158,545 |
| 64 | Radix | 8 | 27,837 | 70,652 | 255,535 | 44,253 |
| 64 | B-tree | 8 | 27,837 | 75,233 | 256,862 | 44,992 |
| 64 | Radix | 32 | 75,218 | 287,271 | 654,886 | 31,793 |
| 64 | B-tree | 32 | 75,218 | 295,906 | 648,761 | 31,871 |
| 128 | Radix | 1 | 14,092 | 14,254 | 131,533 | 159,879 |
| 128 | B-tree | 1 | 14,092 | 16,814 | 131,583 | 162,489 |
| 128 | Radix | 8 | 27,993 | 79,574 | 262,317 | 46,236 |
| 128 | B-tree | 8 | 27,993 | 84,602 | 260,136 | 46,591 |
| 128 | Radix | 32 | 75,589 | 301,131 | 672,436 | 32,786 |
| 128 | B-tree | 32 | 75,589 | 349,991 | 668,919 | 34,203 |

Each clean group performs 43 sync calls for sizes 1/8, or 44 for size 32. The
journal contributes 11 calls on this exact clean control-flow path, source
contributes four, and packed counters measure the remainder. Single-observation
elapsed planning/admission/checkpoint totals are 128.6–150.5 ms for group 1,
174.9–189.9 ms for group 8, and 288.6–295.3 ms for group 32. Group-formation wait,
UI scheduling/rendering, baseline construction and final exhaustive audit are
outside those spans. This is not qualified Saved latency.

Original holds are 19,365,888 B / 19,415,040 B / 19,906,560 B for groups 1/8/32.
This path deliberately settles the **full** conservative admitted high-water
amount, including a 16 MiB source-staging allowance. It supplies no deletion
credit, no exclusive OS reservation and no qualified filesystem-capacity bound.
The large hold, repeated control copies and 43–44 barriers are negative
practical evidence. They do not justify a production format or an index winner.
Both maps reach identical Graph prefix commitments at 105 and 169 operations;
full physical replay also verifies the separately retained 73/137-operation
recovery states and oldest-operation retry.

## Remaining integration gates

- Automatic exact ownership/self-coverage is still a separate publication.
  This Graph path does not yet put every new payload, locator/control generation,
  source/journal extent and original token into one exact ownership closure.
- The measured allocation settlement helper exists in the typed ownership
  fixture, but Graph currently uses conservative full-hold settlement. Refundable
  project charge, rollover ownership and retirement remain later integration.
- Generic all-class pins and liveness remain covered by the shared v3 regression
  suite. This bounded Graph run does not remeasure every pin/retirement class on
  Graph-specific history.
- The source staging pack is a disposable oracle/builder. Its disk writes are
  counted and admitted here; they do not select a production staging architecture.
- Owner fencing, concurrent writers, async session scheduling, qualified platform
  barriers, larger/mixed Graphs, permanent wire review and PS3/PS4 are not supplied
  by this fixture. No live package or production code is changed.

## Reproduction and provenance

```sh
cargo test --release -p photara-store --example ps2_index_furnace
cargo test --release -p photara-store --example ps2_graph_furnace
cargo clippy --release -p photara-store --example ps2_index_furnace --example ps2_graph_furnace --tests -- -D warnings
cargo run --release -p photara-store --example ps2_index_furnace -- placement-v3 graph-journal 64 128 > docs/architecture/verification/ps2-graph-original-token-packed.jsonl
```

The [five-row raw evidence](verification/ps2-graph-original-token-packed.jsonl)
has SHA-256 `3bd6e644c69e8ce00d92048d6b2d7c278a3d575200807c73562e86ed09e45e7e`.
Current Graph integration source hashes at this evidence capture, relative to
`crates/photara-store/examples/`:

| Source | SHA-256 |
|---|---|
| `ps2_graph_furnace/model.rs` | `2e533667aac36ba8dc23ae51930ade8bfe3c80e9a471b838df095fb3a2de3cb1` |
| `ps2_index_furnace.rs` | `e209b4ad8478c1b8fbda5784493be02c12bad03c495365f7937b8c3833bfbfd8` |
| `ps2_index_furnace/graph_preview.rs` | `b04002079309b4ce7c3bfd01df55f6180239ac0ba22d0bc158161a800f8a28cc` |
| `ps2_index_furnace/placement_v3.rs` | `eb088a6657b7daf1f7d536ae175e00f62e3d0e14e12d2fba35045e1b88c69b50` |
| `ps2_index_furnace/placement_v3/graph_journal.rs` | `af1aab23ae4c8dc2be1ea7edb421777c5e6ca189357abd026e96f5614b591016` |
| `ps2_index_furnace/placement_v3/typed_inventory/recipe.rs` | `fdf6b22a666966b09bcf1c1a295a86fefb6a44c96eb44894cc75712fb102dfe8` |
| `ps2_index_furnace/placement_v3/typed_inventory/recipe/accounting.rs` | `6f4624c03a894b3b7a1352408cc596297c664523bd55a8fa9d87b385934cb174` |

The earlier [Graph journal report](PS2_GRAPH_JOURNAL_CHECKPOINT_FURNACE.md)
retains its historical independent measurements; this new run does not replace
or extrapolate the earlier 1k/10k/100k evidence.
