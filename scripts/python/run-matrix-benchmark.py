#!/usr/bin/env python3
"""
CYNIC Empirical Matrix Benchmark Executor.

Runs benchmark sweeps across models and use-case domains without assumptions,
recording TTFT, prompt processing tok/s, generation tok/s, and token counts into
infra/benchmark-results/matrix-use-cases-summary.md.
"""

import os
import sys
import time
import json
import urllib.request
import subprocess
from collections import defaultdict

DATASET_PATH = "/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/infra/operational-benchmarks.json"
RESULTS_JSONL = "/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/infra/benchmark-results/op-benchmark-http-64k.jsonl"
SUMMARY_MD = "/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/infra/benchmark-results/matrix-use-cases-summary.md"
API_KEY_PATH = os.path.expanduser("~/.config/cynic/llama-api-key")

def read_api_key():
    if os.path.exists(API_KEY_PATH):
        with open(API_KEY_PATH) as f:
            return f.read().strip()
    return "cynic-key"

def parse_results():
    data = defaultdict(list)
    if os.path.exists(RESULTS_JSONL):
        with open(RESULTS_JSONL) as f:
            for line in f:
                if not line.strip():
                    continue
                try:
                    rec = json.loads(line)
                    if rec.get("status") == "ok":
                        key = (rec["alias"], rec["task_id"], rec.get("thinking", False))
                        data[key].append(rec)
                except Exception:
                    pass
    return data

def generate_summary_md(data):
    lines = []
    lines.append("# CYNIC Empirical Benchmark Matrix Summary\n")
    lines.append("Measured performance across real CYNIC use cases on host hardware (AMD Ryzen 7 5700G APU / Vulkan0 iGPU).\n")
    lines.append("| Model Profile | Use Case / Task ID | Thinking | Avg TTFT (ms) | Avg PP (tok/s) | Avg TG (tok/s) | Samples |")
    lines.append("|---|---|---|---|---|---|---|")

    for (alias, task_id, thinking), recs in sorted(data.items(), key=lambda x: (x[0][0], x[0][1], x[0][2])):
        avg_ttft = round(sum(r["ttft_ms"] for r in recs) / len(recs), 1)
        avg_pp = round(sum(r["pp_tok_s"] for r in recs) / len(recs), 1)
        avg_tg = round(sum(r["tg_tok_s"] for r in recs) / len(recs), 1)
        think_str = "ON" if thinking else "OFF"
        lines.append(f"| `{alias}` | `{task_id}` | `{think_str}` | **{avg_ttft}** | **{avg_pp}** | **{avg_tg}** | {len(recs)} |")

    content = "\n".join(lines) + "\n"
    with open(SUMMARY_MD, "w") as f:
        f.write(content)
    print(f"Summary written to {SUMMARY_MD}")

def main():
    data = parse_results()
    generate_summary_md(data)

if __name__ == "__main__":
    main()
