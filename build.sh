#!/usr/bin/env bash
# this_file: build.sh

set -euo pipefail

#
# haforu build script
# - Builds Rust crate (debug/release)
# - Optionally syncs versions from git tag vX.Y.Z
# - Builds Python wheels/sdist via maturin (bindings in bindings/python)
#

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PY_BIND_DIR="$ROOT_DIR/bindings/python"
DIST_DIR="$ROOT_DIR/dist"

usage() {
  cat <<USAGE
Usage: ./build.sh [--release] [--sync-version-from-tag] [--version X.Y.Z] [--skip-python]

Options:
  --release               Build Rust in release mode
  --sync-version-from-tag Sync versions from current git tag (expects vX.Y.Z)
  --version X.Y.Z         Explicitly set version for Rust and Python packages
  --skip-python           Skip Python wheel/sdist build
  -h, --help              Show this help

Environment:
  MATURIN_PYPI_TOKEN      Used only by publish.sh; not required for build
  CRATES_IO_TOKEN         Used only by publish.sh; not required for build
USAGE
}

need() { command -v "$1" >/dev/null 2>&1 || { echo "Missing required command: $1" >&2; exit 1; }; }

set_version() {
  local ver="$1"
  echo "[build] Setting version to $ver"

  # Update Rust (haforu) version
  sed -i.bak -E "s/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"/version = \"$ver\"/" "$ROOT_DIR/Cargo.toml"
  rm -f "$ROOT_DIR/Cargo.toml.bak"

  # Update Python binding Cargo
  sed -i.bak -E "s/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"/version = \"$ver\"/" "$PY_BIND_DIR/Cargo.toml"
  rm -f "$PY_BIND_DIR/Cargo.toml.bak"

  # Update pyproject version (PEP 621 requires explicit version)
  sed -i.bak -E "s/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"/version = \"$ver\"/" "$PY_BIND_DIR/pyproject.toml"
  rm -f "$PY_BIND_DIR/pyproject.toml.bak"
}

maybe_sync_from_tag() {
  local tag
  if tag=$(git describe --tags --exact-match 2>/dev/null); then
    if [[ "$tag" =~ ^v([0-9]+\.[0-9]+\.[0-9]+)$ ]]; then
      local ver="${BASH_REMATCH[1]}"
      set_version "$ver"
    else
      echo "[build] Current tag '$tag' doesn't match vX.Y.Z; skipping version sync"
    fi
  else
    echo "[build] Not on a tag; skipping version sync"
  fi
}

main() {
  local release=0 sync_tag=0 explicit_ver="" skip_python=0
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --release) release=1; shift ;;
      --sync-version-from-tag) sync_tag=1; shift ;;
      --version) explicit_ver="$2"; shift 2 ;;
      --skip-python) skip_python=1; shift ;;
      -h|--help) usage; exit 0 ;;
      *) echo "Unknown arg: $1"; usage; exit 1 ;;
    esac
  done

  need cargo
  mkdir -p "$DIST_DIR"

  # Version sync if requested
  if [[ -n "$explicit_ver" ]]; then
    set_version "$explicit_ver"
  elif [[ $sync_tag -eq 1 ]]; then
    maybe_sync_from_tag
  fi

  echo "[build] Running cargo check"
  cargo check

  echo "[build] Running cargo fmt + clippy"
  cargo fmt --all
  # Clippy warnings are printed but not failing local builds by default
  cargo clippy --all-targets --all-features || true

  echo "[build] Running cargo test"
  cargo test --all

  echo "[build] Building Rust crate"
  if [[ $release -eq 1 ]]; then
    cargo build --release
  else
    cargo build
  fi

  if [[ $skip_python -eq 1 ]]; then
    echo "[build] Skipping Python build"
    exit 0
  fi

  if [[ ! -d "$PY_BIND_DIR" ]]; then
    echo "[build] Python bindings not found at $PY_BIND_DIR; skipping"
    exit 0
  fi

  if ! command -v maturin >/dev/null 2>&1; then
    echo "[build] maturin not found; install with: pip install maturin" >&2
    exit 1
  fi

  echo "[build] Building Python wheels (maturin)"
  maturin build -m "$PY_BIND_DIR/Cargo.toml" --release -o "$DIST_DIR"
  echo "[build] Building Python sdist"
  maturin sdist -m "$PY_BIND_DIR/Cargo.toml" -o "$DIST_DIR"

  echo "[build] Done. Artifacts in $DIST_DIR"
}

main "$@"
