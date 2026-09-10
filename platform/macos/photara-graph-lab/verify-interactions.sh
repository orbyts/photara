#!/bin/zsh
set -euo pipefail
SCRIPT_ROOT="${0:A:h}"
REPOSITORY_ROOT="${SCRIPT_ROOT:h:h:h}"
GRAPH_ROOT="$REPOSITORY_ROOT/platform/macos/photara-graph/Sources"
THEME_ROOT="$REPOSITORY_ROOT/platform/macos/photara-theme"
GRAPH_ICON_ROOT="$REPOSITORY_ROOT/platform/macos/photara-graph/Resources/NodeIcons"
GRAPH_TOOL_ICON_ROOT="$REPOSITORY_ROOT/platform/macos/photara-graph/Resources/ToolIcons"
BUILD_ROOT="$SCRIPT_ROOT/.build/verification"
APP_BUNDLE="$BUILD_ROOT/Graph Lab Verification.app"
mkdir -p "$APP_BUNDLE/Contents/MacOS" "$APP_BUNDLE/Contents/Resources" /tmp/photara-graph-verification
cp -p "$SCRIPT_ROOT/Resources/Info.plist" "$APP_BUNDLE/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Set :CFBundleIdentifier com.photara.graph-lab.verification' "$APP_BUNDLE/Contents/Info.plist"
cp -p "$THEME_ROOT/Resources/photara-default.json" "$APP_BUNDLE/Contents/Resources/photara-default.json"
cp -p "$REPOSITORY_ROOT/platform/macos/photara-graph/Resources/photara-graph-presentation-v1.json" "$APP_BUNDLE/Contents/Resources/photara-graph-presentation-v1.json"
ditto "$GRAPH_ICON_ROOT" "$APP_BUNDLE/Contents/Resources/NodeIcons"
ditto "$GRAPH_TOOL_ICON_ROOT" "$APP_BUNDLE/Contents/Resources/ToolIcons"
xcrun swiftc -swift-version 6 -O -parse-as-library -module-cache-path "$SCRIPT_ROOT/.build/module-cache" \
  "$THEME_ROOT/Sources/PhotaraTheme.swift" \
  "$REPOSITORY_ROOT/platform/macos/photara-app/Sources/ThemeStore.swift" \
  "$GRAPH_ROOT/GraphPresentation.swift" "$GRAPH_ROOT/GraphPreset.swift" "$GRAPH_ROOT/GraphDocument.swift" \
  "$GRAPH_ROOT/GraphOverview.swift" "$GRAPH_ROOT/GraphToolIcons.swift" "$GRAPH_ROOT/GraphGeometry.swift" "$GRAPH_ROOT/GraphInteraction.swift" "$GRAPH_ROOT/GraphEventSurface.swift" \
  "$GRAPH_ROOT/GraphToolRail.swift" "$GRAPH_ROOT/GraphNodeView.swift" "$GRAPH_ROOT/GraphCanvas.swift" \
  "$SCRIPT_ROOT/Sources/GraphLabFixtures.swift" "$SCRIPT_ROOT/Sources/GraphLabTooling.swift" "$SCRIPT_ROOT/Sources/GraphLabCanvas.swift" \
  "$SCRIPT_ROOT/Sources/GraphLabView.swift" "$SCRIPT_ROOT/Tests/GraphInteractionChecks.swift" \
  "$SCRIPT_ROOT/Tests/GraphBranchOverviewChecks.swift" "$SCRIPT_ROOT/Tests/GraphGestureOracle.swift" "$SCRIPT_ROOT/Tests/GraphRandomGestures.swift" \
  -framework SwiftUI -framework AppKit -o "$APP_BUNDLE/Contents/MacOS/PhotaraGraphLab"
codesign --force --deep --sign - "$APP_BUNDLE"
# Read the author's existing payload without ever writing their preference domain.
# The verification app adds the author domain as a read-only fallback suite.
"$APP_BUNDLE/Contents/MacOS/PhotaraGraphLab" "$@"
