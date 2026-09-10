#!/bin/zsh
set -euo pipefail
UI_ROOT="${0:A:h}"
FEATURE="$1"
case "$FEATURE" in
  people) NAME="People"; STEM="People" ;;
  locations) NAME="Locations"; STEM="Locations" ;;
  scenes) NAME="Scenes"; STEM="Scenes" ;;
  project-info) NAME="Project Info"; STEM="ProjectInfo" ;;
  *) print -u2 "Expected people, locations, scenes or project-info"; exit 2 ;;
esac
LAB_ROOT="$UI_ROOT/photara-$FEATURE-lab"
BUILD_ROOT="$LAB_ROOT/.build"
APP_BUNDLE="$BUILD_ROOT/Photara $NAME Lab.app"
mkdir -p "$APP_BUNDLE/Contents/Resources" "$APP_BUNDLE/Contents/MacOS" "$BUILD_ROOT/module-cache"
cp -p "$LAB_ROOT/Resources/Info.plist" "$APP_BUNDLE/Contents/Info.plist"
cp -p "$UI_ROOT/photara-theme/Resources/photara-default.json" "$APP_BUNDLE/Contents/Resources/"
DEPENDENCY_SOURCES=()
if [[ "$FEATURE" == project-info ]]; then
  DEPENDENCY_SOURCES=("$UI_ROOT"/photara-people/Sources/*.swift "$UI_ROOT"/photara-locations/Sources/*.swift "$UI_ROOT"/photara-scenes/Sources/*.swift)
fi
# Project Info contracts are shared by deterministic fixtures, but other feature renderers are not compiled.
xcrun swiftc -swift-version 6 -parse-as-library -module-cache-path "$BUILD_ROOT/module-cache" \
  "$UI_ROOT/photara-theme/Sources/PhotaraTheme.swift" \
  "$UI_ROOT"/photara-ui-foundation/Sources/*.swift \
  "$UI_ROOT"/photara-library-ui/Sources/*.swift \
  "$UI_ROOT/photara-project-info/Sources/ProjectInfoPresentation.swift" \
  "$UI_ROOT/photara-lab-support/Sources/LabAppearance.swift" \
  "$UI_ROOT/photara-lab-support/Sources/LibraryFixtures.swift" \
  "${DEPENDENCY_SOURCES[@]}" \
  "$UI_ROOT/photara-$FEATURE"/Sources/*View.swift "$LAB_ROOT"/Sources/*.swift \
  -framework SwiftUI -framework AppKit -o "$APP_BUNDLE/Contents/MacOS/Photara${STEM}Lab"
xattr -cr "$APP_BUNDLE"
codesign --force --deep --sign - "$APP_BUNDLE"
print -r -- "$APP_BUNDLE"
