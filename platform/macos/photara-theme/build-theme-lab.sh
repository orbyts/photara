#!/bin/zsh
set -euo pipefail

SCRIPT_ROOT="${0:A:h}"
REPOSITORY_ROOT="${SCRIPT_ROOT:h:h:h}"
BUILD_ROOT="$SCRIPT_ROOT/.build"
MODULE_CACHE="$BUILD_ROOT/module-cache"
APP_BUNDLE="$BUILD_ROOT/Photara Theme Lab.app"
CONTENTS="$APP_BUNDLE/Contents"
MACOS="$CONTENTS/MacOS"
FRAMEWORKS="$CONTENTS/Frameworks"
RESOURCES="$CONTENTS/Resources"
PHOTARA_APP_ROOT="$REPOSITORY_ROOT/platform/macos/photara-app"
PHOTARA_APP_BUILD="$PHOTARA_APP_ROOT/.build/app"
PHOTARA_APP_BUNDLE="$PHOTARA_APP_BUILD/Photara.app"
GENERATED_ROOT="$PHOTARA_APP_BUILD/generated"

"$PHOTARA_APP_ROOT/build-app.sh" >/dev/null

mkdir -p "$MODULE_CACHE" "$MACOS" "$FRAMEWORKS" "$RESOURCES"
cp -p "$SCRIPT_ROOT/Resources/ThemeLab-Info.plist" "$CONTENTS/Info.plist"
cp -p "$SCRIPT_ROOT/Resources/photara-default.json" "$RESOURCES/photara-default.json"
cp -p \
  "$PHOTARA_APP_BUNDLE/Contents/Frameworks/libphotara_bridge.dylib" \
  "$FRAMEWORKS/libphotara_bridge.dylib"
cp -p \
  "$PHOTARA_APP_BUNDLE/Contents/MacOS/photara-proxy-imageio" \
  "$MACOS/photara-proxy-imageio"

xcrun swiftc \
  -swift-version 6 \
  -parse-as-library \
  -module-cache-path "$MODULE_CACHE" \
  "$GENERATED_ROOT/PhotaraBridge.swift" \
  "$SCRIPT_ROOT/Sources/PhotaraTheme.swift" \
  "$REPOSITORY_ROOT/platform/macos/photara-graph/Sources/GraphPresentation.swift" \
  "$PHOTARA_APP_ROOT/Sources/GalleryPresentationState.swift" \
  "$PHOTARA_APP_ROOT/Sources/ThemeStore.swift" \
  "$PHOTARA_APP_ROOT/Sources/AppModel.swift" \
  "$PHOTARA_APP_ROOT/Sources/AppModel+Gallery.swift" \
  "$PHOTARA_APP_ROOT/Sources/WorkspaceModel.swift" \
  "$PHOTARA_APP_ROOT/Sources/GalleryView.swift" \
  "$PHOTARA_APP_ROOT/Sources/WorkspaceView.swift" \
  "$SCRIPT_ROOT/Sources/ThemeLabView.swift" \
  "$SCRIPT_ROOT/Sources/ThemeLabApp.swift" \
  -Xcc "-fmodule-map-file=$GENERATED_ROOT/PhotaraBridgeFFI.modulemap" \
  -L "$FRAMEWORKS" \
  -lphotara_bridge \
  -framework SwiftUI \
  -framework AppKit \
  -framework QuickLookThumbnailing \
  -Xlinker -rpath \
  -Xlinker "@executable_path/../Frameworks" \
  -o "$MACOS/PhotaraThemeLab"

install_name_tool \
  -change "$PHOTARA_APP_BUILD/rust-target/debug/deps/libphotara_bridge.dylib" \
  "@rpath/libphotara_bridge.dylib" \
  "$MACOS/PhotaraThemeLab"
install_name_tool \
  -id "@rpath/libphotara_bridge.dylib" \
  "$FRAMEWORKS/libphotara_bridge.dylib"

xcrun swiftc \
  -swift-version 6 \
  -parse-as-library \
  -module-cache-path "$MODULE_CACHE" \
  "$SCRIPT_ROOT/Sources/PhotaraTheme.swift" \
  "$SCRIPT_ROOT/Sources/ThemeCLI.swift" \
  -framework SwiftUI \
  -framework AppKit \
  -o "$BUILD_ROOT/photara-theme"

codesign --force --sign - "$MACOS/photara-proxy-imageio"
codesign --force --sign - "$FRAMEWORKS/libphotara_bridge.dylib"
codesign --force --deep --sign - "$APP_BUNDLE"
print -r -- "$APP_BUNDLE"
print -r -- "$BUILD_ROOT/photara-theme"
