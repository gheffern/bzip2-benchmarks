#!/usr/bin/env python3
"""Run per-file Silesia benchmark across all 4 commit states and generate comparison metrics.
"""

import json
import os
import subprocess

REPO_ROOT = "/home/gheffern/Projects/bzip2_optimization"
LIB_DIR = os.path.join(REPO_ROOT, "libbzip2-rs")
BENCH_DIR = os.path.join(REPO_ROOT, "benchmarks")
OUTPUT_DIR = os.path.join(BENCH_DIR, "output")

STEPS = [
    {
        "id": "main",
        "commit": "f47b114",
        "title": "Baseline (upstream main v0.2.5)",
        "json_file": "step_0_main.json"
    },
    {
        "id": "commit1_huffman",
        "commit": "23f190f",
        "title": "Commit 1 (Huffman O(N) Table & Branchless RLE)",
        "json_file": "step_1_huffman.json"
    },
    {
        "id": "commit2_compress",
        "commit": "5c43870",
        "title": "Commit 2 (Compress quadrant shift in mainSort)",
        "json_file": "step_2_compress.json"
    },
    {
        "id": "commit3_crc32",
        "commit": "feature/perf-optimizations",
        "title": "Commit 3 (Slice-by-4 CRC32 & Loop Fix)",
        "json_file": "step_3_crc32.json"
    }
]

os.makedirs(OUTPUT_DIR, exist_ok=True)

def run_step(step_info):
    print(f"\n{'='*60}")
    print(f"Running Step: {step_info['title']} ({step_info['commit']})")
    print(f"{'='*60}")

    # Checkout commit in libbzip2-rs
    subprocess.run(["git", "checkout", step_info["commit"]], cwd=LIB_DIR, check=True)

    json_target = f"output/{step_info['json_file']}"
    cmd = [
        "podman", "run", "--rm",
        "-v", f"{REPO_ROOT}:/workspace:Z",
        "-w", "/workspace/benchmarks",
        "localhost/rust-toolchain:latest",
        "bash", "-c",
        f"cargo build --release --bin bench_silesia_breakdown 2>&1 | tail -2 && cargo run --release --bin bench_silesia_breakdown {json_target}"
    ]
    subprocess.run(cmd, check=True)

for s in STEPS:
    run_step(s)

# Restore branch to feature/perf-optimizations
subprocess.run(["git", "checkout", "feature/perf-optimizations"], cwd=LIB_DIR, check=True)

print("\nAll 4 benchmark runs completed!")
