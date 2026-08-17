# bzip2 Optimization Benchmarks & Verification Suite

A reproducible, high-throughput benchmarking and verification suite for [`libbzip2-rs`](https://github.com/trifectatechfoundation/libbzip2-rs) performance optimizations.

---

## Datasets

1. **NOAA NEXRAD Level-2 Radar Data**:
   - 30 volume archives (~1.5 GB uncompressed, 3.16% compression ratio).
   - High-throughput binary multi-stream archives.
2. **Canonical Silesia Compression Corpus**:
   - All 12 un-truncated canonical files (ASCII literature, x86 binaries, C source tars, XML, medical MRI/X-ray, database records, PDF).
   - ~212 MB uncompressed.

---

## Quickstart

### 1. Clone with Submodules
```bash
git clone --recurse-submodules https://github.com/gheffern/bzip2-benchmarks.git
cd bzip2-benchmarks
```

### 2. Download and Verify Test Datasets
Downloads official, un-truncated Silesia ZIP archive and NOAA AWS S3 radar archives:
```bash
./run_benchmark.sh --fetch-data
```

### 3. Run Benchmarks

#### **A/B Comparison (Baseline `main` vs Optimized Branch)**
```bash
./run_benchmark.sh
```

#### **Stepped Commit Breakdown**
Traces throughput changes across every individual commit on the branch:
```bash
./run_benchmark.sh --stepped
```

---

## Key Results Summary

| Dataset | `main` (v0.2.5) | `feature/perf-optimizations` | Throughput Gain |
| :--- | :--- | :--- | :--- |
| **NEXRAD Radar Decompression** | 284.28 MB/s | **391.74 MB/s** | **+37.8% 🚀** |
| **Silesia Corpus Decompression** | 53.50 MB/s | **56.02 MB/s** | **+4.7% 🚀** |
| **Overall Combined Decompression** | 185.23 MB/s | **224.77 MB/s** | **+21.4% 🚀** |
| **Overall Compression** | 68.90 MB/s | **70.09 MB/s** | **+1.7%** |

Full markdown reports and per-file breakdowns are generated in `benchmarks/output/benchmark_report.md`.

---

## Requirements

- **Rust & Cargo**: Any stable Rust toolchain (via [rustup.rs](https://rustup.rs))
- **Python 3**: Standard library only (no pip dependencies required)
- **Standard C Compiler**: `gcc`, `clang`, or `cc` (for standard library linking)
