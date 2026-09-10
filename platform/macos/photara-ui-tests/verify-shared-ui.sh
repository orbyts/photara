#!/bin/zsh
set -euo pipefail
SCRIPT_ROOT="${0:A:h}"
REPOSITORY_ROOT="${SCRIPT_ROOT:h:h:h}"
source "$REPOSITORY_ROOT/platform/macos/shared-ui-sources.sh"
BUILD_ROOT="$SCRIPT_ROOT/.build"
APP_BUNDLE="$BUILD_ROOT/Shared UI Verification.app"
RESOURCES="$APP_BUNDLE/Contents/Resources"
mkdir -p "$RESOURCES" "$APP_BUNDLE/Contents/MacOS" "$BUILD_ROOT/module-cache"
cp -p "$UI_ROOT/photara-gallery-lab/Resources/Info.plist" "$APP_BUNDLE/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Set :CFBundleExecutable SharedUIChecks' "$APP_BUNDLE/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Set :CFBundleIdentifier com.photara.shared-ui-verification' "$APP_BUNDLE/Contents/Info.plist"
cp -p "$UI_ROOT/photara-theme/Resources/photara-default.json" "$RESOURCES/"
cp -p "$UI_ROOT/photara-gallery/Resources/photara-gallery-presentation-v1.json" "$RESOURCES/"
xcrun swiftc -swift-version 6 -parse-as-library -module-cache-path "$BUILD_ROOT/module-cache" \
  "${SHARED_UI_SOURCES[@]}" "$UI_ROOT"/photara-lab-support/Sources/*.swift \
  "$SCRIPT_ROOT/Capture.swift" "$SCRIPT_ROOT/SharedUIChecks.swift" \
  -framework SwiftUI -framework AppKit -o "$APP_BUNDLE/Contents/MacOS/SharedUIChecks"
codesign --force --deep --sign - "$APP_BUNDLE"
if [[ "${1:-}" != --build-only ]]; then
  "$APP_BUNDLE/Contents/MacOS/SharedUIChecks" "$BUILD_ROOT/snapshots"
fi
