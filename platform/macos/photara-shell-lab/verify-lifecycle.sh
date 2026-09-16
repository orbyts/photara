#!/bin/zsh
set -euo pipefail
SCRIPT_ROOT="${0:A:h}"
REPOSITORY_ROOT="${SCRIPT_ROOT:h:h:h}"
BUILD_ROOT="$SCRIPT_ROOT/.build/lifecycle-verification"
APP_BUNDLE="$BUILD_ROOT/Library Lifecycle Verification.app"
mkdir -p "$APP_BUNDLE/Contents/MacOS" "$BUILD_ROOT/module-cache"
cp -p "$SCRIPT_ROOT/Resources/Info.plist" "$APP_BUNDLE/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Set :CFBundleExecutable LifecycleChecks' "$APP_BUNDLE/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Set :CFBundleIdentifier com.photara.lifecycle-verification' "$APP_BUNDLE/Contents/Info.plist"
xcrun swiftc -swift-version 6 -parse-as-library -module-cache-path "$BUILD_ROOT/module-cache" \
 "$REPOSITORY_ROOT/platform/macos/photara-lab-support/Sources/LibraryLifecycleFixture.swift" \
 "$SCRIPT_ROOT/Verification/LifecycleCapture.swift" \
 "$SCRIPT_ROOT/Verification/LifecycleChecks.swift" -framework SwiftUI -framework AppKit \
 -o "$APP_BUNDLE/Contents/MacOS/LifecycleChecks"
xcrun swiftc -swift-version 6 -parse-as-library -module-cache-path "$BUILD_ROOT/module-cache" \
 "$SCRIPT_ROOT/Verification/SwitcherAccessibilityProbe.swift" -framework AppKit \
 -o "$APP_BUNDLE/Contents/MacOS/SwitcherAccessibilityProbe"
codesign --force --deep --sign - "$APP_BUNDLE"
if [[ "${1:-}" != --build-only ]]; then
 "$APP_BUNDLE/Contents/MacOS/LifecycleChecks" "${1:-$BUILD_ROOT/snapshots}"
fi
