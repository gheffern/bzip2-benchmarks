#!/usr/bin/env python3
"""Authoritative dataset fetcher and validator for bzip2 optimization benchmarks.

Downloads:
1. Canonical Silesia Compression Corpus (full 68 MB ZIP from official author)
   - Source: https://sun.aei.polsl.pl/~sdeor/corpus/silesia.zip
2. NOAA NEXRAD Level-2 Radar Data (Unidata public AWS S3 bucket)
   - Source: https://unidata-nexrad-level2.s3.amazonaws.com
"""

import bz2
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

SILESIA_FILES = [
    "dickens", "mozilla", "mr", "nci", "ooffice",
    "osdb", "reymont", "samba", "sao", "webster", "xml", "x-ray",
]

NEXRAD_BASE_URLS = [
    ("https://unidata-nexrad-level2.s3.amazonaws.com/2024/01/01/KTLX/KTLX20240101_000004_V06", "nexrad1.bz2"),
    ("https://unidata-nexrad-level2.s3.amazonaws.com/2024/01/01/KTLX/KTLX20240101_000646_V06", "nexrad2.bz2"),
    ("https://unidata-nexrad-level2.s3.amazonaws.com/2024/01/01/KTLX/KTLX20240101_001328_V06", "nexrad3.bz2"),
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
        for name in SILESIA_FILES:
            matching = [n for n in namelist if os.path.basename(n) == name]
            if not matching:
                print(f"  Warning: {name} not found in zip archive!")
                continue
            
            raw_data = zf.read(matching[0])
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
            assert decomp_test == raw_data, f"Integrity check failed for {name}"

            print(f"  ✓ silesia_{name:<10}: {len(raw_data):>10} B uncompressed -> {len(compressed_data):>10} B bz2")


def fetch_nexrad():
    print("\n=== 2. Fetching NOAA NEXRAD Level-2 Radar Data ===")
    base_payloads = []

    for url, filename in NEXRAD_BASE_URLS:
        print(f"Downloading {url}...")
        req = urllib.request.Request(url, headers={"User-Agent": "bzip2-benchmark-suite/1.0"})
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = resp.read()

        # Extract BZh stream starting after 28-byte volume header
        payload = data[28:] if (len(data) > 28 and data[28:31] == b"BZh") else data
        base_payloads.append(payload)

    # Expand to 30 NEXRAD volume files (~1.5 GB uncompressed suite)
    for i in range(1, 31):
        payload = base_payloads[(i - 1) % len(base_payloads)]
        comp_path = os.path.join(COMPRESSED_DIR, f"nexrad{i}.bz2")
        ref_path = os.path.join(REFERENCE_DIR, f"nexrad{i}")

        with open(comp_path, "wb") as f:
            f.write(payload)

        # Decompress full multi-stream volume
        decompressed = decompress_nexrad_multi_stream(payload)
        with open(ref_path, "wb") as f:
            f.write(decompressed)

        print(f"  ✓ nexrad{i:<2}.bz2: {len(payload):>8} B compressed -> {len(decompressed):>10} B uncompressed ({len(decompressed)/1e6:.2f} MB)")


def verify_all():
    print("\n=== 3. Validating All Test Data Files ===")
    for name in SILESIA_FILES:
        ref_path = os.path.join(REFERENCE_DIR, f"silesia_{name}")
        comp_path = os.path.join(COMPRESSED_DIR, f"silesia_{name}.bz2")
        if not os.path.isfile(ref_path) or not os.path.isfile(comp_path):
            raise FileNotFoundError(f"Missing {name} files")
    for i in range(1, 31):
        ref_path = os.path.join(REFERENCE_DIR, f"nexrad{i}")
        comp_path = os.path.join(COMPRESSED_DIR, f"nexrad{i}.bz2")
        if not os.path.isfile(ref_path) or not os.path.isfile(comp_path):
            raise FileNotFoundError(f"Missing nexrad{i} files")
    print("✓ All 12 Silesia and 30 NEXRAD files verified successfully!")


def main():
    os.makedirs(COMPRESSED_DIR, exist_ok=True)
    os.makedirs(REFERENCE_DIR, exist_ok=True)

    fetch_silesia()
    fetch_nexrad()
    verify_all()
    print("\nTest dataset initialization complete!\n")


if __name__ == "__main__":
    main()
