# Disposable protected-deletion authority boundary

Status: **SQL authority boundary demonstrated; production authentication handoff
is specified but not implemented**, 2026-09-18. This evidence supplements the
[PostgreSQL executor experiment](LL1_PROTECTED_POSTGRES_EXECUTOR.md) and the
[shared semantic contract](LL1_PROTECTED_EXECUTOR_CONTRACT.md).

Run `python3 scripts/test_ll1_protected_authorization.py`. The run uses only an
automatically generated private socket-only PostgreSQL cluster, installs the 14
unchanged service migrations, and removes its temporary cluster afterward. No
production connection, migration, deployment, or deletion is involved.

## Smallest suitable boundary

Use a dedicated lifecycle service credential whose only lifecycle grants are
EXECUTE on a fixed authorization entry point and fixed execution entry point.
Keep ordinary API, control, and auth-read credentials outside that role. The
service's authenticated identity handoff and this credential are the trusted
computing base. No caller-provided Account/identity or session GUC authenticates
a request. A principal possessing the dedicated authority credential is trusted
to assert authenticated identity; credential theft is outside the isolation
demonstrated here, just as a schema-owner or administrator is outside it.

The fixture models this with a NOLOGIN `ll1_authority` role and a dedicated
`ll1_service` login. Neither is a schema owner, superuser, or BYPASSRLS role. The
ordinary roles cannot SET ROLE to the authority, mint a grant, invoke the
authorized executor, invoke the internal deletion function, or modify private
grant/permit tables. The authority itself cannot invoke the internal deletion
function or modify grants directly. Fixed-search-path SECURITY DEFINER entry
points owned by the existing schema owner mediate those actions.

The authorization entry point takes an identity and Account supplied by the
trusted service, verifies their active database association under existing actor
locks, verifies active Library ownership under the Library lock, and creates a
private grant. It overwrites context from those explicit trusted coordinates.
The grant binds identity, Account, Library, revision, operation, request digest,
transaction ID and backend PID, and expires after 30 seconds. Execution consumes
that exact grant and resets context again before entering the existing executor.
The request binding is immutable and persists across terminal retries. There is
no arbitrary SQL, callback, relation name, owner override, or expiry argument.
The expiration uses server wall clock; the production handoff must also enforce
the authenticated session/credential deadline.

This is smaller than a signed capability protocol for a service that already
owns its authenticated database transactions. A request MAC adds key lifecycle,
canonical encoding, domain separation and verification surfaces without removing
the need for a trusted issuer. Reconsider it only if deployment cannot isolate
the service authority credential or requires execution across an untrusted
queue/worker boundary. Existing cursor keys must not be repurposed. This fixture
does not select any new public credential or token format.

## Service handoff required before production

The existing [Service::authenticate](../../../crates/photara-service/src/runtime.rs)
verifies issuer, audience, subject and expiry and resolves an active identity and
Account. `Service::context` rechecks active Account/identity state and device state
under locks; `Service::begin` adds scoped authorization and generation validation.
The [HTTP credential path](../../../crates/photara-service/src/http.rs) verifies
the bearer token and bounds the device credential. These are inspected source
facts, not code executed by this Python test.

A production protected-deletion method must accept the resulting authenticated
service actor, never identity/Account fields from a request body; validate the
required device/session and lifecycle review preconditions; and carry the verified
actor into a dedicated authority transaction. Ownership is established before
grant issuance and checked again by the internal executor for first execution.
The current `context` opens a control-pool transaction, so adding this handoff
requires an explicit authority pool and equivalent checks. The current API or
control pool must not silently gain the authority role. The fixture uses synthetic
trusted identity inputs and does **not** exercise OIDC signatures, HTTP transport,
device credentials, session revocation, or a Rust lifecycle method. It therefore
proves the required SQL privilege boundary, not an end-to-end production route.

## Executable evidence

The final run passes **46 cases** against the actual catalog: 59 tables, 114 FKs,
122 triggers, 173 policies, 52 functions and 45 forced-RLS tables before overlays.
Source fingerprint:
`7228575621958f4f5063e40bbb80ec5102d00b232bf21d3fb95bad4779c9c638`.

- All three ordinary logins fail six protected-path attempts each despite
  asserting the complete owner GUC context: mint, execute, internal execute,
  grant write, permit write, and role escalation.
- Identity/Account substitution, another Account's Library and viewer identity
  fail. Changing any grant coordinate or digest fails. Changing GUCs after a
  legitimate grant does not substitute the executing actor or Library.
- A missing, consumed, expired, prior-transaction or prior-backend grant fails.
  Administrator fault injection independently changes the backend binding while
  retaining the current transaction; execution still fails. Only fixture setup
  can inject those malformed rows; authority grant modification is denied.
- Stale revision emits an immutable terminal result. An exact authenticated
  retry returns it; changed coordinates or digest under the operation fail.
- An explicit rollback after full authorized execution restores the original
  full-schema snapshot. Omitting terminal inventory makes commit fail and restores
  the aggregate. Existing ordinary-delete guards remain active; no permits leak.
- A legitimate authorized deletion commits through the inherited 48-table exact
  row-permit allowlist and RESTRICT-preserving single-statement deletion. All
  unrelated baseline rows remain identical. After membership deletion, a freshly
  authorized exact retry returns the original removed receipt; reuse of the old
  grant and a changed identity/digest fail.

The fixture retains the original narrow DELETE-only guard exception, terminal
closure trigger, receipt immutability and unrelated-data snapshot checks. It
changes no source migration bytes or FK semantics. Detailed cycle omission and
concurrency evidence remains in the inherited executor suite; this suite does
not replace it.

## Limits and shared engine contract

The PostgreSQL private grant is one implementation of authenticated admission to
the semantic protected deletion operation. SQLite must establish its trusted
local authority before opening its protected transaction/connection scope; an
arbitrary SQL writer or holder of the database file is outside SQLite's service
security boundary. Its transaction mechanics need not mimic PostgreSQL roles or
data-modifying CTEs. Both must bind the same authenticated operation coordinates,
enforce current ownership on first execution, consume narrowly scoped authority,
preserve complete aggregate closure and rollback, and reconcile terminal retries
without resurrecting authorization from deleted membership rows.

The full schema is installed, but populated-row coverage is inherited and sparse;
this suite is not exhaustive data coverage for all 48 owned relations. The grant
and request-binding tables are disposable models, not approved migrations or
canonical wire codecs. A grant committed without execution is unusable in a later
transaction; production should keep issuance and execution in one transaction
and bound cleanup/retention of abandoned grants and request reservations. Session
quiescence, review-token freshness, complete inventory delivery, aggregate/access
generation and cross-account last-Library admission remain separate lifecycle
integration work. The cluster disables fsync, so this proves transactional
rollback rather than power-loss durability.
