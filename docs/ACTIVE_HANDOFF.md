# Active handoff

## 2026-09-14 committed checkpoint and proposed UI0

The user confirms the live first-Mac Google/cloud flow works and supplied Light/Dark
opening captures. CXT4b–d, recovery, account controls and their test furnaces were
committed and pushed to `origin/main` as `45ddedf`. CXT4e second-Mac hydration and a
complete independently captured aggregate audit remain open.

The next proposed work is documented in
[UI ladder and authoring sequence](architecture/UI_LADDER_AND_AUTHORING_SEQUENCE.md).
Preserve Graph Lab. UI0 freezes a reusable three-step neutral ladder, rebuilds
Theme/Shell Lab ownership, removes the Opening content tint, renders real Light/Dark
production states and stops for approval. UI1 then designs and wires the first real
Create Project operation across Library catalog, initial Graph, package and cloud
projection. Do not begin UI1 implementation before its mockups are approved.

Updated 2026-09-13 for the account sidebar aesthetic cleanup.

The user confirmed live sign-in/sign-out and library selection work. Their next
request was visual: move Accounts to the bottom-left library sidebar, shrink the
avatar, and show a person icon with **Sign in** when signed out. The account entry
now sits under the library status in the sidebar; the project editor uses the
same lower-left control. Clicking the compact avatar/name opens the existing
account menu. Settings is accessible there, replacing the empty top-right gear.
The top-right account control is removed. There is no signed-in banner.

The photo is rendered into a circular **20-point** NSImage as well as constrained
in SwiftUI. This fixes native Menu label image extraction bypassing the former
view-only frame/clip and displaying the larger cached photo. A **512×320 raster**
fixture now exercises that path in the production shell, including Light/Dark,
first launch, signed out, returning sessions, and 760×560 / 1280×820 windows.

Signed build + production UI verification passed (exit 0), with visual inspection
of the account/footer and project views. Log: `/private/tmp/photara-sidebar-accounts-ui.log`.
Current UI evidence: `platform/macos/photara-app/.build/cxt4d-gates/sidebar-ui-passed.json`.
Only **4 UI/fixture files** differ from the previous full gate; its **101 other
source hashes** still match. Authentication, service and storage code is unchanged.
The full gate below is historical evidence for that unchanged functionality; it
was not rerun for this presentation-only change.

Current signed app: `/Users/suhail/.codex/worktrees/84ad/photara/platform/macos/photara-app/.build/app/Photara.app`.
Executable SHA-256: `a87e52ccbb977d31d90706ba579a9fed075ddeb7f0c4d9392c07271db2372661`.
No live login/logout, credential change, Neon write, service replacement, commit
or push occurred. Quit/reopen the app to see the layout change.

## Earlier Accounts and library-selection checkpoint

Updated 2026-09-13 for Accounts, explicit logout/rejoin and library selection.

The user confirmed that the previous fix completed live Google sign-in and
created a Neon record, and supplied the cloud-library opening screenshot. That
is user-reported live success; this task has not independently queried the new
production account or receipt. The earlier blocked live-attempt notes below are
historical, not the current status.

The new Accounts menu exposes profile name/email, a circular Google avatar when
available, and Sign Out. A deliberate sign-in now presents **Choose a library**,
including when only **My Library** is available. Normal launches restore the
selected session quietly without a prompt or signed-in label. The current service
contract exposes one owned default library; additional created/invited libraries,
billing and account settings remain future work, without placeholder actions.

Logout durably marks the session signed out, then clears the refresh token under
the existing attempt/refresh locks. The device secret and library/receipt evidence
remain intact. Rejoin verifies fresh Google issuer/subject, reuses the exact device
and original enrollment, reads current `/v1/session`, and waits for selection.
Access is fetched again after the selection before Rust applies it. No bootstrap,
new device or account/library creation occurs on rejoin. Wrong-account and cancelled
selection remain signed out. A failed Keychain cleanup cannot restore local session
authority and is retried before the next deliberate authorization.

Name/email/picture come only from a verified ID token and are presentation data.
The local display cache is scoped to environment and local library. Startup reads
that cache without Keychain/network access. Google avatar downloads happen after
successful explicit sign-in, allow only HTTPS googleusercontent.com subdomains,
refuse redirects/cookies/credentials, bound size/time and store a small raster
thumbnail. The existing signed-in user will get the new profile cache on the next
deliberate login. Missing profile/photo uses the circular person icon.

The complete Accounts gate run exited **0**:
`/private/tmp/photara-cxt4d-accounts-gates.log`. Its passing manifest records
**105** unchanged source hashes, independently rechecked with the executable.
Evidence: **94 + 58 + 64** native security/Auth0/presentation assertions,
**124** production-driver boundary assertions, **253** native service assertions
across **17** real TCP/Rust/PostgreSQL scenarios including account restart stages,
all **11** disposable PostgreSQL tests, **99** process assertions across **13**
scenarios, **10** installed-service checks with unchanged port ownership, **8**
signed Keychain checks, and **62** ordinary Rust tests. Bridge/product/schema/naming,
formatting, Clippy with warnings denied, shared UI, production UI and strict signing
checks passed. Light/Dark library-picker and account-control captures were visually
inspected. The shared session survives opening/project-view transitions.

Signed app: `/Users/suhail/.codex/worktrees/84ad/photara/platform/macos/photara-app/.build/app/Photara.app`.
Executable SHA-256: `b6a04b3637e0347635782a95fb614de5e645585193aa13f4c187e7f3f2a890eb`.
The user should quit/reopen this build for their manual Accounts → Sign Out →
Sign in again → My Library check. No live sign-in/logout, Neon write, service
replacement, commit or push was performed by this feature task. Existing dirty
work is preserved.

## Earlier terminal-expiry and passive-launch checkpoint

Updated 2026-09-13 for deliberate sign-in, passive launch and terminal expiry recovery.

Opening now reads only local Rust journal/binding metadata. It does not read
Keychain, start/refresh authentication, launch the service or open a browser.
The first local opening offers **Sign in with Google**. A returning cached cloud
connection opens quietly, with neither a sign-in prompt nor a **Signed in** label.
The prompt is hidden until the local read finishes to avoid a launch-time flash.
Saved signed-out/access-disabled bindings remain distinct; cached presentation
never establishes current online authority. This follows the user's latest
Notion-style preference and supersedes the earlier automatic recovery behavior.

An unfinished attempt presents **Sign in again**. That explicit click first
reconciles the exact retained operation. When authenticated operation lookup,
a proofless exact replay, and a final authenticated lookup confirm no receipt
and the original enrollment proof is unavailable/expired, Rust appends the
terminal disposition **abandoned-expired**. An additive local migration 0014
retains the original command, principal, operation, credential reference and any
later receipt. No deployed migration checksum is rewritten. Local reader/writer
floor 4 keeps old readers from misinterpreting this state.

The deliberate click may then perform exactly one new browser authorization.
The replacement has a new operation/proof and explicit lineage, and reuses the
same Library, Device and credential commitment. The same principal is mandatory.
The native attempt lock and Rust transaction guards serialize competing clicks.
Cancelling or crashing before dispatch keeps the shared original credential.
A late old receipt remains evidence and cannot change the active library selection.
Real Rust/PostgreSQL tests prove one Account/Identity/Device/default My Library
and membership even if the old operation commits before or after the replacement.
Two valid operation receipts in that race are retained, not treated as duplicate
accounts or libraries.

**Neon has tables:** prior deployed-database verification established 14 successful
service migrations and resolved the onboarding relations. Zero account/library
rows did not mean missing tables. The confirmed earlier defect was the service
rejecting Auth0's API plus issuer `/userinfo` audience pair. The corrected strict
policy and capability marker remain installed. See the historical evidence in
[CXT4d checkpoint](architecture/CXT4D_NATIVE_AUTHENTICATION_CHECKPOINT.md).

The live retained operation remains `35d65e1f-48bb-4fb0-ab06-b4ced18cd4f1`.
This task has not run another live Google sign-in, changed its live journal,
written directly to Neon, or claimed successful live cloud enrollment. The user
can choose the explicit sign-in action in the new signed build. All original
live evidence is preserved; the earlier prohibition on initiating another live
sign-in was respected throughout implementation and verification.

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

Existing dirty CXT4b/c/d, graph and lab changes are preserved. No commit, push or
publication occurred.

## Earlier CXT4b-dev runtime checkpoint

Updated 2026-09-13. **CXT4b-dev runtime provisioning and live readiness are
complete on this development Mac.** The three Neon `main/neondb` logins
`photara_dev_api`, `photara_dev_control`, and `photara_dev_auth_read` each inherit
exactly their corresponding existing capability role. Live authentication/audits
prove no owner, Neon-elevated or cross-capability membership, privileged role
flags, object ownership, database CREATE permission or admin-option grants;
each login has a six-connection ceiling.

One non-sync macOS Keychain item stores the three certificate-verified URLs and
a stable random 32-byte cursor key. A native operator-only launcher retrieves the
bundle, strips inherited configuration and directly executes the development
service. Both `/health/ready` and `/health/live` returned HTTP 200 at
2026-09-13 19:36:54 UTC; restart readiness passed at 19:38:18 UTC. Postflight
Accounts/Libraries/defaults/receipts remain `0/0/0/0`; schema epoch/floor is `1/3`
with 14 successful migrations. The proof service is stopped, not installed as a
background daemon. No sign-in, seed, Auth0 change or Fly resource was created.

**Next bounded gate: CXT4d-dev native integration.** Consume the checked identity
descriptor for browser PKCE, user-token Keychain storage, loopback HTTP and local
Library reconciliation. CXT4e still owns first real sign-in and second-Mac
acceptance; secure provisioning of that second host remains outstanding. Read
[runtime checkpoint](architecture/CXT4B_SERVICE_CHECKPOINT.md#cxt4b-dev-runtime-provisioning-and-live-readiness)
and [exact host runbook](../crates/photara-service/deploy/README.md#host-local-keychain-launcher).

## Earlier CXT4b-dev configuration checkpoint

Updated 2026-09-13. **CXT4b-dev configuration implementation is verified; live
loopback readiness remains gated on host-only runtime credential provisioning.**
The checked `config/product-identity.json` feeds typed Rust and generated Swift,
the application title/support-directory seam and bundle/callback metadata.
Explicit development binds `127.0.0.1:8080` through the original service verifier,
pool/role/schema checks and router. No native sign-in or content worker is active.
Hosted channels remain unconfigured and fail closed. Read the
[exact files, evidence, limits and next gate](architecture/CXT4B_SERVICE_CHECKPOINT.md#cxt4b-dev-configuration-checkpoint)
and [operator profile](../crates/photara-service/deploy/README.md).

Next: separately provision three least-privilege runtime login credentials and a
stable cursor key into approved host-local secure operator storage for Neon `main`,
then prove loopback `/health/ready` without signing in or seeding user data. CXT4d
native PKCE/Keychain/bootstrap and CXT4e multi-Mac acceptance remain later gates.
The source/config seam is complete; real multi-Mac cloud use is not yet active.

## Earlier development-policy decision

Updated 2026-09-13. **The development-cloud and replaceable-product-identity
policy is approved; CXT4b-dev was selected next.** Suhail needs cloud Libraries to follow
him across development Macs, but does not want idle Fly compute before production.
Use the identical Rust/Axum contract at loopback on each explicitly configured
development Mac, with host-only secrets and Neon `main` as cloud source of truth.
Fly.io is retained for brief remote-acceptance deployments and permanent production;
destroy test Machines after evidence. Add one typed environment/product-identity
seam before native integration so `Photara`, bundle/callback/package/API coordinates
have one coordinated pre-release cutover. Read
[the authoritative policy](architecture/CXT4_DEVELOPMENT_CLOUD_AND_PRODUCT_IDENTITY.md).

Updated 2026-09-13. **Additive CXT4b migration 0014 is installed on Neon
`photara` / `main`; no user data was seeded.** Read-only preflight proved
`photara.service.g2`, epoch 1, minimum API 2, 13 ledger entries, zero Accounts,
and zero Libraries. The operator-only `photara-migrate` binary then installed
0014. Postflight proves minimum API 3, 14 successful ledger entries, and checksum
`adeadf906a1cd1e97fbc3b31d2dafe3f54435a10224c47ba8254cfa521f7d2ebf97f30a10c6c45e6116be5aad0f91dec`.
`account_defaults`, `device_credentials`, `onboarding_receipts`, and
`onboarding_challenges` resolve in their expected schemas. Accounts, Libraries,
defaults, and receipts are all still zero. `legacy-v0.1.x` was not opened or
changed.

Updated 2026-09-13. **The dedicated Auth0 development client/API boundary is
configured; no user has signed in.** The exact public tuple is:

- issuer `https://dev-nmturasdrkz7up27.us.auth0.com/`;
- API audience `urn:photara:api:development` (API id
  `6aa6e96c3595ecc7d9ad79f1`);
- Native public client `Photara macOS`, client id
  `CJ3hH2CUSkEk0vSbdvHMJLEqD4gYWGkW`;
- callback/logout return
  `com.photara.desktop://dev-nmturasdrkz7up27.us.auth0.com/macos/com.photara.desktop/callback`.

The Native client uses Authorization Code and Refresh Token grants, no implicit
grant, seven-day idle/thirty-day absolute rotating refresh tokens, and Google as
its only enabled connection. The API uses the Auth0 JWT profile, RS256,
ten-minute access tokens, offline access, per-app user authorization, no client
access, and the sole `photara:onboard` permission; the Native client has 1/1 of
that permission. Username/password was disabled for this client only. Existing
Chordrift clients/connections were not edited. No Auth0 user, Account, Device,
Library, credential, Neon row, or product sign-in was created.

Updated 2026-09-13. **CXT4b now has a selected Fly.io production/acceptance adapter
and verified runnable artifacts; billing is configured and live creation is
deliberately deferred.** Added
separate `photara-service` and operator-only `photara-migrate` binaries, a
non-root multi-stage Docker image, `.dockerignore`, and a valid Fly manifest with
managed HTTPS/readiness/concurrency policy. Release compilation, service tests,
strict Clippy, formatting and `fly config validate` pass. Fly CLI 0.4.102 is
installed and authenticated as the user's account. `fly apps create photara-api`
created nothing during the earlier billing prerequisite. Billing has since been
added, but no retry is authorized under the approved development profile. Auth0's
development resources and Neon migration 0014 have since been configured as
recorded above. No Fly app, runtime credential, user-data row or charge was
created. Do not create persistent Fly resources during CXT4b-dev; use a separately
authorized, short-lived remote-acceptance deployment and destroy its Machines.

Updated 2026-09-13. **The user accepted the CXT4c native opening-shell first
visual baseline and explicitly made minor sizing refinement non-gating.** Shared
production/Shell Lab source now uses `NavigationSplitView`, a native sidebar
`List`, unified toolbar, system accent/selection/material, a plain centered
`Photara` title, and leading-aligned local/cloud status. The opening UI remains
presentation-only: its Google action does not contact Auth0, the service, or Neon.
Both Shell Lab and the production app build successfully. Continue with CXT4b-dev
typed identity/environment and loopback-service wiring, then CXT4d; do not mistake
the rendered fake state for
working authentication.

Updated 2026-09-13. **CXT4b has a verified host-independent service checkpoint;
it is not deployed and its gate is not complete.** Started clean at approved
`14f77ee`; nothing is staged, committed or pushed. Read the
[exact implementation, inventory, tests and remaining inputs](architecture/CXT4B_SERVICE_CHECKPOINT.md).

Additive migration 0014/service floor 3, pinned RS256/JWKS verification, typed
HTTP onboarding/session routes, atomic bootstrap/replay, device logout/resume and
the password-disabled three-login operator template are implemented locally.
Ten disposable PostgreSQL suites pass; all prior migration checksums remain exact.
The live Neon floor is now 3 with 14 migrations and no user data created here.

**Deferred remote-deployment inputs:** the Fly app/HTTPS origin and encrypted runtime
secret injection; operator/deployment identity; confirmed release
signing access group and supported macOS range. The Auth0 development tuple is
now concrete above and no value was inferred from Chordrift. Host-specific
packaging, encrypted credential injection, provider settings, backup restoration
and a controlled live smoke test remain unperformed. Do not seed around onboarding.

Full offline Rust, strict Clippy/all-target checks, schema/naming/fixture guards,
native bridge and production UI pass. Serial Graph rerun passes 15,562 assertions;
the preceding overlapping native run's focus/menu failures remain documented.
All platform/Graph/local/package/fixture sources outside the bounded opening-shell
files are preserved. Real local SQLite was never opened.

## Historical CXT4a approval handoff

Updated 2026-09-13. **CXT4a onboarding/security contract and Chordrift reference
audit are approved and complete.** Read the
[approved implementation contract](architecture/CXT4A_ONBOARDING_SECURITY_CONTRACT.md).
This documentation-only slice starts from clean `main` at `c90274b` and remains
uncommitted/unpushed. The contract does not mark authentication or deployment done.

It specifies native PKCE, exact token verification, Keychain/refresh/logout,
bootstrap DTOs and transactions, additive persistence prerequisites, same-ID local
Library reconciliation, returning-user/collision handling, threat model and tests.
The user approved Google as the only initial Auth0 connection, the
`Sign in with Google` action, device credential, token policy, additive migration/
floor approach and empty-Library enrollment limit. Future providers such as Yahoo
remain additive and require explicit identity linking rather than email matching.
Exact Auth0 coordinates, bundle/signing inputs and host/secret store are concrete
CXT4b inputs, not unresolved CXT4a architecture.
Existing CXT3b startup only accepts local-only authority; CXT3c has no durable
Account default pointer or pre-Account bootstrap receipt. Those require bounded
CXT4b/d changes, not manual seeding or edits to deployed migration checksums.

**Historical gate:** separately select CXT4b and its concrete deployment inputs. The already-approved
[CXT4c native opening-shell boundary](architecture/CXT4_ONBOARDING_AND_OPENING.md#cxt4c--native-opening-library-shell)
is preserved intact and remains before real native Auth0 integration. No production
Rust/Swift/service code, provider state, credentials or database was changed.

**Verification before approval:** naming guard passes 362 files; schema guard passes 46 signatures
and 104 scoped FKs; fixture guard passes 12 canonical containers and 92 embedded
records; `git diff --check` passes. See the contract's exact six-file inventory
and preservation/verification limits. Runtime acceptance tests remain future work.

## Historical CXT3d handoff

Updated 2026-09-13. **CXT3d Neon Generation Two schema activation is complete.**
CXT3c was committed and pushed as `be1ed80`; CXT3d was committed and pushed as
`699395f`. Read the
[live topology, inventory, and onboarding boundary](architecture/CXT3D_NEON_ACTIVATION.md).

The existing Neon `photara` project was retained. Its empty primary/default
`production` branch is now `main`; populated `development` is now
`legacy-v0.1.x`. The latter remains unchanged with 34 public tables and its
20-entry legacy ledger. On `main`, four non-login capability roles and the exact
13 checked migrations are installed. The live inventory is 55 domain tables,
569 columns, 153 indexes, 115 triggers, 48 functions and 173 policies; all 45
RLS tables force RLS. All migration checksums match CXT3c, and a second migration
run accepted the deployed ledger. Accounts, Libraries, and Projects are all empty.

**Historical next step, now prepared above:** CXT4a, the Astra-led onboarding/security
contract and Chordrift Auth0 reference audit. Follow the exact bounded sequence in
[CXT4 onboarding and opening Library shell](architecture/CXT4_ONBOARDING_AND_OPENING.md):
CXT4a contract → CXT4b minimal cloud service → CXT4c native opening Library shell
→ CXT4d native Auth0 integration → CXT4e first-Account acceptance. Stop before
each gate. Do not manually seed around the product experience. No runtime login,
password, or service secret exists, and no privileged Neon credential may enter
the desktop app.

## Historical CXT3c handoff

Updated 2026-09-13. **CXT3c disposable PostgreSQL/service proof is complete.**
Started from clean synchronized `7e6d794`; committed and pushed as `be1ed80`.
Read [service routes, limits and the next Neon plan](architecture/CXT3C_SERVICE_RUNTIME.md),
[implementation and regression inventory](architecture/CXT3C_IMPLEMENTATION_INVENTORY.md)
and [measured PostgreSQL objects/checksums](architecture/CXT3C_POSTGRES_INVENTORY.json).

The new `photara-service` crate owns typed service controllers above Storexa,
13 executable migrations, separated unprivileged pools and fake identity/media/
sync ports. PostgreSQL 18.6 executed 697 migration statements: 55 domain tables,
569 columns, 153 indexes, 115 triggers, 48 functions and 173 RLS policies.
Five real PostgreSQL suites pass, including 678 access/privilege/sensitivity cells,
CAS/concurrency, last-manager and last-identity guards, rollback, checksums/floors,
pooled GUC cleanup, invitations, scoped receipts/media and lost-response recovery.
Every disposable PostgreSQL cluster is stopped; no service was deployed.

The full offline Rust suite passes 258 tests. Its nine ignored tests comprise
four existing fixture-generation checks plus the five PostgreSQL suites that the
explicit disposable runner executes successfully. All-target check, strict Clippy,
fmt, schema/naming/fixture guards, bridge and production UI pass. Graph Lab's
initial randomized prerequisite failure (15,050 assertions, one failure) is retained;
exact seed replay passed 548 assertions, then the full rerun passed 15,586/zero.
All 42 Graph/Graph Lab source files and all UI/Core/SDK/package/local runtime
sources remain byte-for-byte unchanged. The real CXT3b database still matches its
starting SHA-256 and size, 1,179,648 bytes; it was not opened or migrated here.

The authorized slice exercises online transport with an in-memory fake. It does
not activate the native SQLite online worker. Its closed content codecs expose
Library storage and per-Project catalog projections, not a complete backup of
all installed tables. Real identity/media adapters, native online activation and
package publication are not fabricated by this proof.

The subsequent CXT3d activation is recorded above. CXT3c itself did not contact
Neon or activate real Auth0/CloudKit/media adapters.

## Historical CXT3b handoff

Updated 2026-09-12. **CXT3b local runtime and app initialization are complete.**
Started from clean `415039e`; this slice is uncommitted and unpushed. Stop here.
Read [runtime/API boundaries and inspection commands](architecture/CXT3B_LOCAL_RUNTIME.md)
and [exact file/migration inventory](architecture/CXT3B_LOCAL_INVENTORY.md).

The real app startup entry point created/opened:
`/Users/suhail/Library/Application Support/Photara/State/photara-local-v2.sqlite`.
It contains one active `My Library`, explicit local controller, stable device ID,
12 migrations, reader/writer floor 2 and 70 domain tables. Read-only integrity
returns `ok`; foreign-key checks return no violations. No unknown/preexisting
user database was adopted or deleted.

CXT3b includes explicit local membership, restricted Project registration/access,
policy/grant CAS with immutable control audit, atomic last-manager/controller
transfer, classified storage/slots/host binding selection, checked Library
variables/expressions, device observations, context apply/recovery, Project-scoped
media and offline scoped variable intents. Project variables remain package-owned.
Online channel activation/sealing/dispatch/service snapshot installation stay in
CXT3c; package publication and publication receipts stay in L3. No PostgreSQL,
Neon, Auth0, CloudKit, v0.1.3 database or SMB/user source storage was written.

**Verification:** 258 offline Rust tests pass, four intentionally ignored;
all-target compilation, Clippy with warnings denied, fmt and whitespace pass.
Schema: 70 tables / 68 explicit indexes / 169 triggers / 310 statements. All six
baseline migration bodies, all 12 fixtures and all 42 Graph/Graph Lab files are
byte-for-byte preserved. Static checks cover 46 proposal signatures and 104 FKs;
fixture verification covers 12 canonical containers and 92 embedded records.
Bridge passes (revision 19; 3 progress / 2 cancellation callbacks). Shared UI
passes with 98 snapshots, production UI passes with 10 snapshots and explicit
startup assertions. Representative Light/Dark production images were inspected.

Dedicated Graph Lab: the first 15,430-assertion run had one randomized prerequisite
routing-knot failure at seed 20260909 dark/curved. Isolated replay passed 541
assertions; the full rerun passed **15,586 assertions / zero failures**. Graph source
was not edited to obtain the pass. Logs: `/private/tmp/photara-cxt3b-graph.log`,
`/private/tmp/photara-cxt3b-graph-replay.log`,
`/private/tmp/photara-cxt3b-graph-full-retry.log`.

The next gate is separately authorized CXT3c disposable PostgreSQL/service work.
No service deployment, fresh Neon environment, package publisher, commit or push
is authorized by this completion. Continue to preserve the Graph implementation.

## Historical CXT3a/rebaseline handoff

Updated 2026-09-12. **CXT3a and the separately authorized Library nomenclature
rebaseline are complete and verified.** Nothing is committed or pushed.

Read the [CXT3a reader/API record](architecture/CXT3A_PACKAGE_READER.md),
[rebaseline authority/evidence](architecture/LIBRARY_NOMENCLATURE_REBASELINE.md)
and [exact file/hash inventory](architecture/LIBRARY_REBASELINE_INVENTORY.md).
The [execution roadmap](ROADMAP_0_2_EXECUTION.md) is the current gate order.

Suhail superseded D19 R1 physical-name preservation: unshipped Generation Two uses
`Library`, `LibraryId`, `libraries`, `library_id` throughout Rust, package, SQLite,
PostgreSQL proposals and service vocabulary. There are no rename migrations,
aliases, shadow columns, dual writes or runtime naming adapters. The old v0.1.3
and live databases are untouched. An optional future historical importer is
non-gating; it has not been implemented.

**Verification:** 245 full offline Rust tests pass; all-target compilation, Clippy,
formatting and whitespace checks pass. Fresh SQLite baseline: 44 tables, 30 explicit
indexes, 95 triggers, 171 statements; integrity and FK checks pass. S3/S4 documentation
totals include examples (179/275); PostgreSQL baseline DDL is 263 statements and was
never executed. D19 static proposals retain 26/20 new tables, 139/431 statements,
46 checked signatures and 104 FKs. All 12 canonical fixture containers and 92 embedded
byte records pass Rust codec/hash verification. Naming checks, Swift/Rust bridge,
98 shared UI snapshots and 10 production snapshots pass; representative Light/Dark
screens were visually checked.

## Authority and exact next task

Library/account/catalog/access/device-binding records belong in local/cloud
databases. Authored Project graphs, nodes, connections, configuration, Work Surface
state, variables and asset/resource ledgers belong in `.photara`, with the designed
catalog/sync projections. Immutable runs/evidence follow package/cloud projection
rules. UI tokens are code/presets. Window geometry, panes, selection, zoom, scroll,
transient progress and unsaved edits are per-device preferences/session state.
The shell now calls that model `EditorSessionModel`; it is not Library data.

**Next separately authorize CXT3b:** use the clean Library baseline and accepted
D19 proposals to implement executable local SQLite repositories/facade and real
Generation Two app initialization. Prove fresh disposable migrations, floor refusal,
rollback/FKs, scoped permission/CAS behavior and local recovery with fake hosts.
Initialize local My Library explicitly; do not route through a legacy importer.
The current app's existing Library/UI regression is not a claim that D19 app
initialization or the new repositories are implemented.

Then CXT3c proves disposable PostgreSQL/RLS and scoped service behavior. Fresh
Generation Two Neon deployment requires its own authorization after those checks.
The minimum usable Project/UI/node vertical slice follows that deployment gate.
No CXT3b implementation, D19 migration execution, PostgreSQL/Neon execution,
service deployment, live database change, staging, commit or push occurred here.

## Historical CXT1b handoff — superseded next-step labels


Updated 2026-09-12. **CXT1b pure Rust context contracts are complete.** Suhail
separately selected this slice after CXT1a. Read the [implementation/API/grammar/
verification record](architecture/CXT1B_CONTEXT_CONTRACTS.md), then the
[accepted freeze](architecture/D19_CONTRACT_FREEZE.md) and
[bounded gates](architecture/D19_STATIC_SCHEMA_DELTA.md#fixture-delta-implementation-slices-and-gates).

The additive `photara_core::context` API implements explicit literal/expression/
template admission, bounded parser and ID-bound typed AST/interpreter, variable
CAS/precedence/cycles, immutable consent-checked captures, bounded metadata queries,
Run overrides, private device/frozen contexts, cache v2 and proposal/receipt planning.
Literal-only fields never become expression records; fenced containers remain
unsupported. Evaluated results retain privacy labels; no existing graph evaluator,
NodeSDK validator, package codec, database/host adapter or effect executor is changed.

**Verification passed offline: 159 tests**, comprising 57 new context integration
tests, three new serialization compile-fail tests and 99 retained CXT1a/Core/SDK/node
tests. New coverage includes a 441-case deterministic arithmetic matrix. The separate
[d19-context.json](fixtures/generation-two/d19-context.json) was generated and checked
by the unchanged Rust canonical encoder; previous fixture bytes remain exact.
Affected formatting, all-target library compilation, Clippy with `-D warnings`,
whitespace, Markdown links and pre-slice hash checks pass. Database suites were not
executed; retained node tests use disposable filesystem fixtures.

CXT1a source/tests/implementation record, six applied L2 migrations, all 14 CXT2
proposal files, ten pre-existing fixture files, Cargo files and unrelated dirty work
are preserved. The hash audit confirms 317 pre-existing files are byte-identical;
232 local Markdown links/anchors pass. Only the additive context module declaration
changes existing Rust.
No database/SQL, resolver, package reader/writer, Swift/UI, user Project/SMB, service,
staging/commit/push or release work occurred.

**Next eligible slice: separately select CXT3a.** Its package 1.1 reader/closure and
explicit compatibility mapping DTOs belong in disposable roots with no publisher.
CXT3b/c retain separate local/fake-host and service/RLS/sync proof gates. L3 remains
paused. Stop at CXT1b; do not automatically continue into those slices.

## CXT1b exact file inventory

New source files:

- `crates/photara-core/src/context/mod.rs`
- `crates/photara-core/src/context/value.rs`
- `crates/photara-core/src/context/expression.rs`
- `crates/photara-core/src/context/parser.rs`
- `crates/photara-core/src/context/interpreter.rs`
- `crates/photara-core/src/context/variable.rs`
- `crates/photara-core/src/context/snapshot.rs`
- `crates/photara-core/src/context/metadata.rs`
- `crates/photara-core/src/context/cache.rs`
- `crates/photara-core/src/context/proposal.rs`
- `crates/photara-core/tests/context.rs`

New documentation/data: `docs/architecture/CXT1B_CONTEXT_CONTRACTS.md` and
`docs/fixtures/generation-two/d19-context.json`.
Existing Rust edit: `crates/photara-core/src/lib.rs`, additive module declaration only.
Updated documentation: `README.md`, `ROADMAP.md`, `docs/ACTIVE_HANDOFF.md`,
`docs/ROADMAP_0_2_EXECUTION.md`, and architecture `README.md`, `CORE.md`, `ASSETS.md`,
`NODE_PACKAGES.md`, `STORAGE_LOCATIONS_AND_HOST_BINDINGS.md`,
`TYPED_CONTEXT_AND_EXPRESSIONS.md`, `GENERATION_TWO_FIXTURES.md`,
`D19_CONTRACT_FREEZE.md`, `D19_STATIC_SCHEMA_DELTA.md`, `SCHEMA_REVIEW.md`.
This slice adds 13 files and updates 15 pre-existing files. Earlier dirty-tree
changes in git status are not part of this inventory.

## Prior CXT1a completion — historical scope

The following records earlier completed slices. Their then-next-step statements
are historical; the CXT1b status and CXT3a gate above govern current work.

Updated 2026-09-12. **CXT1a pure Rust contracts are complete.** Suhail separately
selected this bounded implementation after accepting R1–R8 and CXT2. Read the
[implementation/API/verification record](architecture/CXT1A_CONTRACTS.md), then the
[accepted freeze](architecture/D19_CONTRACT_FREEZE.md) and
[bounded gates](architecture/D19_STATIC_SCHEMA_DELTA.md#fixture-delta-implementation-slices-and-gates).

Additive `photara_core::contracts` and `photara_node_sdk::v2` implement portable
IDs/adapters, Project access, resource rights/coordinates and nonserializable live
handles, immutable AssetSet v2 snapshots/pages/digests, complete manifest v2
validation and minimal response/opaque context coordinates. The one new
[d19-contracts.json](fixtures/generation-two/d19-contracts.json) is generated and
verified with the existing Rust canonical encoder in the reserved synthetic namespace.
No parser, expression evaluation, capture engine, cache v2 implementation,
package 1.1 codec or host/database adapter is included.

**Verification passed offline: 99 tests** (58 new integration, two compile-fail,
39 retained Core/SDK/node tests), plus explicit golden generation and the final
SDK rerun. Affected-crate formatting, whole-library all-target compilation and
Clippy with `-D warnings` pass. Database suites were not executed. The pre-slice
SHA-256 inventory confirms 302 pre-existing files remain byte-identical and preserves existing dirty work, v1 source/API/cache behavior,
Cargo files, all six L2 migrations, all 14 CXT2 proposal files and all nine
pre-existing fixture files. No database/service, user Project/SMB, UI,
staging/commit/push or release action occurred.

**Next eligible slice: separately select CXT1b.** Its bounded parser/AST/types,
dependencies/snapshots/cache/proposals remain unstarted. CXT3a/b/c retain their
separate package/local/service proof gates. L3 remains paused. Stop at CXT1a;
there is no automatic migration, package publication or live-service continuation.

## CXT1a exact file inventory

New Rust files:

- `crates/photara-core/src/contracts/{mod,ids,schema,access,resource,asset_set,dto}.rs`
- `crates/photara-core/tests/contracts.rs`
- `crates/photara-node-sdk/src/v2/{mod,types,validation}.rs`
- `crates/photara-node-sdk/tests/contracts_v2.rs`

Existing Rust changes: only additive module declarations in
`crates/photara-core/src/lib.rs` and `crates/photara-node-sdk/src/lib.rs`.
New documents/data: `docs/architecture/CXT1A_CONTRACTS.md` and
`docs/fixtures/generation-two/d19-contracts.json`.
Updated documentation: `README.md`, `ROADMAP.md`, `docs/ACTIVE_HANDOFF.md`,
`docs/ROADMAP_0_2_EXECUTION.md`, and architecture `README.md`, `CORE.md`, `ASSETS.md`,
`NODE_PACKAGES.md`, `STORAGE_LOCATIONS_AND_HOST_BINDINGS.md`,
`TYPED_CONTEXT_AND_EXPRESSIONS.md`, `GENERATION_TWO_FIXTURES.md`,
`D19_CONTRACT_FREEZE.md`, `D19_STATIC_SCHEMA_DELTA.md`, `SCHEMA_REVIEW.md`.
This slice adds 14 files and updates 16 pre-existing files; earlier dirty-tree
changes shown by git status are not part of this inventory.

## Prior CXT2 acceptance — historical scope

The following records the completed CXT2 slice before CXT1a was selected. Its
then-next-step and no-source-change statements are historical evidence only.

Updated 2026-09-12. **Suhail accepted R1–R8 as proposed; CXT2 acceptance is complete.**
Read the [accepted contract freeze](architecture/D19_CONTRACT_FREEZE.md),
[static schema delta](architecture/D19_STATIC_SCHEMA_DELTA.md), and
[inert DDL/inventory](architecture/proposals/d19-cxt2/README.md).
The primary review found no blocking inconsistency; approval was explicitly
recorded for all eight decisions on 2026-09-12. Only CXT2 acceptance was selected.

CXT2 translated the approved signatures into eleven `.proposal.sql` files outside
runtime migration directories: SQLite 0007–0012 and PostgreSQL 0008–0012. The
service's unexecuted 0007 privileges reservation is preserved. Added relations
remain **26 local (44 → 70)** and **20 service (35 → 55)**, plus four service
upload-session columns. These are proposed counts, not installed schemas.

The proposal includes concrete constraints, FK indexes, retention/CAS/lifecycle
guards, service access helpers and replacement RLS/grants. Its
[responsibility ledger](architecture/proposals/d19-cxt2/RESPONSIBILITIES.md) names
exact Rust/repository/controller checks that SQL cannot prove, including local
commit-level last-manager checks, access-generation aggregation, canonical typed
bytes, consent, scoped sync installs and package evidence.

**Next eligible slice: separately select CXT1a pure Rust**—IDs, access masks,
portable resource contracts, AssetSet v2 and complete manifest v2 validation.
CXT1b and CXT3a/b/c follow their own bounded gates. CXT1/CXT3/L3 have not begun;
Neon or any live service is not the next step. L3 remains paused.

No runtime/UI/source, applied/local migration, baseline service SQL, fixture
byte/hash, manifest or package specimen changed. No database, service, user
Project or SMB storage was opened. No staging, commit or push occurred. Existing
dirty-tree changes remain intact. The prior documentation-only review evidence
below is retained as history, followed by this acceptance slice's verification.

**S7 D1–D17 approved; bounded L1 and L2 complete.** After L1,
the user explicitly authorized L2 local SQLite implementation and fresh disposable
database tests. No existing user database, service, SMB storage, UI or live Project
was opened/changed. Publication/locks, cloud/auth, staging, commits and release
remain outside this authorization.

## Resume here

Canonical repository:
`/Users/suhail/Library/CloudStorage/Dropbox/matrix/crates/photara`.
Verify the path/branch/status before editing: the task environment may open another
worktree. Read in order:

1. This handoff and [execution roadmap](ROADMAP_0_2_EXECUTION.md), then
   [D19](architecture/LIBRARY_AND_NODE_WORK_SURFACES.md) and
   [revised D18](architecture/TYPED_CONTEXT_AND_EXPRESSIONS.md), followed by
   [storage locations and host bindings](architecture/STORAGE_LOCATIONS_AND_HOST_BINDINGS.md),
   then [exact R1–R8 contracts](architecture/D19_CONTRACT_FREEZE.md) and
   [static delta](architecture/D19_STATIC_SCHEMA_DELTA.md).
2. [Approved product architecture](architecture/GENERATION_TWO_PRODUCT_ARCHITECTURE.md)
   and [S7 decision record](architecture/SCHEMA_REVIEW.md).
3. [L1 implementation boundary](architecture/PROJECT_PACKAGE_CODEC.md).
   Then [L2 implementation boundary](architecture/LOCAL_LIBRARY_IMPLEMENTATION.md).
4. [Logical model](architecture/LOGICAL_DATA_MODEL.md),
   [package format](architecture/PROJECT_PACKAGE_SCHEMA.md),
   [local SQLite](architecture/LOCAL_SQLITE_SCHEMA.md),
   [service PostgreSQL](architecture/SERVICE_POSTGRESQL_SCHEMA.md),
   [synchronization](architecture/SYNCHRONIZATION_CONTRACT.md),
   [fixtures](architecture/GENERATION_TWO_FIXTURES.md), and
   [social profiles/export](architecture/SOCIAL_PROFILES_AND_LIBRARY_EXPORT.md).
5. [Storexa integration](architecture/STOREXA_INTEGRATION.md),
   [current project documents](architecture/PROJECT_DOCUMENTS.md),
   [persistence](architecture/PERSISTENCE.md), and
   [Library architecture](LIBRARY_ARCHITECTURE.md).
6. [Core](architecture/CORE.md), [assets](architecture/ASSETS.md),
   [Node packages](architecture/NODE_PACKAGES.md), and
   [native clients](architecture/NATIVE_CLIENTS.md). For presentation work read
   [Shared UI](../platform/macos/SHARED_UI.md) and
   [design language](../platform/macos/DESIGN_LANGUAGE.md).

[ROADMAP](../ROADMAP.md), [CODEX_HANDOFF](CODEX_HANDOFF.md) and
[FRAME_LIBRARY_HANDOFF](FRAME_LIBRARY_HANDOFF.md) retain historical/operator
context; their older next-work sections do not override this scope.

## Locked decisions

- Native macOS presentation over portable Rust; Graph authoring composition hosts modular
  Inspector and optional Node Work Surfaces. UI implementation still requires
  raster mockup approval; schema approval is not UI approval.
- General versioned Node packages own behavior. No privileged Layout/provider
  cases in Core/bridge. Layout and Gallery are D19's proposed built-ins; LrC/Lr/Ps are
  planned free first-party downloads. Node Store commerce remains future work.
- Stable identity is independent of paths. Package authority owns authored
  Project metadata, typed Library snapshots/assignments, a private graph/run asset
  identity/provenance/artifact ledger, multiple named Graphs and immutable run/attempt/effect/evidence records.
  Catalogs are projections, never a second source of package authority.
- Library Storage Locations are portable logical identities; per-device Host
  Bindings resolve them to macOS, Windows, Linux or provider resources. Packages
  and synchronized Library data contain no absolute paths, bookmarks or secrets.
  External sources, managed Project resources, external artifacts and disposable
  cache are distinct storage classes. Variables resolve typed handles and never
  hide AssetSet membership or workflow dataflow.
- Library is the durable catalog/collaboration boundary, local-first and typed: People with multiple roles and
  relationships, client Organizations, unique LocationKinds and concrete
  Locations with a required kind. Account/Auth0 identity is never a Person.
  Scene is not a separate target domain record; a friendly UI label remains a
  presentation choice. Current Scene code is transitional, not silently removed.
- People/Locations/Project catalog management belongs to the app/Library. Node
  Work Surfaces embed host-owned pickers/Browsers under declared permissions;
  inline creation uses Library commands. Authoring visibility does not grant runtime
  access. Project-only collaborators receive bounded assigned snapshots. Package/SMB
  access remains separate device authorization.
- Graph inputs are explicit typed ports plus declared frozen context. Read/source,
  enrichment and effect/output are distinct; MetadataPatch never mutates originals
  by itself. Stable hierarchical categories/tags are discovery-only metadata.
- Kind claims use the approved Unicode 16.0.0 policy and explicit Beach/beaches
  seed groups. Narrow atomic merge/claim transfer preserves Library/key
  uniqueness. The full normalizer/merge runtime is not implemented by L1.
- Auth0/API/Neon is the cloud trust boundary; desktop has no privileged Neon
  credentials. CloudKit is a deferred explicit record-sync adapter, not SQL.
- Storexa 0.2.0 is published from sibling commit `22c4270`, with explicit SQLite
  and PostgreSQL SQLx types. Photara retains entities, SQL/migrations, package
  publication, authorization and sync policy. L1 uses no database adapter.
- D16 reserves typed manual-first social profiles. Stable bound subjects are
  Library-unique including tombstones; no ordinary reassignment or identity
  proof. Mutable/reusable handles never silently merge owners. Provider adapters,
  Instagram lookup and consent/expiry-aware fetched avatars remain optional.
- D17 reserves future logical checksummed/optionally encrypted Library export:
  separate Project backups, safe root hints/rebind, dry-run/isolated restore,
  no credentials, device IDs, absolute paths, bookmarks or sync/recovery state.
  Export/import is non-gating and unimplemented.
- Clean generation two only. Existing Neon and `v0.1.3` are optional reference/
  salvage. Legacy import is never a schema, implementation or release gate.

## Completed bounded L1

Additive `photara-store::package` provides strict raw JSON validation before
Value construction, canonical-json.v1 checking, versioned typed control/authored/
Graph envelopes, safe descriptor-relative no-follow reads, managed-byte hashing,
HEAD/parent/bootstrap/inventory closure and focused typed authored/history checks.
Original canonical bytes and optional unknown data are retained. No writer API
exists. Current one-JSON ProjectDocument import/export and Core are unchanged.

The S6 33-file inert archive is materialized only in fresh temporary roots.
All 25 focused tests and 29 retained Core/NodeSDK/store tests passed; selected doc
tests passed. Store all-target Clippy with warnings denied and whole-library
all-target compilation passed offline. No database, service, user Project or SMB
storage was opened. Full Unicode graph-name uniqueness, embedded manifest support,
application integration, conversion, publication and recovery remain outside this
bounded reader. See [precise limitations](architecture/PROJECT_PACKAGE_CODEC.md).

## Completed L2 and next exact slice

`photara-library::gen2::LocalLibraryStore` now implements the separate family and
six ordered migrations (44 tables), typed Library CRUD/CAS/tombstones, exact
Unicode-16 Kind claims, local changes and catalog/device/verified-observation
foundations. Storexa 0.2/SQLx 0.9 is adopted privately; existing v1 APIs are intact.
The necessary rusqlite compatibility pin is 0.39.0/libsqlite3-sys 0.37.0. All five
retained Library tests and 16 new L2 tests pass; selected regression total is 75,
with doc tests, library Clippy `-D warnings` and whole-library compile passing.
See [L2 scope and limits](architecture/LOCAL_LIBRARY_IMPLEMENTATION.md).

**Next: separately select CXT3a.** R1–R8, CXT2 inert DDL and CXT1a/b pure contracts
are complete. CXT3a package readers, CXT3b disposable local migrations/repositories
and CXT3c disposable service/RLS/fake transport each require their own scope. L3 waits for accepted
contracts and required conformance. See the companion's exact slices and gates.
S2–S6 remain baseline evidence: S3 is 44 tables/S4 35; all six L2 migration
checksums and pre-existing inert fixture bytes are unchanged. The proposed totals are neither
installed schema counts nor runtime test evidence.
Resolve remaining L1
writer-readiness gaps before publishing any package. L2b Kind merge/claim transfer
remains unexposed: its exact affected-root/CAS/rebind/retirement/promotion proof must
pass before merge or cloud reconciliation is offered. No automatic L3, SMB/user
storage, cloud, UI, staging/commit or release authorization is implied.

## Design evidence still awaiting runtime proof

S3's 44 SQLite tables/179 statements now install through L2's six migrations;
selected local FK/trigger/concurrency tests pass. S4 has 35 PostgreSQL tables/275
statements, RLS and service functions and remains unexecuted. Full merge/sync,
service privilege/concurrency and publication tests remain future work.
S5 defines sealed commands/receipts, offline chains, ordered feeds, bounded reset,
conflict/rebase, media staging and privacy; no API/Auth0/object-store was contacted.
S6 has 51 scenario specifications and 12 crash points; passing L1 does not mark
their database/service/SMB/import families passed. S2's publication and recovery
protocol is approved design, not an fsync/rename/SMB guarantee.

## Working tree warning

The canonical tree already contains substantial uncommitted docs, shell/library
presentation, Inspector, lab, build configuration and UI verification work.
These belong to the user/ongoing work and were preserved. The planning docs and
L1 sources are also uncommitted. Inspect live status; do not reset, restore, clean,
stage or commit unrelated changes. Storexa's separate release is already complete.

## Task and model allocation

Use Astra with high reasoning for architecture/heavy Rust/persistence and Sol for
bounded UI/module slices after contracts and scope approval. This does not itself
authorize spawning tasks/agents. Give each slice a concrete boundary and gate.

## Completion checklist

- [x] Architecture and S1 logical model accepted; Storexa 0.2.0 published.
- [x] S2–S6 designs, inert fixtures and static SQL/JSON/hash/link checks prepared.
- [x] S7 D1–D17 approval recorded on 2026-09-11.
- [x] Bounded L1 read-only implementation/test scope selected and passed.
- [x] Actual changes, limits and next gate recorded.
- [x] L2 database scope separately authorized; implementation and temporary tests passed.
- [x] D18 concept inclusion and source syntax requested; documentation/inert addendum prepared.
- [x] D19 conceptual direction approved and documentation amendment prepared.
- [x] D19 consistency reviewed; exact logical/package/NodeSDK freeze candidate prepared.
- [x] Static physical/package/sync/DTO/fixture delta specified as inert documentation.
- [x] Suhail accepted R1–R8 as proposed, 2026-09-12; CXT2 acceptance only selected.
- [x] CXT2 inert SQL, exact inventory and static checks complete; no execution.
- [x] CXT1a separately selected and completed; pure contracts and golden fixture verified.
- [x] CXT1b separately selected and complete; pure context/golden/regression verified.
- [ ] CXT3a separately selected, followed by CXT3b/c proof before L3.
- [ ] Remaining migration/service/publication/recovery scenarios pass before
  their corresponding release claims.

## CXT2 acceptance verification — historical, 2026-09-12

This acceptance slice adds **14 files** (11 inert SQL proposals and three proposal
README/inventory/responsibility documents) and updates **27 Markdown files**.
The exact paths are listed below; the earlier review's delta is separate history.

| Proposal | Statements | New tables | Explicit indexes | Triggers | Functions | New policies | Old policies removed |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| SQLite 0007–0012 | 139 | 26 | 38 | 74 | 0 | 0 | 0 |
| PostgreSQL 0008–0012 | 431 | 20 | 33 | 42 | 27 | 173 | 25 |

The service total includes 44 ALTER TABLE statements (40 enable/force-RLS, three
forward-reference constraints and the upload-column extension), privilege changes
and the inert API-floor update. Local floor changes likewise remain inert text.
No generated proposal is referenced by a runtime migration runner.

Checks passed: **46 exact relation-column/nullability signatures**, **104 scoped
FK target/key/index checks**, lexical SQL statement/delimiter/quote checks,
identifier uniqueness/length checks, explicit replacement of all 25 old policies,
enable/force RLS on all 20 service additions, **319 local Markdown links/anchors**,
changed-file whitespace and `git diff --check`. These checks read SQL as text;
no database, interpreter, compiler or service was started. No installed pglast,
sqlglot or sqlparse was available in the checked Python environments; grammar,
SQL type/name resolution and runtime behavior remain unverified CXT3 gates.

All **236 pre-existing non-Markdown files** retain their pre-slice SHA-256.
All six applied/local migration files, all nine fixture-directory files and the
original complete S2/S3/S4/S5 baseline bodies remain byte-identical. The inventory
records protected-file hashes. Existing SQLx checksums, package specimen, fixture
hashes, runtime/Rust/Swift/UI, manifests and unrelated dirty files were preserved.

SQL cannot by itself prove local commit-level manager/aggregate invariants,
expected-revision intent, canonical/typed/privacy agreement, complete controller
lock ordering, bootstrap/cursor correctness, identity/token checks or package
publication evidence. The responsibility ledger states the exact future checks.
No CXT1a implementation, CXT3 database work or L3 publication was begun. No staging,
commit or push occurred. **Next: separately select CXT1a pure Rust, not Neon.**

Exact changed files for this acceptance slice:

```text
README.md
ROADMAP.md
docs/ACTIVE_HANDOFF.md
docs/ROADMAP_0_2_EXECUTION.md
docs/architecture/CORE.md
docs/architecture/D19_CONTRACT_FREEZE.md
docs/architecture/D19_STATIC_SCHEMA_DELTA.md
docs/architecture/GENERATION_TWO_FIXTURES.md
docs/architecture/GENERATION_TWO_PRODUCT_ARCHITECTURE.md
docs/architecture/LIBRARY_AND_NODE_WORK_SURFACES.md
docs/architecture/LOCAL_LIBRARY_IMPLEMENTATION.md
docs/architecture/LOCAL_SQLITE_SCHEMA.md
docs/architecture/LOGICAL_DATA_MODEL.md
docs/architecture/NATIVE_CLIENTS.md
docs/architecture/NODE_PACKAGES.md
docs/architecture/PERSISTENCE.md
docs/architecture/PROJECT_DOCUMENTS.md
docs/architecture/PROJECT_PACKAGE_CODEC.md
docs/architecture/PROJECT_PACKAGE_SCHEMA.md
docs/architecture/README.md
docs/architecture/SCHEMA_REVIEW.md
docs/architecture/SERVICE_POSTGRESQL_SCHEMA.md
docs/architecture/SOCIAL_PROFILES_AND_LIBRARY_EXPORT.md
docs/architecture/STORAGE_LOCATIONS_AND_HOST_BINDINGS.md
docs/architecture/STOREXA_INTEGRATION.md
docs/architecture/SYNCHRONIZATION_CONTRACT.md
docs/architecture/TYPED_CONTEXT_AND_EXPRESSIONS.md
docs/architecture/proposals/d19-cxt2/INVENTORY.md
docs/architecture/proposals/d19-cxt2/README.md
docs/architecture/proposals/d19-cxt2/RESPONSIBILITIES.md
docs/architecture/proposals/d19-cxt2/postgresql/0008_library_project_access.proposal.sql
docs/architecture/proposals/d19-cxt2/postgresql/0009_storage_locations.proposal.sql
docs/architecture/proposals/d19-cxt2/postgresql/0010_library_context.proposal.sql
docs/architecture/proposals/d19-cxt2/postgresql/0011_scoped_sync.proposal.sql
docs/architecture/proposals/d19-cxt2/postgresql/0012_d19_access_guards.proposal.sql
docs/architecture/proposals/d19-cxt2/sqlite/0007_library_project_access.proposal.sql
docs/architecture/proposals/d19-cxt2/sqlite/0008_storage_locations_bindings.proposal.sql
docs/architecture/proposals/d19-cxt2/sqlite/0009_library_context.proposal.sql
docs/architecture/proposals/d19-cxt2/sqlite/0010_context_apply_recovery.proposal.sql
docs/architecture/proposals/d19-cxt2/sqlite/0011_scoped_sync.proposal.sql
docs/architecture/proposals/d19-cxt2/sqlite/0012_d19_guards_and_floor.proposal.sql
```

## Prior documentation review verification — 2026-09-12

Passed for this slice: **292 local Markdown links/anchors**, all changed-file
whitespace including untracked Markdown, Project action-mask arithmetic, exact
relation inventories (14 common + 12 local-only; 14 common + 6 service-only),
26 proposed package schema rows, eight unchanged fixture JSON parses, and
`git diff --check`. The six migration files and all nine fixture-directory files
(including README) are byte-identical to the task-start baseline. Hash comparison
also proves all **236 existing non-Markdown files** unchanged. Reconstructing the
pre-notice S2/S3/S4/S5 documents matches their original full-file hashes, so their
SQL, JSON and protocol baseline text is preserved exactly.

No new SQL grammar/runtime/RLS/codec/interpreter test was run: the proposal uses
inert relation signatures, not executable DDL. No Rust build or database test was
needed or authorized for this documentation-only slice. The commands below are
historical L1/L2 checks, not new runtime evidence.

Exact task delta: **two new review documents and 26 updated Markdown files**.
New files: `docs/architecture/D19_CONTRACT_FREEZE.md` and
`docs/architecture/D19_STATIC_SCHEMA_DELTA.md`. Updated files:

```text
README.md
ROADMAP.md
docs/ACTIVE_HANDOFF.md
docs/ROADMAP_0_2_EXECUTION.md
docs/architecture/ASSETS.md
docs/architecture/CORE.md
docs/architecture/GENERATION_TWO_FIXTURES.md
docs/architecture/GENERATION_TWO_PRODUCT_ARCHITECTURE.md
docs/architecture/LIBRARY_AND_NODE_WORK_SURFACES.md
docs/architecture/LOCAL_LIBRARY_IMPLEMENTATION.md
docs/architecture/LOCAL_SQLITE_SCHEMA.md
docs/architecture/LOGICAL_DATA_MODEL.md
docs/architecture/NATIVE_CLIENTS.md
docs/architecture/NODE_PACKAGES.md
docs/architecture/PERSISTENCE.md
docs/architecture/PROJECT_DOCUMENTS.md
docs/architecture/PROJECT_PACKAGE_CODEC.md
docs/architecture/PROJECT_PACKAGE_SCHEMA.md
docs/architecture/README.md
docs/architecture/SCHEMA_REVIEW.md
docs/architecture/SERVICE_POSTGRESQL_SCHEMA.md
docs/architecture/SOCIAL_PROFILES_AND_LIBRARY_EXPORT.md
docs/architecture/STORAGE_LOCATIONS_AND_HOST_BINDINGS.md
docs/architecture/STOREXA_INTEGRATION.md
docs/architecture/SYNCHRONIZATION_CONTRACT.md
docs/architecture/TYPED_CONTEXT_AND_EXPRESSIONS.md
```

Canonical pages now point to exact decisions instead of leaving the freeze
unspecified; older historical baseline text remains labeled and preserved.
Unrelated existing dirty changes are not included in this task's change claims.
Git's full diff includes prior work and omits untracked file content; do not use
its aggregate diff-stat as this review's change inventory.

## Verification commands

Run from canonical repository. L1 checks actually run offline:

```sh
git status --short
git diff --check
cargo fmt -p photara-store -- --check
cargo fmt -p photara-library -- --check
cargo test --offline -p photara-library -p photara-store -p photara-core -p photara-node-sdk
cargo clippy --offline --workspace --all-targets -- -D warnings
cargo check --offline --workspace --all-targets
```

Untracked files need explicit inspection; ordinary git diff omits them. Whole-
library tests were not needed for this bounded slice; selected Library tests
now run with L2's temporary-database authority. Compilation is not service or
package-publication verification.
