# LL1 protected Library-deletion verification contract

Status: disposable verification specification, not a production executor,
migration, or authorization to delete a Library. Read with the accepted
[Library lifecycle contract](../LIBRARY_LIFECYCLE.md) and the
[full-schema gap audit](../LL1_FULL_SCHEMA_GAPS.md).

The PostgreSQL and SQLite experiments may use different statement shapes, but
must prove the same externally visible behavior. Both load every unchanged
current migration, keep all existing `RESTRICT` foreign-key actions and foreign
key enforcement active, and leave production and user databases untouched.
Additional test-only roles, tables, functions, guards, or authorizer hooks must
be identified as a **candidate overlay**, never misreported as existing schema.

## Shared preconditions and transaction boundary

1. Identify a non-default, non-last-owned Library, its initiating account or
   local controller, device, exact reviewed name, revision and aggregate
   generation. Reject a wrong actor, expired/mismatched review, pending work,
   active billing obligation, default/last Library, or stale version before any
   aggregate cleanup. Receipt lookup for an exact retry precedes review expiry
   but still verifies the initiating actor and device.
2. Serialize with every writer that can change eligibility or aggregate scope.
   Acquire locks in the established account/Library order, then recheck the
   complete manifest, authority and reviewed generation inside the write unit.
   A test-only privilege/guard exception must bind to precisely one authorized
   operation, target Library, transaction and reviewed table/row scope; ordinary
   SQL and other actors retain existing denial behavior.
3. In one atomic unit, detach only required historical evidence, remove the
   exact target aggregate (including tombstones, hidden catalog entries,
   retained rows and non-FK logical references), clear live selection/access,
   and durably publish the matching terminal receipt, removed-ID marker and
   account inventory position/event. The protected permit/work state must not
   survive commit. No package, source file, Keychain item or remote object is
   touched.
4. Before commit, prove foreign-key validity, no target aggregate survivor in
   any classified relation, unchanged unrelated Libraries and retained
   account/device rows, and terminal evidence agreement. An injected error at
   every phase must roll back *all* target and terminal changes. A retry with
   the same operation ID and request digest returns the original receipt;
   conflicting digest/actor or a fresh operation against the removed ID refuses.

## Required evidence matrix

| Boundary | PostgreSQL | SQLite |
| --- | --- | --- |
| Actual full schema | All current service migrations, roles, effective grants, forced RLS and enabled triggers | All current generation-two migrations, FKs and triggers |
| Ordinary denial | API/control/read roles cannot directly delete or manufacture a permit; unrelated actor sees no private details | Ordinary connection cannot invoke protected path or delete retained rows |
| Narrow exception | Exact protected executor only; no `BYPASSRLS`, trigger disable, broad DELETE or ambient session flag | Exact protected connection/transaction only; no global FK disable or broad authorizer escape |
| Dependency closure | Every FK and logical reference, including the scoped receipt/batch `RESTRICT` cycle | Every FK and logical reference, including both current-name `RESTRICT` cycles |
| Transactional terminal state | Receipt, marker, inventory and cleanup agree or all roll back | Receipt, marker, local selection/inventory and cleanup agree or all roll back |
| Concurrency | Same-operation retry, two competing removers, stale review, writer/target locks | Competing connections, stale review and writer/target serialization |
| Isolation | All unrelated Library/account rows and external package/source canaries unchanged | Same, including device-local non-target rows |

Passing a miniature cycle, a denial baseline, or a syntax probe is not successful
whole-aggregate retirement. If a candidate needs a changed constraint, altered
security boundary, new persistent historical-evidence representation, or a
relaxed lifecycle rule, stop for review and record the smallest failing case.
Do not convert a missing proof into a production migration recommendation.

## Disposable execution outcome

The [PostgreSQL candidate](LL1_PROTECTED_POSTGRES_EXECUTOR.md) passes 24 cases
against all 14 migrations, with populated instances of all three mutual
`RESTRICT` cycles. One dependency-chained data-modifying statement closes the
seeded aggregate; incomplete cycles, missing/mismatched terminal evidence and
concurrent/stale attempts refuse or roll back. The full catalog is installed,
but the populated fixture is sparse. Critically, the API SQL login can assert
owner identity through caller-set context settings. The trusted authenticated
service boundary is **not** established by that SQL prototype.

The [SQLite candidate](LL1_PROTECTED_SQLITE_EXECUTOR.md) passes 21 cases against
all 15 migrations, removing 18 seeded rows in 14 allowed tables. It breaks the
two current-name cycles through existing deferred child-FK checks using
transaction-local, revisioned temporary names, then deletes under unchanged
`RESTRICT`. No global FK deferral or disablement is used. The caller and
terminal relations are fixture models, and most baseline tables are unpopulated.

These results support **retaining the current RESTRICT actions** for further
engineering. They do not prove full lifecycle authorization or closure, and
they require reviewed narrow guard exceptions plus terminal evidence designs.
No production executor, migration, live deletion or `NO ACTION` change follows
from this experiment. Stop for the security/physical-design review before LL2b.
