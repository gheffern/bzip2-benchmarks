use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use libbz2_rs_sys::{bz_stream, BZ2_bzDecompressInit, BZ2_bzDecompress, BZ2_bzDecompressEnd};

fn decompress_bz2(input: &[u8]) -> Result<Vec<u8>, String> {
    let mut output = Vec::with_capacity(input.len() * 10);
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

        let mut strm: bz_stream = unsafe { std::mem::zeroed() };
        let res = unsafe { BZ2_bzDecompressInit(&mut strm, 0, 0) };
        if res != 0 {
            return Err(format!("BZ2_bzDecompressInit failed: {}", res));
        }

        let mut buf = [0u8; 65536];

        loop {
            let chunk = &input[in_pos..];
            strm.next_in = chunk.as_ptr() as *mut _;
            strm.avail_in = chunk.len() as u32;

            strm.next_out = buf.as_mut_ptr() as *mut _;
            strm.avail_out = buf.len() as u32;

            let ret = unsafe { BZ2_bzDecompress(&mut strm) };
            let produced = buf.len() - strm.avail_out as usize;
            output.extend_from_slice(&buf[..produced]);

            let consumed = chunk.len() - strm.avail_in as usize;
            in_pos += consumed;

            if ret == 4 { // BZ_STREAM_END
                break;
            } else if ret != 0 {
                unsafe { BZ2_bzDecompressEnd(&mut strm) };
                return Err(format!("BZ2_bzDecompress failed: {}", ret));
            }
        }

        unsafe { BZ2_bzDecompressEnd(&mut strm) };
        is_first_stream = false;
    }

    Ok(output)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let compressed_dir = Path::new("data/compressed");
    let reference_dir = Path::new("data/reference");

    let mut nexrad_files = Vec::new();
    for entry in fs::read_dir(compressed_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.file_name().unwrap().to_string_lossy().starts_with("nexrad") && path.extension().map_or(false, |ext| ext == "bz2") {
            nexrad_files.push(path);
        }
    }
    nexrad_files.sort();

    println!("PER-FILE NEXRAD BREAKDOWN (100 iterations per file):\n");
    println!("{:<35} | {:<10} | {:<10} | {:<12}", "NEXRAD File", "Comp Size", "Uncomp Size", "Speed (MB/s)");
    println!("{:-<35}-|-{:-<10}-|-{:-<10}-|-{:-<12}", "", "", "", "");

    for path in &nexrad_files {
        let compressed = fs::read(path)?;
        let fname = path.file_stem().unwrap().to_string_lossy().to_string();
        let ref_path = reference_dir.join(&fname);
        let ref_bytes = fs::read(&ref_path)?;

        let iterations = 100;
        let start = Instant::now();
        for _ in 0..iterations {
            let decompressed = decompress_bz2(&compressed).unwrap();
            std::hint::black_box(decompressed);
        }
        let elapsed = start.elapsed().as_secs_f64();
        let total_mb = (ref_bytes.len() * iterations) as f64 / (1024.0 * 1024.0);
        let mb_s = total_mb / elapsed;

        let comp_kb = compressed.len() as f64 / 1024.0;
        let uncomp_mb = ref_bytes.len() as f64 / (1024.0 * 1024.0);

        println!("{:<35} | {:>7.1} KB | {:>8.2} MB | {:>10.2} MB/s", fname, comp_kb, uncomp_mb, mb_s);
    }

    Ok(())
}
