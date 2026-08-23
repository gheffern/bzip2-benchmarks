# bzip2 Optimization Benchmarks & Verification Suite

A reproducible, high-throughput benchmarking and verification suite for [`libbzip2-rs`](https://github.com/trifectatechfoundation/libbzip2-rs) performance optimizations.

---

## Datasets Overview

1. **NOAA NEXRAD Level-2 Radar Data**:
   - 30 multi-stream volume archives (~1.5 GB uncompressed per pass, ~45 MB compressed, 3.16% ratio).
   - Real-world binary radar sweeps containing ~54 concatenated bzip2 streams per volume with 4-byte big-endian headers.
2. **Canonical Silesia Compression Corpus**:
   - All 12 un-truncated canonical files (~212 MB uncompressed, ~52 MB compressed).
   - Covers diverse real-world data types: ASCII literature, x86 binaries, C source tars, XML, medical MRI/X-ray imaging, database records, and PDF.

---

## Benchmark Results (Median Throughput & RSD Variance)

### Environment & Benchmark Configuration

| Parameter | Value |
| :--- | :--- |
| **CPU Model** | AMD Ryzen 7 7840HS w/ Radeon 780M Graphics |
| **OS / Kernel** | Linux 6.12.0-211.47.1.el10_2.x86_64 (x86_64) |
| **Rust Toolchain** | `rustc 1.97.1 (8bab26f4f 2026-07-14)` |
| **Warmup Passes** | **3 un-timed warmup passes** (cache & TLB pre-faulted) |
| **Benchmark Harness** | **Zero-allocation streaming** (0 heap allocs in timed loops) |
| **Baseline Ref** | `origin/main` (`v0.2.5` / `f47b114`) |
| **Target Ref** | `feature/perf-optimizations-v2` (`6cc9da9`) |

---

### 1. Overall Aggregate Throughput (Median)

| Dataset | Operation | Baseline (`main` v0.2.5) | Target (`feature/perf-optimizations-v2`) | Throughput Delta | Speedup |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **NEXRAD Radar** | Decompression | 322.15 MB/s (±0.3%) | **472.90 MB/s (±0.3%)** | +150.75 MB/s | **+46.8%** |
| **NEXRAD Radar** | Compression | 110.38 MB/s (±2.5%) | **113.44 MB/s (±1.3%)** | +3.06 MB/s | **+2.8%** |
| **Silesia Corpus** | Decompression | 53.27 MB/s (±1.0%) | **53.01 MB/s (±1.5%)** | -0.26 MB/s | **-0.5%** |
| **Silesia Corpus** | Compression | 20.70 MB/s (±0.7%) | **20.30 MB/s (±1.2%)** | -0.40 MB/s | **-1.9%** |

---

### 2. NOAA NEXRAD Radar Dataset Details

| Characteristic | Measurement | Notes |
| :--- | :--- | :--- |
| **Volume Archives Tested** | 30 volume archives | Full Level-2 radar sweeps (`nexrad1.bz2` – `nexrad30.bz2`) |
| **Uncompressed Volume per File** | ~47.6 MB – 52.4 MB | ~54 bzip2 streams per volume file |
| **Total Uncompressed Size** | **1,497.70 MB** (~1.5 GB) | Continuous in-memory zero-allocation streaming |
| **Total Compressed Size** | **45.07 MB** | 3.16% compression ratio |
| **Decompression Speed** | **472.90 MB/s** (vs 322.15 MB/s) | **+46.8% speedup (+150.75 MB/s)** (Slice-by-4 CRC32 & small RLE fast-path) |
| **Compression Speed** | **113.44 MB/s** (vs 110.38 MB/s) | **+2.8% speedup** |

---

### 3. Silesia Corpus Decompression Breakdown (Median)

| File Name | Data Type Category | Size | Baseline (`main` v0.2.5) | Target (`v2`) | Speedup |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **`mozilla`** | Tar / Executables | 51.22 MB | 45.24 MB/s (±2.2%) | **54.48 MB/s (±4.4%)** | **+20.4%** |
| **`x-ray`** | Medical (X-Ray Image) | 8.47 MB | 36.05 MB/s (±10.0%) | **42.80 MB/s (±3.0%)** | **+18.7%** |
| **`mr`** | Medical (MRI Image) | 9.97 MB | 65.63 MB/s (±2.0%) | **68.93 MB/s (±5.0%)** | **+5.0%** |
| **`samba`** | Tar / C Source Code | 21.61 MB | 69.78 MB/s (±6.5%) | **73.08 MB/s (±1.6%)** | **+4.7%** |
| **`ooffice`** | x86 Executable / DLL | 6.15 MB | 41.02 MB/s (±7.5%) | **42.33 MB/s (±3.7%)** | **+3.2%** |
| **`reymont`** | PDF Document | 6.63 MB | 54.26 MB/s (±3.6%) | **55.74 MB/s (±3.2%)** | **+2.7%** |
| **`dickens`** | Text (ASCII Literature) | 10.19 MB | 45.74 MB/s (±4.9%) | **46.92 MB/s (±2.6%)** | **+2.6%** |
| **`nci`** | Chemistry Database / Text | 33.55 MB | 81.70 MB/s (±2.2%) | **80.86 MB/s (±16.4%)** | **-1.0%** |
| **`xml`** | Structured XML Markup | 5.35 MB | 83.67 MB/s (±7.4%) | **78.21 MB/s (±8.5%)** | **-6.5%** |

---

## Repository Layout

```text
.
├── run_benchmark.sh              # Top-level one-shot benchmark script
├── README.md                     # Documentation and benchmark reports
├── benchmarks/
│   ├── Cargo.toml                # Benchmark harness dependencies
│   ├── fetch_data.py             # Dataset downloader with runtime SHA-256 validation
│   ├── runner.py                 # Automated A/B and stepped benchmark orchestrator
│   └── src/bin/bench_suite.rs    # High-throughput in-memory benchmark runner
└── libbzip2-rs/                  # Submodule pointing to libbzip2-rs
```

---

## Quickstart

### 1. Clone with Submodules
```bash
git clone --recurse-submodules https://github.com/gheffern/bzip2-benchmarks.git
cd bzip2-benchmarks
```

### 2. Download and Cryptographically Verify Datasets
Downloads the official canonical Silesia ZIP and NOAA AWS S3 radar sweeps, validating 100% of SHA-256 checksums at runtime:
```bash
./run_benchmark.sh --fetch-data
```

### 3. Run Benchmarks

#### **A/B Comparison (Baseline `main` vs Target Branch)**
```bash
./run_benchmark.sh
```

#### **Stepped Commit Breakdown**
Traces throughput changes across every individual commit on the branch:
```bash
./run_benchmark.sh --stepped
```

#### **Custom Iterations**
```bash
./run_benchmark.sh --iterations 10
```

---

## Requirements

- **Rust & Cargo**: Any stable Rust toolchain (via [rustup.rs](https://rustup.rs))
- **Python 3**: Standard library only (no pip dependencies required)
- **Standard C Compiler**: `gcc`, `clang`, or `cc` (for standard library linking)

---

## License

Dual-licensed under either:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
