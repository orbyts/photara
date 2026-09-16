# Photara Shell Lab

Shell Lab browses integrated scenarios using the exact production `ApplicationShell`,
`OpeningLibraryView` and typed feature views. The preview window is the production
composition; the authoring window selects fixtures, appearance, size and shared
geometry. No bridge, database, source folder, provider or network is required.

Read [`../DESIGN_LANGUAGE.md`](../DESIGN_LANGUAGE.md) and the
[UI0 contract](../../../docs/architecture/UI_LADDER_AND_AUTHORING_SEQUENCE.md).

```sh
platform/macos/photara-shell-lab/build-shell-lab.sh
open 'platform/macos/photara-shell-lab/.build/Photara Shell Lab.app'
```

Opening exposes only its applicable scenario information and native actions. Project
scenarios expose shared module gutter/insets/radius, toolbar identity and pane/header/
status geometry. Legacy launcher tint, material, hero and button editors are removed.
No palette controls, Theme export, Theme draft or Theme override writes remain here.
Theme is inherited read-only and live-reloaded from the shared Theme development store.

Every scenario transition creates a fresh disposable `EditorSessionModel` and resets
the production view's identity. Selections, pending catalog requests, modal state and
completed-context controls cannot carry into a different scenario. Existing collapsed/
expanded recents fixtures remain regression inputs, but are omitted from the browser
because Opening does not yet implement that browser. The UI1 Create Project scenario
renders the accepted shared `CreateProjectView`. Compact is the shipped default;
Balanced and Spacious remain comparison options. Choose records fixture intent,
Cancel returns to Opening, and Create transitions to an empty fixture Project only.
Real destination/package/catalog/cloud creation is not wired. Project Browser is UI2.

Project/Graph, empty/first-node, Inspector selection-cleared, assets, Layout Work
Surface, Review, evaluation, diagnostics, compact, saved and syncing fixtures use
production modules. Fixture actions are recorded locally. Graph's renderer, noodles,
ports, interactions and floating controls stay in Graph; its background now inherits
Foundation at the Graph composition root. Feature modules remain typed.

**Identify controls in preview** labels ownership and pauses preview interaction.
Opening labels point to native macOS controls and window colors; project labels
point to shared geometry, Theme or the feature's own Lab.

The lab saves a Shell draft in `com.photara.shell-lab`. **Apply to Photara**, **Remove
Photara Override** and **Export Shell Preset** now affect only Shell presentation.
The old Shell theme draft is ignored, not deleted. Older Shell JSON fields for the
retired launcher remain readable for compatibility and are not used by Opening.
System sidebar, titlebar, toolbar, selection geometry, vibrancy and optics stay native.

```sh
python3 scripts/verify_ui0_contract.py
platform/macos/photara-ui-tests/verify-shared-ui.sh
platform/macos/photara-ui-tests/verify-production-ui.sh
```

The shared checks cover every scenario transition, the real Opening's accessibility
and actions, and the ladder/parser/contrast contract. Graph's native interaction suite
runs separately from other UI capture windows to preserve its foreground pointer tests.

## Library Lifecycle — LL0 review

Choose **Library Lifecycle** in the authoring window. Its state picker and Light/Dark
control show the accepted native layout: active-Library sidebar and identity menu, create/rename/switch sheets, removal
impact and exact-name entry, final destructive confirmation, blockers, uncertain
response, local reconciliation and receipt. The contract is
[LIBRARY_LIFECYCLE.md](../../../docs/architecture/LIBRARY_LIFECYCLE.md).

These new views are **accepted lab-only presentation**, not production lifecycle
operations. All actions are in-memory fixture changes. The default/last Library
policy and Cloud/On this Mac creation choice are part of the accepted LL0 contract. No package, database,
network or authentication code is linked by the dedicated lifecycle verifier.

```sh
platform/macos/photara-shell-lab/verify-lifecycle.sh --build-only
'platform/macos/photara-shell-lab/.build/lifecycle-verification/Library Lifecycle Verification.app/Contents/MacOS/LifecycleChecks' /private/tmp/photara-lifecycle-review
```

The verifier uses its own native windows and compositor captures. It checks fixture
confirmation/blocked-state behavior; it does not establish production authorization,
cloud correctness or database integrity. It requires macOS window/capture access.
Never run a production app or change credentials to obtain these review images.


## Revised Library switcher

In **Library Lifecycle**, choose a **Switcher context** in the authoring window.
The bottom-left native menu retains the avatar + user display name trigger (or
On This Mac for a local/signed-out identity). Left-click it to see Library and
account context inside the native menu. The
sidebar contains only active-Library content. Open the menu for grouped Libraries,
New Library, Library Settings and account actions. Library Settings contains the
existing management sheets. There is no Library Save or Add Account action.

Contexts cover cloud/local, duplicate names, offline projection, signed-out/unsigned,
unsaved Project and stale target recovery. Ordinary selections apply automatically
after fixture checks; only the unsaved Project context presents Save and Switch.
All state is synthetic. Sign In and Account Settings record existing-flow intent;
no credentials or real account state are accessed. The dedicated verifier additionally
uses an external accessibility helper to inspect the native menu and send keyboard
input to its own synthetic process. No permission grants/resets are automated.


Current [Light/Dark review images and verification](../../../docs/architecture/LIBRARY_LIFECYCLE.md#current-identity-menu-review-images-and-evidence)
use the user-identity trigger. The earlier Library-name trigger was discarded.
The command below reruns only the native keyboard path when diagnosing input:

```sh
'platform/macos/photara-shell-lab/.build/lifecycle-verification/Library Lifecycle Verification.app/Contents/MacOS/LifecycleChecks' /private/tmp/photara-identity-keyboard --keyboard-only
```

The normal full invocation remains the review gate; targeted checks are not a substitute.

The bottom identity button opens its native AppKit menu **above** the row. The
verification checks popup screen bounds for every context in Light and Dark.
