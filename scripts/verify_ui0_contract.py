#!/usr/bin/env python3
"""Deterministic UI0 ownership guard; no application, user data or network access."""
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
UI = ROOT / 'platform/macos'
BASELINE = 'ef3fc00'
GRAPH_SUFFIXES = {'.swift', '.json', '.sh', '.svg', '.py', '.plist'}


def read(relative):
    return (UI / relative).read_text()


def assertion_calls(source):
    """Keep the complete conditions and messages in approved harness regions."""
    calls = []
    search = 0
    while (start := source.find('Self.check(', search)) >= 0:
        index = start + len('Self.check(')
        depth = 1
        quoted = False
        while depth:
            assert index < len(source), 'Unterminated verification assertion'
            character = source[index]
            if quoted:
                if character == '\\':
                    index += 2
                    continue
                if character == '"':
                    quoted = False
            elif character == '"':
                quoted = True
            elif character == '(':
                depth += 1
            elif character == ')':
                depth -= 1
            index += 1
        calls.append(source[start:index])
        search = index
    return calls


def approved_harness_region(expected, actual, start, end, digest):
    """Accept reviewed host plumbing only; retain every original assertion."""
    assert expected.count(start) == actual.count(start) == 1, start
    assert expected.count(end) == actual.count(end) == 1, end
    old_start, new_start = expected.index(start), actual.index(start)
    old_end, new_end = expected.index(end, old_start), actual.index(end, new_start)
    old_region, new_region = expected[old_start:old_end], actual[new_start:new_end]
    assert hashlib.sha256(new_region.encode()).hexdigest() == digest, \
        f'Unapproved Graph harness region: {start.strip()}'
    remaining = iter(assertion_calls(new_region))
    for assertion in assertion_calls(old_region):
        assert any(assertion == candidate for candidate in remaining), \
            f'Graph harness removed or rewrote an assertion: {assertion}'
    return expected[:old_start] + new_region + expected[old_end:]


def await_context_menu_checks(source, count):
    """Await native focus before evaluating the same menu result assertion."""
    calls = [line for line in source.splitlines() if 'Self.check(contextMenu(' in line]
    assert len(calls) == count, 'Unexpected Graph context-menu assertion inventory'
    for line in calls:
        indent, call = line.split('Self.check(contextMenu(', 1)
        expression, message = call.rsplit('), ', 1)
        replacement = (indent + 'let menuPerformed = await contextMenu(' + expression + ')\n'
                       + indent + 'Self.check(menuPerformed, ' + message)
        source = source.replace(line, replacement)
    return source


palette = json.loads(read('photara-theme/Resources/photara-default.json'))
ladder = ['surface.canvas', 'surface.panel', 'surface.elevated']
aliases = {'surface.control', 'graph.background', 'graph.grid', 'graph.node',
           'graph.node-selected', 'gallery.background', 'gallery.cell'}
for appearance in ['light', 'dark']:
    colors = palette['modes'][appearance]['colors']
    assert not aliases.intersection(colors), 'Feature backgrounds must be aliases, not authored slots'
    assert {key for key in colors if key.startswith('surface.')} == set(ladder)
    for key in ladder + ['editor.surround']:
        value = colors[key]
        assert len(value) == 7 and value[1:3] == value[3:5] == value[5:7], (appearance, key, value)
    assert len({colors[key] for key in ladder}) == 3, 'Ladder levels collapsed'
    assert not any(key.endswith('.background') and key != 'selection.background' for key in colors)

shell = read('photara-shell-lab/Sources/ShellLabApp.swift')
fixtures = read('photara-lab-support/Sources/ShellFixtures.swift')
for source in [shell, fixtures]:
    for forbidden in ['ColorPicker', 'themeColorPicker', 'adaptiveColorPicker', 'themeDocument',
                      'PhotaraThemeDevelopmentSettings', 'GlassTestScene']:
        assert forbidden not in source, f'Shell reintroduced Theme authoring: {forbidden}'
assert 'if model.scenario.authorsModuleGeometry' in shell
assert '.id(model.scenario)' in fixtures
assert 'session = EditorSessionModel(persists: false)' in fixtures
for lab in UI.glob('photara-*-lab/Sources/*.swift'):
    if lab.parent.parent.name == 'photara-graph-lab':
        continue  # Accepted Graph styling remains intact; canvas authoring is checked below.
    assert '.setColor(' not in lab.read_text(), f'Feature lab writes shared palette: {lab}'

theme_view = read('photara-theme/Sources/ThemeLabView.swift')
assert 'PhotaraThemeRole.aliases' in theme_view and 'PhotaraSurfaceLevel.allCases' in theme_view
assert 'GlassTestScene' not in theme_view and 'GlassTestScene' not in read('photara-theme/build-theme-lab.sh')
opening = read('photara-shell/Sources/OpeningLibraryView.swift')
assert 'theme?.color(.editorSurround)' in read('photara-gallery/Sources/GalleryCard.swift')
assert 'theme?.color(.editorSurround)' in read('photara-gallery/Sources/GalleryFullImageView.swift')
assert opening.count('Color(nsColor: .windowBackgroundColor)') >= 2
for forbidden in ['@Environment(\\.photaraTheme)', 'theme?.surface(', 'theme?.color(',
                  '.borderedProminent', '.tint(']:
    assert forbidden not in opening, f'Opening reintroduced authored styling: {forbidden}'
application_shell = read('photara-shell/Sources/ApplicationShell.swift')
assert 'if !presentation.hasOpenProject { return Color(nsColor: .windowBackgroundColor) }' in application_shell
assert '.tint(resolved.color(.borderFocus))' not in read('photara-app/Sources/PhotaraMacApp.swift')
create_project = read('photara-shell/Sources/CreateProjectView.swift')
assert 'static let shipped: Self = .compact' in create_project
assert 'var presentation: CreateProjectPresentation = .shipped' in create_project
assert 'var createProjectPresentation: CreateProjectPresentation = .shipped' in fixtures
assert 'CreateProjectPresentation.allCases' in shell

# Every Graph implementation, fixture, preset and test remains byte-identical,
# except the three approved Theme-consumption edits and the narrowly authorized
# macOS 27 verification lifecycle/input adapter. Behavioral assertions remain exact.
paths = subprocess.check_output(['git', 'ls-tree', '-r', '--name-only', BASELINE,
    'platform/macos/photara-graph', 'platform/macos/photara-graph-lab'], cwd=ROOT, text=True).splitlines()
verified = {}
for path in paths:
    if Path(path).suffix not in GRAPH_SUFFIXES:
        continue
    expected = subprocess.check_output(['git', 'show', f'{BASELINE}:{path}'], cwd=ROOT).decode()
    if path.endswith('/GraphPresentation.swift'):
        expected = expected.replace('(backgroundColor\n                ?? theme?.color(.graphBackground)',
                                    '(theme?.color(.graphBackground)\n                ?? backgroundColor')
    elif path.endswith('/GraphLabView.swift'):
        start = expected.index('                ColorPicker(', expected.index('            Section("Background pattern")'))
        end = expected.index('                Picker("Pattern"', start)
        expected = expected[:start] + '''                LabeledContent("Canvas · Foundation", value: theme?.rgba(.graphBackground)?.hex ?? "System")
                Text("Inherited from Theme Lab. The Graph is a composition root and reuses Foundation.")
                    .font(.caption).foregroundStyle(.secondary)
''' + expected[end:]
    elif path.endswith('/GraphLabApp.swift'):
        start = expected.index('    private let document:')
        end = expected.index('    var body:', start)
        expected = expected[:start] + '    @StateObject private var themeStore = PhotaraThemeStore()\n\n' + expected[end:]
        expected = expected.replace('document.resolved(for: appearance)', 'themeStore.document.resolved(for: appearance)')
    elif path.endswith('/GraphInteractionChecks.swift'):
        # Only the verification App lifecycle, input construction, and a fail-fast
        # preflight may differ. Every behavioral assertion/matrix/oracle stays exact.
        start = expected.index('@main\n')
        end = expected.index('@MainActor\nfinal class GraphLabChecks', start)
        expected = expected[:start] + expected[end:]
        expected = expected.replace(
            '        try? await Task.sleep(for: .milliseconds(700))\n'
            '        guard let window = NSApp.windows.first(where: { $0.contentView.flatMap(findSurface) != nil }),',
            '        guard let window = GraphVerificationLifecycle.window,')
        expected = expected.replace(
            'GraphTestLog.write("FAIL: Could not locate production event surface"); exit(1)',
            'GraphTestLog.write("FAIL: Could not locate production event surface"); GraphVerificationResult.recordExitCode(1); exit(1)')
        expected = expected.replace(
            '        let suite = GraphLabChecks(window: window, surface: surface)\n',
            '        let suite = GraphLabChecks(window: window, surface: surface)\n'
            '        guard GraphNativeInput.preflight(check: { Self.check($0, $1) }) else { finish() }\n'
            '        await suite.focusCanvas()\n'
            '        if ProcessInfo.processInfo.environment["PHOTARA_GRAPH_PREFLIGHT_ONLY"] == "1" { finish() }\n')
        expected = expected.replace(
            '        if options.randomOnly {\n',
            '        if ProcessInfo.processInfo.environment["PHOTARA_GRAPH_CAMERA_ONLY"] == "1" {\n'
            '            await suite.nativeCameraChecks()\n'
            '            await suite.overviewChecks()\n'
            '            finish()\n'
            '        }\n'
            '        if options.randomOnly {\n')
        # Targeted replay is opt-in. The normal full matrix, including snapshots,
        # lifecycle, random seeds and coverage checks, remains exactly unchanged.
        expected = expected.replace(
            '            await suite.overviewSnapshotCheck(mode.rawValue)\n',
            '            if ProcessInfo.processInfo.environment["PHOTARA_GRAPH_CONTEXT_MATRIX_ONLY"] == "1" { continue }\n'
            '            await suite.overviewSnapshotCheck(mode.rawValue)\n')
        expected = expected.replace(
            '        await suite.lifecycleChecks()\n',
            '        if ProcessInfo.processInfo.environment["PHOTARA_GRAPH_CONTEXT_MATRIX_ONLY"] == "1" { finish() }\n'
            '        await suite.lifecycleChecks()\n')
        expected = expected.replace('        exit(failures.isEmpty ? 0 : 1)\n',
            '        GraphVerificationResult.recordExitCode(failures.isEmpty ? 0 : 1)\n'
            '        exit(failures.isEmpty ? 0 : 1)\n')
        start = expected.index('        let location = surface.convert(point, to: nil)', expected.index('    func mouse('))
        end = expected.index('\n    func key(', start)
        expected = expected[:start] + r'''        let location = GraphNativeInput.location(point, in: surface)
        do {
            let event = try GraphNativeInput.mouse(type, location: location, flags: flags, window: window)
            lastMouseDelivery = "expected \(location), actual \(event.locationInWindow), window \(event.windowNumber)/\(window.windowNumber), button \(event.buttonNumber)"
            window.sendEvent(event)
        } catch {
            lastMouseDelivery = String(describing: error)
            Self.check(false, "Native event coordinate precondition: \(error)")
        }
    }
''' + expected[end:]
        # Native hitTest receives its superview's coordinates. Preserve the
        # existing slider point, native HID gesture and original priority check.
        # The additional nonnil assertion rejects an invalid receiver outright.
        expected = expected.replace(
            '        let hit = window.contentView?.hitTest(surface.convert(sliderPoint, to: window.contentView))\n',
            r'''        let hit = GraphNativeInput.hitTest(sliderPoint, in: surface, through: window.contentView)
        if let content = window.contentView {
            let localPoint = surface.convert(sliderPoint, to: content)
            let parentPoint = surface.convert(sliderPoint, to: content.superview)
            GraphTestLog.write("NATIVE HIT: slider=\(sliderPoint), contentLocal=\(localPoint), parent=\(parentPoint), legacy=\(String(describing: content.hitTest(localPoint))), actual=\(String(describing: hit)), contentFrame=\(content.frame), contentBounds=\(content.bounds), contentFlipped=\(content.isFlipped), parentFrame=\(String(describing: content.superview?.frame)), parentFlipped=\(String(describing: content.superview?.isFlipped))")
        }
        Self.check(hit != nil, "Native zoom hit-test resolves a view in the content parent coordinate system")
''')
        # Approved focus/menu plumbing has a fixed digest AND must retain every
        # original assertion verbatim/in order. All other bytes are reconstructed
        # from the baseline above, including the original behavioral matrix.
        actual_source = (ROOT / path).read_text()
        for start, end, digest in [
            ('    func focusCanvas() async {', "    /// SwiftUI's native slider",
             '2c151ccd7294536ea10383eb428648c88fe1ecad1d6d3b58274537f044688e74'),
            ('    func ownershipAndMenuChecks() async {', '    func contextMenu(',
             '1a59e646f854aea2f32ccddfedee6d1848382d9003339f8557a6be731a730e81'),
            ('    func contextMenu(', '    func documentChecks()',
             'acfe3f2f5c9f280a0303efaf74aefb485e71a4b09319623eeb6b9dfcbefd44df'),
            ('@MainActor\nprivate final class MenuDriver: NSObject {', '\nenum GraphTestLog {',
             '5c22abaff7718c1b9376eff7b46b8dc623de4798e9db772696400b57bb572e90'),
        ]:
            expected = approved_harness_region(expected, actual_source, start, end, digest)
        assert 'Self.check(stableFocus == 3, "Verification window retained native input focus across layout turns")' in actual_source
    elif path.endswith('/GraphBranchOverviewChecks.swift'):
        expected = await_context_menu_checks(expected, 1)
        # Only adapt the query's coordinate system: both original === surface
        # predicates, sample points and assertion messages stay byte-identical.
        for point in ['point', 'hitPoint']:
            old = f'window.contentView?.hitTest(surface.convert({point}, to: window.contentView))'
            assert expected.count(old) == 1
            expected = expected.replace(old,
                f'GraphNativeInput.hitTest({point}, in: surface, through: window.contentView)')
    elif path.endswith('/GraphRandomGestures.swift'):
        # Only drain native layout before constructing synthetic wheel input.
        # Every action, oracle comparison, coordinate tolerance and seed stays exact.
        expected = expected.replace(
            'window.convertPoint(toScreen: surface.convert(anchor, to: nil))',
            'window.convertPoint(toScreen: GraphNativeInput.location(anchor, in: surface))')
        expected = await_context_menu_checks(expected, 6)
        branch_assertion = '                Self.check(controller.wire?.source == edge.source && Self.near(controller.wireStart ?? .zero, point),\n'
        expected = expected.replace(branch_assertion, r'''                if controller.wire?.source != edge.source || !Self.near(controller.wireStart ?? .zero, point) {
                    let diagnostic = "BRANCH DELIVERY: step=\(index) action=\(action.rawValue) point=\(oracle.screen(point)) "
                        + "hit=\(controller.hitTest(oracle.screen(point))) actualWire=\(String(describing: controller.wire)) "
                        + "actualOrigin=\(String(describing: controller.wireStart)) delivery=\(lastMouseDelivery) \(focusDiagnostic)"
                    trace.append(diagnostic)
                    GraphTestLog.write(diagnostic)
                }
''' + branch_assertion)
    elif path.endswith('/verify-interactions.sh'):
        # Pin the explicitly authorized signing resolver; all unrelated launcher
        # content is still reconstructed from the baseline below.
        launcher = (ROOT / path).read_text()
        expected = expected.replace('cp -p "$THEME_ROOT/Resources/photara-default.json"', '/usr/libexec/PlistBuddy -c \'Add :NSScreenCaptureUsageDescription string Capture only Graph verification windows to check rendering and native controls.\' "$APP_BUNDLE/Contents/Info.plist"\n' + 'cp -p "$THEME_ROOT/Resources/photara-default.json"')
        expected = expected.replace('-framework SwiftUI -framework AppKit -o', '-framework SwiftUI -framework AppKit -framework ScreenCaptureKit -o')
        expected = expected.replace('/usr/libexec/PlistBuddy -c \'Set :CFBundleIdentifier com.photara.graph-lab.verification\' "$APP_BUNDLE/Contents/Info.plist"\n', '/usr/libexec/PlistBuddy -c \'Set :CFBundleIdentifier com.photara.graph-lab.verification\' "$APP_BUNDLE/Contents/Info.plist"\n/usr/libexec/PlistBuddy -c \'Set :CFBundleName Graph Lab Verification\' "$APP_BUNDLE/Contents/Info.plist"\n/usr/libexec/PlistBuddy -c \'Set :CFBundleDisplayName Graph Lab Verification\' "$APP_BUNDLE/Contents/Info.plist"\n')
        signing_start = launcher.index('# Stable development identity')
        signing_end = launcher.index('mkdir -p', signing_start)
        signing = launcher[signing_start:signing_end]
        assert hashlib.sha256(signing.encode()).hexdigest() == 'a389441b8eaf9b635241cc94dfae01ab2a10eccffffb8b6c3abc966dbd454cb7'
        expected = expected.replace('mkdir -p', signing + 'mkdir -p', 1)
        expected = expected.replace('codesign --force --deep --sign - "$APP_BUNDLE"', 'codesign --force --sign "$GRAPH_SIGNING_IDENTITY" --timestamp=none "$APP_BUNDLE"\ncodesign --verify --strict "$APP_BUNDLE"\nif [[ "$GRAPH_SIGNING_IDENTITY" != - ]]; then\n  codesign -d -r- "$APP_BUNDLE" > "$BUILD_ROOT/designated-requirement.txt" 2>&1\n  codesign -dv "$APP_BUNDLE" 2> "$BUILD_ROOT/signature.txt"\n  if ! /usr/bin/grep -Eq \'^TeamIdentifier=[A-Z0-9]{10}$\' "$BUILD_ROOT/signature.txt"; then\n    print -u2 -- "Graph verification: development signature is missing its TeamIdentifier."\n    exit 1\n  fi\nfi')
        expected = expected.replace(
            '  "$SCRIPT_ROOT/Sources/GraphLabView.swift" "$SCRIPT_ROOT/Tests/GraphInteractionChecks.swift" ' + chr(92) + '\n',
            '  "$SCRIPT_ROOT/Sources/GraphLabView.swift" "$SCRIPT_ROOT/Tests/GraphInteractionChecks.swift" ' + chr(92) + '\n'
            '  "$SCRIPT_ROOT/Tests/GraphVerificationHost.swift" "$SCRIPT_ROOT/Tests/GraphNativeInput.swift" "$SCRIPT_ROOT/Tests/GraphPermissionProbe.swift" ' + chr(92) + '\n')
        expected = expected.replace('\"$APP_BUNDLE/Contents/MacOS/PhotaraGraphLab\" \"$@\"\n', r'''if [[ "${1:-}" == --build-only ]]; then
  print -r -- "$APP_BUNDLE"
  exit 0
fi
# The explicit AppKit test host owns its window. Launch its exact signed binary
# as an owned child so startup/readiness and completion both have bounded waits.
exec python3 "$SCRIPT_ROOT/launch-verification.py" --app "$APP_BUNDLE" -- "$@"
''')
    actual = (ROOT / path).read_bytes()
    assert actual == expected.encode(), f'Unapproved Graph change: {path}'
    verified[path] = hashlib.sha256(actual).hexdigest()
# New test-only hosts and launchers also have fixed, reviewed bytes. Logging their
# hashes alone would leave them outside the otherwise strict ownership guard.
for name, digest in {
    'Tests/GraphVerificationHost.swift': '4dbe7f47bcb0d12af7c4c192fa34ce27c0d16c4dd909df3051c1779599561f20',
    'Tests/GraphNativeInput.swift': 'e2511d66f227c72fba3f4067c98b46383b3c20f5f6139cb652bfd4d9ad5a46ce',
    'Tests/GraphPermissionProbe.swift': 'c2c1eac7db2a47916425a7272721945ed1adee8bd0a5788257b66b506938315c',
    'launch-verification.py': 'ef4b0c87cc8a7cc7f64491cca8a1354d267daebfe48eef0c4227ead79c371788',
    'verify-launch-furnace.py': 'e0f250b9978c37b01384e34640ab49061525d726c3d2622b8f97d1e69cd03fb0',
    # The permission launcher validates the exact existing signature. It cannot
    # rebuild or invoke the behavioral matrix; it uses the same bounded launcher.
    'verify-permissions.sh': '262868f8af61fd05239b15273184e0468f4b82e06ad3d11bd293f7f91049c1f0',
}.items():
    path = 'platform/macos/photara-graph-lab/' + name
    actual_digest = hashlib.sha256((ROOT / path).read_bytes()).hexdigest()
    assert actual_digest == digest, f'Unapproved Graph verification adapter: {path}'
    verified[path] = actual_digest
current_paths = subprocess.check_output([
    'git', 'ls-files', '--cached', '--others', '--exclude-standard',
    'platform/macos/photara-graph', 'platform/macos/photara-graph-lab',
], cwd=ROOT, text=True).splitlines()
current_code = {path for path in current_paths if Path(path).suffix in GRAPH_SUFFIXES}
assert current_code == set(verified), \
    f'Unapproved Graph source inventory: {sorted(current_code.symmetric_difference(verified))}'
print(f'PASS: UI0 ownership, neutral defaults, no duplicate background authoring; {len(verified)} Graph source/preset/test files verified')
print(json.dumps(verified, indent=2, sort_keys=True))
