#!/usr/bin/env bash
# this_file: scripts/profile-cli.sh
#
# Profile haforu CLI hot paths:
# - Argument parsing overhead
# - JSON batch parsing
# - JSONL streaming throughput
# - Job validation
# - End-to-end rendering

set -euo pipefail

HAFORU_BIN="${HAFORU_BIN:-./target/release/haforu}"
TESTDATA="./testdata/fonts/Arial-Black.ttf"

echo "=== Profiling haforu CLI hot paths ==="
echo "Binary: $HAFORU_BIN"
echo "Test font: $TESTDATA"
echo

# Check for hyperfine
if ! command -v hyperfine &> /dev/null; then
    echo "Note: hyperfine not found, using basic timing instead"
    echo "Install with: brew install hyperfine"
    USE_HYPERFINE=0
else
    USE_HYPERFINE=1
fi

# 1. Argument parsing overhead
echo "## 1. Argument Parsing Overhead"
echo "Measuring help/version commands (startup + arg parsing only):"
echo

if [ $USE_HYPERFINE -eq 1 ]; then
    hyperfine --warmup 3 \
        "$HAFORU_BIN --help" \
        "$HAFORU_BIN --version" \
        "$HAFORU_BIN diagnostics --format json"
else
    for cmd in "--help" "--version" "diagnostics --format json"; do
        echo "Timing: $HAFORU_BIN $cmd"
        time $HAFORU_BIN $cmd > /dev/null
    done
fi

echo

# 2. JSON batch parsing
echo "## 2. JSON Batch Parsing (hot path: serde_json parse)"
echo "Measuring batch mode with varying input sizes:"
echo

# Create test jobs with different sizes
create_batch_jobs() {
    local count=$1
    echo '{"version":"1.0","jobs":['
    for i in $(seq 1 $count); do
        cat <<EOF
{"id":"job-$i","font":{"path":"$TESTDATA","size":256,"variations":{}},"text":{"content":"A","script":"Latn"},"rendering":{"format":"metrics","encoding":"json","width":64,"height":64}}$([ $i -lt $count ] && echo ",")
EOF
    done
    echo ']}'
}

for count in 1 10 100; do
    echo "Batch with $count jobs:"
    create_batch_jobs $count > /tmp/batch_$count.json

    if [ $USE_HYPERFINE -eq 1 ]; then
        hyperfine --warmup 2 --runs 10 \
            "$HAFORU_BIN batch < /tmp/batch_$count.json"
    else
        echo "Timing: $HAFORU_BIN batch < /tmp/batch_$count.json"
        time $HAFORU_BIN batch < /tmp/batch_$count.json > /dev/null
    fi
    echo
done

# 3. JSONL streaming parsing
echo "## 3. JSONL Streaming (hot path: line-by-line JSON parse + dispatch)"
echo "Measuring stream mode with varying line counts:"
echo

# Create JSONL test data
create_jsonl_jobs() {
    local count=$1
    for i in $(seq 1 $count); do
        echo '{"id":"job-'$i'","font":{"path":"'$TESTDATA'","size":256,"variations":{}},"text":{"content":"A","script":"Latn"},"rendering":{"format":"metrics","encoding":"json","width":64,"height":64}}'
    done
}

for count in 10 100 1000; do
    echo "JSONL stream with $count lines:"
    create_jsonl_jobs $count > /tmp/stream_$count.jsonl

    if [ $USE_HYPERFINE -eq 1 ]; then
        hyperfine --warmup 2 --runs 10 \
            "$HAFORU_BIN stream < /tmp/stream_$count.jsonl"
    else
        echo "Timing: $HAFORU_BIN stream < /tmp/stream_$count.jsonl"
        time $HAFORU_BIN stream < /tmp/stream_$count.jsonl > /dev/null
    fi
    echo
done

# 4. End-to-end rendering (with vs without image encoding)
echo "## 4. End-to-End Rendering Performance"
echo "Comparing metrics-only vs full PGM rendering:"
echo

# Metrics only (fast path)
echo "Metrics-only format:"
if [ $USE_HYPERFINE -eq 1 ]; then
    hyperfine --warmup 5 --runs 50 \
        "$HAFORU_BIN render -f $TESTDATA -s 256 -t A --format metrics"
else
    echo "Timing: $HAFORU_BIN render -f $TESTDATA -s 256 -t A --format metrics"
    time for i in {1..50}; do $HAFORU_BIN render -f $TESTDATA -s 256 -t A --format metrics > /dev/null; done
fi

echo

# PGM rendering (full path)
echo "PGM rendering format:"
if [ $USE_HYPERFINE -eq 1 ]; then
    hyperfine --warmup 5 --runs 50 \
        "$HAFORU_BIN render -f $TESTDATA -s 256 -t A --format pgm -o /tmp/test.pgm"
else
    echo "Timing: $HAFORU_BIN render -f $TESTDATA -s 256 -t A --format pgm -o /tmp/test.pgm"
    time for i in {1..50}; do $HAFORU_BIN render -f $TESTDATA -s 256 -t A --format pgm -o /tmp/test.pgm; done
fi

echo

# Summary
echo "=== Profile Complete ==="
echo
echo "Hot paths profiled:"
echo "1. Argument parsing - baseline CLI startup overhead"
echo "2. JSON batch parsing - serde_json deserialize throughput"
echo "3. JSONL streaming - line-by-line parse + job dispatch"
echo "4. End-to-end rendering - metrics vs PGM output comparison"
echo
echo "Next steps:"
echo "- Review results above for any outliers"
echo "- Use flamegraph/perf for deeper profiling if needed"
echo "- Add regression tests for critical paths"
