//! Low-level benchmark execution worker for individual datasets or files.

use std::path::{Path, PathBuf};
use bzip2_benchmarks::{
    benchmark_nexrad, benchmark_single_file, compute_stats, load_nexrad, load_silesia,
    BenchmarkSuiteReport, DatasetAggregateResult, FileBenchmarkResult, SILESIA_FILES,
    WARMUP_ITERATIONS,
};

#[derive(Debug, Clone)]
struct CliConfig {
    iterations: usize,
    json_path: Option<PathBuf>,
    file_filter: Option<String>,
    nexrad_only: bool,
    silesia_only: bool,
}

impl CliConfig {
    fn parse_from_args() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mut iterations = 5;
        let mut json_path = None;
        let mut file_filter = None;
        let mut nexrad_only = false;
        let mut silesia_only = false;

        let mut i = 1;
        while i < args.len() {
            if args[i] == "--iterations" && i + 1 < args.len() {
                iterations = args[i + 1].parse().unwrap_or(5);
                i += 2;
            } else if args[i] == "--json" && i + 1 < args.len() {
                json_path = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            } else if args[i] == "--file" && i + 1 < args.len() {
                file_filter = Some(args[i + 1].clone());
                i += 2;
            } else if args[i] == "--nexrad-only" {
                nexrad_only = true;
                i += 1;
            } else if args[i] == "--silesia-only" {
                silesia_only = true;
                i += 1;
            } else if !args[i].starts_with("--") && json_path.is_none() {
                json_path = Some(PathBuf::from(&args[i]));
                i += 1;
            } else {
                i += 1;
            }
        }

        Self {
            iterations,
            json_path,
            file_filter,
            nexrad_only,
            silesia_only,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = CliConfig::parse_from_args();
    let comp_dir = Path::new("data/compressed");
    let ref_dir = Path::new("data/reference");

    println!("Loading test datasets into memory & pre-flight validating...");
    let mut silesia_items = load_silesia(ref_dir, comp_dir, SILESIA_FILES)?;
    let mut nexrad_items = load_nexrad(ref_dir, comp_dir)?;

    if let Some(ref filter) = config.file_filter {
        let f_clean = filter.replace("silesia_", "");
        silesia_items.retain(|it| it.name == *filter || it.name.replace("silesia_", "") == f_clean);
        nexrad_items.clear();
    } else if config.nexrad_only {
        silesia_items.clear();
    } else if config.silesia_only {
        nexrad_items.clear();
    }

    println!(
        "✓ Loaded {} Silesia files and {} NEXRAD volume archives.\n",
        silesia_items.len(),
        nexrad_items.len()
    );

    // Pre-allocate maximum reusable working buffers outside all timers (0 allocations inside timed loops)
    let max_silesia_uncomp = silesia_items.iter().map(|it| it.uncompressed.len()).max().unwrap_or(64 * 1024 * 1024);
    let max_nexrad_uncomp = nexrad_items.iter().map(|it| it.uncompressed.len()).max().unwrap_or(64 * 1024 * 1024);
    let max_uncomp = std::cmp::max(max_silesia_uncomp, max_nexrad_uncomp);

    let mut decomp_work_buf = vec![0u8; max_uncomp + 1024 * 1024];
    let mut comp_work_buf = vec![0u8; max_uncomp + (max_uncomp / 100) + 1200];

    let mut report = BenchmarkSuiteReport {
        nexrad: DatasetAggregateResult::default(),
        silesia_aggregate: DatasetAggregateResult::default(),
        silesia_files: Vec::new(),
    };

    // 1. Benchmark NEXRAD Radar Dataset
    if !nexrad_items.is_empty() {
        println!("=== 1. NEXRAD Radar Dataset ({} Iterations + {} Warmup) ===", config.iterations, WARMUP_ITERATIONS);
        let (decomp_stats, comp_stats, uncomp_bytes, comp_bytes) =
            benchmark_nexrad(&nexrad_items, config.iterations, &mut decomp_work_buf, &mut comp_work_buf);

        let mb_per_iter = uncomp_bytes as f64 / 1_000_000.0;
        println!(
            "NEXRAD Decompression: {:.2} MB/pass | Median: {:.2} MB/s (±{:.1}%) [Min: {:.2}, Max: {:.2}]",
            mb_per_iter, decomp_stats.median, decomp_stats.rsd_pct, decomp_stats.min, decomp_stats.max
        );
        println!(
            "NEXRAD Compression:   {:.2} MB/pass | Median: {:.2} MB/s (±{:.1}%) [Min: {:.2}, Max: {:.2}]",
            mb_per_iter, comp_stats.median, comp_stats.rsd_pct, comp_stats.min, comp_stats.max
        );

        report.nexrad = DatasetAggregateResult {
            uncomp_bytes,
            comp_bytes,
            decomp_mb_s: decomp_stats.median,
            decomp_rsd: decomp_stats.rsd_pct,
            comp_mb_s: comp_stats.median,
            comp_rsd: comp_stats.rsd_pct,
        };
    }

    // 2. Benchmark Silesia Corpus Dataset
    if !silesia_items.is_empty() {
        println!("\n=== 2. Silesia Corpus Dataset ({} Iterations + {} Warmup) ===", config.iterations, WARMUP_ITERATIONS);
        println!(
            "{:<18} | {:<16} | {:>10} | {:>8} | {:>16} | {:>16}",
            "File", "Type", "Uncomp MB", "Ratio", "Decomp Median", "Compress Median"
        );
        println!("{:-<18}-|-{:-<16}-|-{:->10}-|-{:->8}-|-{:->16}-|-{:->16}", "", "", "", "", "", "");

        let mut silesia_uncomp_total = 0usize;
        let mut silesia_comp_total = 0usize;
        let mut silesia_agg_decomp_times = vec![0.0f64; config.iterations];
        let mut silesia_agg_comp_times = vec![0.0f64; config.iterations];

        for item in &silesia_items {
            silesia_uncomp_total += item.uncompressed.len();
            silesia_comp_total += item.compressed.len();
            let item_mb = item.uncompressed.len() as f64 / 1_000_000.0;

            let (decomp_stats, comp_stats, decomp_times, comp_times) =
                benchmark_single_file(item, config.iterations, &mut decomp_work_buf, &mut comp_work_buf);

            for (idx, &t) in decomp_times.iter().enumerate() {
                silesia_agg_decomp_times[idx] += t;
            }
            for (idx, &t) in comp_times.iter().enumerate() {
                silesia_agg_comp_times[idx] += t;
            }

            let ratio = (item.compressed.len() as f64 / item.uncompressed.len() as f64) * 100.0;
            println!(
                "{:<18} | {:<16} | {:>9.2}M | {:>7.2}% | {:>9.2} M/s (±{:.1}%) | {:>9.2} M/s (±{:.1}%)",
                item.name, item.category, item_mb, ratio,
                decomp_stats.median, decomp_stats.rsd_pct,
                comp_stats.median, comp_stats.rsd_pct
            );

            report.silesia_files.push(FileBenchmarkResult {
                name: item.name.clone(),
                category: item.category,
                uncomp_bytes: item.uncompressed.len(),
                comp_bytes: item.compressed.len(),
                decomp_mb_s: decomp_stats.median,
                decomp_rsd: decomp_stats.rsd_pct,
                comp_mb_s: comp_stats.median,
                comp_rsd: comp_stats.rsd_pct,
            });
        }

        let silesia_total_mb = silesia_uncomp_total as f64 / 1_000_000.0;
        let silesia_agg_decomp_tp: Vec<f64> = silesia_agg_decomp_times.iter().map(|t| silesia_total_mb / t).collect();
        let silesia_agg_comp_tp: Vec<f64> = silesia_agg_comp_times.iter().map(|t| silesia_total_mb / t).collect();

        let silesia_agg_decomp_stats = compute_stats(silesia_agg_decomp_tp);
        let silesia_agg_comp_stats = compute_stats(silesia_agg_comp_tp);

        if silesia_items.len() > 1 {
            println!(
                "\nSilesia Aggregate Decompression: Median {:.2} MB/s (±{:.1}%)",
                silesia_agg_decomp_stats.median, silesia_agg_decomp_stats.rsd_pct
            );
            println!(
                "Silesia Aggregate Compression:   Median {:.2} MB/s (±{:.1}%)",
                silesia_agg_comp_stats.median, silesia_agg_comp_stats.rsd_pct
            );
        }

        report.silesia_aggregate = DatasetAggregateResult {
            uncomp_bytes: silesia_uncomp_total,
            comp_bytes: silesia_comp_total,
            decomp_mb_s: silesia_agg_decomp_stats.median,
            decomp_rsd: silesia_agg_decomp_stats.rsd_pct,
            comp_mb_s: silesia_agg_comp_stats.median,
            comp_rsd: silesia_agg_comp_stats.rsd_pct,
        };
    }

    if let Some(ref path) = config.json_path {
        report.save_json(path)?;
        println!("\nSaved machine-readable benchmark JSON to {}", path.display());
    }

    Ok(())
}
