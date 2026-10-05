#!/bin/zsh
set -euo pipefail
SCRIPT_ROOT="${0:A:h}"
BUILD_ROOT=$(mktemp -d /private/tmp/photara-autosave-projection.XXXXXX)
xcrun swiftc -swift-version 6 -warnings-as-errors -parse-as-library \
  -module-cache-path "$BUILD_ROOT/module-cache" \
  "$SCRIPT_ROOT/Sources/DisposableAutosaveProjection.swift" \
  "$SCRIPT_ROOT/Tests/DisposableAutosaveChecks.swift" -o "$BUILD_ROOT/checks"
"$BUILD_ROOT/checks"
xcrun swiftc -swift-version 6 -warnings-as-errors -typecheck \
  -module-cache-path "$BUILD_ROOT/module-cache" \
  "$SCRIPT_ROOT/Sources/DisposableAutosaveProjection.swift" \
  "$SCRIPT_ROOT/Sources/DisposableAutosaveStatusView.swift"
xcrun swiftc -swift-version 6 -warnings-as-errors -parse-as-library \
  -module-cache-path "$BUILD_ROOT/module-cache" \
  "$SCRIPT_ROOT/Sources/DisposableAutosaveProjection.swift" \
  "$SCRIPT_ROOT/Sources/DisposableAutosaveProcess.swift" \
  "$SCRIPT_ROOT/Tests/DisposablePipeReadChecks.swift" -o "$BUILD_ROOT/pipe-checks"
python3 - "$BUILD_ROOT/pipe-checks" <<'PY'
import subprocess, sys
subprocess.run([sys.argv[1]], check=True, timeout=10)
PY
