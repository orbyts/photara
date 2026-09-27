# LL1 Rust signed credential seam

Disposable candidate evidence, 2026-09-27. This test adds no production route,
pool, role, migration, SQL deletion, terminal receipt publication or deployment.
PS2 remains the active delivery gate.

The `cfg(test)`-only
[`http_ll1_credential_tests.rs`](../../../crates/photara-service/src/http_ll1_credential_tests.rs)
builds an in-process Axum candidate route using the existing HTTP bounds and
credential parser, real in-memory RSA signing, cached test JWKS, and
`OidcVerifier`. It runs `principal_transaction`, `onboarding::identity` and
`onboarding::device` in one real PostgreSQL transaction, compares the returned
credential revision explicitly, and checks bearer expiry again after the row
locks. The candidate request rejects unknown fields and takes no Account or
identity input. A test-only observer records authenticated facts after commit.

The observer contains seeded immutable original receipt bytes and an original
device review coordinate. These stand in for the SQL receipt/execution boundary;
they are not persisted terminal evidence or an execution grant. The candidate
principal has no seeded Library/default/membership. HTTP uses the real OIDC
verifier, while the embedded fixture `Service` still has `FakeAuth0`; this does
not test `Service::authenticate` with a real OIDC token.

The scenario verifies:

- Valid A can retrieve original bytes and reach an execution-admission observer.
  Wrong signature, issuer, audience, expired bearer, unlinked/revoked identity,
  revoked device, another device's secret, a random correctly sized secret,
  suspended credential and mismatched revision refuse without observer events.
  Supplied Account fields and changed terminal request digests also refuse.
- Eligible B can query a missing receipt but cannot first-execute A's pending
  review. Another authenticated account receives no receipt data. After A is
  revoked and review expiry passes, B and an alternate active identity on the
  same account retrieve exactly the seeded original bytes, including through a
  terminal retry. No further execution-admission event occurs.
- Two controlled races acquire the credential lock on a second connection and
  use the candidate backend PID plus `pg_blocking_pids` to prove an actual wait.
  Advancing the fake clock to bearer expiry or committing credential suspension
  then causes refusal after the lock is released. No sleeps approximate the
  race, and neither path reaches the observer.
- Refusals preserve exact contents of all four credential relations and the
  observer event list. The revocation race permits only the harness's expected
  credential state/revision/timestamp change; the expiry race permits no change.
  This is not an exhaustive snapshot of all protected lifecycle tables.

Run the isolated test with:

```sh
python3 scripts/verify_service_postgres.py --ll1-credential-seam
cargo clippy --offline -p photara-service --lib --tests -- -D warnings
```

The opt-in runner uses its existing generated private `/private/tmp` cluster,
Unix socket, unchanged migrations and ordinary role setup. It accepts no remote
database URL. The scoped mode skips the unrelated native-furnace build and
selects exactly this ignored Rust test; the default full runner is unchanged.
The server is stopped in the runner's cleanup path. Sandbox execution required
escalation for PostgreSQL shared-memory initialization.

The [recorded scoped run](ll1-rust-credential-seam.json) passes one test in 0.91 s
and retains source hashes and captured output; strict service Clippy also passes.
The runner retains the unchanged
schema inventory: 59 tables, 606 columns, 122 triggers, 173 policies, 52 functions
and 14 migration entries. No candidate database objects are created.

The [handoff review](LL1_SERVICE_AUTHORITY_HANDOFF_REVIEW.md) remains the entry
for the next boundary: compose verified claims with the isolated SQL authority
transaction and its fresh credential/deadline recheck, exact original review,
receipt lookup and one-use grants. This fixture does not qualify those paths,
production session expiry behavior, concurrent deletion, crash durability,
authority credential custody or abandoned-grant cleanup.
