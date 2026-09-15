# UI1 — atomic Create Project

Implementation contract, 2026-09-14, starting at `c605475`. This slice is authorized
for implementation and disposable verification, not publication or live data writes.

## Authority and presentation

The approved shared Compact sheet is the production presentation. Opening retains
native macOS colors. Balanced and Spacious remain Lab comparisons. The selected
Library is fixed for a submitted operation. Rust owns validation, identities,
package bytes, state transitions and receipts. Swift owns the native directory
chooser, security scope, progress and recoverable diagnostics.

A new Project has one owning Library, restricted access, an explicit creator
manager, and one empty saved Graph named `Graph 1`. Package format 1.1 and its
checked closure are authoritative. Empty asset/resource ledgers contain no archive
copies. Host paths, credentials and access grants never enter portable objects.

## Immutable request and result

One operation UUID binds Library, actor, trimmed title, destination identity,
Project/Graph/commit UUIDs and exact canonical initial package bytes. Reusing an
operation with different input is a conflict. Competing operations cannot reserve
the same Project identity or destination. Destination edits require a new preview
and operation; a submitted destination never silently changes during recovery.

Titles trim Unicode whitespace, reject empty/control/separator/device names,
trailing dot and filesystem-unsafe characters, and fit a 255-byte filename including
`.photara`. Rejection is explicit; no lossy replacement can collapse two names.
The destination must exist as an authorized directory. Its device/inode identity
is pinned and rechecked; symlinks, disconnected/remounted destinations, occupied
paths and permission failures are distinct recoverable failures.

The successful receipt binds operation, Library, Project, Graph, package commit
and digest, local catalog observation, destination and optional service receipt.
Success requires validating all coordinates. Reopening reads those same saved
identities, never creates a replacement Project or Graph.

## Transaction and recovery

There is no distributed filesystem/SQLite/PostgreSQL ACID transaction. Use a
durable local intent plus private same-volume staging and idempotent service
receipt. No success is returned while any required coordinate is incomplete.

1. Validate and durably reserve the exact request in SQLite before filesystem or
   network effects. Stage canonical objects, bootstrap, commit and HEAD; synchronize
   files and directories and validate through the existing package 1.1 reader.
2. A local Library requires no service. A cloud Library dispatches only through the
   authenticated local development service; that service owns PostgreSQL access.
   Registration, restricted policy/manager, catalog projection and service receipt
   commit together. Desktop never receives PostgreSQL/Neon credentials.
3. Publish the staged directory without replacing an existing destination. Commit
   the local registration, catalog observation/Graph projection and completion
   receipt together. The journal remains durable through publication and restart.
4. Before cloud dispatch/publication, cancellation removes only verified owned
   staging and releases reservations. After dispatch, an unknown remote outcome
   must reconcile the same operation before cancellation can be decided. A known
   committed cloud operation rolls forward; it must never be mislabeled cancelled.
5. Interrupted or failed work retains exact intent and evidence for retry. Missing
   volumes or changed destinations require user action. Existing unrelated files
   are never removed, adopted or overwritten. Recovery revalidates ownership and
   package bytes; a receipt alone does not prove the package still exists.

Cloud offline work remains a local pending intent, labeled awaiting connection;
cached access does not manufacture current remote authority or cloud success.
Post-commit filesystem failure reports recoverable incomplete creation and retains
the service receipt for roll-forward. This is the UI1-approved incomplete-operation
outcome, not an assertion of cross-system instantaneous atomic visibility.

## Verification and boundaries

Use disposable SQLite, process identities, packages and a private PostgreSQL
cluster. Verify exact rows and canonical package closure independently, including
restart/replay, concurrent duplicate requests, cancellation, collision, permission,
volume change and failures on both sides of each durable boundary. Gate production
wiring with Rust/schema/service/package, bridge, native shared/production UI,
signing/accessibility, formatting/Clippy and the unchanged full Graph furnace.
Record measured results here; planned coverage is not passing evidence.

UI2 browsing/recovery redesign and UI3 Graph-window redesign are excluded. Use the
existing safe shell for the resulting Project. Library creation/rename/deletion is
the next separately gated **Library Lifecycle** slice. Deletion requires exact
Library-name retyping, current authorization/ownership and explicit impact counts;
it transactionally removes Library and Project catalog records/references without
database orphans, and never deletes `.photara` packages or external source/archive
files.

## Implementation

The uncommitted implementation starts at `c605475`. Core owns the portable name
and identity contract; Store produces the canonical package closure; Library owns
the durable SQLite coordinator. UniFFI exposes typed public coordinates. Native
Swift selects the destination and uses the existing refresh/device-credential and
Rust HTTP service path for `/v1/projects/create`. No PostgreSQL migration, service
installation, Fly resource, or live Neon/user-state change is part of this work.

The service composes the existing registration and catalog publication helpers in
one PostgreSQL transaction, including explicit restricted creator access, change
feed and the immutable command receipt. Retry checks current authenticated
identity/device/Library authority and exact canonical command bytes. Local cloud
projection requires that receipt. Account snapshots and creator grants come from
the service; cached state cannot synthesize cloud authorization.

SQLite migration **0015** adds `project_creation_intents` and raises the local
reader/writer floor to **5**. Earlier migration bytes are unchanged. Existing v14
clients must refuse this database after upgrade; installing or testing against the
user's real database requires a separately reviewed publication step. Private
PostgreSQL remains migration **14**, API floor **3**. The package remains **1.1**.

The shared Compact sheet supports native directory selection, inline errors,
retry, cancellation and Keep for Later. A submission fixes its title, identities
and destination. Repeated clicks cannot allocate a second operation. A Cancel
received during preparation is honored before staging. Once publication or remote
dispatch has begun, cancellation cannot erase a possibly committed operation;
retry reconciles it, and Keep for Later retains it. Restart restores the pending
operation without opening a browser or reading credentials until explicit retry.

Completion opens the existing shell with the saved `Graph 1` and the accepted
Graph canvas. The shell capability disables legacy document-authoring controls
for this package; no Graph implementation, fixture, preset or behavioral oracle
was changed. Recents, the Open panel and macOS package URL delivery reopen the
same completed creation after reader validation. General moved/imported package
browsing and recovery remain UI2; package Graph editing remains UI3. The sheet uses
the existing default Library display label; Rust resolves and fixes the actual
selected Library identity. Library rename/multi-Library browsing are not added.

### Recovery boundaries and retained evidence

- A private stage is created with a fresh random suffix on every unjournaled
  attempt. Its device/inode pin is committed before any package content is written.
  A crash between `mkdir` and that SQLite commit may leave an **empty private
  directory**. Retry creates a fresh stage and completes the original operation;
  it never adopts or deletes the unproven directory. This deliberate residue has
  no package/catalog/cloud identity and is not reported as a completed Project.
- Once pinned, partial canonical files are reconstructed and synchronized. Final
  publication uses an exclusive same-volume rename; an empty folder, ordinary
  file or symlink at the final path is a collision, never an overwrite.
- A crash after rename is reconciled using the pinned directory identity and
  complete package reader validation. Catalog installation and completion share
  one transaction; rollback leaves a recoverable published package and no partial
  catalog/Graph projection.
- Cancellation has its own durable `cancelling` state before owned-stage cleanup.
  A crash after cleanup resumes cancellation, never publication. Cleanup only
  removes known files from the pinned stage; unknown entries cause an incomplete
  cleanup error. Source assets, archives and unrelated paths are never traversed
  for deletion.
- A dropped cloud response leaves `dispatching`. A fresh native process replays
  the same command through refreshed authentication and receives the exact stored
  receipt. An independent PostgreSQL observer and SQLite/package readers check
  the resulting coordinates. Cloud records may already be visible while local
  publication is pending; this is recoverable creation, not a distributed ACID
  guarantee. An unavailable/replaced volume must be restored before completion.

## Integration checkpoint — 2026-09-15

See the [integration checkpoint](UI1_INTEGRATION_CHECKPOINT.md) and
[machine-readable current evidence](verification/ui1-integration-20260915.json)
for reconciliation onto verified remote main `c605475`, the exact 78-file inventory,
additional SwiftPM cache isolation, fresh command outcomes and remaining gates.
The measurements below retain their original source-run dates and provenance.
The completed follow-up fixes the Graph test host/launcher, native focus/menu harness and native hit-test
coordinate adapter. Final full Graph: **19,093 assertions / 0 failures**; exact failing seed
924/0, context matrix 2,239/0, real-process furnace 49 launcher + 60 native checks.
The initial failed integration run remains preserved in the linked evidence.

## Verification evidence

Disposable runs on 2026-09-14; see the adjacent machine-readable
[UI1 verification report](verification/ui1-creation-20260914.json).

| Gate | Measured result |
| --- | --- |
| Full offline Rust suite | 289 tests passed; 12 PostgreSQL tests run separately; 4 fixture generators intentionally ignored |
| UI1 local coordinator | 5 tests: exact restart/replay, reservation/cancel, published-package catalog rollback, failed stage-pin commit with independent concurrent pools, interrupted cancellation |
| UI1 package filesystem | 4 tests: 13 exact canonical files and reader closure, collision preservation, replaced volume/symlink refusal, partial reconstruction and permission recovery |
| Private PostgreSQL 18.6 | 12 tests passed; creation registration/projection/grant/receipt replay and rollback, plus retained RLS/authority/sync/onboarding suite |
| Native HTTP service furnace | Real `/v1/projects/create`; process exit after committed response, fresh-process replay, exact SQLite/PostgreSQL/package coordinates; 260 Swift assertions total, including 7 added creation-recovery assertions |
| Native authentication | 94 security + 58 network + 64 Opening model assertions |
| Signed service-process furnace | 99 assertions / 13 real-process scenarios; synthetic credentials and processes cleaned up |
| Full unchanged Graph furnace | 15,644 assertions / 0 failures; 12 configurations and 12×180 seeded actions; signed native input/capture gate |
| Graph ownership guard | 37 original source/preset/test files verified against approved baseline exceptions |
| Canonical fixture guard | 12 canonical containers / 92 embedded byte records |
| Static historical DDL guard | 46 relation signatures / 104 FK checks; historical proposal counts remain 431 PostgreSQL / 139 SQLite statements |
| Build/tooling | Rust formatting, full all-target Clippy with warnings denied, Swift bridge, signed production bundle and generated product configuration checks |

Production UI verifies real creation, duplicate-click suppression, same-ID restart
and reopen, invalid name without intent, immediate cancellation, and the safe shell
capability. Shared UI retains 225 scenario transitions, 50 contrast pairs, 117 captures, native
Opening/Create callbacks and external Accessibility probes. Final captures and
27 production captures and command outcomes are recorded in the report; the shared fixture's historical
"no creation wiring" log describes its synthetic callbacks only.

Final saved-project captures: [Light](mockups/ui1/created-light.png) ·
[Dark](mockups/ui1/created-dark.png). These are native compositor captures of the
real production adapter using disposable data.

### 2026-09-15 native verification hardening

Production UI verification previously called `build-app.sh` against its default
interactive `.build/app` directory. Its ad-hoc signing step could replace the
Apple Development-signed Photara bundle and disable cloud authentication. The
verification runner now allocates a fresh `.build/production-ui.XXXXXX` directory
and owns both the production input bundle and verification bundle, Rust target,
helper build, generated bindings/product identity, and Swift caches. Interactive
build/signing/channel/output environment overrides are explicitly excluded.
`build-app.sh` supports `PHOTARA_APP_BUILD_ROOT`; the interactive default is unchanged.

The snapshot harness also checks the actual native host size after layout. On
macOS 27, `setContentSize(420×240)` produced a stable `420×230` host, and a compact
Create root could shrink its fixture to its ideal size. Fixtures now declare
their requested canvas within SwiftUI, keep NSHostingView directly attached to
NSWindow so native toolbars remain present, and adjust window dimensions from the
measured host delta. Both axes must match across two layout turns. The production
Library fixture additionally asserts that its native toolbar is populated.

This follows Photara's native-first policy: preserve the current host OS's real
window chrome, controls and layout behavior; do not substitute a fixed imitation
or hardcode titlebar dimensions. The requested canvas is test geometry only.
Functional or color-critical exceptions belong to separately justified product
requirements. No production appearance or Graph interaction behavior changed.

The two Graph review failures were consistent with a half-point native coordinate
shift: wheel pan differed by `0.5 × (1 − 1.6352352289236711 / 1.8)`, and the routing
knot differed by `0.5 / 1.8` world points. Native mouse/wheel construction now drains
pending layout before fixing window coordinates. The independent Graph oracle,
all gesture assertions/tolerances, seeds, fixtures and production sources remain
unchanged. The ownership guard permits only these explicit native-input changes.
Native focus must remain active/key across three loop turns before input; the
original immediate focus assertion remains and a stability assertion is added.

Final measured results on macOS 27.0 (`26A428`), Xcode 27.0 (`27A266a`), at
3008×1692 points / 2× scale:

- Full Graph: **18,086 assertions, zero failures**, recorded exit `0`; 12 native
  configurations and all 12×180 seeded actions. Log:
  `/tmp/ui1-native-graph-final-full/stdout.log` (and adjacent `exit-code`).
- Production: two final passes, **27 captures each**, including populated native
  toolbar assertions. Logs: `/tmp/ui1-native-production-final-5.log` and
  `/tmp/ui1-native-production-final-6.log`. Captures live in the printed isolated
  `production-ui.2NaFyw/production-snapshots-5` and `production-snapshots-6` roots.
- Shared UI: **117 captures**, 225 transitions, 50 contrast pairs and native
  Opening/Create accessibility probes pass: `/tmp/ui1-native-shared-final-3.log`.
- Product configuration: 7 tests pass. Ownership guard: 37 files pass, including
  unchanged production Graph/fixtures/oracle. Shell syntax and whitespace pass.
- The isolated build succeeds with deliberately invalid inherited channel/signing
  values and unrelated app/Rust/shared-generated output overrides; none of those
  caller output directories is created. Log:
  `/tmp/ui1-native-isolated-build-inherited-overrides.log`.
- All **20** signed interactive bundle file hashes match the pre-task inventory;
  the executable remains SHA-256
  `268ce9aa29c9becb1f8a56a71c84ef6e0957a049af3f17f0566c80280cb8ae3f`
  and Team ID remains `524GTA93Q3`.

Limitations and failed attempts are retained: an initial full attempt had 10
focus/HID-related failures (`/tmp/ui1-native-graph-full-1/stdout.log`); two runs
failed HID capability preflight; another full attempt had 6 failures before the
stable-focus change (`/tmp/ui1-native-graph-clean-3/stdout.log`). The final full run
above passes. Its subsequent separate wheel replay stalled before lifecycle output
and was stopped on request; no result exists for that replay or the queued knot
replay. Both original failing seeds completed inside the passing full matrix.
An earlier knot-only replay passed 630 assertions before the focus strengthening.
Native input checks still require uninterrupted desktop focus; physical trackpad
feel and other display configurations are not claimed. No additional native gates
or builds were run after the stop request.

### Reproduce the gates

Use the existing Apple Development desktop and operator provisioning profiles for
native signing, and an isolated Rust build target. No production app launch,
installed service replacement, or live database connection is needed.

```sh
cargo test --offline --workspace
cargo clippy --offline --workspace --all-targets -- -D warnings
cargo fmt --all --check
python3 scripts/verify_service_postgres.py --inventory /private/tmp/ui1-postgres.json
python3 scripts/verify_generation_two_fixtures.py
python3 scripts/verify_generation_two_schema.py
python3 scripts/verify_generation_two_naming.py
python3 scripts/verify_ui0_contract.py
python3 scripts/test_product_configuration.py
zsh scripts/test-native-authentication.sh
zsh scripts/test-service-process-furnace.sh
platform/macos/photara-app/verify-bridge.sh
platform/macos/photara-ui-tests/verify-production-ui.sh
platform/macos/photara-ui-tests/verify-shared-ui.sh
platform/macos/photara-graph-lab/verify-interactions.sh
```

Run the native Graph furnace separately from other foreground UI fixtures; preserve
its existing signing identity, permissions and assertions. Desktop/operator profile
environment variables are `PHOTARA_MACOS_PROVISIONING_PROFILE` and
`PHOTARA_OPERATOR_PROVISIONING_PROFILE`; Rust build target overrides are
`PHOTARA_APP_RUST_TARGET` and `PHOTARA_BRIDGE_RUST_TARGET`.

### Measured current schema

SQLite fixture: **79 domain tables, 768 columns, 221 foreign-key column entries,
184 indexes (including implicit indexes), 190 triggers, 15 ledger entries**, floor
5/5. Integrity check is `ok`; foreign-key check has no rows. The table/column/FK
counts exclude SQLite internals and the SQLx ledger table. This is an actual
migrated disposable native fixture, not a static inference from DDL.

PostgreSQL fixture: **59 tables, 606 columns, 1,138 constraints, 165 indexes,
122 triggers, 52 functions, 173 policies, 14 ledger entries, 726 migration
statements**. The private cluster is stopped after verification. No live Neon schema
or catalog was read or written to obtain these results.

## Publication and next slice

Implementation is reviewable in the working tree; **no commit, push, installation,
service replacement or live creation is authorized by this verification**. The
currently installed development service may still lack the new endpoint. Publish
the matching desktop/service versions only after explicit user authorization and
review of the local floor-5 migration boundary. UI2/UI3 and CXT4e second-Mac hydration
remain separate. The next requested **Library Lifecycle** gate must include exact
Library-name retyping, authorization/ownership, impact counts, transactional catalog
and reference cleanup without orphans, and proof that packages/source/archive files
are untouched.
