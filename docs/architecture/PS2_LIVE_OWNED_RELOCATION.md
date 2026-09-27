# PS2 disposable same-tip live-owned relocation

This slice composes actual v3 relocation with the existing typed retirement ticket. It is confined to examples, one sealed source generation, and the already enrolled current data/metadata tips. It does not implement runtime rollover, production deletion, portable device/inode rebinding, permanent encoding, or qualified Saved.

## Original-token path

The bounded planner validates the exact sealed ownership claim and attributable charge, refuses every extra pin retaining its ownership or physical placement, and refuses unrelated unresolved work. It scans one bounded source pack. Data relocation copies selected semantic **and ownership** records; copied logical references and bytes remain unchanged. Ownership claim removal and tip updates override superseded copied ownership keys, preserving exact locator closure. Metadata relocation rewrites selected source locator pages after the ownership changes. Active and recovery are independently derived from their respective original roots.

A bounded fixed point covers every new current-tip extent. The unchanged 48 KiB ticket cap, same-tip constraint, and complete capacity hold are checked before effects. The original ticket commits the source, original prefixes, full typed write plans, final locators/inventories, tip claims, and complete control base. Replay regenerates and compares the original plans; partial data and metadata frames use the existing original-prefix recipe replay.

One publication selects both rewritten roots and removes both current source ownership edges. The exact source charge remains represented by the original ticket, and the original capacity hold covers new allocations until settlement. Complete selected closure is audited before releasing the fallback source. Existing selected unlink authorization, exact-generation checks, unlink, directory barrier, fresh absence, and once-only project credit are reused. Growth is observed using the existing per-tip high-water ledger; no filesystem availability credit is issued. All logical roots and authored receipt state remain unchanged.

`relocate_owned(f, data, pack)` applies this path to an existing attributed fixture. It does not construct or enroll a source. The CLI calls the same helper after constructing its expressly synthetic frozen physical baseline.

## Bounded ticket experiment

Only the two `RetirementTicket` plan fields use a compact tagged UTF-8 representation for their already-JSON record bodies. This avoids decimal byte-array amplification without raising the admission cap. Decoding restores exact bytes and checks body length/hash, record size, homogeneous arena frames, physical continuity, plan endpoint, and total byte count. Encoded fields remain bounded by the existing control-frame cap; the complete ticket still must fit 48 KiB before admission.

Legacy byte-array plan decoding is accepted. This is **decode compatibility only**: previously selected authorization/credit hashes commit the old whole-ticket serialization, so an already-authorized fixture from an earlier code epoch may conservatively fence after re-encoding. No cross-version original-token retry or permanent wire compatibility is claimed. Ordinary recipe encoding is unchanged.

## Fixture and measurements

The measured physical baseline has eight semantic keys. It is constructed and frozen before attributable admission; no legacy aggregate is converted into refundable charge. A live ownership leaf describing data-0 resides in sealed data-1 alongside selected semantic records. Data-1's own descriptor resides outside that pack. All sealed charges and final growable tip charges are measured independently, then both roots and complete attribution are audited. This baseline is not represented as output of runtime rollover.

Command:

```sh
cargo run --release -p photara-store --example ps2_index_furnace -- placement-v3 typed-inventory relocation 0 128
```

Raw rows: [verification/ps2-live-owned-relocation.jsonl](verification/ps2-live-owned-relocation.jsonl). Both Btree and Radix produced these project-domain results on the current volume:

| Source | Dead ownership records in data-1 | Selected memberships across both roots | Exact source credit | Observed tip growth | Net project credit |
| --- | ---: | --- | ---: | ---: | ---: |
| data-1 | 0 | 2 semantic + 2 ownership | 4,096 | 12,288 | **−8,192** |
| data-1 | 128 | 2 semantic + 2 ownership | 45,056 | 12,288 | **32,768** |
| metadata-0 | 0 or 128 | 50 locator-page memberships | 12,288 | 20,480 | **−8,192** |

Membership counts include each selected root, not unique physical objects. The garbage parameter changes data-1, so it does not make metadata-0 retirement profitable. Negative outcomes are important overhead evidence, not a proposed product policy to force unprofitable retirement. Actual allocation observations are not filesystem availability measurements. Every row reports zero filesystem-space credit.

The full 64-key live-data candidate exceeds the unchanged 48 KiB ticket limit and is tested to refuse without changing HEAD, file extents, or holds. The eight-key result does not establish arbitrary-scale throughput or completion.

## Verification

Nine focused relocation tests passed, including both map implementations and source arenas, 24 original replay cuts (including partial metadata), exact staged-selector cleanup and its own cut, all ten extra pin classes with active/recovery independently relocated, unlink and credit cuts, capacity and unrelated-work refusal, selected-candidate corruption, and real recipe-cap refusal. Compact-codec tests cover Unicode/escapes, deterministic exact roundtrip, legacy arrays, altered hashes/lengths/continuity, oversized bodies, and unknown tags.

The existing `ledger_` regression filter passed 10 tests after the codec change, including original ticket credit cuts and Graph same-tip ledger recovery. The final integrated regression is recorded by the root task. An additional six-case test covers source replacement, broken symlink, and candidate ownership corruption before/after credit HEAD selection; retries refuse before deleting dispatch evidence or touching the replacement.


## Runtime-born sources

The separate runtime test creates a pack under Graph birth token 2, then seals that exact generation under the next Graph token 3. It exercises both maps and both arenas. All ten extra pin classes refuse relocation without effects; after releasing those pins, the same actual relocation helper preserves both original Graph receipts, logical roots, independent selected closures, and complete attribution.

Raw rows: [verification/ps2-runtime-born-relocation.jsonl](verification/ps2-runtime-born-relocation.jsonl). Data pack 2 contains 6 selected semantic and 8 ownership memberships across both roots: source credit 20,480 bytes, growth 28,672, net **−8,192**. Metadata pack 1 contains 19 selected locator memberships: source credit 12,288, growth 16,384, net **−4,096**. These are actual runtime-created and subsequently sealed generations; their negative net return is retained as overhead evidence.

The corrected four-case test passed with:

```sh
cargo test --release -p photara-store --example ps2_index_furnace turnover_runtime_born_sealed_relocation_preserves_original_graph_receipts -- --nocapture
```

## Fixed project capacity

The workload commits genuine groups of eight Graph records, retaining and validating every original receipt. Each limit is selected before the run as initial charge plus the first full original admission reserve plus a fixed extra allowance. Neither limit changes during its run. The configured ceiling is 128 groups; the test supports 64–256 groups and group sizes 8 or 32. It attempts the original garbage-rich data pack after the first group and a newly sealed data pack every eighth group. No future retirement credit enters admission.

Raw samples and every refusal: [verification/ps2-fixed-capacity-turnover.jsonl](verification/ps2-fixed-capacity-turnover.jsonl).

| Map | Fixed project limit | Completed groups × 8 | Final project charge | Next admission |
| --- | ---: | ---: | ---: | --- |
| Radix | 23,367,680 | 4 = 32 records | 3,166,208 | capacity refusal |
| Btree | 23,367,680 | 4 = 32 records | 3,166,208 | capacity refusal |
| Radix | 39,882,752 | 84 = 672 records | 19,681,280 | capacity refusal |
| Btree | 39,882,752 | 82 = 656 records | 19,677,184 | capacity refusal |

Initial charge is 2,576,384 bytes and the first full admission reserve is 20,529,152 bytes. Extra allowances are respectively 256 KiB and 16 MiB. Every attempted live maintenance operation refused the unchanged 48 KiB full-body recipe cap before effects. Thus this proves bounded runtime progress and exact no-effect refusal, **not repeated maintenance progress or a lifetime capacity plateau**. Immutable receipt/source growth remains separately charged. All runs issue zero filesystem availability credit.

The measurement command was `cargo test --release -p photara-store --example ps2_index_furnace turnover_ -- --nocapture`. Its fixed-capacity test and standing-control test passed, but the aggregate invocation reported one failure from an earlier runtime-born test that incorrectly assumed an arena's first new pack had already sealed. The corrected runtime-born test above passed separately. The wide measurement body was unchanged by that correction. The aggregate failure is not represented as a passing suite.

A compact deterministic plan-commitment mode is the next disposable experiment for the observed 48 KiB maintenance refusal. The unregistered draft is not part of these measurements and no permanent wire choice follows from it.
