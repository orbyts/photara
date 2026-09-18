# PS2 disposable Graph journal/checkpoint evidence

Status: engineering evidence only, 2026-09-17. No production wire, golden bytes,
writer, reader, migration, live package or storage qualification is selected by
this report. This extends the index comparison to genuine authored Graph
commands; it does **not** integrate the earlier combined physical-placement,
liveness and reserve fixture.

## What was exercised

The new `ps2_graph_furnace` example uses Core `apply_graph_command`, typed PS1
`MutationRequest`/coordinates, and PS1's no-op normalization: compare the result
after restoring its previous Graph revision, and keep the old Graph/authored
revision when content did not change. A small independent test runs the real
PS1 package planner and checks change, no-op, inverse and Graph Batch results.
The legacy planner's full-ancestry validation is not the large-count hot loop.

Each fixture has one 16-node Graph. A deterministic eight-operation cycle
contains position edits, a no-op, an ordinary inverse position command, and an
atomic two-node Graph Batch. The inverse has a new OperationId and a new revision;
its `fixture_inverse_of` annotation is **not** a proposed Undo wire or a revision
rollback. Configuration edits, node creation, resource mutation, rendering,
multi-client session scheduling and large Graph-size scaling are not measured.

All N baseline operations really execute Core commands and instantiate their
original typed intent, before/after commitments, receipt and ordinal entry.
Baseline index construction is bulk, with one final publication barrier, not N
timed durable acceptances. Nothing at 100,000 operations is extrapolated. Each
candidate then executes four measured groups at each size 1, 8 and 32, adding
164 operations. Both maps use the same receipt, dense ordinal sequence, Graph,
journal, publication and integrity-hashed immutable pack implementation. The
tree algorithms are copied from the earlier furnace at `853781a`; only map
structure differs. Reachable staged pages are coalesced before pack writes.

One finite original journal group is created exclusively and file/directory
synced before the model's journal barrier completes. Existing indexed identities
and intra-group duplicates refuse **before** that barrier. Each original receipt
is then committed through both indexes and checked against the selected root.
The latest-only checkpoint marker binds selected HEAD, authored coordinate,
accepted prefix and through ordinal; journal removal follows its barrier.
Every original operation receipt remains in the indexes after removal.

These are unqualified model observations, not real `Accepted` or `Saved` grants:
every raw row explicitly records both qualification flags as false. Existing
approved semantics remain controlling. In particular, an exact accepted-prefix
matcher is distinct from the authored-coordinate UI Saved condition. The test
showing an older checkpoint lacks a newer no-op receipt does not decide whether
a no-op should change production UI status or force immediate checkpointing.
No external resource is opened or hashed by this workload; the PS1 oracle uses
only its tiny in-memory specimen. Zero large-media reads is by construction,
not a system-wide I/O tracing claim.

## Measurements

macOS 27.0 build 26A428, arm64, Rust 1.95.0 release build. Each cell below is the
mean successful process-write bytes per operation across four groups. It sums
pack and journal/candidate/HEAD/checkpoint-marker bytes; it is not device-level
write traffic. The N column is baseline count; reopen/retry/audit follow N+164.

| N | Map | Group 1, B/op | Group 8, B/op | Group 32, B/op |
|---:|---|---:|---:|---:|
| 1,000 | Radix | 25,933 | 11,407 | 8,778 |
| 1,000 | B-tree | 30,265 | 12,035 | 8,635 |
| 10,000 | Radix | 27,067 | 12,407 | 10,267 |
| 10,000 | B-tree | 31,377 | 14,929 | 11,943 |
| 100,000 | Radix | 29,080 | 14,015 | 11,484 |
| 100,000 | B-tree | 35,865 | 17,883 | 14,806 |

Following latencies are milliseconds. Journal/checkpoint and root-turnover
figures are medians of four samples; open and retry are single observations.
Four samples do not establish a production tail-latency distribution.

| N | Map | Group-32 journal | Group-32 checkpoint | Fresh-handle open | Old-ID retry | Root turnover |
|---:|---|---:|---:|---:|---:|---:|
| 1,000 | Radix | 53.301 | 54.640 | 0.706 | 0.102 | 20.524 |
| 1,000 | B-tree | 53.158 | 55.459 | 0.688 | 0.097 | 20.036 |
| 10,000 | Radix | 56.780 | 60.517 | 0.688 | 0.112 | 19.934 |
| 10,000 | B-tree | 56.806 | 59.999 | 0.701 | 0.127 | 20.385 |
| 100,000 | Radix | 54.162 | 57.402 | 0.696 | 0.116 | 19.944 |
| 100,000 | B-tree | 55.752 | 62.065 | 0.667 | 0.139 | 22.010 |

Journal latency includes command replay/validation, encoding and barriers, not
only fsync. Planning is separately recorded (100k group-32 medians 33.786 ms
radix, 34.838 ms B-tree). Checkpoint latency includes index publication and exact
original-receipt matching. At 100k, group-32 checkpoint maxima were 62.046 and
65.736 ms respectively. There are **11 `sync_all` calls per group**: two journal
and nine checkpoint/publication/marker/cleanup. Group formation wait and UI
response/render latency are not included. These numbers are not Saved latency
under a qualified platform profile.

Open is a fresh reader over OS-cached files, not a fresh process or disk-cold
measurement. It reads selected active/recovery structures, validates the shared
ordinal prefix, and decodes the selected current Graph; it does not enumerate
lifetime operations. Reads grow from 20 to 24 logical object/envelope requests
across 1k–100k, totaling about 35–37 KB. At 100k, old-ID lookup reads 16 objects /
9,641 B for radix versus 7 / 15,409 B for B-tree. Read counts are instrumented
range/envelope requests, not syscall tracing. Immediate repeated lookup was
0.095 / 0.114 ms. Full current-Graph cloning/canonicalization still scales with
Graph size, which this fixed-size workload deliberately holds constant.

Root-only turnover reuses both indexes and authored state. At 100k it writes
990 process bytes, reads three objects and performs four syncs. This is a
measurement-only turnover, not an independently crash-qualified root-only
publication protocol or an additional Saved acknowledgement.

The explicit replay audit reconstructs every authored operation from the
fixture-known genesis and matches each ordinal receipt to its OperationId
lookup. At 100k+164 it takes 133.390 s / 139.718 s. This exhaustive work is not
on normal open. It is an O(N log N) replay/cross-check, **not** a complete
arbitrary-import structural audit: extra map-only entries are not enumerated.
Trusted-history/import admission and full-map validation remain separate gates.

### Allocation and elapsed-time limits

The largest selected pack is 252,797,940 B; largest measured regular-file
allocation is 269,299,712 B. Candidates use sequential disposable directories,
not simultaneous giant fixtures. `allocated_delta` is net regular-file
`st_blocks * 512` growth after journal deletion, not peak coexistence or an OS
reservation. Allocation is lumpy: the radix 100k group-32 policy adds 16 MiB of
allocated extents across its 128 operations, whereas B-tree's same policy adds
zero newly reported blocks from already allocated capacity. Neither result is
a per-operation physical reserve bound. Directory metadata, APFS snapshots,
quota and device-level write amplification are not qualified.

The complete command reports **1,493.41 s wall**, 538.32 s user CPU, 6.83 s system
CPU, maximum RSS 142,180,352 B and peak memory footprint 112,706,088 B. Per-row
instrumented spans sum to **550.902 s**. Build, temporary-directory lifecycle
and possible host/scheduling intervals are not separately instrumented; the
roughly 943 s difference is unresolved. Do not attribute it to a particular
cause or present the summed sections as end-to-end throughput. Raw data retains
all timing samples and the allocation discontinuities.

## Verification and provenance

Exact commands, from the repository root:

```sh
cargo test --release -p photara-store --example ps2_graph_furnace
cargo clippy --release -p photara-store --example ps2_graph_furnace --tests -- -D warnings
rustfmt --edition 2024 --check crates/photara-store/examples/ps2_graph_furnace.rs
/usr/bin/time -l cargo run --release -p photara-store --example ps2_graph_furnace -- 1000 10000 100000 > /private/tmp/ps2-graph-authored-v2.jsonl 2> /private/tmp/ps2-graph-authored-v2.log
```

Seven release tests pass in 1.13 s; strict Clippy and formatting checks pass:

- `ps1_planner_agrees_on_change_noop_inverse_and_batch`
- `graph_noop_inverse_and_batch_are_real_commands`
- `valid_commands_cannot_reaccept_conflicting_or_group_duplicate_ids`
- `malformed_group_refuses_before_any_journal_write`
- `conflicting_digest_torn_journal_and_unknown_head_refuse`
- `older_checkpoint_is_not_current_after_new_journal_acceptance`
- `original_receipts_recover_at_each_cut`

The last test exercises both maps at five cuts: after the journal barrier,
after pack/candidate staging, after HEAD replacement, after the HEAD directory
barrier, and after the checkpoint-marker barrier. Fresh handles reconcile the
original group and verify every original receipt with no new ordinal. These
are controlled return/error cuts in one process, not process-kill, syscall-fault
or power-loss qualification. Corrupt selection and torn journal refuse while
preserving the journal. Pending-only online retry, concurrent owner fencing,
repeated orphan-producing interruption, reserve/ENOSPC and lease integration
are not certified here.

Development v1 exposed acceptance of a semantically valid reused identity before
the later checkpoint rejected it. The corrected pre-journal checks and explicit
negative test precede **all reported v2 timings**. Earlier partial v1 evidence
remains disposable and is not used in these tables. The interrupted v1 temporary
pack was removed; no live package data was touched.

The [unchanged six-row raw evidence](verification/ps2-graph-authored-v2.jsonl)
has SHA-256 `da6a4c6c5dc0593f2c54f79e65068697bc7880585dfde1019d2c428ee8b16abe`.
Disposable full logs are `/private/tmp/ps2-graph-authored-v2.log` (SHA-256
`8fd2b2bee50f8725faeb59aa92f739222763e46109ab0765dd427c1522a33a58`) and
`/private/tmp/ps2-graph-authored-tests.txt` (SHA-256
`9e4ee989fe7a975a45ee52341197210bca9ef138b5ef68a766d19015d2bf56e7`).
Final source hashes:

- `examples/ps2_graph_furnace.rs`: `aabdcfe60c09fb090bbc956d02afec4ecdcaa588031aa6addadc4eace5f18bdc`
- `examples/ps2_graph_furnace/index.rs`: `264b41bff939a9952778c3ee501d6e55d9cc8fd8e537b2aec2379dc9658229c2`
- `examples/ps2_graph_furnace/oracle.rs`: `8b7839f5eb54d76fd804de88c0c295510836a95bdf9fda706b276f3829787889`

Paths above are relative to `crates/photara-store`. Core/planner dependency
sources are unchanged from starting base `853781a`; intervening shared-worktree
commits added unrelated documentation and PS3 test code.

## Recommendation and next engineering gate

Genuine Graph operations do not require lifetime-history checkpoint/open work
in this model. Grouping reduces process-write bytes substantially, while
preserving every original operation and receipt. The 100k simple-pack workload
favors radix bytes at all measured group sizes; B-tree has fewer lookup objects
and a slightly smaller bulk baseline. The earlier combined physical-placement
B-tree lead and this result measure different costs. **Keep both candidates;
do not freeze the map or shared root/closure wire from either alone.**

Next, connect this real command/receipt path to the reviewed combined placement,
all-class liveness, persistent continuation and per-domain liability model.
Measure increasing Graph sizes, realistic mixed commands and actual grouped
asynchronous session scheduling, including pending-journal retries and no-op
checkpoint policy. Resolve the wall-time discrepancy and measure peak physical
coexistence before making latency or capacity promises. Re-run the qualified
file/namespace/owner fault matrix on that integrated path. This simple append
pack has no metadata reclamation, complete capacity accounting or qualified
barrier profile; none is established by this report. All changes remain new
disposable example/evidence files, with no permanent golden or production change.
