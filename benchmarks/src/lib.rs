//! High-precision, zero-allocation bzip2 performance verification benchmark suite.

pub mod dataset;
pub mod engine;
pub mod report;
pub mod stats;

pub use dataset::{load_nexrad, load_silesia, DataCategory, DatasetItem, FileMeta, SilesiaFileId, SILESIA_FILES};
pub use engine::{
    benchmark_nexrad, benchmark_single_file, compress_bz2_into, decompress_bz2_multistream_into,
    decompress_bz2_single_into, pin_to_core, run_comp_nexrad, run_comp_single,
    run_decomp_nexrad, run_decomp_single, WorkerCommand, WorkerResponse, WARMUP_ITERATIONS,
};
pub use report::{
    render_ab_markdown_report, safe_delta, BenchmarkOp, BenchmarkSuiteReport,
    DatasetAggregateResult, EnvMetadata, FileBenchmarkResult,
};
pub use stats::{compute_stats, Stats};
