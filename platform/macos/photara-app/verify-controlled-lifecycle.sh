#!/bin/zsh
set -euo pipefail
SCRIPT_ROOT="${0:A:h}"
LAB="$SCRIPT_ROOT/../photara-graph-lab"
BUILD_ROOT=$(mktemp -d /private/tmp/photara-controlled-lifecycle.XXXXXX)
APP="$BUILD_ROOT/Lifecycle Check.app"
mkdir -p "$APP/Contents/MacOS"
cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict><key>CFBundleExecutable</key><string>lifecycle-check</string>
<key>CFBundleIdentifier</key><string>com.photara.controlled-lifecycle-test</string>
<key>CFBundlePackageType</key><string>APPL</string></dict></plist>
PLIST
xcrun swiftc -swift-version 6 -warnings-as-errors -parse-as-library -D CONTROLLED_DISPOSABLE \
  -module-cache-path "$BUILD_ROOT/module-cache" \
  "$LAB/Sources/DisposableAutosaveProjection.swift" \
  "$LAB/Sources/DisposableAutosaveProcess.swift" \
  "$SCRIPT_ROOT/Sources/ControlledSessionLifecycle.swift" \
  "$SCRIPT_ROOT/Sources/PhotaraMacApp.swift" \
  "$SCRIPT_ROOT/Tests/ControlledLifecycleRuntime.swift" \
  -framework SwiftUI -framework AppKit -o "$APP/Contents/MacOS/lifecycle-check"
codesign --force --sign - "$APP"
python3 - "$APP/Contents/MacOS/lifecycle-check" <<'PY'
import subprocess, sys
for options in ([], ['--fail-flush']):
    try:
        result = subprocess.run([sys.argv[1], *options], capture_output=True, text=True, timeout=30)
    except subprocess.TimeoutExpired as error:
        print('TIMEOUT', error.stdout, error.stderr, flush=True)
        raise
    assert result.returncode == 0, (result.returncode, result.stdout, result.stderr)
    lines = result.stdout.splitlines()
    assert 'ACTUAL_QUIT_DEFERRED_WHILE_SUBMIT_PENDING' in lines, lines
    assert lines[-1] == 'CLOSE_VERIFIED_REVISION_2', lines
    if options:
        assert 'ACTUAL_QUIT_CANCELED_ON_FLUSH_FAILURE_GRAPH_RETAINED' in lines, lines
    print('PASS:', 'failure/retry' if options else 'success', *lines, sep='\n')
PY
