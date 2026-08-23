//! Strongly typed benchmark data reporting, domain models, JSON serialization, and Markdown rendering.

use std::fmt;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::dataset::DataCategory;

/// Benchmark operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BenchmarkOp {
    Decompression,
    Compression,
}

impl fmt::Display for BenchmarkOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BenchmarkOp::Decompression => write!(f, "Decompression"),
            BenchmarkOp::Compression => write!(f, "Compression"),
        }
    }
}

/// Detailed benchmark metrics for an individual test file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileBenchmarkResult {
    pub name: String,
    pub category: DataCategory,
    pub uncomp_bytes: usize,
    pub comp_bytes: usize,
    pub decomp_mb_s: f64,
    pub decomp_rsd: f64,
    pub decomp_mad: f64,
    pub comp_mb_s: f64,
    pub comp_rsd: f64,
    pub comp_mad: f64,
    #[serde(default)]
    pub decomp_times: Vec<f64>,
    #[serde(default)]
    pub comp_times: Vec<f64>,
}

/// Aggregate metrics for a dataset collection (e.g. Silesia Aggregate or NEXRAD).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasetAggregateResult {
    pub uncomp_bytes: usize,
    pub comp_bytes: usize,
    pub decomp_mb_s: f64,
    pub decomp_rsd: f64,
    pub decomp_mad: f64,
    pub comp_mb_s: f64,
    pub comp_rsd: f64,
    pub comp_mad: f64,
}

impl Default for DatasetAggregateResult {
    fn default() -> Self {
        Self {
            uncomp_bytes: 0,
            comp_bytes: 0,
            decomp_mb_s: 0.0,
            decomp_rsd: 0.0,
            decomp_mad: 0.0,
            comp_mb_s: 0.0,
            comp_rsd: 0.0,
            comp_mad: 0.0,
        }
    }
}

/// Complete machine-readable benchmark run report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkSuiteReport {
    pub nexrad: DatasetAggregateResult,
    pub silesia_aggregate: DatasetAggregateResult,
    pub silesia_files: Vec<FileBenchmarkResult>,
}

impl BenchmarkSuiteReport {
    pub fn save_json(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_json(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let report = serde_json::from_str(&content)?;
        Ok(report)
    }
}

/// Calculate safe percentage delta: ((target - base) / base) * 100.
pub fn safe_delta(target: f64, base: f64) -> f64 {
    if base > 0.0 {
        ((target - base) / base) * 100.0
    } else {
        0.0
    }
}

/// Format percentage speedup with bold markdown markup.
pub fn format_speedup(delta: f64) -> String {
    format!("**{:+.1}%**", delta)
}

/// System environment metadata for benchmark report header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvMetadata {
    pub cpu: String,
    pub os: String,
    pub rustc: String,
    pub iterations: usize,
    pub baseline: String,
    pub target: String,
}

impl EnvMetadata {
    pub fn detect(iterations: usize, baseline: &str, target: &str) -> Self {
        let mut cpu = "Unknown CPU".to_string();
        if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") {
            for line in cpuinfo.lines() {
                if line.starts_with("model name") {
                    if let Some((_, model)) = line.split_once(':') {
                        cpu = model.trim().to_string();
                        break;
                    }
                }
            }
        }

        let os = format!("{} {}", std::env::consts::OS, std::env::consts::ARCH);

        let rustc = std::process::Command::new("rustc")
            .arg("--version")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "rustc (unknown)".to_string());

        Self {
            cpu,
            os,
            rustc,
            iterations,
            baseline: baseline.to_string(),
            target: target.to_string(),
        }
    }
}

/// Render authoritative Markdown A/B comparison report.
pub fn render_ab_markdown_report(
    baseline: &BenchmarkSuiteReport,
    target: &BenchmarkSuiteReport,
    meta: &EnvMetadata,
) -> String {
    let mut out = Vec::new();

    out.push(format!("# Benchmark A/B Report (Iso-Thermal Interleaved): `{}` vs `{}`\n", meta.baseline, meta.target));
    out.push("### Environment & Benchmark Configuration\n".to_string());
    out.push("| Parameter | Value |".to_string());
    out.push("| :--- | :--- |".to_string());
    out.push(format!("| **CPU Model** | {} |", meta.cpu));
    out.push(format!("| **OS / Kernel** | {} |", meta.os));
    out.push(format!("| **Rust Toolchain** | `{}` |", meta.rustc));
    out.push("| **Execution Methodology** | **Iso-Thermal Interleaved A/B (CPU Pinned to Core 2)** |".to_string());
    out.push(format!("| **Iterations per File** | **{} iterations (+ 5 warmup passes)** |", meta.iterations));
    out.push(format!("| **Baseline Ref** | `{}` |", meta.baseline));
    out.push(format!("| **Target Ref** | `{}` |\n", meta.target));

    // Section 1: Aggregate Throughput
    out.push("## 1. Overall Aggregate Throughput (Median ± MAD% Dispersion)\n".to_string());
    out.push(format!("| Dataset | Operation | Baseline (`{}`) | Target (`{}`) | Throughput Delta | Speedup |", meta.baseline, meta.target));
    out.push("| :--- | :--- | :--- | :--- | :--- | :--- |".to_string());

    let b_nex = &baseline.nexrad;
    let t_nex = &target.nexrad;
    let b_sil = &baseline.silesia_aggregate;
    let t_sil = &target.silesia_aggregate;

    let nex_d_decomp = safe_delta(t_nex.decomp_mb_s, b_nex.decomp_mb_s);
    let nex_d_comp = safe_delta(t_nex.comp_mb_s, b_nex.comp_mb_s);
    let sil_d_decomp = safe_delta(t_sil.decomp_mb_s, b_sil.decomp_mb_s);
    let sil_d_comp = safe_delta(t_sil.comp_mb_s, b_sil.comp_mb_s);

    out.push(format!(
        "| **NEXRAD Radar** | {} | {:.2} MB/s (±{:.1}% MAD) | **{:.2} MB/s (±{:.1}% MAD)** | {:+.2} MB/s | {} |",
        BenchmarkOp::Decompression, b_nex.decomp_mb_s, b_nex.decomp_mad, t_nex.decomp_mb_s, t_nex.decomp_mad,
        t_nex.decomp_mb_s - b_nex.decomp_mb_s, format_speedup(nex_d_decomp)
    ));
    out.push(format!(
        "| **NEXRAD Radar** | {} | {:.2} MB/s (±{:.1}% MAD) | **{:.2} MB/s (±{:.1}% MAD)** | {:+.2} MB/s | {} |",
        BenchmarkOp::Compression, b_nex.comp_mb_s, b_nex.comp_mad, t_nex.comp_mb_s, t_nex.comp_mad,
        t_nex.comp_mb_s - b_nex.comp_mb_s, format_speedup(nex_d_comp)
    ));
    out.push(format!(
        "| **Silesia Corpus** | {} | {:.2} MB/s (±{:.1}% MAD) | **{:.2} MB/s (±{:.1}% MAD)** | {:+.2} MB/s | {} |",
        BenchmarkOp::Decompression, b_sil.decomp_mb_s, b_sil.decomp_mad, t_sil.decomp_mb_s, t_sil.decomp_mad,
        t_sil.decomp_mb_s - b_sil.decomp_mb_s, format_speedup(sil_d_decomp)
    ));
    out.push(format!(
        "| **Silesia Corpus** | {} | {:.2} MB/s (±{:.1}% MAD) | **{:.2} MB/s (±{:.1}% MAD)** | {:+.2} MB/s | {} |",
        BenchmarkOp::Compression, b_sil.comp_mb_s, b_sil.comp_mad, t_sil.comp_mb_s, t_sil.comp_mad,
        t_sil.comp_mb_s - b_sil.comp_mb_s, format_speedup(sil_d_comp)
    ));

    // Section 2: Silesia Decompression Breakdown
    out.push("\n## 2. Silesia Decompression Performance (Median ± MAD% Dispersion)\n".to_string());
    out.push(format!("| File Name | Data Type Category | Size | Baseline (`{}`) | Target (`{}`) | Speedup |", meta.baseline, meta.target));
    out.push("| :--- | :--- | :--- | :--- | :--- | :--- |".to_string());

    for tf in &target.silesia_files {
        let bf = baseline.silesia_files.iter().find(|f| f.name == tf.name).unwrap_or(tf);
        let fname = tf.name.replace("silesia_", "");
        let sz = tf.uncomp_bytes as f64 / 1_000_000.0;
        let d = safe_delta(tf.decomp_mb_s, bf.decomp_mb_s);

        out.push(format!(
            "| **`{}`** | {} | {:.2} MB | {:.2} MB/s (±{:.1}% MAD) | **{:.2} MB/s (±{:.1}% MAD)** | {} |",
            fname, tf.category, sz, bf.decomp_mb_s, bf.decomp_mad, tf.decomp_mb_s, tf.decomp_mad, format_speedup(d)
        ));
    }

    // Section 3: Silesia Compression Breakdown
    out.push("\n## 3. Silesia Compression Performance (Median ± MAD% Dispersion)\n".to_string());
    out.push(format!("| File Name | Data Type Category | Size | Ratio | Baseline (`{}`) | Target (`{}`) | Speedup |", meta.baseline, meta.target));
    out.push("| :--- | :--- | :--- | :--- | :--- | :--- | :--- |".to_string());

    for tf in &target.silesia_files {
        let bf = baseline.silesia_files.iter().find(|f| f.name == tf.name).unwrap_or(tf);
        let fname = tf.name.replace("silesia_", "");
        let sz = tf.uncomp_bytes as f64 / 1_000_000.0;
        let ratio = (tf.comp_bytes as f64 / tf.uncomp_bytes as f64) * 100.0;
        let d = safe_delta(tf.comp_mb_s, bf.comp_mb_s);

        out.push(format!(
            "| **`{}`** | {} | {:.2} MB | {:.2}% | {:.2} MB/s (±{:.1}% MAD) | **{:.2} MB/s (±{:.1}% MAD)** | {} |",
            fname, tf.category, sz, ratio, bf.comp_mb_s, bf.comp_mad, tf.comp_mb_s, tf.comp_mad, format_speedup(d)
        ));
    }

    out.join("\n")
}
