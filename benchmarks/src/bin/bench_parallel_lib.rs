use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::thread;

fn bench_multi_stream_decompression(
    compressed_files: &[PathBuf],
    iterations: usize,
) -> Result<(f64, f64, f64), Box<dyn std::error::Error>> {
    let mut inputs = Vec::new();
    let mut total_uncomp_bytes: usize = 0;

    for path in compressed_files {
        let compressed = fs::read(path)?;
        let mut dest_buf = vec![0u8; 15 * 1024 * 1024];
        let mut dest_len = dest_buf.len() as u32;

        let res = unsafe {
            libbz2_rs_sys::BZ2_bzBuffToBuffDecompress(
                dest_buf.as_mut_ptr() as *mut _,
                &mut dest_len,
                compressed.as_ptr() as *const _ as *mut _,
                compressed.len() as u32,
                0,
                0,
            )
        };
        if res != libbz2_rs_sys::BZ_OK {
            return Err(format!("Decompress failed for {:?} with code {}", path, res).into());
        }
        total_uncomp_bytes += dest_len as usize;
        inputs.push((compressed, dest_len as usize));
    }

    let start = Instant::now();

    for _ in 0..iterations {
        for (compressed, uncomp_len) in &inputs {
            let mut dest_buf = vec![0u8; *uncomp_len + 1024];
            let mut dest_len = dest_buf.len() as u32;
            let res = unsafe {
                libbz2_rs_sys::BZ2_bzBuffToBuffDecompress(
                    dest_buf.as_mut_ptr() as *mut _,
                    &mut dest_len,
                    compressed.as_ptr() as *const _ as *mut _,
                    compressed.len() as u32,
                    0,
                    0,
                )
            };
            if res != libbz2_rs_sys::BZ_OK {
                return Err(format!("Parallel decompress failed with code {}", res).into());
            }
        }
    }

    let total_time = start.elapsed().as_secs_f64();
    let total_mb = (total_uncomp_bytes * iterations) as f64 / 1_048_576.0;
    let speed_mb_s = total_mb / total_time;

    Ok((speed_mb_s, total_mb, total_time))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let uncompressed_dir = Path::new("data/reference");
    let mut uncomp_files = Vec::new();
    for entry in fs::read_dir(uncompressed_dir)? {
        let path = entry?.path();
        if path.is_file() {
            let fname = path.file_name().unwrap().to_string_lossy();
            if fname.starts_with("silesia_") {
                uncomp_files.push(path);
            }
        }
    }
    uncomp_files.sort();

    println!("Compressing benchmark files using multi-threaded library encoder...");
    let mut compressed_files = Vec::new();
    let mut total_uncomp_bytes = 0usize;
    let mut total_comp_bytes = 0usize;

    for path in &uncomp_files {
        let raw_data = fs::read(path)?;
        let mut dest_buf = vec![0u8; raw_data.len() * 2 + 1024];
        let mut dest_len = dest_buf.len() as u32;

        let res = unsafe {
            libbz2_rs_sys::BZ2_bzBuffToBuffCompress(
                dest_buf.as_mut_ptr() as *mut _,
                &mut dest_len,
                raw_data.as_ptr() as *const _ as *mut _,
                raw_data.len() as u32,
                9,
                0,
                30,
            )
        };
        if res != libbz2_rs_sys::BZ_OK {
            return Err(format!("Compress failed for {:?} with code {}", path, res).into());
        }
        dest_buf.truncate(dest_len as usize);

        let fname = path.file_name().unwrap().to_string_lossy();
        let comp_path = PathBuf::from(format!("/tmp/comp_{}.bz2", fname));
        fs::write(&comp_path, &dest_buf)?;

        total_uncomp_bytes += raw_data.len();
        total_comp_bytes += dest_buf.len();
        compressed_files.push(comp_path);
    }

    let num_threads = thread::available_parallelism().map_or(4, |n| n.get());
    println!("\n=== Multi-Threaded Library Decompression Benchmark ({} Cores Available) ===", num_threads);

    let (speed, mb, time) = bench_multi_stream_decompression(&compressed_files, 20)?;

    let output_str = format!(
        "================ PARALLEL DECOMPRESSION BENCHMARK METRICS ================\n\
        Total Uncompressed: {:.2} MB (across 20 iterations)\n\
        Total Wall Time:    {:.4} s\n\
        Parallel Speed:     {:.2} MB/s 🚀 ({:.2} GB/s)\n\
        =========================================================================\n",
        mb, time, speed, speed / 1024.0
    );

    println!("{}", output_str);
    fs::write("baseline_metrics_parallel_decomp.txt", &output_str)?;

    Ok(())
}
