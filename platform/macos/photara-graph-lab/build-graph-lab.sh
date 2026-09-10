#!/bin/zsh
set -euo pipefail

SCRIPT_ROOT="${0:A:h}"
REPOSITORY_ROOT="${SCRIPT_ROOT:h:h:h}"
BUILD_ROOT="$SCRIPT_ROOT/.build"
MODULE_CACHE="$BUILD_ROOT/module-cache"
APP_BUNDLE="$BUILD_ROOT/Photara Graph Lab.app"
CONTENTS="$APP_BUNDLE/Contents"
MACOS="$CONTENTS/MacOS"
RESOURCES="$CONTENTS/Resources"
THEME_ROOT="$REPOSITORY_ROOT/platform/macos/photara-theme"
GRAPH_ICON_ROOT="$REPOSITORY_ROOT/platform/macos/photara-graph/Resources/NodeIcons"
GRAPH_TOOL_ICON_ROOT="$REPOSITORY_ROOT/platform/macos/photara-graph/Resources/ToolIcons"

mkdir -p "$MODULE_CACHE" "$MACOS" "$RESOURCES"
cp -p "$SCRIPT_ROOT/Resources/Info.plist" "$CONTENTS/Info.plist"
cp -p "$THEME_ROOT/Resources/photara-default.json" "$RESOURCES/photara-default.json"
cp -p "$REPOSITORY_ROOT/platform/macos/photara-graph/Resources/photara-graph-presentation-v1.json" "$RESOURCES/photara-graph-presentation-v1.json"
ditto "$GRAPH_ICON_ROOT" "$RESOURCES/NodeIcons"
ditto "$GRAPH_TOOL_ICON_ROOT" "$RESOURCES/ToolIcons"

xcrun swiftc \
  -swift-version 6 \
  -O \
  -parse-as-library \
  -module-cache-path "$MODULE_CACHE" \
  "$THEME_ROOT/Sources/PhotaraTheme.swift" \
  "$REPOSITORY_ROOT/platform/macos/photara-app/Sources/ThemeStore.swift" \
  "$REPOSITORY_ROOT/platform/macos/photara-graph/Sources/GraphPresentation.swift" \
  "$REPOSITORY_ROOT/platform/macos/photara-graph/Sources/GraphPreset.swift" \
  "$REPOSITORY_ROOT/platform/macos/photara-graph/Sources/GraphDocument.swift" \
  "$REPOSITORY_ROOT/platform/macos/photara-graph/Sources/GraphOverview.swift" \
  "$REPOSITORY_ROOT/platform/macos/photara-graph/Sources/GraphToolIcons.swift" \
  "$REPOSITORY_ROOT/platform/macos/photara-graph/Sources/GraphGeometry.swift" \
  "$REPOSITORY_ROOT/platform/macos/photara-graph/Sources/GraphInteraction.swift" \
  "$REPOSITORY_ROOT/platform/macos/photara-graph/Sources/GraphEventSurface.swift" \
  "$REPOSITORY_ROOT/platform/macos/photara-graph/Sources/GraphToolRail.swift" \
  "$REPOSITORY_ROOT/platform/macos/photara-graph/Sources/GraphNodeView.swift" \
  "$REPOSITORY_ROOT/platform/macos/photara-graph/Sources/GraphCanvas.swift" \
  "$SCRIPT_ROOT/Sources/GraphLabFixtures.swift" \
  "$SCRIPT_ROOT/Sources/GraphLabTooling.swift" \
  "$SCRIPT_ROOT/Sources/GraphLabCanvas.swift" \
  "$SCRIPT_ROOT/Sources/GraphLabView.swift" \
  "$SCRIPT_ROOT/Sources/GraphLabApp.swift" \
  -framework SwiftUI \
  -framework AppKit \
  -o "$MACOS/PhotaraGraphLab"

codesign --force --deep --sign - "$APP_BUNDLE"
print -r -- "$APP_BUNDLE"
