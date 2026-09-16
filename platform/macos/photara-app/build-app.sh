#!/bin/zsh
set -euo pipefail

SCRIPT_ROOT="${0:A:h}"
REPOSITORY_ROOT="${SCRIPT_ROOT:h:h:h}"
# Verification callers must be able to isolate every generated artifact, not
# merely Cargo's output. The default remains the interactive development app.
BUILD_ROOT="${PHOTARA_APP_BUILD_ROOT:-$SCRIPT_ROOT/.build/app}"
BUILD_ROOT="${BUILD_ROOT:A}"
RUST_TARGET="${PHOTARA_APP_RUST_TARGET:-$BUILD_ROOT/rust-target}"
GENERATED_ROOT="$BUILD_ROOT/generated"
MODULE_CACHE="$BUILD_ROOT/module-cache"
# SwiftPM compiles Package.swift before building the helper. Its manifest and
# Clang caches otherwise escape --scratch-path into the user's cache directory.
export CLANG_MODULE_CACHE_PATH="$MODULE_CACHE"
export SWIFTPM_MODULECACHE_OVERRIDE="$MODULE_CACHE"
PRODUCT_CHANNEL="${PHOTARA_RELEASE_CHANNEL:-development}"
PRODUCT_NAME="$(python3 "$REPOSITORY_ROOT/scripts/generate_product_configuration.py" \
  --channel "$PRODUCT_CHANNEL" --output "$GENERATED_ROOT")"
SIGNING_PROFILE="${PHOTARA_MACOS_PROVISIONING_PROFILE:-}"
SIGNING_ENTITLEMENTS="$GENERATED_ROOT/Photara.entitlements"
SIGNING_IDENTITY="-"
APP_BUNDLE="$BUILD_ROOT/$PRODUCT_NAME.app"
CONTENTS="$APP_BUNDLE/Contents"
MACOS="$CONTENTS/MacOS"
FRAMEWORKS="$CONTENTS/Frameworks"
RESOURCES="$CONTENTS/Resources"
THEME_ROOT="$REPOSITORY_ROOT/platform/macos/photara-theme"
EXECUTABLE="" # Assigned from generated bundle metadata below.
PROXY_HELPER_BUILD="$BUILD_ROOT/proxy-helper-build"
PROXY_HELPER="$MACOS/photara-proxy-imageio"

mkdir -p "$GENERATED_ROOT" "$MODULE_CACHE" "$MACOS" "$FRAMEWORKS" "$RESOURCES"
cp -p "$REPOSITORY_ROOT/platform/macos/photara-shell/Resources/photara-application-presentation-v1.json" "$RESOURCES/"
python3 "$REPOSITORY_ROOT/scripts/generate_product_configuration.py" \
  --channel "$PRODUCT_CHANNEL" --output "$GENERATED_ROOT" --plist "$CONTENTS/Info.plist" >/dev/null
EXECUTABLE="$MACOS/$(plutil -extract CFBundleExecutable raw "$CONTENTS/Info.plist")"
PRODUCT_BUNDLE_ID="$(plutil -extract CFBundleIdentifier raw "$CONTENTS/Info.plist")"
if [[ -n "$SIGNING_PROFILE" ]]; then
  SIGNING_IDENTITY="$(python3 "$REPOSITORY_ROOT/scripts/prepare_macos_signing.py" \
    --profile "$SIGNING_PROFILE" --bundle-id "$PRODUCT_BUNDLE_ID" \
    --entitlements "$SIGNING_ENTITLEMENTS")"
fi
cp -p "$REPOSITORY_ROOT/platform/macos/photara-gallery/Resources/photara-gallery-presentation-v1.json" "$RESOURCES/"
cp -p "$REPOSITORY_ROOT/platform/macos/photara-inspector/Resources/photara-inspector-presentation-v1.json" "$RESOURCES/"
mkdir -p "$RESOURCES/Themes"
cp -p "$THEME_ROOT/Resources/photara-default.json" "$RESOURCES/Themes/photara-default.json"
cp -p "$REPOSITORY_ROOT/platform/macos/photara-graph/Resources/photara-graph-presentation-v1.json" "$RESOURCES/photara-graph-presentation-v1.json"
ditto "$REPOSITORY_ROOT/platform/macos/photara-graph/Resources/NodeIcons" "$RESOURCES/NodeIcons"
ditto "$REPOSITORY_ROOT/platform/macos/photara-graph/Resources/ToolIcons" "$RESOURCES/ToolIcons"

swift build \
  --package-path "$REPOSITORY_ROOT/platform/macos/photara-proxy-imageio" \
  --cache-path "$BUILD_ROOT/swiftpm-cache" \
  --config-path "$BUILD_ROOT/swiftpm-configuration" \
  --security-path "$BUILD_ROOT/swiftpm-security" \
  --manifest-cache local \
  --scratch-path "$PROXY_HELPER_BUILD"
cp -p "$PROXY_HELPER_BUILD/debug/photara-proxy-imageio" "$PROXY_HELPER"

CARGO_TARGET_DIR="$RUST_TARGET" cargo build \
  --manifest-path "$REPOSITORY_ROOT/Cargo.toml" \
  -p photara-bridge

CARGO_TARGET_DIR="$RUST_TARGET" cargo run \
  --manifest-path "$REPOSITORY_ROOT/Cargo.toml" \
  -p photara-bridge \
  --features bindgen \
  --bin photara-uniffi-bindgen \
  -- generate \
  --library "$RUST_TARGET/debug/libphotara_bridge.dylib" \
  --language swift \
  --out-dir "$GENERATED_ROOT"

cp -p "$RUST_TARGET/debug/libphotara_bridge.dylib" "$FRAMEWORKS/libphotara_bridge.dylib"

PHOTARA_SHARED_UI_GENERATED_ROOT="$GENERATED_ROOT"
source "$REPOSITORY_ROOT/platform/macos/shared-ui-sources.sh"

xcrun swiftc \
  -swift-version 6 \
  -parse-as-library \
  -module-cache-path "$MODULE_CACHE" \
  "$GENERATED_ROOT/PhotaraBridge.swift" \
  "${SHARED_UI_SOURCES[@]}" \
  "$SCRIPT_ROOT"/Sources/AppModel*.swift \
  "$SCRIPT_ROOT/Sources/GalleryPresentationState.swift" \
  "$SCRIPT_ROOT/Sources/ProductionGalleryView.swift" \
  "$SCRIPT_ROOT/Sources/GraphAdapter.swift" \
  "$SCRIPT_ROOT/Sources/ProductionGraphView.swift" \
  "$SCRIPT_ROOT/Sources/InspectionAdapter.swift" \
  "$SCRIPT_ROOT/Sources/ApplicationAdapter.swift" \
  "$SCRIPT_ROOT/Sources/ProductionOpeningCloudDriver.swift" \
  "$SCRIPT_ROOT/Sources/EditorSessionView.swift" \
  "$SCRIPT_ROOT/Sources/PhotaraMacApp.swift" \
  -Xcc "-fmodule-map-file=$GENERATED_ROOT/PhotaraBridgeFFI.modulemap" \
  -L "$FRAMEWORKS" \
  -lphotara_bridge \
  -framework SwiftUI \
  -framework AppKit \
  -framework QuickLookThumbnailing \
  -Xlinker -rpath \
  -Xlinker "@executable_path/../Frameworks" \
  -o "$EXECUTABLE"

install_name_tool \
  -change "$RUST_TARGET/debug/deps/libphotara_bridge.dylib" \
  "@rpath/libphotara_bridge.dylib" \
  "$EXECUTABLE"
install_name_tool \
  -id "@rpath/libphotara_bridge.dylib" \
  "$FRAMEWORKS/libphotara_bridge.dylib"

# Cloud-backed working directories may attach Finder/resource-fork metadata to
# generated bundle contents. That metadata is not part of the product and makes
# codesign reject an otherwise valid development build.
xattr -cr "$APP_BUNDLE"
codesign --force --sign "$SIGNING_IDENTITY" --timestamp=none "$PROXY_HELPER"
codesign --force --sign "$SIGNING_IDENTITY" --timestamp=none "$FRAMEWORKS/libphotara_bridge.dylib"
xattr -cr "$APP_BUNDLE"
if [[ -n "$SIGNING_PROFILE" ]]; then
  cp -p "$SIGNING_PROFILE" "$CONTENTS/embedded.provisionprofile"
  codesign --force --sign "$SIGNING_IDENTITY" --entitlements "$SIGNING_ENTITLEMENTS" \
    --timestamp=none --generate-entitlement-der "$APP_BUNDLE"
else
  codesign --force --sign - "$APP_BUNDLE"
fi
codesign --verify --strict --verbose=2 "$APP_BUNDLE"

print -r -- "$APP_BUNDLE"
