#!/bin/bash
set -e

echo "Building Rust CLI and Python package..."
cargo build --release
uv pip install --system --upgrade -e .

echo "Running tests..."
cargo test
uvx hatch test

echo "Building wheels..."
rm -rf target/wheels
uvx maturin build --release

echo "Done."
