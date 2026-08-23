//! High-precision, zero-allocation bzip2 performance verification benchmark suite.

pub mod dataset;
pub mod engine;
pub mod report;
pub mod stats;

pub use dataset::{load_nexrad, load_silesia, DatasetItem, FileMeta, SILESIA_FILES};
pub use engine::{benchmark_nexrad, benchmark_single_file, WARMUP_ITERATIONS};
pub use report::{
    render_ab_markdown_report, safe_delta, BenchmarkSuiteReport, DatasetAggregateResult,
    EnvMetadata, FileBenchmarkResult,
};
pub use stats::{compute_stats, Stats};
