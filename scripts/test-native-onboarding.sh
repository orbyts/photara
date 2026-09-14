#!/bin/zsh
set -euo pipefail
SCRIPT_ROOT="${0:A:h}"
REPOSITORY_ROOT="${SCRIPT_ROOT:h}"
FOUNDATION="$REPOSITORY_ROOT/platform/macos/photara-ui-foundation"
BUILD_ROOT="$REPOSITORY_ROOT/platform/macos/photara-app/.build/onboarding-tests"
RUST_TARGET="${PHOTARA_APP_RUST_TARGET:-$BUILD_ROOT/rust-target}"
mkdir -p "$BUILD_ROOT/generated" "$BUILD_ROOT/module-cache"
python3 "$SCRIPT_ROOT/generate_product_configuration.py" --channel development --output "$BUILD_ROOT/generated" >/dev/null
CARGO_TARGET_DIR="$RUST_TARGET" cargo build --manifest-path "$REPOSITORY_ROOT/Cargo.toml" -p photara-bridge --features bindgen
CARGO_TARGET_DIR="$RUST_TARGET" cargo run --manifest-path "$REPOSITORY_ROOT/Cargo.toml" -p photara-bridge --features bindgen --bin photara-uniffi-bindgen -- generate \
  --library "$RUST_TARGET/debug/libphotara_bridge.dylib" --language swift --out-dir "$BUILD_ROOT/generated"
xcrun swiftc -swift-version 6 -warnings-as-errors -parse-as-library \
  -module-cache-path "$BUILD_ROOT/module-cache" \
  "$BUILD_ROOT/generated/PhotaraBridge.swift" "$BUILD_ROOT/generated/CheckedReleaseConfiguration.swift" \
  "$FOUNDATION/Sources/ReleaseConfiguration.swift" "$FOUNDATION"/Sources/Native*.swift \
  "$REPOSITORY_ROOT/platform/macos/photara-shell/Sources/OpeningCloudModel.swift" \
  "$REPOSITORY_ROOT/platform/macos/photara-app/Sources/ProductionOpeningCloudDriver.swift" \
  "$REPOSITORY_ROOT/platform/macos/photara-app/Tests/NativeOnboardingFlowTests.swift" \
  -Xcc "-fmodule-map-file=$BUILD_ROOT/generated/PhotaraBridgeFFI.modulemap" \
  -L "$RUST_TARGET/debug" -lphotara_bridge -o "$BUILD_ROOT/native-onboarding-tests"
DYLD_LIBRARY_PATH="$RUST_TARGET/debug" "$BUILD_ROOT/native-onboarding-tests"
