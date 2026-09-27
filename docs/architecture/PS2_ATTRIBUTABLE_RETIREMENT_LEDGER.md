# PS2 attributable retirement ledger — disposable actual-v3 fixture

This opt-in extension uses the existing actual-v3 physical files, typed ownership
and semantic locator, original liabilities and sole authoritative `HEAD`. It adds
attributable project accounting and a bounded original-token retirement path.
It is not permanent wire, production GC, a portable inode identity contract,
macOS storage qualification, or a user-visible `Saved` guarantee.

## Selected accounting and recovery

- Bootstrap starts only in a fresh fixture with empty capacity domains, holds
  and pins. It measures each of at most 64 existing pack generations once and
  separately charges a standing control allowance. It does not convert a legacy
  aggregate charge into refundable provenance.
- A bounded, HEAD-selected pending enrollment preserves original generation,
  prefix hash and charge evidence before any typed leaf is installed. Resume
  verifies those original prefixes before any recapture. A single original-token,
  self-covered typed recipe installs both ownership roots and closes its own
  current data/metadata suffix extents. Partial writes and selector cuts resume
  that original recipe; enrollment is cleared atomically at completion.
- Exactly two registered current-tip charges track local generation, extent and
  high-water allocation. Settlement uses `max(previous_registered, observed)`;
  compression or a smaller observation never earns a credit. Growable ownership
  claims carry no guessed future charge. Sealed claims carry immutable registered
  charge provenance, distinct from their pre-effect `observed_charge` field.
- The finite dispatch allowance is 1,163,264 bytes: four capped control frames,
  namespace allowance and explicit uncertainty allowance. It is charged once,
  bounds prospective frames, and is never refunded from observed control inodes.
  Existing legacy high-water fixtures retain their prior behavior.
- Full audit deduplicates registered units across selected/pinned ownership,
  current tips, pending enrollment and original retirement tickets, then requires
  the project total to equal those units plus standing allowances. This is an
  exhaustive audit, not a lifetime scan on ordinary open or checkpoint.

A retirement first preflights every extra pin class, without admitting a hold or
writing state when a pin blocks. Its original token binds an exact sealed
allocation and registered charge, typed removal plan, complete pre-effect
selector, and deterministic data/metadata recipe. Both current ownership roots
lose the source claim while their current-tip ownership extents are updated by
bounded fixed-point planning. The charge remains in the selected ticket.

The unlink path checks the exact local generation and all active/recovery/pin
placements. It durably selects a separate original-token unlink authorization
before unlinking. A missing path without that authorization refuses; it cannot
manufacture a refund. Unlink, directory sync and fresh absence must all succeed
before one HEAD selection consumes the ticket and subtracts its registered
project charge. Unrelated holds, unknown control values or changed generations
retain evidence and charge. Filesystem-availability credit is always zero.

A bounded last-credit receipt reconciles immediate before/after final-HEAD cuts.
A retry after completion cannot refund twice. This is not a lifetime maintenance
receipt index: sufficiently old retries after intervening work may refuse.
Authored Graph receipts retain their separate indexed history.

## Measurements

Commands, both locator implementations:

```sh
cargo run --release -p photara-store --example ps2_index_furnace -- placement-v3 typed-inventory ledger 64 1
cargo run --release -p photara-store --example ps2_index_furnace -- placement-v3 typed-inventory ledger 64 64
```

The last argument selects one or 64 real framed, unreachable metadata records in
an otherwise sealed pack. Every run also performs 64 selector turnovers and
recovers a partial ownership write, an after-unlink cut and an after-credit-HEAD
cut. [Raw evidence](verification/ps2-attributable-retirement-ledger.jsonl) includes
original tokens, holds, selected charges and bounded credit receipts.

| Sealed candidate | Maps | Initial project charge | Release/after-unlink charge | Exact retired charge | New tip allocation | Final charge | Net project release |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| One framed record | Radix, B-tree | 1,269,760 | 1,269,760 | 4,096 | 4,096 | 1,269,760 | 0 |
| 64 framed records | Radix, B-tree | 1,286,144 | 1,286,144 | 20,480 | 4,096 | 1,269,760 | 16,384 |

The single-record case is negative overhead evidence: valid exact retirement
need not increase available project budget. Both cases preserve the original
charge through root release and unlink; only the final verified selection applies
credit. All 64 control turnovers leave charge unchanged. Wall times were collected
while other fixtures ran and are not a locator performance comparison.

## Validation and remaining boundaries

The focused matrix covers constant-charge control turnover; original bootstrap
recipe cuts; corrupted pending-enrollment prefix refusal; all ten extra pin
classes before admission plus native active/recovery closure; partial release and
both release-selector cuts; unlink and directory-barrier cuts; both final-credit
selector cuts; no duplicate credit; unknown controls; unrelated holds; missing
source without authorization; and replaced source inode. Root verification runs
the full shared example suite after joint Graph integration. The final checkpoint
passed 90 index-furnace tests and seven Graph-furnace tests, strict Clippy and
formatting checks; the focused ledger filter passed all ten selected tests.

This retirement proof deliberately uses an already-unreferenced sealed placement.
It does not yet combine live-placement relocation, fresh-generation enrollment and
refund in one operation. Rollover refuses before an attributed hold until a fresh
pack has reserved original-token enrollment and a local inode witness. Standing
bounds are conservative fixture assumptions, not exclusive APFS reservations.
Local `dev`/`ino` evidence does not solve cross-host copy/rebind. Imported packages
retain the explicit full-audit/read-only boundary.

The joint Graph path separately enrolls retained source-file high-water charge and
a nonrefundable standing scratch allowance; it settles source and package-tip
growth under its original Graph token after receipt/journal cleanup. Those source
bytes are not fictional sealed package allocations and cannot use these tickets.

Core ledger source SHA-256 at the evidence run:
`64a59c539b0b3c768393c151f61e8fbf6bea96ab8ce3c0f282ffec2910b11c53`.
