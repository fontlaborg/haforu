#!/usr/bin/env bash
# this_file: scripts/run.sh

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
HAFORU_BIN="${HAFORU_BIN:-target/release/haforu}"
PYTHON_BIN="${PYTHON_BIN:-python3}"
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

echo -e "${GREEN}===== Haforu Demo Runner =====${NC}"
echo "Binary: $HAFORU_BIN"
echo "Temp dir: $TEMP_DIR"
echo ""

# Function to print section headers
section() {
    echo -e "${YELLOW}>>> $1${NC}"
}

# Function to handle errors
error() {
    echo -e "${RED}ERROR: $1${NC}" >&2
    exit 1
}

# Check if haforu binary exists
check_binary() {
    if [ ! -f "$HAFORU_BIN" ]; then
        echo "Haforu binary not found at $HAFORU_BIN"
        echo "Building it now..."
        cargo build --release || error "Failed to build haforu"
    fi
    echo "✓ Using haforu binary: $HAFORU_BIN"
    echo ""
}

# Demo 1: Basic batch rendering
demo_batch_basic() {
    section "Demo 1: Basic Batch Rendering"
    echo "Rendering a simple text with default font..."

    cat > "$TEMP_DIR/batch_job.json" <<'EOF'
{
  "version": "1.0",
  "jobs": [
    {
      "id": "demo_hello",
      "font": {
        "path": "testdata/fonts/NotoSans-Regular.ttf",
        "size": 72
      },
      "text": {
        "content": "Hello"
      },
      "rendering": {
        "format": "pgm",
        "encoding": "base64",
        "width": 400,
        "height": 150
      }
    },
    {
      "id": "demo_world",
      "font": {
        "path": "testdata/fonts/NotoSans-Regular.ttf",
        "size": 72
      },
      "text": {
        "content": "World"
      },
      "rendering": {
        "format": "pgm",
        "encoding": "base64",
        "width": 400,
        "height": 150
      }
    }
  ]
}
EOF

    echo "Input JSON:"
    jq '.' "$TEMP_DIR/batch_job.json" 2>/dev/null || cat "$TEMP_DIR/batch_job.json"
    echo ""

    echo "Running batch job..."
    $HAFORU_BIN batch < "$TEMP_DIR/batch_job.json" > "$TEMP_DIR/batch_output.jsonl"

    echo "Output (first result):"
    head -n1 "$TEMP_DIR/batch_output.jsonl" | jq '.id, .status' 2>/dev/null || \
        head -n1 "$TEMP_DIR/batch_output.jsonl"
    echo ""
}

# Demo 2: Variable font rendering
demo_variable_font() {
    section "Demo 2: Variable Font Rendering"
    echo "Testing weight variations (wght axis)..."

    # Create jobs for different weights
    cat > "$TEMP_DIR/variable_job.json" <<'EOF'
{
  "version": "1.0",
  "jobs": [
    {
      "id": "weight_300",
      "font": {
        "path": "testdata/fonts/IBMPlexSansArabic-Regular.ttf",
        "size": 60,
        "variations": {"wght": 300}
      },
      "text": {"content": "Light"},
      "rendering": {"format": "pgm", "encoding": "base64", "width": 300, "height": 100}
    },
    {
      "id": "weight_500",
      "font": {
        "path": "testdata/fonts/IBMPlexSansArabic-Regular.ttf",
        "size": 60,
        "variations": {"wght": 500}
      },
      "text": {"content": "Medium"},
      "rendering": {"format": "pgm", "encoding": "base64", "width": 300, "height": 100}
    },
    {
      "id": "weight_700",
      "font": {
        "path": "testdata/fonts/IBMPlexSansArabic-Regular.ttf",
        "size": 60,
        "variations": {"wght": 700}
      },
      "text": {"content": "Bold"},
      "rendering": {"format": "pgm", "encoding": "base64", "width": 300, "height": 100}
    }
  ]
}
EOF

    echo "Testing 3 weight variations: 300, 500, 700"
    $HAFORU_BIN batch < "$TEMP_DIR/variable_job.json" > "$TEMP_DIR/variable_output.jsonl"

    echo "Results:"
    while IFS= read -r line; do
        echo "$line" | jq -r '"\(.id): \(.status) - variations: \(.font.variations // {})"' 2>/dev/null || echo "$line"
    done < "$TEMP_DIR/variable_output.jsonl"
    echo ""
}

# Demo 3: Metrics mode
demo_metrics() {
    section "Demo 3: Metrics Mode"
    echo "Computing density and beam measurements without rendering..."

    cat > "$TEMP_DIR/metrics_job.json" <<'EOF'
{
  "version": "1.0",
  "jobs": [
    {
      "id": "metrics_A",
      "font": {"path": "testdata/fonts/NotoSans-Regular.ttf", "size": 100},
      "text": {"content": "A"},
      "rendering": {"format": "metrics", "width": 200, "height": 150}
    },
    {
      "id": "metrics_i",
      "font": {"path": "testdata/fonts/NotoSans-Regular.ttf", "size": 100},
      "text": {"content": "i"},
      "rendering": {"format": "metrics", "width": 200, "height": 150}
    },
    {
      "id": "metrics_W",
      "font": {"path": "testdata/fonts/NotoSans-Regular.ttf", "size": 100},
      "text": {"content": "W"},
      "rendering": {"format": "metrics", "width": 200, "height": 150}
    }
  ]
}
EOF

    echo "Computing metrics for: A, i, W"
    $HAFORU_BIN batch < "$TEMP_DIR/metrics_job.json" > "$TEMP_DIR/metrics_output.jsonl"

    echo "Metrics results:"
    while IFS= read -r line; do
        echo "$line" | jq -r '"\(.id): density=\(.metrics.density), beam=\(.metrics.beam)"' 2>/dev/null || echo "$line"
    done < "$TEMP_DIR/metrics_output.jsonl"
    echo ""
}

# Demo 4: Streaming mode
demo_streaming() {
    section "Demo 4: Streaming Mode"
    echo "Processing jobs one at a time in streaming mode..."

    # Create individual job lines
    cat > "$TEMP_DIR/stream_jobs.jsonl" <<'EOF'
{"id": "stream_1", "font": {"path": "testdata/fonts/NotoSans-Regular.ttf", "size": 48}, "text": {"content": "First"}, "rendering": {"format": "pgm", "encoding": "base64", "width": 250, "height": 100}}
{"id": "stream_2", "font": {"path": "testdata/fonts/NotoSans-Regular.ttf", "size": 48}, "text": {"content": "Second"}, "rendering": {"format": "pgm", "encoding": "base64", "width": 250, "height": 100}}
{"id": "stream_3", "font": {"path": "testdata/fonts/NotoSans-Regular.ttf", "size": 48}, "text": {"content": "Third"}, "rendering": {"format": "pgm", "encoding": "base64", "width": 250, "height": 100}}
EOF

    echo "Streaming 3 jobs..."
    $HAFORU_BIN stream < "$TEMP_DIR/stream_jobs.jsonl" > "$TEMP_DIR/stream_output.jsonl"

    echo "Stream results:"
    while IFS= read -r line; do
        echo "$line" | jq -r '"\(.id): \(.status)"' 2>/dev/null || echo "$line"
    done < "$TEMP_DIR/stream_output.jsonl"
    echo ""
}

# Demo 5: Error handling
demo_error_handling() {
    section "Demo 5: Error Handling"
    echo "Testing graceful error handling..."

    cat > "$TEMP_DIR/error_job.json" <<'EOF'
{
  "version": "1.0",
  "jobs": [
    {
      "id": "valid_job",
      "font": {"path": "testdata/fonts/NotoSans-Regular.ttf", "size": 48},
      "text": {"content": "Valid"},
      "rendering": {"format": "pgm", "encoding": "base64", "width": 200, "height": 100}
    },
    {
      "id": "invalid_font",
      "font": {"path": "/nonexistent/font.ttf", "size": 48},
      "text": {"content": "Error"},
      "rendering": {"format": "pgm", "encoding": "base64", "width": 200, "height": 100}
    },
    {
      "id": "invalid_size",
      "font": {"path": "testdata/fonts/NotoSans-Regular.ttf", "size": -10},
      "text": {"content": "BadSize"},
      "rendering": {"format": "pgm", "encoding": "base64", "width": 200, "height": 100}
    }
  ]
}
EOF

    echo "Processing jobs with intentional errors..."
    $HAFORU_BIN batch < "$TEMP_DIR/error_job.json" > "$TEMP_DIR/error_output.jsonl"

    echo "Error handling results:"
    while IFS= read -r line; do
        status=$(echo "$line" | jq -r '.status' 2>/dev/null)
        if [ "$status" = "error" ]; then
            echo -e "${RED}$(echo "$line" | jq -r '"\(.id): \(.status) - \(.error)"' 2>/dev/null)${NC}"
        else
            echo -e "${GREEN}$(echo "$line" | jq -r '"\(.id): \(.status)"' 2>/dev/null)${NC}"
        fi
    done < "$TEMP_DIR/error_output.jsonl"
    echo ""
}

# Demo 6: Python bindings (if available)
demo_python() {
    section "Demo 6: Python Bindings"

    # Check if Python module is available
    if ! $PYTHON_BIN -c "import haforu" 2>/dev/null; then
        echo "Python haforu module not installed. Skipping Python demo."
        echo "To install: pip install target/wheels/*.whl"
        return
    fi

    echo "Testing Python API..."

    cat > "$TEMP_DIR/demo.py" <<'EOF'
#!/usr/bin/env python3
import haforu
import json

# Check availability
print(f"Haforu version: {haforu.__version__}")
print(f"Is available: {haforu.is_available()}")

# Process a single job
job = {
    "id": "python_test",
    "font": {"path": "testdata/fonts/NotoSans-Regular.ttf", "size": 60},
    "text": {"content": "Python"},
    "rendering": {"format": "pgm", "encoding": "base64", "width": 300, "height": 100}
}

result = haforu.process_jobs({"jobs": [job]})
result_json = json.loads(result[0])
print(f"Result: {result_json['id']} - {result_json['status']}")

# Test streaming session
session = haforu.StreamingSession(max_fonts=10, max_glyphs=100)
session.warm_up()
print(f"Session cache stats: {session.cache_stats()}")

# Render with session
stream_result = session.render(json.dumps(job))
stream_json = json.loads(stream_result)
print(f"Stream result: {stream_json['id']} - {stream_json['status']}")

session.close()
print("Python API test complete!")
EOF

    $PYTHON_BIN "$TEMP_DIR/demo.py"
    echo ""
}

# Performance benchmark
demo_performance() {
    section "Performance Benchmark"
    echo "Testing rendering performance with cache..."

    # Generate many jobs
    cat > "$TEMP_DIR/perf_gen.py" <<'EOF'
import json
jobs = []
for i in range(100):
    jobs.append({
        "id": f"perf_{i:03d}",
        "font": {"path": "testdata/fonts/NotoSans-Regular.ttf", "size": 48},
        "text": {"content": chr(65 + (i % 26))},  # A-Z
        "rendering": {"format": "metrics", "width": 100, "height": 100}
    })
print(json.dumps({"version": "1.0", "jobs": jobs}))
EOF

    $PYTHON_BIN "$TEMP_DIR/perf_gen.py" > "$TEMP_DIR/perf_jobs.json"

    echo "Benchmarking 100 metrics jobs..."
    start_time=$(date +%s%N)
    $HAFORU_BIN batch --max-fonts 10 --max-glyphs 100 < "$TEMP_DIR/perf_jobs.json" > "$TEMP_DIR/perf_output.jsonl"
    end_time=$(date +%s%N)

    # Calculate elapsed time
    elapsed=$((($end_time - $start_time) / 1000000))
    job_count=$(wc -l < "$TEMP_DIR/perf_output.jsonl")

    echo "Results:"
    echo "  Total jobs: $job_count"
    echo "  Total time: ${elapsed}ms"
    if [ "$job_count" -gt 0 ]; then
        avg=$((elapsed / job_count))
        echo "  Average per job: ${avg}ms"
        jobs_per_sec=$((1000 * job_count / elapsed))
        echo "  Throughput: ~${jobs_per_sec} jobs/sec"
    fi
    echo ""
}

# Main demo runner
main() {
    # Check for haforu binary
    check_binary

    # Parse command line arguments
    DEMO=${1:-all}

    case "$DEMO" in
        all)
            demo_batch_basic
            demo_variable_font
            demo_metrics
            demo_streaming
            demo_error_handling
            demo_python
            demo_performance
            ;;
        batch)
            demo_batch_basic
            ;;
        variable)
            demo_variable_font
            ;;
        metrics)
            demo_metrics
            ;;
        stream)
            demo_streaming
            ;;
        error)
            demo_error_handling
            ;;
        python)
            demo_python
            ;;
        perf|performance)
            demo_performance
            ;;
        *)
            echo "Usage: $0 [all|batch|variable|metrics|stream|error|python|perf]"
            echo ""
            echo "Demos:"
            echo "  all        - Run all demos (default)"
            echo "  batch      - Basic batch rendering"
            echo "  variable   - Variable font rendering"
            echo "  metrics    - Metrics computation"
            echo "  stream     - Streaming mode"
            echo "  error      - Error handling"
            echo "  python     - Python bindings"
            echo "  perf       - Performance benchmark"
            exit 1
            ;;
    esac

    echo -e "${GREEN}===== Demo Complete =====${NC}"
}

# Run main function
main "$@"