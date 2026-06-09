#!/usr/bin/env python3
"""Simple A/B test using curl and jq (no requests library dependency)."""

import os
import json
import subprocess
import sys
import time
from pathlib import Path
from typing import Optional, Dict, Tuple

KERNEL_URL = os.getenv("CYNIC_REST_ADDR", "http://localhost:3030")
if not KERNEL_URL.startswith("http"):
    KERNEL_URL = f"http://{KERNEL_URL}"
API_KEY = os.getenv("CYNIC_API_KEY", "")

def call_judge(mint: str, domain: Optional[str] = None) -> Optional[Dict]:
    """Call /judge endpoint using curl."""
    payload = {"content": mint, "dogs": ["deterministic-dog"]}
    if domain:
        payload["domain"] = domain

    cmd = [
        "curl", "-s", "-X", "POST", f"{KERNEL_URL}/judge",
        "-H", f"Authorization: Bearer {API_KEY}",
        "-H", "Content-Type: application/json",
        "-d", json.dumps(payload),
    ]

    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
        if result.returncode != 0:
            return None
        resp = json.loads(result.stdout)
        return resp
    except Exception:
        return None

def verdict_from_qscore(q_score: float) -> str:
    """Map Q-score to verdict."""
    if q_score > 0.528:
        return "Howl"
    elif q_score > 0.382:
        return "Wag"
    elif q_score > 0.236:
        return "Growl"
    else:
        return "Bark"

def main():
    """Run A/B test."""
    print("=" * 80)
    print("A/B TEST: Divergence Enrichment Impact")
    print("=" * 80)
    print(f"Baseline:  /judge without domain")
    print(f"Enriched:  /judge with domain='token-analysis'")
    print(f"Dog: deterministic-dog only")
    print("=" * 80)

    # Load calibration data
    data_file = Path(__file__).parent / "calibration_results_real.json"
    if not data_file.exists():
        print(f"ERROR: {data_file} not found")
        sys.exit(1)

    with open(data_file) as f:
        calib = json.load(f)

    tokens = calib["results"][:33]  # All calibration tokens
    print(f"\nTesting {len(tokens)} tokens...\n")

    results = []
    baseline_correct = 0
    enriched_correct = 0
    improvements = 0
    regressions = 0

    for i, token in enumerate(tokens):
        mint = token["mint"]
        symbol = token["symbol"].strip()
        conviction = token["conviction"]
        tier = token["conviction_tier"]
        expected = token["expected_verdict"]

        sys.stdout.write(f"[{i+1:2d}/{len(tokens)}] {symbol:15s} (conv={conviction:.3f})... ")
        sys.stdout.flush()

        # Baseline
        baseline_resp = call_judge(mint)
        if not baseline_resp or "q_score" not in baseline_resp:
            print("SKIP (baseline failed)")
            continue

        baseline_q = baseline_resp["q_score"]["total"]
        baseline_v = verdict_from_qscore(baseline_q)
        baseline_match = baseline_v == expected
        if baseline_match:
            baseline_correct += 1

        sys.stdout.write(f"B:{baseline_v} ")
        sys.stdout.flush()
        time.sleep(0.1)

        # Enriched
        enriched_resp = call_judge(mint, domain="token-analysis")
        if not enriched_resp or "q_score" not in enriched_resp:
            print("SKIP (enriched failed)")
            continue

        enriched_q = enriched_resp["q_score"]["total"]
        enriched_v = verdict_from_qscore(enriched_q)
        enriched_match = enriched_v == expected
        if enriched_match:
            enriched_correct += 1

        sys.stdout.write(f"E:{enriched_v} ")

        improvement = enriched_match and not baseline_match
        if improvement:
            improvements += 1
            print("✓ IMPROVED")
        elif not enriched_match and baseline_match:
            regressions += 1
            print("✗ REGRESSED")
        else:
            print()

        results.append({
            "mint": mint,
            "symbol": symbol,
            "conviction": conviction,
            "tier": tier,
            "expected": expected,
            "baseline_verdict": baseline_v,
            "baseline_q": baseline_q,
            "baseline_match": baseline_match,
            "enriched_verdict": enriched_v,
            "enriched_q": enriched_q,
            "enriched_match": enriched_match,
            "improvement": improvement,
        })

        time.sleep(0.1)

    # Results
    print("\n" + "=" * 80)
    print("RESULTS")
    print("=" * 80)

    n = len(results)
    if n == 0:
        print("ERROR: No successful test runs")
        sys.exit(1)

    baseline_acc = baseline_correct / n
    enriched_acc = enriched_correct / n
    improvement_pct = (enriched_acc - baseline_acc) * 100

    print(f"N={n}")
    print(f"\nBaseline accuracy:  {baseline_correct}/{n} = {baseline_acc:.1%}")
    print(f"Enriched accuracy:  {enriched_correct}/{n} = {enriched_acc:.1%}")
    print(f"Absolute improvement: {improvement_pct:+.1f}%")
    print(f"\nImprovements: {improvements}/{n}")
    print(f"Regressions:  {regressions}/{n}")

    # Breakdown by tier
    print("\n" + "=" * 80)
    print("BREAKDOWN BY CONVICTION TIER")
    print("=" * 80)

    for tier_name in ["strong", "mixed", "weak"]:
        tier_results = [r for r in results if r["tier"] == tier_name]
        if not tier_results:
            continue

        baseline_tier = sum(1 for r in tier_results if r["baseline_match"])
        enriched_tier = sum(1 for r in tier_results if r["enriched_match"])
        improvements_tier = sum(1 for r in tier_results if r["improvement"])

        print(f"\n{tier_name.upper()} ({len(tier_results)} tokens):")
        print(f"  Baseline: {baseline_tier}/{len(tier_results)} = {baseline_tier/len(tier_results):.1%}")
        print(f"  Enriched: {enriched_tier}/{len(tier_results)} = {enriched_tier/len(tier_results):.1%}")
        print(f"  Improvements: {improvements_tier}")

    # Save results
    output_file = Path(__file__).parent / f"ab_test_results.json"
    with open(output_file, "w") as f:
        json.dump({
            "n": n,
            "baseline_accuracy": baseline_acc,
            "enriched_accuracy": enriched_acc,
            "absolute_improvement_pct": improvement_pct,
            "improvements": improvements,
            "regressions": regressions,
            "results": results,
        }, f, indent=2)

    print(f"\nResults saved to {output_file}")

    # Hypothesis
    print("\n" + "=" * 80)
    print("HYPOTHESIS TEST")
    print("=" * 80)

    if enriched_acc > baseline_acc:
        print(f"✓ HYPOTHESIS SUPPORTED")
        print(f"  Divergence enrichment improved accuracy by {improvement_pct:+.1f}%")
    elif enriched_acc == baseline_acc:
        print(f"⊘ INCONCLUSIVE")
        print(f"  No difference between baseline and enriched")
    else:
        print(f"✗ HYPOTHESIS REJECTED")
        print(f"  Divergence enrichment decreased accuracy by {-improvement_pct:.1f}%")

if __name__ == "__main__":
    main()
