# PS2 disposable compact retirement commitments

This experiment removes copied write bodies from the bounded original live-relocation ticket. It keeps the 48 KiB ticket cap, exact selected semantic and ownership closure, same-tip restriction, original-token admission, and existing attributable retirement credit rules. It changes examples only and makes no permanent codec, portable inode, production deletion, or qualified Saved claim.

A ticket contains exactly one mode: both full plans and no compact descriptor, or neither full plan and one compact descriptor. Missing, partial, and mixed modes refuse at generic liability admission before selecting any hold and again at replay/retirement boundaries. Existing full plans retain their previous tagged UTF-8 object serialization when present. This preserves the current prior full-ticket serialization; the older decimal-byte-array decode-only limitation documented in the preceding experiment remains unchanged.

For each destination, the compact descriptor commits the exact original Prefix (pack, extent, device, inode, full prefix digest), final extent, frame count, byte count, and digest of every appended raw frame. It also commits the complete data plan, metadata plan, and original Continuation together. Device/inode witnesses remain local disposable evidence. The descriptor does not replace or become a second mutable selector.

Before admission, the bounded deterministic planner regenerates the original plan and checks these commitments. Complete new allocation reserve is derived from the real in-memory plans; no future retirement credit is included. Before replay, the source and original base reconstruct those same plans. Existing suffix bytes must exactly equal the corresponding original framed prefix, and unexpected extra tails refuse. Both destination proofs precede staged-selector removal or reconciliation.

After publication, and after authorized source unlink, the original source is unnecessary for destination validation. The verifier authenticates both original destination prefixes and exact complete suffixes, parses all raw frames (including unreachable intermediate ownership records), reconstructs physical WritePlans, and recomputes the complete typed-plan digest against the original Continuation. Merely checking reachable objects would be insufficient. The selected semantic and ownership closures are audited separately. A still-present source must match its original sealed witness and digest before selected dispatch cleanup; absence is allowed only with the exact already-selected original unlink authorization.

The original source charge remains in its ticket until exact-generation unlink, directory barrier, and fresh absence. Unknown replacement files or broken symlinks are not removed. Completion and immediate retry remain exact and once-only, with zero filesystem availability credit. Later unrelated maintenance may make an old bounded completion receipt unavailable; this is not a new lifetime receipt index.

## Focused verification

Five new tests cover a full-body plan that exceeded the unchanged ticket cap but now completes through fresh-handle authorized-unlink recovery; old Prefix reuse against changed bytes or a same-byte replacement inode; altered witness, prefix hash, frame count, final extent, and typed-plan digest before and after unlink; full serialization preservation and generic mixed/absent admission refusal; unreachable suffix corruption, extra tail, and replaced destination before staged dispatch cleanup; and changed/missing source after HEAD selection with all dispatch evidence retained. Existing original replay, all-pin, source substitution, unlink, credit, and legacy full-ticket tests remain applicable.

Final-source commands and measured results are recorded below. Historical full-body refusal evidence is preserved in [PS2_LIVE_OWNED_RELOCATION.md](PS2_LIVE_OWNED_RELOCATION.md), including the earlier fixed-budget runs; it is not overwritten by this experiment.

## Measurement interpretation

The fixed-budget comparison uses the same upfront formula: measured initial attributed charge plus the first complete Graph admission reserve plus either 256 KiB or 16 MiB. Each run holds its selected limit constant. Observed allocation blocks can vary between independent runs on the current volume, so neither identical source charges nor byte-for-byte identical physical starting states are assumed. Final-run rows below are authoritative; the preceding full-body experiment remains historical evidence.

The immediate-source experiment attempts the initial padded source and then the previously active data tip every eighth group. Its initial positive return is separate from runtime maintenance. The aged experiment excludes that initial source and instead selects a genuinely Graph-born tip from four completed groups earlier, after a later Graph publication sealed it. It retains every authored receipt and audits independent selected closures and allocation attribution after every group. Both experiments measure outcomes, including negative returns, without proposing automatic product scheduling or lifetime capacity stability.

The final-source broad run passed 130 index tests (205.31 s) and seven Graph tests (1.61 s), excluding exactly the two long measured workloads. Strict example Clippy and targeted formatting also passed. [Broad log](verification/ps2-compact-retirement-broad-tests.log), [five focused proof tests](verification/ps2-compact-retirement-focused-tests.log), and [20-file source hashes](verification/ps2-compact-retirement-source-sha256.txt) record the checkpoint inputs. The independent long-workload outcomes complete the coverage below.

## Runtime-aged measurements

The aged eight-record workload completed 64 groups (512 records) per map. All eight retirements per map succeeded, but none had positive net project credit. Radix credited 524,288 bytes and spent 647,168 on new allocation, a net **−122,880**; Btree credited 520,192 and spent 618,496, a net **−98,304**. This is successful relocation with unfavorable economics, not useful net capacity recovery. [Raw eight-record samples](verification/ps2-compact-aged-eight-turnover.jsonl).

The supported 32-record workload naturally creates larger runtime generations. With an initial charge of 2,576,384 bytes and a fixed limit of 39,882,752 bytes, Radix completed 28 groups (896 records) and Btree 27 groups (864 records). The next original admission refused capacity without effects. Both performed three runtime-created generation retirements: one broke even and two returned positive credit. Each map returned **49,152 net project bytes** across those three operations. All original Graph receipts and independent selected semantic/ownership closures remained valid. [Raw 32-record samples](verification/ps2-compact-aged-thirty-two-turnover.jsonl).

| Map / zero-based group | Source pack | Source extent | New data bytes | New metadata bytes | Source credit | Observed growth | Net credit | Superseded source-byte lower bound |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Radix / 7 | 5 | 131,194 | 77,321 | 61,629 | 135,168 | 135,168 | 0 | 53,873 |
| Radix / 15 | 13 | 194,491 | 99,187 | 69,740 | 196,608 | 167,936 | 28,672 | 95,304 |
| Radix / 23 | 22 | 216,184 | 118,102 | 78,931 | 217,088 | 196,608 | 20,480 | 98,082 |
| Btree / 7 | 5 | 133,075 | 77,321 | 61,348 | 135,168 | 135,168 | 0 | 55,754 |
| Btree / 15 | 13 | 192,595 | 105,033 | 72,267 | 196,608 | 172,032 | 24,576 | 87,562 |
| Btree / 23 | 22 | 207,350 | 108,923 | 74,093 | 208,896 | 184,320 | 24,576 | 98,427 |

New data bytes include both copied live source records and newly written ownership updates. Consequently they upper-bound retained source bytes rather than measure those bytes exactly. The last column is `max(0, source extent − new data bytes)`, a conservative lower bound on source bytes not retained by the plan. Raw rows separately report selected semantic/ownership membership counts across both roots; these counts are not unique-byte measurements. Metadata writes and observed allocation growth are independently reported, so raw payload savings are not conflated with net project credit.

These runs demonstrate repeated positive reclamation of actual runtime-created generations within a bounded fixture. They do not demonstrate a lifetime plateau: immutable receipts and source staging remain charged, new groups continue to allocate, the same-tip restriction remains, and the larger run stops at its fixed admission limit. No filesystem availability credit is issued.

Exact long-workload commands on the recorded final source:

```sh
cargo test --release -p photara-store --example ps2_index_furnace turnover_aged_runtime_generations_under_fixed_capacity -- --nocapture
PHOTARA_PS2_TURNOVER_RECORDS=32 cargo test --release -p photara-store --example ps2_index_furnace turnover_aged_runtime_generations_under_fixed_capacity -- --nocapture
```

Both passed (379.60 s and 186.67 s respectively). Checked-in test logs remove only trailing empty lines and retain a terminal newline; original temporary logs remain unchanged.

## Immediate-source comparison

The final-source immediate-source workload passed in 535.35 s:

```sh
cargo test --release -p photara-store --example ps2_index_furnace turnover_fixed_capacity_original_groups_and_bounded_refusals -- --nocapture
```

[Raw final-run samples](verification/ps2-compact-immediate-turnover.jsonl) retain every successful maintenance row, its sign, and the exact no-effect refusal.

| Map / extra headroom | Fixed limit | Completed groups × 8 | Successful / net-positive maintenance | Source credits | New observed growth | Net maintenance credit | Final project charge |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| Radix / 262,144 | 23,367,680 | 6 = 48 records | 1 / 1 | 262,144 | 57,344 | 204,800 | 3,198,976 |
| Btree / 262,144 | 23,367,680 | 6 = 48 records | 1 / 1 | 262,144 | 57,344 | 204,800 | 3,203,072 |
| Radix / 16,777,216 | 39,882,752 | 83 = 664 records | 11 / 1 | 942,080 | 1,036,288 | -94,208 | 19,673,088 |
| Btree / 16,777,216 | 39,882,752 | 82 = 656 records | 11 / 1 | 946,176 | 950,272 | -4,096 | 19,668,992 |

Every run ends with the next original Graph admission refusing capacity before effects. In both tight runs, one initial padded-source retirement returns 204,800 net bytes and six groups complete; the historical full-body-cap runs completed four. In the wide runs, compact recipes permit eleven maintenance operations per map, but only the initial padded-source operation is net positive. The ten immediate runtime-source operations collectively cost more than they reclaim: −299,008 bytes for Radix and −208,896 for Btree. Wider immediate-source progress is therefore not a demonstration of useful periodic garbage collection. The independent aged 32-record experiment above supplies that evidence without using the initial padded source.

Across these final-source tests, the compact serialization obstruction is removed for the measured same-tip candidates while actual allocation, admission, and unfavorable economics remain visible. No ticket bound was raised and no future-free or filesystem-space credit was used.
