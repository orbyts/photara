# LL1 service-to-database authority handoff review

Status: source-linked integration review only. No production Library deletion,
API route, fourth database pool, role provisioning, or migration is implemented
here. The [disposable SQL authority fixture](LL1_PROTECTED_AUTHORIZATION.md)
demonstrates the database-side boundary; it does **not** execute this handoff.

## Existing trusted facts

- [`HttpService::credentials`](../../../crates/photara-service/src/http.rs)
  prepares and verifies a bearer token with `OidcVerifier`, parses a bounded
  device credential, and rate-limits a principal key. Its router currently has
  no Library lifecycle route. The HTTP service currently accepts three database
  URLs and constructs three pools.
- [`Service::authenticate`](../../../crates/photara-service/src/runtime.rs)
  verifies issuer, audience, subject and expiry, then resolves an active
  identity and Account through the auth-read pool. `Service::context` rechecks
  active Account/identity/device under locks; `Service::begin` adds Library or
  Project authorization and exact authorization-generation comparison.
- [`set_context`](../../../crates/photara-service/src/runtime.rs) writes actor,
  Library and device IDs into transaction-local PostgreSQL settings. These are
  **context hints**, not cryptographic identity or deletion authority. The
  existing [runtime role template](../../../crates/photara-service/deploy/runtime_roles.sql)
  provisions separate API/control/auth logins, but no lifecycle authority login.

## Proposed minimum production handoff

1. A future explicit, bounded Library-removal API authenticates the bearer and
   device credential through the existing HTTP/Service path. It accepts no
   caller-selected Account or identity ID. It validates the exact-name review,
   target, operation ID and request digest before invoking a protected method.
2. That method uses an isolated, service-only lifecycle authority pool; the
   ordinary API/control/auth logins receive no membership, `SET ROLE`, direct
   EXECUTE or private-table write path into deletion. The authority pool's
   credential must be provisioned and rotated separately, never shipped to a
   desktop client, CLI or agent. Those surfaces call the authenticated service.
3. In one authority transaction, resolve the verified actor again and lock the
   required Account/Library/device rows. Recheck active owner, defaults,
   last-owned Library, billing, pending operations, review expiry, aggregate and
   access generations, and exact request identity. Mint a private one-use grant
   bound to the transaction/backend and complete operation coordinates. The
   executor consumes it atomically, overwrites—not trusts—ambient settings,
   and enforces narrow row/guard scope and terminal publication.
4. Return a terminal receipt only after transaction commit. On receipt lookup,
   reverify the initiating account through a currently eligible authenticated
   device (which may differ from the original device), then fetch the exact original
   receipt without requiring membership that the successful deletion removed.
   A changed digest/target/actor or fresh operation against the removed ID
   refuses. An unknown commit outcome reconciles by the same operation ID.

The separate credential is the trusted computing base: someone who steals it
can impersonate a service call to the private authority entry point. The SQL
fixture demonstrates that stealing an **ordinary** API/control/auth SQL login
or setting arbitrary `photara.*` GUCs is insufficient. It cannot prove secure
credential custody, OIDC/device verification, review UX, or the absence of an
unintended raw-SQL endpoint in production. A request MAC would not remove the
trusted issuer requirement and adds key/codec/replay complexity; revisit only
if the authority credential cannot remain service-only or an untrusted queue
must carry authorized commands. Never reuse the existing cursor key.

## Required integration tests before any production executor

- Synthetic signed OIDC and device credentials through the real HTTP route;
  expired/wrong issuer/audience/subject, revoked identity/device and supplied
  body Account/identity substitution must fail before authority admission.
- All three ordinary SQL logins with forged owner GUCs, changed Library and
  changed digest must fail to mint or execute; readiness must reject authority
  role inheritance by ordinary pools and reject unsafe authority-pool drift.
- Authorized owner exact review succeeds only after the same-transaction
  authority recheck. Concurrent owner/grant/billing/default/review changes,
  expired authorization, stale aggregate/access generation and missing terminal
  inventory must roll back without target-row loss.
- Returning original receipt after committed deletion requires a fresh valid
  authenticated principal but not a now-deleted Library membership. CLI,
  headless and agent callers use the same service operation, never raw package
  or database writes.

This review is a candidate trust boundary, not a completed end-to-end security
proof. No new public token format or `NO ACTION` change is proposed.

## Focused source review — 2026-09-27

Read-only review against `a062a6e1baaffa89c3dc8bf90b8d0aa45ad63ec9` identifies
three concrete distinctions that the next disposable integration must preserve.
These are refinements of existing acceptance requirements, not new authority,
retention, or lifecycle policy. No tests were executed for this source review.

### Receipt retrieval is separate from first-execution authority

Step 4 above must not require the original device to remain eligible for receipt
retrieval. The [typed contract](../LL1_TYPED_CONTRACT_AND_SCHEMA_DELTA.md#proposed-commands-results-and-durable-state)
explicitly permits a freshly authenticated eligible device belonging to the
initiating account after the original device is revoked. New execution still
requires the originally bound device/review and current ownership.

The [SQL authority fixture](../../../scripts/test_ll1_protected_authorization.py)
uses `request_binding.identity` in `authorize` and has no device or credential
coordinate. Its `terminal_retry_fresh_authority` case repeats the original
synthetic identity; it does not prove fresh-device receipt recovery. Implementing
receipt queries by calling the first-execution admission method would therefore
be an unsafe inference from the existing passing case. Receipt access must not
depend on the deleted Library membership or mint an execution grant merely to
read an already committed result.

Next disposable cases: commit and lose the reply on device A; revoke A; query the
same account/operation/hash from eligible device B and return the exact original
receipt without another deletion or inventory event. Deny A's revoked credential,
another account, and a changed request hash. With no terminal receipt, B must not
execute A's reviewed operation. Repeat after review expiry and after restart.
Keep `NotFound` distinct from proof that an in-flight request cannot commit.

### Parsing a device secret does not establish device authority

[`HttpService::credentials`](../../../crates/photara-service/src/http.rs) verifies
the bearer and decodes a 32-byte secret; it does not compare that secret with a
database credential. [`Service::context`](../../../crates/photara-service/src/runtime.rs)
checks the named device's state but does not take a secret or validate its
commitment. The concrete existing check is
[`onboarding::device`](../../../crates/photara-service/src/onboarding.rs): it locks
device and credential rows, compares the commitment, verifies active state and
returns the credential revision. Its surrounding `session` method also resolves
the default Library; that default-only lookup is not a lifecycle target selector.

The future authority transaction must establish equivalent credential facts in
that transaction, without treating an earlier `/session` response or a caller's
active device ID as proof. Next disposable cases: valid bearer plus another
device's secret, random correctly sized secret, suspended credential with active
device row, and mismatched credential revision. Each must fail before grant
issuance. Coordinate a second connection that changes credential state while the
first is waiting for locks; prove a valid serialized outcome and no stale grant.
Use a controllable clock to expire the bearer/review during lock acquisition and
prove admission rechecks the deadline after waiting.

### Unusable abandoned grants are not absent transaction work state

The fixture's `authorization_transaction_expired` case deliberately commits an
unconsumed grant. A later transaction cannot use it, which proves the binding,
but the row is still durable. The [authority evidence limits](LL1_PROTECTED_AUTHORIZATION.md#limits-and-shared-engine-contract)
already acknowledge abandoned-grant cleanup. The
[shared executor contract](LL1_PROTECTED_EXECUTOR_CONTRACT.md#shared-preconditions-and-transaction-boundary)
requires protected permit/work state not to survive commit. The future integrated
candidate must meet that stronger invariant before claiming the shared contract.
The existing `permit` count assertions do not inspect `authorization` rows, and
the baseline snapshot excludes the `ll1_probe` overlay schema.

Next disposable cases: interrupt after minting but before execution; attempt
commit without consumption; cancel or fail during execution; and retry after
connection loss. Assert exact contents of grant/permit/work relations after each
terminal boundary as well as the baseline aggregate snapshot. No usable or
orphaned transaction grant may remain after a successful committed unit. Keep
immutable request/receipt evidence separately classified; do not delete required
dedupe evidence to satisfy the transient-work assertion.

These tests may remain disposable and preserve current `RESTRICT`, ordinary-role
denials, no filesystem/object capabilities and exact terminal publication.
Production role provisioning, a fourth pool, public lifecycle routes, persistent
schema/codec choices and deletion remain behind their existing review gates.

The subsequent [separate commit-guard overlay](LL1_AUTHORITY_COMMIT_GUARD.md)
passes 27 cases for this invariant, including mint-only commit rejection,
cancellation and retained immutable receipts. It leaves the historical raw
fixture unchanged and has not yet been composed with the signed HTTP adapter;
it is not a production constraint choice or orphan-cleanup authorization.

## Fresh-device receipt fixture follow-up — 2026-09-27

The [fresh-device receipt fixture](LL1_FRESH_DEVICE_RECEIPT.md) now passes 42
disposable PostgreSQL cases separating receipt lookup from first-execution
authority. After A commits and loses its reply, revoked A is denied while
eligible B on the initiating account retrieves exact original bytes without
Library membership or a new grant. Another account receives no receipt;
changed hashes and B attempting an unexecuted A review refuse. Receipt lookup
survives review expiry and clean server restart. All candidate-table contents
are compared alongside the baseline, with zero additional terminal events and
empty authorization/permit state on the exercised paths.

This is SQL fixture evidence with synthetic trusted identity inputs and fixture
credential/review/receipt encodings. It does not execute the Rust/OIDC handoff,
resolve credential/deadline lock races, fix the original abandoned-grant case,
or prove crash/power-loss durability. Those integration requirements above
remain open.

## Existing Rust test seams — 2026-09-27

At the time of this read-only source review, the proposed tests below had **not
been executed**. The later credential-seam evidence is linked below. These
distinctions define the next disposable proof, not a claim
that an LL1 route exists or that an implemented LL1 authorization path is
vulnerable.

- [`postgres_http_signed_bootstrap_and_bounds`](../../../crates/photara-service/src/http.rs#L518)
  uses real in-memory RSA signing and the real HTTP `OidcVerifier`. Its embedded
  `Service`, however, comes from
  [`pgtests::Fixture`](../../../crates/photara-service/src/pgtests.rs#L37), whose
  verifier remains `FakeAuth0` with different issuer/audience coordinates.
  Onboarding accepts the HTTP-verified claims directly. Thus this test proves
  that HTTP credential seam, not real OIDC through `Service::authenticate`.
  A test claiming the latter must construct `Service` with the same real
  verifier and matching configured coordinates.
- [`onboarding::device`](../../../crates/photara-service/src/onboarding.rs#L700)
  locks device/credential rows and validates their state and the secret
  commitment, but **returns** the credential revision. It does not accept or
  compare an expected revision. The candidate LL1 command adapter must compare
  its exact expected revision explicitly; passing this helper alone cannot
  prove stale-revision refusal. Use `suspended: false` for the eligible-device
  receipt path; the existing logout exception is not LL1 receipt authority.
- [`principal_transaction`](../../../crates/photara-service/src/onboarding.rs#L222)
  rechecks bearer expiry after its principal advisory-lock wait. The later
  `identity`/`device` calls can themselves wait for row locks. The current
  `operation` and `session` methods do not perform another expiry check after
  those waits. This is an unexecuted source observation about existing
  onboarding/session paths, not evidence that an LL1 candidate already meets
  the required deadline check immediately before authority admission.

The smallest next test is an ignored PostgreSQL test with a `cfg(test)`-only
candidate router beside the existing HTTP tests. Reuse their request helper,
real signed tokens, cached test JWKS and retained `Arc<FakeClock>`. Run it only
through the generated private cluster of
[`verify_service_postgres.py`](../../../scripts/verify_service_postgres.py);
keep the production router, three-pool constructor, role template and
migrations unchanged. The candidate request accepts device/operation/digest
coordinates, never caller-selected Account or identity. Call the real HTTP
credential parser, resolve identity and check the device in one transaction,
compare the returned revision, then recheck expiry after all admission locks.
A bounded test-only observer can first record the resulting authenticated
facts and distinguish receipt lookup from execution admission without granting
ordinary pools any lifecycle authority. This closes a Rust credential seam;
it is not yet the combined SQL executor proof.

Exercise wrong signature/issuer/audience, unlinked or revoked identity, revoked
device, another device's secret, random correctly sized secret, suspended
credential and stale revision. For deterministic races, hold the device or
credential row on another connection, establish that the request is waiting,
advance the fake clock or commit revocation, and then release the lock. Assert
no observer admission on expiry/revocation; avoid sleeps as synchronization.
Keep missing receipt distinct from permission to execute, and verify that an
eligible B can query A's terminal result but cannot first-execute A's review.

Connecting that observer to the existing LL1 SQL overlay is a subsequent
test-only composition step. Its final authority transaction must re-resolve
the verified principal and recheck credential/revision/deadline itself; a
successful earlier control-pool transaction is not a transferable grant.
The existing SQL authority role exposes protected functions, not the raw
identity-table privileges required by the Rust locking helpers, so those
helpers cannot simply be moved onto that connection without an explicit
candidate adapter. Any overlay adapter and isolated fixture connection remain
outside production `HttpService`/`Service`, with ordinary-role denials and exact
baseline/overlay snapshots preserved. Do not report the combined handoff as
proved until signed HTTP credentials actually reach those transaction-bound
receipt/execution paths.

### Disposable credential-seam follow-up

The [Rust credential-seam test](LL1_RUST_CREDENTIAL_SEAM.md) now executes the
test-only router/observer portion above: real signed HTTP OIDC verification,
database identity/device/secret checks, explicit credential revision comparison,
and post-row-lock expiry checking. A synchronized second connection proves both
expiry and credential revocation while admission is blocked on a credential row.
Neither case reaches the observer. Eligible B retrieves seeded original receipt
bytes after A is revoked and the review expires; B cannot first-execute A's
pending review. An alternate eligible identity on the same account also works.

This proves the candidate credential seam, not actual terminal publication or
SQL lifecycle authority. Receipt bytes and execution admission are an in-memory
observer; no LL1 executor is called. The embedded `Service` still uses the
fixture's `FakeAuth0`; only the HTTP credential path uses the real OIDC verifier.
The candidate's post-lock deadline check does not change existing production
session/onboarding methods. Same-transaction SQL authority handoff, review
binding, role isolation and abandoned-grant work remain open.

### Wrapper rollback follow-up

The fresh-device fixture now passes 46 cases, including injected errors before
and after one-use grant consumption, an error after original terminal receipt
capture, and explicit rollback after the complete wrapper call. Each compares
all baseline and overlay rows exactly. This proves rollback for those wrapper
boundaries; it does not remove the original raw-authority fixture's deliberately
committed orphan, test connection loss/cancellation, or complete the signed
HTTP-to-SQL handoff. See [recorded rerun](ll1-fresh-device-rollback-20260927.json).

### Combined signed HTTP-to-SQL fixture follow-up

The [combined adapter proof](LL1_HTTP_SQL_ADAPTER.md) now connects real signed
HTTP OIDC credentials to protected SQL fresh-device validation and actual
terminal execution in one authority transaction. It covers exact original
receipt recovery by B after A is revoked, ordinary-role denial, credential
revocation and three synchronized post-lock deadline boundaries, including an
actual deletion row lock. Internal error-origin assertions and complete
baseline/overlay snapshots distinguish real boundary failures from generic
HTTP denial. The original 46-case SQL suite still passes with the default clock.

This is a cfg(test) router and private candidate overlay, not a production
fourth pool or public route. The separate raw abandoned-grant case,
connection-loss/cancellation behavior, production credential custody and
permanent review/receipt encodings remain open. See the linked proof for
executed source hashes and its synthetic-clock/JWKS and fsync-disabled limits.
