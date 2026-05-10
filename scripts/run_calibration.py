#!/usr/bin/env python3
"""Run calibration benchmark on 50-token corpus via kernel /judge endpoint.

Usage: python3 scripts/run_calibration.py [--defi-only]

Measures accuracy per category, comparing predicted verdicts to ground truth.
Requires kernel running at CYNIC_REST_ADDR with HELIUS enrichment active.
"""

import json
import os
import sys
import time
from pathlib import Path

import requests

CORPUS_PATH = Path.home() / ".cynic/datasets/tokens/calibration_corpus.json"
RESULTS_PATH = Path.home() / ".cynic/datasets/tokens/calibration_results_AFTER.json"
REST_ADDR = os.environ.get("CYNIC_REST_ADDR", "http://localhost:3030")
API_KEY = os.environ.get("CYNIC_API_KEY", "")

# Q-score → verdict mapping (from phi constants)
def q_to_verdict(q: float) -> str:
    if q > 0.528:
        return "HOWL"
    elif q > 0.382:
        return "WAG"
    elif q > 0.236:
        return "GROWL"
    else:
        return "BARK"

def judge_token(mint: str) -> dict:
    """Call /judge on a single token mint address."""
    headers = {"Content-Type": "application/json"}
    if API_KEY:
        headers["Authorization"] = f"Bearer {API_KEY}"

    payload = {
        "content": mint,
        "domain": "token-analysis"
    }

    try:
        resp = requests.post(f"{REST_ADDR}/judge", json=payload, headers=headers, timeout=60)
        if resp.status_code == 200:
            return resp.json()
        else:
            print(f"  ERROR: {resp.status_code} — {resp.text[:100]}", file=sys.stderr)
            return {"error": resp.status_code}
    except Exception as e:
        print(f"  ERROR: {e}", file=sys.stderr)
        return {"error": str(e)}

def main():
    defi_only = "--defi-only" in sys.argv

    with open(CORPUS_PATH) as f:
        corpus = json.load(f)

    if defi_only:
        corpus = [t for t in corpus if t["category"] == "DEFI"]
        print(f"Running DEFI-only calibration ({len(corpus)} tokens)")
    else:
        print(f"Running full calibration ({len(corpus)} tokens)")

    results = []

    for i, token in enumerate(corpus):
        symbol = token["symbol"]
        mint = token["mint"]
        category = token["category"]
        expected = token["ground_truth_verdict"]

        print(f"[{i+1}/{len(corpus)}] {symbol:12s} ({category}) ...", end=" ", flush=True)

        result = judge_token(mint)

        if "error" in result:
            predicted = "ERROR"
            q_score = 0.0
            print(f"ERROR")
        else:
            q_score = result.get("q_score", 0.0)
            predicted = q_to_verdict(q_score)
            match = "✓" if predicted == expected else "✗"
            print(f"q={q_score:.3f} → {predicted:5s} (exp: {expected:5s}) {match}")

        results.append({
            "symbol": symbol,
            "mint": mint,
            "category": category,
            "expected_verdict": expected,
            "predicted_verdict": predicted,
            "q_score": q_score,
            "match": predicted == expected,
            "token_data": result.get("token_data"),
            "full_result": result,
        })

        # Rate limit (DexScreener 60/min)
        time.sleep(1.5)

    # Save results
    with open(RESULTS_PATH, "w") as f:
        json.dump(results, f, indent=2)
    print(f"\nResults saved to {RESULTS_PATH}")

    # Summary
    print("\n" + "=" * 60)
    print("CALIBRATION RESULTS")
    print("=" * 60)

    from collections import defaultdict
    by_cat = defaultdict(list)
    for r in results:
        by_cat[r["category"]].append(r)

    total_match = 0
    total_count = 0
    for cat in ["RUG", "DECLINING", "BORDERLINE", "MEMECOIN", "DEFI"]:
        items = by_cat.get(cat, [])
        matches = sum(1 for r in items if r["match"])
        total = len(items)
        total_match += matches
        total_count += total
        pct = matches / total * 100 if total > 0 else 0
        print(f"  {cat:12s}: {matches}/{total} = {pct:.0f}%")

    print(f"  {'TOTAL':12s}: {total_match}/{total_count} = {total_match/total_count*100:.0f}%")

if __name__ == "__main__":
    main()
