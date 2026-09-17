# PS2 disposable real-file index/inventory comparison

Status: **disposable evidence, not permanent package wire or production code**.
This follows the [scaling design review](PS2_INDEX_INVENTORY_SCALING_REVIEW.md).
The existing flat operation-index and inventory golden bytes remain unfrozen;
the HEAD/bootstrap/outer-commit proposals are not changed by this furnace.

## Question and controlled comparison

Compare two persistent OperationId maps: a compressed radix map and a paged
ordered B-tree. Hold constant the original immutable receipt, dense ordinal
Merkle sequence, typed compositional root inventory, integrity-hashed immutable pack
backend, checkpoint/HEAD process, verification calls, and workload. The result
must separate the cost of the candidate map from shared publication cost.

The [reproducible Rust furnace](../../crates/photara-store/examples/ps2_index_furnace.rs)
uses complete disposable fixtures at increasing accepted-operation counts.
Its immutable object pack stores JSON objects with offset/length/SHA-256
references. Existing child references are shared across roots; identical bytes
at different offsets are **not** deduplicated. This is an integrity-hashed
fixture layout, not a selected portable CAS/object layout or production
package format. Both candidates use it identically.
The synthetic receipts are only about 153–159 bytes and omit full approved
provenance/resource evidence; all byte figures are **fixture** costs, not a
production per-operation storage promise.

For each candidate, measure append/checkpoint latency and bytes, old-ID retry
lookup, root-only turnover, cold structural opening, explicit full audit,
incremental file growth, filesystem allocation and retained stale pages. Keep
the cold-open claim honest: current authored state is additional work, and
structural opening does not certify unread historical descendants. The fixture
reopens file handles and reader state but does not evict the operating-system
page cache; report it as a **fresh-handle**, not power-cold, open. Filesystem
allocation is reported from file blocks, not actual device-write telemetry.

Fault and refusal coverage must include corrupt/missing nodes, ordinal gaps,
duplicate IDs and conflicting request digests, malformed page shape,
incomplete staging, interruption around HEAD selection, reserve exhaustion,
and imported-root read-only admission until a complete audit succeeds.

The furnace must not read or hash large external media during ordinary open,
append, checkpoint or turnover. It is not an Asset Store or package codec.
Its tiny fixed authored object is deliberate: this isolates lifetime operation
count N, but does not benchmark opening a large authored graph. Its fixed
reserve cap only demonstrates refusal before mutation; it is not an approved
worst-case production reservation formula. `sync_all` and directory barriers
are real process calls in the fixture, but do not qualify power-loss durability
on any production storage profile.

## Results

The complete [v2 raw JSONL](verification/ps2-index-inventory-real-file-v2.jsonl)
contains both candidates at 1,000, 10,000, 100,000 and 1,000,000 fully
materialized operations. Each baseline has every original receipt, both
indexes and a published root. After a full imported-root audit, each candidate
runs 32 individually synchronized fixture publications, 64 old-ID retries,
32 absent-ID lookups, 16
metadata-only root turnovers, fresh-handle structural opens and another full
audit. The one-million-operation pair and all smaller cases ran sequentially
in 188.58 seconds total (143.47 user, 28.75 system); peak RSS was unavailable
because the sandbox blocked the timer's kernel query. The HEAD-bound writable
trust check was added immediately afterward. Its [v3 spot evidence](verification/ps2-index-inventory-trust-spot.jsonl)
at 1,000 operations preserved exactly the same object/HEAD byte counts and
sync counts for both maps, and the release tests include substituted-HEAD
refusal. Thus the table below uses v2 timing provenance, not a misleading
claim that timing was rerun at every scale after that constant trust check.

| N | Map | Bulk baseline pack | Append/checkpoint p50 | Pack bytes / accepted append | Old retry p50 | Root-only turnover p50 | Different-root open p50 / object reads | Full audit |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1k | Radix | 0.613 MB | 52.2 ms | 9.68 KB | 180 µs | 16.3 ms | 177 µs / 18 | 0.040 s |
| 1k | B-tree | 0.595 MB | 54.0 ms | 11.65 KB | 234 µs | 16.2 ms | 287 µs / 18 | 0.045 s |
| 10k | Radix | 6.175 MB | 51.0 ms | 10.85 KB | 143 µs | 15.6 ms | 171 µs / 20 | 0.323 s |
| 10k | B-tree | 5.982 MB | 53.1 ms | 14.50 KB | 265 µs | 17.0 ms | 300 µs / 20 | 0.306 s |
| 100k | Radix | 62.101 MB | 53.8 ms | 13.46 KB | 319 µs | 17.5 ms | 329 µs / 22 | 3.290 s |
| 100k | B-tree | 60.220 MB | 52.1 ms | 18.76 KB | 271 µs | 16.3 ms | 288 µs / 22 | 3.014 s |
| 1M | Radix | 625.346 MB | 54.2 ms | 14.78 KB | 277 µs | 16.3 ms | 279 µs / 22 | 32.566 s |
| 1M | B-tree | 606.329 MB | 54.2 ms | 21.63 KB | 270 µs | 16.4 ms | 278 µs / 22 | 28.487 s |

Timing is a single local run, not a statistically stable latency guarantee.
The 32 append samples include a fixed **ten `sync_all` calls per operation**;
their approximately 50–54 ms p50 is dominated by that unoptimized durability
schedule, so the map's lower byte work does not yet translate into faster
visible autosave. Old retries returned the original receipt with zero new
object/envelope bytes. A root-only turnover wrote roughly 0.81–0.84 KB of
pack objects and reused both historical indexes. Different-root structural
open read 18–22 objects, growing at height transitions rather than with N;
after root-only turnover, active/recovery share the indexes and opening read
12 objects. These are fresh file handles under a retained OS page cache.

The radix map wrote **14.78 KB** of pack data per accepted append at 1M versus
**21.63 KB** for B-tree, about 32% less. Conversely its fully bulk-built
baseline pack was **625.35 MB** versus **606.33 MB** for B-tree, about 3% more.
At 1M the 64 old-ID retries read 1,225 objects with radix versus 512 with
B-tree (about 19 versus 8 per lookup), despite similar OS-cached elapsed time.
This matters for a future truly cold or higher-latency storage profile; the
fixture did not evict the OS cache or qualify random-read latency.
After the 32 appends and 16 turnovers, unreachable/stale pack bytes were
approximately 464 KB for radix and 662 KB for B-tree; neither fixture collects
them. Filesystem `st_blocks` deltas are reported in the raw output but are
extent/preallocation-granularity noisy: at 1M they showed only 4 KiB allocated
delta despite hundreds of KiB appended. Pack-length deltas are reliable bytes
written by this process, **not** physical device-write telemetry or a bound on
filesystem metadata amplification. No batching or pack compaction was tested.

The full audit time rises from tens of milliseconds to roughly 29–33 seconds
at 1M. This prototype uses global map reconciliation and assembled subtree
entry vectors, so it is an exhaustive audit but **not** a proof of optimal
linear audit CPU. Ordinary structural opening does not silently upgrade to a
full integrity claim.

The initial fault suite passed for **both** maps: partial object and
candidate-intent writes; interruption before and after HEAD; replay under the
original ID and digest; conflicting digest refusal; same-ID return of the
original receipt without a new object write; reserve refusal before mutation;
missing/corrupt and authenticated-malformed nodes; ordinal gap, duplicate ID,
prefix mismatch and map/sequence disagreement; independent recovery-root
audit; structural read-only open of an imported root followed by audit-enabled
admission; and later corruption of an unread historical receipt (structural
open succeeds, but retry and full audit refuse). Ordinal height transitions at
16, 256 and 4,096 entries were also exercised. The fault cuts are injected
function interruptions followed by handle reopen, **not** process kill or
power-loss qualification. No claim about crash-safe production storage follows
from these checks.

## Recommendation and remaining gate

Retain the **dense ordinal Merkle sequence plus a compressed radix OperationId
map** as the *conditional write-heavy* leading logical design. Both candidates
satisfy the bounded-path goal in this fixture, but radix has lower incremental
pack-byte and stale-page cost at every measured scale. B-tree's smaller
bulk-built baseline and markedly fewer old-ID read calls make it a serious
alternative, especially on higher-latency storage or if a packed-page layout
changes write amplification. The data do **not** justify freezing
fanout, page encoding, object locator, physical CAS/deduplication, or permanent
wire bytes.

Before a production codec or a snappy continuous-autosave claim, review a
bounded batch/flush policy, registered worst-case reserve accounting,
pin/resource/retained-history inventory edges, pack compaction or relocation
without invalidating references, and physical write amplification under a
qualified storage profile. Add process-kill and storage-fault tests; these
function-cut tests are not enough. The fixture has no large external media,
provider backing or Asset Store, so it neither changes nor independently
qualifies the approved zero-routine-media-read and retention semantics. No
new product or security trade-off was exposed that changes those semantics.

## Reproduce and review boundaries

The disposable example is the only new Rust file; production package code,
existing tests, candidate wire/goldens, live packages and deployment were not
modified. The final example source SHA-256 is
`8d806433a4f69c19c0fa8513c39840532337bbba72dcdced9466d19a86b77227`.
Run from the repository root with a disposable Cargo target directory:

```sh
cargo run --release --target-dir /private/tmp/photara-ps2-furnace-target \
  -p photara-store --example ps2_index_furnace -- 1000 10000 100000 1000000
cargo test --release --target-dir /private/tmp/photara-ps2-furnace-target \
  -p photara-store --example ps2_index_furnace
cargo clippy --release --target-dir /private/tmp/photara-ps2-furnace-target \
  -p photara-store --example ps2_index_furnace -- -D warnings
rustfmt --edition 2024 --check crates/photara-store/examples/ps2_index_furnace.rs
```

Clippy and formatting passed; four release tests passed: staged-reserve
exhaustion preserves HEAD and intent, an audited session refuses externally
substituted HEAD, recovery audit does not rely on a corrupt active root, and
adversarial shared hash prefixes route correctly. The process-cut/fault suite
passed in both map runs. Generated temporary packages were removed by the
furnace. The comparison is a design checkpoint for review, **not** approval to
implement a production codec, live-package writer, automatic conversion,
retirement, or project switching.
