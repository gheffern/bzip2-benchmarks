use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let compressed_dir = Path::new("data/compressed");
    let reference_dir = Path::new("data/reference");

    // Collect NEXRAD files
    let mut nexrad_files = Vec::new();
    for entry in fs::read_dir(compressed_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.file_name().unwrap().to_string_lossy().starts_with("nexrad") && path.extension().map_or(false, |ext| ext == "bz2") {
            nexrad_files.push(path);
        }
    }
    nexrad_files.sort();

    // Collect Silesia files
    let mut silesia_files = Vec::new();
    for entry in fs::read_dir(compressed_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.file_name().unwrap().to_string_lossy().starts_with("silesia_") && path.extension().map_or(false, |ext| ext == "bz2") {
            silesia_files.push(path);
        }
    }
    silesia_files.sort();

    println!("Loaded {} NEXRAD files and {} Silesia files.", nexrad_files.len(), silesia_files.len());

    let (nexrad_throughput, nexrad_uncompressed_mb, nexrad_time_sec) = bench_dataset("NEXRAD Radar", &nexrad_files, reference_dir)?;
    let (silesia_throughput, silesia_uncompressed_mb, silesia_time_sec) = bench_dataset("Silesia Corpus", &silesia_files, reference_dir)?;

    let total_mb = nexrad_uncompressed_mb + silesia_uncompressed_mb;
    let total_time = nexrad_time_sec + silesia_time_sec;
    let overall_throughput = total_mb / total_time;

    let output_str = format!(
        "================ BASELINE BENCHMARK METRICS ================\n\
        NEXRAD Radar Dataset:\n\
        - Uncompressed Data: {:.2} MB\n\
        - Total Time:        {:.4} s\n\
        - Decompression Speed: {:.2} MB/s\n\n\
        Silesia Corpus Dataset:\n\
        - Uncompressed Data: {:.2} MB\n\
        - Total Time:        {:.4} s\n\
        - Decompression Speed: {:.2} MB/s\n\n\
        Overall Combined:\n\
        - Total Uncompressed: {:.2} MB\n\
        - Total Time:         {:.4} s\n\
        - Overall Speed:      {:.2} MB/s\n\
        ============================================================\n",
        nexrad_uncompressed_mb, nexrad_time_sec, nexrad_throughput,
        silesia_uncompressed_mb, silesia_time_sec, silesia_throughput,
        total_mb, total_time, overall_throughput
    );

    println!("\n{}", output_str);

    let output_file = std::env::args().nth(1).unwrap_or_else(|| "baseline_metrics.txt".to_string());
    fs::write(&output_file, &output_str)?;
    println!("Baseline metrics saved to {}", output_file);

    Ok(())
}

fn bench_dataset(name: &str, files: &[PathBuf], ref_dir: &Path) -> Result<(f64, f64, f64), Box<dyn std::error::Error>> {
    let mut inputs = Vec::new();
    let mut total_uncompressed_bytes: usize = 0;

    for path in files {
        let compressed = fs::read(path)?;
        let fname = path.file_stem().unwrap().to_string_lossy().to_string();
        let ref_path = ref_dir.join(&fname);
        let ref_bytes = fs::read(&ref_path)?;
        total_uncompressed_bytes += ref_bytes.len();
        inputs.push((compressed, ref_bytes));
    }

    // Warmup run
    for (compressed, expected) in &inputs {
        let decompressed = decompress_bz2(compressed)?;
        assert_eq!(decompressed.len(), expected.len(), "Warmup length mismatch on {:?}", name);
        assert_eq!(decompressed, *expected, "Warmup content mismatch on {:?}", name);
    }

    let iterations = 20;
    let start = Instant::now();

    for _ in 0..iterations {
        for (compressed, _) in &inputs {
            let decompressed = decompress_bz2(compressed)?;
            std::hint::black_box(decompressed);
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    let total_bytes_decompressed = (total_uncompressed_bytes * iterations) as f64;
    let mb = total_bytes_decompressed / 1_000_000.0;
    let throughput = mb / elapsed;

    println!("{:<20}: Decompressed {:.2} MB in {:.4} s ({:.2} MB/s across {} iterations)", name, mb, elapsed, throughput, iterations);

    Ok((throughput, mb, elapsed))
}

fn decompress_bz2(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut out = Vec::with_capacity(input.len() * 4);
    let mut in_pos = 0;
    let mut is_first_stream = true;

    while in_pos < input.len() {
        // For subsequent streams in multi-stream archives (e.g. NEXRAD Level 2),
        // skip the 4-byte big-endian length header preceding each bzip2 stream.
        if !is_first_stream {
            if in_pos + 4 > input.len() {
                break;
            }
            let header = i32::from_be_bytes(input[in_pos..in_pos+4].try_into().unwrap());
            if header < 0 {
                break; // Negative header indicates end-of-volume marker
            }
            in_pos += 4;
        }

        // Verify BZh magic before attempting decompression
        if in_pos + 3 > input.len() || &input[in_pos..in_pos+3] != b"BZh" {
            break;
        }

        let mut stream: libbz2_rs_sys::bz_stream = unsafe { std::mem::zeroed() };
        let res = unsafe {
            libbz2_rs_sys::BZ2_bzDecompressInit(&mut stream, 0, 0)
        };
        if res != libbz2_rs_sys::BZ_OK {
            return Err(format!("BZ2_bzDecompressInit failed code {}", res).into());
        }

        loop {
            let chunk_in = &input[in_pos..];
            stream.next_in = chunk_in.as_ptr() as *mut libc::c_char;
            stream.avail_in = chunk_in.len() as libc::c_uint;

            let mut buf = [0u8; 64 * 1024];
            stream.next_out = buf.as_mut_ptr() as *mut libc::c_char;
            stream.avail_out = buf.len() as libc::c_uint;

            let ret = unsafe { libbz2_rs_sys::BZ2_bzDecompress(&mut stream) };
            let produced = buf.len() - stream.avail_out as usize;
            out.extend_from_slice(&buf[..produced]);

            let consumed = chunk_in.len() - stream.avail_in as usize;
            in_pos += consumed;

            if ret == libbz2_rs_sys::BZ_STREAM_END {
                break;
            } else if ret != libbz2_rs_sys::BZ_OK {
                unsafe { libbz2_rs_sys::BZ2_bzDecompressEnd(&mut stream) };
                return Err(format!("BZ2_bzDecompress returned error code {}", ret).into());
            }
        }

        unsafe { libbz2_rs_sys::BZ2_bzDecompressEnd(&mut stream) };
        is_first_stream = false;
    }

    Ok(out)
}
