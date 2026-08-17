use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let compressed_dir = Path::new("data/compressed");
    let reference_dir = Path::new("data/reference");
    fs::create_dir_all(compressed_dir)?;
    fs::create_dir_all(reference_dir)?;

    println!("=== 1. Fetching NEXRAD Radar Test Data ===");
    let nexrad_urls = vec![
        ("https://unidata-nexrad-level2.s3.amazonaws.com/2024/01/01/KTLX/KTLX20240101_000004_V06", "nexrad1.bz2"),
        ("https://unidata-nexrad-level2.s3.amazonaws.com/2024/01/01/KTLX/KTLX20240101_000646_V06", "nexrad2.bz2"),
        ("https://unidata-nexrad-level2.s3.amazonaws.com/2024/01/01/KTLX/KTLX20240101_001328_V06", "nexrad3.bz2"),
    ];

    let client = reqwest::blocking::Client::builder()
        .user_agent("bzip2-benchmarks/1.0")
        .build()?;

    let mut base_payloads = Vec::new();
    for (url, filename) in nexrad_urls {
        let target_path = compressed_dir.join(filename);
        println!("Downloading NEXRAD: {} -> {:?}", url, target_path);
        let resp = client.get(url).send()?;
        let bytes = resp.bytes()?;
        let bz2_payload = if bytes.len() > 28 && &bytes[28..31] == b"BZh" {
            bytes[28..].to_vec()
        } else {
            bytes.to_vec()
        };
        fs::write(&target_path, &bz2_payload)?;
        base_payloads.push(bz2_payload);
    }

    // Expand to 30 NEXRAD files for a large dataset (~50 MB compressed)
    for i in 4..=30 {
        let base_bytes = &base_payloads[(i - 1) % base_payloads.len()];
        let target_path = compressed_dir.join(format!("nexrad{}.bz2", i));
        fs::write(&target_path, base_bytes)?;
    }

    println!("\n=== 2. Fetching Silesia Corpus Test Data ===");
    let silesia_files = vec![
        "dickens",
        "mozilla",
        "mr",
        "nci",
        "ooffice",
        "osdb",
        "reymont",
        "samba",
        "sao",
        "webster",
        "xml",
        "x-ray",
    ];

    for name in silesia_files {
        let raw_url = format!("https://raw.githubusercontent.com/yewq/Silesia-compression-corpus/main/{}", name);
        let target_bz2_path = compressed_dir.join(format!("silesia_{}.bz2", name));
        
        println!("Fetching Silesia file: {} -> {:?}", raw_url, target_bz2_path);
        let resp = client.get(&raw_url).send();
        let uncompressed_data = match resp {
            Ok(r) if r.status().is_success() => r.bytes()?.to_vec(),
            _ => {
                println!("Failed downloading {}, checking silesia-small.tar backup...", name);
                // Fallback to silesia-small.tar if offline/github raw fails
                extract_from_silesia_tar(name)?
            }
        };

        // Save reference uncompressed file
        let ref_path = reference_dir.join(format!("silesia_{}", name));
        fs::write(&ref_path, &uncompressed_data)?;

        // Compress using libbz2-rs-sys to get reference .bz2
        let bz2_data = compress_bz2(&uncompressed_data)?;
        fs::write(&target_bz2_path, &bz2_data)?;
        println!("Saved compressed {:?} ({} bytes raw -> {} bytes bz2)", target_bz2_path, uncompressed_data.len(), bz2_data.len());
    }

    println!("\n=== 3. Decompressing NEXRAD Data for Reference Output ===");
    for i in 1..=30 {
        let bz2_path = compressed_dir.join(format!("nexrad{}.bz2", i));
        let ref_path = reference_dir.join(format!("nexrad{}", i));
        let compressed_bytes = fs::read(&bz2_path)?;
        let decompressed_bytes = decompress_bz2(&compressed_bytes)?;
        fs::write(&ref_path, &decompressed_bytes)?;
        println!("Saved NEXRAD reference output {:?} ({} bytes uncompressed)", ref_path, decompressed_bytes.len());
    }

    println!("\nTest data setup complete!");
    Ok(())
}

fn extract_from_silesia_tar(filename: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let tar_path = Path::new("../libbzip2-rs/silesia-small.tar");
    let file = File::open(tar_path)?;
    let mut archive = tar::Archive::new(file);
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        if path.to_string_lossy().ends_with(filename) {
            let mut data = Vec::new();
            entry.read_to_end(&mut data)?;
            return Ok(data);
        }
    }
    Err(format!("File {} not found in silesia-small.tar", filename).into())
}

fn compress_bz2(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut dest = vec![0u8; input.len() + input.len() / 100 + 600];
    let mut dest_len = dest.len() as libc::c_uint;
    let res = unsafe {
        libbz2_rs_sys::BZ2_bzBuffToBuffCompress(
            dest.as_mut_ptr() as *mut libc::c_char,
            &mut dest_len,
            input.as_ptr() as *mut libc::c_char,
            input.len() as libc::c_uint,
            9,
            0,
            30,
        )
    };
    if res != libbz2_rs_sys::BZ_OK {
        return Err(format!("BZ2_bzBuffToBuffCompress failed with code {}", res).into());
    }
    dest.truncate(dest_len as usize);
    Ok(dest)
}

fn decompress_bz2(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut out = Vec::new();
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
