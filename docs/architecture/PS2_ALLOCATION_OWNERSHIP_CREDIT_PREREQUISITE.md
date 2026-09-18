# PS2 allocation-ownership credit prerequisite

Disposable engineering evidence, 2026-09-18; starting base `c6b6646`.
No production wire, writer, reader, GC, live package, migration or qualified
Accepted/Saved behavior is changed.

## Why this slice precedes Graph/v3 integration

Inspection confirmed that v3 and `ps2_liveness_reserve` release **unused holds**
but intentionally never subtract retired allocation charges. Consequently the
requested complete/refundable backend did not yet exist to integrate. This
approved smaller iteration implements an executable ownership-credit gate over
real disposable files. It does **not** claim a combined Graph/v3 benchmark or
repeat the 1k/10k/100k map comparison. The remaining integration is explicit.

The accounting distinction is essential:

- A project-owned allocation charge can leave its ledger after authorized,
  verified retirement of that exact namespace generation.
- That transition does not increase observed filesystem free space, defeat a
  snapshot/quota, or reserve blocks against another writer. New admission still
  requires a separate fresh capacity observation, outstanding liabilities and
  margin. Late ENOSPC remains possible without guaranteed forward progress.

## Executable gate

`examples/ps2_allocation_credit.rs` records immutable allocation units by
non-reused ID, capacity domain, device/inode, length, digest and post-barrier
observed regular-file charge. The model's single HEAD selects both the ownership
ledger and active/recovery keep-sets. Its bounded intent retains exact old and
candidate hashes during uncertain publication; new admission refuses until
reconciliation and renewed barriers establish the observed selection.

Admission sums all requests and existing holds in each domain before any
payload effect. A persistent original hold is marked started before writing.
Pre/post-effect failures retain that liability and fence new admission. Retry
verifies and reuses the same file, rather than appending another candidate.
After file/directory barriers and post-barrier allocation sampling, the original
hold becomes an owned charge; only its unused portion is released.

Retirement requires the old unit to leave **both** selected roots. Ten additional
classes block it: accepted journal, undo, history, conversion, export, backup,
reader, unresolved intent, relocation and resource obligation. Pin token/epoch
must match exactly; reader completion/owner-end and unknown-resolution predicates
must be supplied. These are model proof inputs, not actual cross-process lease
or quiescence verification. Stale releases refuse; tokens are never reused.

The unit is first selected as retiring while still fully charged. Its exact
device/inode, length, digest and single-link ownership are verified before
unlink. Credit follows a directory barrier and a fresh absence lookup, then
selection of the ledger without that exact unit. Errors before/after unlink or
before/after HEAD selection never create speculative credit or allow admission
through an unknown outcome. Repeating retirement of an absent ledger ID returns
zero credit; it is not a promise to recover a historical user-operation receipt.

The finite fixture uses at most 128 live units/holds, 64 pins, 16 outstanding
holds and four modeled domains. It rewrites a bounded ledger frame (maximum
256 KiB), not a scalable persistent allocation index. A fixed 1 MiB control
allowance covers its modeled control namespace; it is not an OS allocation
guarantee. Fresh filesystem/quota observations are generation-bound **modeled
inputs** with explicit qualification flags, not an implemented qualified platform
probe. No complete physical reserve, snapshot accounting or allocator upper
bound is established here.

## Real-file measurements

Each turnover allocates an 8,192-byte immutable file under a 65,536-byte modeled
hold. The project budget is fixed at control allowance + 128 KiB. After the
first turnover, two selections release the prior recovery root before retiring
its file. These are allocation-turnover cycles, **not Graph operation groups**.
Timing loops have no extra pins; the tests exercise all ten classes separately.

| Cycles | Process bytes written | Sync calls | Elapsed ms | Retired project charge released | Final project charge |
|---:|---:|---:|---:|---:|---:|
| 1 | 10,424 | 28 | 128 | 0 | 1,056,768 B |
| 8 | 100,331 | 343 | 1,428 | 57,344 B | 1,056,768 B |
| 32 | 409,802 | 1,423 | 6,114 | 253,952 B | 1,056,768 B |

All end with one live unit and no completed-credit tombstone history. At 8/32
cycles, peak modeled owned-plus-held charge is 1,122,304 B, including the entire
old charge, new hold and fixed control allowance. No future deletion credit is
used for admission. Selected-boundary regular-file allocation peaks at 20,480 B;
sampling during control-file coexistence observes 28,672 B. These `st_blocks`
observations exclude directory/APFS internal metadata, snapshots, device writes
and unobserved allocation peaks; the control allowance is deliberately distinct
from measured regular-file blocks.

The final command takes 9.29 s wall, 0.10 s user CPU and 0.79 s system CPU including
build/launch; maximum RSS is 2,965,504 B. This is an intentionally explicit,
unoptimized proof-state protocol, not an application-latency recommendation.
Every output states `qualified_os_reservation: false`, `qualified_saved: false`
and `graph_v3_integration: false`. No external media is accessed.

## Verification and provenance

```sh
cargo test --release -p photara-store --example ps2_allocation_credit
cargo clippy --release -p photara-store --example ps2_allocation_credit --tests -- -D warnings
rustfmt --edition 2024 --check crates/photara-store/examples/ps2_allocation_credit.rs
/usr/bin/time -l cargo run --release -p photara-store --example ps2_allocation_credit > /private/tmp/ps2-allocation-credit-final.jsonl 2> /private/tmp/ps2-allocation-credit-final.log
```

Nine release tests pass in 3.20 s; strict Clippy and formatting checks pass.
They cover shared-domain oversubscription; stale/unqualified/zero availability;
two roots and all pin classes; stale reader epochs and hardlinks; four allocation
cuts; five retirement/selection cuts; unknown selectors; duplicate credit; and
five repeated pre-HEAD retries reusing the exact original physical allocation.
Faults are controlled returned errors and fresh-handle reconciliation, not real
power loss or kernel ENOSPC injection. Secure-handle/owner-lease qualification
and hostile concurrent namespace writers remain outside this fixture.

The [unchanged raw three-row evidence](verification/ps2-allocation-credit.jsonl)
has SHA-256 `2ef443065a2d0cdd5af2c3bccc1d0613631f0d87f48e1de8c2738e4eebdc214b`.
Source `crates/photara-store/examples/ps2_allocation_credit.rs` has SHA-256
`bb16d8c366437e59b43c24ab543c84777ad3dde6fa68e7bc44b832a1fdac344f`.
Test log `/private/tmp/ps2-allocation-credit-tests.log` has SHA-256
`8463599ee4ac2f237ed900d20742f85d3bafe3104ed9631fa2d24714af415030`.

## What this unlocks—and does not

The model demonstrates a refundable **project ownership ledger** that avoids
the old monotonically increasing retirement charge while never manufacturing
filesystem availability. Its exact-generation retirement transition can now be
adapted to v3's persistent selected locator and all-class obligations.

Before that integration, define and test charge growth for an **extending active
pack**, not just immutable whole files, and ownership of locator/control
metadata allocations. Map those descriptors to persistent bounded pages rather
than this 128-unit frame. Bind real qualified filesystem/quota observations and
owner/reader proofs, including any preallocation guarantees and safe refusal
when they fail. Then connect genuine typed Graph intents/original receipts and
the journal to this accounting and v3's continuations, and run the requested
map/group/scale comparison. That combined result and production capacity
qualification remain deferred; no permanent schema or wire choice is implied.
