# PS2 actual-v3 self-owned suffix fixture

Disposable engineering evidence, 2026-09-27; no permanent wire or production
authorization. This extends the [automatic prefix batch](PS2_AUTOMATIC_PREFIX_BATCH.md)
to cover the allocation extents created by its own data and locator metadata.

A claim now distinguishes the strongly hashed original `content_end` prefix
from its owned allocation `extent`. A bounded fixed-point planner updates both
active pack extents until encoding the ownership nodes and locator paths no
longer changes those extents. Existing device/inode bindings and original
prefix hashes remain exact. Both active and recovery ownership trees select
the claims through the existing locator. Original-token replay regenerates
the exact plan, verifies final physical extents, and only then publishes.

Allocation ownership is distinct from a content promise. Original prefix bytes
retain their digest; every selected semantic object, ownership node and locator
page retains its exact content commitment and closure validation. Intermediate
superseded ownership frames are physical garbage covered by the allocation
extent, with no promise to reconstruct their contents. An explicit test mutates
one such unreachable frame and confirms this distinction; separate tests reject
original-prefix and selected-suffix corruption. No selected content is exempted
from the existing audit.

This fixture refuses pack rollover before effects because a fresh generation
has no prebound inode. It retains the 48 KiB recipe cap and a 16-round planning
limit. Both comparison maps refuse a 1,000-operation automatic batch without
effects at the cap; a 512-operation automatic batch verifies more than two
claims. It does not provide scalable lifetime bootstrap or fresh-pack binding.

The complete regression suite passed 62 tests, followed by the separately added
allocation/content-boundary test passing. Tests include original-token partial,
pre-HEAD and post-HEAD recovery, fresh-handle full audit after recipe completion,
missing-arena refusal, exact selected extents and no-effect rollover/cap refusal.

The [two measured rows](verification/ps2-self-coverage.jsonl) use:

```sh
cargo run --release -p photara-store --example ps2_index_furnace -- placement-v3 typed-inventory recipe self-covered 128
```

The radix/B-tree recipes are 16,669/16,630 B, write 4,320/4,319 B of data and
locator payload and 115,341/115,107 B of control envelopes, with 28 fixture sync
calls each. Holds remain approximately 3.25 MB. These are negative practical
amplification measurements, not application latency or qualified barriers.

## Original-token allocation settlement

The subsequent accounting integration records original and current observed
charges separately for each touched pack. After exact suffix replay it checks
the original device/inode, final extent and all recipe bytes, then consumes
only the nonnegative observed allocation delta plus the existing conservative
control/completion allowance. Shrinking or compression gives no credit.
Replaying a completed token cannot charge it again.

Control observations cover the finite `HEAD`, `intent`, `candidate` and
`HEAD.next` roles. Permitted full selectors are reconstructed from the original
pre-effect selection and hold, including domains, charges, pins and token
sequence. Matching roots or token alone cannot admit changed control state.
Unknown selectors, replaced packs, corrupt suffixes and failed settlement keep
the original hold. This is control observation and conservative accounting,
not a durable per-generation control ledger or retirement authorization.

The refreshed [settlement evidence](verification/ps2-allocation-settlement.jsonl)
records 17,495/17,409 B radix/B-tree recipes and 0/4,096 B observed pack growth.
Zero growth means the append fit previously allocated blocks. Each observed
live HEAD used 20,480 B; the conservative completion reserve was 262,144 B.
Centralized counters now include all prefix verification and planning passes:
1,367,344/1,285,032 B of prefix reads and 4,318 B of suffix verification each.
These supersede the earlier rows' partial prefix-read accounting. The scoped
typed-inventory suite passed 29 tests, including four accounting tests covering
both maps, five cuts, selector substitution, replaced inode and late corruption.

Ownership replacement additionally refuses reduced prefix coverage, changed
equal-length prefix commitment and unsealing an immutable generation. It checks
the old digest before recapturing a growing prefix so corruption cannot be
blessed by a new hash.

Standing control accounting still accumulates conservatively per transition.
Per-generation refundable charge provenance, fresh-pack enrollment, typed claim
release/retirement and storage qualification remain gates. The separate
[Graph original-token integration](PS2_GRAPH_ORIGINAL_TOKEN_PACKED.md) now shares
this replay machinery, but composing Graph publication with ownership self-coverage
remains open. Ordinary semantic checkpoints after this ownership publication
can create further uncovered suffixes; the measured row does not claim permanent
automatic coverage. No observed allocation delta is filesystem free-space credit.
