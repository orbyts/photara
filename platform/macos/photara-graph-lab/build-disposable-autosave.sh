#!/bin/zsh
set -euo pipefail
SCRIPT_ROOT="${0:A:h}"
# Isolated local ad-hoc lab artifact only: no production build, install or launch.
BUILD_ROOT="${PHOTARA_PS3_LAB_BUILD:-$SCRIPT_ROOT/.build/disposable-autosave}"
APP="$BUILD_ROOT/Disposable Autosave.app"
mkdir -p "$APP/Contents/MacOS" "$BUILD_ROOT/module-cache"
cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>DisposableAutosave</string>
<key>CFBundleIdentifier</key><string>com.photara.graph-lab.disposable-autosave</string>
<key>CFBundleName</key><string>Disposable Autosave</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
xcrun swiftc -swift-version 6 -warnings-as-errors -parse-as-library \
  -module-cache-path "$BUILD_ROOT/module-cache" \
  "$SCRIPT_ROOT/Sources/DisposableAutosaveProjection.swift" \
  "$SCRIPT_ROOT/Sources/DisposableAutosaveStatusView.swift" \
  "$SCRIPT_ROOT/Sources/DisposableAutosaveProcess.swift" \
  "$SCRIPT_ROOT/Sources/DisposableAutosaveLabApp.swift" \
  -framework SwiftUI -framework AppKit -o "$APP/Contents/MacOS/DisposableAutosave"
codesign --force --sign - "$APP"
print -r -- "$APP"
