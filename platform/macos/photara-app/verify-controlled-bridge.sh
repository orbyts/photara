#!/bin/zsh
set -euo pipefail
SCRIPT_ROOT="${0:A:h}"
GENERATED_ROOT="$1"
FRAMEWORKS="$2"
BUILD_ROOT="$3"
xcrun swiftc -swift-version 6 -warnings-as-errors -parse-as-library \
  -module-cache-path "$BUILD_ROOT/module-cache" \
  "$GENERATED_ROOT/PhotaraBridge.swift" \
  "$SCRIPT_ROOT/Tests/ControlledBridgeAdmissionSmoke.swift" \
  -Xcc "-fmodule-map-file=$GENERATED_ROOT/PhotaraBridgeFFI.modulemap" \
  -L "$FRAMEWORKS" -lphotara_bridge \
  -Xlinker -rpath -Xlinker "$FRAMEWORKS" \
  -o "$BUILD_ROOT/controlled-bridge-smoke"
"$BUILD_ROOT/controlled-bridge-smoke"
