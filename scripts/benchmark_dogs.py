#!/usr/bin/env python3
"""
CYNIC Dog Benchmark — calibrate scoring quality against ground truth.

Loads a labeled dataset, submits each entry to POST /judge, compares
the verdict against the ground truth label. Outputs confusion matrix
and per-Dog metrics.

Usage:
    python3 scripts/benchmark_dogs.py data/benchmark.json
    python3 scripts/benchmark_dogs.py data/benchmark.json --dry-run
    python3 scripts/benchmark_dogs.py data/benchmark.json --limit 10
    python3 scripts/benchmark_dogs.py data/benchmark.json --output results.json

Dataset format (JSON array):
    [
      {"mint": "...", "label": "bark", "stimulus": "...", "domain": "token-analysis"},
      {"mint": "...", "label": "wag",  "stimulus": "...", "domain": "trading"},
    ]

Labels: howl | wag | growl | bark (case-insensitive)
"""

import argparse
import json
import os
import sys
import time
import urllib.request
import urllib.error
from collections import Counter, defaultdict
from pathlib import Path

# ── Verdict bands (from .claude/rules/reference.md) ──

HOWL_THRESHOLD = 0.528   # φ⁻² + φ⁻⁴
WAG_THRESHOLD = 0.382    # φ⁻²
GROWL_THRESHOLD = 0.236  # φ⁻³
VERDICT_ORDER = ["Howl", "Wag", "Growl", "Bark"]


def q_to_verdict(q: float) -> str:
    if q > HOWL_THRESHOLD:
        return "Howl"
    if q > WAG_THRESHOLD:
        return "Wag"
    if q > GROWL_THRESHOLD:
        return "Growl"
    return "Bark"


# ── Env loading (same pattern as token_screener.py) ──

def load_cynic_env() -> tuple[str, str]:
    """Load CYNIC_REST_ADDR and CYNIC_API_KEY from env or ~/.cynic-env."""
    addr = os.environ.get("CYNIC_REST_ADDR", "")
    key = os.environ.get("CYNIC_API_KEY", "")
    if addr and key:
        return addr, key
    env_file = Path.home() / ".cynic-env"
    if env_file.exists():
        for line in env_file.read_text().splitlines():
            line = line.strip()
            if line.startswith("#") or "=" not in line:
                continue
            k, _, v = line.partition("=")
            k = k.strip()
            v = v.strip().strip('"').strip("'")
            if k == "CYNIC_REST_ADDR" and not addr:
                addr = v
            elif k == "CYNIC_API_KEY" and not key:
                key = v
    if not addr or not key:
        print("ERROR: CYNIC_REST_ADDR / CYNIC_API_KEY not set", file=sys.stderr)
        sys.exit(1)
    return addr, key


# ── Dataset loading ──

def load_dataset(path: str) -> list[dict]:
    """Load benchmark dataset from JSON file."""
    with open(path) as f:
        data = json.load(f)
    for entry in data:
        entry["label"] = entry["label"].capitalize()
        if entry["label"] not in VERDICT_ORDER:
            print(f"WARN: unknown label '{entry['label']}' for {entry.get('mint','?')}", file=sys.stderr)
    return data


# ── Submit to kernel ──

def submit_to_kernel(stimulus: str, domain: str, addr: str, key: str) -> tuple[dict | None, int]:
    """POST stimulus to /judge, return (response_json, latency_ms)."""
    url = f"http://{addr}/judge" if not addr.startswith("http") else f"{addr}/judge"
    payload = json.dumps({
        "content": stimulus,
        "domain": domain,
        "context": "benchmark: ground truth calibration",
    }).encode()
    req = urllib.request.Request(
        url,
        data=payload,
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {key}",
        },
    )
    t0 = time.monotonic()
    try:
        with urllib.request.urlopen(req, timeout=180) as resp:
            body = json.loads(resp.read())
        latency = int((time.monotonic() - t0) * 1000)
        return body, latency
    except (urllib.error.URLError, json.JSONDecodeError, OSError) as e:
        latency = int((time.monotonic() - t0) * 1000)
        print(f"  ERROR: {e} ({latency}ms)", file=sys.stderr)
        return None, latency


# ── Result extraction ──

def extract_result(entry: dict, response: dict | None, latency_ms: int) -> dict:
    """Flatten a verdict response into a benchmark result row."""
    if response is None:
        return {
            "mint": entry.get("mint", "?"),
            "ground_truth": entry["label"],
            "predicted": "Error",
            "q_score": 0.0,
            "latency_ms": latency_ms,
            "dogs": [],
            "error": True,
        }
    return {
        "mint": entry.get("mint", "?"),
        "ground_truth": entry["label"],
        "predicted": response.get("verdict", "?"),
        "q_score": response.get("q_score", {}).get("total", 0.0),
        "latency_ms": latency_ms,
        "dogs": [
            {
                "dog_id": d["dog_id"],
                "latency_ms": d.get("latency_ms", 0),
                "q": sum(d.get(f"raw_{a}", 0) for a in ["fidelity","phi","verify","culture","burn","sovereignty"]) / 6,
            }
            for d in response.get("dog_scores", [])
        ],
        "max_disagreement": response.get("max_disagreement", 0),
        "anomaly": response.get("anomaly_detected", False),
        "error": False,
    }


# ── Reporting ──

def print_confusion_matrix(results: list[dict]):
    """Print confusion matrix: rows = ground truth, cols = predicted."""
    labels = VERDICT_ORDER + ["Error"]
    matrix = {gt: Counter() for gt in labels}
    for r in results:
        gt = r["ground_truth"]
        pred = r["predicted"]
        if gt in matrix:
            matrix[gt][pred] += 1

    print("\n=== CONFUSION MATRIX ===")
    header = f"{'':>8} | " + " | ".join(f"{v:>6}" for v in labels)
    print(header)
    print("-" * len(header))
    for gt in labels:
        if not any(matrix[gt].values()):
            continue
        row = f"{gt:>8} | " + " | ".join(f"{matrix[gt].get(v, 0):>6}" for v in labels)
        total = sum(matrix[gt].values())
        correct = matrix[gt].get(gt, 0)
        acc = correct / total * 100 if total > 0 else 0
        print(f"{row}  ({acc:.0f}%)")


def print_per_dog_metrics(results: list[dict]):
    """Print per-Dog: avg latency, avg q, call count."""
    dog_stats = defaultdict(lambda: {"lats": [], "qs": [], "n": 0})
    for r in results:
        for d in r.get("dogs", []):
            s = dog_stats[d["dog_id"]]
            s["lats"].append(d["latency_ms"])
            s["qs"].append(d["q"])
            s["n"] += 1

    print("\n=== PER-DOG METRICS ===")
    print(f"{'Dog':>25} | {'Calls':>5} | {'Avg Lat':>8} | {'Avg Q':>6}")
    print("-" * 55)
    for dog_id, s in sorted(dog_stats.items()):
        avg_lat = sum(s["lats"]) / len(s["lats"]) if s["lats"] else 0
        avg_q = sum(s["qs"]) / len(s["qs"]) if s["qs"] else 0
        print(f"{dog_id:>25} | {s['n']:>5} | {avg_lat:>7.0f}ms | {avg_q:>5.3f}")


def print_accuracy(results: list[dict]):
    """Print accuracy per ground truth label."""
    by_label = defaultdict(lambda: {"correct": 0, "total": 0})
    for r in results:
        gt = r["ground_truth"]
        by_label[gt]["total"] += 1
        if r["predicted"] == gt:
            by_label[gt]["correct"] += 1

    print("\n=== ACCURACY BY LABEL ===")
    total_correct = sum(v["correct"] for v in by_label.values())
    total_all = sum(v["total"] for v in by_label.values())
    for label in VERDICT_ORDER:
        s = by_label[label]
        if s["total"] == 0:
            continue
        acc = s["correct"] / s["total"] * 100
        bar = "█" * int(acc / 5) + "░" * (20 - int(acc / 5))
        print(f"  {label:>5}: {bar} {acc:.0f}% ({s['correct']}/{s['total']})")
    if total_all > 0:
        print(f"  {'TOTAL':>5}: {total_correct}/{total_all} = {total_correct/total_all*100:.1f}%")


def print_summary(results: list[dict]):
    """Print overall summary."""
    errors = sum(1 for r in results if r["error"])
    lats = [r["latency_ms"] for r in results if not r["error"]]
    print(f"\n=== SUMMARY ===")
    print(f"  Entries:  {len(results)}")
    print(f"  Errors:   {errors}")
    if lats:
        print(f"  Latency:  avg={sum(lats)/len(lats):.0f}ms, max={max(lats)}ms, min={min(lats)}ms")


# ── Main ──

def main():
    parser = argparse.ArgumentParser(description="CYNIC Dog Benchmark")
    parser.add_argument("dataset", help="Path to benchmark dataset (JSON)")
    parser.add_argument("--dry-run", action="store_true", help="Print stimuli without submitting")
    parser.add_argument("--limit", type=int, default=0, help="Process only first N entries")
    parser.add_argument("--output", type=str, help="Save results as JSON")
    parser.add_argument("--delay", type=float, default=2.0, help="Delay between requests (seconds)")
    args = parser.parse_args()

    dataset = load_dataset(args.dataset)
    if args.limit > 0:
        dataset = dataset[:args.limit]

    print(f"Loaded {len(dataset)} entries from {args.dataset}")
    label_dist = Counter(e["label"] for e in dataset)
    print(f"Labels: {dict(label_dist)}")

    if args.dry_run:
        for e in dataset:
            print(f"\n--- {e.get('mint','?')} [{e['label']}] ---")
            print(e.get("stimulus", "(no stimulus)")[:300])
        return

    addr, key = load_cynic_env()
    print(f"Kernel: {addr}")
    print()

    results = []
    for i, entry in enumerate(dataset):
        stimulus = entry.get("stimulus", "")
        domain = entry.get("domain", "token-analysis")
        mint = entry.get("mint", "?")
        print(f"[{i+1}/{len(dataset)}] {mint[:12]}... [{entry['label']}] ", end="", flush=True)

        response, latency = submit_to_kernel(stimulus, domain, addr, key)
        result = extract_result(entry, response, latency)
        results.append(result)

        pred = result["predicted"]
        match = "✓" if pred == entry["label"] else "✗"
        print(f"→ {pred} (Q={result['q_score']:.3f}, {latency}ms) {match}")

        if i < len(dataset) - 1:
            time.sleep(args.delay)

    print_summary(results)
    print_confusion_matrix(results)
    print_per_dog_metrics(results)
    print_accuracy(results)

    if args.output:
        Path(args.output).write_text(json.dumps(results, indent=2))
        print(f"\nResults saved to {args.output}")


if __name__ == "__main__":
    main()
