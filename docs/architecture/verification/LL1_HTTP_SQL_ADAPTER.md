# LL1 disposable signed HTTP-to-SQL adapter

Status: executed candidate integration proof, 2026-09-27. This adds no production
route, pool configuration, role provisioning, migration or public token format.
The [private runner](../../../scripts/test_ll1_http_sql_adapter.py) creates and
removes its own socket-only PostgreSQL cluster and accepts no database URL.

## What is now connected

The cfg(test)-only [Rust adapter](../../../crates/photara-service/src/http_ll1_sql_tests.rs)
uses locally signed RSA JWTs, the real `OidcVerifier`, and real
`HttpService::credentials`. The embedded `Service` also receives that verifier;
this test does not invoke the earlier credential seam's `FakeAuth0`.
A test-local authority connection is injected into candidate state. The three
ordinary Service pools and production router remain unchanged.

The candidate SQL entry resolves active identity/account from the verified
issuer/subject, then checks the supplied device secret commitment and exact
credential revision under the existing SQL fixture's locks. Identity resolution,
fresh credential checks, private one-use grant consumption, protected deletion,
and original receipt capture run in one SQL transaction/backend. Account and
identity are not request-body fields. The adapter returns captured bytes only
after commit. The authority credential remains a trusted service capability;
these tests do not establish production custody or provisioning.

The immutable SQL review retains its original actor, identity, device, Library,
revision, digest and expiry. The candidate derives its deadline from that exact
review row. The optional `now_ms` seed argument to the existing receipt overlay
leaves its default behavior unchanged. One SQL review clock and the OIDC
`FakeClock` advance together during the deterministic race tests.

## Executed checks

The scoped ignored Rust test passed in **0.52 seconds**. It verifies:

- Signed device A executes the protected SQL deletion and receives the exact
  independently queried captured receipt bytes. Every baseline table is checked
  for exactly the seeded target-row removal, retaining unrelated rows.
- Revoked A is denied. Eligible B on the same initiating account queries and
  retries the exact original receipt while a trigger makes any new grant fail.
  B cannot first-execute A's unexecuted review, before or after A's deletion.
- Wrong device secret, credential revision and unknown subject refuse with
  exact baseline/overlay snapshots unchanged.
- All three ordinary SQL logins fail candidate entry execution despite forged
  account context; the original raw grant/executor grants remain revoked.
- A PID/`pg_blocking_pids` handshake proves the request actually waits on each
  selected lock. Bearer expiry during a credential-row wait, review expiry at
  the pre-grant gate, and review expiry during an actual deletion-row wait all
  produce the exact internal `http_deadline` failure. Credential suspension
  during the wait produces exact `credential_denied`. Only the harness's
  deliberate clock or credential-row change persists.
- Candidate-only BEFORE DELETE guards check the deadline after row-lock waits
  on every fixture-owned relation. The final precommit deadline check is
  additional protection; it does not substitute for the destructive boundary.
- An injected failure after original receipt capture produces exact internal
  `after_http_terminal` and restores all baseline and overlay rows. Generic
  HTTP denial alone is not accepted as evidence that this boundary was reached.
- Grant and permit tables are empty after the exercised terminal paths.

The optional-clock change also passed the original **46-case** fresh-device
SQL suite. Strict service Clippy (`--lib --tests -- -D warnings`), scoped
rustfmt and diff checks passed. Executed source hashes and complete outputs
are in [the evidence record](ll1-http-sql-adapter-20260927.json).

## Limits

This remains a test router, synthetic trusted JWKS, controlled clock, sparse
seed, candidate guard overlay and fixture review/receipt encoding. The baseline
catalog is measured before applying the disposable overlay; existing migration
files remain unchanged. The inherited PostgreSQL harness disables fsync, so
this proves transaction behavior rather than crash or power-loss durability.
No real provider, network JWT fetching, production authority pool or public
lifecycle command is implemented.

Cancellation, connection loss and arbitrary commit of an unconsumed raw grant
remain outside this proof. It does not repair the separate raw-authority
fixture's intentionally committed orphan. `NotFound` remains absence of a
receipt, not proof that an in-flight operation cannot commit. General lifecycle
concurrency, exact-name review UX, permanent formats and production security
approval remain separate boundaries.
