#!/usr/bin/env python3
"""A/B test: divergence enrichment impact on token judgment accuracy.

Baseline: /judge without domain hint
Enriched: /judge with domain="token-analysis" (includes divergence signal)

Ground truth: CultScreener conviction tiers from calibration_results_real.json

Hypothesis: Divergence enrichment (buy/sell ratio, distribution/accumulation signals)
improves verdict accuracy by disambiguating ambiguous tokens.

Usage:
    python ab_test_divergence_enrichment.py [--limit 33]
"""

import os
import sys
import json
import time
import requests
from typing import Optional, Dict, List
from dataclasses import dataclass, asdict
from pathlib import Path

def _load_env():
    for env_file in [
        Path.home() / ".cynic-env",
        Path(__file__).parent / ".env",
    ]:
        if env_file.exists():
            with open(env_file) as f:
                for line in f:
                    line = line.strip()
                    if line and not line.startswith('#') and '=' in line:
                        key, value = line.split('=', 1)
                        key = key.replace('export ', '').strip()
                        if key not in os.environ:
                            os.environ[key] = value.strip()

_load_env()

CYNIC_REST_ADDR = os.getenv("CYNIC_REST_ADDR", "http://localhost:3030")
CYNIC_API_KEY = os.getenv("CYNIC_API_KEY", "")

@dataclass
class TestResult:
    """Single A/B test result."""
    mint: str
    symbol: str
    conviction: float
    conviction_tier: str
    expected_verdict: str

    # Baseline (no domain hint)
    baseline_verdict: str
    baseline_q_score: float
    baseline_axioms: Dict[str, float]
    baseline_match: bool

    # Enriched (domain=token-analysis with divergence)
    enriched_verdict: str
    enriched_q_score: float
    enriched_axioms: Dict[str, float]
    enriched_match: bool

    # Improvement
    q_score_delta: float
    improvement: bool  # enriched_match and not baseline_match

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

def call_judge(content: str, domain: Optional[str] = None) -> Optional[Dict]:
    """Call kernel /judge endpoint with deterministic-dog only."""
    try:
        payload = {
            "content": content,
            "dogs": ["deterministic-dog"],  # Use only deterministic dog for clean A/B test
        }
        if domain:
            payload["domain"] = domain

        headers = {}
        if CYNIC_API_KEY:
            headers["Authorization"] = f"Bearer {CYNIC_API_KEY}"

        resp = requests.post(
            f"{CYNIC_REST_ADDR}/judge",
            json=payload,
            headers=headers,
            timeout=10,  # Deterministic dog is <1ms, 10s is plenty
        )

        if resp.status_code != 200:
            print(f" ERROR {resp.status_code}", end="")
            return None

        return resp.json()
    except Exception as e:
        print(f" TIMEOUT", end="")
        return None

def run_ab_test(limit: int = 33):
    """Run A/B test on calibration corpus."""
    print("=" * 80)
    print("A/B TEST: Divergence Enrichment Impact")
    print("=" * 80)
    print(f"Baseline: /judge without domain hint")
    print(f"Enriched: /judge with domain='token-analysis' (divergence signal)")
    print(f"Ground truth: CultScreener conviction tiers")
    print("=" * 80)

    # Load calibration data
    calib_file = Path(__file__).parent / "calibration_results_real.json"
    if not calib_file.exists():
        print(f"ERROR: {calib_file} not found")
        sys.exit(1)

    with open(calib_file) as f:
        calib_data = json.load(f)

    tokens = calib_data["results"][:limit]
    print(f"\nLoaded {len(tokens)} tokens from calibration corpus\n")

    results: List[TestResult] = []
    baseline_correct = 0
    enriched_correct = 0
    improvements = 0
    regressions = 0

    for i, token in enumerate(tokens):
        mint = token["mint"]
        symbol = token["symbol"].strip()
        conviction = token["conviction"]
        conviction_tier = token["conviction_tier"]
        expected = token["expected_verdict"]

        # Baseline: no domain hint
        sys.stdout.write(f"[{i+1:2d}/{len(tokens)}] {symbol:15s} (conv={conviction:.3f})...")
        sys.stdout.flush()

        baseline_resp = call_judge(mint)
        if not baseline_resp or "verdict" not in baseline_resp:
            print(" SKIP (baseline failed)")
            continue

        baseline_verdict = verdict_from_qscore(baseline_resp.get("q_score", 0))
        baseline_q_score = baseline_resp.get("q_score", 0)
        baseline_axioms = baseline_resp.get("axioms", {})
        baseline_match = baseline_verdict == expected

        print(f" B:{baseline_verdict}", end="")

        time.sleep(0.5)  # Rate limit

        # Enriched: with domain=token-analysis
        enriched_resp = call_judge(mint, domain="token-analysis")
        if not enriched_resp or "verdict" not in enriched_resp:
            print(" SKIP (enriched failed)")
            continue

        enriched_verdict = verdict_from_qscore(enriched_resp.get("q_score", 0))
        enriched_q_score = enriched_resp.get("q_score", 0)
        enriched_axioms = enriched_resp.get("axioms", {})
        enriched_match = enriched_verdict == expected

        print(f" E:{enriched_verdict}", end="")

        # Score and track
        if baseline_match:
            baseline_correct += 1
        if enriched_match:
            enriched_correct += 1

        if enriched_match and not baseline_match:
            improvements += 1
            print(" ✓ IMPROVED", end="")
        elif not enriched_match and baseline_match:
            regressions += 1
            print(" ✗ REGRESSED", end="")
        else:
            print()

        result = TestResult(
            mint=mint,
            symbol=symbol,
            conviction=conviction,
            conviction_tier=conviction_tier,
            expected_verdict=expected,
            baseline_verdict=baseline_verdict,
            baseline_q_score=baseline_q_score,
            baseline_axioms=baseline_axioms,
            baseline_match=baseline_match,
            enriched_verdict=enriched_verdict,
            enriched_q_score=enriched_q_score,
            enriched_axioms=enriched_axioms,
            enriched_match=enriched_match,
            q_score_delta=enriched_q_score - baseline_q_score,
            improvement=enriched_match and not baseline_match,
        )
        results.append(result)

        time.sleep(0.5)

    # Results
    print("\n" + "=" * 80)
    print("RESULTS")
    print("=" * 80)

    n = len(results)
    if n == 0:
        print("ERROR: No successful test runs")
        return

    baseline_acc = baseline_correct / n
    enriched_acc = enriched_correct / n
    improvement_rate = improvements / n

    print(f"N={n}")
    print(f"\nBaseline accuracy:  {baseline_correct}/{n} = {baseline_acc:.1%}")
    print(f"Enriched accuracy:  {enriched_correct}/{n} = {enriched_acc:.1%}")
    print(f"Absolute improvement: {enriched_acc - baseline_acc:+.1%}")
    print(f"\nImprovements: {improvements}/{n} = {improvement_rate:.1%}")
    print(f"Regressions:  {regressions}/{n} = {regressions/n:.1%}")

    # Breakdown by conviction tier
    print("\n" + "=" * 80)
    print("BREAKDOWN BY CONVICTION TIER")
    print("=" * 80)

    for tier in ["strong", "mixed", "weak"]:
        tier_results = [r for r in results if r.conviction_tier == tier]
        if not tier_results:
            continue

        baseline_correct_tier = sum(1 for r in tier_results if r.baseline_match)
        enriched_correct_tier = sum(1 for r in tier_results if r.enriched_match)
        improvements_tier = sum(1 for r in tier_results if r.improvement)

        print(f"\n{tier.upper()} ({len(tier_results)} tokens):")
        print(f"  Baseline: {baseline_correct_tier}/{len(tier_results)} = {baseline_correct_tier/len(tier_results):.1%}")
        print(f"  Enriched: {enriched_correct_tier}/{len(tier_results)} = {enriched_correct_tier/len(tier_results):.1%}")
        print(f"  Improvements: {improvements_tier}")

    # Save results
    output_file = Path(__file__).parent / "ab_test_results.json"
    with open(output_file, "w") as f:
        json.dump(
            {
                "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S"),
                "n": n,
                "baseline_accuracy": baseline_acc,
                "enriched_accuracy": enriched_acc,
                "absolute_improvement": enriched_acc - baseline_acc,
                "improvements": improvements,
                "regressions": regressions,
                "improvement_rate": improvement_rate,
                "results": [asdict(r) for r in results],
            },
            f,
            indent=2,
        )

    print(f"\nResults saved to {output_file}")

    # Hypothesis conclusion
    print("\n" + "=" * 80)
    print("HYPOTHESIS TEST")
    print("=" * 80)

    if enriched_acc > baseline_acc:
        print(f"✓ HYPOTHESIS SUPPORTED")
        print(f"  Divergence enrichment improved accuracy by {enriched_acc - baseline_acc:.1%}")
        print(f"  From {baseline_acc:.1%} baseline to {enriched_acc:.1%} enriched")
    elif enriched_acc == baseline_acc:
        print(f"⊘ INCONCLUSIVE")
        print(f"  No statistically significant difference between baseline and enriched")
    else:
        print(f"✗ HYPOTHESIS REJECTED")
        print(f"  Divergence enrichment decreased accuracy by {baseline_acc - enriched_acc:.1%}")

if __name__ == "__main__":
    limit = 33
    if len(sys.argv) > 1 and sys.argv[1].startswith("--limit"):
        limit = int(sys.argv[1].split("=")[1])

    run_ab_test(limit=limit)
