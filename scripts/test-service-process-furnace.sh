#!/bin/zsh
set -euo pipefail
SCRIPT_ROOT="${0:A:h}"
REPOSITORY_ROOT="${SCRIPT_ROOT:h}"
FOUNDATION="$REPOSITORY_ROOT/platform/macos/photara-ui-foundation"
TESTS="$REPOSITORY_ROOT/platform/macos/photara-app/Tests/ServiceFurnace"
BUILD_ROOT="$(mktemp -d /private/tmp/photara-service-furnace.XXXXXX)"
OPERATOR="$BUILD_ROOT/Photara Development Operator.app"
CLIENT="$BUILD_ROOT/Service Furnace.app"
OPERATOR_PROFILE="${PHOTARA_OPERATOR_PROVISIONING_PROFILE:?Set the operator profile}"
APP_PROFILE="${PHOTARA_MACOS_PROVISIONING_PROFILE:?Set the desktop profile}"
mkdir -p "$BUILD_ROOT/generated" "$BUILD_ROOT/module-cache" "$OPERATOR/Contents/MacOS" "$OPERATOR/Contents/Resources" "$CLIENT/Contents/MacOS"
python3 "$SCRIPT_ROOT/generate_product_configuration.py" --channel development --output "$BUILD_ROOT/generated" --plist "$CLIENT/Contents/Info.plist" >/dev/null
python3 - "$OPERATOR/Contents/Info.plist" "$BUILD_ROOT/generated/FurnaceOperatorConfiguration.swift" <<'PY'
import plistlib,sys,uuid
with open(sys.argv[1],'wb') as f:
 plistlib.dump({'CFBundleIdentifier':'com.photara.operator.development','CFBundleExecutable':'photara-development-service','CFBundleName':'Photara Development Operator','CFBundlePackageType':'APPL'},f)
with open(sys.argv[2],'w') as f:
 f.write('enum FurnaceOperatorConfiguration { static let keychainService = "com.photara.operator.furnace.'+str(uuid.uuid4())+'" }\n')
PY
OPERATOR_IDENTITY="$(python3 "$SCRIPT_ROOT/prepare_macos_signing.py" --profile "$OPERATOR_PROFILE" --bundle-id com.photara.operator.development --entitlements "$BUILD_ROOT/generated/Operator.entitlements")"
APP_IDENTITY="$(python3 "$SCRIPT_ROOT/prepare_macos_signing.py" --profile "$APP_PROFILE" --bundle-id com.photara.desktop --entitlements "$BUILD_ROOT/generated/App.entitlements")"
xcrun swiftc -swift-version 6 -warnings-as-errors -module-cache-path "$BUILD_ROOT/module-cache" \
  "$TESTS/HTTPFixture.swift" -o "$OPERATOR/Contents/Resources/photara-service"
cp -p "$SCRIPT_ROOT/photara-development-service.swift" "$BUILD_ROOT/generated/main.swift"
xcrun swiftc -swift-version 6 -warnings-as-errors -D PHOTARA_SERVICE_FURNACE -module-cache-path "$BUILD_ROOT/module-cache" \
  "$BUILD_ROOT/generated/main.swift" "$BUILD_ROOT/generated/FurnaceOperatorConfiguration.swift" -o "$OPERATOR/Contents/MacOS/photara-development-service"
xcrun swiftc -swift-version 6 -warnings-as-errors -parse-as-library -D PHOTARA_SERVICE_FURNACE -module-cache-path "$BUILD_ROOT/module-cache" \
  "$FOUNDATION/Sources/ReleaseConfiguration.swift" "$BUILD_ROOT/generated/CheckedReleaseConfiguration.swift" \
  "$FOUNDATION"/Sources/Native*.swift "$TESTS/Launcher.swift" -o "$CLIENT/Contents/MacOS/Photara"
cp -p "$OPERATOR_PROFILE" "$OPERATOR/Contents/embedded.provisionprofile"
cp -p "$APP_PROFILE" "$CLIENT/Contents/embedded.provisionprofile"
codesign --force --sign "$OPERATOR_IDENTITY" --identifier photara-service --timestamp=none "$OPERATOR/Contents/Resources/photara-service"
codesign --force --sign "$OPERATOR_IDENTITY" --entitlements "$BUILD_ROOT/generated/Operator.entitlements" --timestamp=none "$OPERATOR"
codesign --force --sign "$APP_IDENTITY" --entitlements "$BUILD_ROOT/generated/App.entitlements" --timestamp=none "$CLIENT"
codesign --verify --strict "$OPERATOR"
codesign --verify --strict "$CLIENT"
python3 "$SCRIPT_ROOT/service_process_furnace.py" "$BUILD_ROOT" "$OPERATOR" "$CLIENT/Contents/MacOS/Photara"
print -r -- "Service furnace evidence: $BUILD_ROOT/report.json"
