#!/bin/sh
set -eu

BENCH_ROOT=${1:-/private/tmp/photara-proxy-benchmark}
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
FIXTURE_DIR="$BENCH_ROOT/fixtures"
GENERATOR="$BENCH_ROOT/bin/generate-fixtures"
DISPLAY_P3_ICC="/System/Library/ColorSync/Profiles/Display P3.icc"
ACESCG_ICC="/System/Library/ColorSync/Profiles/ACESCG Linear.icc"

mkdir -p "$FIXTURE_DIR" "$BENCH_ROOT/bin"

if [ -e "$FIXTURE_DIR/manifest.json" ]; then
    echo "Fixtures already exist at $FIXTURE_DIR; choose a new benchmark root to regenerate."
    exit 0
fi

cc -O2 -Wall -Wextra $(pkg-config --cflags vips) \
    "$SCRIPT_DIR/generate_fixtures.c" \
    -o "$GENERATOR" \
    $(pkg-config --libs vips)

"$GENERATOR" "$FIXTURE_DIR" "$DISPLAY_P3_ICC" "$ACESCG_ICC" \
    "$FIXTURE_DIR/manifest.json"

# ImageIO rejects the otherwise-valid orientation-tagged TIFF emitted directly
# by libvips on this host. Re-encode this small metadata fixture through
# ImageMagick so every candidate receives the same interoperable TIFF.
magick "$FIXTURE_DIR/paired-sdr-display-p3-u16.tiff" -resize '1600x1000!' \
    -orient RightTop -depth 16 -compress Zip \
    "$FIXTURE_DIR/orientation-6-display-p3-u16.tiff"

shasum -a 256 "$FIXTURE_DIR"/*.tiff > "$FIXTURE_DIR/sha256.txt"
echo "Generated TIFF benchmark fixtures at $FIXTURE_DIR"
