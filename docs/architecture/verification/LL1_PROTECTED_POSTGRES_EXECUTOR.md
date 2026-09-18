# Disposable PostgreSQL protected executor evidence

Status: **partial engine proof; authorization/production gates remain**, 2026-09-18.
Shared semantic requirements: [executor contract](LL1_PROTECTED_EXECUTOR_CONTRACT.md).

Run `python3 scripts/test_ll1_protected_postgres_executor.py` from the repository.
The script initializes a generated private Unix-socket-only cluster, installs all
14 unchanged service migrations and actual runtime role definitions, then stops
and removes only that generated cluster. No live URL is accepted or used. Fixture
setup uses its private administrator; deletion executes as `photara_owner` through
a test-only SECURITY DEFINER function with a fixed search path. Runtime callers
are real non-superuser, non-BYPASSRLS, non-owner logins.

The catalog matches **59 tables, 114 FKs, 122 triggers, 173 policies, 52 functions,
45 forced-RLS tables** before the overlay. Source fingerprint:
`7228575621958f4f5063e40bbb80ec5102d00b232bf21d3fb95bad4779c9c638`.
Every migration byte hash is checked again after execution. Existing RESTRICT
constraints are neither altered nor disabled. No replication-role switch,
trigger disable, RLS disable, or table rebuild occurs.

## Measured result

The final run completed **24 cases**, including the held successful commit;
expected failures are matched against the first PostgreSQL ERROR line.

The 48-table reviewed deletion allowlist is captured as transaction-bound exact
full-row permits under the Library lock. Four existing guard functions receive a
disposable DELETE-only early return when relation, full OLD row and transaction
all match a private permit. Their original bodies remain for every other call.
Account/default/onboarding guards receive no exception. API, control, auth and
PUBLIC cannot write permits. The function has no SQL/callback/table-name/bypass
argument. It deletes the allowlist in one dependency-chained data-modifying CTE,
with the root last. PostgreSQL checks the unchanged RESTRICT edges at the complete
statement boundary.

The seeded actual-schema rows include both sides of each slot/name,
variable/name and accepted scoped-receipt/batch cycle, plus tombstones, changes,
subscriptions, default and unrelated Library/account/device/provider rows.
Committed deletion succeeds. Omitting each cycle's name/receipt rows from the
permit independently fails with the actual FK violation and rolls back every
baseline row. Thus this path does not convert RESTRICT into general deferral or
permit partial-cycle deletion.

Other passing checks cover viewer/cross-account/identity-mismatch denial, control
EXECUTE denial, permit-write denial, ordinary guard refusal, stale transaction
permit refusal, protected default refusal, stale revision terminal response and
replay, changed revision under the same operation refusal, rollback after complete
execution, terminal immutability and exact successful replay after membership
deletion. The original full 59-table row snapshot is checked after failures.
After commit it is compared with the exact expected removal set, retaining all
unrelated rows byte-for-byte as JSON values.

A disposable deferred root-delete trigger requires matching removed receipt,
marker and inventory rows and no remaining current-transaction permit. Fault
injection suppresses each terminal insertion separately and changes inventory's
Library coordinate: every attempt fails at commit and restores the aggregate.
Receipt omission also meets an immediate marker FK. This is stronger than relying
only on voluntary executor publication, but the terminal schema is a fixture,
not the proposed production codec or inventory channel.

Two additional connections attempt the same and a different operation while a
successful deletion transaction is held open. Both hit the bounded lock timeout;
after its commit the exact operation replay returns `removed`. This establishes
same-actor serialization for this fixture, not the full cross-account last-Library
or all-writer generation protocol.

## Explicit adverse evidence and limits

**Authorization is not fully proven.** The `sql_login_can_assert_owner_context_rollback`
case deliberately demonstrates that a holder of the API SQL login can set all
owner identity GUCs and execute this facade. Identity/account consistency checks
and SQL privileges work, but GUC values themselves are not authenticated actor
capabilities. Existing service code is expected to establish trusted actor context;
this experiment neither runs nor verifies that transport/service boundary. It is
not safe to expose this fixture function to untrusted SQL clients. No new trusted
credential or capability architecture is selected here.

The seeded rows are deliberately sparse across the full schema. The allowlist
includes indirect invitation secrets, scoped clients and claim receipts, but
those relations are empty here. This is not exhaustive populated-row closure for
all 48 tables, hidden/inactive project authorization shapes or canonical payload
references. Detached fixture evidence preserves the two populated receipt/billing
rows only; it is not a complete archive implementation or privacy review.

The facade does not implement last-active-Library admission, active bound device,
session quiescence, pending operations, exact-name confirmation, expiring review
tokens, request codecs/hashes, aggregate/access generations, client floors,
anti-resurrection admission for ordinary creates or real inventory delivery.
Historical billing is seeded as cancelled; the active-billing predicate exists
but its state matrix is not exercised. Synthetic receipt keys bind the tested
Library/revision pair only. These gaps prevent approval as a lifecycle executor.

The cluster uses the inherited fixture's `fsync=off`; rollback/transaction
atomicity is measured, not power-loss durability. No production migration or
deletion was performed. This evidence supports retaining current RESTRICT for
the PostgreSQL SQL strategy; it does not establish that every lifecycle/security
requirement can be met without a reviewed production overlay.
