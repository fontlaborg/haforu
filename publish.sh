#!/usr/bin/env bash
# this_file: publish.sh

set -euo pipefail

#
# haforu publish script
# - Publishes Rust crate to crates.io
# - Publishes Python package (bindings/python) to PyPI via maturin
# - Syncs versions from git tag vX.Y.Z or explicit --version
#

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PY_BIND_DIR="$ROOT_DIR/bindings/python"

usage() {
  cat <<USAGE
Usage: ./publish.sh [--dry-run] [--version X.Y.Z] [--sync-version-from-tag] [--no-crates] [--no-pypi]

Environment:
  CRATES_IO_TOKEN   crates.io API token (required unless --no-crates)
  PYPI_TOKEN        PyPI API token (required unless --no-pypi)

Notes:
  - Requires 'maturin' installed for Python publish
  - Ensure git workspace is clean and on a tag when publishing
USAGE
}

need() { command -v "$1" >/dev/null 2>&1 || { echo "Missing required command: $1" >&2; exit 1; }; }

set_version() {
  local ver="$1"
  echo "[publish] Setting version to $ver"
  sed -i.bak -E "s/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"/version = \"$ver\"/" "$ROOT_DIR/Cargo.toml" && rm -f "$ROOT_DIR/Cargo.toml.bak"
  sed -i.bak -E "s/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"/version = \"$ver\"/" "$PY_BIND_DIR/Cargo.toml" && rm -f "$PY_BIND_DIR/Cargo.toml.bak"
  sed -i.bak -E "s/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"/version = \"$ver\"/" "$PY_BIND_DIR/pyproject.toml" && rm -f "$PY_BIND_DIR/pyproject.toml.bak"
}

main() {
  local dry=0 ver="" sync_tag=0 do_crates=1 do_pypi=1
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --dry-run) dry=1; shift ;;
      --version) ver="$2"; shift 2 ;;
      --sync-version-from-tag) sync_tag=1; shift ;;
      --no-crates) do_crates=0; shift ;;
      --no-pypi) do_pypi=0; shift ;;
      -h|--help) usage; exit 0 ;;
      *) echo "Unknown arg: $1"; usage; exit 1 ;;
    esac
  done

  need cargo

  if [[ -n "$ver" ]]; then
    set_version "$ver"
  elif [[ $sync_tag -eq 1 ]]; then
    if tag=$(git describe --tags --exact-match 2>/dev/null); then
      if [[ "$tag" =~ ^v([0-9]+\.[0-9]+\.[0-9]+)$ ]]; then
        set_version "${BASH_REMATCH[1]}"
      else
        echo "[publish] Tag '$tag' not matching vX.Y.Z"
        exit 1
      fi
    else
      echo "[publish] Not on a tag; use --version"
      exit 1
    fi
  fi

  if [[ $do_crates -eq 1 ]]; then
    : "${CRATES_IO_TOKEN:?CRATES_IO_TOKEN env var is required for crates.io publish}"
    echo "[publish] Publishing to crates.io"
    if [[ $dry -eq 1 ]]; then
      echo "[publish] cargo publish --dry-run"
      cargo publish --dry-run
    else
      CARGO_REGISTRIES_CRATES_IO_TOKEN="$CRATES_IO_TOKEN" cargo publish
    fi
  else
    echo "[publish] Skipping crates.io publish"
  fi

  if [[ $do_pypi -eq 1 ]]; then
    need maturin
    : "${PYPI_TOKEN:?PYPI_TOKEN env var is required for PyPI publish}"
    echo "[publish] Publishing Python package to PyPI"
    if [[ $dry -eq 1 ]]; then
      maturin publish -m "$PY_BIND_DIR/Cargo.toml" --username __token__ --password "$PYPI_TOKEN" --skip-existing --dry-run
    else
      maturin publish -m "$PY_BIND_DIR/Cargo.toml" --username __token__ --password "$PYPI_TOKEN" --skip-existing
    fi
  else
    echo "[publish] Skipping PyPI publish"
  fi

  echo "[publish] Done"
}

main "$@"

