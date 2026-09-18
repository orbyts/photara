# LL1 actual PostgreSQL baseline — partial verification

Status: **staged evidence, 2026-09-18; not full LL1 retirement acceptance**.
Source baseline: `e4476257c610299a5835d59444545c4278530082`.
Harness: [test_ll1_actual_postgres_baseline.py](../../../scripts/test_ll1_actual_postgres_baseline.py).
Only a private disposable PostgreSQL cluster was exercised. **SQLite's local
79-table/190-trigger behavioral closure was not exercised and remains separate.**

## Verified result

**30 behavioral cases passed**, plus actual catalog-to-source checks:

| Installed catalog | Verified count / comparison |
| --- | --- |
| Tables | 59 exact names, all owned by `photara_owner` |
| Foreign keys | 114 exact source/target column tuples, delete actions and initial deferral; all validated |
| User triggers | 122 exact relation/name pairs; all enabled |
| Current policies | 173 exact table/name pairs |
| Functions | 52 exact names, all owned by `photara_owner` |
| Forced RLS | 45 tables; enable/force flags match source inventory |
| Runtime logins | Actual API/control/auth login roles; non-superuser/non-bypass, expected group membership, no owner membership |

All 14 unchanged service migration files execute as `photara_owner`; no FK,
trigger, policy, grant or function is rewritten. The harness applies source SQL
directly, with a fixture migration ledger and synthetic metadata/normalization
initialization at the same baseline split as [Service::migrate](../../../crates/photara-service/src/lib.rs).
It does **not** run SQLx's migration installer or the application/API process.
Runtime logins use the repository's [role template](../../../crates/photara-service/deploy/runtime_roles.sql).

Real committed fixture rows include owner/viewer/unrelated accounts, two Libraries,
default ownership, active billing, a tombstoned Person, hidden Project catalog,
retired locator, accepted scoped receipt/batch/change, and two tombstoned rows per
slot/name and variable/name cycle. No fixture path resolves to a package or media.

| Behavioral group | Cases | Observed outcome |
| --- | ---: | --- |
| Runtime authorization/RLS | 6 | Owner manage-access succeeds; viewer read succeeds but manage-access fails; unrelated Library hidden; mismatched identity rejected; auth-only login denied domain read |
| Runtime root DELETE | 2 | API and control logins lack permission |
| Owner ordinary DELETE | 9 | Library, tombstone, catalog, locator, billing, contract and scoped feed rows hit actual no-delete/immutable guards |
| Default protection | 2 | Default deletion and reassignment hit immutable onboarding guard |
| Owner/revision/evidence guards | 3 | Removing last active owner, stale Library revision and nulling accepted receipt coordinates refuse |
| Actual CTE cycle attempts | 6 | Both directions for all three cycles refuse under unchanged retention guards |
| Rollback and concurrency | 2 | Valid ordinary update rolled back; independent session contending for locked root hits bounded lock timeout; holder rolls back |

After **every** behavioral case, superuser inspection compares full logical row
snapshots of all 59 tables to the committed fixture baseline. No target, unrelated
or historical row changed. This is rollback/denial evidence, not successful
deletion or adversarial RLS closure proof. Catalog matching does not dynamically
exercise every FK edge, trigger branch or policy predicate.

## Exact blocker: three cycles, not just scoped feed

| Cycle | Existing source and observed blocker |
| --- | --- |
| Scoped receipt ↔ batch | [0011:118,129](../../../crates/photara-service/migrations/postgres/0011_scoped_sync.sql), both RESTRICT. Actual CTE attempts hit `d19_immutable`. |
| Storage slot ↔ reserved name | [0009:49–61](../../../crates/photara-service/migrations/postgres/0009_storage_locations.sql), both RESTRICT. Populated two-row tombstone pairs hit D19 retention guards. |
| Library variable ↔ reserved name | [0010:63–75](../../../crates/photara-service/migrations/postgres/0010_library_context.sql), both RESTRICT. Populated two-row tombstone pairs hit D19 retention guards. |

The [87-case miniature comparison](LL1_SCOPED_CYCLE_COMPARISON.md) showed that a
single data-modifying CTE can close its minimal scoped cycle without changing
RESTRICT. It did not install these actual guards. In this full-schema baseline,
[D19 immutable/mutable functions and consumers](../../../crates/photara-service/migrations/postgres/0012_d19_access_guards.sql)
deny deletion even as the non-superuser schema owner; runtime roles fail earlier
at privileges. These findings coexist: FK technique feasibility is not retirement
authority or whole-aggregate success. No sequential SPI deletes disguised inside
an outer SELECT are substituted for the tested multi-table CTE shape.

## Required overlay boundary: stop for security-design review

The baseline contains no lifecycle receipt, removed-identity marker, account
inventory event or retirement-work relation. A successful actual-schema retirement
therefore cannot be proved merely by rerunning a different DELETE statement.
No candidate executor or guard bypass was invented in this checkpoint.

A separately reviewed disposable candidate would have to define **all** of:

1. Exact executor role, credentials/trust boundary, table/function grants,
   SECURITY DEFINER ownership/search paths, protected transaction permits and
   forced-RLS policies; ordinary clients must remain unable to mint capabilities.
2. Closed table/row-specific DELETE exceptions preserving every ordinary guard,
   root visibility for `lock_library`, and retained account/default/authentication
   immutability. No blanket trigger disable or bypass-RLS shortcut.
3. Independently checked full row closure, including tombstones, all incoming
   edges, unchanged three-cycle FK actions and full deferred owner/manager/feed
   trigger behavior under actual visibility. Same-transaction queued trigger
   events need explicit tests, not an empty-final-table assumption.
4. A mandatory atomic **receipt + marker + affected-account inventory** publication
   invariant. Missing or mismatched any member must abort deletion. Caller-written
   optional fixture evidence or only marker→receipt FK agreement is insufficient.
5. Owner/default/last-owned/billing/confirmation and concurrent-operation policy:
   any-account defaults retained; serialize all affected owner accounts; include
   trialing/active/past-due/paused obligations; bind both LL0 confirmations and
   exact request identity. None follows from the current broad control role.

This is the precise unfinished physical/security boundary, not approval to change
it. The user prefers unchanged RESTRICT; this checkpoint makes no FK rewrite or
production deletion recommendation and stops for review of candidate authority.

## Covered versus still unproved

The observed `library_requires_active_owner` guard protects a Library's last
active owner; it does **not** prove LL0's last-owned-Library rule for an account.
Default immutability is tested; all-account removal admission is not. Active
billing is seeded and retained by a general delete guard; billing-specific
removal admission for all obligation states is not implemented/tested.

Not proved: successful retirement; terminal/inventory publication; all role/ACL
paths; counterfeit retirement permits; hidden-row closure; exact-name/native
confirmation; durable lifecycle retries; multi-account locking; full manager and
deferred feed event interleavings; package/session/PS3/PS4 cleanup; privacy;
SQLite's full graph including its two non-FK logical tables. The concurrent case
proves ordinary row locking only, not lifecycle serialization. SQL context values
are injected as the service would supply them; no Auth0/API authentication claim.

## Reproduction, source integrity and cleanup

```sh
python3 -B scripts/test_ll1_actual_postgres_baseline.py
git diff --check
```

Reuses the [reviewed private-cluster guard](../../../scripts/test_ll1_retirement_constraints.py):
private generated data/socket root, stripped PG connection environment, no URLs,
TCP disabled, verified socket/listener settings, exact-root validation before
stop/removal. No installation, native build, service deployment or external call.
PostgreSQL shared-memory permission required an approved sandbox escalation.
`fsync=off` means no power-loss durability proof.

Successful cluster `/private/tmp/photara-ll1-constraints-c6euh733` was stopped and
removed; no disposable database remains. This data is reproducible, not retained.
Every service migration's SHA-256 was rechecked unchanged after the run; source-set
fingerprint remains `7228575621958f4f5063e40bbb80ec5102d00b232bf21d3fb95bad4779c9c638`.
SQLite sources were not changed. No production migration, Neon/live data, package/
source deletion, PS2 edit, commit, push, ROADMAP or handoff change occurred.
