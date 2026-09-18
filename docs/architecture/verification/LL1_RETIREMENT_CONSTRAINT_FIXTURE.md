# LL1 minimal retirement-constraint evidence

Date: 2026-09-18. **Disposable engine experiment only, not schema acceptance.**
Companion: [physical unnumbered proposal](../LL1_PHYSICAL_SCHEMA_AND_PRIVILEGE_REVIEW.md).
Harness: [test_ll1_retirement_constraints.py](../../../scripts/test_ll1_retirement_constraints.py).

## Observed results

| Engine actually exercised | Scope | Result |
| --- | --- | --- |
| Python SQLite 3.53.4 | In-memory database, `foreign_keys=ON`, STRICT tables | 38 cases passed: 19 for slots, 19 for variables |
| PostgreSQL 18.6 (Homebrew) | Newly initialized private disposable cluster, Unix socket only, TCP disabled | 40 cases passed: the same 38 plus wrong-transaction admission for each family |

The shell SQLite executable has a different version; it was not the engine used
by this harness. PostgreSQL initialization initially hit the sandbox's shared-memory
restriction. The successful runs used approved execution outside that restriction,
not a live service connection. Final temporary cluster:
`/private/tmp/photara-ll1-constraints-gxi27pn6`; stopped and removed after tests.
That disposable test data is no longer retained and can be recreated by the script.

Each family uses eight minimal tables. Current-name FK column order and actions
derive from [local slots](../../../crates/photara-library/migrations/generation_two/0008_storage_locations_bindings.sql),
[local variables](../../../crates/photara-library/migrations/generation_two/0009_library_context.sql),
[service slots](../../../crates/photara-service/migrations/postgres/0009_storage_locations.sql)
and [service variables](../../../crates/photara-service/migrations/postgres/0010_library_context.sql).
No production migration is loaded, modified, renumbered or executed.

## What the cases demonstrate

- With the baseline current-name `RESTRICT` backedge, deleting either the name
  first or parent first fails even inside a deferred transaction with a matching
  synthetic retirement manifest. Changing only that backedge to deferred
  `NO ACTION` permits names → parent → Library deletion; child→parent `RESTRICT`
  remains intact. Both slot and variable versions are exercised independently.
- Ordinary name, parent and Library deletes remain denied by the model guard.
  Wrong Library, object key, name or table entry cannot borrow the exception.
  PostgreSQL also rejects a manifest bound to another transaction ID.
- A work permit references impossible debt parent 1 while the parent CHECK allows
  only 0. Leaving that permit at commit fails both before deletion and after
  aggregate deletion/terminal publication; rollback restores the original rows.
  Forging debt parent 1 fails its CHECK. Successful retirement removes all work
  and manifest rows; subsequent ordinary deletes of the unrelated Library remain
  guarded. This establishes the candidate **no committed permit** integrity rule.
- Leaving a parent whose current name was deleted fails final FK closure. Every
  failed scenario checks the original aggregate and empty terminal/work tables.
  Successful retirement checks the target is absent, unrelated Library/parent/name
  values are preserved exactly, and no work/manifest rows survive.
- Terminal marker FK agreement is checked independently for authority, actor,
  operation, Library and outcome. Any mismatch fails commit and rolls back the
  deletion. Successful terminal rows reject UPDATE/DELETE; reusing the removed
  Library identity is rejected by the model insertion guard.
- SQLite runs `foreign_key_check` after each scenario. PostgreSQL commits enforced
  constraints, checks no unvalidated fixture constraint remains, and explicitly
  forces the current-name domain constraint while the permit exists, excluding
  its intentionally unsatisfied debt FK. This is not a test of all production
  deferred constraint triggers.

## Reproduce and isolation

```sh
python3 scripts/test_ll1_retirement_constraints.py
python3 scripts/test_ll1_retirement_constraints.py --postgres-disposable
git diff --check
```

The first command needs only stdlib SQLite. The second requires already installed
`initdb`, `pg_ctl` and `psql`; it installs nothing and accepts no connection URL.
It strips inherited PostgreSQL connection variables, disables psql startup files,
uses its own generated mode-private root/socket, checks `listen_addresses` is
empty and confirms the socket directory. Cleanup validates the exact generated
root/data/socket relationship before stopping that cluster and deleting its tree.
JSON-line stdout identifies each case and the successful cluster cleanup.
Do not run with Python assertions disabled. PostgreSQL uses `fsync=off` for this
constraint-only experiment; **no crash/power-loss durability is claimed**.

## Unproved gates and security limits

The fixture deliberately uses small INTEGER identities, simplified payloads,
handwritten model guards and a privileged disposable PostgreSQL owner. It omits
production incoming FK edges, most fields/indexes, table rebuilds, RLS, roles,
SECURITY DEFINER facades, permit grants, canonical hashes and authorization.
It is **not** the full 79/59-table graph or 190/122-trigger replacement proof.
SQLite foreign-key-enabled migration/cutover remains unproved.

In particular, the debt FK is not authority: code already allowed to insert permit
rows can create an exact manifest and remove it before commit. The fixture's
successful runner explicitly publishes the receipt/marker; the minimal model does
not prove every privileged deletion must publish them. Production must confine
permit creation/deletion to the complete reviewed retirement facade and prove
that facade's receipt/marker/inventory publication and actual role/RLS denial.
Likewise, marker→receipt agreement is not a reverse obligation that every receipt
has a marker. No client DELETE privilege can be justified from these tests.

Full logical closure of the two local non-FK tables, embedded historical IDs,
immutable authentication exceptions, generation/lock coverage, stale restore,
inventory/privacy, PS3/PS4 activation cleanup and LL0 owner/default/last/billing/
confirmation policies are not exercised. The script never receives package/source
paths, but that is not a production filesystem-isolation proof.

No new accepted-contract conflict was found. The four proposed FK changes,
SQLite rebuild, narrow executor/RLS policy and complete schema/trigger/privilege
acceptance remain explicit review gates; no LL0 guarantee has been weakened.
Production migration source fingerprints still equal the physical review baseline:
SQLite `02c638bb087862adb764f28d6c3caf86c812a71b28da0170bc409f3a1211d220`;
PostgreSQL `7228575621958f4f5063e40bbb80ec5102d00b232bf21d3fb95bad4779c9c638`.
No live Neon/service access, install/deploy, commit or push occurred.
