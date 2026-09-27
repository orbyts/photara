# PS2 original-token fresh-generation enrollment prerequisite

Disposable actual-v3 evidence, 2026-09-27, continuing the Graph/ownership/ledger
checkpoint `5f6447a`. This uses the existing `Fixture`, `HEAD`, capacity gate,
publication and reconciliation implementation. It is not a forked filesystem
model, permanent format, production lease, GC or qualified `Accepted`/`Saved`
path. No Graph journal is accepted by this prerequisite. Its final pack files
are empty, and its original liability remains selected and fully held.

## Admission and physical identity

The new
[`fresh_generation.rs`](../../crates/photara-store/examples/ps2_index_furnace/placement_v3/typed_inventory/fresh_generation.rs)
stores one immutable birth plan and bounded phase state in `Liability.birth`.
The plan binds the original token, random nonce, request digest, exact initial
selection, directory device/inode and ordered successor pack IDs. At most four
data and four metadata generations are admitted. Existing holds keep `None`.
Admission requires completed attributable bootstrap and an idle audited fixture.

Before creating any stage, the real HEAD selects the original hold covering
full bounded pack capacity, marker staging and namespace/control allowances.
The planner also preflights the largest serialized phase/control envelopes with
maximum-width witnesses. It does not assume an initially small `Intended`
record implies that later inode witnesses will fit. Caps and insufficient
capacity refuse before namespace effects; these are modeled fixture bounds,
not OS reservations or production defaults.

Each slot progresses under the same token:

1. `Intended`: create a no-replace marker-only stage. Its complete canonical
   contents bind the nonce, token, slot and immutable plan digest. A complete
   matching marker may establish the initial observed inode only under the
   declared private, exclusively controlled disposable fixture namespace.
2. `Bound`: after file/directory barriers, select the actual device/inode and
   marker witness in HEAD. All subsequent accesses require this exact inode;
   identical bytes in a replacement inode do not satisfy the witness.
3. `Empty`: truncate only that known marker-only inode, barrier it, and select
   the phase. This preserves existing data/metadata framing at offset zero.
4. `Promoted`: create the final pack name with a no-replace hard link, barrier,
   remove only the recorded staging link, and barrier/reopen the final empty
   single-link file. The two-link exception exists only for that exact recorded
   stage/final pair during promotion.

Empty or partial **unbound** stages are retained and fenced. The fixture does
not infer their ownership, delete them or create a replacement attempt. This
is explicitly incomplete recovery coverage before a full marker exists. The
exclusive namespace assumption is not a qualified production admission profile;
hostile interference before the first inode witness is outside this proof.

## Original identity and control reconciliation

Fresh-handle recovery reconstructs immutable liability fields from the original
plan rather than trusting the recovered hold to authenticate its own budget.
Token, target, origin, epoch, exact capacity/control amounts and absent Graph/
retirement fields must match that reconstruction; only bounded phase progress
may differ. Selected roots, pins, charge and other gate fields remain unchanged.
While a birth hold is selected, ordinary pin acquisition/release, additional
reservation, growth completion and checkpoint/retirement resumption refuse.

The birth path reads private, bounded, no-follow control files and requires
their exact canonical bytes. It accepts only the reconstructed old/candidate
selections and an absent or exact candidate `HEAD.next`. A known redundant
`HEAD.next` is removed and directory-barriered **before** generic reconciliation
removes intent/candidate. Interruption after that cleanup retains the dispatch
evidence. Unknown staged selectors, orphan candidate/next files, or changed
pin/charge fields remain untouched and fence recovery.

| Interruption or interference | Exercised result |
| --- | --- |
| Empty or partial unbound marker | Refuse; preserve stage, selection and hold |
| Complete marker before/after its barrier | Rebarrier and bind under the original token |
| Before/after witness, empty or promoted HEAD selection | Reopen and resume the exact permitted phase |
| Real `HEAD.next` file barrier before rename | Validate and clean only the known staged selector, then reconcile |
| After known staged-selector cleanup | Retain intent/candidate; another reopen completes |
| Bound-stage truncate or empty barrier | Require original inode; resume empty phase |
| Link, directory barrier, known staging unlink or final barrier | Reconcile the exact one/two-link namespace state |
| Same-byte inode substitution after binding | Refuse without adopting the replacement |
| Unknown final occupant, symlink, extra hardlink or corrupt marker | Refuse without replacing/deleting evidence |

These are returned-error and fresh-handle tests over real disposable files.
They do not prove arbitrary mid-control-frame recovery, all prebind crashes,
kernel ENOSPC, abrupt power loss or qualified physical allocation bounds.

## Verification and next integration

```sh
cargo test --release --offline -p photara-store --example ps2_index_furnace fresh_generation
cargo clippy --release --offline -p photara-store --example ps2_index_furnace --tests -- -D warnings
```

The scoped suite passes ten tests in 30.17 s; strict example Clippy and formatting
also pass. Coverage includes both maps, all eight admitted empty
generations, maximum-envelope preflight, unfinished-bootstrap refusal, full
control/charge/pin corruption, fresh-handle altered-liability refusal and
no-effect refusal of ordinary APIs while the birth hold remains pending.
The [recorded evidence](verification/ps2-fresh-generation-enrollment.json)
retains the executed output and exact source hashes. It excludes any unregistered
rollover draft. The checkpoint full regression also passes all 100 index tests
and seven original Graph tests (index suite: 100.29 s).

The next integration must compose enrollment with genuine Graph journal/payload
replay under this same token. Before journal acceptance, reserve a bounded
finalization corridor in the last two witnessed tips for exact sealed-charge
records and both ownership/locator paths. Charge only measured post-barrier
high-water growth; put sealed charge records outside the sealed pack they
describe. Refuse before acceptance if finalization cannot fit its proven bound.
Do not turn a late metadata rollover into an unbound generation or estimate a
sealed refund. Phase publication after payload writes must preserve the old
selected roots and physical-prefix coordinates until final joint publication;
an advanced writer cursor alone is not authority to advance the selected tips.
