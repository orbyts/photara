#!/bin/sh
set -eu

if [ "$#" -lt 3 ] || [ "$#" -gt 4 ]; then
    echo "usage: $0 HELPER HDR_TIFF OUTPUT_DIR [ITERATIONS]" >&2
    exit 2
fi

HELPER=$1
SOURCE=$2
OUTPUT_DIR=$3
ITERATIONS=${4:-3}

if [ -e "$OUTPUT_DIR" ]; then
    echo "output directory already exists: $OUTPUT_DIR" >&2
    exit 1
fi
mkdir -p "$OUTPUT_DIR"
echo "concurrency,iteration,elapsed_seconds,aggregate_peak_rss_bytes" > "$OUTPUT_DIR/results.csv"

now() {
    perl -MTime::HiRes=time -e 'printf "%.6f", time'
}

run_group() {
    concurrency=$1
    iteration=$2
    pids=""
    pid_list=""
    job=1
    started=$(now)
    while [ "$job" -le "$concurrency" ]; do
        output="$OUTPUT_DIR/c${concurrency}-i${iteration}-j${job}.tiff"
        metadata="$OUTPUT_DIR/c${concurrency}-i${iteration}-j${job}.json"
        "$HELPER" authoring-hdr "$SOURCE" "$output" 2048 "$metadata" &
        pid=$!
        pids="$pids $pid"
        if [ -z "$pid_list" ]; then
            pid_list=$pid
        else
            pid_list="$pid_list,$pid"
        fi
        job=$((job + 1))
    done

    peak_kib=0
    running=1
    while [ "$running" -eq 1 ]; do
        current_kib=$(ps -o rss= -p "$pid_list" 2>/dev/null | awk '{ total += $1 } END { print total + 0 }')
        if [ "$current_kib" -gt "$peak_kib" ]; then
            peak_kib=$current_kib
        fi
        running=0
        for pid in $pids; do
            if kill -0 "$pid" 2>/dev/null; then
                running=1
            fi
        done
        if [ "$running" -eq 1 ]; then
            sleep 0.02
        fi
    done
    for pid in $pids; do
        wait "$pid"
    done
    finished=$(now)
    elapsed=$(awk -v start="$started" -v finish="$finished" 'BEGIN { printf "%.3f", finish - start }')
    peak_bytes=$((peak_kib * 1024))
    echo "$concurrency,$iteration,$elapsed,$peak_bytes" >> "$OUTPUT_DIR/results.csv"
}

concurrency=1
while [ "$concurrency" -le 2 ]; do
    iteration=1
    while [ "$iteration" -le "$ITERATIONS" ]; do
        run_group "$concurrency" "$iteration"
        iteration=$((iteration + 1))
    done
    concurrency=$((concurrency + 1))
done

cat "$OUTPUT_DIR/results.csv"
