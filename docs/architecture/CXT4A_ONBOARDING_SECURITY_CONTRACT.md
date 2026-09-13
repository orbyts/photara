# CXT4a onboarding and security contract

Prepared 2026-09-13 from clean Photara `main` at `c90274b`. **Approved by the
user on 2026-09-13; CXT4a documentation and reference audit are complete.**
This is an implementation contract for the separately authorized CXT4b/d gates,
not evidence that authentication, new migrations or a service have shipped.

The [approved CXT4 sequence](CXT4_ONBOARDING_AND_OPENING.md) governs scope.
Its CXT4c native opening shell, system-owned sidebar/toolbar, neutral hierarchy,
Light/Dark approval and shared production/Shell Lab sources remain unchanged.
CXT4c consumes fake versions of the states below and contacts no provider.
No manual seed, production Rust/Swift/service edit, provider configuration,
credential creation, database access, staging, commit or push belongs to CXT4a.

## Authority and observed implementation gaps

Read with [CXT3b](CXT3B_LOCAL_RUNTIME.md), [CXT3c](CXT3C_SERVICE_RUNTIME.md),
[CXT3d](CXT3D_NEON_ACTIVATION.md), the
[Account/Library model](LOGICAL_DATA_MODEL.md#account-and-library), and the
[authentication/claim contract](SYNCHRONIZATION_CONTRACT.md#endpoint-and-authentication-boundary).
Project packages retain authored authority; Person is not Account; email is not
identity; DeviceId is an attribution coordinate, never a bearer credential.

Source inspection establishes these prerequisites, not hypothetical cleanup:

- `crates/photara-library/src/gen2/local.rs::ensure_default_library` derives a
  stable LibraryId and local principal from DatabaseId, then requires
  `authority_mode='local-only'`. Simply changing that row to cloud-member breaks
  restart. CXT4d must make startup association-aware and must never create another
  default because the original was renamed, enrolled, signed out or offline.
- CXT3b has account/membership caches and one `sync_targets` row per Library,
  but no durable enrollment journal. Its online worker is not activated.
- `crates/photara-service/src/management.rs::claim_library` assumes an existing
  verified Actor and registered Device. It commits Library, owner, contract state,
  scoped stream, claim receipt and audit. It cannot bootstrap a new identity by
  itself, and its own transaction cannot be nested as a separately committed step.
- Installed service schema has unique exact `(issuer, subject)`, composite
  `(account_id, device_id)`, and claim receipts. It has no Account default-Library
  pointer, pre-Account bootstrap receipt, or device credential/session boundary.
- CXT3c content snapshots cover StorageProjection and CatalogProjection only.
  An empty onboarding stream is not proof that existing local content was uploaded
  or that all Library entities can synchronize.

Keep the deployed thirteen service and twelve local migration bodies/checksums
intact. The additions specified below require new reviewed migrations and floors;
CXT4a changes no SQL and makes no new installed-table count claim.

## Chordrift read-only reference audit

Verified sibling repository:
`/Users/suhail/Library/CloudStorage/Dropbox/matrix/crates/chordrift`, clean at
`a38d3fc`. Read its `AGENTS.md`, the relevant hosted design records,
`src/hosted.rs` (`HostedConfig`, `OidcVerifier`, login/callback/logout and CLI
exchange), `src/cli.rs` (device authorization), and `src/identity.rs` interfaces.
No deployment files containing values, environment, Keychain, secret vault,
database or live endpoint was accessed. Names below are source symbols only;
no tenant/client identifier, credential or configured product claim is reused.

| Observed pattern | Photara disposition |
| --- | --- |
| Random state, S256 verifier, expiring one-use browser attempts, bounded attempt count | Reuse the principles; own the attempt in the native auth coordinator |
| HTTPS endpoint checks, timeouts, sanitized profile presentation and generic failures | Reuse with exact environment allowlists and redirect refusal |
| Hosted confidential-client code exchange and protected product-session cookie | Different client/trust model; no web client secret or cookie session in Photara native PKCE |
| CLI Native application uses Device Authorization, then product-token exchange | Useful public-client separation; not the approved macOS Authorization Code flow |
| UserInfo response supplies subject; configured issuer is attached and trailing slash is normalized | Not a production Photara API JWT verifier; verify signature and exact issuer/audience without normalization |
| Deployment-configured verified-email adoption of an existing account | Reject for Photara: no email bootstrap, preselected Account or legacy adoption |
| Product-owned revocable session abstraction | Useful revocation model, but do not copy its token policy/schema as Photara authority |
| Provider-specific identity, data and operational assumptions | Outside Photara onboarding; no provider scopes, claims or persistence copied |

This is an audit of the checked source, not a penetration test or a statement
about Chordrift's current tenant configuration. No native refresh/Keychain behavior
is inferred from its browser/CLI implementation.

## Native Auth0 and identity verification contract

The release configuration must bind one `environment_id` to one exact HTTPS
service origin, issuer (including trailing slash), API audience, public Native
client ID, callback URI, logout return URI and allowed signing algorithm. Values
are intentionally unfilled until the CXT4a review selects the Photara environment.
These coordinates are public configuration, not secrets. They must be reviewed
as one tuple; a remotely supplied URL or token `iss` cannot choose a verifier.
Development and production use separate tuples and Keychain namespaces. No
fallback issuer, suffix match, URL normalization or automatic environment switch.

Use an Auth0 **Native public client**, token endpoint authentication `None`,
Authorization Code with mandatory PKCE S256. The initial release enables only
Google through Auth0 Universal Login and presents the user-facing action exactly
as **Sign in with Google**. No implicit/password/device-code flow in this macOS slice.
Request `openid profile email offline_access` plus the single proposed API scope
`photara:onboard`; this scope only permits onboarding endpoints, never confers
Library ownership, billing or developer entitlements. The service checks the
scope explicitly. Disable automatic email-based account linking. A future
identity-link operation requires proof of both identities and a separate contract.
The flow and bearer use follow [Auth0's native PKCE guidance](https://auth0.com/docs/get-started/authentication-and-authorization-flow/authorization-code-flow-with-pkce/call-your-api-using-the-authorization-code-flow-with-pkce).

Use `ASWebAuthenticationSession` with the active native window presentation anchor.
It routes callbacks to the calling session; the browser remains system managed.
See [Apple's session contract](https://developer.apple.com/documentation/authenticationservices/aswebauthenticationsession).
Recommend a reverse-domain private scheme tied to Photara's confirmed bundle ID,
exact registered host/path, and no wildcard callback. Select its literal value at
review. Claimed HTTPS is an alternative only after domain association and supported
macOS/browser delivery are proven; do not silently introduce a loopback listener.

One coordinator owns one interactive attempt per app process. Each attempt has
256-bit random state, a 32-byte random verifier encoded base64url without padding,
its S256 challenge, a nonce, expected callback tuple, start time, five-minute
deadline and monotonically increasing session generation. Keep code/verifier/state
in memory only. For enrollment the nonce comes from the service challenge below;
ordinary reauthentication uses a fresh native nonce. Set `max_age=0` when obtaining
an enrollment proof. Request explicit account selection/reauthentication on switch;
an ephemeral browser session is preferred for switch but is not proof of identity.

On callback: require the current attempt, exact scheme/host/path, no fragment,
one state and exactly one code or OAuth error, bounded values, unexpired deadline,
and constant-time state equality. Reject duplicate query keys, unsolicited, late,
cancelled and already-consumed callbacks. Consume the attempt once before exchange.
Exchange at the pinned token endpoint with the same redirect URI and verifier,
never a client secret. A lost exchange result restarts browser authentication;
do not reuse a consumed code. Cancel invalidates the generation before dismissing
the browser so a late exchange cannot replace credentials or begin bootstrap.

Native validates the signed ID token for exact issuer, native-client audience,
authorized party when required, nonce, subject, time and any `at_hash` supplied.
The service independently validates API access tokens. ID tokens, Google access
tokens, opaque UserInfo tokens and decoded-only JWTs are never API bearer tokens.
For new enrollment only, a signed ID token is additionally supplied as a transient
fresh-authentication proof, checked against the same issuer/subject as the access
token and the service nonce. Its `auth_time` must be no earlier than challenge
creation minus 60 seconds and no older than five minutes. Never persist this token.
These checks apply [OIDC ID token validation](https://openid.net/specs/openid-connect-core-1_0.html#IDTokenValidation).

The production access-token verifier must:

1. Limit token input to 16 KiB, reject malformed/duplicate critical claims and
   unsupported JOSE features, and allow only reviewed RS256 keys. Reject `none`,
   HS256, token-supplied `jku`/`x5u`, embedded trust keys and client-credential grants.
2. Fetch discovery/JWKS only from the configured HTTPS issuer endpoints, with
   no redirects, bounded response sizes, timeouts and single-flight refresh.
   Match discovery issuer exactly. Cache keys for at most one hour; an unknown
   `kid` permits one rate-limited refresh, never unbounded per-request fetches.
   Missing keys after failure reject authentication; expired cache fails closed.
3. Verify signature, exact `iss`, nonempty bounded `sub` (maximum 255 UTF-8 bytes),
   API audience exact string membership, required `exp` and `iat`, and `nbf` if
   present. Accept either string or array `aud`; permit only the configured API
   and optional exact issuer UserInfo audience. No substring audiences.
4. Require the approved native client in the configured token profile's `azp`
   claim. Do not infer it from email or silently accept a missing claim. If the
   chosen Auth0 token profile uses a different client claim, review that exact
   profile before enabling it. Require `photara:onboard` for these endpoints.
5. Use a controlled UTC clock, at most 60 seconds tolerance; reject impossible
   intervals and tokens whose declared lifetime exceeds the selected ten-minute
   access-token policy. Recheck actual expiry at database entry, including retries.
6. Resolve `(iss, sub)` byte-for-byte using the installed case-sensitive unique
   key; lock and require active Account and Identity for every authenticated
   operation. Native-supplied AccountId, email, name, Person, device or receipt
   does not replace this resolution. A revoked mapping stays reserved.

Signature/audience/scope validation follows [Auth0's API validation guidance](https://auth0.com/docs/secure/tokens/access-tokens/validate-access-tokens).
The numeric limits and restricted profile above are Photara recommendations,
not Auth0 defaults. A revoked Auth0 session alone does not immediately invalidate
an already-issued JWT; local/service revocation below provides the bounded policy.

## Refresh, Keychain, logout and account switch

Use refresh-token rotation with reuse detection, a recommended seven-day idle
and thirty-day absolute lifetime, and ten-minute access tokens. Prove the exact
tenant settings later. Serialize refresh across processes sharing a credential
namespace, not merely windows. Store the new refresh token atomically before
publishing the new access token. A durable Keychain `refresh-in-flight` marker
prevents replaying the old rotating token after a crash or ambiguous response.
On ambiguity, `invalid_grant`, reuse detection or failed Keychain replacement,
require reauthentication and keep local work; never retry the old token blindly.
See [Auth0 rotation behavior](https://auth0.com/docs/secure/tokens/refresh-tokens/refresh-token-rotation).

Keychain service/access group is Photara-owned and scoped by environment, native
client, exact issuer/subject and installation. Use a non-synchronizing,
device-only, unlocked-access item with the signed app's explicit access group;
prove its macOS data-protection Keychain behavior in CXT4d. No broad application
access or shared Chordrift item. See [Apple's device-only accessibility](https://developer.apple.com/documentation/security/ksecattraccessiblewhenunlockedthisdeviceonly).
Access tokens stay memory-only; refresh tokens and the proposed device credential
stay in Keychain. SQLite stores opaque item references and public commitments,
not secrets. No tokens in defaults, packages, clipboard, URLs, environment,
crash reports, telemetry, screenshots, command-line arguments or logs. Keychain
locked/denied/missing yields auth-required, without a plaintext fallback.

**Recommended device hardening, requiring CXT4a approval:** retain the stable
CXT3b DeviceId, but issue a separate random 256-bit per-Account installation
credential in Keychain during enrollment. Its SHA-256 commitment is in the
canonical request; its value travels only in a redacted HTTPS header. The service
stores only the commitment and compares in constant time. Authenticated device
requests require both the valid Auth0 bearer and this credential. DeviceId itself
remains attribution. A stolen bearer alone cannot impersonate a registered device
or register a new one without the fresh-authentication proof. This is not hardware
attestation or DPoP; malware stealing both credentials remains a residual risk.

Logout first increments the session generation, stops refresh/network workers,
pauses target dispatch and durably records signed-out state. An online
`POST /v1/session/logout` suspends this Account/device credential with a CAS revision;
it does not revoke membership or disable the Account. Delete local refresh/access
credentials and best-effort revoke the Auth0 refresh token; optional browser
session logout uses the pinned allowlisted return URI. Provider logout is separate
from Google-global logout and must not be described as signing out every Google app.
If offline or a revocation response is lost, local sign-out still completes and
reports that server revocation was unconfirmed. Already stolen credentials can
remain usable until expiry/server revocation; never claim immediate global logout.

Retain the installation credential in Keychain across ordinary logout so fresh
interactive authentication can resume the same device with a CAS operation.
Logout suspends a credential; administrative Device `revoked` is terminal for
ordinary enrollment and cannot be changed by login. Deleting app credentials is
a distinct explicit action. Lost device credential requires a separately reviewed
fresh-authenticated replacement flow; do not generate a new DeviceId to bypass a
revocation or silently restore an old credential from copied SQLite.

Switch accounts through signed-out state, explicit browser choice, then verified
new coordinates. Clear active token memory before switching; maintain separate
Keychain namespaces and retain prior enrollment receipts, local content and
unknown outcomes under their original principal. The same physical DeviceId may
have separate Account/device rows and credentials. Never retarget a Library queue,
change its owner, resolve another Account's operation, or link equal emails.
Returning to the same Account resumes its original IDs and durable journal.

## Service deployment and secret boundary

CXT4b must first select an explicit host, HTTPS origin, ingress, encrypted secret
store, operator and deployment identity. Recommendation: one small Rust HTTP
service around the existing Storexa controller, with separate private pools for
three LOGIN identities inheriting only `photara_api`, `photara_control`, or
`photara_auth_read` respectively. The current capability roles remain NOLOGIN.
No runtime identity has owner membership, SUPERUSER, CREATEROLE, CREATEDB,
BYPASSRLS or cross-runtime-role membership. The migration credential exists only
in a separate operator migration job and is never mounted into the service.

Runtime database URLs, cursor/signing keys and any operational secrets belong to
the selected host's encrypted secret storage. The public Native client has no
Auth0 secret; JWT verification needs public JWKS, not Auth0 Management API access.
Google connection credentials remain with Auth0/provider configuration, not the
app or Photara API. The desktop receives only the reviewed public tuple, user
tokens, device credential and bounded DTOs. No Neon URL or database login enters
the app, bridge, packages, build artifacts, fixtures or logs.

Deploy a reproducible immutable artifact with TLS validation to Neon, least
privilege filesystem/process settings, bounded connections/requests and redacted
structured logs. Use request IDs and typed outcomes rather than bearer headers,
raw subject/email, SQL errors, callback queries or DTO bodies. Rate-limit challenges,
JWT failures and bootstrap by IP plus authenticated principal/device; bounded
backoff avoids retry storms. Liveness exposes only health; readiness checks exact
family/floor/ledger, roles and key configuration without exposing connection data.
Fail readiness on role drift or unknown schema; never run migrations on startup.
Backups and restoration proof precede the separately authorized live migration.

No arbitrary URL fetch, token proxy, SQL endpoint, generic CRUD, Auth0 admin,
email sending, billing entitlement or public invitation endpoint is deployed in
this slice. Public metadata cannot redirect credentials to a different service.
The host choice is unresolved; Chordrift's existing host is reference only and
is not permission to reuse its infrastructure or secret store.

## Endpoint and DTO contract

Paths are proposals under the pinned service origin, wire version
`photara.onboarding.v1` and canonical codec `photara.canonical-json.v1`.
They are not yet routes in a deployed service. Reject unknown fields, duplicate
JSON keys, nil UUIDs, noncanonical command bytes and oversized bodies. UUIDs use
the existing typed encoding; revisions/sequences use decimal strings as in the
existing transport. Requests/responses are at most 64 KiB; credentials have
separate 16 KiB/header bounds and are excluded from canonical durable bytes.
Every response is `Cache-Control: no-store`; API calls do not follow redirects.

| Method/path | Authentication and result |
| --- | --- |
| `GET /health/live` | Public, constant health result only |
| `GET /v1/onboarding/capabilities` | Public pinned environment/protocol/floor/limits; not authority to change endpoints |
| `POST /v1/onboarding/challenges` | Public, rate-limited; binds one action/request digest/device commitment to fresh random nonce and five-minute expiry; creates no Account |
| `POST /v1/onboarding/bootstrap` | Valid API bearer plus device credential; first execution also requires fresh ID-token challenge proof; returns immutable bootstrap receipt |
| `GET /v1/onboarding/operations/{operation_id}` | Valid API bearer and matching device credential; exact principal-bound receipt or nonfinal 404 |
| `GET /v1/session` | Valid API bearer and active device credential; current Account/Identity/Device/default-Library and access state |
| `POST /v1/session/resume` | Valid API bearer, existing device credential, fresh challenge proof and expected credential revision; resumes ordinary logout suspension only |
| `POST /v1/session/logout` | Valid API bearer/device credential and expected credential revision; idempotent suspension; exact retry allowed for the same suspension |

Canonical `BootstrapRequest` has exactly: `schema`, `operation_id`,
`environment_id`, `device_id`, `device_credential_sha256`, `device_display_name`,
`requested_library_id`, `library_display_name`, `intent="enroll-default"`.
Names are trimmed nonempty strings, at most 512 UTF-8 bytes; no machine serial,
absolute path or user email is used as a default device name. The Library name is
the current local name, normally `My Library`; names do not identify the aggregate.
No AccountId, IdentityId, owner role, grant, entitlement or client-supplied subject
is accepted. Digest is SHA-256 of the entire canonical request. The native journal
binds it separately to verified issuer/subject before its first submission.

Transient `EnrollmentProof` consists of `challenge_id` and signed ID token in
redacted headers. Challenge request has `schema`, `action`, `operation_id`,
`request_sha256`, `device_id`, `device_credential_sha256`; response has
`schema`, `challenge_id`, `nonce`, `expires_at`. Challenges do not authenticate
callers. Nonce/commitment/digest/action must agree at execution; a fresh proof may
change on retry without changing the canonical bootstrap command.

Immutable `BootstrapReceipt` has `schema`, `environment_id`, `operation_id`,
`request_sha256`, exact `issuer`, `subject`, service-generated `account_id`,
`identity_id`, `device_id`, device commitment, `outcome`, `requested_library_id`,
`default_library_id`, `membership_id`, `contract_version`, `authorization_generation`,
`stream_id`, `stream_epoch`, `initial_high_water`, `completed_at` and server digest
over its canonical bytes. `outcome` is `claimed-default`, `recovered-default`, or
`existing-default-requires-local-choice`. A brand-new default starts at generation
1 and stream high-water 0, with no Project. Existing-default responses retain the
actual current stream coordinates; never assert that a returning Library is empty.
These immutable coordinates are historical evidence, not a current access lease.

Session response contains the same currently verified identity/device/default
coordinates plus present lifecycle states, membership role/revision, authorization
generation and `observed_at`. It cannot be cached as permanent server authority.
Resume/logout commands carry `schema`, `operation_id`, `device_id`,
`expected_credential_revision`; action is fixed by route. Their immutable receipt
records request digest, action, resulting credential state/revision and time.

Receipt lookup verifies the bearer to an exact external principal even if no
Account mapping exists yet; absence returns nonfinal 404 and creates nothing.
If a receipt exists, require its active Account/Identity, device commitment and
current access before disclosure. Logout's exact retry is the sole exception to
the active-credential check: a suspended credential can retrieve only its own
matching logout receipt, not bootstrap/session/content data. Resume may inspect a
suspended credential only with fresh authentication; if offline logout never
reached the server and the credential is still active at the expected revision,
resume returns an idempotent active result without rotating identity.

Errors contain `schema`, stable `code`, `retryable`, `outcome_known`, optional
`retry_after_seconds` and opaque `request_id`, with no other principal's fields.
Use 401 for invalid/expired authentication, 403 for inactive identity/account/
device/access, 409 for ID/digest/default-state collision or CAS conflict, 422 for
invalid command, 426 for unsupported protocol/floor, 429 for throttling and 503
for unavailable verification/storage. A 404 receipt is not a cancellation receipt.
Transient failures never become durable rejection records. No response promises
that a timed-out request did not commit.

## Atomic bootstrap, replay and concurrency

Prepare the local operation and Keychain device credential before sending any
cloud mutation. Validate provider claims/challenge cryptography before opening a
database transaction. Within a single trusted controller transaction:

1. Check service floor and claim expiry. Serialize exact issuer/subject creation
   with a transaction advisory lock; its digest is only a lock key, never identity.
   Read the exact unique row, including revoked records. For a new identity allocate
   AccountId/IdentityId server-side; unique violation restarts the entire transaction
   and resolves the winner. Never create an Account as a preliminary committed step.
   New Account/Identity rows start active at revision 1. Use a bounded sanitized
   verified display name or `Photara Account` as fallback; optional profile claims
   never change identity, create a Person or grant developer/paid capabilities.
2. Lock participating Accounts/Identities in stable UUID order, then the device,
   then Account default, then Libraries in stable UUID order, then operation and
   challenge. All enrollment, access, logout and disable controllers use compatible
   order. Revalidate active lifecycle under locks; no provider I/O under locks.
3. Look up `(issuer, subject, operation_id)`. Exact stored bytes, digest, device
   commitment and current authorization return the original result. Changed bytes
   return idempotency-conflict without disclosing the stored body. A receipt never
   bypasses later Account/Identity/Device disable or Library access loss.
4. If this is first execution, consume the unexpired challenge in the same
   transaction, bind its nonce to signed proof and its digest to this command.
   Register an absent `(AccountId, DeviceId)` and matching credential. Existing
   device commitment mismatch is conflict; revoked Device is forbidden; suspended
   credential requires explicit resume, not bootstrap resurrection.
5. Resolve the durable Account default. If absent, claim exactly the requested
   LibraryId only when globally unused, including tombstones. Create Library,
   owner membership, cloud-member contract, empty scoped stream and default pointer
   together. UUID collision with another claim never grants access or silently
   allocates another ID. A conflict rolls back new Account/Identity/Device too.
6. If the default exists and remains accessible, return it. Same requested ID is
   recovery. A different requested ID creates no second default or Library and
   returns `existing-default-requires-local-choice`. Renamed defaults are recognized
   by ID; tombstoned defaults or missing ownership require explicit repair, never
   a replacement on sign-in. Do not infer a default from first/list order.
7. Commit aggregate, claim/control audit as applicable, challenge consumption and
   immutable bootstrap receipt in that same transaction. Existing CXT3c claim code
   must be factored into a transaction-owned helper, preserving its invariants;
   do not call it after committing identity bootstrap. No manual seed is involved.

Unique constraints on identity, default and device—not a process mutex—ensure
multiple service replicas converge. Two installations racing for a new Account
produce one Account/default; the winner claims its supplied LibraryId, the other
registers its own Device and receives the existing-default choice outcome. A UUID
collision between different Accounts produces a generic conflict and no leaked
owner information. Retry serialization/deadlock failures at most three times with
jitter, then return retryable unknown outcome. Preserve the original command.

On lost reply, retain the original operation; lookup or replay with renewed bearer
credentials. A same-principal receipt is returned after current checks. If no
receipt exists, replay may still race an in-flight commit and must use the same
bytes/key. If the original fresh proof has expired and nothing committed, obtain a
new challenge for the same digest. Cancellation after dispatch only stops waiting;
it does not roll back a possible cloud Account/Library. A new login reconciles it.

## Required persistence delta, not applied here

New tables below are a logical schema contract for CXT4b/d. Each ID is nonnil;
times are UTC; canonical bytes and their SHA-256 agree in the controller. All
foreign keys are RESTRICT, indexed and scoped. Authoritative receipts are immutable
and retained through ordinary logout/disable. No old migration or floor is rewritten.

| Proposed service relation | Required key, columns and invariants |
| --- | --- |
| `photara_identity.account_defaults` | PK/FK `account_id`; FK `library_id`; `revision>=1`, created/updated times; one durable default per Account; controller proves active owner at creation; no automatic reassignment |
| `photara_private.onboarding_receipts` | PK `(issuer,subject,operation_id)`; exact identity FK; Account/device composite FK; `action`, request/response canonical bytes and digests, completed time; stores bootstrap/resume/logout outcomes, checks route/action; sensitive control-only access |
| `photara_private.onboarding_challenges` | PK `challenge_id`; unique nonce digest; action, operation, request digest, device ID/commitment, issued/expiry, consumed time and consuming identity/operation; TTL five minutes, one successful consumption; prune expired rows after 24 hours without deleting receipts |
| `photara_identity.device_credentials` | PK/FK `(account_id,device_id)`; unique nonzero 32-byte commitment, state active/suspended, revision, created/updated times; Device revoked remains an independent denial; no client reads of commitments belonging to others |

Service receipts reference the full exact identity tuple through an added candidate
key if required for a scoped FK; a digest never substitutes for `(issuer,subject)`.
Challenge consumption and receipt insertion have a commit-level closure check.
Grant control only required operations and auth-read only bounded verification
facts. New private tables must have no API/PUBLIC grants; role and RLS tests extend
the existing matrix. Review the resulting migration number/floor and ledger in
CXT4b; the installed CXT3d minimum API remains 2 until that deployment.
At this baseline, recommend service migration 0014 and minimum API 3; recommend
local migration 0013 with reader/writer floors 3. These numbers are reservations
for review, not files or ledger entries created by CXT4a. Recheck the then-current
ledger before choosing final additive migration versions.

Local CXT4d adds `onboarding_intents` keyed by operation UUID with database/device,
Library, environment, exact verified identity (nullable only before verification),
immutable canonical request/digest, local expected revisions, Keychain item
reference/commitment, disposition and optional exact received receipt/digest.
Enforce at most one unresolved enrollment per Library/environment and one
credential reference per environment/principal/device. Preserve canceled/unknown
operations; bind the identity once before dispatch. A separate `library_cloud_bindings`
row has Library PK/FK, environment, Account/Identity, receipt reference, association
state/revision and cached access time; its LibraryId equals the service LibraryId.
It is not an alias mapping between two Libraries. Keep `sync_targets` paused until
the specific supported content bootstrap has been proven. Existing caches remain
projections. Raise local reader/writer floors before admitting cloud state so old
CXT3b binaries refuse rather than fail halfway through startup.

## Local reconciliation and returning users

First launch always opens the CXT3b database and its original local My Library,
without DNS, Auth0, Keychain login, service discovery or an Account row. Local
People/Locations/catalog work retains its explicit local controller. Offer cloud
enrollment later; local choice is neither an error nor a pending auth state.

For the first **empty local Library → new cloud Account** acceptance path:

1. Read a transactionally consistent local preflight: stable database/device/
   Library IDs, controller and revisions; no cloud binding, active Project,
   authored Library content, mutation/outbox, pending context effect or sealed
   legacy work. User name changes are allowed and included in the command. Build
   the canonical intent; persist it before network dispatch. Do not send DatabaseId,
   local principal, paths, bookmarks, Project content or other device facts.
2. After verified sign-in, durably bind the intent to exact issuer/subject. The
   service claims this LibraryId, never a newly minted desktop replacement ID.
   Persist a returned verified receipt independently before local application.
3. Under one SQLite write transaction, compare expected revisions/controller and
   current session generation, recheck eligibility and verify returned environment,
   operation, digest, identity, device and identical LibraryId. Install Account/
   membership caches, binding and contract state, clear local principal only in
   the authorized transition, retain historical local controller evidence, mark
   intent applied and keep unsupported content dispatch paused. No second
   `libraries` row is inserted. The Library's local revision increments normally.
4. If local edits or a switch happened during cloud work, retain the successful
   receipt and enter reconciliation-required; do not overwrite local rows. Cloud
   success is not undone by deleting its Account. Reauthenticate the same principal
   and retry local application only after resolving the changed local state.
5. On every restart, inspect durable binding/journal before the old local-only
   default assumption. Recognize original IDs, resume exact pending operations and
   report cached cloud state offline. Receipt and binding application are atomic;
   a crash before application repeats it, a crash after it is a no-op.

**Populated local Library:** preserve its exact LibraryId and Project/package
associations. Before cloud dispatch, CXT4d returns `local-content-enrollment-required`
and continues local work. It must not flip authority mode while local-principal
Project grants or unsupported roots exist. The follow-on contract must map local
manager grants to verified Account grants with commit-level last-manager proof,
stage supported roots and immutable intents, preserve overlays and obtain an
authorized initial snapshot before activation. Until those codecs and proofs
exist, claiming a cloud row must not be presented as a completed migration. This
limitation is explicit; CXT4e's empty-Library gate does not cover populated transfer.

**Returning same Account on the same installation:** existing binding wins over
the Library name. Verify active credentials and current access; refresh or offer
reauthentication. Renames, restarts, refresh and repeated bootstrap create no
Account, Identity, Device or Library. A bound Library never becomes local-only
merely because its account is signed out or a network request fails.

**Returning Account with a different local LibraryId, including a second Mac:**
there are two real identities, not duplicates that may be renamed into one. Return
the existing default and require the explicit action `Open my cloud Library`;
preserve the independent local Library and its contents. If a cloud cache already
exists, select it by exact ID instead of inserting it again. A pristine empty
installation placeholder may be hidden from the normal selector after the explicit
choice, with a retained local recovery record; it is never rekeyed or deleted.
A populated local Library stays visible as local. Claiming it as an additional
cloud Library, merging content, changing the Account default or migrating Projects
requires a later explicit command. This is distinct from the first-enrollment
same-ID path, which must retain exactly one local Library row.

Offline cloud launch may read retained content and queue only already-supported
offline intents under their original principal. Cached grants never authorize a
server call, new collaboration or a different Account. Known revocation pauses
dispatch and retains evidence; account switching hides the prior Account's cloud
destinations until explicitly revisited. Logout is not secure erasure of local
files and does not assert protection from another user of the same OS login.

## Opening state machine

Persist identity/binding/intent dispositions in Rust-owned local state; native
presentation consumes immutable DTOs and a session generation. Browser handles,
tokens, progress and window selection are not Library records.

| State | Event and next state |
| --- | --- |
| `local-ready` | Choose local: stay usable. Choose cloud: preflight → `authenticating` or `local-content-enrollment-required` |
| `authenticating` | Valid current callback/proof → `bootstrap-pending`; cancel/offline/denial → `local-ready` with nonblocking reason |
| `bootstrap-pending` | Durable request before dispatch; success → `reconciling`; timeout/cancel after send → `outcome-unknown` |
| `outcome-unknown` | Same principal lookup/replay; receipt → `reconciling`; offline → retain and continue local; switch → pause original intent |
| `reconciling` | Exact same-ID local transaction → `cloud-ready`; different default → `library-choice-required`; local CAS/content change → `reconciliation-required` |
| `cloud-ready` | Restart/current session: retain IDs; network loss → `cloud-offline`; expired credential → `auth-required`; logout → `signed-out-cached` |
| `cloud-offline` | Read retained content and supported offline intents; reconnect → current access verification before dispatch |
| `auth-required` / `signed-out-cached` | Explicit same-account login/resume → verify and reconcile; local-only Libraries remain usable |
| `library-choice-required` | Explicit open-existing choice loads that exact cloud identity; keep independent local content |
| `access-disabled` | Account/Identity/Device/membership denial; stop cloud actions, retain work, show actionable support/recovery state; no automatic recreation |
| `protocol-incompatible` | Preserve state, require compatible app/service; do not migrate or reset automatically |

Every callback/network completion carries the generation that initiated it.
Generation mismatch may persist an already-submitted receipt for recovery but
cannot install credentials, change active account or select a Library. Recovery
copy must distinguish canceled browser authentication, cloud outcome unknown and
local reconciliation pending. CXT4c supplies fake examples of each state while
preserving every approved native UI requirement.

## Threat model and acceptance tests

Assets are user tokens, device credential, exact identity mapping, existing local
content, owner/grant authority, receipts, schema/role boundary and database secrets.
Attackers include malicious callback handlers, network/redirect adversaries,
cross-Account callers, stale/replayed clients, duplicate app/service processes,
stolen access tokens and compromised runtime roles. TLS and signed application
distribution are assumptions; a fully compromised OS or control service can
access its authority and is not solved by PKCE. Public-client ID is not a secret.
OAuth threat guidance: [RFC 9700](https://www.rfc-editor.org/info/rfc9700/).

| Threat/scenario | Required invariant and future proof |
| --- | --- |
| Callback interception/injection or login CSRF | Wrong state/verifier/nonce/host/path, duplicate keys, duplicate callback, cancel-race and expired attempt cause zero enrollment calls |
| Token substitution and issuer confusion | Forged/unsigned/HS256 JWT, wrong issuer slash/case, audience substring, wrong azp, ID-token-as-bearer, Google token, empty sub and duplicate claims all reject before DB writes |
| Key rotation or JWKS outage | Known fresh key works; unknown key triggers one bounded fetch; wrong-origin redirect and stale cache fail closed; no attacker-selected fetch URL |
| Refresh rotation crash/reuse | Parallel windows/processes refresh once; fault before/after Keychain marker/replacement never silently retries consumed token; local work survives |
| Credential disclosure | Inspect app bundle, logs, crash output, SQLite, package and fixtures: no tokens, Neon credential or secret-valued headers; Keychain denial has no fallback |
| Stolen bearer or spoofed DeviceId | Missing/wrong device credential denied; new registration needs fresh nonce-bound proof; stolen bearer cannot unsuspend/revoke-bypass an existing Device |
| Replay/collision | Same principal/key/bytes yields byte-identical receipt; changed body/device/environment and another principal reveal no stored result |
| Simultaneous bootstrap | Two pools/replicas, same identity same/different operation and two installations: one Account/Identity/default, one row per Account/device; loser resolves existing default |
| Global Library UUID collision | Collision with active/tombstoned/other-owned ID grants nothing and rolls back partial new aggregate |
| Crash/unknown outcome | Inject after every Account/Identity/Device/default/Library/member/stream/receipt write and before/after commit/HTTP send/local receipt/local apply; all-or-nothing cloud and repeatable local application |
| Disabled state races | Disable Account/Identity/Device or revoke membership during bootstrap/replay/resume: serialization observes denial; no resurrection or receipt-authority bypass |
| Local first launch | With network unavailable and no credentials, exactly original My Library/controller/device; rename/restart produces no duplicate |
| Local to cloud | Empty fixture preserves DatabaseId/DeviceId/LibraryId and adds no second Library; existing local content/Project grants/unknown queues refuse unsafe authority transition |
| Returning/switching | Same account after logout/restart uses same IDs; other account never inherits old queue; second Mac gets existing default plus explicit local choice, not silent rekey/merge |
| Offline/logout | Local sign-out always finishes; remote revocation uncertainty truthful; cloud identity never relabeled local; revoked remote content remains retained evidence |
| Permissions/deployment | Three real unprivileged pools, owner/login misuse, PUBLIC grants, pooled GUC leakage, newer floors/checksum drift, challenge table privacy and new FK/RLS cases extend CXT3c proof |
| UI boundary | Fake-only CXT4c Light/Dark states, native sidebar shown/collapsed/narrow, menus, keyboard, accessibility and shared-source snapshots; Graph implementation unchanged |

CXT4b runs synthetic JWT/fake clock/fake provider and disposable PostgreSQL tests
before its controlled deployment smoke test. CXT4d uses disposable SQLite and
isolated test Keychain namespaces before the authorized real app path. Tests must
not operate on the real CXT3b database merely because it exists. CXT4e then uses
the native product UI to prove one active Account/exact Identity, one registered
Device, original same-ID cloud My Library, owner/contract/empty stream and zero
Projects. No developer CLI bootstrap substitutes for that acceptance.

## Approved decisions and next gate

The user approved this contract on 2026-09-13, including the recommended security,
persistence, token, device and local-content boundaries below. Exact environment
coordinates and the service host remain required CXT4b configuration inputs; they
were not inferred from Chordrift or configured during CXT4a.

| Decision | Recommendation / required evidence |
| --- | --- |
| Auth0 coordinates | **Approved policy:** dedicated Photara Native application/API, Google as the only initial connection, explicit onboarding scope and reviewed token profile. **CXT4b input:** exact issuer/audience/client/callback/logout tuple. |
| Callback and signing | **Approved policy:** private-scheme `ASWebAuthenticationSession` route and interception/duplicate tests. **CXT4b input:** confirmed bundle ID, signing access group and supported macOS range. |
| Hosting | **Approved boundary:** explicit Rust service host, HTTPS ingress, encrypted secret store, scoped three-login provisioning and separate migration job. **CXT4b input:** select the concrete host and secret store. |
| Device revocation | **Approved:** per-Account installation credential plus fresh-auth proof; UUID-only attribution is insufficient. |
| Token/session policy | **Approved:** ten-minute access, seven-day idle/thirty-day absolute rotating refresh, five-minute attempt/challenge and sixty-second clock tolerance, subject to provider-setting proof. |
| Persistence | **Approved:** additive service/local relations and higher compatibility floors; never edit deployed migration checksums. |
| Existing local content | **Approved:** empty same-ID enrollment is CXT4e; populated enrollment stays local pending full grant/root transfer proof; no silent merge on a returning second Mac. |
| Access/identity recovery | **Approved:** disabled identities/devices and lost installation credentials require explicit recovery; no email-based link, automatic new IDs or hidden admin escape hatch. |

CXT4a is approved and complete. This approval does not itself deploy CXT4b,
approve CXT4c mockups, integrate CXT4d or authorize CXT4e real signup. The next
task must explicitly select CXT4b and resolve its concrete configuration and
hosting inputs. Preserve CXT4c as the approved intervening native-shell gate.

Future Yahoo or other Auth0-supported connections are additive provider work.
They must preserve the same Auth0/OIDC boundary and Account/Library contracts.
Because another provider can yield a different subject, enabling it does not
silently link or merge Accounts by equal email; identity linking requires a later
explicit proof-of-both-identities flow.

## CXT4a verification and file inventory

Documentation checks passed 2026-09-13:

- `python3 scripts/verify_generation_two_naming.py`: 362 files.
- `python3 scripts/verify_generation_two_schema.py`: 46 signatures, 104 scoped
  FKs; proposal statement totals remain PostgreSQL 431 / SQLite 139.
- `python3 scripts/verify_generation_two_fixtures.py`: 12 canonical containers,
  92 embedded byte records, using the existing Rust codec binary.
- `git diff --check`: clean. Explicit new-file whitespace, local Markdown links
  and CXT4c section preservation are checked separately because git diff omits
  untracked content.

New file: `docs/architecture/CXT4A_ONBOARDING_SECURITY_CONTRACT.md`.
Updated files: `ROADMAP.md`, `docs/ACTIVE_HANDOFF.md`,
`docs/ROADMAP_0_2_EXECUTION.md`, `docs/architecture/README.md`, and
`docs/architecture/CXT4_ONBOARDING_AND_OPENING.md` (CXT4a status only).
All production source, migrations, fixtures and CXT4c text remain unchanged.
The clean starting tree had no unrelated edits to preserve. No runtime, database,
real token, native UI or provider smoke test was run; the matrix above is future
acceptance work, not a list of passing implementation tests.
