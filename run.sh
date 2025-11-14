#!/bin/bash
# this_file: run.sh

set -e

MODE="${1:-smoke}"

echo "Running haforu in '$MODE' mode..."
exec ./scripts/run.sh "$MODE"
