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
4. Return a terminal receipt only after transaction commit. On retry, reverify
   the authenticated initiating identity/device, then fetch the exact original
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
