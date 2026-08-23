use std::fs;
use std::path::Path;
use std::time::Instant;

#[derive(Clone, Copy)]
struct FileMeta {
    name: &'static str,
    file_type: &'static str,
}

const SILESIA_FILES: &[FileMeta] = &[
    FileMeta { name: "silesia_dickens", file_type: "Text (ASCII)" },
    FileMeta { name: "silesia_mozilla", file_type: "Tar / Executables" },
    FileMeta { name: "silesia_mr", file_type: "Medical (MRI)" },
    FileMeta { name: "silesia_nci", file_type: "Text (Chem DB)" },
    FileMeta { name: "silesia_ooffice", file_type: "x86 Executable" },
    FileMeta { name: "silesia_osdb", file_type: "DB Binary" },
    FileMeta { name: "silesia_reymont", file_type: "PDF Document" },
    FileMeta { name: "silesia_samba", file_type: "Tar / C Source" },
    FileMeta { name: "silesia_sao", file_type: "Binary Catalog" },
    FileMeta { name: "silesia_webster", file_type: "Text (Dictionary)" },
    FileMeta { name: "silesia_xml", file_type: "XML Markup" },
    FileMeta { name: "silesia_x-ray", file_type: "Medical (X-Ray)" },
];

const WARMUP_ITERATIONS: usize = 3;

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Stats {
    median: f64,
    mean: f64,
    min: f64,
    max: f64,
    rsd_pct: f64,
}

fn compute_stats(mut samples: Vec<f64>) -> Stats {
    if samples.is_empty() {
        return Stats { median: 0.0, mean: 0.0, min: 0.0, max: 0.0, rsd_pct: 0.0 };
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = samples.len();
    let median = if n % 2 == 0 {
        (samples[n / 2 - 1] + samples[n / 2]) / 2.0
    } else {
        samples[n / 2]
    };
    let min = samples[0];
    let max = samples[n - 1];
    let mean = samples.iter().sum::<f64>() / n as f64;
    let variance = samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (if n > 1 { n - 1 } else { 1 }) as f64;
    let std_dev = variance.sqrt();
    let rsd_pct = if mean > 0.0 { (std_dev / mean) * 100.0 } else { 0.0 };
    Stats { median, mean, min, max, rsd_pct }
}

fn decompress_bz2_single_into(input: &[u8], out: &mut [u8]) -> Result<usize, String> {
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

fn decompress_bz2_multistream_into(input: &[u8], out: &mut [u8]) -> Result<usize, String> {
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

fn compress_bz2_into(input: &[u8], output: &mut [u8]) -> Result<usize, String> {
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

struct DatasetItem {
    name: String,
    file_type: String,
    uncompressed: Vec<u8>,
    compressed: Vec<u8>,
}

fn load_dataset(ref_dir: &Path, comp_dir: &Path, files: &[FileMeta]) -> Result<Vec<DatasetItem>, Box<dyn std::error::Error>> {
    let mut items = Vec::new();
    let mut test_buf = vec![0u8; 64 * 1024 * 1024];

    for f in files {
        let ref_path = ref_dir.join(f.name);
        let comp_path = comp_dir.join(format!("{}.bz2", f.name));
        let uncompressed = fs::read(&ref_path)?;
        let compressed = fs::read(&comp_path)?;

        let decomp_len = decompress_bz2_single_into(&compressed, &mut test_buf)
            .map_err(|e| format!("Validation failure on {}: {}", f.name, e))?;
        assert_eq!(decomp_len, uncompressed.len(), "Length mismatch on {}", f.name);
        assert_eq!(&test_buf[..decomp_len], &uncompressed[..], "Content mismatch on {}", f.name);

        items.push(DatasetItem {
            name: f.name.to_string(),
            file_type: f.file_type.to_string(),
            uncompressed,
            compressed,
        });
    }
    Ok(items)
}

fn load_nexrad(ref_dir: &Path, comp_dir: &Path) -> Result<Vec<DatasetItem>, Box<dyn std::error::Error>> {
    let mut items = Vec::new();
    let mut test_buf = vec![0u8; 64 * 1024 * 1024];

    for i in 1..=30 {
        let name = format!("nexrad{}", i);
        let ref_path = ref_dir.join(&name);
        let comp_path = comp_dir.join(format!("{}.bz2", name));
        let uncompressed = fs::read(&ref_path)?;
        let compressed = fs::read(&comp_path)?;

        let decomp_len = decompress_bz2_multistream_into(&compressed, &mut test_buf)
            .map_err(|e| format!("Validation failure on {}: {}", name, e))?;
        assert_eq!(decomp_len, uncompressed.len(), "Length mismatch on {}", name);
        assert_eq!(&test_buf[..decomp_len], &uncompressed[..], "Content mismatch on {}", name);

        items.push(DatasetItem {
            name,
            file_type: "Radar Binary Archive".to_string(),
            uncompressed,
            compressed,
        });
    }
    Ok(items)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mut iterations = 20;
    let mut json_out: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--iterations" && i + 1 < args.len() {
            iterations = args[i + 1].parse().unwrap_or(20);
            i += 2;
        } else if args[i] == "--json" && i + 1 < args.len() {
            json_out = Some(args[i + 1].clone());
            i += 2;
        } else if !args[i].starts_with("--") && json_out.is_none() {
            json_out = Some(args[i].clone());
            i += 1;
        } else {
            i += 1;
        }
    }

    let comp_dir = Path::new("data/compressed");
    let ref_dir = Path::new("data/reference");

    println!("Loading test datasets into memory & pre-flight validating...");
    let silesia_items = load_dataset(ref_dir, comp_dir, SILESIA_FILES)?;
    let nexrad_items = load_nexrad(ref_dir, comp_dir)?;
    println!("✓ Loaded {} Silesia files and {} NEXRAD volume archives.\n", silesia_items.len(), nexrad_items.len());

    // Pre-allocate maximum reusable working buffers outside all timers (0 allocations inside timed loops)
    let max_silesia_uncomp = silesia_items.iter().map(|it| it.uncompressed.len()).max().unwrap_or(64 * 1024 * 1024);
    let max_nexrad_uncomp = nexrad_items.iter().map(|it| it.uncompressed.len()).max().unwrap_or(64 * 1024 * 1024);
    let max_uncomp = std::cmp::max(max_silesia_uncomp, max_nexrad_uncomp);

    let mut decomp_work_buf = vec![0u8; max_uncomp + 1024 * 1024];
    let mut comp_work_buf = vec![0u8; max_uncomp + (max_uncomp / 100) + 1200];

    // =========================================================================
    // 1. Benchmark NEXRAD Radar Dataset
    // =========================================================================
    println!("=== 1. NEXRAD Radar Dataset ({} Iterations + {} Warmup) ===", iterations, WARMUP_ITERATIONS);
    let mut nexrad_uncomp_bytes = 0usize;
    let mut nexrad_comp_bytes = 0usize;
    for item in &nexrad_items {
        nexrad_uncomp_bytes += item.uncompressed.len();
        nexrad_comp_bytes += item.compressed.len();
    }

    // Warmup decompression (un-timed)
    for _ in 0..WARMUP_ITERATIONS {
        for item in &nexrad_items {
            let len = decompress_bz2_multistream_into(&item.compressed, &mut decomp_work_buf).unwrap();
            std::hint::black_box(&decomp_work_buf[..len]);
        }
    }

    // Timed decompression iterations
    let mut nexrad_decomp_samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let t0 = Instant::now();
        for item in &nexrad_items {
            let len = decompress_bz2_multistream_into(&item.compressed, &mut decomp_work_buf).unwrap();
            std::hint::black_box(&decomp_work_buf[..len]);
        }
        let elapsed = t0.elapsed().as_secs_f64();
        let mb_s = (nexrad_uncomp_bytes as f64 / 1_000_000.0) / elapsed;
        nexrad_decomp_samples.push(mb_s);
    }
    let nexrad_decomp_stats = compute_stats(nexrad_decomp_samples);

    // Warmup compression (un-timed)
    for _ in 0..WARMUP_ITERATIONS {
        for item in &nexrad_items {
            let len = compress_bz2_into(&item.uncompressed, &mut comp_work_buf).unwrap();
            std::hint::black_box(&comp_work_buf[..len]);
        }
    }

    // Timed compression iterations
    let mut nexrad_comp_samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let t0 = Instant::now();
        for item in &nexrad_items {
            let len = compress_bz2_into(&item.uncompressed, &mut comp_work_buf).unwrap();
            std::hint::black_box(&comp_work_buf[..len]);
        }
        let elapsed = t0.elapsed().as_secs_f64();
        let mb_s = (nexrad_uncomp_bytes as f64 / 1_000_000.0) / elapsed;
        nexrad_comp_samples.push(mb_s);
    }
    let nexrad_comp_stats = compute_stats(nexrad_comp_samples);

    let nexrad_mb_per_iter = nexrad_uncomp_bytes as f64 / 1_000_000.0;
    println!("NEXRAD Decompression: {:.2} MB/pass | Median: {:.2} MB/s (±{:.1}%) [Min: {:.2}, Max: {:.2}]",
             nexrad_mb_per_iter, nexrad_decomp_stats.median, nexrad_decomp_stats.rsd_pct, nexrad_decomp_stats.min, nexrad_decomp_stats.max);
    println!("NEXRAD Compression:   {:.2} MB/pass | Median: {:.2} MB/s (±{:.1}%) [Min: {:.2}, Max: {:.2}]",
             nexrad_mb_per_iter, nexrad_comp_stats.median, nexrad_comp_stats.rsd_pct, nexrad_comp_stats.min, nexrad_comp_stats.max);

    // =========================================================================
    // 2. Benchmark Silesia Corpus Dataset
    // =========================================================================
    println!("\n=== 2. Silesia Corpus Dataset ({} Iterations + {} Warmup) ===", iterations, WARMUP_ITERATIONS);
    println!("{:<18} | {:<16} | {:>10} | {:>8} | {:>16} | {:>16}", 
             "File", "Type", "Uncomp MB", "Ratio", "Decomp Median", "Compress Median");
    println!("{:-<18}-|-{:-<16}-|-{:->10}-|-{:->8}-|-{:->16}-|-{:->16}", 
             "", "", "", "", "", "");

    let mut silesia_uncomp_total = 0usize;
    let mut silesia_comp_total = 0usize;
    let mut silesia_json_entries = Vec::new();
    let mut silesia_agg_decomp_samples = vec![0.0f64; iterations];
    let mut silesia_agg_comp_samples = vec![0.0f64; iterations];

    for item in &silesia_items {
        silesia_uncomp_total += item.uncompressed.len();
        silesia_comp_total += item.compressed.len();
        let item_mb = item.uncompressed.len() as f64 / 1_000_000.0;

        // Warmup decompression
        for _ in 0..WARMUP_ITERATIONS {
            let len = decompress_bz2_single_into(&item.compressed, &mut decomp_work_buf).unwrap();
            std::hint::black_box(&decomp_work_buf[..len]);
        }

        // Timed decompression iterations
        let mut file_decomp_samples = Vec::with_capacity(iterations);
        for iter_idx in 0..iterations {
            let t0 = Instant::now();
            let len = decompress_bz2_single_into(&item.compressed, &mut decomp_work_buf).unwrap();
            std::hint::black_box(&decomp_work_buf[..len]);
            let elapsed = t0.elapsed().as_secs_f64();
            file_decomp_samples.push(item_mb / elapsed);
            silesia_agg_decomp_samples[iter_idx] += elapsed;
        }
        let file_decomp_stats = compute_stats(file_decomp_samples);

        // Warmup compression
        for _ in 0..WARMUP_ITERATIONS {
            let len = compress_bz2_into(&item.uncompressed, &mut comp_work_buf).unwrap();
            std::hint::black_box(&comp_work_buf[..len]);
        }

        // Timed compression iterations
        let mut file_comp_samples = Vec::with_capacity(iterations);
        for iter_idx in 0..iterations {
            let t0 = Instant::now();
            let len = compress_bz2_into(&item.uncompressed, &mut comp_work_buf).unwrap();
            std::hint::black_box(&comp_work_buf[..len]);
            let elapsed = t0.elapsed().as_secs_f64();
            file_comp_samples.push(item_mb / elapsed);
            silesia_agg_comp_samples[iter_idx] += elapsed;
        }
        let file_comp_stats = compute_stats(file_comp_samples);

        let ratio = (item.compressed.len() as f64 / item.uncompressed.len() as f64) * 100.0;

        println!("{:<18} | {:<16} | {:>9.2}M | {:>7.2}% | {:>9.2} M/s (±{:.1}%) | {:>9.2} M/s (±{:.1}%)",
                 item.name, item.file_type, item_mb, ratio,
                 file_decomp_stats.median, file_decomp_stats.rsd_pct,
                 file_comp_stats.median, file_comp_stats.rsd_pct);

        silesia_json_entries.push(format!(
            "{{\"name\":\"{}\",\"type\":\"{}\",\"uncomp_bytes\":{},\"comp_bytes\":{},\"decomp_mb_s\":{:.2},\"decomp_rsd\":{:.2},\"comp_mb_s\":{:.2},\"comp_rsd\":{:.2}}}",
            item.name, item.file_type, item.uncompressed.len(), item.compressed.len(),
            file_decomp_stats.median, file_decomp_stats.rsd_pct,
            file_comp_stats.median, file_comp_stats.rsd_pct
        ));
    }

    let silesia_total_mb = silesia_uncomp_total as f64 / 1_000_000.0;
    let silesia_agg_decomp_throughput: Vec<f64> = silesia_agg_decomp_samples.iter().map(|elapsed| silesia_total_mb / elapsed).collect();
    let silesia_agg_comp_throughput: Vec<f64> = silesia_agg_comp_samples.iter().map(|elapsed| silesia_total_mb / elapsed).collect();

    let silesia_agg_decomp_stats = compute_stats(silesia_agg_decomp_throughput);
    let silesia_agg_comp_stats = compute_stats(silesia_agg_comp_throughput);

    println!("\nSilesia Aggregate Decompression: Median {:.2} MB/s (±{:.1}%)", silesia_agg_decomp_stats.median, silesia_agg_decomp_stats.rsd_pct);
    println!("Silesia Aggregate Compression:   Median {:.2} MB/s (±{:.1}%)", silesia_agg_comp_stats.median, silesia_agg_comp_stats.rsd_pct);

    // Save JSON if requested
    if let Some(path) = json_out {
        let json_data = format!(
            "{{\n  \"nexrad\": {{\n    \"uncomp_bytes\": {},\n    \"comp_bytes\": {},\n    \"decomp_mb_s\": {:.2},\n    \"decomp_rsd\": {:.2},\n    \"comp_mb_s\": {:.2},\n    \"comp_rsd\": {:.2}\n  }},\n  \"silesia_aggregate\": {{\n    \"uncomp_bytes\": {},\n    \"comp_bytes\": {},\n    \"decomp_mb_s\": {:.2},\n    \"decomp_rsd\": {:.2},\n    \"comp_mb_s\": {:.2},\n    \"comp_rsd\": {:.2}\n  }},\n  \"silesia_files\": [\n    {}\n  ]\n}}\n",
            nexrad_uncomp_bytes, nexrad_comp_bytes,
            nexrad_decomp_stats.median, nexrad_decomp_stats.rsd_pct,
            nexrad_comp_stats.median, nexrad_comp_stats.rsd_pct,
            silesia_uncomp_total, silesia_comp_total,
            silesia_agg_decomp_stats.median, silesia_agg_decomp_stats.rsd_pct,
            silesia_agg_comp_stats.median, silesia_agg_comp_stats.rsd_pct,
            silesia_json_entries.join(",\n    ")
        );
        fs::write(&path, json_data)?;
        println!("\nSaved machine-readable benchmark JSON to {}", path);
    }

    Ok(())
}
