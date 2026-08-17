#!/usr/bin/env python3
"""Fix benchmark data files:
1. Rename Silesia reference files to add silesia_ prefix
2. Regenerate Silesia .bz2 files from (correct) full reference files
3. Regenerate NEXRAD reference files with full multi-stream decompression
"""

import os
import bz2
import struct
import shutil

DATA_DIR = "data"
COMPRESSED_DIR = os.path.join(DATA_DIR, "compressed")
REFERENCE_DIR = os.path.join(DATA_DIR, "reference")

SILESIA_FILES = [
    "dickens", "mozilla", "mr", "nci", "ooffice",
    "osdb", "reymont", "samba", "sao", "webster", "xml", "x-ray",
]


def rename_silesia_references():
    """Rename reference files from {name} to silesia_{name}."""
    print("=== Step 1: Renaming Silesia reference files ===")
    for name in SILESIA_FILES:
        old_path = os.path.join(REFERENCE_DIR, name)
        new_path = os.path.join(REFERENCE_DIR, f"silesia_{name}")
        if os.path.exists(old_path) and not os.path.exists(new_path):
            os.rename(old_path, new_path)
            print(f"  Renamed: {name} -> silesia_{name}")
        elif os.path.exists(new_path):
            print(f"  Already exists: silesia_{name}")
            # Clean up old file if both exist
            if os.path.exists(old_path):
                os.remove(old_path)
                print(f"  Removed duplicate: {name}")
        else:
            print(f"  WARNING: {name} not found in reference dir!")


def regenerate_silesia_compressed():
    """Compress full Silesia reference files to .bz2."""
    print("\n=== Step 2: Regenerating Silesia .bz2 files ===")
    for name in SILESIA_FILES:
        ref_path = os.path.join(REFERENCE_DIR, f"silesia_{name}")
        bz2_path = os.path.join(COMPRESSED_DIR, f"silesia_{name}.bz2")

        if not os.path.exists(ref_path):
            print(f"  SKIP: {ref_path} not found")
            continue

        ref_data = open(ref_path, "rb").read()
        print(f"  Compressing silesia_{name}: {len(ref_data)} bytes...", end="", flush=True)

        # Use compresslevel=9 to match the benchmark's blockSize100k=9
        compressed = bz2.compress(ref_data, compresslevel=9)
        with open(bz2_path, "wb") as f:
            f.write(compressed)

        print(f" -> {len(compressed)} bytes ({len(compressed)/len(ref_data)*100:.1f}%)")

        # Verify round-trip
        decompressed = bz2.decompress(compressed)
        assert decompressed == ref_data, f"Round-trip failed for {name}!"


def decompress_nexrad_multi_stream(data):
    """Decompress a NEXRAD Level 2 archive (multi-stream bzip2 with 4-byte headers).

    Format: first stream starts at offset 0, subsequent streams are preceded
    by a 4-byte big-endian signed int32 (the compressed size of that stream).
    """
    result = bytearray()
    offset = 0
    stream_count = 0

    while offset < len(data):
        # First stream has no header, subsequent streams have a 4-byte header
        if stream_count > 0:
            if offset + 4 > len(data):
                break
            # Read the 4-byte header (compressed stream length)
            header_val = struct.unpack(">i", data[offset:offset+4])[0]
            offset += 4

            if header_val < 0:
                # Negative value can indicate end-of-volume or special marker
                break

        # Check for BZh magic
        if offset + 3 > len(data) or data[offset:offset+3] != b"BZh":
            break

        # Decompress this stream
        try:
            decompressor = bz2.BZ2Decompressor()
            chunk = decompressor.decompress(data[offset:])
            consumed = len(data) - offset - len(decompressor.unused_data)
            result.extend(chunk)
            offset += consumed
            stream_count += 1
        except Exception as e:
            print(f"    Warning: stream {stream_count} failed: {e}")
            break

    return bytes(result), stream_count


def regenerate_nexrad_references():
    """Decompress all bz2 streams from NEXRAD archives to create full references."""
    print("\n=== Step 3: Regenerating NEXRAD reference files (full multi-stream) ===")
    for i in range(1, 31):
        bz2_path = os.path.join(COMPRESSED_DIR, f"nexrad{i}.bz2")
        ref_path = os.path.join(REFERENCE_DIR, f"nexrad{i}")

        if not os.path.exists(bz2_path):
            print(f"  SKIP: {bz2_path} not found")
            continue

        data = open(bz2_path, "rb").read()
        decompressed, stream_count = decompress_nexrad_multi_stream(data)

        old_ref_size = os.path.getsize(ref_path) if os.path.exists(ref_path) else 0
        with open(ref_path, "wb") as f:
            f.write(decompressed)

        print(f"  nexrad{i}: {stream_count} streams, {len(data)} compressed -> "
              f"{len(decompressed)} decompressed (was {old_ref_size})")


def verify_all():
    """Final verification of all data files."""
    print("\n=== Final Verification ===")

    # Check Silesia
    silesia_ok = True
    for name in SILESIA_FILES:
        ref_path = os.path.join(REFERENCE_DIR, f"silesia_{name}")
        bz2_path = os.path.join(COMPRESSED_DIR, f"silesia_{name}.bz2")
        if not os.path.exists(ref_path):
            print(f"  FAIL: {ref_path} missing")
            silesia_ok = False
            continue
        if not os.path.exists(bz2_path):
            print(f"  FAIL: {bz2_path} missing")
            silesia_ok = False
            continue
        ref_data = open(ref_path, "rb").read()
        decomp_data = bz2.decompress(open(bz2_path, "rb").read())
        if ref_data != decomp_data:
            print(f"  FAIL: silesia_{name} round-trip mismatch!")
            silesia_ok = False
        else:
            print(f"  OK: silesia_{name} ({len(ref_data)} bytes)")
    if silesia_ok:
        print(f"  ✓ All {len(SILESIA_FILES)} Silesia files verified\n")

    # Check NEXRAD
    nexrad_ok = True
    for i in range(1, 31):
        ref_path = os.path.join(REFERENCE_DIR, f"nexrad{i}")
        bz2_path = os.path.join(COMPRESSED_DIR, f"nexrad{i}.bz2")
        if not os.path.exists(ref_path) or not os.path.exists(bz2_path):
            print(f"  FAIL: nexrad{i} files missing")
            nexrad_ok = False
            continue
        ref_data = open(ref_path, "rb").read()
        bz2_data = open(bz2_path, "rb").read()
        decomp, _ = decompress_nexrad_multi_stream(bz2_data)
        if ref_data != decomp:
            print(f"  FAIL: nexrad{i} mismatch!")
            nexrad_ok = False
        else:
            ref_mb = len(ref_data) / 1_000_000
            print(f"  OK: nexrad{i} ({ref_mb:.2f} MB)")
    if nexrad_ok:
        print(f"  ✓ All 30 NEXRAD files verified")


if __name__ == "__main__":
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    rename_silesia_references()
    regenerate_silesia_compressed()
    regenerate_nexrad_references()
    verify_all()
    print("\nDone!")
