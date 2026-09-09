#!/bin/zsh
set -euo pipefail
SCRIPT_ROOT="${0:A:h}"
REPOSITORY_ROOT="${SCRIPT_ROOT:h:h:h}"
GRAPH_ROOT="$REPOSITORY_ROOT/platform/macos/photara-graph/Sources"
THEME_ROOT="$REPOSITORY_ROOT/platform/macos/photara-theme"
GRAPH_ICON_ROOT="$REPOSITORY_ROOT/platform/macos/photara-graph/Resources/NodeIcons"
BUILD_ROOT="$SCRIPT_ROOT/.build/verification"
APP_BUNDLE="$BUILD_ROOT/Graph Lab Verification.app"
mkdir -p "$APP_BUNDLE/Contents/MacOS" "$APP_BUNDLE/Contents/Resources" /tmp/photara-graph-verification
cp -p "$SCRIPT_ROOT/Resources/Info.plist" "$APP_BUNDLE/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Set :CFBundleIdentifier com.photara.graph-lab.verification' "$APP_BUNDLE/Contents/Info.plist"
cp -p "$THEME_ROOT/Resources/photara-default.json" "$APP_BUNDLE/Contents/Resources/photara-default.json"
ditto "$GRAPH_ICON_ROOT" "$APP_BUNDLE/Contents/Resources/NodeIcons"
xcrun swiftc -swift-version 6 -O -parse-as-library -module-cache-path "$SCRIPT_ROOT/.build/module-cache" \
  "$THEME_ROOT/Sources/PhotaraTheme.swift" \
  "$REPOSITORY_ROOT/platform/macos/photara-app/Sources/ThemeStore.swift" \
  "$GRAPH_ROOT/GraphPresentation.swift" "$GRAPH_ROOT/GraphDocument.swift" \
  "$GRAPH_ROOT/GraphGeometry.swift" "$GRAPH_ROOT/GraphInteraction.swift" "$GRAPH_ROOT/GraphEventSurface.swift" \
  "$SCRIPT_ROOT/Sources/GraphLabFixtures.swift" "$SCRIPT_ROOT/Sources/GraphLabCanvas.swift" \
  "$SCRIPT_ROOT/Sources/GraphLabView.swift" "$SCRIPT_ROOT/Tests/GraphInteractionChecks.swift" \
  "$SCRIPT_ROOT/Tests/GraphGestureOracle.swift" "$SCRIPT_ROOT/Tests/GraphRandomGestures.swift" \
  -framework SwiftUI -framework AppKit -o "$APP_BUNDLE/Contents/MacOS/PhotaraGraphLab"
codesign --force --deep --sign - "$APP_BUNDLE"
# Read the author's existing payload without ever writing their preference domain.
# The verification app adds the author domain as a read-only fallback suite.
"$APP_BUNDLE/Contents/MacOS/PhotaraGraphLab" "$@"
