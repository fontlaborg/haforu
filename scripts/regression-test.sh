#!/usr/bin/env bash
# this_file: scripts/regression-test.sh
#
# Performance regression tests for haforu CLI
# Runs key performance benchmarks and compares against baseline thresholds
# Exit code 0 = pass, 1 = regression detected

set -euo pipefail

HAFORU_BIN="${HAFORU_BIN:-./target/release/haforu}"
TESTDATA="./testdata/fonts/Arial-Black.ttf"
BASELINE_FILE="${BASELINE_FILE:-.baseline-perf.json}"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo "=== Performance Regression Tests ==="
echo "Binary: $HAFORU_BIN"
echo "Baseline: $BASELINE_FILE"
echo

# Check if binary exists
if [ ! -f "$HAFORU_BIN" ]; then
    echo -e "${RED}ERROR: Binary not found at $HAFORU_BIN${NC}"
    echo "Run: cargo build --release"
    exit 1
fi

# Check for hyperfine
if ! command -v hyperfine &> /dev/null; then
    echo -e "${YELLOW}WARNING: hyperfine not found${NC}"
    echo "Install with: brew install hyperfine"
    echo "Skipping regression tests"
    exit 0
fi

# Performance thresholds (in milliseconds)
# Based on profiling results from 2025-11-14
# Note: bash -c wrapper adds ~5-7ms overhead
THRESHOLD_HELP=10          # --help should be <10ms
THRESHOLD_BATCH_1=15       # 1 job batch should be <15ms (includes bash -c overhead)
THRESHOLD_STREAM_100=15    # 100 line stream should be <15ms (includes bash -c overhead)
THRESHOLD_METRICS=10       # Single metrics render should be <10ms

FAILED=0

# Helper function to run benchmark and check threshold
check_perf() {
    local name=$1
    local threshold=$2
    shift 2
    local cmd="$@"

    echo "Testing: $name"
    echo "  Command: $cmd"
    echo "  Threshold: ${threshold}ms"

    # Run hyperfine and extract mean time
    result=$(hyperfine --warmup 3 --runs 10 --export-json /tmp/bench.json "$cmd" 2>&1)
    mean_ms=$(jq '.results[0].mean * 1000' /tmp/bench.json)

    # Compare (bash doesn't have floating point, so use bc)
    passed=$(echo "$mean_ms < $threshold" | bc -l)

    if [ "$passed" -eq 1 ]; then
        echo -e "  ${GREEN}✓ PASS${NC} (${mean_ms}ms < ${threshold}ms)"
    else
        echo -e "  ${RED}✗ FAIL${NC} (${mean_ms}ms >= ${threshold}ms)"
        FAILED=1
    fi
    echo
}

# Test 1: Argument parsing (--help)
check_perf "Argument parsing (--help)" $THRESHOLD_HELP \
    "$HAFORU_BIN --help"

# Test 2: Batch processing (1 job)
echo '{"version":"1.0","jobs":[{"id":"test","font":{"path":"'$TESTDATA'","size":256,"variations":{}},"text":{"content":"A","script":"Latn"},"rendering":{"format":"metrics","encoding":"json","width":64,"height":64}}]}' > /tmp/batch_1.json
check_perf "Batch processing (1 job)" $THRESHOLD_BATCH_1 \
    "bash -c '$HAFORU_BIN batch < /tmp/batch_1.json'"

# Test 3: JSONL streaming (100 lines)
for i in $(seq 1 100); do
    echo '{"id":"job-'$i'","font":{"path":"'$TESTDATA'","size":256,"variations":{}},"text":{"content":"A","script":"Latn"},"rendering":{"format":"metrics","encoding":"json","width":64,"height":64}}'
done > /tmp/stream_100.jsonl
check_perf "JSONL streaming (100 lines)" $THRESHOLD_STREAM_100 \
    "bash -c '$HAFORU_BIN stream < /tmp/stream_100.jsonl'"

# Test 4: Single metrics render
check_perf "Metrics rendering (single glyph)" $THRESHOLD_METRICS \
    "$HAFORU_BIN render -f $TESTDATA -s 256 -t A --format metrics"

# Summary
echo "=== Regression Test Summary ==="
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ Performance regression detected!${NC}"
    echo
    echo "Some benchmarks exceeded their thresholds."
    echo "This may indicate a performance regression."
    echo
    echo "Next steps:"
    echo "1. Review recent changes for performance impact"
    echo "2. Run 'scripts/profile-cli.sh' for detailed profiling"
    echo "3. Use flamegraph/perf for deep analysis if needed"
    exit 1
fi
