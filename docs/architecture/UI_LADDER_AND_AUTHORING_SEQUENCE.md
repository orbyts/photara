# UI ladder and authoring sequence

Status: UI0 accepted and UI1 Create Project visual checkpoint accepted, 2026-09-14.
Compact is the shipped default. Real creation wiring is pending.
**Verified:** all final Xcode 27 gates pass, including 15,644 Graph assertions with
zero failures. Delivery is an authorized normal fast-forward push.

## Accepted UI0 and UI1 visual checkpoint

Suhail approved the shared neutral ladder/lab foundation, then approved the rendered
Create Project design and chose **Compact** for the shipped default. The final
Opening is native: `windowBackgroundColor` with primary/secondary text, native sidebar,
buttons, selection and toolbar. It is the native exception before the authored ladder
begins in project content. Theme edits do not repaint Opening.

`CreateProjectPresentation.shipped` defines Compact once. Both the shared
`CreateProjectView` and Shell Lab use that default. Compact, Balanced and Spacious
remain available for Lab comparison; only Compact is the shared default. The sheet
shows Library, Project Name, Location, package preview, Choose, Cancel and Create.
Blank/whitespace names disable Create, and the package preview uses the trimmed name.
This is presentation validation, not complete filesystem or creation validation.

The shared view is production source compiled by the shared source manifest, but
only Shell Lab currently presents the new sheet. Its Choose action records fixture
intent, Cancel returns to Opening, and Create opens a disposable empty fixture Project.
The production creation route is not connected to this sheet. No real package,
database or cloud project-creation workflow is implemented by this checkpoint.

Next work must separately wire destination selection and atomic creation across
Project identity, initial Graph, catalog, `.photara` package and cloud projection,
with collision/permissions/restart/cancellation/rollback coverage. Library lifecycle
is a separate future slice: deleting a Library transactionally removes catalog
Project records/references, never `.photara` packages or external archives.

Commit and normal fast-forward push are authorized after the Xcode 27 checkpoint
gates pass. Force-push, real creation wiring and live data mutation are outside scope.

## UI0 bounded migration audit

Baseline: clean `ef3fc00` matching refreshed `origin/main`, including `45ddedf`.
There were no dirty files before editing. No Core, package, cloud or onboarding
behavior changes are in scope.

- `PhotaraTheme.swift` owns the only portable parser/resolver (Swift; there is no
  parallel Rust theme parser). At the baseline, schema 1 required 32 independently stored
  colors, including duplicate feature backgrounds. Keep schema 1 and its stable
  role names; canonicalize legacy alias values on import and omit them on export.
- Foundation/Primary/Inset remain `surface.canvas/panel/elevated`. The exceptional
  custom-control fill aliases Inset. Graph background maps to Foundation at its
  composition root, node fallback to Inset, grid to subtle border and selected
  outline to focus. Gallery background/cell map to Primary/Inset; thumbnail and
  full-image wells explicitly use the dedicated photograph reference surround. Category colors
  remain small functional affordances. No feature-local background slots are added.
- Theme Lab currently defaults to an optical Glass experiment and compiles a Rust
  application fixture. Replace that editor composition with paired semantic fields,
  a shared ladder specimen and the real Opening view, without a bridge or optical
  authoring controls. Preserve the experimental file as inactive historical tooling.
- Shell Lab currently exposes launcher colors/materials in completed project
  scenarios, saves a second theme draft and writes/removes Theme overrides. Remove
  that ownership; expose controls by the selected scenario's typed scope and use a
  read-only shared Theme store. Retain typed feature views and production composition.
- At baseline Opening painted `surface.canvas`. The final approved Opening uses
  native window/text colors and an ordinary native Create action. The global custom
  accent override is removed. Opaque equal-channel neutrals remain in the authored
  project ladder; native Opening is its explicit exception.
- User clarification during UI0 explicitly standardizes the actual Graph background.
  Its shared background renderer now gives the mapped Theme role precedence over
  legacy preset/lab literals. Graph Lab replaces only that ColorPicker with a
  read-only Foundation label and consumes the read-only live Theme store. All other
  Graph rendering, controls, interactions, presets and tests remain byte-identical.
  Legacy Graph background literals remain readable but inert under the shared Theme;
  no saved author preferences are rewritten by migration.
- Existing shared/production suites render native compositor PNGs and verify typed
  callbacks, session isolation and graph digest purity. Extend them with alias,
  neutrality/contrast, scenario isolation, Opening accessibility/action and raster
  assertions; run the unchanged native Graph regression suite separately.

Photara authors a small adaptive neutral system once and applies it by semantic
ownership. Feature views do not carry independent background palettes. The native
macOS sidebar, titlebar, toolbar, controls, selection geometry, vibrancy and Liquid
Glass remain system-owned so a future SDK rebuild inherits the current macOS design.

## Three-step neutral ladder

The authored ladder has three reusable Light/Dark steps:

1. **Foundation** — application or current-scope content base.
2. **Primary** — a primary Library Browser, Project Graph or peer module surface.
3. **Inset** — a genuine nested canvas, browser, preview well or grouped work area.

The ladder resets at a new independently owned composition root instead of creating
ever more grey values. A Graph and a Node Work Surface are peer composition roots;
each may use Foundation → Primary → Inset internally. Ownership remains explicit in
titles, breadcrumbs, icons and accessibility labels—shade is supportive, never the
only scope signal.

The existing portable roles map as follows until a separately approved schema change:

| Authoring label | Existing token | Use |
| --- | --- | --- |
| Foundation | `surface.canvas` | application and scope content base |
| Primary | `surface.panel` | primary module or owned composition surface |
| Inset | `surface.elevated` | genuine nested content/editor/browser well |
| Custom control compatibility | `surface.control` → Inset | no independent fill; prefer system controls |
| Image surround | `editor.surround` | fixed color-neutral photograph reference surround |

`graph.background`, `gallery.background`, and similar compatibility roles must resolve
through an explicit mapping to the ladder or a specialized reference role. They are
not separately authored copies of the same background. Node-category color remains
limited to icons, small port/status affordances and focus feedback. Photographs never
sit on a colored surface.

The Opening primary action is an ordinary native action on the native window surface. It must not
paint a blue or tinted content area. System accent remains appropriate for native
selection, focus and small functional affordances.

## Lab responsibilities

- Preserve **Graph Lab** as the accepted owner of Graph canvas, nodes, ports,
  connections and floating Graph controls. It consumes mapped shared tokens but is
  not redesigned by this work.
- Rebuild **Theme Lab** as the only editor for paired Light/Dark ladder values,
  reference surround, text, borders, selection and semantic status colors. Show the
  complete three-step ladder together and validate neutrality and contrast.
- Simplify **Shell Lab** into an integrated scenario browser. It selects Opening,
  Create Project, Project Browser, Project/Graph, Inspector and Node Work Surface
  states and previews the exact production views. It authors shared geometry and
  behavior only; it does not duplicate colors or feature internals.
- Feature labs inherit the ladder read-only and expose only their typed content,
  interactions and meaningful states. Completed-context controls never leak into the
  currently selected scenario.
- Lab overrides remain developer presentation state. They never enter Core, a
  Library, Project package, Graph digest or cloud synchronization.

## Approval-sized implementation slices

### UI0 — ladder and lab foundation

Freeze the token mapping, remove chromatic casts from the default neutral surfaces,
rebuild Theme/Shell Lab responsibilities, and retain the accepted native Opening window/text colors without a
custom content tint. Render Light/Dark plus narrow/standard screenshots from the
real production views. Obtain approval, assemble into Photara and run visual,
accessibility, contrast and regression checks. Do not change Graph content.

### UI1 — Create Project

Freeze the typed create contract, then mock and approve the native UI before wiring.
The first draft selects Library, project name and package destination; shows the
default `~/Pictures/Photara/Projects` and a user-selected root such as Whisk; validates
name/path/collision/permission state; and explains that the package stores authored
state and manifests rather than the external archive.

The production gate is one staged operation that creates the Project identity,
initial Graph, Project Catalog record and `.photara` package (a folder on platforms
without package presentation), then either commits all visible coordinates or reports
a recoverable incomplete operation. Verify SQLite, Neon projection, on-disk package,
restart, cancellation, duplicate click, unavailable volume and rollback behavior.

### UI2 — Project Browser and package recovery

Mock, approve and implement typed grid/list/card browsing, search/sort, recent state,
Open Package, missing/moved package, Locate, move-through-Photara and duplicate identity
handling. Reuse browser presentation primitives without introducing an untyped Core
Gallery item. Library database import/export remains a later bounded backup slice;
opening or locating a Project package is not database import.

### UI3 — Project window and Graph integration

Open the created Project into the accepted Graph canvas. Add multiple saved Graph
selection, empty/first-node/running/success/failure states, project status and durable
save/reopen. Preserve Graph Lab behavior and apply only the shared outer hierarchy.

### UI4 — node authoring

Approve and implement the searchable categorized node catalog, ordinary built-in
Layout and source nodes, typed port connection, Inspector selection and run diagnostics.
Node category color appears only in permitted small affordances.

### UI5 — Inspector and Node Work Surfaces

Approve the Graph-owned dockable/floating Inspector and a Layout-owned Work Surface.
The Layout Work Surface composes its own Asset Browser/Gallery as a split child using
the same local ladder mapping and the fixed photograph reference surround. Test resize,
dock/float, multiple Layout nodes, selection, authoring, save/reopen and accessibility.

### UI6 — Library browsers and contextual pickers

Complete People, Location Kinds, Locations and Project Info as application-owned typed
Library browsers. Nodes reuse host-owned typed pickers inline without turning those
records into nodes or an untyped Gallery domain.

Each UI slice follows: contract → real Light/Dark raster states → user approval → same
production source in owning lab → Photara assembly → interaction/persistence/
accessibility/regression proof → focused commit. Heavy cross-boundary work uses Astra;
bounded view authoring and ordinary verification may use the default model.

## UI0 implementation inventory and compatibility

UI0 and the UI1 Compact visual checkpoint are accepted; real UI1 creation wiring is pending.
The presentation contract stays in Swift/shared JSON; no Rust, Core, package,
Library schema, graph digest, synchronization or onboarding behavior is changed.

### Frozen bundled ladder

| Role | Light | Dark |
| --- | --- | --- |
| Foundation | `#EEEEEE` | `#202020` |
| Primary | `#F7F7F7` | `#292929` |
| Inset | `#FFFFFF` | `#333333` |
| Photograph reference surround | `#808080` | `#808080` |

These are opaque, equal-channel sRGB values. New composition roots reuse Foundation;
there is no depth counter or derived fourth grey. Native macOS materials may retain
the system's own optical tint; authored content surfaces do not inherit it.

### Complete schema-1 alias mapping

| Compatibility role | Canonical owner | Active use |
| --- | --- | --- |
| `surface.control` | `surface.elevated` | exceptional custom fills; native controls remain native |
| `graph.background` | `surface.canvas` | Foundation at the Graph composition root |
| `graph.node` | `surface.elevated` | Graph's unconfigured node fallback |
| `graph.grid` | `border.subtle` | unconfigured grid fallback |
| `graph.node-selected` | `border.focus` | unconfigured selected outline fallback |
| `gallery.background` | `surface.panel` | browser chrome and text on Primary |
| `gallery.cell` | `surface.elevated` | card chrome and labels on Inset |

Thumbnail, full-image and Layout photograph wells use `editor.surround` explicitly.
Graph's accepted configured node/grid/port/noodle/control styling remains unchanged.
Only its background renderer reverses precedence so the shared semantic role wins
against old `graphBackground` literals. The Graph preset and preference payloads
are retained verbatim. Legacy background literals are ignored under a Theme; hosts
without a Theme retain the old fallback. No saved Graph preferences were rewritten.

The parser accepts old schema-1 literal aliases, validates their syntax and drops
them during decode. Canonical owner values always win. New exports omit all seven
aliases (25 authored slots). Unknown roles and missing canonical roles are rejected.
The UI lists aliases read-only. Older chromatic authored ladder values load with
neutrality warnings rather than being silently recolored; Apply requires neutral,
ordered levels and passing contrast. The CLI retains its explicit use command and
reports warnings during validation. This is presentation compatibility only.

### Production and Lab source ownership

- `photara-theme/Sources/PhotaraTheme.swift`: three typed levels, canonical mapping,
  schema-1 normalization, neutral/order warnings and 50 contrast checks over the
  three ladder levels plus custom selection. The default JSON removes duplicate
  roles and uses neutral surfaces/text/borders; status colors meet the new
  worst-surface contrast target.
- `photara-ui-foundation/Sources/ThemeLadderSpecimen.swift`: one shared paired raster
  specimen. `ThemeStore.swift`: read-only live Theme consumption in production and
  all labs, with explicit override isolation in test hosts.
- Theme Lab: paired fields for the complete authored palette, read-only alias table,
  neutral/contrast feedback, ladder and exact production Opening preview, Theme-only
  Apply/Remove. The app no longer instantiates AppModel, links a bridge or compiles
  the retained historical Glass experiment.
- Shell Lab and `ShellFixtures.swift`: scenario-specific geometry/identity controls;
  no color pickers, Theme draft/export/apply/remove or retired launcher optics.
  Fresh disposable sessions and view identity reset per scenario. Old Shell theme
  drafts are ignored. Recents compatibility fixtures remain test inputs but are
  omitted from the browser. Create Project has the accepted UI1 preview; Project Browser awaits UI2.
- `OpeningLibraryView.swift`: native window background/text, ordinary native Create
  action and explicit accessible container/button identities. The native sidebar and
  cloud model are unchanged. `PhotaraMacApp.swift` and lab hosts stop overriding the
  system accent globally. Custom focus/selection tokens remain available to content.
- `GalleryCard.swift` and `GalleryFullImageView.swift`: photograph wells explicitly
  consume the reference surround; card/browser labels remain on the ladder.
- Graph has exactly three source deltas: semantic background precedence in
  `GraphPresentation.swift`, a read-only background field in `GraphLabView.swift`,
  and live read-only Theme injection in `GraphLabApp.swift`. Its nodes, ports,
  connections, geometry, controller, event surface, fixtures, resources and tests
  otherwise remain exact. `verify_ui0_contract.py` checks 33 original Graph files
  against the baseline; the final checkpoint additionally permits the narrow macOS 27
  verification launch/input adaptation described in the evidence section.
- UI verification adds parser/alias/neutral/contrast tests, all 225 ordered scenario
  transitions, native Opening accessibility and action checks, paired size captures
  and the ladder specimen. Test apps use the native launch callback for reliable
  accessibility registration. The external probe inspects only the synthetic app's
  windows. Captures use bounded native windows and up to three compositor attempts;
  no UI is reimplemented as mock HTML or painted image data.

### Accepted images and limitations

The images below are real native compositor captures of shared production views
with disposable fixture state. Suhail approved UI0 and the Create Project design,
selecting Compact. Opening uses native colors; the ladder specimen shows the authored
project surfaces. Pixel checks require the native Opening content to remain flat,
without pinning native macOS colors to the authored palette. All six ladder samples
remain pinned to the authored neutral levels.

Human VoiceOver navigation, physical trackpad feel and alternate display/hardware
conditions remain hands-on checks. System Reduce Transparency/Increase Contrast
preferences were not toggled. These fixtures do not prove live authentication,
cloud writes, package creation or complete path/collision/permissions validation.

| Capture | Content points | Evidence |
| --- | --- | --- |
| Opening Light, narrow | 760 × 560 | [Opening Light narrow](mockups/ui0/opening-light-narrow.png) |
| Opening Dark, narrow | 760 × 560 | [Opening Dark narrow](mockups/ui0/opening-dark-narrow.png) |
| Opening Light, standard | 1280 × 820 | [Opening Light standard](mockups/ui0/opening-light-standard.png) |
| Opening Dark, standard | 1280 × 820 | [Opening Dark standard](mockups/ui0/opening-dark-standard.png) |
| Paired three-level specimen | 880 × 460 | [Ladder](mockups/ui0/ladder.png) |
| Create Project Compact, Light | 470-point content width | [Compact Light](mockups/ui1/create-project-compact-light.png) |
| Create Project Compact, Dark | 470-point content width | [Compact Dark](mockups/ui1/create-project-compact-dark.png) |

Balanced/Spacious comparisons remain in Shell Lab and are captured by shared
verification. The retained Compact captures show the shared sheet content in a
native test window; Shell Lab also exercises it as a sheet over Opening.

### Verification and raster evidence

Checkpoint verification runs on macOS 27.0 (26A428), Xcode 27.0 (27A266a), macOS SDK
27.0 and Swift 6.4. The source, raster and log hashes are recorded in
[UI0_VERIFICATION.json](UI0_VERIFICATION.json).

| Gate | Final result |
| --- | --- |
| Shared UI | Pass; 225 transitions, 50 contrast pairs, native Opening and UI1 callbacks, 117 PNGs |
| Production UI | Pass; existing Core/adapter/persistence checks, 25 PNGs |
| Bridge | Pass; UniFFI fixtures, progress and cancellation |
| Theme | Build and CLI default-palette validation pass |
| Shell / Graph Lab | Xcode 27 builds pass |
| Production build/signature | Pass |
| Source guard / formats / raster checks | Pass; all 37 guarded Graph files match allowed baseline changes |
| Graph runtime, unchanged behavioral assertions | **Pass: 15,644 assertions, 0 failures; exit 0; 12 configurations and 12×180 seeded actions** |

The narrowly authorized macOS 27 harness adaptation preserves the original SwiftUI
WindowGroup and every behavioral matrix/assertion/oracle. LaunchServices supplies
the normal application-open event; the host waits for `applicationDidFinishLaunching`
and a running AppKit loop. The runner requires an atomic exit-code file written by
the actual finish path, so startup failure or a crash cannot report success.

`GraphNativeInput` preserves already-correct native coordinates when setting a mouse
button. A native probe showed the old unconditional `CGEvent.location` rewrite itself
invalidated window association on SDK 27. Legacy normalization remains only for an
incorrect initial point; the original 0.001-point coordinate bound and window/button/
modifier checks remain intact. An 81-delivery move/resize/fractional-point preflight
runs before the original Graph matrix, using actual NSWindow-to-NSView delivery.

The verification app has a stable Apple Development identity, bundle ID
`com.photara.graph-lab.verification`, and screen-capture purpose string. Two builds
preserved its designated requirement (SHA-256
`ec79c9c10d93f1a6f5ac398a76b6af7086bef30b08801de56a1a6b507279253b`)
and team `524GTA93Q3`. Validation checks the selected certificate fingerprint,
Apple anchor, team, bundle ID and stored designated requirement; it does not rely
on the `Authority` display string, which can be unavailable on SDK 27. Real negative
checks rejected wrong-certificate, wrong-team, ad-hoc and unsigned code.

The old Accessibility grant was tied to an ad-hoc cdhash. Targeted TCC logs proved
that identity mismatch; the user explicitly approved resetting only Accessibility
for this bundle, then manually granted the fresh entry. Screen Recording was
refreshed separately. Both permissions survive the final same-identity rebuild.
The runner never resets TCC or grants consent. The one-time
[`verify-permissions.sh` workflow](../../platform/macos/photara-graph-lab/README.md#explicit-one-time-request-mode-no-rebuild)
launches the exact built binary; normal verification is prompt-free.

The capability probe receives tagged down/drag/up events through the same public
HID route as the unchanged native slider tests. It also captures only its disposable
window through ScreenCaptureKit and the existing `screencapture -l` route. Capture
validation converts to sRGB, checks uniform spatial patches and their relative hue/
contrast, and does not assume nominally-zero color channels remain zero after color
management. Two real captures and a lifted-channel fixture pass; blank, grayscale,
reversed, transparent and corrupted fixtures fail. This checks the new diagnostic,
not any production Graph renderer change.

The optional self-targeted AX action remains an API diagnostic: its observed callback
is logged separately from its nonzero AX error and is never labeled API success.
It cannot establish general input authorization and is not part of Graph's input
path. Gate readiness requires native authorization, actual HID receipt and both
capture validations; the original Graph interaction assertions remain the final oracle.

Historical failures remain in the log inventory: the unadapted run had 9,742
assertions/181 failures from invalid coordinates; an intermediate explicit hosting
view had 15,644/1 (native zoom hit priority) and was removed; the pre-grant
LaunchServices run had 9,944/49 from denied capture/input. Those failures are not
claimed as passes, and no behavioral assertion was changed to accommodate them.

The source guard checks all 33 original Graph files against `ef3fc00`, allowing the
three UI0 Theme edits plus the explicit verification launch/preflight/mouse adapter
spans and two added compilation inputs. The two new verification files are recorded
separately in the same hash inventory (37 files total, including the explicit permission launcher and capability probe). All behavioral matrix,
branch/overview assertions, fixtures, presets and independent gesture oracles retain
their original source. The production renderer/controller/event surface is untouched
apart from the previously approved shared background precedence.

Historical failing run: `/private/tmp/photara-checkpoint-graph.log`. Native diagnosis:
`/private/tmp/photara-event-probe.log`. Coordinate preflight:
`/private/tmp/photara-graph-preflight.log`. Adapted full matrix:
`/private/tmp/photara-harness-graph-full.log`. The inventory retains their hashes and
final counts. Other checkpoint logs use `/private/tmp/photara-checkpoint-` names.

The shared suite covers canonical/legacy aliases, 50 contrast pairs, all 225 scenario
transitions, native Opening accessibility/actions, and Compact default/blank draft/
package preview/Choose/Cancel/Create fixture callbacks. It captures all three UI1
styles in Light/Dark. The production suite retains adapter/Core edit/undo/redo, HDR
proxy, disposable SQLite, thumbnails, snapshots, save/reopen and graph-digest checks.
The Graph ownership guard compares the original 33 files with `ef3fc00`, allowing
only the three UI0 Theme edits and the explicitly authorized test-harness adaptation.
All production behavior and behavioral assertions/oracles remain unchanged.

Earlier macOS 26 verification passed shared/production/bridge and 15,562 Graph
assertions. That earlier Graph run first encountered native focus/slider failures
while other windows were active, then passed isolated replays and the full suite.
The Xcode 27 checkpoint reruns these gates with Graph isolated from other native
verification. Initial UI1 probes showed that a direct AX value write changed the AppKit field
without updating SwiftUI's draft. The final probe types native key events targeted
only to the fixture process, refreshes the AX tree and waits for enablement/package
text. Native typing and all four Light/Dark Create/Cancel paths pass. The production
fixture also stopped relying on closed screenshot windows retaining an `onChange`
observer: its existing new-project intent is now sent while `EditorSessionView` is
mounted. These changes repair test fidelity without changing product behavior.
