# CXT4b service checkpoint

## CXT4d modern Keychain supersedes the legacy launcher

2026-09-13: user requires no legacy Keychain behavior. Checked operator source
now uses Data Protection Keychain with private signed access-group authority,
device-only unlocked items and current LocalAuthentication APIs. It compiles
without deprecations and fails closed with `signing-required` on the current
ad-hoc build. Zero valid signing identities and a synthetic `errSecMissingEntitlement`
(-34018) probe establish the live signing gate. The prior installed legacy helper
and file-based item are untouched for recovery only; previous live readiness is
historical and was not repeated through the modern operator. Secure migration
and fresh live readiness/restart follow legitimate signing/profile setup.

See [CXT4d source, synthetic tests and remaining integration](CXT4D_NATIVE_AUTHENTICATION_CHECKPOINT.md).

## CXT4b-dev runtime provisioning and live readiness

Completed 2026-09-13 on the canonical dirty CXT4b/CXT4c work without staging,
committing or pushing. The approved operator gate created these logins through
SQL on Neon project `steep-waterfall-28781561`, exact `main` branch
`br-shiny-glitter-afu6adtk`, database `neondb`:

| Runtime login | Sole capability membership | Connection ceiling |
| --- | --- | ---: |
| `photara_dev_api` | `photara_api` | 6 |
| `photara_dev_control` | `photara_control` | 6 |
| `photara_dev_auth_read` | `photara_auth_read` | 6 |

Each independently authenticated with its new random credential. Audits prove
LOGIN/INHERIT, no SUPERUSER/CREATEDB/CREATEROLE/REPLICATION/BYPASSRLS, exactly one
membership without admin option, no other runtime capability, `photara_owner`
or `neon_superuser` membership, no relation/schema ownership and no database
CREATE privilege. Existing capability grants/migrations were not edited. The
operator `neondb_owner` URL was obtained in process memory through the existing
authenticated Neon CLI and supplied only to short-lived provisioning/preflight
psql children. It was never stored or supplied to the service.

`scripts/provision_development_service.py` generates three independent 32-byte
random passwords and one 32-byte cursor key. It atomically stores the four
values as one encrypted, non-sync Keychain generic-password item before creating
the logins transactionally. It refuses name/item collisions, does not rotate or
adopt existing state, never prints SQL/provider error bodies, and stops for
manual inspection after an uncertain outcome. Existing-item protection keeps
the cursor key stable. No secret value entered an argument, file, app bundle,
Project, repository, synchronized directory or log.

The native `scripts/photara-development-service.swift` operator helper is installed
at `/Users/suhail/.local/share/photara-development-service/launcher`; the private
directory and its executables are mode 700. Keychain item service is
`com.photara.operator.development.neon-main`, account `photara-runtime-v1`.
Its ACL automatically trusts only this native helper. It disables unattended
Keychain interaction and core dumps; there is no secret-export command. `run`
validates the fixed development role/URL/key structure, refuses a migration-owner
setting, validates a user-owned non-writable-by-others executable, and directly
executes the separately installed `photara-service` with only the four secrets,
the development channel and a fixed PATH. Service credentials do not reach Cargo
or any desktop process. The operator helper intentionally uses the macOS login
Keychain's supported legacy ACL APIs (Swift emits deprecation warnings); it does
not establish the future signed native user-token access-group contract.

**Measured acceptance:**

- Live preflight: family `photara.service.g2`, epoch 1, floor 3; 14 successful
  ledger entries; Accounts/Libraries/defaults/receipts `0/0/0/0`; no runtime-name
  collisions. The runtime subsequently verifies exact migration checksums.
- Current locked/offline release service build and native helper compile pass.
  Three helper refusal checks pass (unknown action, invalid secret bundle and
  unsafe executable); real Keychain store/read and each runtime login audit pass.
- At **19:36:54 UTC**, loopback `/health/ready` returns HTTP 200
  `{"ready":true}` and `/health/live` returns HTTP 200 `{"healthy":true}`;
  both responses carry `Cache-Control: no-store`. This exercises real pinned
  Auth0 discovery/JWKS, certificate-verified Neon connections, separated pools,
  role admission and exact family/floor/ledger checks.
- Restart using the same Keychain item returns readiness HTTP 200 at
  **19:38:18 UTC**. No provisioning/key regeneration occurs on launch.
- Operator postflight confirms the same empty counts and safe roles. The
  foreground proof service is then stopped cleanly; no auto-start daemon exists.

Initial psql attempts failed before database access because libpq does not expand
a connection URI in `PGDATABASE`. The checked helper supplies separate private
libpq fields and `verify-full`/system certificate roots. Initial loopback curl
was denied by the tool sandbox; the ordinary escalated read-only check succeeded.
No TLS, role or Keychain control was weakened to obtain evidence.

**Limits and next gate:** no Auth0 user signed in, no Account/Library was seeded,
no real local SQLite was opened, and no cloud-content worker, Fly resource or
background service was activated. This proves the service on the first Mac;
CXT4d still owns native PKCE, user-token Keychain authority, transport/ATS and
bootstrap/reconciliation. CXT4e owns real onboarding and the same Account/Library
on a second securely configured Mac. This provisioning command deliberately
refuses a second creation on `main`; reviewed host-to-host secure provisioning
of the existing environment's secrets is still needed before second-Mac use.

Exact new files: `scripts/photara-development-service.swift` and
`scripts/provision_development_service.py`. Updated documentation:
`crates/photara-service/deploy/README.md`, this checkpoint, `ROADMAP.md`,
`docs/ROADMAP_0_2_EXECUTION.md`, and `docs/ACTIVE_HANDOFF.md`. No Rust, Graph, node,
native app, database-migration or product-identity source changed in this slice.
See the [exact launch/recovery runbook](../../crates/photara-service/deploy/README.md#host-local-keychain-launcher).

## CXT4b-dev configuration checkpoint

Implemented 2026-09-13 on top of the existing dirty CXT4b/CXT4c and policy work;
nothing is staged, committed or pushed. `config/product-identity.json` is the
single checked, nonsecret identity and release coordinate source. Typed Rust
admission embeds it; native build tooling generates typed immutable Swift and
bundle name/identifier/executable/callback metadata from it. The current app and
shared opening title/default-Library copy consume this seam. The app's support
directory remains `Photara`, now resolved through the descriptor. Current
`.photara` package identity, callback, Keychain service, API namespace, user-agent,
cache and future website/support/privacy/store fields are available to CXT4d/L3.

The descriptor is not a mass source rename or a persistence migration. Existing
legacy preference keys, legacy single-file document suffix, database family and
deployed migration checksums remain unchanged; package publication and native
Keychain/PKCE are not implemented here. CXT4d/L3 must consume these typed fields
as their adapters are built. Final distribution still requires the coordinated
brand/resource cutover and an inventory of remaining presentation/packaging
labels; a global search-and-replace over durable schema IDs is not the contract.

`PHOTARA_RELEASE_CHANNEL=development` selects the checked Auth0 tuple and exact
`127.0.0.1:8080` bind. The original `HttpService::connect`, RS256/JWKS verifier,
certificate-verified PostgreSQL URLs, bounded pools, distinct safe runtime roles,
readiness/schema/ledger checks and Axum router remain the shared contract.
The five optional old public-coordinate settings become exact-match assertions.
Unknown channels, mismatching ports/coordinates and a runtime migration-owner
setting refuse with redacted errors. No environment override can select an
arbitrary service or issuer. Hosted descriptors are intentionally null; a default
service launch selects production and refuses. Remote acceptance requires its
own reviewed HTTPS origin and environment ID; it may retain the development
Auth0 tuple. Production additionally rejects the codename and development trust
coordinates. No paid hosting is needed to compile or test this work.

**Verification:** seven offline Python release-admission tests and the standalone
Swift decoding smoke test pass. Rust service tests pass seven executed
tests (five new configuration/runtime tests, two retained OIDC tests); the ten
explicit disposable-PostgreSQL suites remain excluded from this ordinary runner.
Strict Clippy for all service targets and Rust formatting pass. Production app
and Shell Lab builds pass without launching either app or opening user storage.
Generated bundle inspection confirms the development channel, existing bundle
identity and registered callback scheme. The first overlapping Shell Lab build
detected a rewritten generated input; generation now locks and atomically writes
only changed content, a new regression test passes, and the repeat build passes.
The existing PostgreSQL, Graph and native UI behavior suites were not rerun because
their semantics did not change. This is build/configuration evidence, not live
provider/readiness or sign-in proof. Shell Lab's title authoring control is retained;
the production composition applies the typed product name.

**Live-readiness limitation and exact next gate:** no actual runtime LOGIN
credentials or cursor key exist in this slice. No service was started against
Neon, no `/health/ready` was obtained, and no Auth0 user signed in. Separately
authorize the operator to create three distinct least-privilege logins behind
the existing API/control/auth-read capability roles on Neon `main`, provision
their credentials and a stable 32-byte cursor key directly into approved
host-local secure storage, and inject them only into the separate service
process. Keep the migration owner out of runtime, do not write secrets into
Dropbox/repository/app/package/logs, and verify readiness without a user seed.
Repeat secure host provisioning for the second development Mac. Then CXT4d owns
real browser PKCE, Keychain user tokens and local/cloud reconciliation; CXT4e
proves first sign-in and the same Account/default Library on the second Mac.
The [operator runbook](../../crates/photara-service/deploy/README.md) records exact
settings and startup behavior. Native transport/ATS, signed Keychain authority,
live JWKS/Neon connectivity and multi-Mac hydration remain unproven.

Exact new files:

```text
config/product-identity.json
crates/photara-service/src/release.rs
scripts/generate_product_configuration.py
scripts/test_product_configuration.py
platform/macos/photara-ui-foundation/Sources/ReleaseConfiguration.swift
platform/macos/photara-ui-foundation/Tests/ReleaseConfigurationTests.swift
```

Exact updated files in this slice (earlier dirty changes preserved):

```text
.gitignore
crates/photara-service/src/lib.rs
crates/photara-service/src/http.rs
crates/photara-service/src/bin/photara-service.rs
crates/photara-service/deploy/README.md
platform/macos/shared-ui-sources.sh
platform/macos/photara-app/build-app.sh
platform/macos/photara-app/Sources/PhotaraMacApp.swift
platform/macos/photara-app/Sources/AppModel.swift
platform/macos/photara-shell/Sources/OpeningLibraryView.swift
ROADMAP.md
docs/ACTIVE_HANDOFF.md
docs/ROADMAP_0_2_EXECUTION.md
docs/architecture/CXT4B_SERVICE_CHECKPOINT.md
```

No Graph, node, bridge, database-migration or Core semantic source changed.
No Fly, Neon, Auth0, user Project, real SQLite, credential, commit or push operation
was performed. The builds produce ordinary ignored native artifacts only.

## 2026-09-13 Neon migration activation

The operator-only migrator installed additive migration 0014 on the existing
Neon `photara` project's primary/default `main` branch. Preflight showed the
exact Generation Two family/epoch, minimum API 2, 13 successful migrations, zero
Accounts, and zero Libraries. Postflight shows:

```text
schema family:       photara.service.g2
schema epoch:        1
minimum API:         3
migration ledger:    14 successful entries
0014 checksum:       adeadf906a1cd1e97fbc3b31d2dafe3f54435a10224c47ba8254cfa521f7d2ebf97f30a10c6c45e6116be5aad0f91dec
accounts/libraries:  0 / 0
defaults/receipts:   0 / 0
```

The four expected relations resolve as
`photara_identity.account_defaults`,
`photara_identity.device_credentials`,
`photara_private.onboarding_receipts`, and
`photara_private.onboarding_challenges`. No user or bootstrap aggregate was
seeded. The separate `legacy-v0.1.x` branch was not opened or modified.

## 2026-09-13 Auth0 development tuple

The dedicated development resources are configured without creating a user or
running product onboarding:

```text
issuer:       https://dev-nmturasdrkz7up27.us.auth0.com/
audience:     urn:photara:api:development
native app:   Photara macOS
client id:    CJ3hH2CUSkEk0vSbdvHMJLEqD4gYWGkW
callback:     com.photara.desktop://dev-nmturasdrkz7up27.us.auth0.com/macos/com.photara.desktop/callback
logout:       com.photara.desktop://dev-nmturasdrkz7up27.us.auth0.com/macos/com.photara.desktop/callback
API id:       6aa6e96c3595ecc7d9ad79f1
scope:        photara:onboard
```

The Native public client permits Authorization Code and Refresh Token grants,
disables Implicit, and applies rotating refresh tokens with seven-day idle and
thirty-day maximum lifetimes. Google is its only connection;
Username-Password-Authentication is disabled for this application only. The API
uses the Auth0 JWT profile with RS256, a 600-second maximum access-token lifetime,
offline access, per-app user-delegated authorization, no client access, and one
permission: `photara:onboard`. `Photara macOS` is explicitly granted that one
permission. Existing Chordrift resources were not changed.

These are public release coordinates, not credentials. No client secret was
copied or exposed. No Auth0 user, product Account/Device/Library, Neon row, or
runtime secret was created. CXT4d/e remain gated on an accepted service profile;
during development that profile may be the approved loopback adapter.

## 2026-09-13 selected host adapter

Fly.io is selected for the first Photara API deployment because it runs the
existing Axum service as an immutable Docker artifact, provides managed HTTPS,
encrypted runtime secrets and health/concurrency controls without a proprietary
service rewrite. The repository now includes:

- `photara-service`, a fail-closed runtime entry point binding `0.0.0.0:$PORT`;
- `photara-migrate`, a separate operator-only migration entry point;
- a multi-stage non-root Docker image and bounded build context; and
- a validated Fly manifest with readiness checks and HTTPS enforcement.

The migration-owner URL is deliberately not a Fly application secret because
Fly application secrets are visible to the runtime process. Migration 0014 must
run from a separately authorized operator/CI context before runtime deployment.

Local verification: release binaries compile with the checked lockfile; service
tests pass (2 executed, 10 explicit disposable-PostgreSQL tests ignored by the
ordinary runner); all targets pass strict Clippy and formatting; Fly CLI 0.4.102
accepts the manifest. Docker is not installed locally, so the container build
still requires Fly's remote builder or another reviewed builder.

The Fly account is authenticated and billing information is now present. No app,
hostname, secret, Machine or charge was created. Continuous Fly deployment is
deliberately deferred under the approved
[development-cloud policy](CXT4_DEVELOPMENT_CLOUD_AND_PRODUCT_IDENTITY.md): local
service execution plus Neon `main` is the normal development cloud, Fly is created
briefly for remote acceptance, and permanent hosting begins at production.

Started 2026-09-13 from clean approved `14f77ee` in the canonical repository.
The CXT4a contract and CXT4c gate remain authoritative.

## Deferred remote deployment inputs

The approved repository records do not provide a concrete Rust service host,
HTTPS origin/ingress, encrypted secret store, operator or deployment identity.
The dedicated Photara Auth0 development tuple is now recorded above. Release
signing access group and supported macOS range still need confirmation as one
release configuration; the current ad-hoc build does not establish release
signing authority. The Fly HTTPS origin remains unavailable until app creation.

Observed development build facts are `CFBundleIdentifier=com.photara.desktop`
and `LSMinimumSystemVersion=14.0` in the app's existing `Info.plist`; its build
script signs with `codesign --sign -`. These are retained evidence, not an
approved Auth0 callback/Keychain signing tuple or a verified supported release
range. No signing access group is established by that ad-hoc build.

Decision: the Auth0 inputs were created explicitly and were not inferred from
Chordrift. The reviewed Neon migration is installed as recorded above. CXT4b-dev
may now provide loopback runtime identities through host-only secure configuration;
it must not use the migration owner or manually seed an Account/Library. Fly HTTPS,
release signing and production secret injection remain separately accepted gates.
CXT4b is not remotely deployed or complete.

## Host-independent implementation

The local service checkpoint implements the approved bootstrap and session
boundary. It does not close remote deployment or advance CXT4d/e. The subsequent
CXT4b-dev seam/profile implementation is recorded above; live readiness remains open.

- Additive `0014_onboarding.sql` installs four relations, exact identity/device
  FKs, indexed challenge/receipt bindings, immutable receipts/defaults, monotonic
  credential transitions, active-owner default admission and deferred two-way
  challenge/receipt closure. New private and identity relations use explicit
  controller-only privileges, matching the existing identity/private model; no
  API/PUBLIC grants or broad RLS policies were introduced. Auth-read receives no
  new secret-bearing relation access. UPDATE privilege on defaults exists only
  because PostgreSQL requires it for `FOR UPDATE`; the trigger rejects updates.
- Minimum service API becomes 3 locally. A 13-entry/floor-2 database upgrades
  additively; repeated migration is accepted; unknown families/floors/ledgers and
  old checksum drift refuse. Deployed 0001–0013 and the live minimum API 2 remain
  unchanged. Runtime startup never migrates.
- Storexa retains ownership of all pools/transactions. The production HTTP
  constructor requires three distinct safe role boundaries, certificate-verified
  PostgreSQL URLs (`sslmode=verify-full`), four connections per pool and bounded
  acquisition. Readiness rechecks role capabilities and exact family/floor/ledger.
- The production verifier accepts pinned RS256 keys only, validates exact issuer,
  audience/native azp/scope, subject/time/access lifetime, and independently checks
  fresh ID-token subject/audience/auth_time/nonce. Duplicate keys/claims, unsupported
  JOSE features and token-selected trust URLs reject. Discovery and JWKS have
  exact pinned paths, no redirects, five-second fetch deadlines, 64-KiB responses,
  16-key bounds, one-hour cache expiry and a single-flight 30-second refresh limit.
  Keys warm before readiness; missing/expired trust fails closed. The ten-minute
  lifetime limit applies to API access tokens; ID proofs require fresh auth_time.
- Typed canonical commands reject duplicate/unknown fields, nil typed UUIDs,
  noncanonical bytes and oversized bodies. Bootstrap claims precisely the supplied
  LibraryId, creates no Project, and uses one transaction for Account, Identity,
  Device, credential, Library/owner/contract/stream, durable default, challenge and
  immutable receipt. The existing claim implementation is a transaction-owned
  helper; no preliminary identity commit exists. Exact identity advisory locks
  and durable unique constraints converge concurrent installations. Deadlock and
  serialization retries restart the entire transaction at most three times.
- Replays require identical bytes/digests and current Account/Identity/Device/
  ownership. A different local Library receives the existing-default choice;
  collisions roll back new aggregates. Logout suspends with CAS, retains its exact
  matching retry/lookup exception, and cannot reactivate a revoked Device. Resume
  requires a fresh proof and existing credential. A stale logout receipt never
  suspends or impersonates a later resumed revision.
- The Axum router exposes only capabilities, challenges, bootstrap, operation
  lookup, current session, resume/logout and health/readiness. Limits include
  64-KiB command/response bodies, 16-KiB tokens, 48-KiB aggregate headers, 64 active
  handlers, ten-second requests, bounded IP/principal rate state and redacted
  typed outcomes with `no-store`. It never emits request/token/SQL-error bodies.
  Unknown outcomes remain unknown. Expired challenges are pruned in batches of
  at most 100 after 24 hours; receipts are retained.
- The operator [role template](../../crates/photara-service/deploy/runtime_roles.sql)
  creates three password-disabled LOGIN roles transactionally with only their
  own capability membership. The disposable runner uses this exact template.
  Host-specific credential generation/injection and immutable daemon packaging
  require the missing selected encrypted secret store and ingress adapter.

Wire timestamps in this local checkpoint are UTC Unix milliseconds encoded as
decimal strings. Revisions and sequences are decimal strings; digests are lowercase
hex. An immutable receipt is returned in `{receipt, receipt_sha256}`; its digest
covers the canonical enclosed receipt bytes, excluding the digest envelope.
The native CXT4d adapter must consume these explicit encodings, not infer them.

### Existing floor-1 compatibility audit

Review against approved commit `14f77ee` confirms that existing floor 1 was already
refused. Although its first metadata condition mentioned `1..=2`, the immediately
following condition required `minimum_api == 2` before running any migration.
The baseline `postgres_schema_and_runtime_refusal` test explicitly exercised
floor 1 and expected `ServiceError::Unsupported`; the
[CXT3c runtime contract](CXT3C_SERVICE_RUNTIME.md#executable-database) explicitly
requires existing databases to have floor 2 and rejects existing floor 1.

CXT4b's single `2..=3` admission check preserves that refusal while adding the
new floor-3 restart case. Floor 1 during a fresh installation is only an internal
state in its one outer migration transaction; it is never a supported committed
service baseline. Supporting an independently persisted floor-1 ledger would be
a new migration/adoption contract, not restoration of previously supported
behavior. No such expansion was made. The final ten-suite PostgreSQL run includes
the retained floor-1 refusal proof and the new exact floor-2/13-ledger upgrade
and repeat-run proof.

## Verification

The [measured PostgreSQL inventory](CXT4B_POSTGRES_INVENTORY.json) records 59 domain
tables, 606 columns, 165 indexes, 122 noninternal triggers, 52 functions, 173
policies and 14 ledger entries (726 migration statements). All 45 RLS-enabled
tables still force RLS; four new control-only relations use private privileges.

- Full offline Rust tests and doctests pass: 258 retained tests plus
  two synthetic OIDC tests. The final service-specific run repeats its synthetic
  tests. Ten PostgreSQL suites are intentionally ignored by ordinary Rust runs
  and are explicitly executed by the disposable runner.
  Final PostgreSQL evidence: `/private/tmp/photara-cxt3c-jcy6zta9/proof.log`;
  the private cluster was stopped by the runner.
- Ten PostgreSQL suites pass: five retained suites (678 counted authorization/
  privilege/sensitivity cells), signed HTTP bootstrap/bounds, atomic replay/
  collision/logout/resume, concurrent installations, deployed-ledger upgrade,
  and write-fault/unknown-commit recovery. The new private privilege matrix adds
  32 cells. Faults after each of 14 write/commit boundaries prove rollback or
  byte-identical after-commit recovery; these are synthetic injected failures,
  not a physical power-loss or live network proof.
- Synthetic RSA keys are generated in test-process memory and discarded. Tests
  verify real signatures, issuer/audience/azp/scope/time substitutions, duplicate
  claims/JOSE features, ID-token substitution, nonce proof, key replacement and
  stale cache. The in-process HTTP test uses those actual signatures and private
  PostgreSQL pools; it performs no external provider request.
- Whole-library all-target check, strict Clippy, formatting and whitespace pass.
  Naming, 46 schema signatures/104 scoped FKs, and 12 canonical fixture containers/
  92 embedded byte records pass. Proposal counts remain unchanged.
- Native bridge passes: revision 19, three progress and two cancellation callbacks.
  Production UI passes using its explicit disposable `supportRootOverride`;
  ten top-level snapshots are generated. The real local SQLite was never opened.
- First Graph run overlapped production UI: 15,091 assertions/three native
  focus/menu failures. The retained failures are native focus-loss precondition,
  native disconnect-menu delivery and its consequent random-state mismatch at
  seed 12648430, Dark/straight, step 21. A serial full rerun passes **15,562
  assertions/zero failures**, recorded at `/private/tmp/photara-cxt4b-graph-serial.log`.
  No Graph source was changed; concurrency/focus interference is an inference.

Runtime verification does not prove real Auth0 tenant settings, HTTPS ingress,
host secret injection, backup restoration or a live Neon smoke test. JWKS redirect/
size/timeout/single-flight protections are implemented; the local suite does not
simulate an external HTTPS discovery/JWKS outage or redirect server. Those host/
provider adapter checks remain before deployment. No real account signup, native
Keychain/PKCE flow, local enrollment or cloud-content worker was activated.

## Exact file inventory and preservation

New files:

```text
crates/photara-service/deploy/README.md
crates/photara-service/deploy/runtime_roles.sql
crates/photara-service/migrations/postgres/0014_onboarding.sql
crates/photara-service/src/http.rs
crates/photara-service/src/oidc.rs
crates/photara-service/src/onboarding.rs
crates/photara-service/src/onboarding_pgtests.rs
docs/architecture/CXT4B_SERVICE_CHECKPOINT.md
docs/architecture/CXT4B_POSTGRES_INVENTORY.json
```

Modified files:

```text
Cargo.lock
crates/photara-service/Cargo.toml
crates/photara-service/src/lib.rs
crates/photara-service/src/management.rs
crates/photara-service/src/pgtests.rs
crates/photara-service/src/runtime.rs
scripts/verify_service_postgres.py
ROADMAP.md
docs/ACTIVE_HANDOFF.md
docs/ROADMAP_0_2_EXECUTION.md
docs/architecture/README.md
docs/architecture/SERVICE_POSTGRESQL_SCHEMA.md
```

The exact baseline diff preserves every platform/Graph/Graph Lab source, native
bridge source, Core/SDK/node/package/local runtime source, all local migrations,
all deployed service migrations 0001–0013 and all canonical fixtures. The entire
`CXT4_ONBOARDING_AND_OPENING.md`, including CXT4c, remains byte-for-byte unchanged.
The real SQLite database, Neon, Auth0, Google, Keychain, user Projects and external
storage were not opened or changed. Nothing is staged, committed or pushed.
