# LL1 private COMMIT acknowledgement interruption

Status: executed disposable protocol proof, 2026-09-27. This extends the
[signed adapter](LL1_HTTP_SQL_ADAPTER.md) and
[transaction lifecycle proof](LL1_HTTP_SQL_LIFECYCLE.md). It adds no production
route, pool, protocol format, role provisioning or storage qualification.

The opt-in `--commit-relay` mode of the
[private runner](../../../scripts/test_ll1_http_sql_adapter.py) proxies **only**
the test authority connection through a generated private Unix socket. The
administrator connection and three ordinary Service pools retain direct
connections to the same private PostgreSQL instance. The
[relay](../../../scripts/ll1_commit_relay.py) accepts no database URL, host or
external socket path; both listeners and its backend are under the runner's
generated root. The real signed OIDC/device/review checks and original SQL
executor remain those of the existing candidate adapter.

## Exact observed protocol boundary

Installed SQLx 0.9.0's depth-one transaction commit uses literal `COMMIT`.
Its argument-free executor path queues a PostgreSQL simple Query message, whose
encoder writes a NUL-terminated SQL string. The exact expected frontend frame is:

```text
51 00 00 00 0b 43 4f 4d 4d 49 54 00
 Q <length11> C  O  M  M  I  T  NUL
```

The relay requires those exact bytes after arming; it does not search arbitrary
bind values or SQL text for a substring. It associates the connection with the
PID from the server's BackendKeyData frame and checks that against the PID
queried inside the original SQL transaction. It never retains or logs the
backend cancellation secret. An operation/device-specific cfg(test) pause lets
the controller arm only that backend, after actual SQL execution and original
receipt capture have returned, immediately before SQLx calls `commit()`.

For the committed case, the exact withheld server frames are:

```text
43 00 00 00 0b 43 4f 4d 4d 49 54 00   CommandComplete: COMMIT
5a 00 00 00 05 49                     ReadyForQuery: idle
```

Unsupported armed framing refuses the experiment. These are assertions about
this installed driver and private server path, not a general protocol proxy or
an authorization to change a production driver.

## Two outcomes behind a client commit error

**COMMIT captured, never forwarded.** The relay captures the original frontend
COMMIT and closes both streams without sending it to PostgreSQL. The adapter
records an error specifically from `tx.commit().await`, and returns no successful
HTTP result. The captured backend must disappear and all its locks must end
before complete baseline/overlay snapshot equality is accepted. Every row is
restored. Fresh eligible B queries no committed receipt.

**Server committed, acknowledgement withheld.** The relay forwards COMMIT but
buffers both complete server acknowledgement frames. Neither reaches SQLx.
Before any disconnect, the direct administrator connection independently reads
the actual committed original receipt and compares it with the receipt observed
inside the original transaction. The original request is still unfinished and
its commit-failure counter has not advanced. PostgreSQL is idle with no remaining
transaction locks. Only then does the controller close both relay streams.

SQLx now reports a commit error despite the independently proven committed
outcome. The exact target deletion and one original receipt survive, unrelated
baseline rows remain, and no authorization/permit remains. After A is revoked,
fresh eligible B queries and retries the exact stored receipt while a trigger
makes new grant creation fail. Every table remains unchanged during those
reads/retries; a changed request digest conflicts. The relay reports one
forwarded COMMIT, exactly two withheld acknowledgement frames, and zero
acknowledgement bytes forwarded to the client.

The two controlled outcomes demonstrate why a generic commit error is not proof
of rollback. The fixture knows the outcome through its deliberate cut and an
independent observer; a real caller without those facts still needs original-ID
reconciliation. Neither a connection error nor missing receipt authorizes a new
operation ID or another device's first execution of the original review.

## Bounds, cleanup and evidence

Startup frames are capped at 64 KiB and ordinary frames at 1 MiB. A shared
monotonic deadline covers each complete frame header/body and each startup read;
control-command assembly has its own absolute five-second deadline. The receive
deadline is not restarted by each arriving byte. A local socketpair trickle
check must refuse a 100-byte read under a 50 ms total budget while its writer
supplies bytes every 10 ms. This is a parser-bound check, not a timing-based
substitute for the database/relay state handshakes.

Connection/control worker counts are capped at 16/256; startup negotiation at
four packets; commit-response buffering at eight frames. Connections have a
90-second loop deadline, frame reads are additionally capped by its remaining
time, the relay Rust subprocess has a 180-second bound, and Rust state waits are
bounded. Only explicit negative SSL/GSS negotiation is supported; opaque TLS
refuses. No protocol payload, SQL, bind value, credential or receipt traffic is
included in relay logs. Evidence records only PID, fault mode and protocol-state
booleans/counts. All listeners/accepted streams are closed and workers joined;
the private cluster is stopped and its directory removed.

```sh
python3 scripts/test_ll1_http_sql_adapter.py --commit-relay
python3 scripts/test_ll1_http_sql_adapter.py --lifecycle
python3 scripts/test_ll1_http_sql_adapter.py
cargo clippy --offline -p photara-service --lib --tests -- -D warnings
```

The [evidence record](ll1-commit-ack-relay-20260927.json) contains separate outputs
for the relay, prior five-case lifecycle mode and original adapter mode, plus
source/output hashes and strict Clippy. All three modes pass; Python syntax,
Rust formatting and scoped whitespace checks pass. The baseline catalog and
migration files remain unchanged. The new commit-guard overlay is a separate
fixture and is not installed by this relay mode.

This proves two specific Unix-socket interruption cuts in a running private
server. It does not classify every real transport failure, implement a public
unknown-outcome response policy, prove TCP client-disconnect propagation or
resolve all lifecycle concurrency. The inherited cluster disables fsync; neither
server-crash nor power-loss durability is established. Synthetic JWKS/clocks,
sparse seed, guard overlay and fixture receipt encoding remain explicit limits.
