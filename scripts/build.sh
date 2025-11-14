#!/usr/bin/env bash
# this_file: scripts/build.sh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROFILE="${HAFORU_PROFILE:-release}"
RUN_TESTS=1
RUN_SMOKE=1
SKIP_WHEELS=0
TARGETS="auto"

usage() {
    cat <<'EOF'
Usage: scripts/build.sh [options]

Options:
  --profile <name>     Cargo profile to build/test (default: release)
  --skip-tests         Skip cargo/uvx test suites
  --skip-smoke         Skip scripts/batch_smoke.sh
  --skip-wheels        Skip Python wheel builds
  --targets "<list>"   Wheel targets (auto|universal2|manylinux|windows|all)
  -h, --help           Show this help

Environment:
  ARTIFACT_DIR     Override target/artifacts output root
  HAFORU_PROFILE   Alias for --profile
EOF
}

while (($#)); do
    case "$1" in
        --profile) PROFILE="$2"; shift ;;
        --skip-tests) RUN_TESTS=0 ;;
        --skip-smoke) RUN_SMOKE=0 ;;
        --skip-wheels) SKIP_WHEELS=1 ;;
        --targets) TARGETS="$2"; shift ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            printf 'Unknown option: %s\n\n' "$1" >&2
            usage
            exit 1
            ;;
    esac
    shift
done

cd "$ROOT_DIR"

if [[ -z "${CARGO_BUILD_JOBS:-}" ]]; then
    if command -v getconf >/dev/null 2>&1; then
        CARGO_BUILD_JOBS="$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 1)"
    else
        CARGO_BUILD_JOBS="${NUMBER_OF_PROCESSORS:-1}"
    fi
    [[ "$CARGO_BUILD_JOBS" -ge 1 ]] 2>/dev/null || CARGO_BUILD_JOBS=1
    export CARGO_BUILD_JOBS
fi

if [[ -z "${CARGO_SOURCE_CRATES_IO_REPLACE_WITH:-}" ]]; then
    export CARGO_SOURCE_CRATES_IO_REPLACE_WITH="crates-io"
fi

if [[ -z "${CARGO_SOURCE_VENDORED_SOURCES_DIRECTORY:-}" ]]; then
    export CARGO_SOURCE_VENDORED_SOURCES_DIRECTORY="$ROOT_DIR/target/vendor-empty"
    mkdir -p "$CARGO_SOURCE_VENDORED_SOURCES_DIRECTORY"
fi

ARTIFACT_ROOT="${ARTIFACT_DIR:-$ROOT_DIR/target/artifacts}"
RUN_STAMP="$(date +%Y%m%d-%H%M%S)"
RUN_DIR="$ARTIFACT_ROOT/$RUN_STAMP"
BIN_DIR="$RUN_DIR/bin"
WHEEL_DIR="$RUN_DIR/wheels"
LOG_DIR="$RUN_DIR/logs"
TIMINGS_FILE="$LOG_DIR/timings.txt"
mkdir -p "$BIN_DIR" "$WHEEL_DIR" "$LOG_DIR"
ln -sfn "$RUN_DIR" "$ARTIFACT_ROOT/latest"

log() {
    printf '\n[%s] %s\n' "$(date +%H:%M:%S)" "$*"
}

require_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        printf 'Missing required tool: %s\n' "$1" >&2
        exit 1
    fi
}

run_step() {
    local label="$1"
    shift
    log "$label"
    local start end
    start=$(date +%s)
    "$@"
    end=$(date +%s)
    printf '%s\t%ss\n' "$label" "$((end - start))" >> "$TIMINGS_FILE"
}

detect_platform() {
    local uname_out
    uname_out="$(uname -s)"
    case "$uname_out" in
        Darwin) PLATFORM="mac" ;;
        Linux) PLATFORM="linux" ;;
        MINGW*|MSYS*|CYGWIN*) PLATFORM="windows" ;;
        *) PLATFORM="unknown" ;;
    esac
    ARCH="$(uname -m)"
    BIN_NAME="haforu"
    [[ "$PLATFORM" == "windows" ]] && BIN_NAME="haforu.exe"
    BUILD_DIR_NAME=$([[ "$PROFILE" == "release" ]] && echo "release" || echo "$PROFILE")
    SOURCE_BIN="$ROOT_DIR/target/$BUILD_DIR_NAME/$BIN_NAME"
    HAFORU_BIN="$BIN_DIR/$BIN_NAME"
    export HAFORU_BIN
}

resolve_targets() {
    TARGET_SET=()
    if [[ "$TARGETS" == "auto" ]]; then
        case "$PLATFORM" in
            mac) TARGET_SET=("universal2") ;;
            linux) TARGET_SET=("manylinux") ;;
            windows) TARGET_SET=("windows") ;;
        esac
    elif [[ "$TARGETS" == "all" ]]; then
        TARGET_SET=("universal2" "manylinux" "windows")
    else
        read -r -a TARGET_SET <<<"$TARGETS"
    fi
}

build_cli() {
    local args=(build --locked --bin haforu)
    if [[ "$PROFILE" == "release" ]]; then
        args+=(--release)
    else
        args+=(--profile "$PROFILE")
    fi
    run_step "cargo ${args[*]}" cargo "${args[@]}"
    if [[ ! -f "$SOURCE_BIN" ]]; then
        printf 'Unable to locate built binary at %s\n' "$SOURCE_BIN" >&2
        exit 1
    fi
    cp "$SOURCE_BIN" "$HAFORU_BIN"
    chmod +x "$HAFORU_BIN"
    log "CLI ready at $HAFORU_BIN"
}

build_wheels() {
    [[ "$SKIP_WHEELS" -eq 1 ]] && return
    if [[ "${#TARGET_SET[@]}" -eq 0 ]]; then
        log "No wheel targets selected for $PLATFORM; skipping wheel build."
        return
    fi
    mkdir -p "$WHEEL_DIR"
    local rel_out="/io${WHEEL_DIR#$ROOT_DIR}"
    for target in "${TARGET_SET[@]}"; do
        case "$target" in
            universal2)
                run_step "maturin universal2" \
                    uvx maturin build --release --target universal2-apple-darwin \
                    --features python --out "$WHEEL_DIR"
                ;;
            manylinux)
                if command -v docker >/dev/null 2>&1; then
                    run_step "maturin manylinux (docker)" \
                        docker run --rm -v "$ROOT_DIR":/io ghcr.io/pyo3/maturin \
                        build --release --features python --compatibility manylinux_2_28 \
                        --out "$rel_out"
                else
                    run_step "maturin manylinux host" \
                        uvx maturin build --release --features python \
                        --compatibility manylinux_2_28 --out "$WHEEL_DIR"
                fi
                ;;
            windows)
                run_step "maturin windows" \
                    uvx maturin build --release --features python --out "$WHEEL_DIR"
                ;;
            *)
                printf 'Unknown wheel target: %s\n' "$target" >&2
                exit 1
                ;;
        esac
    done
    log "Wheels stored in $WHEEL_DIR"
}

run_rust_tests() {
    [[ "$RUN_TESTS" -eq 0 ]] && return
    local args=(test --locked --workspace)
    if [[ "$PROFILE" == "release" ]]; then
        args+=(--release)
    else
        args+=(--profile "$PROFILE")
    fi
    run_step "cargo ${args[*]}" cargo "${args[@]}"
}

run_python_tests() {
    [[ "$RUN_TESTS" -eq 0 ]] && return
    run_step "uvx hatch test" uvx hatch test
}

run_smoke() {
    [[ "$RUN_SMOKE" -eq 0 ]] && return
    export HAFORU_BIN
    run_step "scripts/batch_smoke.sh" bash "$ROOT_DIR/scripts/batch_smoke.sh"
}

write_summary() {
    local summary="$LOG_DIR/summary.txt"
    {
        printf "timestamp=%s\n" "$RUN_STAMP"
        printf "profile=%s\n" "$PROFILE"
        printf "platform=%s\n" "$PLATFORM"
        printf "arch=%s\n" "$ARCH"
        printf "cli=%s\n" "$HAFORU_BIN"
        printf "wheels=%s\n" "$WHEEL_DIR"
        printf "tests=%s\n" "$([[ "$RUN_TESTS" -eq 1 ]] && echo on || echo off)"
        printf "smoke=%s\n" "$([[ "$RUN_SMOKE" -eq 1 ]] && echo on || echo off)"
    } >"$summary"
    log "Summary written to $summary"
    log "Timings recorded in $TIMINGS_FILE"
}

main() {
    require_cmd cargo
    require_cmd uvx
    detect_platform
    resolve_targets
    log "Haforu build start (profile=$PROFILE, platform=$PLATFORM, arch=$ARCH)"
    build_cli
    build_wheels
    run_rust_tests
    run_python_tests
    run_smoke
    write_summary
}

main "$@"
