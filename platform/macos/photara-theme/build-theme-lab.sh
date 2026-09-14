#!/bin/zsh
set -euo pipefail

SCRIPT_ROOT="${0:A:h}"
REPOSITORY_ROOT="${SCRIPT_ROOT:h:h:h}"
BUILD_ROOT="$SCRIPT_ROOT/.build"
MODULE_CACHE="$BUILD_ROOT/module-cache"
APP_BUNDLE="$BUILD_ROOT/Photara Theme Lab.app"
CONTENTS="$APP_BUNDLE/Contents"
MACOS="$CONTENTS/MacOS"
RESOURCES="$CONTENTS/Resources"
source "$REPOSITORY_ROOT/platform/macos/shared-ui-sources.sh"
mkdir -p "$MODULE_CACHE" "$MACOS" "$RESOURCES"
cp -p "$SCRIPT_ROOT/Resources/ThemeLab-Info.plist" "$CONTENTS/Info.plist"
cp -p "$SCRIPT_ROOT/Resources/photara-default.json" "$RESOURCES/photara-default.json"
cp -p "$UI_ROOT/photara-shell/Resources/photara-application-presentation-v1.json" "$RESOURCES/"

xcrun swiftc \
  -target arm64-apple-macosx26.0 \
  -swift-version 6 \
  -parse-as-library \
  -module-cache-path "$MODULE_CACHE" \
  "${SHARED_UI_SOURCES[@]}" \
  "$SCRIPT_ROOT/Sources/ThemeLabView.swift" \
  "$SCRIPT_ROOT/Sources/ThemeLabApp.swift" \
  -framework SwiftUI \
  -framework AppKit \
  -o "$MACOS/PhotaraThemeLab"

xcrun swiftc \
  -target arm64-apple-macosx26.0 \
  -swift-version 6 \
  -parse-as-library \
  -module-cache-path "$MODULE_CACHE" \
  "$SCRIPT_ROOT/Sources/PhotaraTheme.swift" \
  "$SCRIPT_ROOT/Sources/ThemeCLI.swift" \
  -framework SwiftUI \
  -framework AppKit \
  -o "$BUILD_ROOT/photara-theme"

# Strip generated cloud-provider metadata before signing the development
# bundle. Finder/resource-fork attributes are not application resources.
xattr -cr "$APP_BUNDLE"
codesign --force --deep --sign - "$APP_BUNDLE"
print -r -- "$APP_BUNDLE"
print -r -- "$BUILD_ROOT/photara-theme"
