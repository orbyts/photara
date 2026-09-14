#!/bin/zsh
set -euo pipefail

SCRIPT_ROOT="${0:A:h}"
BUILD_ROOT="${PHOTARA_MIGRATION_BUILD_ROOT:-/private/tmp/photara-development-credential-migration}"
PROFILE="${PHOTARA_OPERATOR_PROVISIONING_PROFILE:-}"
BUNDLE_ID="com.photara.operator.development"
APP="$BUILD_ROOT/Photara Development Credential Migration.app"
CONTENTS="$APP/Contents"
MACOS="$CONTENTS/MacOS"
# The recovery helper intentionally admits only this basename.
EXECUTABLE="$MACOS/photara-service"
ENTITLEMENTS="$BUILD_ROOT/migration.entitlements"

if [[ -z "$PROFILE" ]]; then
  print -u2 -- "photara-signing:missing-operator-profile"
  exit 1
fi
IDENTITY="$(python3 "$SCRIPT_ROOT/prepare_macos_signing.py" \
  --profile "$PROFILE" --bundle-id "$BUNDLE_ID" --entitlements "$ENTITLEMENTS")"

mkdir -p "$MACOS"
swiftc -swift-version 6 -warnings-as-errors \
  "$SCRIPT_ROOT/photara-development-credential-migration.swift" \
  -framework Foundation -framework Security -framework LocalAuthentication \
  -o "$EXECUTABLE"
plutil -create xml1 "$CONTENTS/Info.plist"
plutil -insert CFBundleDevelopmentRegion -string en "$CONTENTS/Info.plist"
plutil -insert CFBundleExecutable -string photara-service "$CONTENTS/Info.plist"
plutil -insert CFBundleIdentifier -string "$BUNDLE_ID" "$CONTENTS/Info.plist"
plutil -insert CFBundleInfoDictionaryVersion -string 6.0 "$CONTENTS/Info.plist"
plutil -insert CFBundleName -string "Photara Development Credential Migration" "$CONTENTS/Info.plist"
plutil -insert CFBundlePackageType -string APPL "$CONTENTS/Info.plist"
cp -p "$PROFILE" "$CONTENTS/embedded.provisionprofile"
xattr -cr "$APP"
codesign --force --sign "$IDENTITY" --entitlements "$ENTITLEMENTS" \
  --timestamp=none --generate-entitlement-der "$APP"
codesign --verify --strict --verbose=2 "$APP"
print -r -- "$APP"
