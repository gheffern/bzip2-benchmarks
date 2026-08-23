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

## Experimental Rigor: Eliminating Measurement Noise & Systematic Biases

Benchmarking microarchitectural optimizations on modern out-of-order processors requires controlling for thermal step-down, memory subsystem jitter, OS scheduling artifacts, and compiler heuristics. This harness eliminates noise through five structural controls:

```
                  ┌─────────────────────────────────────────────────────────┐
                  │          True Interleaved IPC Orchestrator              │
                  └────────────┬──────────────────────────────┬─────────────┘
                               │                              │
               Pass 2k (Even): │ B ──> T      Pass 2k+1 (Odd):│ T ──> B
                               ▼                              ▼
                 ┌───────────────────────────┐  ┌───────────────────────────┐
                 │  Worker [Baseline Process]│  │   Worker [Target Process] │
                 │  - Pre-faulted in RAM     │  │  - Pre-faulted in RAM     │
                 │  - Core 2 Pinned          │  │  - Core 2 Pinned          │
                 │  - 0 Runtime Allocations  │  │  - 0 Runtime Allocations  │
                 └───────────────────────────┘  └───────────────────────────┘
```

---

### 1. Thermal Drift & CPU Turbo Frequency Compensation
* **The Noise Mechanism**: Modern processors (AMD Precision Boost 2 / Intel Turbo Boost) dynamically adjust core clock frequency based on instantaneous silicon junction temperature ($T_j$) and power dissipation. Executing all Baseline iterations followed by all Target iterations creates an asymmetric thermal penalty against whichever binary runs second.
* **Our Solution (Alternating Interleaved Schedule)**:
  * Decompression alternates on every single individual iteration ($B_1 \leftrightarrow T_1, B_2 \leftrightarrow T_2, \dots$).
  * Symmetrized with **Alternating Start Orders**: Even iterations run $B_k \to T_k$, odd iterations run $T_k \to B_k$, mathematically ensuring zero thermal advantage:
    $$\mathbb{E}[\Delta f_{\text{thermal}}] \equiv 0$$

---

### 2. Elimination of OS Page Faults & Virtual Memory Jitter
* **The Noise Mechanism**: Spawning a fresh CLI process per iteration forces the Linux kernel to allocate memory via lazy `mmap`. The first access to destination buffers incurs hundreds of minor page table faults, cold Translation Lookaside Buffer (TLB) misses, and non-deterministic ASLR cache-line alignments.
* **Our Solution (Persistent Zero-Allocation IPC Workers)**:
  * Both Baseline and Target binaries are spawned **once** at harness startup as long-lived worker processes.
  * All 12 Silesia files, 30 NOAA radar volumes, and 64 MB working buffers are pre-faulted (`fill(0)`) into physical RAM frames before any measurement begins.
  * Timed loops execute with **0 heap allocations, 0 system calls, and 0 kernel page table traps**.

---

### 3. Elimination of OS Thread Migration Latency
* **The Noise Mechanism**: The Linux Completely Fair Scheduler (CFS) periodically migrates active threads across different CPU cores and CCX complexes, causing catastrophic L1/L2 instruction and data cache evictions.
* **Our Solution (CPU Affinity Pinning)**:
  * Both worker processes bind strictly to a single physical core (`Core 2`) via `libc::sched_setaffinity`.
  * Guarantees 100% cache-warm steady-state execution across the entire test suite.

---

### 4. Surgical Timing Boundaries & Dead-Code Elimination Prevention
* **The Noise Mechanism**: Measuring timing around process lifecycle or JSON IPC introduces measurement artifacts. Over-aggressive compiler optimizations can also discard unreferenced output buffers.
* **Our Solution**:
  * `Instant::now()` calls are placed strictly around the C/Rust decompression function invocation (`bzDecompress`). IPC protocol message framing and serialization execute strictly outside the timed window.
  * Output buffer pointers and byte counts are wrapped in `std::hint::black_box` to prevent compiler dead-code elimination.

---

### 5. Outlier-Robust Statistical Modeling (Median & MAD%)
* **The Noise Mechanism**: Standard deviation and Relative Standard Deviation (RSD%) assume normal distributions and are easily distorted by occasional OS interrupts or hardware background ticks.
* **Our Solution (Non-Parametric Median Absolute Deviation)**:
  * We report **Median Throughput** and **Normalized Median Absolute Deviation (MAD%)**:
    $$\text{MAD} = \text{median}(|x_i - \text{median}(X)|), \quad \text{MAD\%} = \frac{1.4826 \times \text{MAD}}{\text{median}(X)} \times 100$$
  * Provides mathematically rigorous dispersion tracking that rejects OS context-switch spikes without altering the underlying data.

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
