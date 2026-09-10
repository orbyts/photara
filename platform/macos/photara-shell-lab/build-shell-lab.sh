#!/bin/zsh
set -euo pipefail
SCRIPT_ROOT="${0:A:h}"
REPOSITORY_ROOT="${SCRIPT_ROOT:h:h:h}"
source "$REPOSITORY_ROOT/platform/macos/shared-ui-sources.sh"
BUILD_ROOT="$SCRIPT_ROOT/.build"
APP_BUNDLE="$BUILD_ROOT/Photara Shell Lab.app"
RESOURCES="$APP_BUNDLE/Contents/Resources"
mkdir -p "$RESOURCES" "$APP_BUNDLE/Contents/MacOS" "$BUILD_ROOT/module-cache"
cp -p "$SCRIPT_ROOT/Resources/Info.plist" "$APP_BUNDLE/Contents/Info.plist"
cp -p "$UI_ROOT/photara-theme/Resources/photara-default.json" "$RESOURCES/"
cp -p "$UI_ROOT/photara-shell/Resources/photara-application-presentation-v1.json" "$RESOURCES/"
cp -p "$UI_ROOT/photara-gallery/Resources/photara-gallery-presentation-v1.json" "$RESOURCES/"
cp -p "$UI_ROOT/photara-inspector/Resources/photara-inspector-presentation-v1.json" "$RESOURCES/"
cp -p "$UI_ROOT/photara-graph/Resources/photara-graph-presentation-v1.json" "$RESOURCES/"
ditto "$UI_ROOT/photara-graph/Resources/NodeIcons" "$RESOURCES/NodeIcons"
ditto "$UI_ROOT/photara-graph/Resources/ToolIcons" "$RESOURCES/ToolIcons"
xcrun swiftc -swift-version 6 -parse-as-library -module-cache-path "$BUILD_ROOT/module-cache" \
  "${SHARED_UI_SOURCES[@]}" "$UI_ROOT"/photara-lab-support/Sources/*.swift \
  "$SCRIPT_ROOT"/Sources/*.swift -framework SwiftUI -framework AppKit \
  -o "$APP_BUNDLE/Contents/MacOS/PhotaraShellLab"
xattr -cr "$APP_BUNDLE"
codesign --force --deep --sign - "$APP_BUNDLE"
print -r -- "$APP_BUNDLE"
