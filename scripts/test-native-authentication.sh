#!/bin/zsh
set -euo pipefail
SCRIPT_ROOT="${0:A:h}"
REPOSITORY_ROOT="${SCRIPT_ROOT:h}"
FOUNDATION="$REPOSITORY_ROOT/platform/macos/photara-ui-foundation"
BUILD_ROOT="$FOUNDATION/.build/native-authentication"
mkdir -p "$BUILD_ROOT/module-cache" "$BUILD_ROOT/generated"
python3 "$SCRIPT_ROOT/generate_product_configuration.py" --channel development \
  --output "$BUILD_ROOT/generated" >/dev/null
xcrun swiftc -swift-version 6 -warnings-as-errors -parse-as-library \
  -module-cache-path "$BUILD_ROOT/module-cache" \
  "$FOUNDATION/Sources/ReleaseConfiguration.swift" \
  "$BUILD_ROOT/generated/CheckedReleaseConfiguration.swift" \
  "$FOUNDATION/Sources/NativeAuthentication.swift" \
  "$FOUNDATION/Sources/NativeIDToken.swift" \
  "$FOUNDATION/Sources/NativeAccessToken.swift" \
  "$FOUNDATION/Sources/NativeOnboardingState.swift" \
  "$FOUNDATION/Sources/NativeRefresh.swift" \
  "$FOUNDATION/Sources/NativeAuth0Client.swift" \
  "$FOUNDATION/Sources/NativeAccountProfile.swift" \
  "$FOUNDATION/Tests/NativeAuthenticationTests.swift" \
  -o "$BUILD_ROOT/native-authentication-tests"
"$BUILD_ROOT/native-authentication-tests"

xcrun swiftc -swift-version 6 -warnings-as-errors -parse-as-library \
  -module-cache-path "$BUILD_ROOT/module-cache" \
  "$FOUNDATION/Sources/ReleaseConfiguration.swift" \
  "$BUILD_ROOT/generated/CheckedReleaseConfiguration.swift" \
  "$FOUNDATION"/Sources/Native*.swift \
  "$FOUNDATION/Tests/NativeAuth0ClientTests.swift" \
  -o "$BUILD_ROOT/native-auth0-client-tests"
"$BUILD_ROOT/native-auth0-client-tests"

xcrun swiftc -swift-version 6 -warnings-as-errors -parse-as-library \
  -module-cache-path "$BUILD_ROOT/module-cache" \
  "$FOUNDATION/Sources/ReleaseConfiguration.swift" \
  "$BUILD_ROOT/generated/CheckedReleaseConfiguration.swift" \
  "$FOUNDATION"/Sources/Native*.swift \
  "$REPOSITORY_ROOT/platform/macos/photara-shell/Sources/OpeningCloudModel.swift" \
  "$FOUNDATION/Tests/OpeningCloudModelTests.swift" \
  -o "$BUILD_ROOT/opening-cloud-model-tests"
"$BUILD_ROOT/opening-cloud-model-tests"
