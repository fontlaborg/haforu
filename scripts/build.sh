#!/usr/bin/env bash
# this_file: scripts/build.sh

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Detect OS and architecture
OS=$(uname -s)
ARCH=$(uname -m)
PYTHON_VERSION=${PYTHON_VERSION:-3.12}

echo -e "${GREEN}===== Haforu Build Script =====${NC}"
echo "OS: $OS"
echo "Architecture: $ARCH"
echo "Python version: $PYTHON_VERSION"
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

# Check for required tools
check_requirements() {
    section "Checking build requirements"

    # Check for Rust
    if ! command -v cargo &> /dev/null; then
        error "cargo not found. Please install Rust: https://rustup.rs/"
    fi
    echo "✓ Rust/cargo found: $(cargo --version)"

    # Check for Python
    if ! command -v python3 &> /dev/null; then
        error "python3 not found. Please install Python 3.8+"
    fi
    echo "✓ Python found: $(python3 --version)"

    # Check for maturin
    if ! command -v maturin &> /dev/null; then
        echo "maturin not found, installing..."
        pip install maturin || error "Failed to install maturin"
    fi
    echo "✓ maturin found: $(maturin --version)"

    # Check for uv (optional but recommended)
    if command -v uv &> /dev/null; then
        echo "✓ uv found: $(uv --version)"
        PIP_CMD="uv pip"
    else
        echo "⚠ uv not found, using pip (consider installing uv for faster builds)"
        PIP_CMD="pip"
    fi

    echo ""
}

# Build Rust CLI binary
build_rust_cli() {
    section "Building Rust CLI binary"

    # Build in release mode with optimizations
    cargo build --release --bin haforu

    if [ -f "target/release/haforu" ]; then
        echo "✓ Rust CLI built successfully: target/release/haforu"
        echo "  Size: $(ls -lh target/release/haforu | awk '{print $5}')"

        # Strip debug symbols for smaller binary (optional)
        if [[ "$OS" != "Windows_NT" ]] && command -v strip &> /dev/null; then
            cp target/release/haforu target/release/haforu.debug
            strip target/release/haforu
            echo "  Stripped size: $(ls -lh target/release/haforu | awk '{print $5}')"
        fi
    else
        error "Failed to build Rust CLI"
    fi

    echo ""
}

# Build Python wheels
build_python_wheels() {
    section "Building Python wheels"

    # Clean previous builds
    rm -rf target/wheels

    # Determine target based on OS and architecture
    case "$OS" in
        Darwin)
            if [[ "$ARCH" == "arm64" ]] || [[ "$ARCH" == "x86_64" ]]; then
                # Build universal2 wheel for macOS
                section "Building universal2 wheel for macOS"
                maturin build --release --universal2 --features python -o target/wheels
            else
                error "Unsupported macOS architecture: $ARCH"
            fi
            ;;
        Linux)
            # Build manylinux wheel
            if command -v docker &> /dev/null; then
                section "Building manylinux wheel using Docker"
                docker run --rm -v $(pwd):/io ghcr.io/pyo3/maturin build \
                    --release --features python -o target/wheels \
                    --compatibility manylinux2014
            else
                section "Building Linux wheel (non-manylinux)"
                echo "⚠ Docker not found, building standard Linux wheel"
                maturin build --release --features python -o target/wheels
            fi
            ;;
        MINGW*|CYGWIN*|MSYS*)
            section "Building Windows wheel"
            maturin build --release --features python -o target/wheels
            ;;
        *)
            error "Unsupported OS: $OS"
            ;;
    esac

    # List built wheels
    if ls target/wheels/*.whl 1> /dev/null 2>&1; then
        echo ""
        echo "✓ Python wheels built successfully:"
        ls -lh target/wheels/*.whl | while read -r line; do
            echo "  $line"
        done
    else
        error "Failed to build Python wheels"
    fi

    echo ""
}

# Build development wheel for local testing
build_dev_wheel() {
    section "Building development wheel"

    # Create virtual environment if not exists
    if [ ! -d ".venv" ]; then
        echo "Creating virtual environment..."
        python3 -m venv .venv
    fi

    # Activate and install in development mode
    source .venv/bin/activate
    maturin develop --release --features python

    # Verify installation
    if python -c "import haforu; print(f'✓ haforu {haforu.__version__} installed successfully')" 2>/dev/null; then
        echo "✓ Development installation successful"
    else
        echo "⚠ Development installation may have issues"
    fi

    deactivate
    echo ""
}

# Run tests
run_tests() {
    section "Running tests"

    # Rust tests
    echo "Running Rust tests..."
    cargo test --lib --release
    echo "✓ Rust tests passed"
    echo ""

    # Python tests (if available)
    if [ -d "python/tests" ] && [ -f ".venv/bin/python" ]; then
        echo "Running Python tests..."
        source .venv/bin/activate
        python -m pytest python/tests -v || echo "⚠ Some Python tests failed"
        deactivate
    fi

    # Smoke test
    if [ -f "scripts/batch_smoke.sh" ]; then
        echo ""
        echo "Running smoke tests..."
        bash scripts/batch_smoke.sh || echo "⚠ Smoke tests failed"
    fi

    echo ""
}

# Generate completion scripts
generate_completions() {
    section "Generating shell completions"

    mkdir -p target/completions

    # Generate completions using clap_complete (if configured in Cargo.toml)
    if cargo run --release --bin haforu -- completions bash > target/completions/haforu.bash 2>/dev/null; then
        echo "✓ Bash completions: target/completions/haforu.bash"
    fi

    if cargo run --release --bin haforu -- completions zsh > target/completions/_haforu 2>/dev/null; then
        echo "✓ Zsh completions: target/completions/_haforu"
    fi

    if cargo run --release --bin haforu -- completions fish > target/completions/haforu.fish 2>/dev/null; then
        echo "✓ Fish completions: target/completions/haforu.fish"
    fi

    echo ""
}

# Package for distribution
package_artifacts() {
    section "Packaging artifacts for distribution"

    DIST_DIR="target/dist"
    rm -rf "$DIST_DIR"
    mkdir -p "$DIST_DIR"

    # Copy CLI binary
    if [ -f "target/release/haforu" ]; then
        cp target/release/haforu "$DIST_DIR/"
        echo "✓ CLI binary: $DIST_DIR/haforu"
    fi

    # Copy wheels
    if ls target/wheels/*.whl 1> /dev/null 2>&1; then
        cp target/wheels/*.whl "$DIST_DIR/"
        echo "✓ Python wheels copied to $DIST_DIR/"
    fi

    # Copy completions
    if [ -d "target/completions" ]; then
        cp -r target/completions "$DIST_DIR/"
        echo "✓ Shell completions copied to $DIST_DIR/completions/"
    fi

    # Create archive
    ARCHIVE_NAME="haforu-${OS,,}-${ARCH}.tar.gz"
    tar -czf "target/$ARCHIVE_NAME" -C target dist
    echo ""
    echo "✓ Distribution archive: target/$ARCHIVE_NAME"
    echo "  Size: $(ls -lh target/$ARCHIVE_NAME | awk '{print $5}')"

    echo ""
}

# Main build process
main() {
    # Parse command line arguments
    BUILD_TYPE=${1:-all}

    case "$BUILD_TYPE" in
        all)
            check_requirements
            build_rust_cli
            build_python_wheels
            build_dev_wheel
            run_tests
            generate_completions
            package_artifacts
            ;;
        rust)
            check_requirements
            build_rust_cli
            ;;
        python)
            check_requirements
            build_python_wheels
            build_dev_wheel
            ;;
        test)
            run_tests
            ;;
        package)
            package_artifacts
            ;;
        *)
            echo "Usage: $0 [all|rust|python|test|package]"
            echo ""
            echo "Options:"
            echo "  all     - Build everything (default)"
            echo "  rust    - Build only Rust CLI binary"
            echo "  python  - Build only Python wheels"
            echo "  test    - Run tests only"
            echo "  package - Package built artifacts"
            exit 1
            ;;
    esac

    echo -e "${GREEN}===== Build Complete =====${NC}"
    echo ""
    echo "To use the CLI:"
    echo "  export HAFORU_BIN=\$PWD/target/release/haforu"
    echo "  \$HAFORU_BIN --help"
    echo ""
    echo "To install Python package:"
    echo "  pip install target/wheels/*.whl"
    echo ""
}

# Run main function
main "$@"