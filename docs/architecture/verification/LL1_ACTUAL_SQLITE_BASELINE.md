# LL1 actual SQLite baseline — partial verification

Status: **staged evidence, 2026-09-18; not successful retirement or full LL1 acceptance**.
Source checkpoint: `24499cf510a31edf179f628874db84d0ed286261`.
Harness: [test_ll1_actual_sqlite_baseline.py](../../../scripts/test_ll1_actual_sqlite_baseline.py).
This is separate from the [actual PostgreSQL baseline](LL1_ACTUAL_POSTGRES_BASELINE.md).
It loads all actual local schema objects, but does not exercise every edge or
trigger branch, and supplies no lifecycle executor or retirement exception.

## Verified result

SQLite **3.53.4: 24 behavioral cases and four logical-join checks passed**.
All 15 unchanged [generation-two migrations](../../../crates/photara-library/migrations/generation_two)
execute in a new `:memory:` database with foreign keys enabled. Installed catalog
matches the source inventory: **79 tables, 137 foreign keys, 190 triggers**.
Checks compare exact table names, trigger/table pairs, normalized table/trigger
SQL, and every FK's child/parent column tuples and delete action. SQLite's FK
PRAGMA omits deferral metadata; exact CREATE TABLE SQL comparison covers source
deferral declarations rather than pretending the PRAGMA reports them.

Committed synthetic seed rows cover two local-only Libraries/controllers, a
tombstoned Person/root, closed Project ownership, hidden catalog/retired locator,
two tombstoned rows per slot/name and variable/name cycle, local bookmark binding,
device context snapshot, and legacy external-resource resolution. Bookmark handles
are synthetic bytes; locator paths are NULL. No package or source path is opened.
The default Library and database-scoped principal use the hash domains in
[local.rs](../../../crates/photara-library/src/gen2/local.rs) (lines 80–110).
That fixture derivation does not call or verify Rust default-creation/admission.
Metadata is initialized at the current 5/5 reader/writer floors; the application
migration installer and compatibility rejection paths are not exercised.

| Behavioral group | Cases | Observed result |
| --- | ---: | --- |
| Ordinary aggregate/retained-row DELETE | 13 | Root, controller, Person, Project/catalog/locator, root/slot/name, variable/name and both logical context tables refuse with actual `hard_delete_not_supported` or `d19_retained_record` guards |
| Local binding leaf DELETE | 1 | `device_root_bindings` deletion is allowed; a real change is verified and rolled back |
| Retention/stale/generation/lifecycle | 5 | Onboarding deletion, unchanged onboarding generation, stale Library revision, controller generation jump and reopening closed Project all refuse |
| Ordinary update rollback | 1 | Valid Library rename makes a real change, then rolls back |
| PostgreSQL-style data-modifying CTE | 4 | Both orderings for each current-name cycle fail `near "DELETE": syntax error` |

After every behavioral case, all 79 tables' complete logical-row snapshots equal
the committed seed, including unrelated and historical rows; `foreign_key_check`
is empty. Successful ordinary changes are also checked for FK validity before
rollback. These are denial/rollback and seed-consistency checks, **not committed
retirement/no-orphan closure**. No failed case is silently treated as success.

The allowed [device root binding](../../../crates/photara-library/migrations/generation_two/0003_catalog_device.sql)
leaf deletion is baseline behavior, not an invented guard bypass or authority to
delete a Library. SQLite has no service roles/RLS; this direct SQL fixture does
not establish application/controller authorization.

## Unchanged RESTRICT: exact engine boundary

Both populated current-name cycles retain their actual FK actions:

- [Storage slots/names](../../../crates/photara-library/migrations/generation_two/0008_storage_locations_bindings.sql), including the current-name backlink at line 41.
- [Library variables/names](../../../crates/photara-library/migrations/generation_two/0009_library_context.sql), including the current-name backlink at line 33.

Actual [D19 guards](../../../crates/photara-library/migrations/generation_two/0012_d19_guards_and_floor.sql)
deny ordinary DELETE first. Additionally, the PostgreSQL miniature's exact
single-statement form does not parse here:

```sql
WITH a AS (DELETE FROM storage_slots WHERE library_id = ? RETURNING library_id)
DELETE FROM storage_slot_names WHERE library_id = ?;
```

Both directions, and the equivalent variable/name forms, are tested against the
actual schema. This establishes that this PostgreSQL technique is **not portable
to the tested SQLite engine**; it is not a proof that every possible SQLite
retirement strategy is impossible. No FK rewrite, `defer_foreign_keys`, trigger
disable, cascade overlay or candidate executor is tried. The existing
[minimal constraint fixture](LL1_RETIREMENT_CONSTRAINT_FIXTURE.md) remains separate
evidence, not a substitute for actual-schema committed closure.

## Logical closure and remaining decision gate

`device_context_snapshots` and `legacy_external_resource_resolutions` have Project
coordinates but no Project/Library FK. Four explicit joins through
`project_ownership(project_id, library_id)` identify one target and zero unrelated
rows for each table. Both rows are retained by actual D19 guards. This proves the
seeded typed relational route only, not classification of all historical payloads,
codec validity, hidden survivors or completeness under concurrent mutation.
Sources: [snapshots](../../../crates/photara-library/migrations/generation_two/0010_context_apply_recovery.sql)
and [legacy resolutions](../../../crates/photara-library/migrations/generation_two/0008_storage_locations_bindings.sql).

No lifecycle receipt, removed-identity marker, account inventory position or
retirement-work relation exists in this baseline (absence is asserted). Successful
retirement therefore still requires separately reviewed physical/authority design:
exact guarded exceptions, an engine-compatible closed deletion strategy, mandatory
terminal receipt/marker/inventory agreement, and all protected admission rules.
**Stop at this existing LL1 physical-design gate; no strategy is selected here.**

Still unproved: committed whole-aggregate closure, every FK/trigger branch (including
catalog-selected locator/observation paths), terminal agreement and retries,
concurrent lifecycle fencing/lock order, complete generation coverage, default or
last-owned admission, controller transfer, billing/cloud authorization, exact-name
confirmation, compatibility-floor rejection, privacy/codecs, and PS3/PS4 activation.
The controller jump guard is not proof of a general aggregate-generation counter.
The fixture does not test crash/power-loss durability or actual Rust session paths.
It changes no accepted LL0 behavior and makes no production durability claim.

## Reproduction and integrity

```sh
python3 -B scripts/test_ll1_actual_sqlite_baseline.py
git diff --check
```

The harness accepts no database/path arguments, opens only an in-memory connection,
and closes it in `finally`; no database file or cluster cleanup is needed. Source
parsing uses the [bounded static analyzer](../../../scripts/audit_ll1_schema_inventory.py),
not a general SQL parser; live catalog/normalized declaration comparison checks
its interpretation for this baseline. Each migration SHA-256 is emitted and
rechecked unchanged after the run. Source-set fingerprint:

`02c638bb087862adb764f28d6c3caf86c812a71b28da0170bc409f3a1211d220`

No production schema/code, live local DB, Neon/service, source/package, PS2,
ROADMAP, handoff, commit or push is touched by this checkpoint.
