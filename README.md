# bzip2 Optimization Benchmarks & Verification Suite

A reproducible, high-throughput benchmarking and verification suite for [`libbzip2-rs`](https://github.com/trifectatechfoundation/libbzip2-rs) performance optimizations.

---

## Datasets Overview

1. **NOAA NEXRAD Level-2 Radar Data**:
   - 30 multi-stream volume archives (~1.5 GB uncompressed, ~45 MB compressed, 3.16% ratio).
   - Real-world binary radar sweeps containing ~54 concatenated bzip2 streams per volume with 4-byte big-endian headers.
2. **Canonical Silesia Compression Corpus**:
   - All 12 un-truncated canonical files (~212 MB uncompressed, ~52 MB compressed).
   - Covers diverse real-world data types: ASCII literature, x86 binaries, C source tars, XML, medical MRI/X-ray imaging, database records, and PDF.

---

## Benchmark Results (20 Iterations)

### 1. Overall Aggregate Throughput

| Dataset | Metric / Operation | Baseline (`main` v0.2.5) | Target (`feature/perf-optimizations`) | Throughput Gain |
| :--- | :--- | :--- | :--- | :--- |
| **NEXRAD Radar** | Decompression | 266.41 MB/s | **359.10 MB/s** | **+34.8% 🚀** |
| **NEXRAD Radar** | Compression | 110.93 MB/s | **111.06 MB/s** | **+0.1%** |
| **Silesia Corpus** | Decompression | 49.42 MB/s | **52.14 MB/s** | **+5.5% 🚀** |
| **Silesia Corpus** | Compression | 19.63 MB/s | **19.69 MB/s** | **+0.3%** |
| **Overall Combined** | **Decompression** | 185.23 MB/s | **224.77 MB/s** | **+21.4% 🚀** |
| **Overall Combined** | **Compression** | 68.90 MB/s | **70.09 MB/s** | **+1.7%** |

---

### 2. NEXRAD Radar Dataset Details

| Characteristic | Measurement | Notes |
| :--- | :--- | :--- |
| **Volume Archives Tested** | 30 volume archives | Full Level-2 radar sweeps (`nexrad1.bz2` – `nexrad30.bz2`) |
| **Uncompressed Volume per File** | ~47.6 MB – 52.4 MB | ~54 bzip2 streams per volume file |
| **Total Uncompressed Size** | **1,497.70 MB** (~1.5 GB) | 29,953.95 MB across 20 benchmark iterations |
| **Total Compressed Size** | **45.07 MB** | 3.16% compression ratio |
| **Decompression Speed** | **359.10 MB/s** (vs 266.41 MB/s) | **+34.8% speedup 🚀** (powered by Slice-by-4 parallel CRC32) |
| **Compression Speed** | **111.06 MB/s** (vs 110.93 MB/s) | **+0.1% speedup** |

---

### 3. Silesia Per-File & Subtype Decompression Breakdown

| File Name | Data Type Category | Size | Baseline (`main` v0.2.5) | Target (`feature/perf-optimizations`) | Speedup |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **`mr`** | Medical (MRI Image) | 9.97 MB | 45.33 MB/s | **62.99 MB/s** | **+39.0% 🚀** |
| **`reymont`** | PDF Document | 6.63 MB | 43.69 MB/s | **54.56 MB/s** | **+24.9% 🚀** |
| **`samba`** | Tar / C Source Code | 21.61 MB | 60.78 MB/s | **68.10 MB/s** | **+12.0% 🚀** |
| **`ooffice`** | x86 Executable / DLL | 6.15 MB | 36.66 MB/s | **40.67 MB/s** | **+10.9% 🚀** |
| **`osdb`** | DB Binary Records | 10.09 MB | 44.79 MB/s | **48.34 MB/s** | **+7.9% 🚀** |
| **`mozilla`** | Tar / Executables | 51.22 MB | 47.38 MB/s | **50.40 MB/s** | **+6.4% 🚀** |
| **`webster`** | English Dictionary Text | 41.46 MB | 47.35 MB/s | **50.14 MB/s** | **+5.9% 🚀** |
| **`dickens`** | Text (ASCII Literature) | 10.19 MB | 43.83 MB/s | **45.33 MB/s** | **+3.4% 🚀** |
| **`sao`** | Astronomical Star Catalog | 7.25 MB | 32.17 MB/s | **33.07 MB/s** | **+2.8% 🚀** |
| **`nci`** | Chemistry Database / Text | 33.55 MB | 72.32 MB/s | **73.52 MB/s** | **+1.7% 🚀** |
| **`xml`** | Structured XML Markup | 5.35 MB | 79.53 MB/s | **77.79 MB/s** | **−2.2%** |
| **`x-ray`** | Medical (X-Ray Image) | 8.47 MB | 38.03 MB/s | **28.65 MB/s** | **−24.7%** |

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

#### **A/B Comparison (Baseline `main` vs Active Branch)**
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
