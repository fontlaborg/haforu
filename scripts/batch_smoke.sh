#!/usr/bin/env bash
# this_file: scripts/batch_smoke.sh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

JOBS_FILE="${JOBS_FILE:-$ROOT_DIR/testdata/jobs_smoke.jsonl}"
CACHE_SIZE="${CACHE_SIZE:-256}"
JOB_THREADS="${JOB_THREADS:-4}"

if [[ ! -r "$JOBS_FILE" ]]; then
  echo "Smoke jobs file not found: $JOBS_FILE" >&2
  exit 1
fi

if [[ ! -x "${HAFORU_BIN:-}" ]]; then
  cargo build --release >/dev/null 2>&1
fi

BIN_PATH="${HAFORU_BIN:-$ROOT_DIR/target/release/haforu}"
if [[ ! -x "$BIN_PATH" ]]; then
  echo "haforu binary not found at $BIN_PATH" >&2
  exit 1
fi

exec "$BIN_PATH" batch --cache-size "$CACHE_SIZE" --jobs "$JOB_THREADS" < "$JOBS_FILE"
