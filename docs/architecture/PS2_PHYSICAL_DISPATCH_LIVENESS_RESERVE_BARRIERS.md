# PS2 physical dispatch, liveness, reserve and macOS barrier review

Status: disposable design/verification gate complete, with a negative physical
layout result. The logical radix sequence/map remains a conditional write-heavy
lead; the paged B-tree remains the comparison. Neither logical nor physical
wire is frozen. This review does not authorize a production codec, live writer,
migration, GC, Asset Store, project switching or production latency claim.

## Contract that must not move

- Speculative preview is never committed.
- `Accepted` requires the exact finite journal group and original IDs to pass
  its qualified durability barrier.
- `Saved` requires the selected package checkpoint/HEAD and original receipt
  to be durably verified for the **current** accepted authored revision/digest.
- `HEAD.json` remains the sole *semantic* atomic dispatch. Physical relocation
  cannot change semantic object identity, authored state, operation identity or
  retention obligation, and active/recovery roots remain independently valid.
- Ordinary open, autosave, root turnover and metadata validation never read or
  strongly hash large external managed media.

## Decision under test: physical dispatch

The prior fixture's per-segment placement pointer was an additional physical
selector. Its uncached two-read lookup amplified full-audit and retry costs;
the 100k case supplied negative evidence, not a production choice. Two
disposable alternatives need an exact comparison:

1. A bounded generation-safe cache over per-segment placement tables. The
   cache key must bind package/volume/owner epoch, segment and selected
   generation; no stale table can survive a relocation or reopened session.
   A cache hit cannot imply the selected pointer itself is still valid after
   an unobserved external change. Uncertain pointer publication freezes
   admission and reconciles both generations.
2. A HEAD-selected persistent locator root separate from semantic object IDs.
   Its bootstrap pages require direct, nonrecursive physical addressing (for
   example immutable hashed metadata objects), or resolving the locator would
   depend on itself. A physical-only root-set commit may advance package
   revision while leaving authored revision/digest unchanged. The old
   recovery locator must remain readable after selection; old physical bytes
   cannot be retired merely because the new locator was selected. This avoids
   a second selector but changes the candidate root-set contract and leaves
   locator-metadata retirement to prove.

Neither is approved wire. The comparison must measure lookup path/bytes with
and without cache, cache invalidation under process restart and generation
change, physical-only turnover, recovery-root access, locator metadata growth,
and interruptions before/after the selecting atomic step.

## Persistent liveness, without foreground lifetime scans

Liveness must include active, recovery, explicit pins, conversion source,
accepted-but-uncheckpointed journal and unresolved publication/compaction
intents. It is **conservative**: a false live count may delay reclaim; a false
dead count is a data-loss bug. A persistent reference/edge ledger plus durable
bounded retirement work queue is one candidate. New dependencies become
protected before they can be selected; removal of old protection is processed
only after the new selection and its qualified barriers. Queue replay is
idempotent by original work ID and epoch. A root removal may trigger a large
descendant cascade, but normal checkpoint admission and compaction selection
must do only bounded work; remaining cascade is background debt whose bytes
stay charged and unavailable for reuse. Imported/unaudited packages require a
separate full liveness bootstrap audit before writable admission.

The fixture must prove no undercount across interrupted increment/decrement,
shared subtrees, two independently selected roots, retained pins, partial
relocation and recovery. A compaction decision reads the persistent segment
summary plus bounded verification of its own candidate segment, never a
lifetime walk. Full integrity audit remains explicit and may be linear.

## Physical reserve proof to attempt

Before admitting a finite journal group, charge physical allocated bytes and
unresolved reservations **per qualified capacity domain**. Compute a
worst-case bound for framed journal+original receipts, all changed index and
inventory paths, authored objects, pack/locator/metadata allocation units,
candidate intent/HEAD/receipt and directory entries, selected recovery and
pins, plus any already admitted compaction copy. Every bound must be based on
checked maximum request/page/segment/path sizes, not an average benchmark.
Pending journal-accepted but not package-Saved work keeps its checkpoint
liability. Compaction reserves new bytes and both old/new locator generations
before copying; it never spends anticipated deletion credit. Unknown outcomes
hold the charge until exact reconciliation. An OS free-space check alone is
advisory: concurrent consumers and quota changes remain modeled ENOSPC paths.
On a user-selected shared APFS volume, a numeric ledger cannot exclusively
reserve the filesystem's future free blocks or every copy-on-write/namespace
allocation. Unless the selected platform/profile offers a proven physical
reservation mechanism for every required category, the honest guarantee is
conservative internal admission plus **fail-safe refusal/reconciliation** on
later ENOSPC, not unconditional forward progress from `Accepted` to `Saved`.
This distinction may require an explicit qualification/product decision;
it cannot be erased by rounding `st_blocks` or claiming a safety margin as
an absolute proof.

The review should show a ledger trace for batch admission, checkpoint,
relocation, interruption and recovery, including peak reserved/allocated
bytes, overflow refusal and inability to erase active/recovery content to
make room. The previous fixture's `RESERVE` constant was only a synthetic cap.

## Qualified macOS barrier fault matrix

The [local-APFS qualification plan](PS2_MACOS_STORAGE_QUALIFICATION.md)
requires real full-file and directory barrier observations plus injected
pre-effect, post-effect, ambiguous and process-exit cuts at every publication
boundary. Apple's [fsync manual](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/fsync.2.html)
explains why ordinary `fsync` alone does not establish drive-buffer ordering.
The installed macOS manual documents `F_FULLFSYNC` for APFS and same-device
previously-fsynced data, but successful calls on one machine still do not
prove the entire namespace/firmware/power-loss protocol. The fixture must
record OS build, APFS/local mount and device facts, operation trace, observed
HEAD, independently readable old/candidate closure, original intent/receipt,
cleanup inventory and acknowledgment for every cut. Unsupported file or
directory barrier must refuse the candidate profile, not silently downgrade
to `sync_all` or claim `Saved`.

| Candidate phase | Barrier dependency that must be demonstrated | Potential grouping, subject to qualification |
| --- | --- | --- |
| Existing journal group | Full persistence of exact appended frames, original IDs and inclusion prefix before `Accepted` | One barrier per finite group, not one per operation; a newly created journal also needs namespace persistence |
| Changed immutable objects/index/locator pages | File bytes and newly published names persist before HEAD may refer to them | Multiple pages in one already-owned pack can share a file barrier; unchanged files need none |
| Candidate intent and reserve evidence | Intent must survive before any selecting HEAD replacement | No grouping *across* this ordering edge unless a documented same-device barrier proves it |
| HEAD temporary and replacement | Complete HEAD temporary bytes, atomic same-volume replacement and root directory entry persist | Previously fsynced same-device data may be covered by a later documented full-device barrier only under an exact qualified profile |
| Original checkpoint receipt | Durable original-ID receipt and current authored revision/digest after selected HEAD validation | May cover several accepted operations, but cannot precede the selected HEAD barrier or be inferred from memory |

Every row has an injected before-effect, after-effect, lost-completion and
child-exit case; filesystem state observed after restart is classified as
exact old, exact candidate or refusal/unknown. This matrix tests the *program
ordering and reconciliation logic*. It cannot reproduce kernel crash, drive
cache loss, firmware lies, quota races or provider behavior on the developer's
ordinary Mac, so it cannot by itself qualify a production profile.

No production profile is qualified by deterministic syscall cuts alone.
Hardware power interruption, remount/provider behavior and uncooperative
writers remain separate qualification/authorization gates.

### Disposable local-APFS observations

The reproducible, explicitly ignored macOS test is
`cargo test -p photara-store --test package_v1_1 macos_barrier_faults --
--include-ignored --nocapture`. It ran on macOS 27.0 (26A428), local APFS,
with a fresh synthetic package and independent writer subprocess for each
scenario. Forty-nine ordered boundaries times seven pre-effect/post-effect/
unknown/process-exit cuts, plus a successful control, produced **344 passing
scenarios**: 284 reopened the exact old HEAD, 60 reopened the exact candidate
HEAD. The independent v1.1 opener validated the selected complete closure,
unchanged original non-HEAD bytes, retained writes, unchanged root identity and
an untouched outside-package sentinel. Exactly one *synthetic*, explicitly
unqualified acknowledgement occurred, on the successful control. The trace
observed 2,994 successful `fsync` and 2,875 successful `F_FULLFSYNC` calls,
with no actual syscall errors on this machine. Separate tests froze unrelated
valid and truncated HEAD bytes without mutating the fixture.

The 3.8 MiB raw process trace is
`/private/tmp/photara-ps2-barrier-matrix-final.log` (SHA-256
`5c4699f786624d7eb13f108adf671c2c487fa286171979c14350ebb3a073a299`).
The normal v1.1 suite passed 106 tests with four ignored; strict Clippy and
whitespace checks passed. These numbers test process-visible syscall ordering,
not hardware failure. The fixture's original package materialization, trace
logging, provider exclusion, directory power-loss ordering, adversarial
namespace replacement and a production writer remain unqualified. The test
must never be used as a positive production `Saved` authorization.

## Disposable physical fixture and measured results

`crates/photara-store/examples/ps2_index_furnace/placement_gate.rs` adds a
separate experimental mode over both logical alternatives. It uses a
HEAD-bound active/recovery locator pair, direct-offset+digest metadata
bootstrap arena, immutable 16-nibble persistent metadata trie, 32-entry
locator cache keyed by the exact locator-root digest, persisted object incoming
counts and live-block summaries, and a durable retirement queue processed in
at most 32 steps per selection. A physical-only selection leaves semantic
object IDs and authored state unchanged. The old physical copy survives the
first selection and may retire only after a second selection drops its recovery
locator. Full liveness/placement audit is explicit; an unaudited reopened
fixture is read-only until that audit succeeds. Candidate inspection touches
one segment, not the entire lifetime history.

All six real-file fixtures at 1k, 10k and 100k accepted operations, radix and
B-tree, were fully materialized. The 100k run completed in 58.50 seconds on
this developer Mac. The figures below describe **four physical-only
publications** sampled per alternative/size, following separately measured
semantic source commits; they are not end-to-end autosave latency. Times are
process/file-cache observations, not power-loss or production claims.

| Index / accepted operations | Candidate locator pages | Cold root locator pages | 32 warm reads: locator pages | Metadata written per publication | Tail bytes copied per publication | One-segment compaction copy |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Radix / 1k | 17 | 17 | 0 | 124–148 KB | 623–659 KB | 659 KB |
| B-tree / 1k | 17 | 17 | 0 | 161–178 KB | 610–651 KB | 647 KB |
| Radix / 10k | 17 | 17 | 0 | 146–158 KB | 946–975 KB | 1,049 KB |
| B-tree / 10k | 17 | 17 | 0 | 199–229 KB | 758–799 KB | 1,049 KB |
| Radix / 100k | 17 | 17 | 0 | 222–233 KB | 250–287 KB | 1,049 KB |
| B-tree / 100k | 17 | 17 | 0 | 295–330 KB | 471–524 KB | 1,049 KB |

At 100k, radix physical publication wrote 36.8–40.9 times the new logical
source-append bytes in metadata plus copied tail; B-tree wrote 41.2–48.5
times. At 10k, radix's segment alignment made that amplification 97.5–109.3
times. The bound is finite but unacceptable for responsive continuous
authoring. Constant-depth lookup does **not** compensate for these writes.
Candidate inspection took 113–179 microseconds across all six cases; cold
root-object placement lookup took 117–182 microseconds, and a fresh active/
recovery root-object placement/hash probe took 259–391 microseconds. These are
not complete project structural-open measurements, nor was the OS cache
evicted. The fixture's 100k gate directory reached about 180.6 MB allocated
for radix and 179.7 MB for B-tree, in addition to its temporary semantic
source fixture; persistent metadata and old generations lack a complete
bounded reclaim path.

The raw six-row JSONL, including each publication's bytes, reserve charges,
sync counts, liveness queue steps, interruption cases and root-pair probes, is
[`verification/ps2-physical-dispatch-liveness-reserve.jsonl`](verification/ps2-physical-dispatch-liveness-reserve.jsonl).
The benchmark command was `ps2_index_furnace placement-gate 1000 10000
100000`. Eight release example tests (including corrupt metadata/forged count
refusal and unknown-selection/reserve exhaustion), strict Clippy and Rustfmt
checks passed.

### What remains unproved

This fixture shows bounded path depth, candidate selection and queue step
count, but not a suitable physical format. The direct-reference metadata arena
and superseded tail generations have no bounded reclamation. The 32-step
retirement queue can defer work safely, but its remaining bytes are charged;
the fixture only models active/recovery roots plus queued edge pins, not undo,
export, concurrent reader leases or all resource obligations. The fixture's
`HEAD.pending` overwrite is not a production staging ownership/cleanup
protocol. Reopening a root-pair probe is not a complete structural or full
integrity audit. The persisted liability and allocation-unit ledger demonstrate
refusal/retention under injected exhaustion, not exclusive OS space
reservation, integrated journal admission or guaranteed progress after ENOSPC.
No successful `fsync`/`F_FULLFSYNC` trace supplies the absent directory and
power-loss qualification by itself.

**Recommendation:** do not freeze this HEAD-bound locator/trie/tail-copy
layout or the prior uncached per-segment pointer. Preserve stable semantic IDs
and the sole-semantic-HEAD rule; next compare a bounded append-unit/pack
directory or equivalent non-tail-copy placement with a reclaimable metadata
bootstrap, all pin classes and reader leases, and exact per-capacity-domain
admission liabilities. Compaction should use persistent conservative liveness
and bounded candidate inspection, but may not delete on the strength of this
fixture. A shared user-selected APFS volume cannot promise unconditional
future `Saved` progress from a numeric free-space ledger alone. If a product
promise requires that stronger guarantee, the storage profile or reservation
model needs separate explicit review. No approved `Accepted`/`Saved` semantic
is weakened by this negative result.
