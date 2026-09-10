#!/bin/zsh
set -euo pipefail
UI_ROOT="${0:A:h}"
FEATURE="$1"
case "$FEATURE" in
  gallery) LAB_NAME="Gallery" ;;
  inspector) LAB_NAME="Inspector" ;;
  *) print -u2 "Expected gallery or inspector"; exit 2 ;;
esac
LAB_ROOT="$UI_ROOT/photara-$FEATURE-lab"
BUILD_ROOT="$LAB_ROOT/.build"
APP_BUNDLE="$BUILD_ROOT/Photara $LAB_NAME Lab.app"
RESOURCES="$APP_BUNDLE/Contents/Resources"
mkdir -p "$RESOURCES" "$APP_BUNDLE/Contents/MacOS" "$BUILD_ROOT/module-cache"
cp -p "$LAB_ROOT/Resources/Info.plist" "$APP_BUNDLE/Contents/Info.plist"
cp -p "$UI_ROOT/photara-theme/Resources/photara-default.json" "$RESOURCES/"
if [[ "$FEATURE" == gallery ]]; then
  cp -p "$UI_ROOT/photara-gallery/Resources/photara-gallery-presentation-v1.json" "$RESOURCES/"
else
  cp -p "$UI_ROOT/photara-inspector/Resources/photara-inspector-presentation-v1.json" "$RESOURCES/"
fi
xcrun swiftc -swift-version 6 -parse-as-library -module-cache-path "$BUILD_ROOT/module-cache" \
  "$UI_ROOT/photara-theme/Sources/PhotaraTheme.swift" \
  "$UI_ROOT"/photara-ui-foundation/Sources/*.swift \
  "$UI_ROOT/photara-lab-support/Sources/LabAppearance.swift" \
  "$UI_ROOT/photara-lab-support/Sources/${LAB_NAME}Fixtures.swift" \
  "$UI_ROOT/photara-$FEATURE"/Sources/*.swift "$LAB_ROOT"/Sources/*.swift \
  -framework SwiftUI -framework AppKit -o "$APP_BUNDLE/Contents/MacOS/Photara${LAB_NAME}Lab"
codesign --force --deep --sign - "$APP_BUNDLE"
print -r -- "$APP_BUNDLE"
