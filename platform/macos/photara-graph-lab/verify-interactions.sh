#!/bin/zsh
set -euo pipefail
SCRIPT_ROOT="${0:A:h}"
REPOSITORY_ROOT="${SCRIPT_ROOT:h:h:h}"
GRAPH_ROOT="$REPOSITORY_ROOT/platform/macos/photara-graph/Sources"
THEME_ROOT="$REPOSITORY_ROOT/platform/macos/photara-theme"
GRAPH_ICON_ROOT="$REPOSITORY_ROOT/platform/macos/photara-graph/Resources/NodeIcons"
GRAPH_TOOL_ICON_ROOT="$REPOSITORY_ROOT/platform/macos/photara-graph/Resources/ToolIcons"
BUILD_ROOT="$SCRIPT_ROOT/.build/verification"
APP_BUNDLE="$BUILD_ROOT/Graph Lab Verification.app"
# Stable development identity keeps TCC grants valid across rebuilds. The optional
# override accepts only a fingerprint present among valid Apple Development identities.
# BEGIN VERIFICATION SIGNING
GRAPH_SIGNING_IDENTITY="$(python3 - "${1:-}" <<'PY_SIGNING'
import os
import re
import subprocess
import sys

result = subprocess.run(['/usr/bin/security', 'find-identity', '-v', '-p', 'codesigning'],
                        text=True, capture_output=True)
if result.returncode:
    sys.exit('Graph verification: cannot inspect signing identities; check Keychain access.')
identities = sorted(set(re.findall(
    r'^\s*\d+\) ([0-9A-Fa-f]{40}) "Apple Development:[^"\n]+"', result.stdout, re.M)))
requested = os.environ.get('PHOTARA_GRAPH_VERIFY_SIGNING_SHA1', '').upper()
if requested:
    if requested not in identities:
        sys.exit('Graph verification: PHOTARA_GRAPH_VERIFY_SIGNING_SHA1 must match a valid Apple Development fingerprint.')
    print(requested)
elif identities:
    print(identities[0])
elif sys.argv[1] == '--build-only':
    print('Graph verification: no Apple Development identity; ad-hoc BUILD ONLY, native gate unavailable.', file=sys.stderr)
    print('-')
else:
    sys.exit('Graph verification requires an Apple Development signing identity. Configure one in Xcode/Keychain or use --build-only for ad-hoc CI. No native gate was run.')
PY_SIGNING
)"
# END VERIFICATION SIGNING
mkdir -p "$APP_BUNDLE/Contents/MacOS" "$APP_BUNDLE/Contents/Resources" /tmp/photara-graph-verification
cp -p "$SCRIPT_ROOT/Resources/Info.plist" "$APP_BUNDLE/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Set :CFBundleIdentifier com.photara.graph-lab.verification' "$APP_BUNDLE/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Set :CFBundleName Graph Lab Verification' "$APP_BUNDLE/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Set :CFBundleDisplayName Graph Lab Verification' "$APP_BUNDLE/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Add :NSScreenCaptureUsageDescription string Capture only Graph verification windows to check rendering and native controls.' "$APP_BUNDLE/Contents/Info.plist"
cp -p "$THEME_ROOT/Resources/photara-default.json" "$APP_BUNDLE/Contents/Resources/photara-default.json"
cp -p "$REPOSITORY_ROOT/platform/macos/photara-graph/Resources/photara-graph-presentation-v1.json" "$APP_BUNDLE/Contents/Resources/photara-graph-presentation-v1.json"
ditto "$GRAPH_ICON_ROOT" "$APP_BUNDLE/Contents/Resources/NodeIcons"
ditto "$GRAPH_TOOL_ICON_ROOT" "$APP_BUNDLE/Contents/Resources/ToolIcons"
xcrun swiftc -swift-version 6 -O -parse-as-library -module-cache-path "$SCRIPT_ROOT/.build/module-cache" \
  "$THEME_ROOT/Sources/PhotaraTheme.swift" \
  "$REPOSITORY_ROOT/platform/macos/photara-app/Sources/ThemeStore.swift" \
  "$GRAPH_ROOT/GraphPresentation.swift" "$GRAPH_ROOT/GraphPreset.swift" "$GRAPH_ROOT/GraphDocument.swift" \
  "$GRAPH_ROOT/GraphOverview.swift" "$GRAPH_ROOT/GraphToolIcons.swift" "$GRAPH_ROOT/GraphGeometry.swift" "$GRAPH_ROOT/GraphInteraction.swift" "$GRAPH_ROOT/GraphEventSurface.swift" \
  "$GRAPH_ROOT/GraphToolRail.swift" "$GRAPH_ROOT/GraphNodeView.swift" "$GRAPH_ROOT/GraphCanvas.swift" \
  "$SCRIPT_ROOT/Sources/GraphLabFixtures.swift" "$SCRIPT_ROOT/Sources/GraphLabTooling.swift" "$SCRIPT_ROOT/Sources/GraphLabCanvas.swift" \
  "$SCRIPT_ROOT/Sources/GraphLabView.swift" "$SCRIPT_ROOT/Tests/GraphInteractionChecks.swift" \
  "$SCRIPT_ROOT/Tests/GraphVerificationHost.swift" "$SCRIPT_ROOT/Tests/GraphNativeInput.swift" "$SCRIPT_ROOT/Tests/GraphPermissionProbe.swift" \
  "$SCRIPT_ROOT/Tests/GraphBranchOverviewChecks.swift" "$SCRIPT_ROOT/Tests/GraphGestureOracle.swift" "$SCRIPT_ROOT/Tests/GraphRandomGestures.swift" \
  -framework SwiftUI -framework AppKit -framework ScreenCaptureKit -o "$APP_BUNDLE/Contents/MacOS/PhotaraGraphLab"
codesign --force --sign "$GRAPH_SIGNING_IDENTITY" --timestamp=none "$APP_BUNDLE"
codesign --verify --strict "$APP_BUNDLE"
if [[ "$GRAPH_SIGNING_IDENTITY" != - ]]; then
  codesign -d -r- "$APP_BUNDLE" > "$BUILD_ROOT/designated-requirement.txt" 2>&1
  codesign -dv "$APP_BUNDLE" 2> "$BUILD_ROOT/signature.txt"
  if ! /usr/bin/grep -Eq '^TeamIdentifier=[A-Z0-9]{10}$' "$BUILD_ROOT/signature.txt"; then
    print -u2 -- "Graph verification: development signature is missing its TeamIdentifier."
    exit 1
  fi
fi
# Read the author's existing payload without ever writing their preference domain.
# The verification app adds the author domain as a read-only fallback suite.
if [[ "${1:-}" == --build-only ]]; then
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
