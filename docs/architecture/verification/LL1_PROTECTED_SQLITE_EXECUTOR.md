# LL1 disposable SQLite protected executor diagnostic

Status: **21 executable cases pass on SQLite 3.53.4; cycle feasibility proved;
the complete shared lifecycle/security contract is not yet proved.**

Run from the disposable checkout:

```sh
python3 scripts/test_ll1_protected_sqlite_executor.py
```

The script accepts no database path or endpoint. It creates its own temporary
directory, opens only a fresh SQLite database there, closes both connections and
removes the directory. It applies all 15 generation-two migrations unchanged,
then checks the actual catalog against the lexical inventory: 79 tables, 137
foreign keys and 190 triggers. Baseline fingerprint:
`02c638bb087862adb764f28d6c3caf86c812a71b28da0170bc409f3a1211d220`.
All migration bytes are hashed again at completion. All baseline table DDL is
compared unchanged after the disposable overlay is installed.

## What resolves the SQLite execution question

Both current-name cycles can be deleted with their existing `RESTRICT` actions.
The executor takes `BEGIN IMMEDIATE`, rechecks the Library revision and computes
an exact row manifest. It updates only the target slots' and variables'
`current_name` to a valid, intentionally unbound name, incrementing each revision
once. **All existing update, admission, CAS and immutability guards remain
unchanged.** The backlink constraints already say `DEFERRABLE INITIALLY DEFERRED`.
The temporary missing referenced name is therefore permitted until commit.

After this change the old name rows are no longer referenced by those backlinks.
Deleting name rows and then their parents satisfies the existing immediate
`RESTRICT` delete actions. The executor removes the remaining aggregate rows and
validates the final state before commit. It never toggles `foreign_keys` off,
enables `defer_foreign_keys`, changes a FK action, disables all triggers, nulls an
identity, changes receipt outcomes, or leaves aggregate rows behind.

This is an engine-specific execution candidate for the
[shared semantic contract](LL1_PROTECTED_EXECUTOR_CONTRACT.md), not evidence that
SQLite needs `NO ACTION`. The intermediate unbound name is visible on the writer
connection only. A second connection observes the original names and cannot
start a write transaction. Attempting to commit the intermediate state fails
the real deferred FK; injected failure rolls back names, revisions and all
deletions together. The prototype retries individual `RESTRICT` failures to
derive a valid order for the seeded rows; it is not a proposed production planner.

## Exact scope of the disposable overlay

The migrations contain unconditional retained-row deletion guards. The fixture
qualifies their delete rejection with a connection-local UDF that checks an
in-memory exact `(table, rowid)` manifest and an active write transaction. A
second guard rejects deletion outside that manifest during the operation,
including tables without an existing delete guard. SQL cannot manufacture the
Python permit. The permit is cleared in `finally` on success or rollback.

The trusted caller is represented by an opaque Python object, held inside this
fixture. This is **a model of the private executor boundary, not a proof of
production authentication, SQLite database-owner isolation or security against
code holding the raw connection**. SQLite has no server role/privilege boundary.
Ordinary guarded deletion still fails before and after protected execution.

The fixture also adds three disposable immutable terminal relations: receipt,
removed-Library marker and local inventory event. Marker and event references
must match receipt operation, Library and request digest. An explicit final join
check rejects missing or mismatched evidence; all three publish atomically.
These tables are evidence models, not selected production historical-evidence
representations or migrations.

The deletion target is the cloned, non-default Library. The default Library and
its complete seeded clone remain. Manifest traversal follows the actual FK
catalog, adds the two typed Project ownership joins for
`device_context_snapshots` and `legacy_external_resource_resolutions`, and
independently checks every direct target `library_id`. The executor refuses any
nonempty manifest table outside its explicitly tested 14-table allowlist. FK
reachability alone never authorizes another table's cleanup.

## Executed boundaries

| Boundary | Observed evidence |
| --- | --- |
| Authorization | Wrong opaque actor rejected before transaction; ordinary retained Library deletion rejected. |
| Narrow exception | Missing manifest row rejects; unrelated Library deletion rejects; ordinary guard remains after success. |
| Seeded closure | All 18 target rows in 14 tables removed, including two rows per current-name cycle, tombstones, hidden/closed Project catalog data and both logical tables. Every baseline table's remaining rows equal exact precomputed set subtraction. |
| Atomic rollback | Injected failure after temporary rename, after aggregate removal and after receipt insertion restores every baseline row and all terminal tables. |
| Partial cycle | Commit of unbound current-name state fails actual deferred FK and rolls back. |
| Logical completeness | Deliberately omitted non-FK rows prevent completion and force rollback, even though FK checking alone cannot detect their omission. |
| Terminal agreement | Missing marker, wrong marker digest and wrong inventory digest each roll back the entire removal. |
| Idempotence | Exact same operation/digest returns original receipt; changed digest refuses; fresh operation after removal refuses. |
| Stale revision | Wrong reviewed Library revision refuses before cleanup. |
| Concurrency | Independent write holder causes `database is locked`; active executor excludes a second writer; retry after winner returns original receipt or refuses a fresh operation. |
| Isolation | Concurrent reader sees original names; unrelated Library, device/identity rows and all unselected baseline data remain byte-for-byte equal as represented by SQLite values. |

## Remaining review gates

This diagnostic proves a viable unchanged-`RESTRICT` SQLite cycle strategy and
atomic behavior for its populated fixture. It does not establish complete
79-table aggregate coverage: 65 baseline tables are present but unpopulated or
outside the supported deletion allowlist. It does not exercise all other cyclic
components, payload/codec references, stale restores, package contents, recovery
and onboarding terminal evidence, pending work, billing, account/default/last
eligibility comprehensively, or aggregate generation invalidation by every writer.
The two logical joins prove typed Project IDs only, not references embedded in
canonical payloads. No filesystem, Keychain or remote deletion is attempted.

Real actor/device authorization, review expiry, locked authorization generations,
private connection ownership, durable historical receipt/marker/inventory design,
and production guard installation need review and integration evidence. The
candidate introduces narrow **delete guard exceptions** and fixture-only terminal
tables; those must not be misreported as already available in current schema.
No update guard or referential constraint change was needed by this experiment.
The experiment is not approval to install guards, migrate a user database or
enable production Library deletion.
