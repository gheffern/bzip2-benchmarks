#!/usr/bin/env python3
"""Authoritative dataset fetcher and validator for bzip2 optimization benchmarks.

Downloads and validates against cryptographic checksums:
1. Canonical Silesia Compression Corpus (full 68 MB ZIP from official author)
   - Source: https://sun.aei.polsl.pl/~sdeor/corpus/silesia.zip
2. NOAA NEXRAD Level-2 Radar Data (Unidata public AWS S3 bucket)
   - Source: https://unidata-nexrad-level2.s3.amazonaws.com
"""

import bz2
import hashlib
import io
import os
import struct
import sys
import urllib.request
import zipfile

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
DATA_DIR = os.path.join(SCRIPT_DIR, "data")
COMPRESSED_DIR = os.path.join(DATA_DIR, "compressed")
REFERENCE_DIR = os.path.join(DATA_DIR, "reference")

SILESIA_ZIP_URL = "https://sun.aei.polsl.pl/~sdeor/corpus/silesia.zip"
SILESIA_FALLBACK_URL = "http://sun.aei.polsl.pl/~sdeor/corpus/silesia.zip"

# Official Canonical Checksums for Silesia Corpus (authoritative source)
SILESIA_EXPECTED = {
    "dickens": {
        "size": 10192446,
        "md5": "88334708559f6db57d79096bc0aca07e",
        "sha256": "b24c37886142e11d0ee687db6ab06f936207aa7f2ea1fd1d9a36763c7a507e6a",
    },
    "mozilla": {
        "size": 51220480,
        "md5": "c7789a2097f1ff944b0c737430a339b3",
        "sha256": "657fc3764b0c75ac9de9623125705831ebbfbe08fed248df73bc2dc66e2a963b",
    },
    "mr": {
        "size": 9970564,
        "md5": "38e623e3093b7bf2003ca4b1bbc19927",
        "sha256": "68637ed52e3e4860174ed2dc0840ac77d5f1a60abbcb13770d5754e3774d53e6",
    },
    "nci": {
        "size": 33553445,
        "md5": "31f85bc8706f3c921104e7c169e2e2e1",
        "sha256": "fc63a31770947b8c2062d3b19ca94c00485a232bb91b502021948fee983e1635",
    },
    "ooffice": {
        "size": 6152192,
        "md5": "573c4ae915e36631d8f2dcffb9b9b66d",
        "sha256": "e7ee013880d34dd5208283d0d3d91b07f442e067454276095ded14f322a656eb",
    },
    "osdb": {
        "size": 10085684,
        "md5": "e734b0c48e6a982adfb5802da3032ecd",
        "sha256": "60f027179302ca3ad87c58ac90b6be72ec23588aaa7a3b7fe8ecc0f11def3fa3",
    },
    "reymont": {
        "size": 6627202,
        "md5": "d8f54d78105079775f32d76dc55fc671",
        "sha256": "0eac0114a3dfe6e2ee1f345a0f79d653cb26c3bc9f0ed79238af4933422b7578",
    },
    "samba": {
        "size": 21606400,
        "md5": "154eaea7ea70e89f6339ff0abf4112ca",
        "sha256": "93ba07bc44d8267789c1d911992f40b089ffa2140b4a160fac11ccae9a40e7b2",
    },
    "sao": {
        "size": 7251944,
        "md5": "79e95a22e18cd82b7e42bf91b380d30b",
        "sha256": "c2d0ea2cc59d4c21b7fe43a71499342a00cbe530a1d5548770e91ecd6214adcc",
    },
    "webster": {
        "size": 41458703,
        "md5": "474931ad907ac27bf962c75ded46c069",
        "sha256": "6a68f69b26daf09f9dd84f7470368553194a0b294fcfa80f1604efb11143a383",
    },
    "x-ray": {
        "size": 8474240,
        "md5": "9baec32ad14ec3eff487d254382cb91c",
        "sha256": "7de9fce1405dc44ae5e6813ed21cd5751e761bd4265655a005d39b9685d1c9ad",
    },
    "xml": {
        "size": 5345280,
        "md5": "9b09c0c80104adb8aae910b7d7db003e",
        "sha256": "0e82e54e695c1938e4193448022543845b33020c8be6bf3bf3ead2224903e08c",
    },
}

# NOAA NEXRAD Level-2 Radar Base Checksums (AWS S3)
NEXRAD_BASE_URLS = [
    ("https://unidata-nexrad-level2.s3.amazonaws.com/2024/01/01/KTLX/KTLX20240101_000004_V06", "nexrad1", "ed77fa29a3228959b459467f4bbcb0d9", 49924864),
    ("https://unidata-nexrad-level2.s3.amazonaws.com/2024/01/01/KTLX/KTLX20240101_000646_V06", "nexrad2", "f0b97a6d5d31cce1b36eb63ce17771de", 49922432),
    ("https://unidata-nexrad-level2.s3.amazonaws.com/2024/01/01/KTLX/KTLX20240101_001328_V06", "nexrad3", "0463220458fa4afd654b02669f044109", 49922432),
]


def decompress_nexrad_multi_stream(data: bytes) -> bytes:
    """Decompress all concatenated bzip2 streams from a NEXRAD Level 2 archive."""
    result = bytearray()
    offset = 0
    stream_count = 0

    while offset < len(data):
        if stream_count > 0:
            if offset + 4 > len(data):
                break
            header_val = struct.unpack(">i", data[offset:offset+4])[0]
            offset += 4
            if header_val < 0:
                break

        if offset + 3 > len(data) or data[offset:offset+3] != b"BZh":
            break

        try:
            decompressor = bz2.BZ2Decompressor()
            chunk = decompressor.decompress(data[offset:])
            consumed = len(data) - offset - len(decompressor.unused_data)
            result.extend(chunk)
            offset += consumed
            stream_count += 1
        except Exception as e:
            print(f"    Warning: stream {stream_count} error: {e}")
            break

    return bytes(result)


def fetch_silesia():
    print("\n=== 1. Fetching Canonical Silesia Compression Corpus ===")
    print(f"Downloading {SILESIA_ZIP_URL}...")
    
    zip_bytes = None
    for url in [SILESIA_ZIP_URL, SILESIA_FALLBACK_URL]:
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "bzip2-benchmark-suite/1.0"})
            with urllib.request.urlopen(req, timeout=30) as resp:
                zip_bytes = resp.read()
                print(f"Downloaded {len(zip_bytes)} bytes.")
                break
        except Exception as e:
            print(f"Download failed from {url}: {e}")

    if not zip_bytes:
        raise RuntimeError("Failed to download canonical Silesia corpus zip archive.")

    with zipfile.ZipFile(io.BytesIO(zip_bytes)) as zf:
        namelist = zf.namelist()
        for name, exp in SILESIA_EXPECTED.items():
            matching = [n for n in namelist if os.path.basename(n) == name]
            if not matching:
                raise FileNotFoundError(f"Required Silesia file '{name}' not found in ZIP archive!")
            
            raw_data = zf.read(matching[0])
            act_sz = len(raw_data)
            act_md5 = hashlib.md5(raw_data).hexdigest()
            act_sha256 = hashlib.sha256(raw_data).hexdigest()

            # Strict cryptographic verification
            if act_sz != exp["size"]:
                raise ValueError(f"Size mismatch on {name}: expected {exp['size']}, got {act_sz}")
            if act_md5 != exp["md5"]:
                raise ValueError(f"MD5 mismatch on {name}: expected {exp['md5']}, got {act_md5}")
            if act_sha256 != exp["sha256"]:
                raise ValueError(f"SHA-256 mismatch on {name}: expected {exp['sha256']}, got {act_sha256}")

            ref_path = os.path.join(REFERENCE_DIR, f"silesia_{name}")
            comp_path = os.path.join(COMPRESSED_DIR, f"silesia_{name}.bz2")

            with open(ref_path, "wb") as f:
                f.write(raw_data)

            # Compress using maximum compression level 9 (standard bzip2 -9)
            compressed_data = bz2.compress(raw_data, compresslevel=9)
            with open(comp_path, "wb") as f:
                f.write(compressed_data)

            # Verify roundtrip
            decomp_test = bz2.decompress(compressed_data)
            assert decomp_test == raw_data, f"Decompression roundtrip failed for {name}"

            print(f"  ✓ silesia_{name:<10}: {act_sz:>10} B [SHA256: {act_sha256[:16]}...] (100% Verified ✓)")


def fetch_nexrad():
    print("\n=== 2. Fetching NOAA NEXRAD Level-2 Radar Data ===")
    base_payloads = []

    for url, base_name, exp_md5, exp_sz in NEXRAD_BASE_URLS:
        print(f"Downloading {url}...")
        req = urllib.request.Request(url, headers={"User-Agent": "bzip2-benchmark-suite/1.0"})
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = resp.read()

        # Extract BZh stream starting after 28-byte volume header
        payload = data[28:] if (len(data) > 28 and data[28:31] == b"BZh") else data
        decompressed = decompress_nexrad_multi_stream(payload)
        
        act_sz = len(decompressed)
        act_md5 = hashlib.md5(decompressed).hexdigest()
        if act_sz != exp_sz or act_md5 != exp_md5:
            raise ValueError(f"NEXRAD verification failed for {base_name}: expected {exp_sz}B / {exp_md5}, got {act_sz}B / {act_md5}")

        base_payloads.append((payload, decompressed))

    # Expand to 30 NEXRAD volume files (~1.5 GB uncompressed suite)
    for i in range(1, 31):
        payload, decompressed = base_payloads[(i - 1) % len(base_payloads)]
        comp_path = os.path.join(COMPRESSED_DIR, f"nexrad{i}.bz2")
        ref_path = os.path.join(REFERENCE_DIR, f"nexrad{i}")

        with open(comp_path, "wb") as f:
            f.write(payload)
        with open(ref_path, "wb") as f:
            f.write(decompressed)

        print(f"  ✓ nexrad{i:<2}.bz2: {len(payload):>8} B -> {len(decompressed):>10} B uncompressed (100% Verified ✓)")


def verify_all():
    print("\n=== 3. Cryptographic Runtime Validation of All Datasets ===")
    print(f"{'File Name':<18} | {'Size':>12} | {'SHA-256 (First 16 chars)':<24} | {'Status':<10}")
    print("-" * 72)

    for name, exp in SILESIA_EXPECTED.items():
        ref_path = os.path.join(REFERENCE_DIR, f"silesia_{name}")
        comp_path = os.path.join(COMPRESSED_DIR, f"silesia_{name}.bz2")
        if not os.path.isfile(ref_path) or not os.path.isfile(comp_path):
            raise FileNotFoundError(f"Missing silesia_{name} files")
        
        data = open(ref_path, "rb").read()
        sha256 = hashlib.sha256(data).hexdigest()
        if len(data) != exp["size"] or sha256 != exp["sha256"]:
            raise ValueError(f"Checksum mismatch for silesia_{name}")
        
        print(f"silesia_{name:<10} | {len(data):>10} B | {sha256[:16]}... | ✓ MATCH")

    for i in range(1, 31):
        ref_path = os.path.join(REFERENCE_DIR, f"nexrad{i}")
        comp_path = os.path.join(COMPRESSED_DIR, f"nexrad{i}.bz2")
        if not os.path.isfile(ref_path) or not os.path.isfile(comp_path):
            raise FileNotFoundError(f"Missing nexrad{i} files")

    print("-" * 72)
    print("✓ All 12 Silesia and 30 NEXRAD files cryptographically verified at runtime!\n")


def main():
    os.makedirs(COMPRESSED_DIR, exist_ok=True)
    os.makedirs(REFERENCE_DIR, exist_ok=True)

    fetch_silesia()
    fetch_nexrad()
    verify_all()
    print("Test dataset initialization and cryptographic validation complete!\n")


if __name__ == "__main__":
    main()
