# PS2 active-prefix ownership adapter

Disposable engineering evidence, 2026-09-18. This bounded follow-on to local
checkpoint `952c22c` adapts the allocation-credit prerequisite to extending
data, locator and control packs. It is **not the integrated v3 backend** and
does not establish qualified Accepted/Saved, OS reservation, permanent wire,
production GC or a genuine Graph/map benchmark. No production or live-package
code changes. The measurement ran while the shared worktree HEAD was `4aaa147`.

## Implemented invariant

One owned allocation describes one regular file by non-reused allocation ID,
domain, device/inode, current barriered length/hash and charge. Selected active,
recovery and pin snapshots separately preserve exact prefix length/hash. Thus
extending the same inode does not invalidate an older selected prefix or charge
the same physical file once per logical snapshot.

An original growth group contains at most three packs and persists each exact
old allocation, suffix, target commitment and incremental hold before payload
effects. Admission sums the group in each capacity domain, with no anticipated
retirement credit. The project-owned-plus-held limit is independent of the
fresh generation-bound modeled filesystem availability input.

Application verifies the old prefix and any existing partial suffix. Only the
missing suffix is appended. After file/directory barriers, exact identity/extent
and target hash are checked, and observed regular-file charge converts the
original hold into a delta on the same unit. Interrupted retries reuse the
same extent and hold. Corrupt or unexpected tails retain liability and refuse.
Unknown HEAD outcomes require exact old/candidate reconciliation; they cannot
admit new growth. Completed retry coverage does not promise a historical
original Graph receipt: that path is not integrated here.

Sealing changes the owned unit's mutability state without adding another charge.
A full pack rolls to a new, distinctly owned file; it is not copied on each
append. Sealed retirement still requires two selected roots and all ten pin
classes to release the old generation: accepted journal, undo, history,
conversion, export, backup, reader, unresolved, relocation and resource.
Only exact-generation unlink, directory barrier, fresh absence and ownership
HEAD selection release its project charge. Neither absence nor project credit
manufactures filesystem availability. Pin completion/owner-end predicates are
model inputs, not implemented native reader leases.

The existing v3 arena framing is mirrored (little-endian length plus data key
for data records; length for metadata), but record bodies are small synthetic
role/ordinal/commitment JSON. They are **not v3 locator pages or selection
records**. This proves physical-prefix ownership accounting, not reference
rewriting, actual locator relocation, or combined Graph journal recovery.

## Bounds and important negative evidence

The fixture caps packs at 256 KiB, one growth suffix at 8 KiB and a group at
three packs. Its ledger remains a bounded flat control frame, not an adopted
scalable ownership tree: at most 128 units/holds, 64 pins, 16 allocation holds
and four modeled domains. The 256 KiB encoded-frame cap may refuse before those
count limits; candidate size is checked before creating a publication intent.

The current growth hold is deliberately conservative:
`max(256 KiB - existing charge, 0) + 16 KiB` per touched pack. All three growth
runs peak at **1,884,160 B owned plus held**, including the fixed 1 MiB control
allowance, even for very small suffixes. This is a finite accounting envelope,
not a tuned admission policy or a demonstrated allocator upper bound. The
fixed control allowance, namespace effects, APFS snapshots/clones/quota,
concurrent external allocation and delayed allocation remain unqualified.
Late ENOSPC can still prevent progress without permitting false completion or
speculative deletion credit.

Preflight/retry verification hashes bounded pack prefixes, up to three 256 KiB
packs; it is not optimized to hash only new bytes. No external media is read.
Ordinary fresh-handle open validates one bounded ownership HEAD and reference
shapes, with zero pack payload reads. Explicit prefix audit is separate. This
does not make the flat bounded ownership fixture a lifetime-scale inventory.

## Real-file measurements

Each row runs eight groups, with 1/8/32 synthetic records appended **per role**
in each group. Three packs are initialized before counters reset. Each group
persists growth intent, applies the three suffixes, and selects current prefixes;
prefix audit is timed separately. Then all packs seal, two empty selections
release active/recovery, and exact retirement releases their project charges.
No extra pins are held in timing runs; tests exercise all classes.

| Records/role/group | Appended payload, 8 groups | Process bytes, checkpoint phase | Write amplification | Checkpoint median / max | Net allocated delta at seal | Retired project charge |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 2,968 B | 58,335 B | 19.65× | 98.1 / 122.9 ms | 0 B | 12,288 B |
| 8 | 23,909 B | 148,978 B | 6.23× | 90.0 / 95.2 ms | 16,384 B | 28,672 B |
| 32 | 96,188 B | 461,009 B | 4.79× | 88.6 / 106.4 ms | 90,112 B | 102,400 B |

Median uses the lower central sample of eight. Every checkpoint group performs
**24 sync calls** (192 per row), including file and directory `sync_all`; no
hardware/power-loss qualification is implied. Control envelopes include the
pending suffixes, contributing substantial write amplification. These results
are protocol evidence, **not a snappy application implementation**.

Fresh-handle structural opens take 142/512/53 microseconds and read ownership
HEADs of 1,425/1,428/1,439 bytes respectively; files remain OS-cached, not
disk-cold. Three-pack seal takes 84.7/76.3/70.6 ms; two-selection turnover takes
52.4/52.8/44.5 ms; three-pack retirement takes 189.5/157.0/158.5 ms. All end at
the fixed 1,048,576 B project control charge with no owned packs. Raw output
separates checkpoint and all-phase counters.

Observed peak regular-file allocation is 24,576/53,248/155,648 B. Allocation
metrics use `st_blocks * 512`, not device writes or total APFS physical space;
they exclude directories, filesystem metadata, snapshots and unobserved peaks.
Net delta at seal is not peak coexistence. The complete command takes 6.30 s
wall including 1.87 s compilation, with 4,620,288 B reported maximum RSS.

## Verification and provenance

```sh
cargo test --release -p photara-store --example ps2_prefix_ownership
cargo clippy --release -p photara-store --example ps2_prefix_ownership --tests -- -D warnings
rustfmt --edition 2024 --check crates/photara-store/examples/ps2_prefix_ownership.rs
/usr/bin/time -l cargo run --release -p photara-store --example ps2_prefix_ownership > /private/tmp/ps2-prefix-ownership-final.jsonl 2> /private/tmp/ps2-prefix-ownership-final.log
```

**16 release tests pass** (final captured run 6.15 s); strict Clippy and formatting
pass. Nine are deliberately inherited from the prior allocation-credit gate.
Seven new tests cover original prefixes under all pins; four growth cuts;
matching/corrupt partial suffixes; unsealed/chunk-bound refusal; exact full-pack
rollover; five repeated pre-HEAD retries with zero payload reappend; and atomic
shared-domain group refusal independently of modeled availability.
Inherited tests cover credit/retirement cuts, unknown selectors, stale reader
epochs, hardlinks, repeated exact hold recovery and independent availability.
Faults are controlled returned errors and fresh-handle reopen/reconciliation,
not actual kernel ENOSPC or power-loss injection. Secure namespace handles and
hostile concurrent writers remain outside this model.

The main example deliberately forks the prior roughly 925-line disposable gate:
Unit/Ledger/Pin shapes change, so extracting a generic backend would expand this
slice and disturb previous evidence. Both the original source and previous raw
results remain untouched; no claim is made that this duplicate model is the
production or integrated v3 implementation.

- Main source SHA-256: `5a0a2bbdb7f6b87dbf961000ad186d7ff0fe9ec0e51f829e06fb860675b639b1`.
- Adapter source SHA-256: `cc64b8e5f8458080bb72dfa07d39e53d5ad94dd9536ddaa01bca41e7764c7e39`.
- Unchanged inherited source SHA-256: `bb16d8c366437e59b43c24ab543c84777ad3dde6fa68e7bc44b832a1fdac344f`.
- [Raw three-row evidence](verification/ps2-prefix-ownership.jsonl), identical to captured stdout (8,459 B), SHA-256: `06d9baaa5e80fc7f736c6398df58e05aa425fd4b4f88c703c4bc0c01e3cc2dd3`.
- `/private/tmp/ps2-prefix-ownership-tests.log` SHA-256: `954db0153452185f0b83371ba7bdd6fb8f111524055774714a30743f652e799f`.

## Remaining integration gate

Next bind these exact prefix/generation ownership descriptors to **actual v3
data and locator arenas, control publication and persistent liveness**, replacing
the synthetic metadata records and bounded flat ledger with the reviewed
persistent structure. Charge real preflight extent/namespace effects and bind
qualified platform availability and reader ownership. Demonstrate locator
rewrite and control-generation retirement under the same selector and original
continuations, without inventing a second dispatch authority.

Only then connect genuine Core Graph intents/original receipts/journal groups
and rerun radix/B-tree at 1k/10k/100k and groups 1/8/32. That comparison, actual
locator compaction, end-to-end Accepted/Saved qualification and production
capacity guarantees remain unimplemented. This adapter establishes the smaller
prefix-accounting transition without selecting permanent bytes or semantics.
