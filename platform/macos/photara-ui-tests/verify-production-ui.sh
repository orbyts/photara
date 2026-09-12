#!/bin/zsh
set -euo pipefail
SCRIPT_ROOT="${0:A:h}"
REPOSITORY_ROOT="${SCRIPT_ROOT:h:h:h}"
source "$REPOSITORY_ROOT/platform/macos/shared-ui-sources.sh"
PRODUCTION_ROOT="$UI_ROOT/photara-app"
PRODUCTION_BUILD="$PRODUCTION_ROOT/.build/app"
PRODUCTION_RUST_TARGET="${PHOTARA_APP_RUST_TARGET:-$PRODUCTION_BUILD/rust-target}"
GENERATED_ROOT="$PRODUCTION_BUILD/generated"
BUILD_ROOT="$SCRIPT_ROOT/.build"
APP_BUNDLE="$BUILD_ROOT/Production UI Verification.app"
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
PRODUCTION_SOURCES=("${(@)PRODUCTION_SOURCES:#*/WorkspaceModel.swift}")
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
