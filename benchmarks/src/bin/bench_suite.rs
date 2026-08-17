use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::ffi::c_char;

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

fn compress_bz2(input: &[u8]) -> Result<Vec<u8>, String> {
    let dest_capacity = input.len() + (input.len() / 100) + 600;
    let mut output = vec![0u8; dest_capacity];
    let mut dest_len = dest_capacity as u32;

    let ret = unsafe {
        libbz2_rs_sys::BZ2_bzBuffToBuffCompress(
            output.as_mut_ptr() as *mut c_char,
            &mut dest_len,
            input.as_ptr() as *mut c_char,
            input.len() as u32,
            9,
            0,
            30,
        )
    };

    if ret != 0 {
        return Err(format!("BZ2_bzBuffToBuffCompress failed: {}", ret));
    }

    output.truncate(dest_len as usize);
    Ok(output)
}

fn decompress_bz2(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut out = Vec::with_capacity(input.len() * 4);
    let mut in_pos = 0;
    let mut is_first_stream = true;

    while in_pos < input.len() {
        if !is_first_stream {
            if in_pos + 4 > input.len() {
                break;
            }
            let header = i32::from_be_bytes(input[in_pos..in_pos+4].try_into().unwrap());
            if header < 0 {
                break;
            }
            in_pos += 4;
        }

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

struct DatasetItem {
    name: String,
    file_type: String,
    uncompressed: Vec<u8>,
    compressed: Vec<u8>,
}

fn load_dataset(ref_dir: &Path, comp_dir: &Path, files: &[FileMeta]) -> Result<Vec<DatasetItem>, Box<dyn std::error::Error>> {
    let mut items = Vec::new();
    for f in files {
        let ref_path = ref_dir.join(f.name);
        let comp_path = comp_dir.join(format!("{}.bz2", f.name));
        let uncompressed = fs::read(&ref_path)?;
        let compressed = fs::read(&comp_path)?;

        // Pre-flight verification
        let decomp = decompress_bz2(&compressed)?;
        assert_eq!(decomp.len(), uncompressed.len(), "Preflight decomp length mismatch on {}", f.name);
        assert_eq!(decomp, uncompressed, "Preflight decomp content mismatch on {}", f.name);

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
    for i in 1..=30 {
        let name = format!("nexrad{}", i);
        let ref_path = ref_dir.join(&name);
        let comp_path = comp_dir.join(format!("{}.bz2", name));
        let uncompressed = fs::read(&ref_path)?;
        let compressed = fs::read(&comp_path)?;

        let decomp = decompress_bz2(&compressed)?;
        assert_eq!(decomp.len(), uncompressed.len(), "Preflight decomp mismatch on {}", name);
        assert_eq!(decomp, uncompressed, "Preflight decomp mismatch on {}", name);

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

    let mut silesia_json = Vec::new();
    let mut nexrad_json = Vec::new();

    // 1. Benchmark NEXRAD
    println!("=== 1. NEXRAD Radar Dataset ({} Iterations) ===", iterations);
    let start_nexrad_decomp = Instant::now();
    let mut nexrad_uncomp_bytes = 0usize;
    let mut nexrad_comp_bytes = 0usize;

    for item in &nexrad_items {
        nexrad_uncomp_bytes += item.uncompressed.len();
        nexrad_comp_bytes += item.compressed.len();
    }

    for _ in 0..iterations {
        for item in &nexrad_items {
            let decomp = decompress_bz2(&item.compressed)?;
            std::hint::black_box(decomp);
        }
    }
    let nexrad_decomp_time = start_nexrad_decomp.elapsed().as_secs_f64();
    let nexrad_decomp_total_mb = (nexrad_uncomp_bytes * iterations) as f64 / 1_000_000.0;
    let nexrad_decomp_speed = nexrad_decomp_total_mb / nexrad_decomp_time;

    let start_nexrad_comp = Instant::now();
    for _ in 0..iterations {
        for item in &nexrad_items {
            let comp = compress_bz2(&item.uncompressed).unwrap();
            std::hint::black_box(comp);
        }
    }
    let nexrad_comp_time = start_nexrad_comp.elapsed().as_secs_f64();
    let nexrad_comp_total_mb = (nexrad_uncomp_bytes * iterations) as f64 / 1_000_000.0;
    let nexrad_comp_speed = nexrad_comp_total_mb / nexrad_comp_time;

    println!("NEXRAD Decompression: {:.2} MB in {:.4} s ({:.2} MB/s)", nexrad_decomp_total_mb, nexrad_decomp_time, nexrad_decomp_speed);
    println!("NEXRAD Compression:   {:.2} MB in {:.4} s ({:.2} MB/s)", nexrad_comp_total_mb, nexrad_comp_time, nexrad_comp_speed);

    // 2. Benchmark Silesia (per-file and aggregate)
    println!("\n=== 2. Silesia Corpus Dataset ({} Iterations) ===", iterations);
    println!("{:<18} | {:<16} | {:>10} | {:>8} | {:>12} | {:>14}", 
             "File", "Type", "Uncomp MB", "Ratio", "Decomp MB/s", "Compress MB/s");
    println!("{:-<18}-|-{:-<16}-|-{:->10}-|-{:->8}-|-{:->12}-|-{:->14}", 
             "", "", "", "", "", "");

    let mut silesia_uncomp_total = 0usize;
    let mut silesia_comp_total = 0usize;
    let mut silesia_decomp_total_time = 0.0f64;
    let mut silesia_comp_total_time = 0.0f64;

    for item in &silesia_items {
        silesia_uncomp_total += item.uncompressed.len();
        silesia_comp_total += item.compressed.len();

        let start_decomp = Instant::now();
        for _ in 0..iterations {
            let decomp = decompress_bz2(&item.compressed)?;
            std::hint::black_box(decomp);
        }
        let elapsed_decomp = start_decomp.elapsed().as_secs_f64();
        silesia_decomp_total_time += elapsed_decomp;
        let file_decomp_mb = (item.uncompressed.len() * iterations) as f64 / 1_000_000.0;
        let file_decomp_speed = file_decomp_mb / elapsed_decomp;

        let start_comp = Instant::now();
        for _ in 0..iterations {
            let comp = compress_bz2(&item.uncompressed).unwrap();
            std::hint::black_box(comp);
        }
        let elapsed_comp = start_comp.elapsed().as_secs_f64();
        silesia_comp_total_time += elapsed_comp;
        let file_comp_mb = (item.uncompressed.len() * iterations) as f64 / 1_000_000.0;
        let file_comp_speed = file_comp_mb / elapsed_comp;

        let uncomp_mb = item.uncompressed.len() as f64 / 1_000_000.0;
        let ratio = (item.compressed.len() as f64 / item.uncompressed.len() as f64) * 100.0;

        println!("{:<18} | {:<16} | {:>9.2}M | {:>7.2}% | {:>10.2} M/s | {:>12.2} M/s",
                 item.name, item.file_type, uncomp_mb, ratio, file_decomp_speed, file_comp_speed);

        silesia_json.push(format!(
            "{{\"name\":\"{}\",\"type\":\"{}\",\"uncomp_bytes\":{},\"comp_bytes\":{},\"decomp_mb_s\":{:.2},\"comp_mb_s\":{:.2}}}",
            item.name, item.file_type, item.uncompressed.len(), item.compressed.len(), file_decomp_speed, file_comp_speed
        ));
    }

    let silesia_decomp_total_mb = (silesia_uncomp_total * iterations) as f64 / 1_000_000.0;
    let silesia_decomp_speed = silesia_decomp_total_mb / silesia_decomp_total_time;
    let silesia_comp_total_mb = (silesia_uncomp_total * iterations) as f64 / 1_000_000.0;
    let silesia_comp_speed = silesia_comp_total_mb / silesia_comp_total_time;

    println!("\nSilesia Aggregate Decompression: {:.2} MB/s", silesia_decomp_speed);
    println!("Silesia Aggregate Compression:   {:.2} MB/s", silesia_comp_speed);

    // Save JSON if requested
    if let Some(path) = json_out {
        let json_data = format!(
            "{{\n  \"nexrad\": {{\n    \"uncomp_bytes\": {},\n    \"comp_bytes\": {},\n    \"decomp_mb_s\": {:.2},\n    \"comp_mb_s\": {:.2}\n  }},\n  \"silesia_aggregate\": {{\n    \"uncomp_bytes\": {},\n    \"comp_bytes\": {},\n    \"decomp_mb_s\": {:.2},\n    \"comp_mb_s\": {:.2}\n  }},\n  \"silesia_files\": [\n    {}\n  ]\n}}\n",
            nexrad_uncomp_bytes, nexrad_comp_bytes, nexrad_decomp_speed, nexrad_comp_speed,
            silesia_uncomp_total, silesia_comp_total, silesia_decomp_speed, silesia_comp_speed,
            silesia_json.join(",\n    ")
        );
        fs::write(&path, json_data)?;
        println!("\nSaved machine-readable benchmark JSON to {}", path);
    }

    Ok(())
}
