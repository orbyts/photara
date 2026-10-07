#!/bin/zsh
set -euo pipefail
SCRIPT_ROOT="${0:A:h}"
LAB="$SCRIPT_ROOT/../photara-graph-lab"
BUILD_ROOT=$(mktemp -d /private/tmp/photara-controlled-session-checks.XXXXXX)
xcrun swiftc -swift-version 6 -warnings-as-errors -parse-as-library \
  -module-cache-path "$BUILD_ROOT/module-cache" \
  "$LAB/Sources/DisposableAutosaveProjection.swift" \
  "$LAB/Sources/DisposableAutosaveProcess.swift" \
  "$SCRIPT_ROOT/Sources/ControlledDisposableConfiguration.swift" \
  "$SCRIPT_ROOT/Tests/ControlledSessionChecks.swift" -o "$BUILD_ROOT/checks"
python3 - "$BUILD_ROOT/checks" "$SCRIPT_ROOT" <<'PY'
import os, pathlib, subprocess, sys
subprocess.run([sys.argv[1]], check=True, timeout=20)
root = pathlib.Path(sys.argv[2])
for patch in ({'PHOTARA_MACOS_PROVISIONING_PROFILE': '/unopened/profile'}, {'PHOTARA_RELEASE_CHANNEL': 'production'}):
    env = dict(os.environ, **patch)
    result = subprocess.run(['zsh', str(root/'build-controlled-disposable.sh')], env=env, capture_output=True, timeout=5)
    assert result.returncode == 2 and b'ad-hoc' in result.stderr
# The compile-time controlled branch cannot initialize the legacy AppModel.
entry = (root/'Sources/PhotaraMacApp.swift').read_text().split('#else', 1)[0]
assert 'ControlledDisposableScene(delegate: delegate, model: model)' in entry
assert '@NSApplicationDelegateAdaptor' in entry and 'AppModel(' not in entry
print('PASS: controlled entry excludes legacy initialization and production signing/channel')
PY
