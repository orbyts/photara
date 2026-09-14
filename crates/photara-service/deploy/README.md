# Onboarding deployment boundary

## Current modern Keychain gate (CXT4d)

The user requires current Keychain behavior. Checked operator source now uses
only Data Protection Keychain SecItem calls, an explicit private signed access
group, `WhenUnlockedThisDeviceOnly`, non-synchronizing items and
`LAContext.interactionNotAllowed`. All deprecated ACL/interaction APIs were
removed. The unsigned build refuses with `signing-required` before secret access.

This Mac reports no valid signing identity and the isolated DP-Keychain probe
returns `errSecMissingEntitlement`. Package/sign the operator with a legitimate
profile authorizing `TEAMID.com.photara.operator.development`; the desktop uses
its own separate signed group. Ad-hoc signatures and invented team identifiers
do not establish this boundary. See
[Apple's current requirements](https://developer.apple.com/documentation/technotes/tn3137-on-mac-keychains).

The old installed executable and old file-Keychain item are retained untouched
for recovery only. **Do not follow the historical launcher/provisioning commands
below for current development use.** After signing, securely migrate/reprovision
the same runtime bundle, verify modern access/readiness/restart, and retire the
old item only after recovery is proved. Do not rerun initial provisioning or
regenerate the stable environment cursor key. No modern live-readiness claim has
been made. Read the [CXT4d checkpoint](../../../docs/architecture/CXT4D_NATIVE_AUTHENTICATION_CHECKPOINT.md).

## Development cloud profile (CXT4b-dev)

`config/product-identity.json` is the single checked **public** product/release
descriptor. The Rust runtime embeds it; native build tooling generates typed
Swift configuration and bundle identity/callback metadata from the same file.
`PHOTARA_RELEASE_CHANNEL=development` explicitly selects the operator-only
loopback profile. It binds exactly `127.0.0.1:8080`, uses the recorded Auth0
development tuple, and enters the existing `HttpService::connect` path: real
RS256/JWKS verification, certificate-verified PostgreSQL, separate API/control/
auth-read pools, readiness/ledger/role admission, and the unchanged HTTP router.
It neither migrates nor creates a user. `PORT`, if supplied, must equal 8080.

The process requires only these secret values from the operator's host-local
secure launcher environment:

```text
PHOTARA_DB_API_URL
PHOTARA_DB_CONTROL_URL
PHOTARA_DB_AUTH_READ_URL
PHOTARA_CURSOR_KEY_B64
```

All three URLs require `sslmode=verify-full` and distinct least-privilege runtime
logins. The cursor key is 32 random bytes encoded as unpadded base64url, stable
for this development environment and securely provisioned consistently to its
configured service hosts. The first Mac's credentials/key are now provisioned;
see the host-local runbook below.
There is no `.env` loader, command-line secret argument, app preference, bundled
database configuration or Project-package secret. A host-only launcher may
retrieve values from macOS Keychain and pass them directly to the separate service
process; the operator launcher below implements this boundary. Do not put real secrets in checked JSON, shell
history, logs, the app bundle or a synchronized Dropbox directory.

The original semantic launch is equivalent to:

```sh
PHOTARA_RELEASE_CHANNEL=development cargo run --locked --offline -p photara-service --bin photara-service
```

Use the native launcher below in practice so Cargo never receives credentials.
The four secrets must already be injected securely by the launcher. With no
credentials this command refuses before provider/network initialization and
prints only the missing setting name. `PHOTARA_DB_MIGRATION_URL` is forbidden in
the runtime environment. Existing public-coordinate environment variables, if
present, must exactly match the descriptor; they cannot override it. Diagnostics
expose only the channel and fixed refusal codes, never supplied values.

No channel defaults to development: the service defaults to `production` and
refuses because both hosted descriptors are intentionally unconfigured. At the
remote-acceptance gate, record the exact reviewed HTTPS origin and its own
environment ID in `remoteAcceptance`, then select that channel explicitly. It
may intentionally retain the dedicated development Auth0 tuple. It cannot use
loopback HTTP or an ad hoc runtime origin. Production requires the final brand
and independent issuer/audience/client/callback/logout coordinates; changing an
environment variable alone cannot turn today's build into a customer release.

The current native scripts are ad-hoc development builds and default to the
development descriptor. They generate immutable coordinates at compile time;
normal product UI and process environment cannot change them at runtime. Setting
`PHOTARA_RELEASE_CHANNEL=production` or `remoteAcceptance` fails before native
compilation until the reviewed channel exists. The future distribution pipeline
must explicitly select production and establish Apple signing/Keychain authority.
Neither PKCE nor native loopback requests are activated by this configuration
slice; CXT4d still owns token storage, transport/ATS tests and onboarding UI state.

## Host-local Keychain launcher

The first development Mac was provisioned and verified on 2026-09-13. Its exact
Neon project is `steep-waterfall-28781561`, `main` branch
`br-shiny-glitter-afu6adtk`, database `neondb`. SQL-created logins
`photara_dev_api`, `photara_dev_control` and `photara_dev_auth_read` have only
their respective capability membership and a six-connection ceiling. Roles
are created through SQL because Neon console/API-created roles inherit
`neon_superuser`; see [Neon's role compatibility rules](https://neon.com/docs/reference/compatibility).

All four runtime values are held in one atomic, non-sync login-Keychain item:

```text
service: com.photara.operator.development.neon-main
account: photara-runtime-v1
trusted executable: /Users/suhail/.local/share/photara-development-service/launcher
```

The item contains only the runtime URLs and cursor key, never the Neon operator
URL or user Auth0 tokens. `scripts/photara-development-service.swift` uses native
Security APIs, a helper-only ACL, disabled unattended Keychain interaction and
disabled core dumps. Its `check` action reports availability without exposing
values; `store` consumes a private stdin pipe, validates the complete bundle and
refuses an existing item. There is no secret-export/delete/update command.
The login-Keychain ACL APIs currently produce Swift deprecation warnings; this
host-only operator facility does not replace CXT4d's signed user-token Keychain
design. Recompiling/moving the helper can invalidate its trusted-code ACL; stop
at a Keychain refusal and review access explicitly instead of broadening the ACL.

Start the already installed service from any terminal on this Mac:

```sh
/Users/suhail/.local/share/photara-development-service/launcher run /Users/suhail/.local/share/photara-development-service/photara-service
```

The native helper directly executes the service in the foreground. Only the
four secrets, `PHOTARA_RELEASE_CHANNEL=development` and a fixed PATH are passed;
inherited debug/proxy/loader settings are omitted. A migration-owner environment
setting is an explicit refusal. The target must be an absolute, user-owned,
regular executable named `photara-service`, without group/other write permission.
Stop with Control-C. No background daemon or auto-start job is installed.

Read-only acceptance from a second terminal:

```sh
curl --noproxy '*' --fail --max-time 15 http://127.0.0.1:8080/health/ready
curl --noproxy '*' --fail --max-time 15 http://127.0.0.1:8080/health/live
```

Expected JSON is `{"ready":true}` and `{"healthy":true}` with HTTP 200 and
`Cache-Control: no-store`. Startup warms real pinned Auth0 discovery/JWKS and
verifies live Neon TLS, role separation and exact migration ledger. These health
requests do not sign in, create a challenge, bootstrap or seed user data.
The first start and a same-Keychain restart both passed; the proof process was
stopped afterward. See [measured evidence](../../../docs/architecture/CXT4B_SERVICE_CHECKPOINT.md#cxt4b-dev-runtime-provisioning-and-live-readiness).

To refresh the installed runtime after reviewed source changes, build first
without credentials and stop any previous foreground instance before install:

```sh
cargo build --locked --offline --release -p photara-service --bin photara-service
install -m 700 /Users/suhail/.cache/cargo/targets/release/photara-service /Users/suhail/.local/share/photara-development-service/photara-service
```

The target path above is this Mac's configured Cargo target directory. Never
inject the runtime secrets into Cargo, build tools, a shell `.env`, launchd plist
or the desktop app. Do not rebuild the Keychain helper for an ordinary service
update; the existing helper and its access identity remain stable.

The one-time initial operator sequence was:

1. Compile `scripts/photara-development-service.swift` with `swiftc`, install the
   resulting `launcher` and separately built `photara-service` under the private
   mode-700 host-local directory above; both executables are mode 700.
2. Run `python3 scripts/provision_development_service.py --preflight`. It uses
   the existing authenticated Neon CLI and Homebrew psql to verify exact
   branch/database, floor/ledger and zero Accounts/Libraries/defaults/receipts.
3. Run the same helper with `--provision` once. It generates random values only
   in memory, establishes Keychain durability, creates all three roles in one
   SQL transaction, authenticates separately with each runtime credential and
   checks effective privileges and unchanged empty counts. Secrets use stdin
   pipes and private child environments; errors never include SQL/URL bodies.
4. Launch via the native helper, verify both health endpoints and restart
   readiness, then run the Python helper with `--postflight` for read-only
   privilege/empty-data verification. These empty-baseline operator commands
   deliberately refuse once real onboarding has created data.

This operation is complete: **do not rerun `--provision` to rotate or install a
second Mac**. It refuses any existing runtime role or Keychain item. An error
after Keychain storage or an uncertain SQL commit requires read-only inspection
before recovery; it never deletes an item, adopts a role, overwrites a password
or regenerates the stable cursor key automatically. Stop on a locked Keychain,
ACL refusal or missing operator authentication and resolve that exact gate.

The psql operator helper uses separate private libpq connection fields with
`PGSSLMODE=verify-full` and `PGSSLROOTCERT=system` (supported by local psql 18.6).
The runtime URL remains `sslmode=verify-full` and uses its Rust TLS verifier.
See [PostgreSQL certificate trust options](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-SSLROOTCERT).

Second-Mac setup still needs reviewed secure provisioning of this existing
environment's values into that host's Keychain. This code intentionally has no
clipboard, file export or synchronized secret-distribution path. Native PKCE,
user-token storage, local/cloud reconciliation and actual multi-Mac hydration
remain CXT4d/e; the first-Mac service proof does not claim those are complete.

## Hosted adapter (deferred)

Fly.io is the selected first service host. The checked `Dockerfile` and `fly.toml`
provide a runnable host adapter with public HTTPS ingress, an unprivileged runtime
identity, health checks, bounded concurrency and encrypted runtime-secret injection.
The manifest intentionally contains no globally unique app name, database URL,
credential, Auth0 coordinate or custom domain. `sjc` is the provisional first
region because the initial operator is in Northern California; confirm latency to
the selected Neon branch before live creation and change it if necessary.

`runtime_roles.sql` creates three fresh LOGIN roles with one capability membership
each, no privileged capabilities and a six-connection ceiling. It refuses name
collisions transactionally. Password authentication is disabled (`PASSWORD NULL`);
the script neither creates nor embeds a credential. The disposable PostgreSQL
runner exercises this same template over its private trust-authenticated socket.

After the missing host inputs are approved, the host's operator job must provision
random credentials directly into its encrypted secret store, attach each only to
its corresponding login, and inject three private URLs into the service process.
Do not pass passwords in CLI arguments, shell history, repository files or logs.
The migration-owner URL belongs only to a separate operator migration job. The
public Native application needs no client secret or Management API credential.

`HttpService::connect` accepts the exact public configuration and three private
URLs in API/control/auth-read order. It requires `sslmode=verify-full`, bounds each
Storexa pool to four connections and rejects role/floor/ledger drift. It loads
only pinned public discovery/JWKS and never runs migrations. Its Axum router has
only the approved onboarding/session/health endpoints; no generic controller,
SQL, invitation, content, media or administration endpoint is mounted.

The selected host adapter must serve that router through HTTPS ingress, supply
`ConnectInfo<SocketAddr>` from its authenticated transport, use a restricted
process/filesystem identity, disable request/header/body/SQL-value logging and
bound accepted headers/connections at ingress. The router ignores forwarded-IP
headers; a proxy deployment must review its trusted peer/IP handling explicitly.
No endpoint or host is selected from request headers or bearer claims.

`photara-service` is the public runtime entry point. `photara-migrate` is a separate
operator-only executable requiring `PHOTARA_DB_MIGRATION_URL`. Do not store that
owner URL as a Fly application secret: application secrets are available to the
runtime process. Run migration 0014 from a separately authorized operator/CI job,
remove its credential when complete, then deploy the runtime with only the three
least-privilege URLs.

The hosted runtime requires these encrypted Fly secrets:

```text
PHOTARA_CURSOR_KEY_B64
PHOTARA_DB_API_URL
PHOTARA_DB_CONTROL_URL
PHOTARA_DB_AUTH_READ_URL
```

Its public `PHOTARA_RELEASE_CHANNEL` must select a checked hosted descriptor.
The five old public-coordinate settings are now optional exact-match assertions,
not secrets or endpoint overrides.

Build with the checked Cargo.lock and deploy an immutable artifact only after
the host-specific adapter, encrypted-secret injection, backup/restore proof and
separately authorized live migration/smoke test are reviewed. Packaging a daemon,
provisioning provider settings, the unique Fly app/origin and Auth0 release tuple
are reviewed. CXT4d/e remain separate gates.
