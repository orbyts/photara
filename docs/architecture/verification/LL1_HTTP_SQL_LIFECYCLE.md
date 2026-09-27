# LL1 disposable transaction cancellation and lost-response proof

Status: candidate test-only lifecycle extension, 2026-09-27. This extends the
[signed HTTP-to-SQL adapter proof](LL1_HTTP_SQL_ADAPTER.md), using its real
`OidcVerifier`, locally signed JWTs, fresh SQL identity/device checks, exact
immutable review, protected executor and original captured receipt. It adds no
production route, pool, migration, privilege grant or canonical format.

The [private runner](../../../scripts/test_ll1_http_sql_adapter.py) accepts an
opt-in `--lifecycle` flag. Its default still runs the original adapter test.
Both modes create separate socket-only PostgreSQL clusters under private
temporary directories, accept no database URL, check the unchanged baseline
migration hashes and stop/remove their cluster on completion. Only the
cfg(test) [adapter](../../../crates/photara-service/src/http_ll1_sql_tests.rs)
has the new before/after-commit observer points and error-code capture.

## Five executed cases

1. **In-process future cancellation before commit.** The request reaches a
   test-local pause after SQL deletion and original receipt capture have
   returned successfully, before `commit()`. The observer verifies the actual
   terminal receipt and captured authority PID. The controller aborts and awaits
   the Tokio request task. It then waits until that backend has no active
   transaction and no locks before comparing every baseline and overlay table
   with the exact original snapshot. All rows are restored.
2. **SQL statement cancellation at terminal capture.** A candidate-only AFTER
   trigger checks inside the original transaction that the exact receipt exists
   and the grant/permit tables are empty, then waits on a distinct advisory lock
   held by the controller. `pg_blocking_pids` proves the captured authority
   backend reached that wait. Cancelling only that private backend's statement
   produces exact SQLSTATE `57014`. After its transaction and locks end, the
   complete snapshot matches. Eligible B then sees no receipt and remains
   unable to first-execute A's review.
3. **Private database connection termination at terminal capture.** The same
   proven SQL boundary is interrupted with `pg_terminate_backend` for the
   captured fixture PID. Exact SQLSTATE `57P01` is observed. The original backend
   must disappear and all its locks must end before exact snapshot comparison.
   Subsequent authenticated B requests succeed through the authority pool and
   find no receipt; B still cannot first-execute A's review.
4. **Absent committed receipt while execution remains in flight.** Initially B
   can query `NotFound` before A starts, but cannot execute A's review. After A's
   complete SQL execution pauses before commit, an independent administrator
   snapshot still equals the original rows. A's transaction is demonstrably
   active. B's authenticated lookup is demonstrably blocked on that transaction,
   rather than returning `NotFound`. Releasing A to commit lets that same B
   request obtain the exact original receipt.
5. **Known commit followed by a lost in-process response.** The request pauses
   after `commit()` has returned successfully and before returning response
   bytes. The controller aborts the request task. The exact original receipt
   remains committed, only the seeded target rows are removed from every
   baseline table, unrelated rows remain, and no grant/permit remains. After A
   is revoked, fresh eligible B queries and retries the exact stored bytes while
   a trigger makes any new grant fail. Every table remains unchanged during
   those reads/retries; a changed request digest conflicts.

SQLx transaction drop queues rollback; it is not a synchronous rollback
acknowledgement. Therefore an empty receipt query or unchanged observer snapshot
alone is deliberately insufficient for cases 1–3. Their backend transaction and
lock-completion handshake distinguishes actual rollback from invisible pending
work. Conversely, case 4's snapshot equality is explicitly only an MVCC
visibility observation, followed by a demonstrated commit.

## Source finding: receipt queries serialize behind this executor

The protected fixture's `remove` function takes the initiating account
`FOR UPDATE`. Fresh receipt authentication invokes `d19_lock_actor`, which takes
that account `FOR SHARE`. Consequently B's query waits while A holds the terminal
transaction. The first test draft incorrectly expected B to return `NotFound`
at that point; the active-backend assertion rejected that result after the
existing HTTP timeout cancelled A. A bounded diagnostic confirmed the exact
transaction-ID blocker. The final test preserves the locks and proves their
actual ordering. No lock or production timeout policy was changed to make the
test pass.

This is not a general linearizable status API claim. The original rule remains:
absence of a receipt is not proof that pending work cannot later commit, and it
does not grant another device permission to execute the original review.

## Reproduction and boundaries

```sh
python3 scripts/test_ll1_http_sql_adapter.py --lifecycle
python3 scripts/test_ll1_http_sql_adapter.py
cargo clippy --offline -p photara-service --lib --tests -- -D warnings
```

The [evidence record](ll1-http-sql-lifecycle-20260927.json) contains separate
lifecycle and original-adapter outputs, source hashes, strict Clippy output and
the measured unchanged baseline catalog. The five lifecycle cases are one
scoped ignored Rust test. The existing original-adapter test runs separately;
the earlier 46-case SQL wrapper result remains historical evidence and is not
relabelled as a new lifecycle run.

The HTTP request helper uses in-process `Router::oneshot`; aborting its future
does **not** prove how a real TCP client disconnect propagates. Backend
termination proves a database connection-loss case in a running private server,
not a PostgreSQL crash or abrupt power loss. The inherited private cluster
disables fsync. JWKS, clocks, sparse seed, SQL guard overlay and review/receipt
encodings remain synthetic fixture components.

The post-commit observer knows that COMMIT succeeded before suppressing the
response. It does **not** interrupt the wire precisely between server commit and
client acknowledgement. Cancellation or connection loss during `commit().await`
can leave an unresolved outcome; these tests do not convert that error into
proof of rollback or authorize a new operation ID. General commit ambiguity,
real transport-disconnect handling and lifecycle scheduling remain open.

The subsequent [private COMMIT relay](LL1_COMMIT_ACK_RELAY.md) adds two exact
Unix-socket cuts: COMMIT not forwarded, and a server-committed result whose
acknowledgement is withheld. Those controlled outcomes preserve the rule that
a generic client commit error remains ambiguous.

The separate raw-authority fixture's deliberately committed unconsumed grant
also remains outside this proof. Production authority credential custody,
permanent encodings and lifecycle/security approval are unchanged.
