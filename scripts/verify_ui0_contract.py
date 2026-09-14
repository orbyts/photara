#!/usr/bin/env python3
"""Deterministic UI0 ownership guard; no application, user data or network access."""
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
UI = ROOT / 'platform/macos'
BASELINE = 'ef3fc00'


def read(relative):
    return (UI / relative).read_text()


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
    if Path(path).suffix not in {'.swift', '.json', '.sh', '.svg'}:
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
            '        let suite = GraphLabChecks(window: window, surface: surface)\n',
            '        let suite = GraphLabChecks(window: window, surface: surface)\n'
            '        guard GraphNativeInput.preflight(check: { Self.check($0, $1) }) else { finish() }\n'
            '        await suite.focusCanvas()\n'
            '        if ProcessInfo.processInfo.environment["PHOTARA_GRAPH_PREFLIGHT_ONLY"] == "1" { finish() }\n')
        expected = expected.replace('        exit(failures.isEmpty ? 0 : 1)\n',
            '        GraphVerificationResult.recordExitCode(failures.isEmpty ? 0 : 1)\n'
            '        exit(failures.isEmpty ? 0 : 1)\n')
        start = expected.index('        let location = surface.convert(point, to: nil)', expected.index('    func mouse('))
        end = expected.index('\n    func key(', start)
        expected = expected[:start] + r'''        let location = surface.convert(point, to: nil)
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
# LaunchServices delivers the normal application-open event to the original
# WindowGroup. Direct executable launch can remain windowless on macOS 27.
run_root="$(mktemp -d "$BUILD_ROOT/run.XXXXXX")"
print -r -- "Graph verification log: $run_root/stdout.log"
/usr/bin/open -n -W --stdout "$run_root/stdout.log" --stderr "$run_root/stderr.log" \
  --env "PHOTARA_GRAPH_EXIT_FILE=$run_root/exit-code" \
  --env "PHOTARA_GRAPH_PREFLIGHT_ONLY=${PHOTARA_GRAPH_PREFLIGHT_ONLY:-0}" \
  "$APP_BUNDLE" --args "$@"
cat "$run_root/stdout.log" "$run_root/stderr.log"
if [[ ! -f "$run_root/exit-code" ]]; then
  print -u2 -- "Graph verification exited without a completed result; startup failure or crash."
  exit 1
fi
run_exit_code="$(cat "$run_root/exit-code")"
case "$run_exit_code" in
  0|1) exit "$run_exit_code" ;;
  *) print -u2 -- "Invalid Graph verification result: $run_exit_code"; exit 1 ;;
esac
''')
    actual = (ROOT / path).read_bytes()
    assert actual == expected.encode(), f'Unapproved Graph change: {path}'
    verified[path] = hashlib.sha256(actual).hexdigest()
for name in ['GraphVerificationHost.swift', 'GraphNativeInput.swift', 'GraphPermissionProbe.swift']:
    path = 'platform/macos/photara-graph-lab/Tests/' + name
    verified[path] = hashlib.sha256((ROOT / path).read_bytes()).hexdigest()
# The explicit permission launcher cannot rebuild or invoke the behavioral matrix.
permission_launcher = 'platform/macos/photara-graph-lab/verify-permissions.sh'
permission_bytes = (ROOT / permission_launcher).read_bytes()
assert hashlib.sha256(permission_bytes).hexdigest() == '14606ee1c8707dfe838aa9e1acdcf1c7d1e98ffe60722a9d94ae030b15173e98'
verified[permission_launcher] = hashlib.sha256(permission_bytes).hexdigest()
print(f'PASS: UI0 ownership, neutral defaults, no duplicate background authoring; {len(verified)} Graph source/preset/test files verified')
print(json.dumps(verified, indent=2, sort_keys=True))
