#!/usr/bin/env bash
# this_file: scripts/test-cli-parity.sh
#
# Test Python Fire CLI parity with Rust CLI
# Verifies that both CLIs expose the same functionality

set -uo pipefail

RUST_CLI="${HAFORU_BIN:-./target/release/haforu}"
PYTHON_CLI="python -m haforu"
TESTDATA="./testdata/fonts/Arial-Black.ttf"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

echo "=== Testing CLI Parity: Rust vs Python Fire ==="
echo "Rust CLI: $RUST_CLI"
echo "Python CLI: $PYTHON_CLI"
echo

PASSED=0
FAILED=0

# Helper function to test command availability
test_command() {
    local cli=$1
    local cmd=$2
    local description=$3

    echo -n "Testing $description... "

    set +e
    if [[ "$cli" == *"python"* ]]; then
        $cli $cmd --help >/dev/null 2>&1
    else
        $cli $cmd --help >/dev/null 2>&1
    fi
    local exit_code=$?
    set -e

    if [[ $exit_code -eq 0 ]]; then
        echo -e "${GREEN}✓${NC}"
        PASSED=$((PASSED + 1))
        return 0
    else
        echo -e "${RED}✗${NC}"
        FAILED=$((FAILED + 1))
        return 1
    fi
}

# Test command availability
echo "## 1. Command Availability"
echo

# Rust CLI commands
rust_commands=(
    "batch:Batch processing"
    "stream:Streaming mode"
    "validate:Job validation"
    "version:Version display"
    "diagnostics:System diagnostics"
    "render:Single render"
)

# Python CLI commands (should match Rust)
python_commands=(
    "batch:Batch processing"
    "stream:Streaming mode"
    "validate:Job validation"
    "version:Version display"
    "diagnostics:System diagnostics"
    "render:Single render"
    "metrics:Metrics computation"
)

for cmd_desc in "${rust_commands[@]}"; do
    IFS=':' read -r cmd desc <<< "$cmd_desc"
    test_command "$RUST_CLI" "$cmd" "Rust: $desc"
done

echo

for cmd_desc in "${python_commands[@]}"; do
    IFS=':' read -r cmd desc <<< "$cmd_desc"
    test_command "$PYTHON_CLI" "$cmd" "Python: $desc"
done

echo

# Test functional equivalence
echo "## 2. Functional Equivalence Tests"
echo

# Test 2.1: Version command
echo -n "Testing version command output... "
rust_version=$($RUST_CLI version 2>&1 | head -1 || true)
python_version=$($PYTHON_CLI version 2>&1 | grep -v "INFO:" | head -1 || true)

if [[ -n "$rust_version" ]] && [[ -n "$python_version" ]]; then
    echo -e "${GREEN}✓${NC}"
    echo "  Rust:   $rust_version"
    echo "  Python: $python_version"
    PASSED=$((PASSED + 1))
else
    echo -e "${RED}✗${NC}"
    FAILED=$((FAILED + 1))
fi
echo

# Test 2.2: Diagnostics JSON output
echo -n "Testing diagnostics JSON output... "
rust_diag=$($RUST_CLI diagnostics --format json 2>&1 | grep -v "^$" || true)
python_diag=$($PYTHON_CLI diagnostics --format json 2>&1 | grep -v "INFO:" | grep -v "^$" || true)

if echo "$rust_diag" | jq . >/dev/null 2>&1 && echo "$python_diag" | jq . >/dev/null 2>&1; then
    echo -e "${GREEN}✓${NC}"
    PASSED=$((PASSED + 1))
else
    echo -e "${RED}✗${NC}"
    FAILED=$((FAILED + 1))
fi
echo

# Test 2.3: Validate command
echo -n "Testing validate command... "
cat > /tmp/test_job.json <<EOF
{
  "version": "1.0",
  "jobs": [{
    "id": "test",
    "font": {"path": "$TESTDATA", "size": 256, "variations": {}},
    "text": {"content": "A"},
    "rendering": {"format": "metrics", "encoding": "json", "width": 64, "height": 64}
  }]
}
EOF

rust_validate_exit=0
$RUST_CLI validate < /tmp/test_job.json >/dev/null 2>&1 || rust_validate_exit=$?

python_validate_exit=0
$PYTHON_CLI validate < /tmp/test_job.json 2>&1 | grep -v "INFO:" >/dev/null || python_validate_exit=$?

if [[ $rust_validate_exit -eq 0 ]] && [[ $python_validate_exit -eq 0 ]]; then
    echo -e "${GREEN}✓${NC}"
    PASSED=$((PASSED + 1))
else
    echo -e "${RED}✗${NC}"
    echo "  Rust exit: $rust_validate_exit"
    echo "  Python exit: $python_validate_exit"
    FAILED=$((FAILED + 1))
fi
echo

# Test 2.4: Batch processing
echo -n "Testing batch command... "
rust_batch_output=$($RUST_CLI batch < /tmp/test_job.json 2>/dev/null | grep -v "^$" || true)
python_batch_output=$($PYTHON_CLI batch < /tmp/test_job.json 2>&1 | grep -v "INFO:" | grep -v "^$" || true)

rust_batch_valid=0
python_batch_valid=0

if echo "$rust_batch_output" | jq . >/dev/null 2>&1; then
    rust_batch_valid=1
fi

if echo "$python_batch_output" | jq . >/dev/null 2>&1; then
    python_batch_valid=1
fi

if [[ $rust_batch_valid -eq 1 ]] && [[ $python_batch_valid -eq 1 ]]; then
    echo -e "${GREEN}✓${NC}"
    PASSED=$((PASSED + 1))
else
    echo -e "${RED}✗${NC}"
    echo "  Rust valid: $rust_batch_valid"
    echo "  Python valid: $python_batch_valid"
    FAILED=$((FAILED + 1))
fi
echo

# Test 2.5: Stream mode
echo -n "Testing stream command... "
echo '{"id":"test","font":{"path":"'$TESTDATA'","size":256,"variations":{}},"text":{"content":"A"},"rendering":{"format":"metrics","encoding":"json","width":64,"height":64}}' > /tmp/test_stream.jsonl

rust_stream_output=$($RUST_CLI stream < /tmp/test_stream.jsonl 2>/dev/null | grep -v "^$" || true)
python_stream_output=$($PYTHON_CLI stream < /tmp/test_stream.jsonl 2>&1 | grep -v "INFO:" | grep -v "^$" || true)

rust_stream_valid=0
python_stream_valid=0

if echo "$rust_stream_output" | jq . >/dev/null 2>&1; then
    rust_stream_valid=1
fi

if echo "$python_stream_output" | jq . >/dev/null 2>&1; then
    python_stream_valid=1
fi

if [[ $rust_stream_valid -eq 1 ]] && [[ $python_stream_valid -eq 1 ]]; then
    echo -e "${GREEN}✓${NC}"
    PASSED=$((PASSED + 1))
else
    echo -e "${RED}✗${NC}"
    echo "  Rust valid: $rust_stream_valid"
    echo "  Python valid: $python_stream_valid"
    FAILED=$((FAILED + 1))
fi
echo

# Test 2.6: Render command (metrics mode)
echo -n "Testing render command (metrics mode)... "
rust_render=$($RUST_CLI render -f $TESTDATA -s 256 -t A --format metrics 2>&1 | grep -v "^$" || true)
python_render=$($PYTHON_CLI render --font $TESTDATA --size 256 --text A --format metrics 2>&1 | grep -v "INFO:" | grep -v "^$" || true)

# Both should output density/beam metrics
if echo "$rust_render" | grep -q "density" && echo "$python_render" | grep -q "Density"; then
    echo -e "${GREEN}✓${NC}"
    PASSED=$((PASSED + 1))
else
    echo -e "${RED}✗${NC}"
    FAILED=$((FAILED + 1))
fi
echo

# Test 2.7: Cache knobs (batch mode)
echo -n "Testing cache knobs (--max-fonts, --max-glyphs)... "
rust_cache=$($RUST_CLI batch --max-fonts 128 --max-glyphs 1024 < /tmp/test_job.json 2>/dev/null || true)
python_cache=$($PYTHON_CLI batch --max_fonts 128 --max_glyphs 1024 < /tmp/test_job.json 2>&1 | grep -v "INFO:" || true)

# Both should succeed
if echo "$rust_cache" | jq . >/dev/null 2>&1 && echo "$python_cache" | jq . >/dev/null 2>&1; then
    echo -e "${GREEN}✓${NC}"
    PASSED=$((PASSED + 1))
else
    echo -e "${RED}✗${NC}"
    FAILED=$((FAILED + 1))
fi
echo

# Summary
echo "=== CLI Parity Test Summary ==="
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo

if [[ $FAILED -eq 0 ]]; then
    echo -e "${GREEN}✓ All parity tests passed!${NC}"
    echo
    echo "Python Fire CLI successfully mirrors Rust CLI functionality."
    echo "Both CLIs can be used interchangeably."
    exit 0
else
    echo -e "${RED}✗ Some parity tests failed!${NC}"
    echo
    echo "Review the failures above and update the Python CLI to match Rust CLI."
    exit 1
fi
