# LL1 signed adapter with the disposable commit guard

Status: **six private composition runs passed**, 2026-09-27. The
[HTTP runner](../../../scripts/test_ll1_http_sql_adapter.py) now accepts an
independent `--commit-guard` flag alongside its existing adapter, `--lifecycle`
and `--commit-relay` modes. Defaults remain unguarded. No Rust, relay, migration,
production route, production pool or permanent format changed in this slice.

The flag imports the existing
[no-committed-work overlay](LL1_AUTHORITY_COMMIT_GUARD.md) after the inherited
executor, authority, fresh-device and HTTP deadline overlays. It adds only its
`ll1_probe` function and two deferred triggers. It does not regrant the raw
authority functions that the signed adapter deliberately denies.

## Composition assertions

Before installing the optional guard, the runner captures the actual baseline
constraint definitions/actions/deferral flags, function definitions/ACLs and
trigger definitions/enabled states **after** all inherited overlays. That exact
catalog matches immediately after optional installation and again after the Rust
test completes. The pristine 59-table baseline measurement still precedes those
inherited overlays; the comparison does not pretend their known candidate
deletion/deadline triggers are production schema. Existing baseline `RESTRICT`
rules and migration-file hashes remain unchanged.

Guarded runs verify the function's owner is `photara_owner`, SECURITY DEFINER is
enabled, and its fixed search path is `pg_catalog, pg_temp`. Exactly the
`authorization_no_commit` and `permit_no_commit` triggers target that function;
both are enabled, deferrable, initially deferred and attached to the expected
`ll1_probe` tables. The metadata is identical after execution. Unguarded runs
verify the guard function is absent.

A 25-entry privilege matrix across the three ordinary logins, `ll1_service` and
`ll1_authority` is checked before installation, after installation and after the
test. Only the service/authority roles can execute `http_command`; none can
directly execute `authorize`, `execute`, `query_receipt` or `execute_review` in
this composed HTTP surface. PUBLIC and all five runtime roles lack direct
EXECUTE on the new guard function.

All modes finish with exactly zero authorization rows, zero permit rows and one
original captured receipt. The unchanged Rust tests continue to assert exact
receipt bytes and baseline/overlay snapshots at their existing boundaries:

- **Signed adapter:** actual OIDC/device checks, expected credential revision,
  exact immutable review binding, post-lock expiry/revocation, generic-role
  denials, terminal rollback and eligible B's original receipt access.
- **Lifecycle:** in-process cancellation, SQL cancellation, private backend
  termination, completed rollback verified after backend/locks end, account-lock
  serialization and known-commit lost-response recovery.
- **COMMIT relay:** no-forward COMMIT rolls back; forwarded COMMIT with both
  server acknowledgement frames withheld is independently observed committed,
  yet the client receives a commit error. Exact original receipt retrieval by B
  still succeeds without minting a new grant.

Normal signed execution consumes authorization and clears permits before the
deferred checks run. Rollback/cancellation aborts the whole unit. In the relay's
committed case, successful server COMMIT includes completion of the new deferred
checks before its acknowledgement is withheld. The guard does not make a
generic commit error conclusive.

## Executed matrix

Each row is one existing scoped Rust test in its own generated socket-only
PostgreSQL cluster; times are observed fixture execution times, not service
latency measurements.

| Mode | Guard enabled | Guard absent |
| --- | ---: | ---: |
| Signed adapter | 0.54 s | 0.61 s |
| Lifecycle, five cases | 0.57 s | 0.55 s |
| COMMIT relay, two cuts and deadline check | 0.30 s | 0.37 s |

```sh
python3 scripts/test_ll1_http_sql_adapter.py --commit-guard
python3 scripts/test_ll1_http_sql_adapter.py --commit-guard --lifecycle
python3 scripts/test_ll1_http_sql_adapter.py --commit-guard --commit-relay
python3 scripts/test_ll1_http_sql_adapter.py
python3 scripts/test_ll1_http_sql_adapter.py --lifecycle
python3 scripts/test_ll1_http_sql_adapter.py --commit-relay
```

All six clusters were stopped and removed. The
[composition evidence](ll1-http-commit-guard-composition-20260927.json) records
separate outputs, catalog/privilege/guard observations and source/output hashes.
Python syntax and scoped whitespace checks pass. The Rust adapter and relay
sources are unchanged from checkpoint `4170142`; this slice does not claim a new
Clippy run. The earlier standalone guard's 27 cases and historical raw-authority
46-case result remain their own unchanged evidence, not additional executions
in this matrix.

This is opt-in disposable composition, not a production constraint selection,
automatic orphan cleanup or alteration of the historical raw-grant
counterexample. Synthetic JWKS/clocks, sparse seed, candidate SQL guards and
fixture review/receipt encodings remain limits. The server disables fsync.
Neither real TCP disconnect behavior, general outcome classification, server
crash/power-loss durability nor production authority credential custody is
established by these runs.
