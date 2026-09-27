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
