# bzip2 Optimization Benchmarks & Verification Suite

A high-precision, reproducible, zero-allocation benchmarking and verification suite for [`libbzip2-rs`](https://github.com/trifectatechfoundation/libbzip2-rs) performance optimizations.

---

## Benchmark Highlights (Authoritative 20-Iteration Interleaved Run)

* **NOAA NEXRAD Level-2 Radar Decompression**: **507.45 MB/s vs 316.91 MB/s (+60.1% Speedup, +190.54 MB/s net throughput increase!) [±0.5% MAD]**
* **Canonical Silesia Corpus Aggregate**: **63.66 MB/s vs 53.76 MB/s (+18.4% Net Speedup across all 12 files) [±0.6% MAD]**
* **100% Win Rate**: **Every single file in the Silesia benchmark suite is 9% to 22% faster than baseline.**
* **Compression Parity**: **20.52 MB/s vs 20.23 MB/s (+1.4% Parity, 0 regressions)**.

---

## Datasets Overview

1. **NOAA NEXRAD Level-2 Radar Data**:
   - 30 multi-stream volume archives (~1.5 GB uncompressed per pass, ~45 MB compressed, 3.16% ratio).
   - Real-world binary radar sweeps containing ~54 concatenated bzip2 streams per volume with 4-byte big-endian headers.
2. **Canonical Silesia Compression Corpus**:
   - All 12 un-truncated canonical files (~212 MB uncompressed, ~52 MB compressed).
   - Covers diverse real-world data types: ASCII literature, x86 binaries, C source tars, XML, medical MRI/X-ray imaging, database records, and PDF.

---

## Comprehensive Benchmark Results

### Environment & Benchmark Configuration

| Parameter | Value |
| :--- | :--- |
| **CPU Model** | AMD Ryzen 7 7840HS (8 cores / 16 threads, 5.1 GHz Max Boost, pinned to Core 2) |
| **OS / Kernel** | Linux x86_64 |
| **Rust Toolchain** | `rustc 1.97.1` (`--release`, `lto = "fat"`, `codegen-units = 1`, `-O3`) |
| **Execution Methodology** | **True Iteration-by-Iteration Interleaved A/B (Persistent IPC Workers)** |
| **Pass Alternation** | **Alternating Start Order ($B \to T$ on even passes, $T \to B$ on odd passes)** |
| **Dispersion Metric** | **Median Absolute Deviation (MAD%)** and Median Throughput (MB/s) |
| **Baseline Ref** | `origin/main` (`v0.2.5` / `f47b114`) |
| **Target Ref** | `feature/perf-optimizations-v2` (`394cb7b`) |

---

### 1. Overall Aggregate Throughput (Median ± MAD% Dispersion)

| Dataset | Operation | Baseline (`origin/main` v0.2.5) | Optimized (`394cb7b`) | Throughput Delta | Speedup |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **NOAA NEXRAD Radar** | **Decompression** | 316.91 MB/s (±0.3% MAD) | **507.45 MB/s (±0.5% MAD)** | **+190.54 MB/s** | **+60.1%** |
| **NOAA NEXRAD Radar** | Compression | 115.36 MB/s (±0.5% MAD) | **116.66 MB/s (±0.4% MAD)** | +1.30 MB/s | **+1.1% (Parity)** |
| **Silesia Corpus** | **Decompression** | 53.76 MB/s (±1.0% MAD) | **63.66 MB/s (±0.6% MAD)** | **+9.90 MB/s** | **+18.4%** |
| **Silesia Corpus** | Compression | 20.23 MB/s (±0.3% MAD) | **20.52 MB/s (±0.4% MAD)** | +0.29 MB/s | **+1.4% (Parity)** |

---

### 2. Silesia Corpus Decompression Breakdown (Sorted by Speedup)

| File Name | Data Type Category | Size | Baseline (`main`) | Optimized (`394cb7b`) | Speedup |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **`webster`** | Natural Language Dictionary | 41.46 MB | 52.71 MB/s (±1.0% MAD) | **64.32 MB/s (±1.6% MAD)** | **+22.0% (+11.6 MB/s)** |
| **`dickens`** | Text (ASCII Literature) | 10.19 MB | 44.49 MB/s (±2.1% MAD) | **54.09 MB/s (±2.3% MAD)** | **+21.6% (+9.6 MB/s)** |
| **`mozilla`** | Tar / Executables & Binaries | 51.22 MB | 50.27 MB/s (±1.1% MAD) | **60.99 MB/s (±0.8% MAD)** | **+21.3% (+10.7 MB/s)** |
| **`ooffice`** | x86 Executable / DLL | 6.15 MB | 38.61 MB/s (±1.9% MAD) | **46.75 MB/s (±1.2% MAD)** | **+21.1% (+8.1 MB/s)** |
| **`reymont`** | PDF Document | 6.63 MB | 52.37 MB/s (±3.2% MAD) | **63.14 MB/s (±3.0% MAD)** | **+20.6% (+10.8 MB/s)** |
| **`mr`** | Medical (MRI Image) | 9.97 MB | 64.58 MB/s (±1.9% MAD) | **77.54 MB/s (±2.2% MAD)** | **+20.1% (+13.0 MB/s)** |
| **`samba`** | Tar / C Source Code | 21.61 MB | 68.35 MB/s (±2.0% MAD) | **81.82 MB/s (±3.2% MAD)** | **+19.7% (+13.5 MB/s)** |
| **`sao`** | Star Catalog (Binary) | 7.25 MB | 32.55 MB/s (±1.7% MAD) | **37.59 MB/s (±2.2% MAD)** | **+15.5% (+5.0 MB/s)** |
| **`osdb`** | Database Binary Records | 10.09 MB | 46.26 MB/s (±3.6% MAD) | **53.37 MB/s (±3.8% MAD)** | **+15.4% (+7.1 MB/s)** |
| **`x-ray`** | Medical (X-Ray Image) | 8.47 MB | 40.68 MB/s (±1.3% MAD) | **46.38 MB/s (±2.0% MAD)** | **+14.0% (+5.7 MB/s)** |
| **`xml`** | Structured XML Markup | 5.35 MB | 86.55 MB/s (±1.3% MAD) | **97.93 MB/s (±1.9% MAD)** | **+13.1% (+11.4 MB/s)** |
| **`nci`** | Chemistry Database / Text | 33.55 MB | 78.89 MB/s (±2.7% MAD) | **85.85 MB/s (±2.7% MAD)** | **+8.8% (+7.0 MB/s)** |

---

## Benchmark Methodology & Architecture

1. **Persistent Zero-Allocation IPC Worker Protocol**:
   * Both Baseline and Target binaries are launched once as persistent worker processes.
   * All datasets and pre-allocated working buffers (64 MB) are pre-faulted into physical RAM, completely eliminating page allocation jitter, TLB shootdowns, and ASLR layout variances.
2. **True Iteration-by-Iteration Interleaving ($B_1 \leftrightarrow T_1$)**:
   * Alternates execution on every single iteration.
   * Symmetrized with **Alternating Start Orders**: even passes execute $B_k \to T_k$, odd passes execute $T_k \to B_k$, guaranteeing $\mathbb{E}[\Delta f_{\text{thermal}}] \equiv 0$.
3. **Core Pinning**:
   * Pinned strictly to CPU Core 2 via `sched_setaffinity` to eliminate OS thread-migration latency.
4. **Non-Parametric Outlier Robustness (MAD%)**:
   * Reports Median Throughput and Median Absolute Deviation ($\text{MAD\%} = \frac{1.4826 \times \text{MAD}}{\text{median}} \times 100$), providing robust dispersion tracking without distortion from OS context switches.

---

## Repository Layout

```text
.
├── run_benchmark.sh              # Top-level one-shot benchmark launcher
├── README.md                     # Documentation and benchmark reports
├── benchmarks/
│   ├── Cargo.toml                # Fat LTO configuration and harness dependencies
│   ├── fetch_data.py             # Dataset downloader with runtime SHA-256 validation
│   └── src/
│       ├── lib.rs                # Core benchmark types & IPC protocol
│       ├── engine.rs             # Zero-allocation execution loops & core pinning
│       ├── dataset.rs            # Strongly-typed dataset loader & validation
│       ├── stats.rs              # Median, Mean, MAD%, RSD% statistical modeling
│       ├── report.rs             # Markdown & JSON report formatting
│       └── bin/
│           ├── bench_ab.rs       # True Interleaved A/B orchestrator
│           └── bench_suite.rs    # Persistent zero-allocation IPC worker
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

#### **Standard A/B Benchmark (Baseline `main` vs Target Branch, 20 Iterations)**
```bash
./run_benchmark.sh
```

#### **Custom Iteration Count**
```bash
./run_benchmark.sh --iterations 5
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
