# LL1 disposable authorization commit guard

Status: **27 candidate-overlay cases passed**, 2026-09-27. This is a separate
opt-in [private fixture](../../../scripts/test_ll1_authority_commit_guard.py),
not a migration, production constraint choice or automatic cleanup mechanism.
It tests the already documented requirement that transaction grant/permit work
must not survive commit. It does not alter the historical
[raw-authority counterexample](LL1_SERVICE_AUTHORITY_HANDOFF_REVIEW.md#unusable-abandoned-grants-are-not-absent-transaction-work-state).

## Candidate guard and scope

The new overlay adds one SECURITY DEFINER trigger function with a fixed
`pg_catalog,pg_temp` search path and schema-qualified relation accesses. Two
`DEFERRABLE INITIALLY DEFERRED` constraint triggers observe INSERT/UPDATE on
`ll1_probe.authorization` and `ll1_probe.permit`. At constraint checking, a
surviving authorization with the original transaction/backend/operation key
raises `unconsumed_authorization_at_commit`; a surviving permit for the inserted
transaction raises `unconsumed_permit_at_commit`. Both use SQLSTATE `23514`.
Permit comparison also matches NULL, so a malformed NULL transaction coordinate
cannot evade this candidate guard.

The trigger queries the final rows, not just the historical insertion event.
Insertion followed by legitimate consumption/removal therefore commits; leaving
work behind rejects the whole transaction. It never deletes authorization,
permit, request-binding or receipt rows to make commit succeed. Direct function
execution is revoked from ordinary roles and the raw authority role. A narrow
temporary observer is callable only during the cancellation test so the actual
mint boundary can be checked inside its owning transaction without giving that
login raw table access.

This overlay starts on a fresh fixture and protects newly inserted/updated work.
It is not a retrofit operation that discovers or deletes previously committed
orphans. It exercises the raw authority surface with synthetic trusted identity
inputs; it does not replace or weaken the signed HTTP/fresh-device adapter.

## Executed evidence

- Mint-only COMMIT and an early `SET CONSTRAINTS ALL IMMEDIATE` both fail with
  the exact authorization guard error. Every baseline and overlay row equals
  the pre-operation snapshot, including newly attempted request binding.
- Explicit rollback after mint restores every row.
- Cancellation after mint uses two actual backend PIDs and
  `pg_blocking_pids`, with the request blocked after an in-transaction observer
  verifies its authorization row. The controller cancels that exact private
  backend's statement and observes SQLSTATE `57014`. The request connection and
  all its locks must disappear before complete snapshot equality is accepted.
- A fault after successful grant consumption and aggregate deletion verifies
  that authorization/permit work is empty at the terminal receipt insertion,
  then raises `after_consumed_terminal`. All rows roll back exactly.
- A successful consumed-grant transaction deletes exactly the seeded target
  rows across the baseline, preserves unrelated rows, and commits one original
  raw receipt and request binding with no grant or permit.
- Retrying the same original operation preserves the complete committed row
  snapshot and original immutable receipt. Mint-only retry cannot commit an
  orphan; its failure retains that earlier receipt. A changed digest refuses.
- All three ordinary logins remain unable to mint, consume, write either work
  relation or call the guard, even after asserting the owner GUC context. The
  raw authority login also cannot call the guard directly. Administrator-only
  malformed standalone permit cases, including NULL transaction ID, cannot
  commit.

The immutable receipt here is the existing raw fixture's stored receipt row,
not a newly specified byte codec or a signed HTTP response. Raw same-operation
retry still follows that fixture's mint/consume path; this is not the separate
fresh-device receipt path's no-new-grant proof.

## Catalog and regression accounting

The pristine baseline measurement precedes the inherited executor overlay:
59 tables, 114 foreign keys, 122 triggers, 173 policies, 52 functions and 45
forced-RLS tables; fingerprint
`7228575621958f4f5063e40bbb80ec5102d00b232bf21d3fb95bad4779c9c638`.
The existing executor deliberately adds its candidate deletion/terminal guards.
Those inherited changes are not attributed to this new commit guard.

After installing that inherited overlay, the fixture snapshots actual baseline
constraint definitions/actions/deferral flags, function definitions/ACLs, and
trigger definitions/enabled state. They match exactly after the new overlay and
again after the test sequence. The new guard adds only `ll1_probe` objects. Existing
baseline `RESTRICT` rules and migration-file hashes remain unchanged. This is
not a claim that the inherited executor overlay itself has the pristine catalog.

The unchanged historical authority runner passed its own **46 cases** separately.
Its mint-only committed grant remains the documented historical limitation;
the new overlay is not installed by that runner and its old result is not
relabelled as fixed. Source hashes and full separate outputs are in the
[evidence record](ll1-authority-commit-guard-20260927.json).

```sh
python3 scripts/test_ll1_authority_commit_guard.py
python3 scripts/test_ll1_protected_authorization.py
```

Both commands use generated socket-only clusters and accept no live URL. The
new runner explicitly records that its cluster was stopped and removed. Its
Python syntax and scoped whitespace checks pass; no Rust source changed in this
slice. The inherited server disables fsync, so this establishes transaction and
constraint behavior, not crash or power-loss durability. The new guard has not
yet been composed with the signed HTTP fixture. Production DDL, authorization
custody, permanent encodings and exact COMMIT-acknowledgement interruption remain
separate work/review boundaries.
