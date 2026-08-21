#!/bin/sh
set -eu

BENCH_ROOT=${1:-/private/tmp/photara-proxy-benchmark}
RESULTS_DIR=${2:-$BENCH_ROOT/results}
ITERATIONS=${3:-3}
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
FIXTURE_DIR="$BENCH_ROOT/fixtures"
BIN_DIR="$BENCH_ROOT/bin"
CONTRACTS="$SCRIPT_DIR/contracts.json"
DISPLAY_P3_ICC="/System/Library/ColorSync/Profiles/Display P3.icc"
SRGB_ICC="/System/Library/ColorSync/Profiles/sRGB Profile.icc"
ACESCG_ICC="/System/Library/ColorSync/Profiles/ACESCG Linear.icc"
RUST_MANIFEST="$SCRIPT_DIR/rust-image/Cargo.toml"
RUST_TARGET=$(cargo metadata --manifest-path "$RUST_MANIFEST" --format-version 1 --no-deps | \
    sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')
RUST_PROXY="$RUST_TARGET/release/photara-rust-image-proxy-benchmark"

if [ ! -f "$FIXTURE_DIR/manifest.json" ]; then
    echo "Missing fixtures. Run generate-fixtures.sh first."
    exit 1
fi
if [ -e "$RESULTS_DIR" ]; then
    echo "Results directory already exists: $RESULTS_DIR"
    exit 1
fi

mkdir -p "$BIN_DIR" "$RESULTS_DIR/logs" "$RESULTS_DIR/outputs"

cc -O2 -Wall -Wextra $(pkg-config --cflags vips) "$SCRIPT_DIR/vips_proxy.c" \
    -o "$BIN_DIR/vips-proxy" $(pkg-config --libs vips)
swiftc -module-cache-path "$BENCH_ROOT/swift-module-cache" -O \
    -framework Foundation -framework CoreGraphics -framework CoreImage -framework ImageIO \
    "$SCRIPT_DIR/apple_proxy.swift" -o "$BIN_DIR/apple-proxy"
cargo build --manifest-path "$RUST_MANIFEST" --release

{
    hostname
    sw_vers
    xcodebuild -version
    xcrun swift --version
    rustc --version
    cargo --version
    vips --version
    magick -version
    sysctl -n hw.memsize hw.ncpu machdep.cpu.brand_string
} > "$RESULTS_DIR/environment.txt" 2>&1
cp "$CONTRACTS" "$RESULTS_DIR/contracts.json"
cp "$FIXTURE_DIR/manifest.json" "$RESULTS_DIR/fixture-manifest.json"
cp "$FIXTURE_DIR/sha256.txt" "$RESULTS_DIR/fixture-sha256.txt"

echo "backend,profile,iteration,real_seconds,user_seconds,sys_seconds,peak_rss_bytes,output_bytes" \
    > "$RESULTS_DIR/measurements.csv"

run_one() {
    backend=$1
    profile=$2
    iteration=$3
    extension=$4
    shift 4
    output="$RESULTS_DIR/outputs/${backend}-${profile}-${iteration}.${extension}"
    timing="$RESULTS_DIR/logs/${backend}-${profile}-${iteration}.time"
    /usr/bin/time -lp "$@" "$output" > "$RESULTS_DIR/logs/${backend}-${profile}-${iteration}.stdout" \
        2> "$timing"
    real=$(awk '$1 == "real" { print $2 }' "$timing")
    user=$(awk '$1 == "user" { print $2 }' "$timing")
    sys=$(awk '$1 == "sys" { print $2 }' "$timing")
    peak=$(awk '/maximum resident set size/ { print $1 }' "$timing")
    bytes=$(stat -f '%z' "$output")
    echo "$backend,$profile,$iteration,$real,$user,$sys,$peak,$bytes" \
        >> "$RESULTS_DIR/measurements.csv"
}

iteration=1
while [ "$iteration" -le "$ITERATIONS" ]; do
    run_one apple-cpu thumbnail-sdr "$iteration" png \
        "$BIN_DIR/apple-proxy" thumbnail-sdr \
        "$FIXTURE_DIR/paired-sdr-display-p3-u16.tiff" "$SRGB_ICC" 512 cpu
    run_one apple-gpu thumbnail-sdr "$iteration" png \
        "$BIN_DIR/apple-proxy" thumbnail-sdr \
        "$FIXTURE_DIR/paired-sdr-display-p3-u16.tiff" "$SRGB_ICC" 512 gpu
    run_one vips thumbnail-sdr "$iteration" png \
        "$BIN_DIR/vips-proxy" thumbnail-sdr \
        "$FIXTURE_DIR/paired-sdr-display-p3-u16.tiff" "$SRGB_ICC" 512
    run_one imagemagick thumbnail-sdr "$iteration" png \
        magick -limit memory 1024MiB -limit map 2048MiB \
        "$FIXTURE_DIR/paired-sdr-display-p3-u16.tiff" -auto-orient \
        -black-point-compensation -intent relative -profile "$SRGB_ICC" \
        -colorspace RGB -filter Lanczos -resize '512x512>' -colorspace sRGB \
        -depth 8 -profile "$SRGB_ICC"
    run_one rust-image thumbnail-sdr "$iteration" png \
        "$RUST_PROXY" thumbnail-sdr "$FIXTURE_DIR/paired-sdr-display-p3-u16.tiff" \
        "$SRGB_ICC" "$CONTRACTS"

    run_one apple-cpu authoring-hdr "$iteration" tiff \
        "$BIN_DIR/apple-proxy" authoring-hdr \
        "$FIXTURE_DIR/paired-hdr-acescg-f32.tiff" "$ACESCG_ICC" 2048 cpu
    run_one apple-gpu authoring-hdr "$iteration" tiff \
        "$BIN_DIR/apple-proxy" authoring-hdr \
        "$FIXTURE_DIR/paired-hdr-acescg-f32.tiff" "$ACESCG_ICC" 2048 gpu
    run_one vips authoring-hdr "$iteration" tiff \
        "$BIN_DIR/vips-proxy" authoring-hdr \
        "$FIXTURE_DIR/paired-hdr-acescg-f32.tiff" "$ACESCG_ICC" 2048
    run_one imagemagick authoring-hdr "$iteration" tiff \
        magick -limit memory 1024MiB -limit map 2048MiB \
        "$FIXTURE_DIR/paired-hdr-acescg-f32.tiff" -auto-orient -set colorspace RGB \
        -filter Lanczos -resize '2048x2048>' -define quantum:format=floating-point \
        -depth 16 -compress Zip
    run_one rust-image authoring-hdr "$iteration" tiff \
        "$RUST_PROXY" authoring-hdr "$FIXTURE_DIR/paired-hdr-acescg-f32.tiff" \
        "$ACESCG_ICC" "$CONTRACTS"

    iteration=$((iteration + 1))
done

for renderer in cpu gpu; do
    "$BIN_DIR/apple-proxy" thumbnail-sdr \
        "$FIXTURE_DIR/orientation-6-display-p3-u16.tiff" \
        "$SRGB_ICC" 512 "$renderer" "$RESULTS_DIR/outputs/apple-$renderer-orientation.png"
done
"$BIN_DIR/vips-proxy" thumbnail-sdr "$FIXTURE_DIR/orientation-6-display-p3-u16.tiff" \
    "$SRGB_ICC" 512 "$RESULTS_DIR/outputs/vips-orientation.png"
magick "$FIXTURE_DIR/orientation-6-display-p3-u16.tiff" -auto-orient \
    -black-point-compensation -intent relative -profile "$SRGB_ICC" \
    -colorspace RGB -filter Lanczos -resize '512x512>' -colorspace sRGB -depth 8 \
    -profile "$SRGB_ICC" "$RESULTS_DIR/outputs/imagemagick-orientation.png"
"$RUST_PROXY" thumbnail-sdr "$FIXTURE_DIR/orientation-6-display-p3-u16.tiff" \
    "$SRGB_ICC" "$CONTRACTS" "$RESULTS_DIR/outputs/rust-image-orientation.png"

echo "backend,sdr_profile,sdr_sample_a,sdr_sample_b,hdr_bits,hdr_profile,hdr_max,hdr_min,orientation_width,orientation_height,status,reason" \
    > "$RESULTS_DIR/correctness.csv"
reference_a=$(vips getpoint "$RESULTS_DIR/outputs/apple-cpu-thumbnail-sdr-1.png" 384 85 | \
    tr '\n' ' ' | xargs)
reference_b=$(vips getpoint "$RESULTS_DIR/outputs/apple-cpu-thumbnail-sdr-1.png" 128 256 | \
    tr '\n' ' ' | xargs)

samples_match() {
    awk -v left="$1" -v right="$2" 'BEGIN {
        split(left, a, " "); split(right, b, " ");
        for (i = 1; i <= 3; i++) if (a[i] - b[i] > 2 || b[i] - a[i] > 2) exit 1;
    }'
}

for backend in apple-cpu apple-gpu vips imagemagick rust-image; do
    sdr="$RESULTS_DIR/outputs/$backend-thumbnail-sdr-1.png"
    hdr="$RESULTS_DIR/outputs/$backend-authoring-hdr-1.tiff"
    orientation="$RESULTS_DIR/outputs/$backend-orientation.png"
    sdr_profile=$(sips -g profile "$sdr" 2>/dev/null | tail -1 | sed 's/^ *profile: //')
    sample_a=$(vips getpoint "$sdr" 384 85 | tr '\n' ' ' | xargs)
    sample_b=$(vips getpoint "$sdr" 128 256 | tr '\n' ' ' | xargs)
    hdr_bits=$(sips -g bitsPerSample "$hdr" 2>/dev/null | tail -1 | sed 's/^ *bitsPerSample: //')
    hdr_profile=$(sips -g profile "$hdr" 2>/dev/null | tail -1 | sed 's/^ *profile: //')
    hdr_max=$(vips max "$hdr")
    hdr_min=$(vips min "$hdr")
    orientation_width=$(vipsheader -f width "$orientation")
    orientation_height=$(vipsheader -f height "$orientation")
    status=pass
    reason=ok
    case "$sdr_profile" in
        *sRGB*) ;;
        *) status=fail; reason=sdr-profile ;;
    esac
    if [ "$status" = pass ]; then
        if ! samples_match "$sample_a" "$reference_a" || \
            ! samples_match "$sample_b" "$reference_b"; then
            status=fail; reason=sdr-color-transform
        fi
    fi
    if [ "$status" = pass ] && [ "$hdr_bits" != 16 ]; then
        status=fail; reason=hdr-depth
    fi
    case "$hdr_profile" in
        *ACES*) ;;
        *) if [ "$status" = pass ]; then status=fail; reason=hdr-profile; fi ;;
    esac
    if [ "$status" = pass ] && ! awk -v maximum="$hdr_max" -v minimum="$hdr_min" \
        'BEGIN { exit !(maximum > 1.0 && minimum < 0.0) }'; then
        status=fail; reason=hdr-range
    fi
    if [ "$status" = pass ]; then
        if [ "$orientation_width" != 320 ] || [ "$orientation_height" != 512 ]; then
            status=fail; reason=orientation
        fi
    fi
    echo "$backend,$sdr_profile,\"$sample_a\",\"$sample_b\",$hdr_bits,$hdr_profile,$hdr_max,$hdr_min,$orientation_width,$orientation_height,$status,$reason" \
        >> "$RESULTS_DIR/correctness.csv"
done

echo "Benchmark results written to $RESULTS_DIR"
