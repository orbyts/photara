# PS2 batching, packing, reserve and qualified flush review

Status: **disposable architecture/measurement review**. The existing flat
operation-index and inventory wire/goldens are not frozen. This review does not
implement or authorize a production reader, writer, migration, GC, live
package, or project switching. It follows the [real-file index comparison](PS2_INDEX_INVENTORY_REAL_FILE_COMPARISON.md).

## Three distinct acknowledgment levels

| State | Durable evidence required | What Photara may claim |
| --- | --- | --- |
| Speculative preview | None | Pending visual feedback only; never committed or Saved |
| Accepted authored operation | Original OperationId/request, ordinal and mutation in an ordered, recoverable journal prefix under a completed qualified barrier | The operation is recoverable; package checkpoint may still be pending, so display `Saving…` |
| Saved authored revision | Selected package HEAD, complete current-state closure, and original checkpoint receipt all verified under the qualified file/namespace barriers; included authored revision and digest exactly equal the current accepted session state | `Saved` for that revision only; a later accepted edit immediately returns to `Saving…` |

Validation and ordinal admission are serialized per project. A short group
commit may collect multiple independently identified operations before one
journal barrier. No operation in that group is acknowledged as accepted into
the committed UI model before the shared barrier succeeds; speculative previews
remain visibly pending. A failed or ambiguous barrier acknowledges no success;
it retains original intents and fences the group for reconciliation rather
than declaring that no bytes reached storage.
Operations keep individual IDs, digests, ordinals and receipts even when one
checkpoint includes the whole group. All GUI, CLI, headless and agent clients
use this same shared Rust authority, lease and ordered journal.
The shared API must expose the two receipts separately: `Accepted` carries the
durable journal coordinate and original operation identity; `Saved` carries a
verified package HEAD/checkpoint coordinate and covered authored revision.
A CLI or agent may explicitly await a Save barrier, but no surface should infer
package durability from an `Accepted` response or from the absence of an error.

Package checkpoints may coalesce several journal-accepted operations into one
package/root revision containing the latest authored revision, but never erase
their individual authored revisions or original dedupe evidence.
Unchanged flush is a no-op. A Save Now, close, switch, sleep or quit barrier
captures a finite accepted prefix, drains pending input, publishes and verifies
the target checkpoint, and only then returns its receipt. If new edits arrive
after that prefix, a successful older receipt does not set global `Saved`.
Unknown outcomes freeze later writes and reconcile exact old/candidate HEAD
and original identities; no fresh-ID retry.

The current [session contract](PROJECT_SESSION_DURABILITY.md) proposes idle,
sustained-edit and gesture/text debounce intervals; those remain experimental
tuning values, not approved production latency or durability thresholds.

## Qualified barrier dependency, not a sync-count shortcut

The current [local-APFS qualification plan](PS2_MACOS_STORAGE_QUALIFICATION.md)
requires qualified full-file and directory-entry persistence, exact ordering,
registered cooperative admission and same-volume atomic HEAD replacement.
Its profile is **not yet qualified**. Apple's [fsync manual](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/fsync.2.html)
warns that `fsync` may not flush a drive's own buffered writes and points to
`F_FULLFSYNC` for stronger ordering; the installed SDK documents a same-device
barrier for previously fsynced data. Neither statement alone qualifies the
whole Photara multi-file/namespace protocol. The first furnace used
`sync_all`, so its roughly 54 ms per-operation latency is process-level
evidence, not production `Saved` latency.

The following grouping is a candidate **only if** the profile proves its
barriers and failure model:

1. Append a bounded group of framed journal operations and original IDs to a
   registered journal file. Validate frames, then make that exact prefix
   durable with one qualified journal barrier. Acknowledge each included
   operation only after that barrier; an ambiguous group gets no successful
   acknowledgment and retains original evidence for reconciliation.
2. Build changed immutable index/inventory/receipt and authored objects into
   an append-only pack or sealed segment. Hash and reread the exact changed
   bytes, flush the data file(s), and flush any newly published namespace
   entries before depending on them. One barrier can cover multiple changed
   pages in the **same** pack; per-page full flush is not semantically needed.
3. Persist candidate intent and reserve evidence before selection. Write and
   flush temporary HEAD, compare exact old HEAD and lease, replace HEAD on the
   same qualified volume, and complete the required root-directory/device
   barrier. Reopen the selected current state and changed paths.
4. Append and durably barrier the original checkpoint acknowledgment in the
   recovery journal. Only then may the UI report `Saved`, and only while its
   included revision/digest equals the current accepted state.

The journal barrier cannot be postponed past an accepted-operation response.
Object/HEAD/receipt barriers cannot be skipped for a `Saved` response. Some
full-device barriers may cover previously flushed same-device files under a
qualified platform profile, but neither cross-device stores nor unflushed
directory entries are assumed covered. Flush failure or ambiguous completion
means `OutcomeUnknown`, preserved evidence and no `Saved` claim.

| Interruption point | Permitted recovery result; forbidden claim |
| --- | --- |
| Before journal-group barrier | Reconcile framed prefix; no successful acceptance for the uncertain group |
| After journal barrier, before candidate HEAD | Recovered accepted operations remain pending checkpoint; no `Saved` |
| After pack/intent barriers, before HEAD replacement | Exact old HEAD remains selected; candidate bytes/intent are retained for original-ID reconciliation |
| During/after HEAD replacement, before its barrier | Inspect exact old or candidate HEAD and closure; outcome is unknown until the qualified protocol resolves persistence |
| After selected HEAD barrier, before checkpoint receipt barrier | Selected state may be valid, but no `Saved` response until original receipt is durably reconciled |
| During compaction generation change | Old and new physical generations must independently read active **and** recovery roots; neither generation is retired on an unknown outcome |

Cold opening may authenticate HEAD and both selected root descriptors, validate
current-state package closure and fetched index/locator paths, then defer
historical exhaustive traversal. A newly imported or not-yet-audited package
remains structural/read-only until a full audit succeeds for writable admission.
An explicit full integrity audit walks all committed operation, inventory and
receipt pages plus required package-resident closure; it does not turn an
ordinary open into a large external-media verification job. Retaining a valid
recovery root is a measured physical cost, not an optional optimization.

## Packing and compaction question

The first furnace's `offset + length + SHA-256` references share immutable
subtrees but are not stable logical object identities. Moving a live object
changes its reference and recursively changes parent/root hashes. The packed
comparison must charge for a bounded persistent locator or another stable
logical-reference method; it must not silently restore a flat lifetime lookup
or second semantic HEAD. Compare sealed immutable segments with whole-segment
retirement against stable digest identity plus relocating physical placement.
For relocation, publish and verify new bytes and locator state before any old
copy becomes retirement-eligible. Active, recovery, pinned and conversion
roots remain independently readable throughout. The disposable furnace may
retire its own temporary segments after verifying both generations; no
production/live deletion or GC is authorized in this review.

| Physical option | Strength | Cost or unresolved condition |
| --- | --- | --- |
| Immutable sealed segments, direct references | No locator authority or read; old and recovery refs remain stable | Can retire only an entirely unreachable segment. A single old pin per segment can strand most physical bytes indefinitely; any claim of bounded space needs a measured pin/churn model or another reviewed rewrite protocol. |
| Stable virtual/content identity plus bounded segment locator | Relocation can leave semantic root hashes unchanged; bounded locator lookup can preserve logarithmic operation paths | Extra read/space on access; locator-generation selection, old/new coexistence, atomicity and liveness tracking require a reviewed physical-dispatch protocol. A flat lifetime locator or whole-history liveness scan on every checkpoint is unacceptable. |

The locator selector, if any, must be **placement-only**: it cannot choose a
different authored root, operation outcome, retention obligation or receipt.
Nevertheless it is another independently recoverable selector for physical
bytes. Proving that both old and new locator generations resolve every active
and recovery object identically is a new protocol obligation, not implicit
approval from the existing sole-semantic-HEAD rule. An explicit full liveness
audit can plan compaction in a disposable fixture; production responsive
compaction still needs incremental, persistent liveness or an equivalently
bounded safe-retirement proof. Neither may retire a managed Asset Store backing
solely because a package page was compacted.

Admission must reserve worst-case bounded journal group, operation/index path
growth, candidate pack and locator writes, checkpoint envelopes, HEAD/receipt
staging, and any in-flight compaction copy **in addition to** the retained old
and recovery bytes. Charge actual physical allocations and unresolved
reservations; do not count anticipated deletion as free capacity. Exhaustion
refuses or backpressures before accepting a new operation, preserving authored
and recovery content.

For a proposed batch of at most `B` operations, the admission ledger must
conservatively prove free qualified capacity at least
`J(B) + I(B) + P(B) + H + R + C + A`, where `J` bounds complete framed journal
and dedupe/receipt evidence, `I` bounds changed ordinal/map/inventory paths,
`P` bounds changed authored objects and physical pack framing, `H` bounds
candidate intent plus HEAD staging and old-HEAD preservation, `R` bounds the
post-publication receipt, `C` bounds any already admitted compaction copy and
locator generation, and `A` covers measured filesystem allocation granularity
and in-flight unknown outcomes. Old active/recovery/pinned bytes remain charged
separately. Bounds depend on explicit maximum request/object/page/segment sizes
and tree height, not an average benchmark result. Once a batch begins, its
reservation cannot be lent to another writer. Failure or uncertain publication
holds the charge until exact reconciliation; no rollback may erase an accepted
operation. A separate compaction admission must reserve its complete bounded
copy plus old and new locator/segment copies before it starts, and it may pause
without blocking an already reserved journal batch.
The ledger carries a cumulative liability for all journal-accepted but not
package-Saved operations: coalescing their checkpoint may reduce actual writes,
but it may not release their worst-case reserve in advance. Admission of the
next batch proves the new marginal bound **plus** that outstanding liability.
Once a verified checkpoint consumes a prefix, release only the proved surplus;
active/recovery/history/pin requirements still charge their retained bytes.

The physical free-space preflight is advisory because unrelated processes can
consume space afterward. ENOSPC at any subsequent syscall is an ordinary
tested failure path, never a reason to retire recovery content or claim that an
unbarriered operation was Saved. For a user-selected project volume and a
separate managed Asset Store, capacity and durability are accounted **per
qualified location**. A package checkpoint may not borrow Asset Store free
space; an exact capture spanning both locations has its own intent and
publication ordering. Ordinary package turnover still does not read or hash
large external media.

An already verified retained backing on a temporarily offline qualified store
remains a valid historical retention commitment with unavailable bytes; it is
not silently reclassified as lost merely because the mount is absent. By
contrast, a **new** captured version cannot be acknowledged as retained, or
included in a `Saved` root that promises its new backing, until that backing's
publication, verification and durable evidence complete under its own
qualified location profile. Cross-location group flushes do not create a
single atomic transaction: the package intent must reconcile an orphan
backing, an absent backing, or an uncertain outcome without claiming `Saved`.
Runtime Host Bindings and mount/availability observations remain device-local;
only portable logical backing and retention evidence enter selected roots.

## Measurements and recommendation

The [disposable extension](../../crates/photara-store/examples/ps2_index_furnace/packing.rs)
compares both logical maps under identical 1 MiB packed-segment storage with
64 extra synthetic operations per batch policy (groups of 1, 8 or 32). Sealed
mode uses direct virtual offsets; relocatable mode resolves stable virtual
offsets through a 256-slot, 2 KiB per-segment table. Relocation copies 4 KiB
blocks but small objects are densely appended, not block-aligned one by one.
The fixture retains OS page cache, uses synthetic small receipts and `sync_all`,
and does **not** qualify a production APFS profile, authored Graph-command
recovery, a user-visible scheduling deadline or device-level write traffic.
Within each map/mode run, it applies 64 operations at batch 1, then 64 at
batch 8, then 64 at batch 32. The two maps receive matching deterministic
operations for each policy, but the three policies do not begin from cloned
identical trees; occupancy/split history can affect cross-policy differences,
especially at small N. Batch 32 has only two checkpoint samples, so its p95
and max are exploratory, not stable tail-latency estimates.

The [unchanged raw JSONL](verification/ps2-batching-packing-flush.jsonl) has a
header, fault result and 12 fully materialized map/mode/scale rows at 1k, 10k
and 100k lifetime operations. Its SHA-256 is
`6ebd83ec3f9ac179f6560aedc04118920a3a18bc66025dd6b46ef0f8dc3c4170`.
The full sequential run took **2,238.32 seconds wall**, 101.00 user and
231.17 system seconds. This unexplained wall/CPU disparity is a reason **not**
to advertise stable latency or throughput from the single run. Maximum
reported *final* file allocation was 64,479,232 bytes; peak allocation during
old+new relocation was not measured. All disposable packages were removed.

| Lifetime N | Map | 1 / 8 / 32 operation checkpoint: pack bytes per accepted operation | 32-operation total fixture process-write bytes per operation |
| ---: | --- | ---: | ---: |
| 1k | Radix | 9.12 / 5.79 / 4.70 KB | 4.83 KB |
| 1k | B-tree | 10.97 / 5.97 / 4.28 KB | 4.41 KB |
| 10k | Radix | 10.77 / 7.02 / 5.95 KB | 6.09 KB |
| 10k | B-tree | 14.26 / 8.96 / 7.54 KB | 7.68 KB |
| 100k | Radix | 13.61 / 8.09 / 7.20 KB | 7.33 KB |
| 100k | B-tree | 18.88 / 11.72 / 10.27 KB | 10.40 KB |

The pack bytes are identical in the sealed and relocatable modes because both
use the same densely appended logical objects; the right column is sealed
mode and includes envelope and placement bookkeeping. Relocatable mode adds a
small per-segment pointer/table write on new segments. At 100k and batch 32,
radix wrote about **30% fewer fixture process bytes** than B-tree. The tiny
1k batch-32 reversal shows why the write-heavy lead is conditional rather
than a universal tree claim. At every size both modes preserve original
receipts and the dense ordinal history; only intermediate unselected pages
are eliminated. Filesystem allocation deltas are in the raw data, but APFS
allocation granularity means they are not device-write telemetry.

Each fixture group took about 13 `sync_all` calls whether it contained one
or 32 operations: approximately **13.0 calls/operation** at batch 1,
**1.6** at batch 8 and **0.4** at batch 32. At 100k in sealed mode, median
journal-group barrier time was about 10–12 ms. Median checkpoint-after-journal
was 59–79 ms at batch 1 and 93–111 ms at batch 32. A returned `Accepted`
receipt therefore cannot be presented as `Saved`; the latter waits for that
checkpoint and its original acknowledgment. The modeled 50 ms group deadline
was **not** exercised by a scheduler, so user-visible wait, fairness and tail
latency are unmeasured. These `sync_all` counts include fixture redundancies
(for example an unchanged pack/state sync during HEAD publication); a
qualified barrier protocol may group unchanged-file work but cannot remove
the journal acceptance or selected-object/HEAD/receipt persistence edges.

At 100k, old-ID retry p50 was 445/297 µs for radix/B-tree in sealed mode and
1,200/5,463 µs in relocatable mode; it wrote no new objects. Fresh-handle
structural open before compaction was 671/588 µs sealed and 1,484/13,798 µs
relocatable (radix/B-tree). These are OS-cache-retaining opens, not power-cold
opens. Metadata-only turnover stayed roughly 24–32 ms across the measured
rows and reused historical indexes. Before turnover, the independently valid
recovery root cost an additional 209,275 bytes for radix or 297,210 bytes for
B-tree at 100k, atop shared active structure; this fixture's small authored
state is not a bound for real projects.

The sealed mode reclaimed **zero** bytes in these mixed-lifetime fixtures:
every candidate closed segment still had a retained block. Relocatable mode
required a full selected-root liveness audit before its bounded two-segment
copy. At 100k radix, that planning took 20.805 s; copying 2,043,904 bytes
and reserving up to 1,046,528 extra bytes for a *sequential segment* retired
2,097,152 old bytes but reduced current data size by only **53,248 bytes**.
Production policy should skip such low-yield work; no numerical threshold is
approved. The 100k B-tree relocatable planning row took 109.582 s, whereas
its later full audit took 15.584 s. The fixture performs roughly 511k uncached
pointer/table reads (547 MB) in that planning row; radix performs roughly
598k (640 MB). This is negative evidence for the current uncached locator,
not proof of a stable B-tree-versus-radix latency ranking. A generation-aware
safe locator cache and persistent incremental liveness are needed before
claiming responsive compaction. The selected and recovery roots remained
independently auditable after each fixture relocation. This physical fixture's
liveness input covers those two selected roots; it does not prove the eventual
full keep-set for explicitly pinned history, conversion snapshots, retained
resource obligations or in-flight operations. Those edges remain normative
requirements and a production compaction gate.

Controlled function cuts passed for reserve refusal before group acceptance,
journal-only replay, pack-before-HEAD replay and HEAD-before-ack reconciliation
in both maps/modes. Relocation cuts passed at partial copy, before pointer,
after pointer, reserve refusal and torn placement-pointer refusal. These are
synthetic **index/dedupe** operations: their journal does not carry real
authored Graph mutations, and the cuts do not simulate process kill or power
loss. The persisted fixture reserve is a finite virtual-byte cap, not the
physical `J+I+P+H+R+C+A` proof above. Fault and measurement evidence therefore
supports a design direction, not writable production admission.

**Recommendation:** retain the ordinal Merkle sequence plus compressed radix
map as the conditional write-heavy logical lead; keep the paged B-tree as the
read-favoring comparison. Coalesce only unselected checkpoint pages, not
accepted operations or receipts. Batch journal barriers with a finite
queue/time and capacity bound, acknowledge `Accepted` only after that barrier,
and report `Saved` only after the selected package and original checkpoint
receipt are durably verified for the current authored revision. Do **not**
freeze a physical locator, permanent wire, production batch interval or
`F_FULLFSYNC` schedule yet. Next gates are a bounded physical-dispatch and
incremental-liveness proof, complete physical reserve accounting, and a
qualified macOS file/directory barrier fault matrix on a designated storage
profile. The production Graph journal/session path remains unimplemented.

Reproduce with a disposable target directory:

```sh
cargo run --release --target-dir /private/tmp/photara-ps2-furnace-target \
  -p photara-store --example ps2_index_furnace -- packing 1000 10000 100000
cargo test --release --target-dir /private/tmp/photara-ps2-furnace-target \
  -p photara-store --example ps2_index_furnace
cargo clippy --release --target-dir /private/tmp/photara-ps2-furnace-target \
  -p photara-store --example ps2_index_furnace -- -D warnings
rustfmt --edition 2024 --check \
  crates/photara-store/examples/ps2_index_furnace.rs \
  crates/photara-store/examples/ps2_index_furnace/packing.rs
```

Five release example tests, strict Clippy, formatting and whitespace checks
passed. The example's final SHA-256 is
`8c1e8357917fc7cef4fbbdb9262efb0fe1e7eec512b5f50d2a384d64aa1b65e3`;
its companion module is
`8dcfc175783c7f22fc1af861ada80ff0e2abc3e1369f5a94c4f9079a5febec0a`.
The scale run preceded lint-only checked-conversion/style edits; no fixture
algorithm/layout change followed it. The raw output and commands remain the
measurement provenance; this report is not a production codec acceptance.
