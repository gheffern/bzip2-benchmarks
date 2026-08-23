//! Low-level bzip2 decompression & compression routines and zero-allocation execution harness.

use std::time::Instant;
use serde::{Deserialize, Serialize};
use crate::dataset::DatasetItem;
use crate::stats::{compute_stats, Stats};

pub const WARMUP_ITERATIONS: usize = 5;

/// Pin current thread/process to a specific physical CPU core.
pub fn pin_to_core(core_id: usize) {
    #[cfg(target_os = "linux")]
    unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        libc::CPU_SET(core_id, &mut set);
        let ret = libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set);
        if ret != 0 {
            eprintln!("Warning: sched_setaffinity failed with code {}", ret);
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = core_id;
    }
}

/// IPC worker command protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum WorkerCommand {
    Warmup {
        target: String,
        op: String,
        iterations: usize,
    },
    RunIteration {
        target: String,
        op: String,
    },
    Exit,
}

/// IPC worker response protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum WorkerResponse {
    Ready {
        silesia_count: usize,
        nexrad_count: usize,
    },
    WarmupDone,
    IterationSuccess {
        elapsed_secs: f64,
        uncomp_bytes: usize,
        comp_bytes: usize,
    },
    Error {
        message: String,
    },
}

/// Decompress a single bzip2 stream into a caller-provided destination buffer.
pub fn decompress_bz2_single_into(input: &[u8], out: &mut [u8]) -> Result<usize, String> {
    let mut stream: libbz2_rs_sys::bz_stream = unsafe { std::mem::zeroed() };
    let res = unsafe { libbz2_rs_sys::BZ2_bzDecompressInit(&mut stream, 0, 0) };
    if res != libbz2_rs_sys::BZ_OK {
        return Err(format!("BZ2_bzDecompressInit failed code {}", res));
    }

    stream.next_in = input.as_ptr() as *mut libc::c_char;
    stream.avail_in = input.len() as libc::c_uint;
    stream.next_out = out.as_mut_ptr() as *mut libc::c_char;
    stream.avail_out = out.len() as libc::c_uint;

    let ret = unsafe { libbz2_rs_sys::BZ2_bzDecompress(&mut stream) };
    unsafe { libbz2_rs_sys::BZ2_bzDecompressEnd(&mut stream) };

    if ret == libbz2_rs_sys::BZ_STREAM_END {
        Ok(out.len() - stream.avail_out as usize)
    } else {
        Err(format!("BZ2_bzDecompress error {}", ret))
    }
}

/// Decompress concatenated/multistream bzip2 files (e.g. NOAA NEXRAD volume archives).
pub fn decompress_bz2_multistream_into(input: &[u8], out: &mut [u8]) -> Result<usize, String> {
    let mut in_pos = 0;
    let mut out_pos = 0;
    let mut is_first_stream = true;

    while in_pos < input.len() {
        if !is_first_stream {
            if in_pos + 4 > input.len() {
                break;
            }
            let header = i32::from_be_bytes(input[in_pos..in_pos + 4].try_into().unwrap());
            if header < 0 {
                break;
            }
            in_pos += 4;
        }

        if in_pos + 3 > input.len() || &input[in_pos..in_pos + 3] != b"BZh" {
            break;
        }

        let mut stream: libbz2_rs_sys::bz_stream = unsafe { std::mem::zeroed() };
        let res = unsafe { libbz2_rs_sys::BZ2_bzDecompressInit(&mut stream, 0, 0) };
        if res != libbz2_rs_sys::BZ_OK {
            return Err(format!("BZ2_bzDecompressInit failed code {}", res));
        }

        let chunk_in = &input[in_pos..];
        let chunk_out = &mut out[out_pos..];
        stream.next_in = chunk_in.as_ptr() as *mut libc::c_char;
        stream.avail_in = chunk_in.len() as libc::c_uint;
        stream.next_out = chunk_out.as_mut_ptr() as *mut libc::c_char;
        stream.avail_out = chunk_out.len() as libc::c_uint;

        let ret = unsafe { libbz2_rs_sys::BZ2_bzDecompress(&mut stream) };
        unsafe { libbz2_rs_sys::BZ2_bzDecompressEnd(&mut stream) };

        let consumed = chunk_in.len() - stream.avail_in as usize;
        let produced = chunk_out.len() - stream.avail_out as usize;
        in_pos += consumed;
        out_pos += produced;

        if ret != libbz2_rs_sys::BZ_STREAM_END && ret != libbz2_rs_sys::BZ_OK {
            return Err(format!("BZ2_bzDecompress returned error code {}", ret));
        }

        is_first_stream = false;
    }

    Ok(out_pos)
}

/// Compress an in-memory byte buffer using bzip2 block-level compression (level 9).
pub fn compress_bz2_into(input: &[u8], output: &mut [u8]) -> Result<usize, String> {
    let mut dest_len = output.len() as u32;
    let ret = unsafe {
        libbz2_rs_sys::BZ2_bzBuffToBuffCompress(
            output.as_mut_ptr() as *mut libc::c_char,
            &mut dest_len,
            input.as_ptr() as *mut libc::c_char,
            input.len() as u32,
            9,
            0,
            30,
        )
    };
    if ret != 0 {
        return Err(format!("BZ2_bzBuffToBuffCompress failed: {}", ret));
    }
    Ok(dest_len as usize)
}

/// Single-pass decompression for a single dataset item.
pub fn run_decomp_single(item: &DatasetItem, decomp_work_buf: &mut [u8]) -> Result<f64, String> {
    let t0 = Instant::now();
    let len = decompress_bz2_single_into(&item.compressed, decomp_work_buf)?;
    std::hint::black_box(&decomp_work_buf[..len]);
    Ok(t0.elapsed().as_secs_f64())
}

/// Single-pass compression for a single dataset item.
pub fn run_comp_single(item: &DatasetItem, comp_work_buf: &mut [u8]) -> Result<f64, String> {
    let t0 = Instant::now();
    let len = compress_bz2_into(&item.uncompressed, comp_work_buf)?;
    std::hint::black_box(&comp_work_buf[..len]);
    Ok(t0.elapsed().as_secs_f64())
}

/// Single-pass decompression across all NEXRAD multistream archives.
pub fn run_decomp_nexrad(items: &[DatasetItem], decomp_work_buf: &mut [u8]) -> Result<f64, String> {
    let t0 = Instant::now();
    for item in items {
        let len = decompress_bz2_multistream_into(&item.compressed, decomp_work_buf)?;
        std::hint::black_box(&decomp_work_buf[..len]);
    }
    Ok(t0.elapsed().as_secs_f64())
}

/// Single-pass compression across all NEXRAD uncompressed files.
pub fn run_comp_nexrad(items: &[DatasetItem], comp_work_buf: &mut [u8]) -> Result<f64, String> {
    let t0 = Instant::now();
    for item in items {
        let len = compress_bz2_into(&item.uncompressed, comp_work_buf)?;
        std::hint::black_box(&comp_work_buf[..len]);
    }
    Ok(t0.elapsed().as_secs_f64())
}

/// Standalone batch benchmark on NEXRAD radar archives.
pub fn benchmark_nexrad(
    items: &[DatasetItem],
    iterations: usize,
    decomp_work_buf: &mut [u8],
    comp_work_buf: &mut [u8],
) -> (Stats, Stats, usize, usize) {
    let mut uncomp_bytes = 0usize;
    let mut comp_bytes = 0usize;
    for item in items {
        uncomp_bytes += item.uncompressed.len();
        comp_bytes += item.compressed.len();
    }

    for _ in 0..WARMUP_ITERATIONS {
        let _ = run_decomp_nexrad(items, decomp_work_buf);
    }
    let mut decomp_samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let elapsed = run_decomp_nexrad(items, decomp_work_buf).unwrap();
        decomp_samples.push((uncomp_bytes as f64 / 1_000_000.0) / elapsed);
    }
    let decomp_stats = compute_stats(decomp_samples);

    for _ in 0..WARMUP_ITERATIONS {
        let _ = run_comp_nexrad(items, comp_work_buf);
    }
    let mut comp_samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let elapsed = run_comp_nexrad(items, comp_work_buf).unwrap();
        comp_samples.push((uncomp_bytes as f64 / 1_000_000.0) / elapsed);
    }
    let comp_stats = compute_stats(comp_samples);

    (decomp_stats, comp_stats, uncomp_bytes, comp_bytes)
}

/// Standalone batch benchmark on a single file.
pub fn benchmark_single_file(
    item: &DatasetItem,
    iterations: usize,
    decomp_work_buf: &mut [u8],
    comp_work_buf: &mut [u8],
) -> (Stats, Stats, Vec<f64>, Vec<f64>) {
    let item_mb = item.uncompressed.len() as f64 / 1_000_000.0;

    for _ in 0..WARMUP_ITERATIONS {
        let _ = run_decomp_single(item, decomp_work_buf);
    }
    let mut decomp_samples = Vec::with_capacity(iterations);
    let mut decomp_times = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let elapsed = run_decomp_single(item, decomp_work_buf).unwrap();
        decomp_times.push(elapsed);
        decomp_samples.push(item_mb / elapsed);
    }
    let decomp_stats = compute_stats(decomp_samples);

    for _ in 0..WARMUP_ITERATIONS {
        let _ = run_comp_single(item, comp_work_buf);
    }
    let mut comp_samples = Vec::with_capacity(iterations);
    let mut comp_times = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let elapsed = run_comp_single(item, comp_work_buf).unwrap();
        comp_times.push(elapsed);
        comp_samples.push(item_mb / elapsed);
    }
    let comp_stats = compute_stats(comp_samples);

    (decomp_stats, comp_stats, decomp_times, comp_times)
}
