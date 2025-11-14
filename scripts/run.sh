#!/usr/bin/env bash
# this_file: scripts/run.sh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
JOBS_FILE="${JOBS_FILE:-$ROOT_DIR/scripts/jobs_smoke.jsonl}"
HAFORU_BIN="${HAFORU_BIN:-$ROOT_DIR/target/release/haforu}"
PYTHON_BIN="${PYTHON_BIN:-python3}"
CONVERTER_PY="${CONVERTER_PY:-python3}"
MODE="${1:-smoke}"

RUN_LOG_DIR="$ROOT_DIR/target/run-log"
RUN_STAMP="$(date +%Y%m%d-%H%M%S)"
RUN_LOG="$RUN_LOG_DIR/$MODE-$RUN_STAMP.log"
TIMINGS_FILE="$RUN_LOG_DIR/$MODE-$RUN_STAMP.timings"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT
mkdir -p "$RUN_LOG_DIR"
touch "$RUN_LOG" "$TIMINGS_FILE"

log() {
    local line="[$(date +%H:%M:%S)] $*"
    printf '\n%s\n' "$line"
    printf '%s\n' "$line" >> "$RUN_LOG"
}

require_file() {
    if [[ ! -f "$1" ]]; then
        printf 'Missing file: %s\n' "$1" >&2
        exit 1
    fi
}

ensure_cli() {
    if [[ -x "$HAFORU_BIN" ]]; then
        return
    fi
    log "haforu binary not found at $HAFORU_BIN, building release binary"
    (cd "$ROOT_DIR" && cargo build --release >/dev/null)
    HAFORU_BIN="$ROOT_DIR/target/release/haforu"
    export HAFORU_BIN
}

convert_jobs() {
    local mode="$1"
    local dest="$2"
    "$CONVERTER_PY" - "$JOBS_FILE" "$mode" "$dest" <<'PY'
import json, sys
from pathlib import Path

src = Path(sys.argv[1])
mode = sys.argv[2]
dest = Path(sys.argv[3])
jobs = []
for line in src.read_text().splitlines():
    line = line.strip()
    if not line:
        continue
    job = json.loads(line)
    fmt = job.get("rendering", {}).get("format")
    if mode == "metrics" and fmt != "metrics":
        continue
    if mode == "nonmetrics" and fmt == "metrics":
        continue
    jobs.append(job)
dest.write_text(json.dumps({"version": "1.0", "jobs": jobs}))
PY
}

summarize_jobs() {
    local label="$1"
    local file="$2"
    "$CONVERTER_PY" - "$file" "$label" <<'PY'
import json, sys
from pathlib import Path

path = Path(sys.argv[1])
label = sys.argv[2]
print(f"{label} summaries:")
for line in path.read_text().splitlines():
    line = line.strip()
    if not line:
        continue
    job = json.loads(line)
    status = job.get("status")
    job_id = job.get("id")
    metrics = job.get("metrics")
    err = job.get("error")
    if metrics:
        metrics_text = f"density={metrics.get('density'):.3f}, beam={metrics.get('beam'):.3f}"
        print(f"  {job_id}: {status} ({metrics_text})")
    elif err:
        print(f"  {job_id}: {status} -> {err}")
    else:
        fmt = job.get("rendering", {}).get("format")
        print(f"  {job_id}: {status} ({fmt})")
PY
}

summarize_stream() {
    local file="$1"
    "$CONVERTER_PY" - "$file" <<'PY'
import json, sys
from pathlib import Path

path = Path(sys.argv[1])
print("Streaming summaries:")
for line in path.read_text().splitlines():
    line = line.strip()
    if not line:
        continue
    job = json.loads(line)
    job_id = job.get("id")
    status = job.get("status")
    err = job.get("error")
    if err:
        print(f"  {job_id}: {status} -> {err}")
    else:
        print(f"  {job_id}: {status}")
PY
}

run_cli() {
    local label="$1"
    local input="$2"
    local output="$3"
    shift 3
    log "$label"
    local start end
    start=$(date +%s)
    if [[ -n "$input" ]]; then
        "$HAFORU_BIN" "$@" < "$input" > "$output"
    else
        "$HAFORU_BIN" "$@" > "$output"
    fi
    end=$(date +%s)
    printf '%s\t%ss\n' "$label" "$((end - start))" >> "$TIMINGS_FILE"
}

batch_demo() {
    local batch_json="$TMP_DIR/batch.json"
    local output="$TMP_DIR/batch.out"
    convert_jobs all "$batch_json"
    run_cli "haforu batch (jobs_smoke)" "$batch_json" "$output" batch
    summarize_jobs "Batch" "$output" | tee -a "$RUN_LOG"
}

metrics_demo() {
    local metrics_json="$TMP_DIR/metrics.json"
    local output="$TMP_DIR/metrics.out"
    convert_jobs metrics "$metrics_json"
    run_cli "haforu batch (metrics only)" "$metrics_json" "$output" batch
    summarize_jobs "Metrics" "$output" | tee -a "$RUN_LOG"
}

stream_demo() {
    local output="$TMP_DIR/stream.out"
    run_cli "haforu stream (jobs_smoke)" "$JOBS_FILE" "$output" stream
    summarize_stream "$output" | tee -a "$RUN_LOG"
}

python_demo() {
    log "Python StreamingSession demo"
    if ! "$PYTHON_BIN" -c "import haforu" >/dev/null 2>&1; then
        log "haforu Python module not found for $PYTHON_BIN; skip demo (install the wheel first)."
        return
    fi
    local status
    "$PYTHON_BIN" - "$JOBS_FILE" <<'PY' | tee -a "$RUN_LOG"
import json, sys, haforu, pathlib

jobs = [json.loads(line) for line in pathlib.Path(sys.argv[1]).read_text().splitlines() if line.strip()]
first = jobs[0]
print(f"haforu {haforu.__version__}, available={haforu.is_available()}")
session = haforu.StreamingSession(max_fonts=8, max_glyphs=128)
session.warm_up()
result = json.loads(session.render(json.dumps(first)))
print(f"session render -> {result['id']} {result['status']}")
session.close()
PY
    status=${PIPESTATUS[0]}
    if [[ $status -ne 0 ]]; then
        exit $status
    fi
}

smoke_suite() {
    ensure_cli
    batch_demo
    metrics_demo
    stream_demo
}

main() {
    require_file "$JOBS_FILE"
    case "$MODE" in
        smoke)
            smoke_suite
            ;;
        batch)
            ensure_cli
            batch_demo
            ;;
        metrics)
            ensure_cli
            metrics_demo
            ;;
        stream)
            ensure_cli
            stream_demo
            ;;
        python)
            python_demo
            ;;
        all)
            smoke_suite
            python_demo
            ;;
        *)
            printf 'Usage: %s [smoke|batch|metrics|stream|python|all]\n' "$0"
            exit 1
            ;;
    esac
    log "Run artifacts: $RUN_LOG (timings: $TIMINGS_FILE)"
}

main
