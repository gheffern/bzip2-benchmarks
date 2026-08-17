# bzip2 Optimization Benchmarks & Verification Suite

A reproducible, high-throughput benchmarking and verification suite for [`libbzip2-rs`](https://github.com/trifectatechfoundation/libbzip2-rs) performance optimizations.

---

## Datasets

1. **NOAA NEXRAD Level-2 Radar Data**:
   - 30 volume archives (~1.5 GB uncompressed, 3.16% compression ratio).
   - High-throughput binary multi-stream archives.
2. **Canonical Silesia Compression Corpus**:
   - All 12 un-truncated canonical files (~212 MB uncompressed).
   - Covers diverse data types: ASCII literature, x86 PE binaries, C source tars, XML markup, medical MRI/X-ray imaging, database records, and PDF.

---

## Benchmark Results

### 1. Overall Aggregate Throughput

| Dataset | `main` (v0.2.5) | `feature/perf-optimizations` | Throughput Gain |
| :--- | :--- | :--- | :--- |
| **NEXRAD Radar Decompression** | 284.28 MB/s | **391.74 MB/s** | **+37.8% 🚀** |
| **Silesia Corpus Decompression** | 53.50 MB/s | **56.02 MB/s** | **+4.7% 🚀** |
| **Overall Combined Decompression** | 185.23 MB/s | **224.77 MB/s** | **+21.4% 🚀** |
| **Overall Combined Compression** | 68.90 MB/s | **70.09 MB/s** | **+1.7%** |

---

### 2. Silesia Per-File & Subtype Breakdown

| File Name | Data Type Category | Uncompressed Size | `main` (v0.2.5) Decomp | `feature/perf-optimizations` Decomp | Decompression Speedup |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **`mozilla`** | Tar / Executables | 51.22 MB | 36.38 MB/s | **50.00 MB/s** | **+37.4% 🚀** |
| **`osdb`** | DB Binary Records | 10.09 MB | 38.26 MB/s | **47.99 MB/s** | **+25.4% 🚀** |
| **`mr`** | Medical (MRI Image) | 9.97 MB | 60.74 MB/s | **66.95 MB/s** | **+10.2% 🚀** |
| **`x-ray`** | Medical (X-Ray Image) | 8.47 MB | 38.52 MB/s | **42.17 MB/s** | **+9.5% 🚀** |
| **`dickens`** | Text (ASCII Literature) | 10.19 MB | 41.81 MB/s | **44.46 MB/s** | **+6.3% 🚀** |
| **`nci`** | Chemistry Database / Text | 33.55 MB | 63.67 MB/s | **66.61 MB/s** | **+4.6% 🚀** |
| **`reymont`** | PDF Document | 6.63 MB | 51.40 MB/s | **53.39 MB/s** | **+3.9% 🚀** |
| **`xml`** | Structured XML Markup | 5.35 MB | 86.79 MB/s | **88.43 MB/s** | **+1.9% 🚀** |
| **`sao`** | Astronomical Star Catalog | 7.25 MB | 33.51 MB/s | **34.06 MB/s** | **+1.6% 🚀** |
| **`webster`** | English Dictionary Text | 41.46 MB | 51.23 MB/s | **51.16 MB/s** | **−0.1%** |
| **`ooffice`** | x86 PE Executable / DLL | 6.15 MB | 39.90 MB/s | **39.76 MB/s** | **−0.4%** |
| **`samba`** | Tar / C Source Code | 21.61 MB | 65.61 MB/s | **56.55 MB/s** | **−13.8%** |

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

---

## Requirements

- **Rust & Cargo**: Any stable Rust toolchain (via [rustup.rs](https://rustup.rs))
- **Python 3**: Standard library only (no pip dependencies required)
- **Standard C Compiler**: `gcc`, `clang`, or `cc` (for standard library linking)
