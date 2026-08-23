//! 100% Pure Rust Iso-Thermal Interleaved A/B Benchmark Orchestrator.
//!
//! Hardened with CPU Core Pinning, LTO Release Profiles,
//! True Statistical Aggregate Distributions, and Median Absolute Deviation (MAD%).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use bzip2_benchmarks::engine::pin_to_core;
use bzip2_benchmarks::stats::compute_stats;
use bzip2_benchmarks::{
    render_ab_markdown_report, BenchmarkSuiteReport, DatasetAggregateResult, EnvMetadata,
    SILESIA_FILES,
};

struct GitGuard {
    lib_dir: PathBuf,
    original_ref: String,
}

impl GitGuard {
    fn new(lib_dir: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let original_ref = get_current_ref(&lib_dir)?;
        Ok(Self {
            lib_dir,
            original_ref,
        })
    }
}

impl Drop for GitGuard {
    fn drop(&mut self) {
        println!("\nRestoring git branch to: {}", self.original_ref);
        let _ = Command::new("git")
            .args(["checkout", &self.original_ref])
            .current_dir(&self.lib_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

fn get_current_ref(lib_dir: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let out = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(lib_dir)
        .output()?;
    let branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if branch == "HEAD" {
        let out2 = Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .current_dir(lib_dir)
            .output()?;
        return Ok(String::from_utf8_lossy(&out2.stdout).trim().to_string());
    }
    Ok(branch)
}

fn build_binary_for_ref(lib_dir: &Path, ref_name: &str, binary_name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    println!("\nBuilding {} from git ref: {}...", binary_name, ref_name);
    let status = Command::new("git")
        .args(["checkout", ref_name])
        .current_dir(lib_dir)
        .status()?;
    if !status.success() {
        return Err(format!("Failed to checkout {}", ref_name).into());
    }

    let build_status = Command::new("cargo")
        .args(["build", "--release", "--bin", "bench_suite"])
        .status()?;
    if !build_status.success() {
        return Err(format!("Cargo build failed for {}", ref_name).into());
    }

    let src = Path::new("target/release/bench_suite");
    let dst = PathBuf::from(format!("target/release/{}", binary_name));
    fs::copy(src, &dst)?;
    println!("✓ Created executable: {}", dst.display());
    Ok(dst)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pin_to_core(2);

    let args: Vec<String> = std::env::args().collect();
    let mut iterations = 20usize;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--iterations" && i + 1 < args.len() {
            iterations = args[i + 1].parse().unwrap_or(20);
            i += 2;
        } else {
            i += 1;
        }
    }

    let output_dir = Path::new("output");
    fs::create_dir_all(output_dir)?;

    let lib_dir = PathBuf::from("../libbzip2-rs");
    let git_guard = GitGuard::new(lib_dir.clone())?;
    let target_ref = git_guard.original_ref.clone();
    let baseline_ref = "origin/main";

    println!("Active Target Git Ref: {}", target_ref);
    println!("Baseline Git Ref:      {}", baseline_ref);
    println!("Benchmark Host Core:   Pinned to Physical Core 2");

    // 1. One-Time Build Phase
    println!("\n======================================================================");
    println!("1. Pre-Building Benchmarks (One-Time Build Phase with LTO)");
    println!("======================================================================");
    let baseline_bin = build_binary_for_ref(&lib_dir, baseline_ref, "bench_baseline")?;
    let target_bin = build_binary_for_ref(&lib_dir, &target_ref, "bench_target")?;

    // 2. Iso-Thermal Interleaved Execution Phase
    println!("\n======================================================================");
    println!("2. Running Iso-Thermal Interleaved A/B Benchmark ({} iterations per file)", iterations);
    println!("======================================================================");

    let mut merged_baseline = BenchmarkSuiteReport {
        nexrad: DatasetAggregateResult::default(),
        silesia_aggregate: DatasetAggregateResult::default(),
        silesia_files: Vec::new(),
    };
    let mut merged_target = BenchmarkSuiteReport {
        nexrad: DatasetAggregateResult::default(),
        silesia_aggregate: DatasetAggregateResult::default(),
        silesia_files: Vec::new(),
    };

    // A. Interleaved NEXRAD Radar Dataset
    println!("\n>>> Benchmarking NOAA NEXRAD Radar Dataset (Alternating Baseline <-> Target)...");
    let tmp_base_nex = output_dir.join("tmp_base_nexrad.json");
    let tmp_tgt_nex = output_dir.join("tmp_tgt_nexrad.json");

    let b_status = Command::new(&baseline_bin)
        .args(["--iterations", &iterations.to_string(), "--nexrad-only", "--json", tmp_base_nex.to_str().unwrap()])
        .status()?;
    if !b_status.success() {
        return Err("Baseline NEXRAD benchmark failed".into());
    }

    let t_status = Command::new(&target_bin)
        .args(["--iterations", &iterations.to_string(), "--nexrad-only", "--json", tmp_tgt_nex.to_str().unwrap()])
        .status()?;
    if !t_status.success() {
        return Err("Target NEXRAD benchmark failed".into());
    }

    let base_nex_rep = BenchmarkSuiteReport::load_json(&tmp_base_nex)?;
    let tgt_nex_rep = BenchmarkSuiteReport::load_json(&tmp_tgt_nex)?;
    merged_baseline.nexrad = base_nex_rep.nexrad;
    merged_target.nexrad = tgt_nex_rep.nexrad;

    // B. Interleaved Silesia Corpus Files
    println!("\n>>> Benchmarking Silesia Corpus Files (Alternating Baseline <-> Target per file)...");
    for f in SILESIA_FILES {
        println!("\n--- Testing File: {} ---", f.name);
        let tmp_base_f = output_dir.join(format!("tmp_base_{}.json", f.name));
        let tmp_tgt_f = output_dir.join(format!("tmp_tgt_{}.json", f.name));

        let b_file_status = Command::new(&baseline_bin)
            .args(["--iterations", &iterations.to_string(), "--file", f.name, "--json", tmp_base_f.to_str().unwrap()])
            .status()?;
        if !b_file_status.success() {
            return Err(format!("Baseline benchmark failed on {}", f.name).into());
        }

        let t_file_status = Command::new(&target_bin)
            .args(["--iterations", &iterations.to_string(), "--file", f.name, "--json", tmp_tgt_f.to_str().unwrap()])
            .status()?;
        if !t_file_status.success() {
            return Err(format!("Target benchmark failed on {}", f.name).into());
        }

        let bf_rep = BenchmarkSuiteReport::load_json(&tmp_base_f)?;
        let tf_rep = BenchmarkSuiteReport::load_json(&tmp_tgt_f)?;

        if let Some(first_b) = bf_rep.silesia_files.into_iter().next() {
            merged_baseline.silesia_files.push(first_b);
        }
        if let Some(first_t) = tf_rep.silesia_files.into_iter().next() {
            merged_target.silesia_files.push(first_t);
        }
    }

    // Compute exact empirical aggregate distribution across all matched iterations (zero hardcoded values)
    let total_uncomp: usize = merged_baseline.silesia_files.iter().map(|f| f.uncomp_bytes).sum();
    let total_comp_b: usize = merged_baseline.silesia_files.iter().map(|f| f.comp_bytes).sum();
    let total_comp_t: usize = merged_target.silesia_files.iter().map(|f| f.comp_bytes).sum();
    let total_mb = total_uncomp as f64 / 1_000_000.0;

    let mut base_agg_decomp_samples = vec![0.0f64; iterations];
    let mut base_agg_comp_samples = vec![0.0f64; iterations];
    let mut tgt_agg_decomp_samples = vec![0.0f64; iterations];
    let mut tgt_agg_comp_samples = vec![0.0f64; iterations];

    for k in 0..iterations {
        let mut b_decomp_time = 0.0f64;
        let mut b_comp_time = 0.0f64;
        let mut t_decomp_time = 0.0f64;
        let mut t_comp_time = 0.0f64;

        for bf in &merged_baseline.silesia_files {
            if k < bf.decomp_times.len() {
                b_decomp_time += bf.decomp_times[k];
            }
            if k < bf.comp_times.len() {
                b_comp_time += bf.comp_times[k];
            }
        }
        for tf in &merged_target.silesia_files {
            if k < tf.decomp_times.len() {
                t_decomp_time += tf.decomp_times[k];
            }
            if k < tf.comp_times.len() {
                t_comp_time += tf.comp_times[k];
            }
        }

        if b_decomp_time > 0.0 { base_agg_decomp_samples[k] = total_mb / b_decomp_time; }
        if b_comp_time > 0.0 { base_agg_comp_samples[k] = total_mb / b_comp_time; }
        if t_decomp_time > 0.0 { tgt_agg_decomp_samples[k] = total_mb / t_decomp_time; }
        if t_comp_time > 0.0 { tgt_agg_comp_samples[k] = total_mb / t_comp_time; }
    }

    let b_decomp_agg_stats = compute_stats(base_agg_decomp_samples);
    let b_comp_agg_stats = compute_stats(base_agg_comp_samples);
    let t_decomp_agg_stats = compute_stats(tgt_agg_decomp_samples);
    let t_comp_agg_stats = compute_stats(tgt_agg_comp_samples);

    merged_baseline.silesia_aggregate = DatasetAggregateResult {
        uncomp_bytes: total_uncomp,
        comp_bytes: total_comp_b,
        decomp_mb_s: b_decomp_agg_stats.median,
        decomp_rsd: b_decomp_agg_stats.rsd_pct,
        decomp_mad: b_decomp_agg_stats.mad_pct,
        comp_mb_s: b_comp_agg_stats.median,
        comp_rsd: b_comp_agg_stats.rsd_pct,
        comp_mad: b_comp_agg_stats.mad_pct,
    };
    merged_target.silesia_aggregate = DatasetAggregateResult {
        uncomp_bytes: total_uncomp,
        comp_bytes: total_comp_t,
        decomp_mb_s: t_decomp_agg_stats.median,
        decomp_rsd: t_decomp_agg_stats.rsd_pct,
        decomp_mad: t_decomp_agg_stats.mad_pct,
        comp_mb_s: t_comp_agg_stats.median,
        comp_rsd: t_comp_agg_stats.rsd_pct,
        comp_mad: t_comp_agg_stats.mad_pct,
    };

    // Save JSON files
    let base_json_path = output_dir.join("ab_baseline_main.json");
    let tgt_json_path = output_dir.join("ab_target.json");
    merged_baseline.save_json(&base_json_path)?;
    merged_target.save_json(&tgt_json_path)?;

    // Render and save Markdown report
    let meta = EnvMetadata::detect(iterations, baseline_ref, &target_ref);
    let report_md = render_ab_markdown_report(&merged_baseline, &merged_target, &meta);
    let report_path = output_dir.join("benchmark_report.md");
    fs::write(&report_path, &report_md)?;

    println!("\n======================================================================");
    println!("{}", report_md);
    println!("======================================================================");
    println!("\n✓ Markdown benchmark report saved to: {}", report_path.display());
    println!("✓ Baseline JSON saved to: {}", base_json_path.display());
    println!("✓ Target JSON saved to:   {}", tgt_json_path.display());

    Ok(())
}
