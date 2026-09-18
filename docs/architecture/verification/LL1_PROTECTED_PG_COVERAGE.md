# LL1 full-table PostgreSQL protected deletion coverage

Status: disposable implementation evidence, 2026-09-18. No production deletion,
deployment, migration, changed FK action, disabled trigger, or production URL.

Run `python3 scripts/test_ll1_protected_pg_coverage.py`. The fixture creates and
removes a private temporary PostgreSQL cluster and loads all unchanged service
migrations. On this host local cluster initialization requires execution outside
the filesystem sandbox. The final run passed 19 cases.

The actual catalog has 59 tables, 114 foreign keys, 122 triggers, 173 policies,
52 functions, and 45 FORCE RLS tables. Inventory fingerprint:
`7228575621958f4f5063e40bbb80ec5102d00b232bf21d3fb95bad4779c9c638`.
The fixture verifies migration file hashes remain unchanged.

## Coverage and discovered implementation gap

All 48 reviewed Library-owned tables contain target rows: 53 target rows total;
**no unseeded owned tables**. Memberships, storage slots and their names, and
Library variables and their names have two rows each. Every other owned table
has one. The script emits exact per-table counts and fails if any owned table
lacks a target row.

The sparse executor overlay initially failed with `term_reserved` from
`photara_private.guard_kind_term` when deleting the populated
`photara.location_kind_terms`. This failure and complete rollback are retained
as the first regression test. The dense coverage overlay adds the same narrow
exception already used by the executor's other four guards: only DELETE, only
the exact JSONB OLD row and qualified relation recorded in the private permit
table, and only the current transaction. The original function body, privileges,
search path and ordinary update behavior remain. This was the only additional
guard exception needed. No RESTRICT edge needed modification.

The additional exception is tested against ordinary deletion, immutable term
identity update even with a matching deletion permit, stale transaction,
wrong relation, wrong row content, and a different Library whose row is visible
to the test actor. All remain denied. The exact matching DELETE permit succeeds
inside an explicitly rolled-back test transaction. Administrator-only permit
fault injection is confined to the disposable cluster; ordinary runtime logins
cannot create permits.

## Closure and rollback evidence

Normal enabled triggers and constraints validate seed construction. Fixtures
include hidden catalog entries, retired locators, tombstoned people,
organizations, social profiles, relationships, location kinds, locations,
storage roots/slots, variables and media links; revoked project grants and
invitations; expired uploads; cancelled subscription and revoked entitlement;
accepted old and scoped mutation feeds; media objects; security audit; claim
receipts; expression dependencies; and synchronization clients.

The three RESTRICT cycles have real rows: storage-slot/name, variable/name,
and scoped-receipt/change-batch. Removing a required member from the permit
set fails on FK enforcement and preserves the complete baseline snapshot.
The same negative test covers invitation secrets, scoped sync clients, and
Library claim receipts, whose ownership is indirect or uses a differently
named Library column. Location-kind/term and old mutation-receipt/batch
deferred cycles also have real rows and delete successfully without changes.

The exact expected closure is computed independently from the pre-delete
snapshot. Invitation IDs and stream/epoch pairs are captured before parent
deletion; post-delete disappearance of the parent cannot hide residual child
rows. After authorized commit every one of 59 tables is compared with its
original rows minus that exact closure. All 53 target rows disappear and every
other row is identical, including another Library's active kind/term pair,
memberships, default reference, accounts, identities, devices, billing event,
schema metadata and normalization policy.

An explicit rollback after executing the populated aggregate preserves the
entire baseline. Suppressing terminal inventory publication after aggregate
deletion causes commit-time `terminal_closure`, restores the entire baseline,
and leaves no permits. Successful commit has one removed receipt, marker and
inventory record and no permits. Fresh trusted authorization can replay the
terminal result after ownership rows have been deleted without further data
changes. Forging owner context through the ordinary API SQL login cannot
invoke the protected entry point.

## Limits

This is full-table row coverage, not exhaustive data-state, scale or concurrency
coverage. It does not exercise every enum alternative, nullable FK combination,
merged redirect chain, duplicate cardinality, or every unrelated table with
populated rows. Most unrelated preservation witnesses remain the sparse
baseline, strengthened here by a second Library's kind/term pair. Separate
executor and authorization fixtures retain stale/idempotent, role denial and
concurrent lock evidence; this suite does not repeat that entire matrix.

The fixture uses the authority-role boundary from
`test_ll1_protected_authorization.py`; production authenticated credential
verification, canonical review format, generation binding, authority connection
isolation and durable retirement publication remain subject to the companion
authorization/design evidence. This report demonstrates that the populated
actual schema supports the proposed transaction/guard implementation. It does
not grant production deployment approval or validate external object cleanup.
