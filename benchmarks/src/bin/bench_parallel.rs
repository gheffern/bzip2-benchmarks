use std::fs;
use std::path::Path;
use std::time::Instant;
use std::thread;

fn decompress_file(compressed_path: &Path) -> Result<(usize, u128), Box<dyn std::error::Error>> {
    let compressed = fs::read(compressed_path)?;
    let start = Instant::now();
    
    let mut decompressed = Vec::with_capacity(compressed.len() * 5);
    let mut in_pos = 0;
    let mut is_first_stream = true;

    while in_pos < compressed.len() {
        // For subsequent streams in multi-stream archives (e.g. NEXRAD Level 2),
        // skip the 4-byte big-endian length header preceding each bzip2 stream.
        if !is_first_stream {
            if in_pos + 4 > compressed.len() {
                break;
            }
            let header = i32::from_be_bytes(compressed[in_pos..in_pos+4].try_into().unwrap());
            if header < 0 {
                break; // Negative header indicates end-of-volume marker
            }
            in_pos += 4;
        }

        // Verify BZh magic before attempting decompression
        if in_pos + 3 > compressed.len() || &compressed[in_pos..in_pos+3] != b"BZh" {
            break;
        }

        let mut strm: libbz2_rs_sys::bz_stream = unsafe { std::mem::zeroed() };
        let init_res = unsafe {
            libbz2_rs_sys::BZ2_bzDecompressInit(
                &mut strm,
                0,
                0,
            )
        };
        if init_res != libbz2_rs_sys::BZ_OK {
            return Err(format!("BZ2_bzDecompressInit failed with code {}", init_res).into());
        }

        let mut buf = [0u8; 65536];
        loop {
            let chunk = &compressed[in_pos..];
            strm.next_in = chunk.as_ptr() as *mut _;
            strm.avail_in = chunk.len() as u32;

            strm.next_out = buf.as_mut_ptr() as *mut _;
            strm.avail_out = buf.len() as u32;

            let res = unsafe { libbz2_rs_sys::BZ2_bzDecompress(&mut strm) };
            let written = buf.len() - strm.avail_out as usize;
            decompressed.extend_from_slice(&buf[..written]);

            let consumed = chunk.len() - strm.avail_in as usize;
            in_pos += consumed;

            if res == libbz2_rs_sys::BZ_STREAM_END {
                break;
            } else if res != libbz2_rs_sys::BZ_OK {
                unsafe { libbz2_rs_sys::BZ2_bzDecompressEnd(&mut strm) };
                return Err(format!("BZ2_bzDecompress failed with code {}", res).into());
            }
        }

        unsafe { libbz2_rs_sys::BZ2_bzDecompressEnd(&mut strm) };
        is_first_stream = false;
    }

    let elapsed = start.elapsed().as_nanos();
    Ok((decompressed.len(), elapsed))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let compressed_dir = Path::new("data/compressed");
    
    let mut nexrad_files = Vec::new();
    for entry in fs::read_dir(compressed_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.file_name().unwrap().to_string_lossy().starts_with("nexrad") && path.extension().map_or(false, |ext| ext == "bz2") {
            nexrad_files.push(path);
        }
    }
    nexrad_files.sort();

    let num_threads = thread::available_parallelism().map_or(4, |n| n.get());
    println!("=== Task 11: Multi-Core Parallel Decompression ({} Cores Available) ===", num_threads);

    let start = Instant::now();
    let mut total_uncompressed_bytes = 0usize;

    // Parallel file decompression using thread pool
    let chunk_size = (nexrad_files.len() + num_threads - 1) / num_threads;
    let file_chunks: Vec<_> = nexrad_files.chunks(chunk_size).map(|c| c.to_vec()).collect();

    let handles: Vec<_> = file_chunks
        .into_iter()
        .map(|chunk| {
            thread::spawn(move || {
                let mut chunk_bytes = 0usize;
                for _iter in 0..20 {
                    for path in &chunk {
                        if let Ok((bytes, _)) = decompress_file(path) {
                            chunk_bytes += bytes;
                        }
                    }
                }
                chunk_bytes
            })
        })
        .collect();

    for h in handles {
        total_uncompressed_bytes += h.join().unwrap();
    }

    let elapsed_sec = start.elapsed().as_secs_f64();
    let mb = total_uncompressed_bytes as f64 / 1_048_576.0;
    let mb_per_sec = mb / elapsed_sec;

    let output_str = format!(
        "================ PARALLEL BENCHMARK METRICS ({}) ================\n\
        NEXRAD Radar Dataset (20 Iterations across {} Cores):\n\
        - Uncompressed Data: {:.2} MB\n\
        - Total Wall Time:   {:.4} s\n\
        - Parallel Speed:    {:.2} MB/s 🚀\n\
        ============================================================\n",
        num_threads, num_threads, mb, elapsed_sec, mb_per_sec
    );

    println!("{}", output_str);
    fs::write("baseline_metrics_parallel.txt", output_str)?;
    println!("Parallel metrics saved to baseline_metrics_parallel.txt");

    Ok(())
}
