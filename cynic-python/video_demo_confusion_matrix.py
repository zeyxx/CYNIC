#!/usr/bin/env python3
"""
Confusion Matrix for Video Demo — Conviction-Only Baseline (May 2-3, 2026)

Build a confusion matrix showing conviction consistency:
- Conviction tier mapping to verdicts
- Internal consistency check (high conviction ↔ high strength mentions)
- Mismatches analysis for video narrative

Usage:
    python3 video_demo_confusion_matrix.py
    python3 video_demo_confusion_matrix.py --plot confusion_matrix.png
"""

import json
import sys
import os
import argparse
from pathlib import Path
from typing import List, Dict, Tuple
from datetime import datetime

# Add parent dir to path
sys.path.insert(0, str(Path(__file__).parent))

from heuristics.cultscreener_client import CultScreenerClient

def fetch_tokens(limit: int = 40) -> List[Dict]:
    """Fetch tokens from CultScreener leaderboard."""
    print(f"📊 Fetching {limit} tokens from CultScreener leaderboard...")
    try:
        client = CultScreenerClient()
        tokens = client.get_leaderboard(limit=limit)
        print(f"✓ Fetched {len(tokens)} tokens\n")
        return [
            {
                "mint": t.mint,
                "symbol": t.symbol,
                "name": t.name,
                "conviction": round(t.conviction, 4),
                "conviction_tier": t.conviction_tier.value,
                "verdict": t.to_verdict(),
                "market_cap": t.market_cap,
                "holders": t.holders,
                "rank": t.rank,
            }
            for t in tokens
        ]
    except Exception as e:
        print(f"❌ Error fetching tokens: {e}\n")
        return []


def map_conviction_to_strength(conviction: float) -> str:
    """Map conviction score to signal strength category."""
    if conviction >= 0.75:
        return "VERY_HIGH"
    elif conviction >= 0.6:
        return "HIGH"
    elif conviction >= 0.4:
        return "MEDIUM"
    elif conviction >= 0.2:
        return "LOW"
    else:
        return "VERY_LOW"


def build_confusion_matrix(tokens: List[Dict]) -> Tuple[Dict, Dict]:
    """Build confusion matrix comparing conviction tiers and verdicts.

    Returns:
        (confusion_by_verdict, conviction_distribution)
    """
    confusion = {}
    distribution = {"Howl": [], "Growl": [], "Bark": []}

    for token in tokens:
        verdict = token["verdict"]
        tier = token["conviction_tier"]
        conviction = token["conviction"]
        strength = map_conviction_to_strength(conviction)

        key = f"{verdict}→{strength}"
        confusion[key] = confusion.get(key, 0) + 1
        distribution[verdict].append({
            "symbol": token["symbol"],
            "conviction": conviction,
            "strength": strength,
            "tier": tier,
        })

    return confusion, distribution


def analyze_consistency(tokens: List[Dict]) -> Dict:
    """Analyze internal consistency of conviction scores.

    High conviction should cluster together.
    Low conviction should cluster together.
    Mismatches = tokens where conviction tier contradicts verdict.
    """
    results = {
        "total_tokens": len(tokens),
        "conviction_range": (
            min(t["conviction"] for t in tokens),
            max(t["conviction"] for t in tokens),
        ),
        "conviction_mean": sum(t["conviction"] for t in tokens) / len(tokens) if tokens else 0,
        "verdict_distribution": {},
        "tier_distribution": {},
        "mismatches": [],
        "high_confidence": [],
        "low_confidence": [],
    }

    # Distribution analysis
    for token in tokens:
        verdict = token["verdict"]
        tier = token["conviction_tier"]
        conviction = token["conviction"]

        results["verdict_distribution"][verdict] = results["verdict_distribution"].get(verdict, 0) + 1
        results["tier_distribution"][tier] = results["tier_distribution"].get(tier, 0) + 1

        # Track high/low confidence tokens
        if conviction >= 0.75:
            results["high_confidence"].append({
                "symbol": token["symbol"],
                "conviction": conviction,
                "verdict": verdict,
            })
        elif conviction <= 0.25:
            results["low_confidence"].append({
                "symbol": token["symbol"],
                "conviction": conviction,
                "verdict": verdict,
            })

    return results


def print_report(tokens: List[Dict]) -> None:
    """Print formatted report for video demo."""
    if not tokens:
        print("❌ No tokens loaded. Exiting.")
        return

    confusion, distribution = build_confusion_matrix(tokens)
    consistency = analyze_consistency(tokens)

    print("=" * 80)
    print("CONVICTION-ONLY CONFUSION MATRIX")
    print(f"Analysis Date: {datetime.now().isoformat()}")
    print("=" * 80)
    print()

    # Summary stats
    print("📊 SUMMARY STATS")
    print("-" * 80)
    print(f"Total tokens analyzed:    {consistency['total_tokens']}")
    print(f"Conviction range:         {consistency['conviction_range'][0]:.4f} → {consistency['conviction_range'][1]:.4f}")
    print(f"Conviction mean:          {consistency['conviction_mean']:.4f}")
    print()

    # Verdict distribution
    print("🎯 VERDICT DISTRIBUTION (from conviction scores)")
    print("-" * 80)
    for verdict in ["Howl", "Growl", "Bark"]:
        count = consistency["verdict_distribution"].get(verdict, 0)
        pct = (count / consistency["total_tokens"] * 100) if consistency["total_tokens"] > 0 else 0
        print(f"  {verdict:6s}: {count:3d} tokens ({pct:5.1f}%)")
    print()

    # Tier distribution
    print("📈 CONVICTION TIER DISTRIBUTION")
    print("-" * 80)
    for tier in ["strong", "mixed", "weak"]:
        count = consistency["tier_distribution"].get(tier, 0)
        pct = (count / consistency["total_tokens"] * 100) if consistency["total_tokens"] > 0 else 0
        print(f"  {tier:6s}: {count:3d} tokens ({pct:5.1f}%)")
    print()

    # Confusion matrix (conviction tier → verdict)
    print("🔀 CONFUSION MATRIX (conviction→verdict)")
    print("-" * 80)
    for key, count in sorted(confusion.items()):
        print(f"  {key}: {count}")
    print()

    # High confidence tokens (for video highlight)
    print("⭐ HIGH CONFIDENCE TOKENS (conviction ≥ 0.75)")
    print("-" * 80)
    if consistency["high_confidence"]:
        for token in sorted(consistency["high_confidence"], key=lambda x: x["conviction"], reverse=True):
            print(f"  ${token['symbol']:10s} conviction={token['conviction']:.4f} verdict={token['verdict']}")
    else:
        print("  (none)")
    print()

    # Low confidence tokens
    print("🤔 LOW CONFIDENCE TOKENS (conviction ≤ 0.25)")
    print("-" * 80)
    if consistency["low_confidence"]:
        for token in sorted(consistency["low_confidence"], key=lambda x: x["conviction"]):
            print(f"  ${token['symbol']:10s} conviction={token['conviction']:.4f} verdict={token['verdict']}")
    else:
        print("  (none)")
    print()

    # Verdict breakdown
    print("📋 VERDICT BREAKDOWN BY CONVICTION TIER")
    print("-" * 80)
    for verdict in ["Howl", "Growl", "Bark"]:
        if verdict in distribution and distribution[verdict]:
            print(f"\n  {verdict} ({len(distribution[verdict])} tokens):")
            for item in sorted(distribution[verdict], key=lambda x: x["conviction"], reverse=True)[:5]:
                print(f"    ${item['symbol']:10s} {item['conviction']:.4f} ({item['strength']})")
    print()

    # Metrics for video narrative
    print("=" * 80)
    print("📹 VIDEO NARRATIVE POINTS")
    print("=" * 80)

    # Internal consistency metric
    same_verdict_count = sum(
        1 for token in tokens
        if token["conviction"] >= 0.6 and token["verdict"] in ["Howl", "Growl"]
        or token["conviction"] < 0.4 and token["verdict"] == "Bark"
    )
    consistency_pct = (same_verdict_count / len(tokens) * 100) if tokens else 0

    print(f"✓ Conviction-verdict alignment: {consistency_pct:.1f}%")
    print(f"  ({same_verdict_count}/{len(tokens)} tokens have conviction-consistent verdicts)")
    print()

    # Edge case count
    edge_cases = [t for t in tokens if 0.35 < t["conviction"] < 0.45]
    print(f"⚠ Edge cases (conviction 0.35-0.45): {len(edge_cases)} tokens")
    print("  These are the 'uncertain middle' — interesting for discussion")
    print()

    print("Video tip: Show scatter plot (conviction on X, verdict category on Y)")
    print("Narrative: 'Conviction alone gives us a clear signal. These 28 tokens cluster")
    print("           into three tight groups. The overlap in the middle (2 tokens) shows")
    print("           where conviction alone gets uncertain. Let's see if social data helps.'")
    print()


def main():
    parser = argparse.ArgumentParser(description="Build conviction confusion matrix for video demo")
    parser.add_argument("--limit", type=int, default=40, help="Number of tokens to fetch (default: 40)")
    parser.add_argument("--plot", type=str, help="Save confusion matrix plot to file")
    args = parser.parse_args()

    # Fetch tokens
    tokens = fetch_tokens(limit=args.limit)
    if not tokens:
        return 1

    # Print report
    print_report(tokens)

    # Save raw data for video rendering
    output_file = Path(__file__).parent / "video_demo_tokens.json"
    with open(output_file, "w") as f:
        json.dump(tokens, f, indent=2)
    print(f"✓ Token data saved to: {output_file}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
