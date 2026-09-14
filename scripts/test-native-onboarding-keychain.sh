#!/bin/zsh
set -euo pipefail
SCRIPT_ROOT="${0:A:h}"
REPOSITORY_ROOT="${SCRIPT_ROOT:h}"
FOUNDATION="$REPOSITORY_ROOT/platform/macos/photara-ui-foundation"
BUILD_ROOT="$REPOSITORY_ROOT/platform/macos/photara-app/.build/onboarding-keychain-tests"
PROFILE="${PHOTARA_MACOS_PROVISIONING_PROFILE:?Set the desktop development provisioning profile}"
APP="$BUILD_ROOT/Onboarding Keychain Verification.app"
mkdir -p "$APP/Contents/MacOS" "$BUILD_ROOT/generated" "$BUILD_ROOT/module-cache"
python3 "$SCRIPT_ROOT/generate_product_configuration.py" --channel development --output "$BUILD_ROOT/generated" --plist "$APP/Contents/Info.plist" >/dev/null
IDENTIFIER="$(plutil -extract CFBundleIdentifier raw "$APP/Contents/Info.plist")"
EXECUTABLE="$(plutil -extract CFBundleExecutable raw "$APP/Contents/Info.plist")"
IDENTITY="$(python3 "$SCRIPT_ROOT/prepare_macos_signing.py" --profile "$PROFILE" --bundle-id "$IDENTIFIER" --entitlements "$BUILD_ROOT/generated/Probe.entitlements")"
xcrun swiftc -swift-version 6 -warnings-as-errors -parse-as-library -module-cache-path "$BUILD_ROOT/module-cache" \
  "$FOUNDATION/Sources/ReleaseConfiguration.swift" "$BUILD_ROOT/generated/CheckedReleaseConfiguration.swift" \
  "$FOUNDATION/Sources/NativeAuthentication.swift" "$REPOSITORY_ROOT/platform/macos/photara-app/Tests/NativeOnboardingKeychainTests.swift" \
  -o "$APP/Contents/MacOS/$EXECUTABLE"
cp -p "$PROFILE" "$APP/Contents/embedded.provisionprofile"
codesign --force --sign "$IDENTITY" --entitlements "$BUILD_ROOT/generated/Probe.entitlements" --timestamp=none --generate-entitlement-der "$APP"
codesign --verify --strict "$APP"
"$APP/Contents/MacOS/$EXECUTABLE"
