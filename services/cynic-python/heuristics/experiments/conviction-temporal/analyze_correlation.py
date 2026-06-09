#!/usr/bin/env python3
"""Tier 1 EXPERIMENTAL: static on-chain metric correlation analysis.

Research question: Can holders/top1_pct/top10_pct differentiate strong vs weak conviction?
Falsification: If Spearman ρ > 0.5 for any metric, static data has predictive value.
Result: ALL ρ < 0.4 → static metrics are NOISE for conviction. CONFIRMED.
"""

import json
import os
from typing import List, Tuple

DATA_PATH = os.path.join(os.path.dirname(__file__), "calibration_results_real.json")


def spearman_rank(x: List[float], y: List[float]) -> float:
    """Spearman rank correlation (no scipy dependency)."""
    n = len(x)
    if n < 3:
        return 0.0

    def ranks(vals: List[float]) -> List[float]:
        indexed = sorted(enumerate(vals), key=lambda t: t[1])
        r = [0.0] * n
        i = 0
        while i < n:
            j = i
            while j < n - 1 and indexed[j + 1][1] == indexed[j][1]:
                j += 1
            avg_rank = (i + j) / 2.0 + 1.0
            for k in range(i, j + 1):
                r[indexed[k][0]] = avg_rank
            i = j + 1
        return r

    rx = ranks(x)
    ry = ranks(y)
    d_sq = sum((a - b) ** 2 for a, b in zip(rx, ry))
    return 1.0 - (6.0 * d_sq) / (n * (n * n - 1))


def main() -> None:
    with open(DATA_PATH) as f:
        data = json.load(f)

    results = data["results"]
    n = len(results)

    print(f"Tokens: {n}")
    print(f"Accuracy: {data['accuracy']:.1%}")
    print()

    # Extract vectors
    conviction = [r["conviction"] for r in results]
    holders = [r["metrics"]["holders"] for r in results]
    top1 = [r["metrics"]["top1_pct"] for r in results]
    top10 = [r["metrics"]["top10_pct"] for r in results]
    q_scores = [r["q_score"] for r in results]

    # 1. Spearman correlations: metric vs conviction
    print("=" * 60)
    print("SPEARMAN ρ (metric vs conviction)")
    print("  ρ > 0.5 = useful signal | ρ < 0.3 = noise")
    print("=" * 60)

    metrics: List[Tuple[str, List[float]]] = [
        ("holders", holders),
        ("top1_pct", top1),
        ("top10_pct", top10),
        ("q_score", q_scores),
    ]

    for name, values in metrics:
        rho = spearman_rank(values, conviction)
        signal = "SIGNAL" if abs(rho) > 0.5 else ("weak" if abs(rho) > 0.3 else "NOISE")
        print(f"  {name:12s}  ρ = {rho:+.3f}  [{signal}]")

    # 2. Per-tier statistics
    print()
    print("=" * 60)
    print("PER-TIER DISTRIBUTIONS")
    print("=" * 60)

    for tier in ["strong", "mixed", "weak"]:
        tier_data = [r for r in results if r["conviction_tier"] == tier]
        if not tier_data:
            continue

        t_holders = [r["metrics"]["holders"] for r in tier_data]
        t_top1 = [r["metrics"]["top1_pct"] for r in tier_data]
        t_top10 = [r["metrics"]["top10_pct"] for r in tier_data]
        t_conv = [r["conviction"] for r in tier_data]
        t_q = [r["q_score"] for r in tier_data]

        print(f"\n  {tier.upper()} (n={len(tier_data)}, conviction {min(t_conv):.3f}-{max(t_conv):.3f}):")
        print(f"    holders:  median={sorted(t_holders)[len(t_holders)//2]:>8,}  "
              f"range=[{min(t_holders):,} - {max(t_holders):,}]")
        print(f"    top1_pct: median={sorted(t_top1)[len(t_top1)//2]:>8.1f}%  "
              f"range=[{min(t_top1):.1f}% - {max(t_top1):.1f}%]")
        print(f"    top10_pct:median={sorted(t_top10)[len(t_top10)//2]:>8.1f}%  "
              f"range=[{min(t_top10):.1f}% - {max(t_top10):.1f}%]")
        print(f"    q_score:  median={sorted(t_q)[len(t_q)//2]:>8.3f}  "
              f"range=[{min(t_q):.3f} - {max(t_q):.3f}]")

    # 3. Overlap analysis: can ANY threshold separate tiers?
    print()
    print("=" * 60)
    print("SEPARABILITY: Can a threshold on any metric separate strong from weak?")
    print("=" * 60)

    strong = [r for r in results if r["conviction_tier"] == "strong"]
    weak = [r for r in results if r["conviction_tier"] == "weak"]

    if strong and weak:
        for name, key in [("holders", "holders"), ("top1_pct", "top1_pct"), ("top10_pct", "top10_pct")]:
            s_vals = sorted([r["metrics"][key] for r in strong])
            w_vals = sorted([r["metrics"][key] for r in weak])

            s_min, s_max = min(s_vals), max(s_vals)
            w_min, w_max = min(w_vals), max(w_vals)

            overlap = max(0, min(s_max, w_max) - max(s_min, w_min))
            total_range = max(s_max, w_max) - min(s_min, w_min)
            overlap_pct = (overlap / total_range * 100) if total_range > 0 else 100

            print(f"  {name:12s}: strong=[{s_min:.1f}-{s_max:.1f}] weak=[{w_min:.1f}-{w_max:.1f}] "
                  f"overlap={overlap_pct:.0f}%")
    else:
        print("  Not enough data in strong/weak tiers")

    # 4. Q-score clustering (the compression problem)
    print()
    print("=" * 60)
    print("Q-SCORE COMPRESSION")
    print("=" * 60)

    q_min, q_max = min(q_scores), max(q_scores)
    q_range = q_max - q_min
    print(f"  Total q_score range: {q_min:.3f} - {q_max:.3f} (Δ = {q_range:.3f})")
    print(f"  Verdict thresholds:  BARK ≤0.236 | GROWL ≤0.382 | WAG ≤0.528 | HOWL")
    print(f"  All 33 tokens fit in {q_range/0.618*100:.1f}% of the useful range (0-0.618)")

    # Count per verdict bucket
    buckets = {"Bark": 0, "Growl": 0, "Wag": 0, "Howl": 0}
    for q in q_scores:
        if q > 0.528:
            buckets["Howl"] += 1
        elif q > 0.382:
            buckets["Wag"] += 1
        elif q > 0.236:
            buckets["Growl"] += 1
        else:
            buckets["Bark"] += 1
    print(f"  Distribution: {buckets}")

    # 5. Scatter: conviction vs each metric (text plot)
    print()
    print("=" * 60)
    print("CONVICTION vs HOLDERS (text scatter)")
    print("=" * 60)

    # Sort by conviction descending
    sorted_results = sorted(results, key=lambda r: -r["conviction"])
    for r in sorted_results:
        conv = r["conviction"]
        h = r["metrics"]["holders"]
        bar = "█" * min(int(h / 5000), 30)
        tier_mark = {"strong": "S", "mixed": "M", "weak": "W"}[r["conviction_tier"]]
        print(f"  {r['symbol']:12s} conv={conv:.3f} [{tier_mark}] h={h:>7,} {bar}")


if __name__ == "__main__":
    main()
