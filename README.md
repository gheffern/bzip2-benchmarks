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

## Benchmark Results (20 Iterations Steady-State)

### Environment & Benchmark Configuration

| Parameter | Value |
| :--- | :--- |
| **CPU Model** | AMD Ryzen 7 7840HS w/ Radeon 780M Graphics |
| **OS / Kernel** | Linux 6.12.0-211.47.1.el10_2.x86_64 (x86_64) |
| **Rust Toolchain** | `rustc 1.97.1 (8bab26f4f 2026-07-14)` |
| **Iterations per File** | **20 iterations** (~34.2 GB processed) |
| **Baseline Ref** | `origin/main` (`v0.2.5` / `f47b114`) |
| **Target Ref** | `feature/perf-optimizations-v2` (`9852792`) |

---

### 1. Overall Aggregate Throughput

| Dataset | Operation | Baseline (`main` v0.2.5) | Target (`feature/perf-optimizations-v2`) | Throughput Delta | Speedup |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **NEXRAD Radar** | Decompression | 283.19 MB/s | **390.23 MB/s** | +107.04 MB/s | **+37.8%** |
| **NEXRAD Radar** | Compression | 112.54 MB/s | **114.10 MB/s** | +1.56 MB/s | **+1.4%** |
| **Silesia Corpus** | Decompression | 54.73 MB/s | **54.27 MB/s** | -0.46 MB/s | **-0.8%** |
| **Silesia Corpus** | Compression | 20.76 MB/s | **20.78 MB/s** | +0.02 MB/s | **+0.1%** |

---

### 2. NOAA NEXRAD Radar Dataset Details

| Characteristic | Measurement | Notes |
| :--- | :--- | :--- |
| **Volume Archives Tested** | 30 volume archives | Full Level-2 radar sweeps (`nexrad1.bz2` – `nexrad30.bz2`) |
| **Uncompressed Volume per File** | ~47.6 MB – 52.4 MB | ~54 bzip2 streams per volume file |
| **Total Uncompressed Size** | **1,497.70 MB** (~1.5 GB) | 29,953.95 MB across 20 benchmark iterations |
| **Total Compressed Size** | **45.07 MB** | 3.16% compression ratio |
| **Decompression Speed** | **390.23 MB/s** (vs 283.19 MB/s) | **+37.8% speedup (+107 MB/s)** (Slice-by-4 parallel CRC32) |
| **Compression Speed** | **114.10 MB/s** (vs 112.54 MB/s) | **+1.4% speedup** |

---

### 3. Silesia Corpus Decompression Breakdown

| File Name | Data Type Category | Size | Baseline (`main` v0.2.5) | Target (`v2`) | Speedup |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **`webster`** | English Dictionary Text | 41.46 MB | 52.42 MB/s | **54.44 MB/s** | **+3.9%** |
| **`sao`** | Astronomical Star Catalog | 7.25 MB | 33.70 MB/s | **34.50 MB/s** | **+2.4%** |
| **`ooffice`** | x86 Executable / DLL | 6.15 MB | 40.73 MB/s | **41.39 MB/s** | **+1.6%** |
| **`samba`** | Tar / C Source Code | 21.61 MB | 67.90 MB/s | **68.97 MB/s** | **+1.6%** |
| **`osdb`** | DB Binary Records | 10.09 MB | 48.04 MB/s | **48.77 MB/s** | **+1.5%** |
| **`reymont`** | PDF Document | 6.63 MB | 54.63 MB/s | **55.17 MB/s** | **+1.0%** |
| **`x-ray`** | Medical (X-Ray Image) | 8.47 MB | 41.67 MB/s | **41.93 MB/s** | **+0.6%** |
| **`dickens`** | Text (ASCII Literature) | 10.19 MB | 45.51 MB/s | **45.74 MB/s** | **+0.5%** |
| **`xml`** | Structured XML Markup | 5.35 MB | 87.15 MB/s | **86.73 MB/s** | **-0.5%** |
| **`nci`** | Chemistry Database / Text | 33.55 MB | 76.14 MB/s | **75.57 MB/s** | **-0.7%** |
| **`mozilla`** | Tar / Executables | 51.22 MB | 52.14 MB/s | **49.81 MB/s** | **-4.5%** |
| **`mr`** | Medical (MRI Image) | 9.97 MB | 64.84 MB/s | **53.69 MB/s** | **-17.2%** |

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
