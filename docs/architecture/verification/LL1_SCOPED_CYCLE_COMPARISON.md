# LL1 scoped receipt/batch FK comparison

Status: **disposable PostgreSQL evidence, 2026-09-18; no option selected**.
User-authorized comparison against source at
`06bbfcd4b744e3c1ee6f1d60255d67f447d5d8e7`.
Harness: [test_ll1_scoped_cycle_comparison.py](../../../scripts/test_ll1_scoped_cycle_comparison.py).
Prior finding: [LL1 full-schema gaps](../LL1_FULL_SCHEMA_GAPS.md).

## Result

**87/87 cases passed on PostgreSQL 18.6 (Homebrew): 29 per FK option.**
Pass includes expected refusals and verified rollback, not 87 successful deletions.

| Fixture FK option | Receipts, then batches: separate statements | Batches, then receipts: separate statements | One statement, receipt-returning CTE dependency | One statement, batch-returning CTE dependency |
| --- | --- | --- | --- | --- |
| Baseline: both RESTRICT | FK refusal | FK refusal | Commit | Commit |
| Only receipt→batch becomes deferred NO ACTION | FK refusal | Commit | Commit | Commit |
| Only batch→receipt becomes deferred NO ACTION | Commit | FK refusal | Commit | Commit |

All successful cases first remove dependent change rows and afterwards remove
the target stream and Library, publish agreeing terminal evidence, check the two
domain FKs, clear transaction work and commit. Every case compares complete
logical row snapshots for the unrelated Library across all five domain tables.
Failure cases verify exact target restoration and absence of terminal/work rows;
successful cases verify no target rows remain and terminal evidence is immutable.

The single-statement result is important: **this minimal cycle does not itself
force an FK rewrite**. The harness's two CTE variants both retire the pair inside
one SQL statement while keeping both RESTRICT actions. This is evidence about
these exact statements and engine version, not a guarantee about arbitrary CTE
execution/row order or the full production trigger/RLS configuration. No strategy
is selected or authorized for production by this result.

## Exact source mapping and tested statement shape

The relevant baseline declarations are unchanged:

- [0011_scoped_sync.sql:118](../../../crates/photara-service/migrations/postgres/0011_scoped_sync.sql):
  `scoped_command_receipts(accepted_stream_id,accepted_epoch,accepted_sequence,library_id)`
  references `scoped_change_batches(stream_id,epoch,sequence,library_id)`,
  `ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED`.
- [0011_scoped_sync.sql:129](../../../crates/photara-service/migrations/postgres/0011_scoped_sync.sql):
  `d19_batch_receipt`, `scoped_change_batches(library_id,operation_id)` references
  `scoped_command_receipts(library_id,operation_id)` with the same action/deferral.

The miniature schema preserves those composite keys, the accepted-coordinate
presence rule, and incoming changes→batch and batch→stream/Library dependencies.
Names live in `ll1_fixture`; INTEGER identities and payload text stand in for
production UUIDs and canonical byte records. The CTE comparison uses this shape
(and its converse); it is fixture SQL, **not a proposed production facade**:

```sql
WITH retired_receipts AS (
  DELETE FROM scoped_command_receipts WHERE library_id=1
  RETURNING library_id
), retired_batches AS (
  DELETE FROM scoped_change_batches WHERE library_id=1
    AND (SELECT count(*) FROM retired_receipts)>=0
  RETURNING library_id
)
SELECT (SELECT count(*) FROM retired_receipts)
     + (SELECT count(*) FROM retired_batches);
```

`SET CONSTRAINTS receipt_to_batch,batch_to_receipt IMMEDIATE` checks domain closure
while the permit still exists; the intentionally unsatisfied work-debt FK is not
forced until work has been removed. No accepted receipt fields are nulled or
rewritten to break the cycle; the fixture explicitly denies that UPDATE attempt.

## Case inventory per option

| Group | Cases | Required observed outcome |
| --- | ---: | --- |
| Deletion strategy comparison | 4 | Exact success/refusal matrix above |
| Ordinary deletes | 5 | Root, stream, batch, receipt and change all deny without manifest |
| Wrong/expired permits | 8 | Wrong Library, actor, request, transaction, expired deadline, mismatched row key/hash or missing table entry deny |
| Mid-transaction/closure failures | 6 | Expiration after leaf deletion, omitted change closure, wrong CTE Library, committed permit debt, injected division failure after cycle deletion, accepted-coordinate UPDATE all refuse/roll back |
| Terminal agreement | 6 | Authority, actor, operation, Library, request or outcome mismatch fails commit and rolls back deletion |

Post-success checks additionally deny unrelated deletes and terminal UPDATE/DELETE.
The permit binds current transaction ID, target, fixed reviewed actor/request,
deadline and exact table/key manifest. The deadline is deliberately expired by
injection—no sleep/timing race is required. These predicates model integrity;
they are **not authenticated capabilities or an approved expiration policy**.

## Reproduction, isolation and cleanup

```sh
python3 -B scripts/test_ll1_scoped_cycle_comparison.py
git diff --check
```

The harness reuses the [prior disposable PostgreSQL lifecycle](../../../scripts/test_ll1_retirement_constraints.py),
requires installed `initdb`, `pg_ctl`, `psql`, and accepts no connection URL or
database target. It strips inherited `PG*` environment variables, disables psql
startup files, creates a private generated data/socket root, disables TCP, verifies
socket/listener settings, and validates the exact generated root before cleanup.
It needs permission to allocate PostgreSQL shared memory; the run used approved
execution outside the sandbox restriction. `fsync=off` is inherited for this
constraint experiment, so **no crash/power-loss durability is claimed**.

Observed cluster `/private/tmp/photara-ll1-constraints-ma38nfzw` was stopped and
removed after the successful run. The disposable database is not retained;
reproduction creates another isolated cluster. JSON-line stdout records each
case, expected outcome, totals, root and cleanup. No installation or deployment.

## Limits and next review boundary

This is a five-domain-table subset with model guards and one accepted receipt/
batch/change per Library, not the complete 59-table schema or multi-batch stress
test. Production `d19_immutable`, `d19_feed_at_commit`, owner/manager guards and all
122 triggers are **not installed or exercised**. Their shared functions, deferred
visibility and full incoming edges remain separate proof work.

The fixture runs as the isolated database owner; it does not exercise production
FORCE RLS, application roles, SECURITY DEFINER owners/search paths, hidden-row
effects, prepared-token authorization, lock ordering or concurrent writers.
Changing an FK action would affect every writer and needs compatibility review;
keeping actions via a CTE would constrain the protected retirement statement and
needs full trigger/privilege/row-completeness proof. The CTE result does not relax
the requirement that only the complete reviewed facade may create/use permits.

The caller still explicitly publishes matching terminal receipt/marker; neither
the debt FK nor their one-way FK proves that arbitrary privileged deletion must
publish them. Retained account evidence, inventory events, payload detachment,
privacy and local/session cleanup remain unproved. No option may weaken LL0
no-orphan, immutable-evidence or file-isolation guarantees.

No new accepted-contract conflict appeared in these minimal cases. Stop here for
review of the comparison and authority for any larger fixture; do not infer a
numbered migration or LL2 wiring decision. Production schema fingerprints remain:
SQLite `02c638bb087862adb764f28d6c3caf86c812a71b28da0170bc409f3a1211d220`;
PostgreSQL `7228575621958f4f5063e40bbb80ec5102d00b232bf21d3fb95bad4779c9c638`.
No Neon/live service, package/source deletion, production edit, commit or push.
