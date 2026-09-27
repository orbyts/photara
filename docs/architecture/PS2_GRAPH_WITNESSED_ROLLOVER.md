# PS2 witnessed Graph rollover and exact charge finalization

Status: disposable engineering evidence, 2026-09-27. This extends the
[same-tip joint Graph fixture](PS2_GRAPH_JOINT_OWNERSHIP.md). It does not qualify
production Accepted/Saved, freeze a permanent wire, or change production code.

## Integrated lifecycle

The actual Graph dispatcher now composes the original command journal with
finite witnessed births, packed payload replay, exact sealed-allocation charge
records, ownership finalization and atomic ledger transfer. Both map variants
use their existing command model, receipt index and packed locator implementation.
The source remains a retained disposable builder with its own surrogate HEAD;
the physical package HEAD selects the authoritative fixture checkpoint.

Admission performs an in-memory Graph preview and semantic payload plan. It sizes
ownership finalization with maximum-width numeric fields and the exact changed
allocation keys and tree shape. Four finite choices reuse the payload tips or
reserve additional finalization tips. A separate explicit dedicated-corridor
mode reserves a new final tip in each arena. It supplies useful created-and-sealed
intermediate-generation coverage at deliberately higher allocation cost.
Sizing values never become persisted allocation charges. The actual finalizer
must fit the admitted corridor and cannot create another generation.

Before any journal acknowledgement boundary, the same token admits the complete
source/journal work, physical generations, finalization and cleanup reserve.
Its immutable bound R never expands. Birth uses the separately tested nonce-bound
marker and selected inode witness path, including exact private namespace,
no-replace promotion, and refusal of unbound partial markers. Every fresh
physical generation must complete enrollment before journal or source effects,
including on recovery. Byte-identical inode replacement is rejected.

After the journal and source barriers, witnessed replay writes the semantic
objects and preliminary locator pages. These roots remain unselected. Every
payload pack passes its barrier before newly sealed generations are measured.
The same token then selects actual sealed charges and an exact, regenerable
finalizer commitment before finalizer bytes are written. Both final ownership
closures include the generated suffixes. Their prefix authentication and owned
allocation extents remain distinct.

Finalizer replay is restricted to the two admitted final tips. Publication
verifies its complete original prefix and suffix bytes, measures tip/source
high-water growth, and atomically selects the final Graph roots, ownership roots,
sealed charges, tip registry and project charge increment C. The selected hold
still carries immutable R, while its outstanding reservation becomes R−C and
retains the original cleanup allowance. Thus publication does not count the
same growth as both committed charge and the full original hold. Cleanup removes
the remaining hold without charging C again. No source retirement or filesystem
availability credit is introduced.

Pinned historical growable claims resolve to the same canonical allocation even
when that allocation is now sealed. The exact attribution audit counts the
allocation once; the retained pin still prevents its retirement. Current-tip
growth under an older same-tip recipe/ticket is accepted by the audit only with
matching selected continuation, original control provenance, and observed growth
within its allocation reserve. Generic pin acquisition/release or new admission
cannot change a pending original-bound operation's selector.

## Recovery and validation

The eight main rollover tests exercise both maps and cover:

- data, metadata and simultaneous rollover, including born-and-sealed
  intermediate generations and historical growable pins;
- twenty-one physical phase cuts: before/staged/after HEAD at payload binding,
  finalization binding, Graph/charge publication and cleanup; partial payload
  and finalizer frames; and intervening payload/finalizer barriers;
- ten journal/source/cleanup cuts under the new rollover hold, including partial
  journal and source bytes, candidate/marker publication and cleanup unlinks;
- interrupted known HEAD.next cleanup retaining intent/candidate evidence;
- actual joint birth recovery with immutable admission tampering and an unbound
  partial marker that cannot create a journal;
- coherent consumed-charge/ledger alteration, substituted witnessed files,
  unknown controls and insufficient capacity with no effects.

The [six independent review tests](verification/ps2-graph-rollover-review-tests.log)
add complete file/inode/byte snapshots around rejected generic gate mutations,
changed no-intent selectors, stale historical groups, foreign journal artifacts,
retained source replacement, and pending first-binding inode substitution.
The [expanded seven-test run](verification/ps2-graph-rollover-review-seven-tests.log)
also corrupts an existing source suffix while a physical selector dispatch is
pending. Recovery validates that suffix before cleaning any dispatch artifacts.
Every refusal happens before unauthorized journal/source repair or removal of
original dispatch evidence. Strict private, bounded, no-follow control reads
reject noncanonical selector encodings and unknown fields. Unknown files remain
untouched. Known staged selectors are removed only after exact validation and
a directory barrier, with a separate interruption test.

These are bounded controlled-return and fresh-handle tests. They do not establish
arbitrary syscall-cut, abrupt process-kill, remount or power-loss qualification.
Full closure and Graph replay audits occur after measured clean completion;
interrupted fixture reconciliation may also run the existing exhaustive audit.
The completion path itself verifies original recipes, witnesses, receipts and
allocation transfer. No external-media reads are introduced.

## Release observations

The command `placement-v3 graph-journal rollover 8` measures one eight-operation
group after a bulk eight-operation Graph baseline. Explicit framed padding makes
one or both original tips nearly full before attributable bootstrap. Baseline,
padding and initial source enrollment are outside the measured spans. Both maps
are still small enough that matching write counts are unsurprising; these rows
do not determine an index winner or establish scaling behavior.

All byte columns are successful process writes, not device traffic. Packed
writes include data, metadata, every selected control generation and birth
marker writes. Journal includes group/candidate/marker. Source includes retained
objects and source HEAD. The original conservative admission includes a 16 MiB
source-work allowance. Standing package controls are 1,163,264 B, separately from
the source's 1,114,112 B standing scratch charge and its retained actual pack
high-water. Neither standing pool changes during these measured groups.

| Map | Padded arena | Corridor | Fresh packs | Immutable R B | Committed C B | Packed writes B | All writes B | Sync calls |
|---|---|---|---:|---:|---:|---:|---:|---:|
| Radix | data | natural | 1 | 19,939,328 | 172,032 | 887,581 | 981,478 | 95 |
| Radix | data | dedicated | 3 | 20,529,152 | 118,784 | 1,615,201 | 1,709,098 | 157 |
| Radix | metadata | natural | 1 | 19,939,328 | 114,688 | 887,487 | 981,384 | 95 |
| Radix | metadata | dedicated | 3 | 20,529,152 | 122,880 | 1,615,161 | 1,709,058 | 157 |
| Radix | both | natural | 2 | 20,234,240 | 114,688 | 1,251,473 | 1,345,370 | 127 |
| Radix | both | dedicated | 4 | 20,824,064 | 126,976 | 1,983,833 | 2,077,730 | 189 |
| Btree | data | natural | 1 | 19,939,328 | 110,592 | 887,581 | 981,478 | 95 |
| Btree | data | dedicated | 3 | 20,529,152 | 118,784 | 1,615,201 | 1,709,098 | 157 |
| Btree | metadata | natural | 1 | 19,939,328 | 114,688 | 887,487 | 981,384 | 95 |
| Btree | metadata | dedicated | 3 | 20,529,152 | 184,320 | 1,615,161 | 1,709,058 | 157 |
| Btree | both | natural | 2 | 20,234,240 | 114,688 | 1,251,473 | 1,345,370 | 127 |
| Btree | both | dedicated | 4 | 20,824,064 | 249,856 | 1,983,833 | 2,077,730 | 189 |

Every row additionally writes 66,144 B through retained source staging and
27,753 B through journal files. Total writes are approximately 123–260 kB per
operation in this small forced-rollover run. Sync totals include the separately
counted source calls and eleven journal syncs on the exact clean path. Measured
planning/admission/completion sums range 430–1,021 ms; these are single observations
on a shared development machine, not latency distributions or qualified Saved
latency. Group formation, UI scheduling and final exhaustive audits are excluded.

Observed C can differ for identical process writes because filesystem allocation
and preallocation differ. Larger observed charges in several rows
are retained rather than normalized away. Recorded high-water charges are never
lowered through allocation shrink or optimistic free-space assumptions.

Birth phase controls, duplicated original commitments, staged finalization and
95–189 sync calls are substantial negative practical evidence. This proves a
bounded lifecycle composition, not an efficient production storage design.
Runtime relocation/retirement and fixed-capacity turnover have separate tests and
measurements; their outcomes must not be inferred from these rollover rows.

## Reproduction and provenance

```sh
cargo test --release -p photara-store --example ps2_index_furnace real_graph_rollover
cargo test --release -p photara-store --example ps2_index_furnace review_rollover_
cargo run --release -p photara-store --example ps2_index_furnace -- placement-v3 graph-journal rollover 8 > docs/architecture/verification/ps2-graph-witnessed-rollover.jsonl
```

[ps2-graph-witnessed-rollover.jsonl](verification/ps2-graph-witnessed-rollover.jsonl): SHA-256 `d7e34b53924af03a3543ebb176cbad179a834415b1dd17ad6da13d1e2371699e`.

[ps2-graph-rollover-review-tests.log](verification/ps2-graph-rollover-review-tests.log): SHA-256 `c7910dd2846e4c8c09dd89439622d622fe4a8c830d97ee1d415aca450b652724`.

[ps2-graph-rollover-review-seven-tests.log](verification/ps2-graph-rollover-review-seven-tests.log): SHA-256 `442c21b3ed9840fb363409fff5a28527972e6e9503d3de9ff7d3f25d4979c834`.

[Measurement source hashes](verification/ps2-graph-witnessed-rollover-source-sha256.txt)
record the final recovery guard and compiled fixture sources. The historical
same-tip and legacy Graph measurement files remain unchanged.
The relocation child received only Rustfmt test-signature whitespace after
these measurements; the manifest records that formatted checkpoint source.

The checked-in review logs omit the command output's final empty line.
