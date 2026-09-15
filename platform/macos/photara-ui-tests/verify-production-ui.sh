#!/bin/zsh
set -euo pipefail
SCRIPT_ROOT="${0:A:h}"
REPOSITORY_ROOT="${SCRIPT_ROOT:h:h:h}"
PRODUCTION_ROOT="$REPOSITORY_ROOT/platform/macos/photara-app"
# A fresh run owns its production input bundle as well as the verification app.
# Never inherit interactive build/signing/output overrides: the complete build
# (Rust, generated bindings/configuration, Swift caches and helper) lives here.
mkdir -p "$SCRIPT_ROOT/.build"
BUILD_ROOT="$(mktemp -d "$SCRIPT_ROOT/.build/production-ui.XXXXXX")"
export CLANG_MODULE_CACHE_PATH="$BUILD_ROOT/module-cache"
export SWIFTPM_MODULECACHE_OVERRIDE="$BUILD_ROOT/module-cache"
export PHOTARA_RELEASE_CHANNEL=development
export PHOTARA_SHARED_UI_GENERATED_ROOT="$BUILD_ROOT/product-identity"
source "$REPOSITORY_ROOT/platform/macos/shared-ui-sources.sh"
PRODUCTION_BUILD="$BUILD_ROOT/production-input"
PRODUCTION_RUST_TARGET="$PRODUCTION_BUILD/rust-target"
GENERATED_ROOT="$PRODUCTION_BUILD/generated"
APP_BUNDLE="$BUILD_ROOT/Production UI Verification.app"
print -r -- "Production UI verification artifacts: $BUILD_ROOT"
PHOTARA_APP_BUILD_ROOT="$PRODUCTION_BUILD" \
  PHOTARA_APP_RUST_TARGET="$PRODUCTION_RUST_TARGET" \
  PHOTARA_RELEASE_CHANNEL=development PHOTARA_MACOS_PROVISIONING_PROFILE= \
  "$PRODUCTION_ROOT/build-app.sh" >/dev/null
mkdir -p "$APP_BUNDLE/Contents/MacOS" "$BUILD_ROOT/module-cache"
cp -p "$PRODUCTION_ROOT/Resources/Info.plist" "$APP_BUNDLE/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Set :CFBundleExecutable ProductionUIChecks' "$APP_BUNDLE/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Set :CFBundleIdentifier com.photara.production-ui-verification' "$APP_BUNDLE/Contents/Info.plist"
ditto "$PRODUCTION_BUILD/Photara.app/Contents/Resources" "$APP_BUNDLE/Contents/Resources"
cp -p "$UI_ROOT/photara-theme/Resources/photara-default.json" "$APP_BUNDLE/Contents/Resources/"
cp -p "$UI_ROOT/photara-gallery/Resources/photara-gallery-presentation-v1.json" "$APP_BUNDLE/Contents/Resources/"
cp -p "$UI_ROOT/photara-inspector/Resources/photara-inspector-presentation-v1.json" "$APP_BUNDLE/Contents/Resources/"
cp -p "$UI_ROOT/photara-shell/Resources/photara-application-presentation-v1.json" "$APP_BUNDLE/Contents/Resources/"
ditto "$PRODUCTION_BUILD/Photara.app/Contents/Frameworks" "$APP_BUNDLE/Contents/Frameworks"
cp -p "$PRODUCTION_BUILD/Photara.app/Contents/MacOS/photara-proxy-imageio" "$APP_BUNDLE/Contents/MacOS/"
PRODUCTION_SOURCES=("$PRODUCTION_ROOT"/Sources/*.swift)
# Shared compatibility paths and the production @main are not compiled twice.
PRODUCTION_SOURCES=("${(@)PRODUCTION_SOURCES:#*/ThemeStore.swift}")
PRODUCTION_SOURCES=("${(@)PRODUCTION_SOURCES:#*/EditorSessionModel.swift}")
PRODUCTION_SOURCES=("${(@)PRODUCTION_SOURCES:#*/PhotaraMacApp.swift}")
xcrun swiftc -swift-version 6 -parse-as-library -module-cache-path "$BUILD_ROOT/module-cache" \
  "$GENERATED_ROOT/PhotaraBridge.swift" "${SHARED_UI_SOURCES[@]}" "${PRODUCTION_SOURCES[@]}" \
  "$UI_ROOT/photara-lab-support/Sources/LabAppearance.swift" \
  "$SCRIPT_ROOT/Capture.swift" "$SCRIPT_ROOT/ProductionUIChecks.swift" \
  -Xcc "-fmodule-map-file=$GENERATED_ROOT/PhotaraBridgeFFI.modulemap" \
  -L "$APP_BUNDLE/Contents/Frameworks" -lphotara_bridge \
  -framework SwiftUI -framework AppKit -framework QuickLookThumbnailing \
  -Xlinker -rpath -Xlinker '@executable_path/../Frameworks' \
  -o "$APP_BUNDLE/Contents/MacOS/ProductionUIChecks"
install_name_tool -change "$PRODUCTION_RUST_TARGET/debug/deps/libphotara_bridge.dylib" \
  '@rpath/libphotara_bridge.dylib' "$APP_BUNDLE/Contents/MacOS/ProductionUIChecks"
codesign --force --deep --sign - "$APP_BUNDLE"
if [[ "${1:-}" != --build-only ]]; then
  "$APP_BUNDLE/Contents/MacOS/ProductionUIChecks" "$BUILD_ROOT/production-snapshots"
fi
