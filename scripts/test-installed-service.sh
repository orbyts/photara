#!/bin/zsh
set -euo pipefail
SCRIPT_ROOT="${0:A:h}"
REPOSITORY_ROOT="${SCRIPT_ROOT:h}"
FOUNDATION="$REPOSITORY_ROOT/platform/macos/photara-ui-foundation"
BUILD_ROOT="$(mktemp -d /private/tmp/photara-installed-service-smoke.XXXXXX)"
APP="$BUILD_ROOT/Installed Service Smoke.app"
PROFILE="${PHOTARA_MACOS_PROVISIONING_PROFILE:?Set the desktop profile}"
mkdir -p "$APP/Contents/MacOS" "$BUILD_ROOT/generated" "$BUILD_ROOT/module-cache"
python3 "$SCRIPT_ROOT/generate_product_configuration.py" --channel development --output "$BUILD_ROOT/generated" --plist "$APP/Contents/Info.plist" >/dev/null
IDENTITY="$(python3 "$SCRIPT_ROOT/prepare_macos_signing.py" --profile "$PROFILE" --bundle-id com.photara.desktop --entitlements "$BUILD_ROOT/generated/Smoke.entitlements")"
xcrun swiftc -swift-version 6 -warnings-as-errors -parse-as-library -D PHOTARA_SERVICE_FURNACE -module-cache-path "$BUILD_ROOT/module-cache" \
  "$FOUNDATION/Sources/ReleaseConfiguration.swift" "$BUILD_ROOT/generated/CheckedReleaseConfiguration.swift" \
  "$FOUNDATION"/Sources/Native*.swift "$REPOSITORY_ROOT/platform/macos/photara-app/Tests/ServiceFurnace/InstalledServiceSmoke.swift" -o "$APP/Contents/MacOS/Photara"
cp -p "$PROFILE" "$APP/Contents/embedded.provisionprofile"
codesign --force --sign "$IDENTITY" --entitlements "$BUILD_ROOT/generated/Smoke.entitlements" --timestamp=none "$APP"
codesign --verify --strict "$APP"
python3 - "$APP/Contents/MacOS/Photara" <<'PY'
import subprocess,sys
command=['/usr/sbin/lsof','-nP','-a','-iTCP:8080','-sTCP:LISTEN','-t']
before=subprocess.check_output(command).strip()
subprocess.run([sys.argv[1]],check=True,timeout=100)
after=subprocess.check_output(command).strip()
assert before == after, 'Shared service ownership changed'
print('installed-service-smoke: shared port owner unchanged; no production service lifecycle mutation')
PY
