#!/usr/bin/env python3
"""Automated orchestrator for bzip2 optimization benchmarks.

Supports:
- A/B mode (default): compares `main` baseline vs active branch/commit.
- Stepped mode (`--stepped`): evaluates every commit on the branch sequentially.
Generates comprehensive Markdown comparison reports in benchmarks/output/benchmark_report.md.
"""

import argparse
import json
import os
import shutil
import subprocess
import sys

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.dirname(SCRIPT_DIR)
LIB_DIR = os.path.join(REPO_ROOT, "libbzip2-rs")
OUTPUT_DIR = os.path.join(SCRIPT_DIR, "output")
REPORT_MD = os.path.join(OUTPUT_DIR, "benchmark_report.md")


def get_current_branch_or_commit():
    res = subprocess.run(["git", "rev-parse", "--abbrev-ref", "HEAD"], cwd=LIB_DIR, capture_output=True, text=True)
    branch = res.stdout.strip()
    if branch == "HEAD":
        res2 = subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=LIB_DIR, capture_output=True, text=True)
        return res2.stdout.strip()
    return branch


def get_commits_on_branch():
    """Get list of commits between origin/main and HEAD."""
    res = subprocess.run(["git", "log", "--reverse", "--oneline", "origin/main..HEAD"], cwd=LIB_DIR, capture_output=True, text=True)
    lines = [l.strip() for l in res.stdout.strip().splitlines() if l.strip()]
    commits = []
    for l in lines:
        parts = l.split(" ", 1)
        commits.append({"hash": parts[0], "title": parts[1] if len(parts) > 1 else ""})
    return commits


def run_benchmark_for_ref(ref_target, json_filename, iterations):
    print(f"\n{'='*70}")
    print(f"Benchmarking Git Ref: {ref_target} ({iterations} iterations)")
    print(f"{'='*70}")

    subprocess.run(["git", "checkout", ref_target], cwd=LIB_DIR, check=True)
    json_path = os.path.join(OUTPUT_DIR, json_filename)

    cmd = [
        "cargo", "run", "--release", "--bin", "bench_suite", "--",
        "--iterations", str(iterations),
        "--json", f"output/{json_filename}"
    ]
    
    subprocess.run(cmd, cwd=SCRIPT_DIR, check=True)
    return json.load(open(json_path))


def format_ab_report(baseline_json, target_json, target_name):
    base_nex = baseline_json["nexrad"]
    tgt_nex = target_json["nexrad"]
    base_sil = baseline_json["silesia_aggregate"]
    tgt_sil = target_json["silesia_aggregate"]

    nex_decomp_d = ((tgt_nex["decomp_mb_s"] - base_nex["decomp_mb_s"]) / base_nex["decomp_mb_s"]) * 100
    nex_comp_d = ((tgt_nex["comp_mb_s"] - base_nex["comp_mb_s"]) / base_nex["comp_mb_s"]) * 100
    sil_decomp_d = ((tgt_sil["decomp_mb_s"] - base_sil["decomp_mb_s"]) / base_sil["decomp_mb_s"]) * 100
    sil_comp_d = ((tgt_sil["comp_mb_s"] - base_sil["comp_mb_s"]) / base_sil["comp_mb_s"]) * 100

    def format_speedup(d):
        icon = " 🚀" if d > 1.0 else ""
        return f"**{d:+.1f}%{icon}**"

    report = []
    report.append(f"# Benchmark A/B Report: `origin/main` vs `{target_name}`\n")
    report.append("## 1. Overall Aggregate Throughput\n")
    report.append("| Dataset | Operation | Baseline (`main`) | Target (`" + target_name + "`) | Throughput Delta | Speedup |")
    report.append("| :--- | :--- | :--- | :--- | :--- | :--- |")
    report.append(f"| **NEXRAD Radar** | Decompression | {base_nex['decomp_mb_s']:.2f} MB/s | **{tgt_nex['decomp_mb_s']:.2f} MB/s** | {tgt_nex['decomp_mb_s'] - base_nex['decomp_mb_s']:+.2f} MB/s | {format_speedup(nex_decomp_d)} |")
    report.append(f"| **NEXRAD Radar** | Compression | {base_nex['comp_mb_s']:.2f} MB/s | **{tgt_nex['comp_mb_s']:.2f} MB/s** | {tgt_nex['comp_mb_s'] - base_nex['comp_mb_s']:+.2f} MB/s | {format_speedup(nex_comp_d)} |")
    report.append(f"| **Silesia Corpus** | Decompression | {base_sil['decomp_mb_s']:.2f} MB/s | **{tgt_sil['decomp_mb_s']:.2f} MB/s** | {tgt_sil['decomp_mb_s'] - base_sil['decomp_mb_s']:+.2f} MB/s | {format_speedup(sil_decomp_d)} |")
    report.append(f"| **Silesia Corpus** | Compression | {base_sil['comp_mb_s']:.2f} MB/s | **{tgt_sil['comp_mb_s']:.2f} MB/s** | {tgt_sil['comp_mb_s'] - base_sil['comp_mb_s']:+.2f} MB/s | {format_speedup(sil_comp_d)} |")

    report.append("\n## 2. Silesia Decompression Performance\n")
    report.append("| File Name | Data Type Category | Size | Baseline (`main`) | Target (`" + target_name + "`) | Speedup |")
    report.append("| :--- | :--- | :--- | :--- | :--- | :--- |")

    base_files = {f["name"]: f for f in baseline_json["silesia_files"]}
    for tf in target_json["silesia_files"]:
        name = tf["name"]
        bf = base_files.get(name, tf)
        fname = name.replace("silesia_", "")
        sz = tf["uncomp_bytes"] / 1e6
        d_decomp = ((tf["decomp_mb_s"] - bf["decomp_mb_s"]) / bf["decomp_mb_s"]) * 100
        report.append(f"| **`{fname}`** | {tf['type']} | {sz:.2f} MB | {bf['decomp_mb_s']:.2f} MB/s | **{tf['decomp_mb_s']:.2f} MB/s** | {format_speedup(d_decomp)} |")

    report.append("\n## 3. Silesia Compression Performance\n")
    report.append("| File Name | Data Type Category | Size | Ratio | Baseline (`main`) | Target (`" + target_name + "`) | Speedup |")
    report.append("| :--- | :--- | :--- | :--- | :--- | :--- | :--- |")

    for tf in target_json["silesia_files"]:
        name = tf["name"]
        bf = base_files.get(name, tf)
        fname = name.replace("silesia_", "")
        sz = tf["uncomp_bytes"] / 1e6
        ratio = (tf["comp_bytes"] / tf["uncomp_bytes"]) * 100
        d_comp = ((tf["comp_mb_s"] - bf["comp_mb_s"]) / bf["comp_mb_s"]) * 100
        report.append(f"| **`{fname}`** | {tf['type']} | {sz:.2f} MB | {ratio:.2f}% | {bf['comp_mb_s']:.2f} MB/s | **{tf['comp_mb_s']:.2f} MB/s** | {format_speedup(d_comp)} |")

    return "\n".join(report)


def format_stepped_report(step_results):
    report = ["# Silesia & NEXRAD Stepped Commit Benchmark Report\n"]
    base = step_results[0]["json"]

    report.append("## 1. Aggregate Progress Across Commits\n")
    report.append("| Step / Git Commit | NEXRAD Decomp | NEXRAD Comp | Silesia Decomp | Silesia Comp |")
    report.append("| :--- | :--- | :--- | :--- | :--- |")

    for s in step_results:
        j = s["json"]
        title = s["title"]
        report.append(f"| **{title}** | {j['nexrad']['decomp_mb_s']:.2f} MB/s | {j['nexrad']['comp_mb_s']:.2f} MB/s | {j['silesia_aggregate']['decomp_mb_s']:.2f} MB/s | {j['silesia_aggregate']['comp_mb_s']:.2f} MB/s |")

    report.append("\n## 2. Silesia Per-File Decompression Trajectory (MB/s & Total Δ%)\n")
    headers = ["File Name", "Type"]
    for s in step_results:
        headers.append(s["short_title"])
    headers.append("Total Δ%")
    report.append("| " + " | ".join(headers) + " |")
    report.append("| " + " | ".join([":---"] * len(headers)) + " |")

    files_count = len(base["silesia_files"])
    for fi in range(files_count):
        fname = base["silesia_files"][fi]["name"].replace("silesia_", "")
        ftype = base["silesia_files"][fi]["type"]
        row = [f"**`{fname}`**", ftype]
        base_val = base["silesia_files"][fi]["decomp_mb_s"]
        final_val = step_results[-1]["json"]["silesia_files"][fi]["decomp_mb_s"]

        for s in step_results:
            v = s["json"]["silesia_files"][fi]["decomp_mb_s"]
            row.append(f"{v:.2f} MB/s")

        total_d = ((final_val - base_val) / base_val) * 100
        icon = "🚀" if total_d > 5.0 else ""
        row.append(f"**{total_d:+.1f}% {icon}**")
        report.append("| " + " | ".join(row) + " |")

    return "\n".join(report)


def main():
    parser = argparse.ArgumentParser(description="bzip2 Benchmark Suite Orchestrator")
    parser.add_argument("--stepped", action="store_true", help="Step through all commits on the branch")
    parser.add_argument("--iterations", type=int, default=20, help="Number of iterations per test (default: 20)")
    args = parser.parse_args()

    os.makedirs(OUTPUT_DIR, exist_ok=True)
    original_ref = get_current_branch_or_commit()
    print(f"Current branch/commit: {original_ref}")

    try:
        if args.stepped:
            commits = get_commits_on_branch()
            steps = [{"ref": "origin/main", "title": "Baseline (origin/main v0.2.5)", "short_title": "Main (v0.2.5)", "json_file": "step_0_main.json"}]
            for idx, c in enumerate(commits, 1):
                steps.append({
                    "ref": c["hash"],
                    "title": f"Commit {idx}: {c['title']} ({c['hash']})",
                    "short_title": f"Commit {idx} ({c['hash']})",
                    "json_file": f"step_{idx}_{c['hash']}.json"
                })

            step_results = []
            for s in steps:
                j = run_benchmark_for_ref(s["ref"], s["json_file"], args.iterations)
                step_results.append({**s, "json": j})

            report_content = format_stepped_report(step_results)
        else:
            # A/B mode
            base_json = run_benchmark_for_ref("origin/main", "ab_baseline_main.json", args.iterations)
            target_json = run_benchmark_for_ref(original_ref, "ab_target.json", args.iterations)
            report_content = format_ab_report(base_json, target_json, original_ref)

        with open(REPORT_MD, "w") as f:
            f.write(report_content)

        print("\n" + "="*70)
        print(report_content)
        print("="*70)
        print(f"\n✓ Markdown benchmark report saved to: {REPORT_MD}")

    finally:
        # Always restore original git branch
        print(f"\nRestoring git branch to: {original_ref}")
        subprocess.run(["git", "checkout", original_ref], cwd=LIB_DIR, capture_output=True)


if __name__ == "__main__":
    main()
