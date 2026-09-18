# PS2: bounded typed write-recipe recovery prerequisite

Status: disposable actual-v3 experiment, based on `2b13d74`; not permanent wire, production storage, or an Accepted/Saved qualification. This is a deliberately smaller prerequisite, **not complete automatic ownership coverage or refundable capacity integration**.

## Result and scope

The actual typed-v3 locator can retain an exact, size-capped physical write recipe under its original HEAD-selected liability and resume a partially written framed record without duplicating payload or inventing a new hold. The recipe binds the typed claim, original data/metadata prefix identities and hashes, exact framed output, intended locator/inventory roots, and original semantic target. Recovery re-derives the typed changed-path plan and compares it exactly before appending only missing matching bytes. Existing mismatched bytes, non-regular destinations, unknown HEAD, and mismatched original holds refuse without deleting evidence.

The experiment retains one HEAD authority and independent semantic/ownership closures. The recipe is fixture control state, not an operation request field and not an adopted wire choice. Stable authored semantic identity does not change when this physical claim is published. Existing active/recovery and all-class pin behavior remains under the regression suite; this new path does not add a Graph journal or a receipt codec.

This unlocks a recoverable exact-output mechanism needed before automatic coverage can safely stage descriptors and locator metadata. It does not yet generate ownership claims for every active/sealed data pack, locator pack, control generation, or retired allocation. It does not implement exact refundable charge reconciliation. In particular, successful recipe completion conservatively consumes its full modeled hold into the inherited high-water ledger. That is safe refusal-oriented fixture accounting, not a practical final capacity policy.

## Bounded path versus audit

An initial implementation reused exhaustive continuation verification. Preserved negative evidence shows publication increasing from roughly 317–341 ms at 1k operations to 2.27–2.47 s at 10k, with lifetime-scale object/locator reads. This was rejected for ordinary publication.

The final trusted-local recipe path instead reconstructs only its admitted typed mutation and compares exact plans, reads at most the bounded original pack prefixes, validates occupied suffix bytes, and performs existing selector reconciliation. Prefix reads are counted separately; `data_reads: 0` does **not** mean no physical I/O. The observed 0.92–1.16 MB of internal prefix hashing is independent of lifetime count because packs are capped at 256 KiB. No external media is opened or hashed.

Normal structural reopen remains finite selected/accessed-path validation, not exhaustive historical assurance. Explicit full audit and establishing trust for imported read-only fixtures still traverse history. Legacy non-recipe continuation recovery also retains its exhaustive path; this change does not silently claim that every recovery path is logarithmic. Benchmark baseline construction, observer filesystem snapshots, and the final audit are exhaustive and are outside the timed publication/checkpoint/open operations. There is no directory-wide snapshot in recipe admission.

The recipe cap is 49,152 bytes, and the existing disposable HEAD frame cap remains 262,144 bytes. The measured recipe is 5.60–5.65 KB; pending HEAD is 7.63–7.69 KB versus an idle 735–743 bytes. The experiment therefore fits the existing fixture envelope without increasing its cap. It is **not** evidence that permanent HEAD bytes should carry this payload: an immutable recipe object referenced by a bounded HEAD is an alternative requiring a separately reviewed bootstrap, retention, and admission design. No permanent outer-envelope or golden-byte choice is made here.

## Measured evidence

Final exact raw output: [bounded JSONL](verification/ps2-typed-recipe-bounded.jsonl). Initial rejected path: [exhaustive negative JSONL](verification/ps2-typed-recipe-exhaustive-negative.jsonl). Each row fully materializes its N synthetic operations/receipts, then publishes one selected ownership claim and checkpoints groups of 1, 8, and 32 additional synthetic operations. This is not the genuine Core Graph furnace, automatic all-pack coverage, or an ownership-count scaling benchmark.

One observation per cell, not latency percentiles; all sizes are bytes unless labeled. Process bytes sum instrumented data, locator metadata, and envelope writes, including replacement envelopes. Net allocation is `st_blocks * 512` after minus before; it is neither peak coexistence nor filesystem free-space consumption under APFS snapshots/clones.

| N / map | Claim prepare + resume ms | Process bytes | Pending HEAD | Original hold | Net allocated delta | Fresh-handle open ms | Oldest retry ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1k radix | 80.373 | 50,411 | 7,626 | 3,022,352 | 4,096 | 1.289 | 1.177 |
| 1k B-tree | 90.247 | 50,502 | 7,642 | 3,022,656 | 4,096 | 2.115 | 1.010 |
| 10k radix | 100.761 | 50,799 | 7,687 | 3,023,104 | 4,096 | 1.415 | 1.444 |
| 10k B-tree | 98.789 | 50,672 | 7,665 | 3,022,704 | 4,096 | 1.602 | 1.089 |

Claim publication writes only 187 data bytes and 1,168–1,176 metadata bytes (four persisted metadata pages), but approximately 49 KB of envelopes and 28 `sync_all` calls. It stages the plan twice for comparison, without writing the preview. The roughly 3.02 MB hold is about 737–738 times the 4 KiB net allocation delta; this ratio is negative practical-admission evidence, not a peak-reserve adequacy proof. Its model charges complete touched 256 KiB packs, namespace/COW allowances, and conservative envelope headroom. It is not an exclusive OS reservation or a bound on arbitrary filesystem allocation behavior.

| N / map | Group 1 process bytes / ms | Group 8 process bytes / ms | Group 32 process bytes / ms | Sync calls, groups 1 / 8 / 32 |
| --- | ---: | ---: | ---: | --- |
| 1k radix | 44,956 / 89.973 | 107,286 / 137.916 | 240,854 / 128.734 | 26 / 26 / 28 |
| 1k B-tree | 41,897 / 101.432 | 109,454 / 118.025 | 225,264 / 139.453 | 26 / 26 / 27 |
| 10k radix | 53,363 / 102.415 | 174,272 / 124.558 | 429,983 / 155.009 | 27 / 26 / 28 |
| 10k B-tree | 49,807 / 101.082 | 147,440 / 113.158 | 383,698 / 136.584 | 26 / 26 / 28 |

These ordinary semantic checkpoints preserve the already-selected typed claim, but do not automatically extend ownership coverage to their newly written packs. Their existing checkpoint continuation is not this new recipe path. At 10k/group32, process bytes per operation are 13,437 radix versus 11,991 B-tree; this narrow fixture supports keeping B-tree as a serious comparator, not selecting permanent structure from single samples. No new compaction/retirement or unchanged-root-turnover benchmark is claimed in this slice.

Open/retry use fresh Rust handles over OS-cached files, not disk-cold processes. Open reads 12 semantic objects and 43–49 locator pages plus HEAD; retry reads 5–12 semantic objects and 25–53 locator pages. There are no open/retry writes or barriers. The complete four-row command took 17.32 seconds including 6.12 seconds compilation; maximum resident set size was 29,523,968 bytes. Temporary fixtures are removed on completion.

## Fault and regression evidence

Final verification: all seven recipe tests passed in 2.49 seconds; the complete example suite passed **53/53** in 41.28 seconds. Strict release Clippy, edition-2024 rustfmt, and `git diff --check` passed. Full regression log SHA-256: `f35426cc082dea5e8414fef05b4b30b1fad2adb40121a07d1d65f39cd9f98631` (`/private/tmp/ps2-typed-recipe-regression.log`).

Seven new recipe tests cover:

1. Original-token reopen/completion after pre-effect, partial byte 1/7/100, after-data, before-HEAD, and after-HEAD cuts.
2. Repeated partial and pre-HEAD retries: accumulated data/metadata payload writes equal the original recipe exactly, with unchanged recipe bytes/digest and no duplicate payload.
3. Mismatched occupied partial frame refuses without deleting it or replacing the hold.
4. Exhausted project budget and oversized recipe refuse before pack/HEAD effects.
5. Partial new metadata-pack rollover reopens under the original recipe.
6. Unknown HEAD preserves original hold, intent, and candidate evidence.
7. A symlink at a new-pack destination refuses without writing its target.

These are controlled returned-error cuts and deliberate disposable corruption, not actual kernel ENOSPC, storage power loss, or hardware flush qualification. Each resumed output receives explicit full audit in the tests; routine resume itself does not. The fixture rejects existing symlink/hardlink destinations and checks observed/opened identity, but does not establish a production secure-handle/namespace-owner lease protocol against hostile concurrent writers. Repeated retries can still repeat bounded control writes and barriers; payload idempotence is not a claim of zero repeated filesystem/COW allocation. The fixture's `sync_all` calls are not a production macOS F_FULLFSYNC qualification or a Saved acknowledgement.

## Reproduction and provenance

Run from the review worktree:

```sh
/usr/bin/time -l cargo run --release -p photara-store --example ps2_index_furnace -- placement-v3 typed-inventory recipe 1000 10000
cargo test --release -p photara-store --example ps2_index_furnace recipe
cargo test --release -p photara-store --example ps2_index_furnace
cargo clippy --release -p photara-store --example ps2_index_furnace --tests -- -D warnings
rustfmt --edition 2024 --check crates/photara-store/examples/ps2_index_furnace/placement_v3.rs
```

Final raw SHA-256: `a52393fef5925a329cf47e1211d972f6239569f8abd583939798876ecd0bf0c0`.
Negative raw SHA-256: `0f9b1acf26684ebbee689a4549545f6b6915a9fbf4186c3ef80ace853c5058f3`.
Negative recipe source SHA-256 (pre-bounded refinement): `7808985e69a7a63d663c7a4b243b06f8415cadef137ea2cc5023eb759b5df5d7`.

Final measured source SHA-256:

- `placement_v3.rs`: `c164e1177fa65105ed372a829f26c1671761c32090f8fd7167a719e61aacfd5a`
- `placement_v3/typed_inventory.rs`: `70e94e3f18d645ad7556cdf430a881f7f678c5e998a190fe729fde917fd1d2ab`
- `placement_v3/typed_inventory/recipe.rs`: `770b1ce66f9d30ccfe7beed2985b594a623a3b015cbaa735304f35035a6bdc57`

The only additional source edit is a `recipe: None` initializer in the existing ownership probe. Production code and prior published raw evidence are untouched.

## Remaining engineering gate

Use this bounded replay mechanism to prove complete automatically generated ownership closure and original-token coverage for active-prefix growth, metadata self-coverage, and control generations; then reconcile measured allocation ownership and exact post-retirement credits without double charging, speculative deletion credit, or an OS-reservation claim. That must remain coherent with all selected/recovery/reader/pin/hold roots. The conservative high-water ledger and control-copy overhead demonstrated here must not be mislabeled as complete refundable integration. Genuine Graph/journal integration, practical barrier optimization, and a permanent recipe placement choice remain subsequent gates.
