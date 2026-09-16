# BR0 — Brand Cutover / Rename Readiness

Status: implemented in isolated worktree, **awaiting Suhail review**. Baseline is
published PS0 `6cfd0f12a9cc42c7e7ff28ded84d1dffd8975f26`. No commit, push, install,
deployment or final brand selection is authorized by this report.

[PS0 contract](PROJECT_SESSION_DURABILITY.md) remains normative. PS1 remains gated
on BR0 acceptance; the 1,024-commit retention issue stays with PS1/PS2. This slice
adds no Graph durability writer, project switch or database migration.

## Result and compatibility

The development product remains Photara. Existing display name, bundle identity,
Auth0 issuer/client/audience/callback, Keychain service, public URLs, project suffix,
default project destination and support/journal database location remain unchanged
under the checked descriptor. Production and remoteAcceptance remain unprovisioned.

`config/product-identity.json` now also names product bundle and executable
independently, document UTI, legacy package extensions, default project folder,
journal folder, legacy preference suites and the installed development-operator
folder/product. The generator validates safe path components and extension policies,
emits typed Swift configuration and bundle metadata, and registers current plus
legacy extensions. No runtime descriptor or endpoint override was added. The
synthetic build uses a disposable source snapshot with its own checked descriptor.

Native create form, initial destination, recent/open/URL routing, alerts, launcher,
project-title and shell-preset defaults consume configuration. Package open detection
accepts reviewed aliases without changing names or contents. The native UI1 route
still requires the existing creation journal; this does not implement arbitrary
package adoption/opening or an editable package session.

The bridge receives a validated filename extension from native configuration.
Core title validation is independent of the suffix; actual filesystem filename
validation budgets the configured extension against 255 UTF-8 bytes. The local
creation request pins its extension immutably. No extension enters InitialProject,
portable package records, content digests or cloud creation command bytes.
Missing extension in pre-BR0 local requests decodes as the explicit legacy alias.
Under current Photara configuration that default field is omitted when serialized,
preserving the earlier local request encoding. Nonlegacy requests require this
reader; older binaries fail closed on their unknown field, rather than silently
publishing the wrong filename. No migration ordinal or DDL was added.

New stages use `.project-create-<operation UUID>-<nonce UUID>`. Existing
`.photara-create-…` stages retain a narrowly scoped read/recovery alias and must
still pass exact directory pins. New creation always uses the configured extension.
Completed legacy operations reopen without rewriting any package bytes. The bridge
refuses advancing an **incomplete** operation whose recorded extension differs
from current configuration. Resolve such intents under the original configuration
before cutover; this checkpoint does not silently relocate/rewrite them.

Support, cache and journal roots are configuration-driven; creation and onboarding
now agree on the journal root. The existing `photara-local-v2.sqlite` filename,
legacy preference keys and `photara.*` protocol/schema IDs are explicit compatibility
coordinates, not public names. New PS session journals must use UUID components
beneath configured roots. The proxy cache now uses the configured user Caches root
(or an explicit disposable support-root override); old `Support/ProxyCache` bytes
are left untouched and not migrated/deleted. Regeneration is intentional.

No existing local directory is automatically adopted or moved when public roots
change. Legacy preference-suite reading is explicit in config; the synthetic
identity has no legacy suite. Directly generated configuration is required at
runtime; defaults in Swift memberwise initializers support existing synthetic test
fixtures and do not replace required generated JSON keys.

## Verification

| Gate | Evidence |
| --- | --- |
| Core/store contracts | 176 passed, 0 failed, 3 explicit fixture generators ignored |
| Local creation coordinator | 6 passed, including synthetic suffix/restart, old request decode and changed-policy refusal |
| Rust lint/compile | `cargo clippy --locked --offline -p photara-core -p photara-store -p photara-library -p photara-bridge --all-targets -- -D warnings` passed |
| Descriptor admission | 9 Python tests passed, including unsafe path/extension refusal and synthetic identity with stable internal schema family |
| Source regression guards | Production app/shell/foundation public literals and Rust filename policy checked; only explicit persisted aliases allowed |
| Current native app | Full Rust/UniFFI/Swift build; ad-hoc signature verification passed; no launch/install |
| Current typed config | Existing Swift release-configuration assertions passed, retaining development login coordinates |
| Synthetic native app | Full `Juniper Studio.app` build with distinct `JuniperHost` executable, `jprtest` suffix, configured UTI/aliases/callback metadata; ad-hoc verification passed |
| Generated configuration consistency | Decoded generated Swift payload equals validated descriptor, including Keychain namespace, directories, audience, user agent and URLs |
| Synthetic persistence furnace | Separate CLI processes create initial durable package, exit, reopen and verify exact Project/Graph/commit IDs plus byte manifests; both current suffix and legacy `.photara` pass |
| Interrupted cutover | Fresh incomplete legacy intent under synthetic configuration is refused before package publication |
| Current offline authentication | 94 security + 58 fake-network + 64 presentation assertions passed; credentials in memory, temporary locks only |
| Synthetic offline authentication | Same existing offline authentication suites run against generated synthetic identity; no live provider or Keychain store |
| Documentation/scope | Relative links, whitespace, exact inventory and protected-source/config comparison checked at review handoff |

The persistence furnace verifies **UI1's initial durable save**, not Graph-edit
Save Now/autosave. It calls the actual Rust bridge from Swift but never launches
AppModel, a service or the installed application. Package editing remains read-only
until later PS slices. Source/config guards and build metadata are evidence for
public identity substitution. The subsequent native GUI acceptance is recorded below;
VoiceOver inspection is not claimed.

Reproduce offline checks with `python3 scripts/test_product_configuration.py`,
`python3 scripts/verify_brand_readiness.py` and the listed Cargo commands. Run
`python3 scripts/verify_brand_readiness.py --build-output /private/tmp/<new-name>`
for the synthetic build/furnace. The output directory must not exist; it retains
source snapshot, generated metadata, package manifests and disposable databases.
Native builds need host execution where SwiftPM's nested sandbox is unavailable.

Raw local logs (ephemeral, not repository artifacts):

- `/private/tmp/photara-br0-core-store-final.log`
- `/private/tmp/photara-br0-creation-final.log`
- `/private/tmp/photara-br0-clippy-final.log`
- `/private/tmp/photara-br0-current-final.log`
- `/private/tmp/photara-br0-synthetic-final.log`
- `/private/tmp/photara-br0-auth-tests-retry.log`
- `/private/tmp/photara-br0-synthetic-auth.log`
- `/private/tmp/photara-br0-synthetic-approved/current-manifest.json`
- `/private/tmp/photara-br0-synthetic-approved/legacy-manifest.json`

Earlier attempts are retained: SwiftPM nested sandbox refusal on initial builds;
the first synthetic harness asserted on Foundation's `/private/tmp` → `/tmp`
normalization (fixed without changing product code); initial offline authentication
failed at the temporary-lock gate under sandbox and passed on host retry. Clippy
found a newly added panic accessor; it was replaced with an infallible locator over
validated requests and strict clippy now passes. These failures are not hidden.

## Synthetic native GUI acceptance — 2026-09-16

**Passed:** the real native UI displayed Juniper Studio / Juniper Visual Library,
hid/restored the sidebar, created `Juniper Visual Acceptance.jprtest`, displayed
Graph 1 / Saved, quit, restarted, and reopened that package through the native
Open Project Package dialog. A completed disposable legacy intent/package was
then seeded through the real Rust bridge under the `photara` policy, and the
renamed native app opened `Legacy Visual Fixture.photara` through the same picker.
Both extensions appeared as Juniper Project. This legacy seed was CLI-created;
the `.jprtest` project was GUI-created. Screenshots of opening, creation and saved
Graph states were displayed inline in the acceptance task; no screenshot files
were exported. The synthetic app was quit after testing.

This exercises UI1 initial creation/save and completed-operation reopen only.
Graph editing, autosave, Save Now, arbitrary package adoption, project switching,
VoiceOver and live authentication were not exercised.

The sole new repository implementation file for this extension is
`scripts/prepare_br0_visual.py`; all app injections exist only in the disposable
source snapshot. The original 27-file BR0 manifest matched before this extension;
only this report, ROADMAP and ACTIVE_HANDOFF were subsequently amended for evidence.
Run `python3 scripts/prepare_br0_visual.py --output /private/tmp/<new-name> --build`
to prepare/build a fresh fixture. It never launches automatically.

### Isolation and concrete evidence

Fixture root: `/private/tmp/photara-br0-visual`; executable:
`build/Juniper Studio.app/Contents/MacOS/JuniperHost` beneath that root.
Bundle: `org.example.juniper.br0.visual.2ba2c20b226a4795a34e186f099b51e6`.
The distinct defaults suite appends `.preferences`; app defaults are file-backed
at `disposable/Preferences.plist`. AppModel receives the explicit
`disposable/ApplicationSupport` root; the journal/database are
`Recovery/photara-local-v2.sqlite`, packages are in `CreatedProjects`, and the
configured cache is `JuniperCache` beneath that support root. No cache directory
was needed/created during these empty-Graph checks.

Before app stores open, the fixture applies a fail-closed runtime sandbox and
verifies that a disposable denied-read canary cannot be read. The profile denies
IP networking, reads/writes beneath `/Users` and `/Volumes`, and securityd/secd
lookups; writes are limited to disposable data and OS temporary roots. Separate
IPv4/IPv6 connection probes returned EPERM. The production cloud driver is absent
from this snapshot and replaced by a local-ready stub; it cannot construct a
credential store or installed-service client. Shared developer preference/theme
overrides are disabled. This is a synthetic test seam, not a production override.
Cocoa launches before the pre-store gate; native system open panels still display
OS favorites/recent-location metadata. No live path was selected. This does not
claim isolation of all system UI metadata.

The final read-only database audit returned `integrity_check = ok`, two complete
creation intents, two project catalog rows, two graph projections, two device
bindings and one library contract row. All 14 files in each package (including
the creation lock) matched their pre-reopen SHA-256 manifests exactly.

| Package | Project UUID | Commit UUID |
| --- | --- | --- |
| Juniper Visual Acceptance.jprtest | `603a3c11-c799-4528-9547-34ab6b416413` | `c2271e00-4608-4a62-84a2-f58de09a9c0b` |
| Legacy Visual Fixture.photara | `47fe467e-61e2-4c18-8436-760cbc884d4d` | `ac1d7afc-96e3-46fe-9695-308004e55949` |

Raw evidence beneath the fixture root: `visual-fixture.json`, `isolation.sb`,
`disposable/startup-proof.json`, `disposable/final-audit.json`,
`disposable/gui-package-before-reopen.json`,
`disposable/legacy-package-before-reopen.json`, and `disposable/legacy-seed.json`.
Final build/signature log: `/private/tmp/photara-br0-visual-native-build.log`.

Protected-file comparison found **zero changes across 1,838 file/path entries**:
Photara Application Support, Pictures/Photara, installed development service,
real app bundle, real preference files and the checked product descriptor.
Baselines are `/private/tmp/photara-br0-live-before.json` and
`/private/tmp/photara-br0-live-after.json`. Keychain contents were never read;
of 14 metadata entries, one system `keychain-2.db-wal` entry changed. Its cause
is unestablished; this audit does not claim Keychain byte identity or attribute
the change. The fixture excludes credential APIs and denies their service access.
No Auth0/Neon requests or service mutation occurred. The existing service remained
PID 68675. The real app was stopped initially and was not launched or replaced:
`/Users/suhail/.codex/worktrees/84ad/photara/platform/macos/photara-app/.build/app/Photara.app`.

Earlier fixture-only failures are retained: a preparation syntax error, SDK 27
rejecting direct sandbox declarations, a wrapper launch that CUA could not attach
to, an overly broad network rule affecting local IPC, temporary-directory alias
write refusal, and SDK rejection of a defaults suite equal to the bundle ID.
The final fixture uses the system sandbox runtime entry point resolved before
stores open, IP-only network denial, explicit temporary path aliases and a
separate preferences suite. No production code was changed to fix these harness
issues. Abandoned wrapper processes were stopped; final process inspection showed
no Juniper process and the original service still running.

## Deferred coordinated cutover

Brand, website/domain, marketing name and production trust enrollment stay deferred
until the product is concrete. A final cutover must review/provision Apple signing,
entitlements and registrations, Auth0 callback/client/audience and service trust
coordinates together. Changed credential namespaces may require sign-in again.
No token copy, new account enrollment or implicit authority transfer is implemented.
Development operator installation/provisioning tools retain their existing internal
executable and trust contracts; a final operator deployment must explicitly match
its configured folder/product and signing coordinates.

A controlled local-directory migration is **not implemented**. Before enabling one,
review exact old/new roots and owned files, active-process exclusion, collision and
permission handling, byte/identity manifests, interruption recovery, rollback and
completion receipts. Never merge unrelated directories or delete old data based
on a brand string. This version simply uses configured roots and leaves old roots
alone. No rename-related fixture accesses live packages, SQLite/Neon, Keychain,
Auth0, installed services, source photographs or archives.

Stable persisted format/node/value/schema identifiers, operation UUIDs, migration
history and deployed PostgreSQL namespaces remain unchanged. Any internal rename
still requires its own migration/alias contract. The existing explicit-save legacy
route and UI1 read-only package route remain separate; general package migration,
project switching, Gallery/COV and LL1 are not part of BR0.

## Exact change inventory

28 files; no staged files, migration files or generated build artifacts.

1. `ROADMAP.md`
2. `config/product-identity.json`
3. `crates/photara-bridge/src/creation.rs`
4. `crates/photara-core/src/creation.rs`
5. `crates/photara-library/src/gen2/creation.rs`
6. `crates/photara-store/src/package/creation/filesystem.rs`
7. `crates/photara-store/tests/package_creation.rs`
8. `docs/ACTIVE_HANDOFF.md`
9. `docs/architecture/BR0_RENAME_READINESS.md`
10. `platform/macos/photara-app/Sources/AppModel.swift`
11. `platform/macos/photara-app/Sources/AppModelLibrary.swift`
12. `platform/macos/photara-app/Sources/ApplicationAdapter.swift`
13. `platform/macos/photara-app/Sources/EditorSessionView.swift`
14. `platform/macos/photara-app/Sources/PhotaraMacApp.swift`
15. `platform/macos/photara-app/Sources/ProductionOpeningCloudDriver.swift`
16. `platform/macos/photara-app/Tests/BrandReadiness/main.swift`
17. `platform/macos/photara-app/build-app.sh`
18. `platform/macos/photara-shell/Resources/photara-application-presentation-v1.json`
19. `platform/macos/photara-shell/Sources/ApplicationShellPreset.swift`
20. `platform/macos/photara-shell/Sources/CreateProjectView.swift`
21. `platform/macos/photara-shell/Sources/ProjectLauncherView.swift`
22. `platform/macos/photara-ui-foundation/Sources/NativeDevelopmentService.swift`
23. `platform/macos/photara-ui-foundation/Sources/ReleaseConfiguration.swift`
24. `platform/macos/photara-ui-foundation/Sources/ThemeStore.swift`
25. `scripts/generate_product_configuration.py`
26. `scripts/test_product_configuration.py`
27. `scripts/verify_brand_readiness.py`
28. `scripts/prepare_br0_visual.py`

All pre-existing product-identity fields and every environment/trust coordinate
match the published PS0 descriptor. Tracked migration and golden fixture bytes,
Graph source and unrelated files are unchanged. HEAD remains the published PS0
commit; all BR0 work is uncommitted for review.
