# Photara Graph Lab

Graph Lab is a small macOS developer utility for evaluating Photara's reusable
Graph presentation primitives without opening a project or building the Rust
bridge. It owns fixture data and temporary authoring controls only.

Follow the shared [`DESIGN_LANGUAGE.md`](../DESIGN_LANGUAGE.md). Graph Lab owns the
canvas and its accepted floating rail/zoom glass; Shell owns the surrounding module
base, title, selection tint, placement and resize behavior. Do not add a second module
frame here.

UI0 standardizes only the canvas background: it inherits the shared Foundation
at the Graph composition root. Theme Lab is the single editor for that paired color;
Graph Lab shows a read-only value and live-reloads shared Theme overrides. Legacy
Graph background literals remain decodable but the shared semantic role takes
precedence. The accepted pattern, nodes, ports, noodles, interactions, floating
controls and their complete test suite are preserved. The remaining Graph styling
controls below remain Graph-owned; they do not create extra shared ladder levels.

Node headers follow the reusable
[Graph Node Design Language](../photara-graph/NODE_DESIGN_LANGUAGE.md): a
free-standing 28-point outline icon centered against a tight title/category
stack. The icon alone receives an adaptive functional-category color; the title
and category label remain neutral. SVGs are cached native template images, so
node movement, pan, and zoom do not regenerate paths. Presentation metadata is
resolved separately from the graph document and interaction controller.

The current slice compares procedural major/minor background patterns, native
glass treatments, node corner radius, round versus pill ports, port edge
offset, and one stable node-shadow model. There are deliberately no
status indicators or Graph semantics.

Lines, Dots, and Crosses share aligned minor and major phases while the camera
pans or zooms. Graph Lab authors their colors, opacity, size or line width, and
major interval independently. Light and Dark each retain a complete authored
palette for the minor and major grid, noodles, node glass tints,
port-bead tint, and node text. Semantic port hues remain shared concepts while
each appearance authors a native brightness adjustment. Dense minor marks fade
before they alias.

The scene uses one-, three-, and six-row node definitions with live port-to-port
connections. Pressing an output immediately starts a noodle. Dropping near a
compatible input connects; dropping on empty canvas cancels. Outputs fan out,
inputs accept one incoming connection, and identical pairs are idempotent.
Dragging a connected input rewires its existing connection; an invalid or
cancelled rewire preserves its identity and routing knot.

Hold `Y` and slice to cut all crossed noodles. Option-click a noodle to add a
routing knot. Drag from the knot to branch to another input; Option-drag moves
the shared point and all of its attached noodles. Click a noodle or knot and press Delete,
or use the native right-click/Control-click menu. Input-port menus list compatible
outputs from the document. Straight and curved noodle styles remain available.

Empty-canvas dragging, middle-mouse dragging, and precise two-finger scrolling
pan the graph. Mouse-wheel zoom and trackpad magnification preserve the pointer
anchor. The bottom slider zooms about the canvas center. All graph geometry,
text, ports, knots and hit targets use the same camera. The floating control
retains Apple's interactive Regular Liquid Glass capsule and native material
fallback for Reduce Transparency.

The translucent overview shows the entire graph and a viewport rectangle. It
fades in at zoom start and fades out after 650 ms of inactivity; held slider or
pinch gestures keep it visible until they end. The Overview section at the top
of the sidebar offers Show While Zooming (default), Always Show, and Never Show,
all four corners, a relative size control, and corner radius. Its default width
is 16% of the graph window, bounded to 144–360 native points; native display
scaling handles Retina resolution. The frame matches the graph canvas aspect
ratio and follows window reshaping immediately. Bottom placements clear the zoom controls.
It does not intercept graph input. Save Preferences persists all overview choices
alongside the existing palettes, with defaults for older preference payloads.

The narrow floating tool rail groups camera controls, selected-noodle actions,
and pinned native-node shortcuts. Its icons are flat while its container uses
Apple's native Liquid Glass, with the semantic material fallback when Reduce
Transparency is enabled. A top-center rail is horizontal; left and right rails
can align to the top, center, or bottom, switching to two columns only when their
vertical proposal is too short. The Graph Lab authoring sidebar exposes corner
radius, light/dark shadow opacity, blur, and vertical offset. The Settings window
contains user preferences for rail visibility/placement, overview behavior, and
knife cursor size. Hiding the rail does not disable keyboard, pointer,
contextual-menu, or zoom behavior. The passive gesture guide appears only while
the rail is hidden. When the rail is visible, native hover help describes each
tool and includes its available shortcut or pointer gesture. The command menu
shows graph gestures and shortcuts. Native node types can
be pinned or removed there, or removed from a shortcut's contextual menu.
Clicking a shortcut inserts an ordinary node DTO at an unoccupied position near
the viewport center; it does not create a separate node interaction path.

Selection remains one typed value: at most one node, noodle, or routing knot is
selected. Contextual rail actions therefore apply only to the current selection.
Add Knot appears for a selected direct noodle, Delete Knot for a selected routing
point, and Disconnect for a selected noodle.

The tool rail and overview are mutually exclusive overlays. If both request the
same occupied corner, the overview is presented in the opposite corner. A
left-bottom rail also moves the passive gesture hint away from its hit region.

A native floating tool palette sits at the leading edge of the canvas. It uses
the same controller commands as the canvas responder instead of duplicating graph
interaction state. Center and zoom commands are always available. The contextual
slot offers Add Routing Knot only for a selected direct noodle, Delete Routing
Knot only for a selected knot, and Disconnect only for a selected noodle. The
commands menu documents the corresponding native gestures and shortcuts. Only
one node, noodle, or routing knot can be selected at a time. When vertical space
is constrained, the palette falls back to a compact two-column layout.

The flat knife artwork is the shared SVG for the palette and custom cursor. Its
lower blade tip is the cut hotspot. The standard macOS Settings command opens a
Graph Tools pane with a 16–32 point cursor-size control; the default is 20 points.
The setting is persisted by macOS independently of graph documents and visual
palette exports.

One AppKit canvas responder captures each pointer sequence. Its interaction
controller owns node drag, pan, wire/rewire, knife and knot previews. SwiftUI
renders those previews and the document without graph drag gestures or delayed
mouse-up callbacks. Escape, tool changes, focus loss and viewport changes cancel
uncommitted motion. Node positions and every attached endpoint derive from the
same preview in the same frame. Axis sweeps from the last accepted position keep
the authored non-overlap gap, including large/coalesced moves and boundary rounding.
The authoring form does not observe per-frame pointer state.

Node rendering, port geometry and hit testing share reusable definitions. New
nodes require fixture/document data, not gesture code. Selection uses the saved
stroke, with no additional shadow state. Native glass remains on the connected
port beads and zoom control; the accepted flat node rendering and separate light
and dark palettes are preserved. Node-shadow blur and vertical offset are shared,
while opacity is authored per appearance.

The node silhouette has no port recesses. The Assets-to-Input fixture noodle
terminates beneath its ports. A disconnected port is only a small flat semantic
dot that remains fully opaque; its saturation, optional same-color stroke,
stroke width, and Light/Dark brightness can be authored. Connecting keeps that
dot at the same size and exact bead center, raises its separately authored
Light/Dark brightness, and adds the native glass bead beneath it. A separate
semantic-colored shadow belongs only to the connected bead and provides native
opacity, blur, and vertical-offset controls. There is no connection ring or
state animation. The node shadow is applied only to the node surface, so text
and ports never inherit or duplicate it.

The node body can independently use a native flat fill or native Liquid Glass.
Flat mode keeps the port beads as Liquid Glass and provides one node color plus
a selected-node stroke color for each of Light and Dark, independent of the
graph background. Stroke width is shared across appearances. Selection is
communicated by the stroke rather than a second shadow state.

Save Preferences stores the current appearance, both complete appearance
palettes, and the visual controls in Graph Lab's local, versioned `UserDefaults`
payload. It restores them on the next launch and migrates the earlier shared
palette into the appearance in which it was saved without losing the user's
choices. Camera pan/zoom, node positions, selection, and gesture state are not
saved. This developer convenience is intentionally separate from the future
Photara Lab import/export format and never enters a project document.

```console
platform/macos/photara-graph-lab/build-graph-lab.sh
open "platform/macos/photara-graph-lab/.build/Photara Graph Lab.app"
```

The complete production canvas and node renderer live in
`platform/macos/photara-graph/Sources`; Graph Lab and Photara both compile and
render those same sources. After saving an accepted visual iteration, explicitly
promote it into their versioned shipped preset with:

```console
swift platform/macos/photara-graph-lab/export-shared-preset.swift
```

Review and commit the resulting `photara-graph-presentation-v1.json`. Rail
visibility/placement, overview policy/position, pinned shortcuts, and cursor
size remain per-application user preferences and are never exported.

## Interaction engineering and verification

[INTERACTION_MODEL.md](INTERACTION_MODEL.md) describes the event lifecycle and
serializable document boundary. [Tests/README.md](Tests/README.md) describes the
native acceptance matrix and seeded random gesture harness, including exact
failure replay. Run both with:

```console
platform/macos/photara-graph-lab/verify-interactions.sh
```

The verification app uses a separate preferences domain and reads the existing
visual payload without saving to the author's domain. Reports, compositor
screenshots, and seed traces are written to `/tmp/photara-graph-verification`.

## macOS 27 verification compatibility

The behavioral matrix and independent gesture oracle remain unchanged. The
verification-only `GraphVerificationHost` preserves the original WindowGroup and
uses public `.defaultLaunchBehavior(.presented)` / `.restorationBehavior(.disabled)`
settings to prevent a restored windowless app from skipping its test task. The
runner uses LaunchServices (`open -n -W`) for the normal application-open event and
requires an atomic exit-code file written by the actual test finish path; a missing
result fails the command even if `open` returns success. Per-run stdout/stderr and
status files are isolated under `.build/verification/run.*`. `--build-only` compiles
without launching. The preflight environment flag is explicitly forwarded.
`GraphNativeInput` preserves a correct NSEvent window-local point when changing its
mouse button number. The former unconditional `cg.location` rewrite invalidates
that point on macOS 27. Legacy normalization is attempted only when the initial
bridged point is actually incorrect; every result must still match the requested
coordinates, window, button, type and modifiers before NSWindow dispatch.

An 81-delivery preflight checks actual NSWindow → NSView delivery at fractional
points, through three moved/resized window configurations, all three mouse buttons,
and Option/Control modifiers. It fails before the behavioral matrix if delivery is
wrong. Physical HID slider tracking remains unchanged. To run only the preflight:

```sh
PHOTARA_GRAPH_PREFLIGHT_ONLY=1 platform/macos/photara-graph-lab/verify-interactions.sh
```

The UI0 source guard permits only the explicit launch/preflight/input adapter edits
in the original harness and compares every behavioral assertion and oracle with
`ef3fc00`. Production Graph changes remain limited to the three approved shared
Theme-consumption edits.

### Stable signing and one-time native permissions

`verify-interactions.sh` signs the verification app with a valid Apple Development
identity from the login Keychain. It deterministically selects the lowest sorted
SHA-1 certificate fingerprint; set `PHOTARA_GRAPH_VERIFY_SIGNING_SHA1` to a specific
valid Apple Development fingerprint when multiple teams/certificates are installed.
No personal certificate name, private key, profile, or secret is stored in source.
The signature is verified and its team and designated requirement are recorded in
`.build/verification/signature.txt` and `designated-requirement.txt`. Keep the same
identity and app location for subsequent runs. Changing signing identity/team or
moving checkouts can require new macOS grants; ad-hoc rebuilds do not retain this
identity. This development signature is for the verification host only.

On an unconfigured CI machine, `--build-only` permits ad-hoc compilation and prints
a warning. Any launch requires a valid development identity; an invalid explicit
fingerprint fails even in build-only mode. The signed host checks Accessibility and
screen-capture access before any coordinate/matrix tests. Missing access writes
exit code 1 with the exact app path. Ordinary verification and `--permissions-only` never request or modify TCC grants.
Only the explicit `--request-permissions` mode calls the public native request APIs.

For this development checkout, add **Graph Lab Verification.app** at this stable
path (use Command-Shift-G in the System Settings add-app file picker):

```text
/Users/suhail/.codex/worktrees/fa7c/photara/platform/macos/photara-graph-lab/.build/verification/Graph Lab Verification.app
```

Bundle ID: `com.photara.graph-lab.verification`. This is the verification app,
separate from the production app and ordinary Graph Lab.

1. Open **System Settings → Privacy & Security → Accessibility**. Use **+** to add
   the app above and turn its switch on. On this macOS 27 development build,
   the corresponding Settings category is shown as **Device Control and Data Access**.
2. Open **System Settings → Privacy & Security → Screen & System Audio Recording**.
   Add the same app and enable screen recording. The harness does not capture audio.
3. Accept macOS's Quit & Reopen request if shown; otherwise relaunch the verifier.
4. Run `platform/macos/photara-graph-lab/verify-permissions.sh --permissions-only`.
   This checks access in the signed host and exits without running the matrix.
   Only after it passes should the full verification command be run.

No test assertions or production Graph code are changed by this signing setup.

### Explicit one-time request mode (no rebuild)

If System Settings shows the app enabled but the signed host still reports false,
use the explicit native request mode. Build/sign once with
`verify-interactions.sh --build-only`, then leave that app unchanged while granting
access and checking it again. From the repository root:

```sh
platform/macos/photara-graph-lab/verify-permissions.sh --request-permissions
```

This helper validates the existing development signature and bundle ID, then uses
LaunchServices to launch that exact binary. It never compiles, re-signs, resets TCC,
or runs the Graph matrix. The app calls only `AXIsProcessTrustedWithOptions` with
`kAXTrustedCheckOptionPrompt: true` and `CGRequestScreenCaptureAccess` to request
access. Respond to Apple's dialogs manually; the helper never clicks consent or
changes System Settings. macOS may direct you to Settings instead of presenting a
new dialog when it already has a decision for this app.

Request-mode exit zero means only that both public request calls returned. Their
return values can remain false until consent is completed and the app is relaunched;
they are not a passing permission check or a test result. After granting access,
accept Quit & Reopen if offered, then explicitly start a fresh process of the same
built app using:

```sh
platform/macos/photara-graph-lab/verify-permissions.sh --permissions-only
```

That mode is prompt-free and exits 1 unless both native checks return true. Neither
permission mode runs behavioral tests. No full-matrix rerun is authorized by merely
requesting permissions. Both helpers record per-launch logs and actual app exit codes
under `.build/verification`; a missing exit-code file fails closed.

The launch-only helper does not depend on `codesign`'s human-readable `Authority`
line, which can say `(unavailable)` on macOS/Xcode 27 for valid development code.
It resolves the same configured Apple Development certificate fingerprint as the
builder, checks the recorded TeamIdentifier and designated requirement, rejects
ad-hoc flags, and asks `codesign --verify --strict -R` to enforce the exact leaf
certificate, Apple trust anchor, team and verification bundle identifier. The
`signature.txt` and `designated-requirement.txt` files must exist from the signed
build; missing records or any identity/team/requirement mismatch fail closed.
If multiple certificates are installed, use the same
`PHOTARA_GRAPH_VERIFY_SIGNING_SHA1` for both build and permission launch.

This validation passed on the exact existing binary, and real negative checks
rejected a wrong leaf fingerprint, wrong team, ad-hoc copy and unsigned copy.
The signature-validation correction did not require rebuilding the verifier.
Keep the same signing identity for later rebuilds and use the explicit request
command only when native authorization is absent.

### macOS 27 permission contract and operational probe

Apple's current [ScreenCaptureKit documentation](https://developer.apple.com/documentation/screencapturekit)
requires `NSScreenCaptureUsageDescription`. The verifier's generated Info.plist now
contains: “Capture only Graph verification windows to check rendering and native
controls.” It does not request audio capture. No extra AX purpose-string requirement
was found in the public AX API documentation; the verifier uses the documented
Accessibility prompt option. Production app and Graph Lab resource plists are unchanged.

The Xcode 27 `CGWindow.h` header still describes `CGPreflightScreenCaptureAccess` as
a prompt-free authorization check: false is not evidence of authorized access. It
also says a previously denied process is not prompted again by the request API.
[Apple's AX request documentation](https://developer.apple.com/documentation/applicationservices/1459186-axisprocesstrustedwithoptions)
says its prompt is asynchronous and does not affect the returned trust value.
All modes now wait for `applicationDidFinishLaunching`, a running NSApplication event
loop and one further main-actor turn before calling permission APIs (5-second limit).

An explicit diagnostic mode tests actual operations even when preflight reports
false. This may invoke macOS capture UI; run it interactively if consent is needed:

```sh
platform/macos/photara-graph-lab/verify-permissions.sh --probe-permissions
```

It opens only its own disposable 320×220-point calibration window and has a
15-second watchdog. It measures tagged mouse down/drag/up delivery through the same
public HID posting path as the unchanged slider tests; performs an AX button press
and verifies the callback; captures the window via ScreenCaptureKit; and exercises
the original `/usr/sbin/screencapture -l` compositor route with a 2-second limit.
Both image paths must contain spatially uniform, distinct magenta/cyan regions.
Validation converts into sRGB and compares relative chroma/contrast, rather than
assuming display-managed channel endpoints. A nonempty image alone is insufficient. Images and exact API/error results stay in the per-launch folder.
Same-process AX can invoke the target directly and is not proof of general
Accessibility authorization; its callback records delivery safely on the main actor.
Its return code and observed callback are diagnostic only: an AX error remains an
error even if a callback occurred. The input gate requires actual tagged HID receipt
and native authorization; it does not depend on this self-targeted AX operation.

The diagnostic records operational results separately from reported authorization.
Neither a self-window AX action nor a capture alone overrides failed authorization.
The normal gate still fails closed when the native authorization checks are false;
when true, it now also requires the real capability probes before the unchanged
Graph matrix. Permission modes never run the matrix, and no direct controller calls,
weakened assertions, TCC resets, or automated consent were added.
