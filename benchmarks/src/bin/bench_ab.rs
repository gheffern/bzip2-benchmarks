//! 100% Pure Rust Iso-Thermal Interleaved A/B Benchmark Orchestrator.
//!
//! Hardened with CPU Core Pinning, LTO Release Profiles,
//! True Iteration-by-Iteration Alternating Start Order (B -> T on even passes, T -> B on odd passes),
//! Persistent Zero-Allocation IPC Workers, and Exact Empirical Aggregate Distributions.

use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use bzip2_benchmarks::engine::{pin_to_core, WorkerCommand, WorkerResponse, WARMUP_ITERATIONS};
use bzip2_benchmarks::stats::compute_stats;
use bzip2_benchmarks::{
    render_ab_markdown_report, BenchmarkSuiteReport, DatasetAggregateResult, EnvMetadata,
    FileBenchmarkResult, SILESIA_FILES,
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

/// Client handle for managing an IPC worker process.
struct WorkerClient {
    name: String,
    child: Child,
    writer: BufWriter<std::process::ChildStdin>,
    reader: BufReader<std::process::ChildStdout>,
}

impl WorkerClient {
    fn spawn(name: &str, binary: &Path, core_id: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let mut child = Command::new(binary)
            .args(["--worker", "--core", &core_id.to_string()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("Failed to spawn worker {}: {}", binary.display(), e))?;

        let stdin = child.stdin.take().ok_or("Failed to open child stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to open child stdout")?;
        let mut client = Self {
            name: name.to_string(),
            child,
            writer: BufWriter::new(stdin),
            reader: BufReader::new(stdout),
        };

        let mut line = String::new();
        client.reader.read_line(&mut line)?;
        let resp: WorkerResponse = serde_json::from_str(&line)
            .map_err(|e| format!("Invalid ready response from {}: {}. Raw: {:?}", name, e, line))?;

        match resp {
            WorkerResponse::Ready { silesia_count, nexrad_count } => {
                println!("✓ Worker [{}] online: {} Silesia files, {} NEXRAD archives resident in memory.", name, silesia_count, nexrad_count);
            }
            _ => return Err(format!("Unexpected startup response from {}: {:?}", name, resp).into()),
        }

        Ok(client)
    }

    fn warmup(&mut self, target: &str, op: &str, iterations: usize) -> Result<(), Box<dyn std::error::Error>> {
        let cmd = WorkerCommand::Warmup {
            target: target.to_string(),
            op: op.to_string(),
            iterations,
        };
        self.send_cmd(&cmd)?;
        let resp = self.read_resp()?;
        match resp {
            WorkerResponse::WarmupDone => Ok(()),
            WorkerResponse::Error { message } => Err(format!("Worker {} warmup error: {}", self.name, message).into()),
            _ => Err(format!("Worker {} unexpected warmup response: {:?}", self.name, resp).into()),
        }
    }

    fn run_iteration(&mut self, target: &str, op: &str) -> Result<(f64, usize, usize), Box<dyn std::error::Error>> {
        let cmd = WorkerCommand::RunIteration {
            target: target.to_string(),
            op: op.to_string(),
        };
        self.send_cmd(&cmd)?;
        let resp = self.read_resp()?;
        match resp {
            WorkerResponse::IterationSuccess { elapsed_secs, uncomp_bytes, comp_bytes } => {
                Ok((elapsed_secs, uncomp_bytes, comp_bytes))
            }
            WorkerResponse::Error { message } => Err(format!("Worker {} iteration error: {}", self.name, message).into()),
            _ => Err(format!("Worker {} unexpected iteration response: {:?}", self.name, resp).into()),
        }
    }

    fn send_cmd(&mut self, cmd: &WorkerCommand) -> Result<(), Box<dyn std::error::Error>> {
        let mut json = serde_json::to_string(cmd)?;
        json.push('\n');
        self.writer.write_all(json.as_bytes())?;
        self.writer.flush()?;
        Ok(())
    }

    fn read_resp(&mut self) -> Result<WorkerResponse, Box<dyn std::error::Error>> {
        let mut line = String::new();
        self.reader.read_line(&mut line)?;
        if line.is_empty() {
            return Err(format!("Worker {} terminated unexpectedly (EOF on stdout)", self.name).into());
        }
        let resp: WorkerResponse = serde_json::from_str(&line)?;
        Ok(resp)
    }

    fn shutdown(mut self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.send_cmd(&WorkerCommand::Exit);
        let _ = self.child.wait();
        Ok(())
    }
}

impl Drop for WorkerClient {
    fn drop(&mut self) {
        let _ = self.send_cmd(&WorkerCommand::Exit);
        let _ = self.child.kill();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const BENCHMARK_CORE_ID: usize = 2;
    pin_to_core(BENCHMARK_CORE_ID);

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
    println!("Benchmark Host Core:   Pinned to Physical Core {}", BENCHMARK_CORE_ID);

    // 1. One-Time Build Phase
    println!("\n======================================================================");
    println!("1. Pre-Building Benchmarks (One-Time Build Phase with LTO)");
    println!("======================================================================");
    let baseline_bin = build_binary_for_ref(&lib_dir, baseline_ref, "bench_baseline")?;
    let target_bin = build_binary_for_ref(&lib_dir, &target_ref, "bench_target")?;

    // 2. Launch Persistent IPC Workers
    println!("\n======================================================================");
    println!("2. Initializing Persistent Zero-Allocation IPC Workers");
    println!("======================================================================");
    let mut base_worker = WorkerClient::spawn("Baseline", &baseline_bin, BENCHMARK_CORE_ID)?;
    let mut tgt_worker = WorkerClient::spawn("Target", &target_bin, BENCHMARK_CORE_ID)?;

    // 3. True Iteration-by-Iteration Interleaved Execution
    println!("\n======================================================================");
    println!(
        "3. Running True Interleaved A/B Benchmark ({} iterations per file)",
        iterations
    );
    println!("   Strategy: Alternating Start Order (B->T on even passes, T->B on odd passes)");
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
    println!("\n>>> Benchmarking NOAA NEXRAD Radar Dataset (Alternating B <-> T)...");
    base_worker.warmup("nexrad", "decomp", WARMUP_ITERATIONS)?;
    tgt_worker.warmup("nexrad", "decomp", WARMUP_ITERATIONS)?;

    let mut b_nex_decomp_times = vec![0.0f64; iterations];
    let mut t_nex_decomp_times = vec![0.0f64; iterations];
    let mut nex_uncomp = 0usize;
    let mut nex_comp = 0usize;

    for k in 0..iterations {
        if k % 2 == 0 {
            let (tb, u, c) = base_worker.run_iteration("nexrad", "decomp")?;
            let (tt, _, _) = tgt_worker.run_iteration("nexrad", "decomp")?;
            b_nex_decomp_times[k] = tb;
            t_nex_decomp_times[k] = tt;
            nex_uncomp = u;
            nex_comp = c;
        } else {
            let (tt, _, _) = tgt_worker.run_iteration("nexrad", "decomp")?;
            let (tb, u, c) = base_worker.run_iteration("nexrad", "decomp")?;
            t_nex_decomp_times[k] = tt;
            b_nex_decomp_times[k] = tb;
            nex_uncomp = u;
            nex_comp = c;
        }
    }

    base_worker.warmup("nexrad", "comp", WARMUP_ITERATIONS)?;
    tgt_worker.warmup("nexrad", "comp", WARMUP_ITERATIONS)?;

    let mut b_nex_comp_times = vec![0.0f64; iterations];
    let mut t_nex_comp_times = vec![0.0f64; iterations];

    for k in 0..iterations {
        if k % 2 == 0 {
            let (tb, _, _) = base_worker.run_iteration("nexrad", "comp")?;
            let (tt, _, _) = tgt_worker.run_iteration("nexrad", "comp")?;
            b_nex_comp_times[k] = tb;
            t_nex_comp_times[k] = tt;
        } else {
            let (tt, _, _) = tgt_worker.run_iteration("nexrad", "comp")?;
            let (tb, _, _) = base_worker.run_iteration("nexrad", "comp")?;
            t_nex_comp_times[k] = tt;
            b_nex_comp_times[k] = tb;
        }
    }

    let nex_mb = nex_uncomp as f64 / 1_000_000.0;
    let b_nex_decomp_stats = compute_stats(b_nex_decomp_times.iter().map(|&t| nex_mb / t).collect());
    let t_nex_decomp_stats = compute_stats(t_nex_decomp_times.iter().map(|&t| nex_mb / t).collect());
    let b_nex_comp_stats = compute_stats(b_nex_comp_times.iter().map(|&t| nex_mb / t).collect());
    let t_nex_comp_stats = compute_stats(t_nex_comp_times.iter().map(|&t| nex_mb / t).collect());

    println!(
        "NEXRAD Decomp: Base {:.2} MB/s (±{:.1}%) | Tgt {:.2} MB/s (±{:.1}%) -> {:+.1}%",
        b_nex_decomp_stats.median, b_nex_decomp_stats.mad_pct,
        t_nex_decomp_stats.median, t_nex_decomp_stats.mad_pct,
        ((t_nex_decomp_stats.median - b_nex_decomp_stats.median) / b_nex_decomp_stats.median) * 100.0
    );
    println!(
        "NEXRAD Comp:   Base {:.2} MB/s (±{:.1}%) | Tgt {:.2} MB/s (±{:.1}%) -> {:+.1}%",
        b_nex_comp_stats.median, b_nex_comp_stats.mad_pct,
        t_nex_comp_stats.median, t_nex_comp_stats.mad_pct,
        ((t_nex_comp_stats.median - b_nex_comp_stats.median) / b_nex_comp_stats.median) * 100.0
    );

    merged_baseline.nexrad = DatasetAggregateResult {
        uncomp_bytes: nex_uncomp,
        comp_bytes: nex_comp,
        decomp_mb_s: b_nex_decomp_stats.median,
        decomp_rsd: b_nex_decomp_stats.rsd_pct,
        decomp_mad: b_nex_decomp_stats.mad_pct,
        comp_mb_s: b_nex_comp_stats.median,
        comp_rsd: b_nex_comp_stats.rsd_pct,
        comp_mad: b_nex_comp_stats.mad_pct,
    };
    merged_target.nexrad = DatasetAggregateResult {
        uncomp_bytes: nex_uncomp,
        comp_bytes: nex_comp,
        decomp_mb_s: t_nex_decomp_stats.median,
        decomp_rsd: t_nex_decomp_stats.rsd_pct,
        decomp_mad: t_nex_decomp_stats.mad_pct,
        comp_mb_s: t_nex_comp_stats.median,
        comp_rsd: t_nex_comp_stats.rsd_pct,
        comp_mad: t_nex_comp_stats.mad_pct,
    };

    // B. Interleaved Silesia Corpus Files
    println!("\n>>> Benchmarking Silesia Corpus Files (Alternating B <-> T per iteration)...");
    println!(
        "{:<16} | {:>14} | {:>14} | {:>8} | {:>14} | {:>14} | {:>8}",
        "File", "Base Decomp", "Tgt Decomp", "D-Speed", "Base Comp", "Tgt Comp", "C-Speed"
    );
    println!("{:-<16}-|-{:->14}-|-{:->14}-|-{:->8}-|-{:->14}-|-{:->14}-|-{:->8}", "", "", "", "", "", "", "");

    for f in SILESIA_FILES {
        let fname = f.name;

        // Decompression Warmup + Interleaved Passes
        base_worker.warmup(fname, "decomp", WARMUP_ITERATIONS)?;
        tgt_worker.warmup(fname, "decomp", WARMUP_ITERATIONS)?;

        let mut b_decomp_times = vec![0.0f64; iterations];
        let mut t_decomp_times = vec![0.0f64; iterations];
        let mut file_uncomp = 0usize;
        let mut file_comp = 0usize;

        for k in 0..iterations {
            if k % 2 == 0 {
                let (tb, u, c) = base_worker.run_iteration(fname, "decomp")?;
                let (tt, _, _) = tgt_worker.run_iteration(fname, "decomp")?;
                b_decomp_times[k] = tb;
                t_decomp_times[k] = tt;
                file_uncomp = u;
                file_comp = c;
            } else {
                let (tt, _, _) = tgt_worker.run_iteration(fname, "decomp")?;
                let (tb, u, c) = base_worker.run_iteration(fname, "decomp")?;
                t_decomp_times[k] = tt;
                b_decomp_times[k] = tb;
                file_uncomp = u;
                file_comp = c;
            }
        }

        // Compression Warmup + Interleaved Passes
        base_worker.warmup(fname, "comp", WARMUP_ITERATIONS)?;
        tgt_worker.warmup(fname, "comp", WARMUP_ITERATIONS)?;

        let mut b_comp_times = vec![0.0f64; iterations];
        let mut t_comp_times = vec![0.0f64; iterations];

        for k in 0..iterations {
            if k % 2 == 0 {
                let (tb, _, _) = base_worker.run_iteration(fname, "comp")?;
                let (tt, _, _) = tgt_worker.run_iteration(fname, "comp")?;
                b_comp_times[k] = tb;
                t_comp_times[k] = tt;
            } else {
                let (tt, _, _) = tgt_worker.run_iteration(fname, "comp")?;
                let (tb, _, _) = base_worker.run_iteration(fname, "comp")?;
                t_comp_times[k] = tt;
                b_comp_times[k] = tb;
            }
        }

        let f_mb = file_uncomp as f64 / 1_000_000.0;
        let b_d_stats = compute_stats(b_decomp_times.iter().map(|&t| f_mb / t).collect());
        let t_d_stats = compute_stats(t_decomp_times.iter().map(|&t| f_mb / t).collect());
        let b_c_stats = compute_stats(b_comp_times.iter().map(|&t| f_mb / t).collect());
        let t_c_stats = compute_stats(t_comp_times.iter().map(|&t| f_mb / t).collect());

        let d_speedup = ((t_d_stats.median - b_d_stats.median) / b_d_stats.median) * 100.0;
        let c_speedup = ((t_c_stats.median - b_c_stats.median) / b_c_stats.median) * 100.0;

        let short_name = fname.replace("silesia_", "");
        println!(
            "{:<16} | {:>8.2} (±{:.1}%) | {:>8.2} (±{:.1}%) | {:>+7.1}% | {:>8.2} (±{:.1}%) | {:>8.2} (±{:.1}%) | {:>+7.1}%",
            short_name,
            b_d_stats.median, b_d_stats.mad_pct,
            t_d_stats.median, t_d_stats.mad_pct,
            d_speedup,
            b_c_stats.median, b_c_stats.mad_pct,
            t_c_stats.median, t_c_stats.mad_pct,
            c_speedup
        );

        merged_baseline.silesia_files.push(FileBenchmarkResult {
            name: fname.to_string(),
            category: f.category,
            uncomp_bytes: file_uncomp,
            comp_bytes: file_comp,
            decomp_mb_s: b_d_stats.median,
            decomp_rsd: b_d_stats.rsd_pct,
            decomp_mad: b_d_stats.mad_pct,
            comp_mb_s: b_c_stats.median,
            comp_rsd: b_c_stats.rsd_pct,
            comp_mad: b_c_stats.mad_pct,
            decomp_times: b_decomp_times,
            comp_times: b_comp_times,
        });

        merged_target.silesia_files.push(FileBenchmarkResult {
            name: fname.to_string(),
            category: f.category,
            uncomp_bytes: file_uncomp,
            comp_bytes: file_comp,
            decomp_mb_s: t_d_stats.median,
            decomp_rsd: t_d_stats.rsd_pct,
            decomp_mad: t_d_stats.mad_pct,
            comp_mb_s: t_c_stats.median,
            comp_rsd: t_c_stats.rsd_pct,
            comp_mad: t_c_stats.mad_pct,
            decomp_times: t_decomp_times,
            comp_times: t_comp_times,
        });
    }

    base_worker.shutdown()?;
    tgt_worker.shutdown()?;

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
