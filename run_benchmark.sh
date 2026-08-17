#!/usr/bin/env bash
set -eo pipefail

# ==============================================================================
# Generic Reproducible Benchmark Suite for libbzip2-rs Optimizations
# ==============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BENCH_DIR="${SCRIPT_DIR}/benchmarks"
DATA_DIR="${BENCH_DIR}/data"

show_help() {
    cat <<EOF
Usage: ./run_benchmark.sh [OPTIONS]

One-shot reproducible benchmark suite for libbzip2-rs optimizations.
Measures both NEXRAD Radar and Silesia Compression Corpus datasets.

Options:
  --fetch-data         Download and prepare the full test datasets (canonical Silesia ZIP & NOAA NEXRAD)
  --stepped            Step through every commit on the branch individually vs main
  --iterations <N>     Number of iterations per dataset/file (default: 20)
  -h, --help           Show this help message and exit

Requirements:
  - Rust & Cargo (https://rustup.rs)
  - Python 3 (standard library only)
  - Standard C compiler (gcc / clang / cc)

Examples:
  ./run_benchmark.sh --fetch-data      # Download & verify full datasets
  ./run_benchmark.sh                   # Run standard A/B benchmark (main vs current branch)
  ./run_benchmark.sh --stepped         # Run commit-by-commit stepped benchmark
EOF
}

FETCH_DATA=false
STEPPED=false
ITERATIONS=20

while [[ $# -gt 0 ]]; do
    case "$1" in
        --fetch-data)
            FETCH_DATA=true
            shift
            ;;
        --stepped)
            STEPPED=true
            shift
            ;;
        --iterations)
            ITERATIONS="$2"
            shift 2
            ;;
        -h|--help)
            show_help
            exit 0
            ;;
        *)
            echo "Error: Unknown argument '$1'" >&2
            show_help
            exit 1
            ;;
    esac
done

# 1. Handle dataset fetching
if [ "$FETCH_DATA" = true ]; then
    echo "Fetching and preparing complete test datasets..."
    python3 "${BENCH_DIR}/fetch_data.py"
    exit 0
fi

# 2. Check if data exists
if [ ! -d "${DATA_DIR}/reference" ] || [ ! -d "${DATA_DIR}/compressed" ] || [ "$(ls -A "${DATA_DIR}/reference" 2>/dev/null | wc -l)" -lt 40 ]; then
    echo "========================================================================="
    echo "ERROR: Test datasets are missing or incomplete in ${DATA_DIR}."
    echo "Please run './run_benchmark.sh --fetch-data' to download the test suite."
    echo "========================================================================="
    exit 1
fi

# 3. Verify standard host build tools
if ! command -v cargo &>/dev/null; then
    echo "Error: 'cargo' was not found in PATH. Please install Rust from https://rustup.rs" >&2
    exit 1
fi

# 4. Invoke benchmark runner
RUNNER_ARGS=("--iterations" "${ITERATIONS}")
if [ "$STEPPED" = true ]; then
    RUNNER_ARGS+=("--stepped")
fi

python3 "${BENCH_DIR}/runner.py" "${RUNNER_ARGS[@]}"
