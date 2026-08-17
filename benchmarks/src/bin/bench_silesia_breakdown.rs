use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::ffi::c_char;

struct SilesiaFileInfo {
    name: &'static str,
    file_type: &'static str,
    description: &'static str,
}

const SILESIA_FILES: &[SilesiaFileInfo] = &[
    SilesiaFileInfo { name: "silesia_dickens", file_type: "Text (ASCII)", description: "Charles Dickens literature" },
    SilesiaFileInfo { name: "silesia_mozilla", file_type: "Tar / Executables", description: "Mozilla tarred binaries" },
    SilesiaFileInfo { name: "silesia_mr", file_type: "Medical (MRI)", description: "Medical resonance imaging" },
    SilesiaFileInfo { name: "silesia_nci", file_type: "Text (Chem DB)", description: "NCI chemical database" },
    SilesiaFileInfo { name: "silesia_ooffice", file_type: "x86 Executable", description: "OpenOffice dynamic library" },
    SilesiaFileInfo { name: "silesia_osdb", file_type: "DB Binary", description: "Database binary records" },
    SilesiaFileInfo { name: "silesia_reymont", file_type: "PDF Document", description: "Reymont literature PDF" },
    SilesiaFileInfo { name: "silesia_samba", file_type: "Tar / C Source", description: "Samba source code tar" },
    SilesiaFileInfo { name: "silesia_sao", file_type: "Binary Catalog", description: "SAO star database" },
    SilesiaFileInfo { name: "silesia_webster", file_type: "Text (Dictionary)", description: "Webster dictionary text" },
    SilesiaFileInfo { name: "silesia_xml", file_type: "XML Markup", description: "XML structured files" },
    SilesiaFileInfo { name: "silesia_x-ray", file_type: "Medical (X-Ray)", description: "Medical X-ray image" },
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let compressed_dir = Path::new("data/compressed");
    let reference_dir = Path::new("data/reference");
    let iterations = 20;

    println!("{:<18} | {:<16} | {:>10} | {:>10} | {:>8} | {:>12} | {:>14}", 
             "File", "Type", "Uncomp MB", "Comp MB", "Ratio", "Decomp MB/s", "Compress MB/s");
    println!("{:-<18}-|-{:-<16}-|-{:->10}-|-{:->10}-|-{:->8}-|-{:->12}-|-{:->14}", 
             "", "", "", "", "", "", "");

    let mut json_results = Vec::new();

    for info in SILESIA_FILES {
        let ref_path = reference_dir.join(info.name);
        let comp_path = compressed_dir.join(format!("{}.bz2", info.name));

        let uncompressed = fs::read(&ref_path)?;
        let compressed = fs::read(&comp_path)?;

        // Warmup & verification
        let decomp_warm = decompress_bz2(&compressed)?;
        assert_eq!(decomp_warm.len(), uncompressed.len(), "Warmup decomp length mismatch for {}", info.name);
        assert_eq!(decomp_warm, uncompressed, "Warmup decomp content mismatch for {}", info.name);

        let comp_warm = compress_bz2(&uncompressed).map_err(|e| e)?;
        let decomp_comp_warm = decompress_bz2(&comp_warm)?;
        assert_eq!(decomp_comp_warm, uncompressed, "Warmup comp roundtrip mismatch for {}", info.name);

        // Decompression benchmark
        let start_decomp = Instant::now();
        for _ in 0..iterations {
            let decomp = decompress_bz2(&compressed)?;
            std::hint::black_box(decomp);
        }
        let elapsed_decomp = start_decomp.elapsed().as_secs_f64();
        let total_decomp_mb = (uncompressed.len() * iterations) as f64 / 1_000_000.0;
        let decomp_speed = total_decomp_mb / elapsed_decomp;

        // Compression benchmark
        let start_comp = Instant::now();
        for _ in 0..iterations {
            let comp = compress_bz2(&uncompressed).unwrap();
            std::hint::black_box(comp);
        }
        let elapsed_comp = start_comp.elapsed().as_secs_f64();
        let total_comp_mb = (uncompressed.len() * iterations) as f64 / 1_000_000.0;
        let comp_speed = total_comp_mb / elapsed_comp;

        let uncomp_mb = uncompressed.len() as f64 / 1_000_000.0;
        let comp_mb = compressed.len() as f64 / 1_000_000.0;
        let ratio = (compressed.len() as f64 / uncompressed.len() as f64) * 100.0;

        println!("{:<18} | {:<16} | {:>9.2}M | {:>9.2}M | {:>7.2}% | {:>10.2} M/s | {:>12.2} M/s",
                 info.name, info.file_type, uncomp_mb, comp_mb, ratio, decomp_speed, comp_speed);

        json_results.push(format!(
            "{{\"name\":\"{}\",\"type\":\"{}\",\"uncomp_bytes\":{},\"comp_bytes\":{},\"decomp_mb_s\":{:.2},\"comp_mb_s\":{:.2}}}",
            info.name, info.file_type, uncompressed.len(), compressed.len(), decomp_speed, comp_speed
        ));
    }

    if let Some(json_path) = std::env::args().nth(1) {
        let json_content = format!("[\n{}\n]\n", json_results.join(",\n"));
        fs::write(&json_path, json_content)?;
        println!("\nPer-file metrics saved to {}", json_path);
    }

    Ok(())
}
