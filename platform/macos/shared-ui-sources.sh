# Sourced by zsh build hosts after REPOSITORY_ROOT is defined.
# One source manifest for every production-shared native feature. No bridge or fixtures.
UI_ROOT="$REPOSITORY_ROOT/platform/macos"
PRODUCT_CHANNEL="${PHOTARA_RELEASE_CHANNEL:-development}"
PRODUCT_GENERATED="$REPOSITORY_ROOT/platform/macos/photara-ui-foundation/.build/product-identity/$PRODUCT_CHANNEL"
python3 "$REPOSITORY_ROOT/scripts/generate_product_configuration.py" \
  --channel "$PRODUCT_CHANNEL" --output "$PRODUCT_GENERATED" >/dev/null
SHARED_UI_SOURCES=(
  "$PRODUCT_GENERATED/CheckedReleaseConfiguration.swift"
  "$UI_ROOT/photara-theme/Sources/PhotaraTheme.swift"
  "$UI_ROOT"/photara-ui-foundation/Sources/*.swift
  "$UI_ROOT"/photara-graph/Sources/*.swift
  "$UI_ROOT"/photara-gallery/Sources/*.swift
  "$UI_ROOT"/photara-inspector/Sources/*.swift
  "$UI_ROOT"/photara-shell/Sources/*.swift
  "$UI_ROOT"/photara-layout/Sources/*.swift
  "$UI_ROOT"/photara-library-ui/Sources/*.swift
  "$UI_ROOT"/photara-people/Sources/*.swift
  "$UI_ROOT"/photara-locations/Sources/*.swift
  "$UI_ROOT"/photara-scenes/Sources/*.swift
  "$UI_ROOT"/photara-project-info/Sources/*.swift
)
