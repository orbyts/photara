#!/bin/zsh
set -euo pipefail

SCRIPT_ROOT="${0:A:h}"
REPOSITORY_ROOT="${SCRIPT_ROOT:h:h:h}"
BUILD_ROOT="$SCRIPT_ROOT/.build/app"
RUST_TARGET="$BUILD_ROOT/rust-target"
GENERATED_ROOT="$BUILD_ROOT/generated"
MODULE_CACHE="$BUILD_ROOT/module-cache"
APP_BUNDLE="$BUILD_ROOT/Photara.app"
CONTENTS="$APP_BUNDLE/Contents"
MACOS="$CONTENTS/MacOS"
FRAMEWORKS="$CONTENTS/Frameworks"
RESOURCES="$CONTENTS/Resources"
THEME_ROOT="$REPOSITORY_ROOT/platform/macos/photara-theme"
EXECUTABLE="$MACOS/Photara"
PROXY_HELPER_BUILD="$BUILD_ROOT/proxy-helper-build"
PROXY_HELPER="$MACOS/photara-proxy-imageio"

mkdir -p "$GENERATED_ROOT" "$MODULE_CACHE" "$MACOS" "$FRAMEWORKS" "$RESOURCES"
cp -p "$SCRIPT_ROOT/Resources/Info.plist" "$CONTENTS/Info.plist"
mkdir -p "$RESOURCES/Themes"
cp -p "$THEME_ROOT/Resources/photara-default.json" "$RESOURCES/Themes/photara-default.json"
cp -p "$REPOSITORY_ROOT/platform/macos/photara-graph/Resources/photara-graph-presentation-v1.json" "$RESOURCES/photara-graph-presentation-v1.json"
ditto "$REPOSITORY_ROOT/platform/macos/photara-graph/Resources/NodeIcons" "$RESOURCES/NodeIcons"
ditto "$REPOSITORY_ROOT/platform/macos/photara-graph/Resources/ToolIcons" "$RESOURCES/ToolIcons"

swift build \
  --package-path "$REPOSITORY_ROOT/platform/macos/photara-proxy-imageio" \
  --scratch-path "$PROXY_HELPER_BUILD"
cp -p "$PROXY_HELPER_BUILD/debug/photara-proxy-imageio" "$PROXY_HELPER"

CARGO_TARGET_DIR="$RUST_TARGET" cargo build \
  --manifest-path "$REPOSITORY_ROOT/Cargo.toml" \
  -p photara-bridge

CARGO_TARGET_DIR="$RUST_TARGET" cargo run \
  --manifest-path "$REPOSITORY_ROOT/Cargo.toml" \
  -p photara-bridge \
  --features bindgen \
  --bin photara-uniffi-bindgen \
  -- generate \
  --library "$RUST_TARGET/debug/libphotara_bridge.dylib" \
  --language swift \
  --out-dir "$GENERATED_ROOT"

cp -p "$RUST_TARGET/debug/libphotara_bridge.dylib" "$FRAMEWORKS/libphotara_bridge.dylib"

xcrun swiftc \
  -swift-version 6 \
  -parse-as-library \
  -module-cache-path "$MODULE_CACHE" \
  "$GENERATED_ROOT/PhotaraBridge.swift" \
  "$THEME_ROOT/Sources/PhotaraTheme.swift" \
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
  "$SCRIPT_ROOT/Sources/GalleryPresentationState.swift" \
  "$SCRIPT_ROOT/Sources/ThemeStore.swift" \
  "$SCRIPT_ROOT/Sources/AppModel.swift" \
  "$SCRIPT_ROOT/Sources/AppModel+Gallery.swift" \
  "$SCRIPT_ROOT/Sources/WorkspaceModel.swift" \
  "$SCRIPT_ROOT/Sources/GraphAdapter.swift" \
  "$SCRIPT_ROOT/Sources/ProductionGraphView.swift" \
  "$SCRIPT_ROOT/Sources/GalleryView.swift" \
  "$SCRIPT_ROOT/Sources/WorkspaceView.swift" \
  "$SCRIPT_ROOT/Sources/PhotaraMacApp.swift" \
  -Xcc "-fmodule-map-file=$GENERATED_ROOT/PhotaraBridgeFFI.modulemap" \
  -L "$FRAMEWORKS" \
  -lphotara_bridge \
  -framework SwiftUI \
  -framework AppKit \
  -framework QuickLookThumbnailing \
  -Xlinker -rpath \
  -Xlinker "@executable_path/../Frameworks" \
  -o "$EXECUTABLE"

install_name_tool \
  -change "$RUST_TARGET/debug/deps/libphotara_bridge.dylib" \
  "@rpath/libphotara_bridge.dylib" \
  "$EXECUTABLE"
install_name_tool \
  -id "@rpath/libphotara_bridge.dylib" \
  "$FRAMEWORKS/libphotara_bridge.dylib"

codesign --force --sign - "$PROXY_HELPER"
codesign --force --sign - "$FRAMEWORKS/libphotara_bridge.dylib"
codesign --force --deep --sign - "$APP_BUNDLE"

print -r -- "$APP_BUNDLE"
