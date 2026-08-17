use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::ffi::c_char;
use libbz2_rs_sys::BZ2_bzBuffToBuffCompress;

fn compress_bz2(input: &[u8]) -> Result<Vec<u8>, String> {
    let dest_capacity = input.len() + (input.len() / 100) + 600;
    let mut output = vec![0u8; dest_capacity];
    let mut dest_len = dest_capacity as u32;

    let ret = unsafe {
        BZ2_bzBuffToBuffCompress(
            output.as_mut_ptr() as *mut c_char,
            &mut dest_len,
            input.as_ptr() as *mut c_char,
            input.len() as u32,
            9, // blockSize100k = 9 (900 KB)
            0, // verbosity
            30, // workFactor
        )
    };

    if ret != 0 {
        return Err(format!("BZ2_bzBuffToBuffCompress failed: {}", ret));
    }

    output.truncate(dest_len as usize);
    Ok(output)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let reference_dir = Path::new("data/reference");

    let mut nexrad_files = Vec::new();
    let mut silesia_files = Vec::new();

    for entry in fs::read_dir(reference_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let fname = path.file_name().unwrap().to_string_lossy().to_string();
            if fname.starts_with("nexrad") {
                nexrad_files.push(path);
            } else if fname.starts_with("silesia_") {
                silesia_files.push(path);
            }
        }
    }
    nexrad_files.sort();
    silesia_files.sort();

    println!("Loaded {} NEXRAD reference files and {} Silesia reference files for compression benchmarking.\n", nexrad_files.len(), silesia_files.len());

    let (nexrad_speed, nexrad_uncomp_mb, nexrad_comp_mb, nexrad_time) = bench_compression_dataset("NEXRAD Radar", &nexrad_files, 20)?;
    let (silesia_speed, silesia_uncomp_mb, silesia_comp_mb, silesia_time) = bench_compression_dataset("Silesia Corpus", &silesia_files, 20)?;

    let total_uncomp_mb = nexrad_uncomp_mb + silesia_uncomp_mb;
    let total_comp_mb = nexrad_comp_mb + silesia_comp_mb;
    let total_time = nexrad_time + silesia_time;
    let overall_speed = (total_uncomp_mb * 20.0) / total_time;
    let overall_ratio = (total_comp_mb / total_uncomp_mb) * 100.0;

    let nexrad_ratio = (nexrad_comp_mb / nexrad_uncomp_mb) * 100.0;
    let silesia_ratio = (silesia_comp_mb / silesia_uncomp_mb) * 100.0;

    let output_str = format!(
        "================ BASELINE COMPRESSION BENCHMARK METRICS ================\n\
        NEXRAD Radar Dataset:\n\
        - Uncompressed Size: {:.2} MB\n\
        - Compressed Size:   {:.2} MB (Compression Ratio: {:.2}%)\n\
        - Total Time:        {:.4} s\n\
        - Compression Speed: {:.2} MB/s\n\n\
        Silesia Corpus Dataset:\n\
        - Uncompressed Size: {:.2} MB\n\
        - Compressed Size:   {:.2} MB (Compression Ratio: {:.2}%)\n\
        - Total Time:        {:.4} s\n\
        - Compression Speed: {:.2} MB/s\n\n\
        Overall Combined Suite:\n\
        - Total Uncompressed: {:.2} MB\n\
        - Total Compressed:   {:.2} MB (Overall Ratio: {:.2}%)\n\
        - Total Time:         {:.4} s\n\
        - Overall Speed:      {:.2} MB/s\n\
        =======================================================================\n",
        nexrad_uncomp_mb, nexrad_comp_mb, nexrad_ratio, nexrad_time, nexrad_speed,
        silesia_uncomp_mb, silesia_comp_mb, silesia_ratio, silesia_time, silesia_speed,
        total_uncomp_mb, total_comp_mb, overall_ratio, total_time, overall_speed
    );

    println!("\n{}", output_str);

    let output_file = std::env::args().nth(1).unwrap_or_else(|| "baseline_compress_metrics.txt".to_string());
    fs::write(&output_file, &output_str)?;
    println!("Baseline compression metrics saved to {}", output_file);

    Ok(())
}

fn bench_compression_dataset(_name: &str, files: &[PathBuf], iterations: usize) -> Result<(f64, f64, f64, f64), Box<dyn std::error::Error>> {
    let mut inputs = Vec::new();
    let mut total_uncomp_bytes: usize = 0;
    let mut total_comp_bytes: usize = 0;

    for path in files {
        let uncompressed = fs::read(path)?;
        total_uncomp_bytes += uncompressed.len();
        let compressed = compress_bz2(&uncompressed)?;
        total_comp_bytes += compressed.len();
        inputs.push(uncompressed);
    }

    // Warmup
    for input in &inputs {
        let res = compress_bz2(input)?;
        std::hint::black_box(res);
    }

    let start = Instant::now();
    for _ in 0..iterations {
        for input in &inputs {
            let res = compress_bz2(input)?;
            std::hint::black_box(res);
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    let uncomp_mb = (total_uncomp_bytes as f64) / (1024.0 * 1024.0);
    let comp_mb = (total_comp_bytes as f64) / (1024.0 * 1024.0);

    let total_decompressed_mb = uncomp_mb * (iterations as f64);
    let mb_s = total_decompressed_mb / elapsed;

    Ok((mb_s, uncomp_mb, comp_mb, elapsed))
}
