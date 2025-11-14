#!/usr/bin/env bash
# this_file: scripts/batch_smoke.sh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

JOBS_FILE="${JOBS_FILE:-$ROOT_DIR/scripts/jobs_smoke.jsonl}"
CACHE_SIZE="${CACHE_SIZE:-256}"
GLYPH_CACHE_SIZE="${GLYPH_CACHE_SIZE:-512}"
JOB_THREADS="${JOB_THREADS:-4}"

if [[ ! -r "$JOBS_FILE" ]]; then
  echo "Smoke jobs file not found: $JOBS_FILE" >&2
  exit 1
fi

BIN_PATH="${HAFORU_BIN:-$ROOT_DIR/target/release/haforu}"
if [[ ! -x "$BIN_PATH" ]]; then
  cargo build --release >/dev/null 2>&1
fi

if [[ ! -x "$BIN_PATH" ]]; then
  echo "haforu binary not found at $BIN_PATH" >&2
  exit 1
fi

OUTPUT_FILE="$(mktemp)"
trap 'rm -f "$OUTPUT_FILE"' EXIT

if ! "$BIN_PATH" batch \
  --max-fonts "$CACHE_SIZE" \
  --max-glyphs "$GLYPH_CACHE_SIZE" \
  --jobs "$JOB_THREADS" \
  < "$JOBS_FILE" | tee "$OUTPUT_FILE"; then
  echo "haforu batch run failed" >&2
  exit 1
fi

python3 <<'PY' "$OUTPUT_FILE"
import json
import pathlib
import sys

lines = [ln.strip() for ln in pathlib.Path(sys.argv[1]).read_text().splitlines() if ln.strip()]
if not lines:
    raise SystemExit("Smoke run produced no JSONL output")

expected = {
    "smoke-1": "success",
    "smoke-2": "success",
    "smoke-metrics": "success",
    "smoke-invalid": "error",
}

seen = {}
for raw in lines:
    try:
        payload = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise SystemExit(f"Invalid JSONL line from haforu: {exc}: {raw!r}") from exc
    job_id = payload.get("id")
    status = payload.get("status")
    if job_id is None or status is None:
        raise SystemExit(f"Missing id/status in payload: {payload}")
    seen[job_id] = payload
    if job_id not in expected:
        raise SystemExit(f"Unexpected job id {job_id}; expected {sorted(expected)}")
    if status != expected[job_id]:
        raise SystemExit(
            f"Job {job_id} returned status {status}, expected {expected[job_id]}"
        )

invalid_error = seen["smoke-invalid"].get("error", "")
if "Font size" not in invalid_error:
    raise SystemExit(
        f"smoke-invalid should report font-size validation error, got: {invalid_error!r}"
    )

metrics_payload = seen["smoke-metrics"]
if "metrics" not in metrics_payload:
    raise SystemExit(f"smoke-metrics missing metrics payload: {metrics_payload}")
if "rendering" in metrics_payload:
    raise SystemExit("metrics format should not emit rendering field")
metrics = metrics_payload["metrics"]
for key in ("density", "beam"):
    value = metrics.get(key)
    if value is None:
        raise SystemExit(f"metrics payload missing {key}: {metrics}")
    if not isinstance(value, (int, float)) or not (0 <= value <= 1):
        raise SystemExit(f"{key} should be 0<=x<=1, got {value!r}")

print("✓ batch_smoke JSON contract verified")
PY
