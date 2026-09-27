# PS2 automatic original-prefix batch prerequisite

Disposable actual-v3 integration evidence, 2026-09-27. This extends the typed
inventory and original-token recipe; it does not freeze wire or qualify storage,
production `Accepted`/`Saved`, live packages, migration or project switching.

The prior recipe accepted one manually selected allocation claim. The fixture
now captures bounded batches of current data and locator pack prefixes and
plans both selected ownership trees together. The active and recovery semantic
closures remain exact and independent. Changed ownership paths share new
immutable nodes inside one batch; superseded intermediate nodes are omitted
from the locator closure, rather than accepted as extra semantic or ownership
leaves. The recipe carries the whole exact claim batch and regenerates its
original plan during replay under the original liability token.

Each claim retains device/inode, prefix extent/digest, sealed status and the
observed `max(st_blocks * 512, length)` regular-file allocation charge. This is
local allocation provenance only. It does not replace the existing conservative
high-water admission ledger, establish available filesystem space, or authorize
credit. Duplicate units, unknown active suffixes and bounded exhaustion refuse
before publication. A bootstrap helper enumerates at most 64 existing packs;
a separate touched-unit helper supports bounded incremental capture. Routine
structural open does not use either bootstrap enumeration or full prefix audit.

Four added adversarial tests cover both comparison maps and partial/data/
pre-HEAD/post-HEAD cuts, exact original-token completion without duplicated
payload, both selected typed closures, growing prefixes while a reader retains
the previous ownership root, corrupt locator-prefix refusal retaining liability,
and no-effect duplicate/budget/unknown-suffix refusal. The complete release
furnace passed **57 tests, zero failures, in 45.29 seconds** at this foundation
stage. Later integration may extend that suite.

## Measurements and practical limit

The command below produced the two successful 128-operation rows, then exited
with `recipe byte admission cap` on the first 1,000-operation automatic batch.
The 48 KiB bound remains unchanged. Failure is negative scaling evidence: this
bounded bootstrap is not a scalable all-history ownership initialization path.
The command overlapped regression tests, so timings are not application latency
or an isolated comparison.

| 128 original operations | Radix | B-tree |
| --- | ---: | ---: |
| Automatically captured original pack prefixes | 2 | 2 |
| Observed original pack charge | 217,088 B | 167,936 B |
| Encoded original recipe | 16,437 B | 16,471 B |
| Data + locator payload writes | 4,279 B | 4,278 B |
| Control envelope writes | 113,949 B | 114,153 B |
| Fixture sync calls | 28 | 28 |

The approximately 3.25 MB original holds and large control amplification remain
conservative negative practical evidence. No barrier was removed and no future
retirement credit was taken to improve the measurement.

```sh
cargo test --release -p photara-store --example ps2_index_furnace automatic_
cargo test --release -p photara-store --example ps2_index_furnace
cargo run --release -p photara-store --example ps2_index_furnace -- placement-v3 typed-inventory recipe automatic 128 1000
```

The [versioned compact record](verification/ps2-automatic-prefix-batch.jsonl)
retains both complete successful output rows and the failed command's scope.
Temporary logs are `/private/tmp/ps2-automatic-prefix-tests.log` and
`/private/tmp/ps2-automatic-prefix.log`.

## Remaining integration

This foundation captures **original base prefixes**, including locator packs.
It does not by itself claim selected ownership of the new ownership/locator
suffixes created by its own recipe, complete control-generation accounting,
post-retirement charge release, or genuine Graph/journal integration. The recipe
covers those candidate writes while the original token is pending, but the
foundation does not preserve a complete ownership witness after completion.
That distinction motivates the next self-coverage integration; an enclosing
file cannot naively store its own full-content digest. Allocation coverage must
remain separate from semantic/content commitments without weakening either
exact typed closure or corruption refusal.
