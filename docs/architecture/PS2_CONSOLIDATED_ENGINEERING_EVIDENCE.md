# PS2 packed persistence engineering evidence

Status: consolidated disposable engineering review, 2026-09-17. This is a
comparison and qualification record, **not** a permanent package wire or a
production write authorization. The approved Resource Storage, Verification
and Retention contract and the separate `Accepted`/`Saved` acknowledgements
remain unchanged. Large external managed media is not read or strongly hashed
by ordinary open, autosave, root turnover or metadata validation.

## What changed after the negative tail-copy experiment

The [prior physical review](PS2_PHYSICAL_DISPATCH_LIVENESS_RESERVE_BARRIERS.md)
rejected a HEAD-bound implementation that copied a segment tail and wrote
hundreds of kilobytes of metadata for a small publication. A second disposable
fixture in `crates/photara-store/examples/ps2_index_furnace/placement_v2.rs`
instead appends to bounded 256 KiB data/metadata packs and selects barriered
prefixes. Physical placement is separate from stable semantic object identity.
The HEAD-selected active/recovery locator roots are cached by exact selected
digest/generation; no mutable per-segment selector is trusted. Shared changed
locator ancestors are materialized once per selected batch. Cold open probes
selected structure; exhaustive closure and liveness auditing remain separate.

One candidate pack is scanned at a time. A no-write preview counts the live
payload, locator/metadata rewrites and both physical selection envelopes. It
defers a candidate whose standalone retirement would grow storage. Old pack
bytes survive until **both** active and recovery placement selections move;
an additional reader/undo pin conservatively refuses retirement until exact
release. This is fixture maintenance evidence, **not** production GC or a
complete all-class liveness integration.

| Fully materialized 100k operation fixture | Radix sequence/map | Paged B-tree sequence/map |
| --- | ---: | ---: |
| Single physical publication, total process bytes/op | 43,590 | 38,677 |
| 8-operation publication, total process bytes/op | 26,339 | 21,960 |
| 32-operation publication, total process bytes/op | 19,728 | 17,038 |

The grouped fixture retains every original receipt and contiguous ordinal;
recovery selects the prior group boundary. At batch 32, the measured median
physical publication/group was 151.29 ms radix versus 101.39 ms B-tree, with
about 12.4 versus 12.0 fixture `sync_all` calls per group. These timings exclude
semantic source construction, journal acceptance, full qualification barriers,
OS-cold caches and user-visible scheduling. They are not `Saved` latency.
Unlike the earlier *logical-only* batch-32 comparison, which favored radix
pack bytes by about 30% at 100k, the bounded physical locator/pack implementation
favors B-tree on this synthetic workload. That reversal is real evidence but
not yet a universal representation choice. Keep radix as the conditional
logical lead and B-tree as the physical/read comparison until the combined
Graph/session workload is measured.

At 100k, a controlled high-garbage candidate pack containing 2,647 live bytes
reclaimed about 233 KiB after relocation/locator cost for either map. An
all-live candidate predicted about 1.7 MiB net growth and was deferred with
zero physical writes or selection change. A mixed-lifetime candidate also
showed that a standalone data phase can be negative while a later metadata
phase is positive; this fixture does **not** presume an unreserved future
maintenance phase. Coupled maintenance requires a single reviewed reserve and
publication plan.

The versioned [compact placement evidence](verification/ps2-packed-placement-summary.jsonl)
contains provenance, six scale rows, four controlled-density rows and four
grouped rows, plus raw-output hashes and exact commands. Fourteen release
example tests, strict Clippy and Rustfmt passed. The source remains a
disposable example; no live package is written.

## Combined physical placement, all-class pins and capacity

The [final combined evidence](verification/ps2-combined-packed-final.jsonl)
(SHA-256 `a975a6aed6eaa198bd98339ab86a331867c3a5ab3e49013615f5a2845a9bbe7a`)
contains six scale rows, four economic rows, turnover measurements, exact
commands and raw-output hashes. The v3 disposable fixture composes bounded packs and HEAD-selected locator
roots with all twelve pin classes and a finite capacity-liability ledger. It
was not enough for the earlier independent models to pass separately. An
independent audit of the actual v3 code found a token/work mismatch and then
a rollover/restart bug: after exact-old reconciliation, resume could clear a
liability while selecting an old pack extent. Both were corrected before the
final run. [The versioned adversarial record](verification/ps2-combined-v3-independent-review.jsonl)
retains the counterexample and 15 corrected actual-code checks. The original
loose v3 reserve run is retained as negative evidence.

Preflight now creates a no-effect encoded write plan for the exact checkpoint
or candidate-local compaction. Its fixture admission bound charges each
touched 256 KiB pack, explicit namespace/selector work and a 64 KiB COW
uncertainty allowance; it takes no credit for expected retirement. The
original compaction bound was about 3.34 GB for a 262 KiB candidate and was
not usable. In the final 100k run it was **3.57 MB radix / 3.29 MB B-tree**
against **2.25 MB / 2.23 MB** modeled consumption (1.59× / 1.48×). The
checkpoint reserve remains conservative, especially for one operation:
1.21 MB for either map, about 10.0× / 10.9× modeled consumption; at batch 32
it is 1.91 MB / 1.84 MB, about 2.53× / 2.78×. These are internal *modeled*
bounds, not an APFS allocation theorem or exclusive space reservation.

Before-HEAD recovery validates and reuses the exact barriered candidate
roots/extents under the original liability, rather than appending a second
copy. Repeated cuts did not duplicate payload. Missing or corrupt continuation
bytes refuse selection. Unknown selection retains the original evidence.
Active/recovery locator roots and all ten additional classes are checked at
the candidate pack; an unresolved hold blocks unrelated work. Exact release
allows a high-garbage candidate to reclaim bytes, while an all-live candidate
is refused with no writes. The fixture's high-water capacity ledger does
*not* yet credit physical retirement. This avoids false credit but would
eventually over-refuse a long-lived project; see the remaining gate below.

| Final 100k materialized combined fixture | Radix + ordinal | B-tree + ordinal |
| --- | ---: | ---: |
| Single publication, total process bytes/op | 59,963 | 49,847 |
| 8-operation publication, total process bytes/op | 27,274 | 23,245 |
| 32-operation publication, total process bytes/op | 20,303 | 17,562 |
| Unchanged-root turnover, median at 100k | 26.23 ms | 26.85 ms |

Unchanged-root turnover wrote no data/index/locator pages and about 2.04–2.09
KB of envelopes, with eight fixture sync calls; its median stayed roughly
26–29 ms at 1k, 10k and 100k. This is OS-cache-retaining process evidence,
not a cold-device or user-visible `Saved` latency claim. The packed combined
fixture favors **B-tree + ordinal sequence** on total physical bytes at all
measured counts/batches, reversing the earlier logical-only radix advantage.
Thus B-tree becomes the provisional packed logical/physical lead; radix stays
the comparison. Neither is a permanent wire choice, and the genuine Graph
authored-operation and receipt/barrier codec remains a pre-freeze gate. The
v3 synthetic receipts include ID/request/ordinal but do not prove actual
authored before/after chains, no-op semantics, journal acceptance or `Saved`.

## Incremental liveness and admission

The separate `ps2_liveness_reserve.rs` fixture persists 12 distinct pin classes:
active/recovery roots, accepted journal, undo, history, conversion source,
export, backup, reader lease, unresolved intent, relocation and resource
obligation. Tokens and epochs prevent stale release; unknown selection and
reader activity hold pins. Conservative reference counts and a bounded durable
retirement queue let foreground work touch changed paths only. Normal reopen
reads one selected cursor (two with an unresolved finite transition) and does
not reconstruct the lifetime operation map. Exhaustive replay/audit is an
explicit separate operation.

Its first fixed-page encoding was rejected at 39.8 MB for only 128 objects.
Coalesced, route-anchored compact metadata reached **84,099,072 allocated
bytes at 100k object/pin/original-operation records**. A maximum measured
64-record transaction appended 48,544 bytes; a genuine 32-descendant cascade
appended 11,496 bytes in one selection. Point lookup touches 19 bounded pages.
Route-anchor tests prove candidate-local membership/refusal against active,
recovery and reader roots, including shared descendants and unknown epochs.
Actual shared-pack relocation/retirement of these liveness pages is still an
integration gate; the standalone arena does not delete packs.

[Scale results](verification/ps2-liveness-reserve-route.jsonl) and
[reserve/interruption trace](verification/ps2-liveness-reserve-trace.jsonl)
are versioned. Twenty example tests (including the later capacity cross-check),
strict Clippy and Rustfmt passed.

The capacity model checks all requested physical domains together. It charges
the old active/recovery/pinned keep-set, finite accepted-but-unsaved liability,
journal/original receipts, data/metadata/index/inventory/locator pages,
namespace and pack rollover, HEAD/intent/Saved receipt, liveness and reserve
metadata, barrier evidence, and **both** compaction selections with old/new
bytes resident. It gives no credit for anticipated retirement. File-data
preallocation can satisfy only that file-block category, not future namespace,
metadata copy-on-write or quota costs.

The [cross-check](verification/ps2-capacity-placement-cross-check.jsonl)
examined 424 observed placement transitions; every observed lower bound fit
the expanded *synthetic* category maxima. In 85 cases, positive allocated
growth exceeded the fixture's own write-charge counter; the largest shortfall
was 86,016 bytes. The cross-check therefore uses the larger observed lower
bound and does not treat missing journal/receipt/source/liveness measurements
as zero. Three accepted-unsaved groups plus compaction in its cumulative trace
required a 260,673,920-byte peak against a complete old keep-set input of
150,000,000 bytes. Twenty-eight pre/post-effect fault cases retained liability
and withheld `Saved`. None of this gives the process exclusive future APFS
space on a shared user-selected volume. A later ENOSPC must preserve exact
evidence and refuse/reconcile, not claim `Saved` or delete old content.

## macOS qualification evidence and boundary

On macOS 27.0 build 26A428, local APFS tests passed the earlier 344-scenario
subprocess barrier matrix, plus 93 ENOSPC/EIO/unknown boundary injections,
namespace/advisory-lock substitution cases and fail-closed path-classification
tests. They exercised real file/directory `fsync` and `F_FULLFSYNC` calls,
same-volume publication and independent old/candidate validation. A new
three-process prepare/publish/verify harness passed seven cases and applies
candidate barriers even to the synthetic base/new ancestors. It does **not**
invoke a mount tool or infer remount/power-loss from separate processes.

The installed macOS 27 SDK `fcntl(2)` manual describes APFS `F_FULLFSYNC` and
the barrier for earlier `fsync` calls on the same device. Apple's
[published fsync manual](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/fsync.2.html)
warns that ordinary `fsync` does not by itself force drive buffers in order;
Apple's [disk-write guidance](https://developer.apple.com/documentation/xcode/reducing-disk-writes)
also cautions that full flushes are costly and describes best-effort behavior
in its iOS discussion. Successful syscalls on this Mac do not establish the
entire fresh-ancestor/object/HEAD namespace protocol or physical power-loss
behavior. Local APFS/mount flags and public File Provider/iCloud queries also
cannot prove that an arbitrary user-selected path is not provider-managed.
The version-bound dry-run classifier refuses missing provider/volume/mount/
device evidence and invalidates on remount, rebind or namespace substitution.

One uniquely scoped 128 MiB APFS sparse-image creation attempt failed with
`Device not configured` before any image/mount existed; there was no escalation
or alternate route. The split-phase harness is ready for a **designated,
disposable** remount environment, but actual clean remount and abrupt
kernel/storage/power cuts are unexecuted. Clean detach/remount may itself flush
pending writes and cannot substitute for abrupt-power evidence. No production
macOS storage profile or `Saved` authorization is qualified yet.

## Remaining pre-permanent and storage-qualification gates

The combined disposable fixture passed 34 release example tests, including
15 independently authored actual-v3 adversarial tests; strict Clippy,
formatting and whitespace checks passed. That establishes the fixture's
bounded algorithmic and process-interruption behavior, not production
durability or a permanent wire.

Before permanent implementation, the physical capacity ledger needs exact
per-domain ownership and allocation-credit provenance. A pack can leave the
*namespace keep-set* only after both selected roots and every extra pin cease
to reference its exact generation, and after retirement/barrier/reopen proof.
That must not be conflated with APFS free blocks: snapshots, clones, quotas and
other writers may prevent physical space from becoming available. A qualified
fresh free-space/quota observation and a safety margin must precede each new
admission; a late ENOSPC preserves old state and original liability. The
current no-credit high-water ledger is safe but can over-refuse indefinitely,
so it is not a complete long-lived capacity/progress solution.

The pre-freeze codec gate must exercise genuine canonical receipts, authored
before/after and no-op semantics, accepted journal groups, and matching
`Saved` barriers rather than the v3 synthetic receipt. Shared Rust owner/lease
coordination, multi-surface queueing, reader completion proof, qualified
macOS remount/abrupt-power and provider/path admission, and long-term
compaction scheduling remain separate. No live package writes, migration,
production GC/Asset Store, project switching or production latency claim is
authorized by this report.
