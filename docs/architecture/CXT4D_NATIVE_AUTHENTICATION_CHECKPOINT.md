# CXT4d native authentication, durable reconciliation and modern Keychain gate

## Account sidebar presentation — 2026-09-13

After user-confirmed live login/logout, the Accounts entry moved from the
top-right toolbar to the bottom-left library sidebar (and editor footer). The
signed-in row has a circular 20-point avatar and account name; signed out it has
a person icon and **Sign in**. The existing menu retains Sign Out and exposes
Settings; the empty toolbar gear is removed. The native image itself is scaled
and circularly masked because native Menu label extraction bypassed the old
SwiftUI-only frame/clip. No authentication/service/storage code changed.

Production UI verification and signed build passed, log
`/private/tmp/photara-sidebar-accounts-ui.log`. Updated captures use a large
nonsquare raster avatar and real ApplicationShell in Light/Dark at minimum and
large window sizes. Visual inspection confirms a small circular avatar anchored
at the lower left. **101** earlier gate source hashes remain identical; only the
three UI sources and the UI fixture changed. The previous full gate was not
repeated for this aesthetic change. Evidence: `.build/cxt4d-gates/sidebar-ui-passed.json`
under the macOS app. Current executable SHA-256:
`a87e52ccbb977d31d90706ba579a9fed075ddeb7f0c4d9392c07271db2372661`.

## Accounts and explicit library choice — 2026-09-13

The user reports successful live Google enrollment and a new Neon record. The
following feature work preserves that live state; automated checks use isolated
credentials and temporary databases, with no automated live Google login/logout.

The shared app session drives the Accounts toolbar menu on both the opening and
project screens. It exposes the current display profile and Sign Out. Explicit
sign-in waits at a library picker before opening the cloud destination, even for
one choice. Rust validates the principal, original receipt and fresh active session
before offering the choice, and validates fresh access again after selection.
The existing default-library service contract supplies the one supported choice.
Multi-library listing, creation, invitations, billing and account settings are
deferred; no new service endpoint or deployed migration is required here.

Sign Out first advances the durable local session to nil and marks the binding
signed out, then removes only the refresh token/refresh-in-flight marker. The
device credential, opaque namespace locator and original binding remain. Rejoin
requires fresh PKCE login, exact verified issuer/subject and credential namespace,
the stored device commitment, and current service access. It reuses the applied
operation and never calls onboarding bootstrap. Local edits survive reconnect.
Original independent local libraries are preserved when an existing cloud default
was explicitly selected; opening-state lookup follows only that origin's selection.

Google profile claims are extracted after ID-token verification. They are bounded
display metadata, not identity authority. Startup reads an environment/library
cache without credentials or network. Only successful deliberate sign-in triggers
an optional bounded Google-hosted image fetch and 96-pixel thumbnail cache. There
is a circular person fallback. Existing sessions populate the profile on next login.

Cancellation, wrong accounts, stale generations and cleanup failure leave the
retained cloud library signed out; selection is not inferred from elapsed time.
The current opening model also rejects stale picker callbacks and suppresses repeat
sign-in/logout actions while an operation is running.

Verification completed with `scripts/verify-cxt4d-gates.sh` (exit 0), log
`/private/tmp/photara-cxt4d-accounts-gates.log`, **105** frozen source hashes and
signed executable SHA-256 `b6a04b3637e0347635782a95fb614de5e645585193aa13f4c187e7f3f2a890eb`.
Security/Auth0/presentation: **94/58/64** assertions; native driver: **124**;
native service: **253**, **17** scenarios including separate logout/rejoin/relaunch
processes; disposable PostgreSQL: **11** tests; process furnace: **99**, **13**
scenarios; installed service: **10**; signed Keychain: **8**; ordinary Rust: **62**.
Formatting, Clippy with warnings denied, bridge/product/schema/naming, shared UI,
production UI and signature checks passed. Library-picker Light/Dark captures and
the account control were visually inspected. Native tests prove no bootstrap on
rejoin, unchanged device secret, one account/library, fresh access after selection,
wrong-account rejection, cancelled selection, and cleanup failure preserving logout.

## Earlier terminal-expiry and passive-launch checkpoint

Updated 2026-09-13 for terminal expiry recovery and passive launch.

## Current deliberate sign-in behavior

The user's final preference is Notion-style returning-session behavior: offer
sign-in for an unconnected library, start authorization only after a click, and
open a connected library quietly on subsequent launches without a **Signed in**
label. `OpeningCloudModel.loadSavedState` only reads the production driver's
local Rust metadata projection. It never calls automatic recovery. Loading is
not an authentication attempt; no Keychain access, token refresh, service start,
network request or browser occurs. The button waits for this local read to finish.
Saved `cached`, `signed-out` and `access-disabled` states are differentiated and
scoped to the configured environment. Cached presentation grants no fresh access.

An expired unresolved enrollment now has an explicit durable terminal disposition,
`abandoned-expired`, separate from unknown/cancelled. Admission requires the exact
bound principal/current generation, no recorded receipt, a canonical authenticated
operation-not-found response, and the native caller's proof-unavailable condition.
The driver checks a final exact authenticated GET after proofless replay is refused,
so a receipt that has become visible wins over terminal disposition.

Local migration 0014 adds append-only disposition and replacement-lineage tables.
It preserves the original 0013 intent bytes and checksum, including the physical
unknown row, while the journal projects its effective terminal state. The reader
and writer floor rises to 4. Transactional guards exclude terminal rows from the
one-unresolved-attempt rule. Disposition/lineage cannot be deleted or rewritten.

An explicit **Sign in again** click can retire the previous attempt and then
start one fresh authorization. The new operation/challenge/proof reuses the same
local Library, Device, credential reference and commitment, and requires the same
verified principal before replacing its refresh token. The native lock and durable
SQLite guards prevent duplicate active attempts. A cancelled/prepared replacement
keeps the original shared credential. A late old receipt is retained but terminal
operations cannot dispatch again or become the current local binding.

The real service's principal serialization and default/device constraints converge
both operations on the same Account/Identity/Device/default My Library/membership.
The expanded native TCP/Rust/PostgreSQL furnace exercises actual process restarts
after unknown outcome, terminal disposition, prepared replacement, server commit,
and receipt storage. It covers cancelled replacement, concurrent clicks, one-click
retire-and-retry, and old commits arriving before or after the replacement. SQL
asserts exact ownership counts; two operation receipts in a late-commit race are
expected evidence, with only one set of ownership records. Browser/provider and
disposable credential persistence remain synthetic; service replies are real.
Proof loss across restart exercises the unavailable-proof branch; JWT expiry
rejection is covered by the separate authentication tests. The late-commit fixture
resubmits the synthetic original request with its still-valid proof to model an
old dispatch arriving late. Production does not persist that proof.

Deterministic Rust tests also reject malformed absence, still-available proof,
stale generation, changed credential and wrong principal; retain immutable history;
exercise competing independent pools; and verify populated v13 unknown-attempt
upgrade with every original migration checksum unchanged. Presentation tests
prove zero authentication calls on launch, no busy state or sign-in flash for
cached connections, and rejection of old callbacks after terminal replacement.
Production Light/Dark captures include the 760×560 minimum opening window.
The native button retains an explicit accessibility label matching its visible
action. VoiceOver interaction itself has not been exercised.

No live Google authorization, live terminal transition, new live cloud binding or
direct Neon write was performed in this implementation task. The earlier exact
operation evidence below remains historical evidence, not a claim of new acceptance.
The active handoff and source-hashed gate manifest record final build verification.

## Current verification and signed build

The complete `scripts/verify-cxt4d-gates.sh` run exited **0**. Log:
`/private/tmp/photara-cxt4d-terminal-gates.log`. The passing manifest at
`platform/macos/photara-app/.build/cxt4d-gates/passed.json` records **103**
unchanged source hashes; they and the executable hash were checked after completion.

Evidence: **94 + 52 + 56** native security/Auth0/presentation assertions,
**106** browser/journal assertions, **225** native service-flow assertions across
**16** real TCP/Rust/PostgreSQL scenarios (including restart stages), all **11**
executed disposable PostgreSQL tests, **99** process-furnace assertions across
**13** scenarios, **10** installed-service checks with unchanged port ownership,
**6** signed Keychain checks, and **61** ordinary Rust tests. Bridge execution,
product/schema/naming/whitespace, shared UI and signed production UI checks passed.
Separate Rust formatting and Clippy with warnings denied passed. Minimum opening
captures in Light/Dark were visually inspected; the fixture now prevents automatic
window expansion and asserts the requested 560-point content height.

Signed app: `/Users/suhail/.codex/worktrees/84ad/photara/platform/macos/photara-app/.build/app/Photara.app`.
Executable SHA-256: `86d7ca25a946b4932edf3fc32bb2c208465d1772eaad1375f3844117f4630de0`.
The app was built and signature-verified without starting a live Google sign-in.

## Historical audience-policy repair checkpoint

The following records the earlier repair and its then-current proof blocker.
Its automatic-startup and blocked-retry behavior is superseded by the terminal
transition and deliberate launch behavior above.

## Confirmed bootstrap failure and live operation outcome

The retained operation is `35d65e1f-48bb-4fb0-ab06-b4ced18cd4f1`, in
`photara-development`. It was not discarded or replaced. A signed, narrowly
scoped diagnostic read its SQLite coordinates and matching Data Protection
Keychain item in memory, rotated the retained refresh token through the existing
durable marker/lock, and made the authenticated operation GET. No token, subject,
credential, commitment, or raw response was printed or written to diagnostic files.

The old installed service returned **401 authentication-required**. The retained
access token had the exact issuer, principal, native client, onboarding scope and
600-second lifetime, but its audience was the pair consisting of the Photara API
and the issuer's `/userinfo` endpoint. The checked Rust release policy had
`allow_userinfo_audience=false`. Auth0 documents that pair for a custom API plus
`openid`: <https://auth0.com/docs/secure/tokens/access-tokens/get-access-tokens>.
The existing single-audience fixture missed this provider-shaped case.

The checked policy now permits the API plus only that exact issuer's `/userinfo`.
API audience presence, unique audiences, the two-audience bound, exact issuer and
client, RS256/JWKS, expiry, lifetime, scope and user-token restrictions remain
mandatory. Foreign, duplicate, excess, userinfo-only and wrong-client audiences
are rejected. This fixes application policy; no Auth0 secret or tenant setting was
changed. Capabilities now include `authentication_profile` with exact value
`auth0-rs256-api-userinfo-v1`. The native gate refuses an old or missing profile
before starting browser authentication or creating an intent.

The corrected operator/service was signed, installed with the old signed bundle
retained for rollback, and the verified original port owner (71768) was gracefully
replaced. The corrected service passed readiness and the new capability check.
Installed service SHA-256 at repair:
`cd2d7d92b4cef38f97c80d276c46b6647dd26adae15c0847c13f473842e63626`.
Backup: `~/.local/share/photara-development-service/Photara Development Operator.before-recovery-6f3edf5f-455c-4a2f-828c-8e1d1018d3bb.app`.
No migration, credential provisioning, direct Neon write, commit, push or publication
was performed. The operation boundary reads the actual deployed database; this is
not an inference from migrations merely being present in the repository.

After repair, the same authenticated GET returned **404 operation-not-yet-found**:
there is no committed receipt for this exact operation. A signed helper then ran
the production recovery driver, with browser and authorization-code exchange
explicitly prohibited. It looked up that same operation and replayed the original
canonical request with the matching retained device credential. The service
refused first commit without the original fresh enrollment proof. The driver
reported `operationReplay / proofRequired` and preserved the unknown journal and
Keychain item. No receipt was fabricated and no local cloud binding was applied.

**Live acceptance remains blocked:** the original ID-token enrollment proof was
transient, has expired, and is unavailable after the failed attempt. A refresh
access token proves the retained session but does not replace a fresh challenge
proof for a first commit. The user prohibited another sign-in; that instruction
was honored. No code relaxes this trust boundary. The repaired build must not be
represented as having completed this user's cloud enrollment.

## Recovery behavior

- Rust returns only an opaque credential reference for an unresolved bound
  journal. It still refuses command/principal disclosure to a different identity.
- The signed native store resolves only the exact private service/reference item.
  Refresh uses the existing cross-process lock, durable in-flight marker and atomic
  rotation. A pinned signed access-token check establishes the recovered identity;
  its installation/environment/principal namespace must match the Keychain item
  before Rust opens the original bound journal.
- Opening the library checks for a retained bound operation and automatically
  attempts recovery. An ordinary local opening does not query Keychain or contact
  the service. Recovery never starts a browser or creates a new operation.
- Failed/lost bootstrap results are looked up through the authenticated existing
  operation endpoint. A committed receipt is validated and recorded independently,
  then a fresh session DTO gates local binding of the original Library.
- A canonical not-found response permits at most one exact replay. While the
  original proof is still in memory and fresh, that replay may safely complete a
  rolled-back attempt. After restart, replay without proof can recover a racing
  existing commit but cannot create an uncommitted account/library. The latter
  produces the explicit retained-proof blocker, not a fresh operation.
- HTTP diagnostics retain phase, bounded status and a fixed allowlisted service
  code. Provider text, arbitrary response bodies and secrets never reach UI.

## Real service and database furnace

`scripts/verify_service_postgres.py` now builds the native service-flow executable
and runs it as part of the disposable database gate. The Swift production driver,
Rust generated ABI, temporary SQLite, pinned URLSession, TCP socket, production
Axum handlers, JWT signature verifier, onboarding controller, migrations and
least-privileged PostgreSQL roles are real. Only the provider/browser, credential
persistence in the disposable crash fixture, and explicit fault injection are
synthetic. Bootstrap, operation and session responses are never canned.

The furnace covers normal creation, lost committed response, a real request
**timeout after database commit**, post-commit 503, rollback 503, not-found followed
by exact replay with the original proof, transient operation-lookup 503, real 401,
missing/stale capabilities, and a real native process exit immediately after
server commit followed by a new native process recovering the same journal.
Database queries assert either zero committed identity/device/receipt/default rows
or exactly one of each; no partial or duplicate account/library path is accepted.
Absent operations after lost proof remain blocked with unchanged canonical bytes.

The existing signed process/startup furnace, installed-service smoke, Keychain
acceptance, native auth/journal tests, Rust/bridge checks and both UI gates remain
mandatory. `scripts/verify-cxt4d-gates.sh` now includes all disposable PostgreSQL
proofs before any signed UI build and records Rust recovery/service and native
furnace source hashes as well as the application source hashes. Final gate and
signed-build evidence is recorded in the active handoff and the generated
`.build/cxt4d-gates/passed.json`; a failed run invalidates the old marker first.


## Final recovery verification

The complete mandatory gate exited **0**. Evidence:
`/private/tmp/photara-cxt4d-recovery-gates.log`; current manifest:
`platform/macos/photara-app/.build/cxt4d-gates/passed.json`.
All **70** captured source hashes matched after the run.

- Native security/Auth0/presentation: **88 + 52 + 31** assertions.
- Production browser/journal flow: **106** assertions against the real Rust ABI.
- Real TCP/Rust/PostgreSQL recovery furnace: **46** native assertions across
  **10** scenarios, plus committed database counts for every scenario.
- All **11** disposable PostgreSQL tests passed; these were actually executed,
  not left ignored. The ordinary Rust pass separately reports these ignored
  because only the explicit disposable runner provisions their environment.
- Signed startup/process furnace: **99** assertions / **13** scenarios.
- Installed service: **10** checks plus unchanged ownership during its smoke test.
- Signed Data Protection Keychain: **6** assertions, including exact-reference
  discovery/read; all synthetic items removed.
- Rust: **44** library, **8** bridge, **6** service-library and **1** entrypoint
  tests. Clippy with warnings denied and Rust formatting passed separately.
- Bridge execution, product configuration, schema/naming/whitespace, shared UI
  and signed production UI gates passed. Fresh Light/Dark opening captures
  were visually inspected; native title/sidebar/layout are intact.

Final Apple Development signed app:
`/Users/suhail/.codex/worktrees/84ad/photara/platform/macos/photara-app/.build/app/Photara.app`

Executable SHA-256: `e74259c65377a2c1aac74b6eac1f14748b0bfe2e84a9a05d85fa4594f455019b`. Strict app/nested-code verification passed.
This app was built and verified; it was not opened for another live sign-in.
The original live operation remains **unknown**, principal retained, **0** local
receipts and **0** local cloud bindings, confirmed by a final read-only SQLite
query. Its authenticated service lookup confirmed no committed receipt, and its
original-request replay was blocked by missing fresh proof. Passing tests and a
signed build do not imply that this live enrollment has completed.

## Historical startup checkpoint (superseded above)


Updated 2026-09-13 after the subsequent live **Local service startup failed** result.
The earlier signed build and injected-flow harness were insufficient evidence for
service startup. The service layer now has a real-process furnace and a separate
installed-runtime smoke test. Final gate/build evidence is recorded below.
Real Google onboarding and its Neon aggregate audit remain unperformed here.

## Missed startup race and stronger build gate

The previously presented executable (`f71f59db…`) failed at local service startup
although the installed service was healthy on port 8080 (reported PID 71768,
readiness latency about 0.57–0.67 seconds). The old path used a one-second readiness
request. A transient miss could launch a second child, which could lose the bind;
child exit was then treated as failure without checking the service that won.
Those are confirmed defects in the old code. Attribution of that particular live
failure to the bind race is supported by the healthy-service evidence, but the
old code discarded child output, so the exact historical child-exit cause is not
claimed as directly observed.

The provisional 3/2/3-second patch is superseded by `NativeDevelopmentService`:

- One 25-second monotonic startup budget. Readiness and capabilities are requested
  concurrently with bounded real URLSession transport. A valid capability document
  plus delayed/503 readiness is a starting service, not an absent service.
- The exact canonical capability validator is shared with the enrollment driver.
  Wrong health/capability responses cannot establish readiness.
- A private cross-process startup lock covers launch and readiness observation.
  Every lock winner probes again before spawning; other launchers keep observing.
  There is at most one new child per startup call. A crashed owned service may be
  restarted by a later call.
- A launched child's exit never immediately decides failure. Observation continues
  within the same budget, allowing an independent bind winner to become ready.
- Before launch, the actual operator bundle and embedded service must have strict
  valid Apple signatures from the desktop's team with the exact identifiers.
- Cancellation/startup failure cleans up only a newly owned child. Cleanup sends
  TERM, waits independently of cancellation for one second, escalates to KILL if
  necessary, and reaps the child. A reused/unowned service is not killed.
- Production remains pinned to checked loopback port 8080. Alternate origins,
  ports and fixture environment forwarding are available only under the separate
  `PHOTARA_SERVICE_FURNACE` compile flag; app builds never set it. The real Rust
  executable still rejects an alternate development PORT before credential reads.
- Operator source now limits `run` to its own embedded executable by resolved file
  URL. The furnace caught and fixed a raw path-string comparison that differed
  between a Foundation launch and direct subprocess launch. The installed operator
  was not replaced during this task; its signature, existing run contract and
  Keychain access are checked separately. The fixture operator compiles the current
  source with a unique synthetic Keychain namespace and test-only cleanup command.

**Policy:** run `zsh scripts/verify-cxt4d-gates.sh` with the desktop/operator profiles
and offline Rust target configured. It orders native auth/journal, real-process,
installed-runtime, signed Keychain, Rust/bridge and shared UI gates before building
and testing the signed production UI. Its final `passed.json` records the executable
hash and onboarding source hashes; it refuses a passing result if those sources
changed during execution. A passing injected service stub alone must never unlock
another UI build for user testing.

## Which layers are real

`test-service-process-furnace.sh` and `service_process_furnace.py` compile a signed
Foundation launcher using the **production startup controller**, a signed operator
using the **operator source**, and a native socket HTTP fixture. Each case uses an
isolated ephemeral port/configuration and supervised subprocess groups. The actual
Process/exec boundary, code-signature checks, private startup file lock, URLSession,
TCP bind, timeout, cancellation, TERM/KILL and reaping are real. The operator reads
only its own unique synthetic Data Protection Keychain item, never live credentials.
HTTP health/capability responses are simulated; Auth0/JWKS, PostgreSQL and Neon are
not part of those fixture processes. No token endpoint exists in the HTTP fixture.

The furnace passes **99 assertions across 13 real-process scenarios**:

1. No listener → signed operator exec → socket bind → exact readiness/capabilities.
2. Healthy existing listener → zero duplicate launches and no owner termination.
3. Slow 1.6-second health response → reuse without launch.
4. First health request delayed four seconds → actual timeout, retry, zero launches.
5. Explicit delayed-readiness barrier → repeated observation without launch.
6. Two independent launcher processes → one child and both callers ready.
7. Child loses bind and exits while another authority is still unready → success
   only after the winner's readiness barrier opens.
8. Stale/wrong capability document on the port → refusal without launch/kill.
9. Wrong health body → refusal without launch/kill.
10. Child exits with no service → bounded failure and cleanup.
11. App task cancellation during startup, child ignores TERM → bounded KILL/reap.
12. Owned service crash → reaping and restart by the same controller.
13. Never-ready existing service → bounded failure without terminating it.

Every case verifies known fixture PIDs are gone and its port can be rebound.
The outer supervisor also kills/reaps fixture process groups on assertion failure.
The unique synthetic Keychain item is removed in `finally`. The source's
nonembedded-child refusal is checked before credentials could be delivered.
Evidence from the complete-gate run: `/private/tmp/photara-service-furnace.9uOYE8/report.json`.

`test-installed-service.sh` passes **10 assertions**, plus an unchanged-port-owner
check, against the actual installed artifacts. It verifies both code signatures,
runs the installed operator's read/validate-only `check` in a fresh process,
proves desktop → operator Keychain access fails with `errSecMissingEntitlement`,
and runs the production readiness/capability algorithm three times through real
URLSession with launching disabled. Exact liveness is also checked. A separate
credential-free child of the actual Rust binary must reject PORT=18081 and exit
before database/provider initialization. The existing service on port 8080 is
neither killed nor restarted, and no installed file is replaced. Its readiness
endpoint may perform its normal read-only dependency checks; no live credential
is exported into a fixture and no Neon mutation or Google login is performed.

This proves the local process contract and current installed health. It is not a
fresh live-Neon enrollment, an Auth0 browser callback or the real aggregate audit.
The prior **106** Rust-backed browser/journal assertions remain a separate layer:
the browser completion executor and Rust/SQLite are real, while token/bootstrap/
session boundaries there are synthetic. Signed Keychain probes use the actual
store with isolated synthetic items. Returning-session and unknown-outcome replay
UI remain the separately documented gates below.

## Final complete-gate evidence

`scripts/verify-cxt4d-gates.sh` completed with **exit 0**. The log is
`/private/tmp/photara-cxt4d-final-gates.log`; the generated passing record is
`platform/macos/photara-app/.build/cxt4d-gates/passed.json`. All 40 tracked onboarding
source hashes matched at the end of the run. A subsequent run clears the previous
passing marker before starting, so a failed run cannot leave stale passing evidence.

- **99** real-process assertions / **13** scenarios.
- **10** installed-runtime assertions + unchanged shared port owner.
- **106** browser/journal assertions.
- **88** native security + **40** Auth0 HTTP/JWKS + **28** presentation assertions.
- **4** signed Data Protection Keychain assertions; probe items removed.
- **44** library + **8** bridge + **6** service-library + **1** service-entrypoint
  Rust tests passed; **10** disposable-PostgreSQL tests explicitly ignored.
- Bridge executable passed: revision **19**, **3** progress / **2** cancellation events.
- Product configuration **7** passed; schema **46** signatures / **104** foreign keys;
  naming guard and whitespace checks passed.
- Shared UI and final production UI both passed. Final Light/Dark opening captures
  were visually checked: one principal Photara title, My Library sidebar, native
  selection, bounded footer and unchanged neutral layout. Capture hosts remain
  isolated ad-hoc fixtures, not a real sign-in session.
- Final Apple Development app passes strict codesign verification. Test-only
  alternate-origin / observation / fixture-stop symbols are absent from the app.

Signed build in this worktree:
`/Users/suhail/.codex/worktrees/84ad/photara/platform/macos/photara-app/.build/app/Photara.app`.
Executable SHA-256:
`b2bf6c109c4a7430a30a95aa0936b5568614505768a2c1c02a362291b1c89ac9`.
The app was not opened for user/provider sign-in by this task.

The local process and installed-readiness gates are now clean. Actual interactive
Google callback/token issuance/bootstrap and the Neon aggregate audit remain the
next real-provider acceptance. No claim is made that simulated HTTP health or a
synthetic cloud-ready state proves that provider acceptance.

## Earlier browser/journal audit and verification

The following sections retain the earlier failure audit and verification history;
their earlier signed-build hash and service-readiness policy are superseded above.

## Native onboarding failure audit (macOS 26.6.2)

Four independently checked failure causes explain the observed sequence:

1. Foundation's canonical reserialization escaped URL slashes, while the Rust
   canonical codec emits literal `/`. Retained `AuthenticationJSON.validateCanonicalObject`
   uses `.sortedKeys` and `.withoutEscapingSlashes`; malformed/duplicate/noncanonical
   envelopes still fail closed. The production capabilities path is now covered
   end to end, including its literal loopback URL.
2. Native preparation originally sent a bare UUID; Rust requires `keychain:<UUID>`.
   Retained `NativeCredentialReference.create()` supplies that exact coordinate.
   The new harness passes it through the generated Swift ABI into real Rust/SQLite.
3. `Photara-2026-09-13-161124.ips` identifies `EXC_BREAKPOINT/SIGTRAP` in
   `_dispatch_assert_queue_fail` / `_swift_task_checkIsolatedSwift`, at the browser
   completion on `com.apple.NSXPCConnection.m-user.com.apple.SafariLaunchAgent`.
   The stack includes AuthenticationServices `_startDryRun:`; it does **not** prove
   that Google delivered a successful authorization callback. Retained
   `NativeAuthenticationCallback.handler` is manufactured `nonisolated`, then hops
   explicitly to MainActor. Session-factory injection now exercises the actual
   browser owner's continuation/generation handling from a background queue.
4. The crash left an unresolved prepared intent. A read-only count of the real
   local database found **one `prepared` intent and zero receipts**. The old driver
   unconditionally prepared a new operation, conflicting with
   `onboarding_unresolved_library`; the UI flattened this into its generic message.
   The audit did not change that database. The next authorized sign-in recovers
   its unsubmitted intent automatically.

## Hardening and actual harness boundary

`ProductionOpeningCloudDriver` now injects service HTTP, service startup, browser
presentation/session construction, credential storage, token exchange, support
location, installation and clock. Tests compile this production source with the
real generated UniFFI binding and Rust library. They use fresh private temporary
SQLite databases, synthetic HTTPS identity coordinates, in-memory credentials and
HTTP/token stubs. No real provider token, Auth0 credential, Neon credential or
user database is used as a fixture. The existing Auth0 client suite independently
exercises the real token/JWKS implementation with ephemeral RSA keys and HTTP fakes.

- A private, nonblocking `flock` is held across the whole attempt and awaited
  cleanup. A second driver/process cannot cancel another live attempt. Process
  death releases the lock. Rust reads the pending journal and its compare-and-set
  cancellation retires only `prepared` / unbound operations; immutable bytes remain.
- Bound/unknown/receipt-bearing journals are retained, and a safe recovery-required
  message replaces repeated preparation. The first-enrollment driver does **not**
  yet implement authenticated unknown-result replay or returning-session UI.
  It must not be advertised as completing those acceptance cases.
- Credential namespaces retain environment/client/issuer/subject/installation
  isolation and now include the journal's unique reference. The item also carries
  that reference in `kSecAttrGeneric`. Pre-dispatch crash cleanup can delete that
  exact staged item without enumerating principals or deleting an existing session.
  Failed Keychain cleanup leaves the prepared coordinate available for retry.
- Cleanup is awaited, not a fire-and-forget `Task` in `defer`. A failed dispatch
  is treated conservatively until Rust positively confirms the operation remained
  prepared. Unknown/committed dispatch evidence and credentials are preserved.
- Cancellation is checked at asynchronous boundaries, after token exchange and
  credential insertion, and inside the journal actor before preparation/dispatch/
  application. A late receipt is recorded independently, but cancelled work cannot
  fetch/apply new authority. Cancellation advances the durable session generation
  for submitted work. The opening view cancels work when it disappears.
- The journal opens lazily on its storage actor and closes explicitly; opening
  availability inspects public signing metadata, without a Keychain query.
- The challenge nonce must decode canonically to 32 bytes; its ID must be nonnil;
  expiry must be canonical decimal milliseconds, unexpired and bounded. Expiry is
  rechecked after browser return/token exchange. Callback scheme must match the
  configured redirect; callback consumption remains one-use and state-bound.
- Browser start/completion failures are distinguished from a user cancellation.
  `NativeOnboardingFailure` retains only fixed phase/reason flags, never an
  underlying provider error, URL, response body or token. Presentation covers
  signing, service, compatibility, recovery, preparation, challenge, browser,
  callback, token verification, Keychain, dispatch, bootstrap, receipt, session,
  reconciliation and cleanup. Truncated sidebar messages have their full safe
  text available as help.
- Local service readiness uses a one-second request/two-second resource timeout,
  an eight-second startup polling deadline, cancellation checks, and cleanup of
  its own failed startup process. A running owned process is not duplicated.
- The opening toolbar uses `.toolbar(removing: .title)` and one `.principal`
  product title. This replaces the one-time NSWindow title visibility assignment
  that SwiftUI could override. Native NavigationSplitView/List behavior, the
  `My Library` sidebar label, neutral styling and bounded footer remain intact.

## Hardening verification (2026-09-13)

- `zsh scripts/test-native-onboarding.sh`: **106 production-boundary assertions**.
  Covers canonical capabilities, actual Rust preparation/credential-reference
  formatting, abandoned prepared recovery and orphan-credential cleanup, PKCE
  URL fields, challenge validation, non-main browser completion, failure phases,
  duplicate/late callbacks, failed start, concurrent attempt exclusion, cancellation
  at capabilities/challenge/browser/token/storage/bootstrap/session, local revision
  change before dispatch, late receipt retention, unknown-operation preservation,
  cleanup failure/retry, same-ID reconciliation and absence of token strings in
  the database. Auth0 token/bootstrap/session boundaries use explicit synthetic
  stubs; this is not live Google or Neon acceptance.
- `zsh scripts/test-native-authentication.sh`: **88 security + 40 Auth0 network +
  28 presentation assertions**. Swift 6, warnings-as-errors. macOS Security access
  is needed only for ephemeral synthetic RSA keys.
- `zsh scripts/test-native-onboarding-keychain.sh`: **4 signed Data Protection
  assertions**. Uses the real native store under the desktop profile, inserts two
  unique synthetic items, removes only one by reference, verifies the other,
  atomically replaces it, and removes all probe items. No real item is enumerated.
- `cargo test -p photara-library -p photara-bridge`: **44 + 8 passed**.
- `cargo test -p photara-service --lib`: **6 passed, 10 explicitly ignored**
  disposable-PostgreSQL tests. No production PostgreSQL test was run.
- `platform/macos/photara-app/verify-bridge.sh`: **passed**, graph revision 19,
  3 progress events / 2 cancellation events.
- Product configuration: **7 passed**; frozen schema verification: **46 signatures,
  104 foreign keys**; generation-two naming guard and whitespace checks pass.
- Shared UI verification: **passed**. Light/Dark opening snapshots retain the
  native sidebar/footer and one principal product title.
- `zsh platform/macos/photara-ui-tests/verify-production-ui.sh`: **passed** final
  production composition, inspection, Core action/undo/redo, HDR proxy, temporary
  SQLite Library, thumbnails, save/reopen and presentation-purity verification.
  This command also builds the **Apple Development signed Photara app**. Final
  `codesign --verify --strict --verbose=2` passes with host trust-store access;
  its only Keychain group is `524GTA93Q3.com.photara.desktop`, and the registered
  callback scheme is `com.photara.desktop`.
  Executable SHA-256:
  `f71f59db513ddd5a977e10e4d4c05c56820402de5114697e3f424aa67c036a1f`.
  Build: `platform/macos/photara-app/.build/app/Photara.app` in this worktree.
  Final Light/Dark captures:
  `platform/macos/photara-ui-tests/.build/production-snapshots/production-launcher-false.png`
  and `production-launcher-true.png`. Both were visually inspected: one principal
  Photara title, My Library sidebar, native selection and bounded footer.
  Capture hosts are isolated ad-hoc test apps (their cloud action is deliberately
  unavailable), not a live sign-in or screenshot of the signed app in use.

Reproduction uses `CARGO_NET_OFFLINE=true` and
`PHOTARA_APP_RUST_TARGET=/private/tmp/photara-cxt4d-authfix-rust` (also set
`PHOTARA_BRIDGE_RUST_TARGET` to that path for bridge verification). The desktop
profile supplied through `PHOTARA_MACOS_PROVISIONING_PROFILE` is
`~/Library/Developer/Xcode/UserData/Provisioning Profiles/55f15633-1f63-4ba1-85f5-a924a6e5f33e.provisionprofile`;
it expires **2026-09-20 21:34:35 UTC**. No profile or private key is added to source.
Sandbox restrictions required Security/Swift-cache/UI execution approval; the
sandbox-only signature check could not evaluate host certificate trust, while
the approved final strict verification passed. One
production UI compile detected an in-flight source edit and was rerun after
source changes finished; neither event is counted as a passing verification.

This audit performs no Google login, service launch, Neon write, Auth0 setting
change, commit, push or publication. The actual local journal remains untouched.
The next real-provider acceptance must complete Google interaction, verify the
600-second access-token policy and enrollment proof against the checked service,
and audit the resulting Account/Identity/Device/Library aggregate. A successful
synthetic `cloudReady` result or signed build is not evidence of that acceptance.

## Verified signing boundary (2026-09-13)

Apple recommends `SecItem` with `kSecUseDataProtectionKeychain=true` on macOS.
Its access groups are enforced through signing entitlements authorized by a
provisioning profile. A command-line operator also requires an app-like bundle
for that profile; its process must run in a logged-in user context. See
[Apple TN3137](https://developer.apple.com/documentation/technotes/tn3137-on-mac-keychains)
and [access-group authorization](https://developer.apple.com/documentation/security/sharing-access-to-keychain-items-among-a-collection-of-apps).

Apple Development identity `46625CCDBA2A30BE7AFC200274F99FD1CC5BA92A` now has a
complete Apple WWDR G3 chain. Xcode produced distinct managed profiles for
`com.photara.desktop` and `com.photara.operator.development`; the exact signed
groups are `524GTA93Q3.com.photara.desktop` and
`524GTA93Q3.com.photara.operator.development`. `prepare_macos_signing.py` decodes
the supplied profile, rejects expiry or unauthorized identifiers/groups, writes
only exact private entitlements, and selects the profile certificate by SHA-1.
The app build obtains its bundle ID from generated product configuration, keeping
the future rename seam intact. Ad-hoc signing remains only the local-only build
mode and cannot silently stand in for cloud authority.

The real Photara app, signed operator, and one-use signed migration receiver pass
strict codesign/profile/entitlement validation. Both groups independently passed
Data Protection Keychain add/read/update/delete probes. A desktop-signed process
querying the operator group receives **-34018 (`errSecMissingEntitlement`)**.
Personal Team profiles expire after seven days (currently 2026-09-20); the helper
fails closed and Xcode must renew them for continued development builds.

The existing runtime bundle was transferred once, in memory, from the legacy
recovery helper into the operator's `WhenUnlockedThisDeviceOnly`, non-synchronizing
Data Protection Keychain item. The receiver admits only the exact three Neon
runtime roles plus the stable cursor key, validates their shape, refuses overwrite/
conflict, performs readback, and never prints a value. The installed operator then
passed `check` in a fresh process. It launched the checked local service twice;
both `/health/ready` and `/health/live` returned HTTP 200 true responses against
real Auth0 trust and Neon TLS. The service is stopped. The legacy item/helper remain
recovery-only and were not deleted or modified.

## Implemented source

- `scripts/photara-development-service.swift` removes `SecAccess`,
  `SecTrustedApplication`, `SecKeychainSetUserInteractionAllowed`, and deprecated
  authentication-UI flags. It selects the Data Protection Keychain, requires the
  explicit signed operator group, uses device-only unlocked accessibility, and
  uses `LAContext.interactionNotAllowed`. The ad-hoc executable returns only
  `development-service:signing-required` before reading secret input or items.
- `NativeAuthentication.swift` provides random 256-bit PKCE/state/nonce attempts,
  a five-minute one-use callback gate, exact callback coordinates, duplicate-key
  and state rejection, and generation checks. Its system browser wrapper uses
  the current custom-scheme callback API and rejects late completions from
  cancelled sessions. Credential namespaces bind environment, client, exact
  issuer/subject and installation; only rotating refresh/device credentials can
  be stored, with atomic SecItem replacement and no plaintext fallback.
- `NativeIDToken.swift` verifies pinned-key RS256 signatures and exact issuer,
  audience, authorized party, nonce, subject, time, fresh enrollment auth time,
  and supplied access-token hash. A structural JSON pass rejects duplicate
  fields before decoding; JOSE trust URL/key extensions are rejected. This is a
  pure verifier consumed by the native Auth0 client below.
- `NativeAuth0Client.swift` pins discovery issuer, authorization, token and JWKS
  endpoints to the checked HTTPS issuer; imports only bounded RS256 RSA keys;
  rejects duplicate key IDs and weak/unsupported keys; caches keys for at most
  one hour; and shares a single refresh with a 30-second unknown-key limit.
  Expired/unavailable trust fails closed. Authorization codes are exchanged once
  with the exact native client, redirect and verifier; ambiguous results cannot
  reuse the attempt. Strict token responses and rotating refresh adapters are
  covered with synthetic HTTP. No provider-supplied URL selects a request origin.
- `NativeRefresh.swift` serializes rotating refresh across processes using a
  private installation lock. It persists an in-flight marker before exchange,
  atomically stores the replacement before returning access credentials, and
  refuses to replay the old refresh token after an ambiguous exchange or failed
  replacement. Tests use in-memory storage and an isolated empty lock file.
- `NativeOnboardingState.swift` provides a generation-aware state reducer and
  bounded, cookie/cache-free, redirect-refusing HTTP transport. Only the checked
  development origin `http://127.0.0.1:8080/` permits HTTP. A submitted operation
  survives cancellation as outcome-unknown; a late receipt cannot select a
  Library or switch accounts. Only a caller-confirmed local transaction can
  enter cloud-ready. The reducer does not itself implement persistence.

The shared source manifest includes these foundation files in both native hosts.
`OpeningCloudModel.swift` adds an injectable presentation driver with tested
progress/cancel/retry/late-result behavior. `OpeningLibraryView` uses that model:
an unsigned build visibly disables Google sign-in and explains that signing is
required, while the Apple-signed development build injects
`ProductionOpeningCloudDriver`. Local-only startup inspects public signing
metadata only, without querying Keychain or contacting a provider. The system
sidebar/toolbar presentation remains native.

The production driver validates the exact public capability document, asks Rust
for both canonical challenge and bootstrap bytes, requests a five-minute service
challenge, completes the system-browser Auth0/Google PKCE exchange, stores only
the rotating refresh token and random device credential in its signed Data
Protection Keychain group, commits unknown outcome before bootstrap dispatch,
records the canonical receipt independently, fetches current session authority,
and lets Rust apply the same-Library transaction. SQLite work runs behind a
non-main actor. Cancellation before dispatch rolls back only the prepared journal
and newly inserted credential; cancellation or failure after dispatch preserves
the unknown operation. No access/ID token is persisted. The development channel
can start only the fixed, separately signed operator and its embedded service;
other channels never use that host-local path.

## Verification

`zsh scripts/test-native-authentication.sh` compiles Swift 6 with
warnings-as-errors and runs **135 synthetic assertions**: 84 security/refresh,
40 pinned Auth0 networking, and 11 opening presentation checks. Coverage
includes wrong/duplicate callbacks, expiry/cancellation/generation races,
duplicate JSON claims, signed-token claim substitution, forbidden JOSE headers,
wrong keys/hash, preserved unknown outcomes, account isolation, reconciliation
failure, exact origins, concurrent refresh locking and injected refresh failures
before the marker, during exchange, and during atomic replacement. RSA keys exist
in memory only; no real JWT, Google identity, Keychain secret or provider call is
used. The test requires access to macOS Security services for ephemeral RSA keys.
Network fixtures verify discovery endpoint substitution, JWKS single-flight and
unknown-key throttling, rotation, expired-cache outage, duplicate/weak keys,
redirect status refusal, correct PKCE exchange bytes, one-use/ambiguous codes,
malformed token responses and rotating refresh. Presentation fixtures prove zero
driver calls while unavailable, progress, cancellation before/after dispatch,
late-completion rejection, reconciliation failure and successful fixture state.

The operator source compiles with warnings-as-errors and the unsigned executable
fails closed as expected. The synthetic Data Protection probe above confirms why
real Keychain persistence cannot be claimed on this installation yet.
The production app and Shell Lab both build successfully with the shared sources;
neither application was launched for browser authentication. The final production
build used `PHOTARA_APP_RUST_TARGET=/private/tmp/photara-cxt4d.rq4OEo/production-rust-target`
and `CARGO_NET_OFFLINE=true` after the existing Dropbox Cargo cache refused a
hard-link/copy operation; the fresh target build passed without cache deletion.
Isolated native Light/Dark opening-window screenshots were visually checked:
the disabled action and explanatory text fit in the native sidebar, and local
Project actions remain present. These are synthetic window fixtures, not live
cloud or full keyboard/accessibility acceptance. The temporary captures are
`/private/tmp/photara-cxt4d.rq4OEo/opening-cloud-light.png` and
`/private/tmp/photara-cxt4d.rq4OEo/opening-cloud-dark.png`.

## Durable local boundary (2026-09-13)

Additive SQLite migration `0013_onboarding.sql` advances the local reader/writer
floor from 2 to 3; migrations 0001–0012 are unchanged. Its six STRICT tables retain
the session generation, immutable intent, opaque credential reference/commitment,
canonical receipt, cloud binding/current-access observation and explicit selection.
No access/refresh/ID token, PKCE verifier or device credential enters SQLite.
Unsubmitted attempts may be retained as cancelled; unknown submitted operations
cannot be deleted or regenerated. Binding is exact issuer/subject/environment/device.

Rust creates canonical bootstrap commands from stored local identities/names,
commits unknown outcome before dispatch, checks the empty-Library preflight again,
and strictly verifies canonical DTO/envelope bytes and hashes before independently
persisting receipts. Late receipts remain recoverable under the original principal;
only the current durable session generation can reconcile. Current active session
observations must be fresh and cannot regress cached revisions/time.

Same-ID application uses one reserved SQLite writer transaction, repeats local
content/authority/revision checks, then switches the contract and stores the binding
and selection. Conflict preserves local data and the cloud receipt for explicit
reconciliation. Startup retains the original LibraryId instead of creating another
default; its local principal is historical evidence, not current cloud authority.
An account with a different default returns a choice first. The separately named
`open_existing_cloud_library` operation opens an empty cloud cache under that ID,
preserving the independent original local Library without rekeying/deletion.

`PhotaraLocalOnboarding` exposes typed UniFFI commands/results and public canonical
challenge/bootstrap bytes; Rust retains SQL, clock, journaling and transaction
ownership. Native calls run off the UI thread. The generated Swift binding and
production opening driver compile together in the signed app.

The exact current session DTO is stored in the new binding as account/membership
observation. The checked service does not return the account revision/display name
required by the older `account_cache`; these values are not invented. No old
account/membership cache, sync target, cursor or content outbox is activated.
Nonzero bootstrap high-water is rejected; existing-cloud labels are explicitly
provisional (`Cloud Library`) until the remote name/content snapshot contract is
implemented. This is enrollment identity reconciliation, not multi-Mac content
hydration or online write authorization.

Verification: **44 photara-library tests and 8 photara-bridge tests pass** (including
10 new enrollment tests and one typed bridge test). Fixtures prove additive v12
upgrade preserves IDs/content/ledger checksums, malformed/cross-account receipts
fail closed, first-dispatch CAS, durable cancellation/unknown recovery, injected
binding-insert failure rolls back authority but retains receipt, concurrent pools
converge, restart retains same IDs, and the existing-cloud choice is explicit.
All fixtures use isolated temporary databases and synthetic DTOs; the production
app was rebuilt, not launched against the user's database. The historical D19
schema checker remains frozen at its reviewed 0012 inventory; its 46 signatures
and 104 foreign keys pass, as does the generation-two naming guard.
Focused library/bridge Clippy with `--all-targets -- -D warnings`, Rust formatting
and `git diff --check` also pass. The atomic reconciliation function retains one
documented line-count lint exception; UniFFI methods retain owned ABI parameters.

## Remaining acceptance before CXT4d completion

1. Run the real Google sign-in through the signed Photara window and audit the
   resulting Account/Identity/Device/Library aggregate in Neon. Do not count the
   compiled path or synthetic cloud-ready state as live reconciliation.
2. Exercise restart/idempotent retry, local edits during enrollment, unknown-result
   replay, logout/resume/account-switch/offline states, and native accessibility.
   The first-enrollment path is wired; returning-session UI remains acceptance-led.
3. CXT4e retains the full first-Account audit and second-Mac hydration proof. No
   live first-Account creation occurred in this checkpoint.

The user's intended acceptance workflow is a bounded Fly deployment and first
end-to-end onboarding proof, followed by stopping/scaling-to-zero/deleting the
unneeded Fly runtime after verifying its cost and retained-state implications.
Ongoing personal development remains desktop → Auth0 browser → local Rust
service → shared Neon from each configured Mac. Fly is temporary hosted compute,
not a substitute for the service boundary. Legitimate modern Keychain signing is
now established on this Mac; it remains a required host setup rather than a
Fly-only concern. Cloud UI unavailability is temporary, not a product decision.

The installed legacy operator executable and its file-based Keychain item remain
**recovery only**. The helper was executed once to inject its existing values into
the signed one-use receiver; the item itself was not overwritten, exported to disk,
or deleted. The checked modern runtime source never queries it. No Neon roles/schema/
data, Auth0 settings, Fly resources, graph/node code, staged files, commits or pushes
changed during signing and migration.
