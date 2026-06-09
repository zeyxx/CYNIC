#!/usr/bin/env python3
"""Calibration v2: HeliusTokenProfiler → enriched stimulus → /judge → measure spread.

Uses the proper ingestion pipeline (getAsset + getTokenLargestAccounts +
getTransactionsForAddress) instead of raw RPC hacks.

Usage:
    python3 calibrate_v2.py              # Full run (36 survivors + 10 dead)
    python3 calibrate_v2.py --quick      # 3 survivors + 3 dead
    python3 calibrate_v2.py --profile-only  # Profile tokens without judging
"""

import os
import sys
import json
import time
from typing import List, Dict, Optional

from helius_token_profiler import HeliusTokenProfiler, TokenProfile, load_env

load_env()

from cultscreener_client import CultScreenerClient

# Known dead pump.fun tokens (from searchAssets scan)
DEAD_TOKENS = [
    ("COMBAT PENIS COIN", "JEKNGFTwpaaN5mw9SzaLPoefLfkDXXqgCVfNS7F1pump"),
    ("Official Pandvil Coin", "JEKNBRsaavuqGJBYhU39WPUPis2g47LiKyhXC13tpump"),
    ("bro", "JEKLejKLbsLBvue465mE5VBDxdCzJUAX4hS4qhv1pump"),
    ("ORO", "JEKKTTvosk5AN7wpk9GiRoo2pi7FJaceWCzc42NDpump"),
    ("Non-Fake Tits", "JEKJzTWEiCSJ2UAZu28xErzSwEp5yeq28MWTkP3Ypump"),
    ("Adult Coin", "JEKHzks3dAiLRT61Use2bHS3xTp21o4hs2eGGAxNpump"),
    ("8LANA", "JEKGQgSk8UHMpuknbdYhaK4TAspvKhY2iUwBCwFYpump"),
    ("Green Limmy", "JEKGKPy2VKhgcQGUaU985p1RhvPeW7qiXVswaHEVpump"),
    ("LABUDI", "JEKB63jfhcVyGAgkvyVSjwMpWCRifdVCTeDAqU48pump"),
    ("Retard Finder Kennedy", "JEKLT4Y6DN6qf2U3jXAcEEwiU7zmon6hz2haaLGFpump"),
]


def build_population_stats(profiles: List[TokenProfile]) -> dict:
    """Build population statistics from profiled tokens."""
    convictions = sorted(p.conviction_1m for p in profiles if p.conviction_1m is not None)
    holders = sorted(p.holder_count for p in profiles if p.holder_count and p.holder_count > 0)
    mcaps = sorted(p.market_cap_usd for p in profiles if p.market_cap_usd and p.market_cap_usd > 0)

    return {
        "label": "CultScreener leaderboard",
        "n": len(profiles),
        "conviction": convictions,
        "holders": holders,
        "mcap": mcaps,
    }


def run_calibration(quick: bool = False, profile_only: bool = False):
    print("=" * 70)
    print("CYNIC Token Calibration v2 — Helius Profiler Pipeline")
    print("=" * 70)

    profiler = HeliusTokenProfiler()

    # Phase 1: Fetch CultScreener tokens
    print("\n[1/4] Fetching CultScreener leaderboard...")
    client = CultScreenerClient()
    leaderboard = client.get_leaderboard(limit=100)
    print(f"  {len(leaderboard)} tokens from leaderboard")

    if quick:
        # Pick 3 diverse tokens
        leaderboard = [leaderboard[0], leaderboard[len(leaderboard)//2], leaderboard[-1]]
        print(f"  Quick mode: selected {len(leaderboard)} tokens")

    # Phase 2: Profile survivors
    print("\n[2/4] Profiling survivors via Helius...")
    survivor_profiles: List[TokenProfile] = []

    for i, token in enumerate(leaderboard):
        sym = token.symbol or token.mint[:8]
        sys.stdout.write(f"  [{i+1}/{len(leaderboard)}] {sym:15s}")
        sys.stdout.flush()

        p = profiler.profile(
            token.mint,
            holder_count=token.holders,
            conviction_1m=token.conviction,
        )

        age_str = f"{p.age_hours/24:.0f}d" if p.age_hours else "?"
        mcap_str = f"${p.market_cap_usd:,.0f}" if p.market_cap_usd else "$?"
        print(f" age={age_str:>6s} mcap={mcap_str:>14s} top1={p.top1_pct_supply:.1f}% "
              f"last_tx={p.hours_since_last_tx:.0f}h" if p.hours_since_last_tx is not None
              else f" age={age_str:>6s} mcap={mcap_str:>14s} top1={p.top1_pct_supply:.1f}%")

        survivor_profiles.append(p)
        time.sleep(0.15)  # Rate limit courtesy

    # Phase 3: Profile dead tokens
    dead_list = DEAD_TOKENS[:3] if quick else DEAD_TOKENS
    print(f"\n[3/4] Profiling {len(dead_list)} dead tokens...")
    dead_profiles: List[TokenProfile] = []

    for i, (name, mint) in enumerate(dead_list):
        sys.stdout.write(f"  [{i+1}/{len(dead_list)}] {name[:15]:15s}")
        sys.stdout.flush()

        p = profiler.profile(mint)
        p.name = name  # Override with known name

        age_str = f"{p.age_hours/24:.0f}d" if p.age_hours else "?"
        last_str = f"{p.hours_since_last_tx:.0f}h" if p.hours_since_last_tx is not None else "?"
        print(f" age={age_str:>6s} top1={p.top1_pct_supply:.1f}% last_tx={last_str}ago")

        dead_profiles.append(p)
        time.sleep(0.15)

    # Build population stats (from survivors only — they are the reference class)
    pop_stats = build_population_stats(survivor_profiles)

    # Phase 4: Generate stimuli and save
    print(f"\n[4/4] Generating enriched stimuli...")

    all_results = []

    for p in survivor_profiles:
        stimulus = p.to_stimulus(pop_stats)
        all_results.append({
            "mint": p.mint,
            "symbol": p.symbol or p.name or p.mint[:12],
            "class": "survivor",
            "conviction": p.conviction_1m,
            "holder_count": p.holder_count,
            "market_cap": p.market_cap_usd,
            "age_days": p.age_hours / 24 if p.age_hours else None,
            "hours_since_last_tx": p.hours_since_last_tx,
            "top1_pct": p.top1_pct_supply,
            "top10_pct": p.top10_pct_supply,
            "recent_swaps": p.recent_swap_count,
            "origin_pump_fun": p.origin_pump_fun,
            "stimulus": stimulus,
            "stimulus_chars": len(stimulus),
        })

    for p in dead_profiles:
        stimulus = p.to_stimulus(pop_stats)
        all_results.append({
            "mint": p.mint,
            "symbol": p.symbol or p.name or p.mint[:12],
            "class": "dead",
            "conviction": None,
            "holder_count": p.holder_count,
            "market_cap": p.market_cap_usd,
            "age_days": p.age_hours / 24 if p.age_hours else None,
            "hours_since_last_tx": p.hours_since_last_tx,
            "top1_pct": p.top1_pct_supply,
            "top10_pct": p.top10_pct_supply,
            "recent_swaps": p.recent_swap_count,
            "origin_pump_fun": p.origin_pump_fun,
            "stimulus": stimulus,
            "stimulus_chars": len(stimulus),
        })

    # Save
    output_path = os.path.join(os.path.dirname(__file__), "calibration_v2_profiles.json")
    with open(output_path, "w") as f:
        json.dump({
            "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S"),
            "survivor_count": len(survivor_profiles),
            "dead_count": len(dead_profiles),
            "population_stats": {
                "n": pop_stats["n"],
                "conviction_range": [min(pop_stats["conviction"]), max(pop_stats["conviction"])] if pop_stats["conviction"] else [],
                "holders_range": [min(pop_stats["holders"]), max(pop_stats["holders"])] if pop_stats["holders"] else [],
                "mcap_range": [min(pop_stats["mcap"]), max(pop_stats["mcap"])] if pop_stats["mcap"] else [],
            },
            "tokens": all_results,
        }, f, indent=2)

    print(f"\n  Saved {len(all_results)} profiles to {output_path}")

    # Summary stats
    print("\n" + "=" * 70)
    print("DATASET SUMMARY")
    print("=" * 70)

    survivors = [r for r in all_results if r["class"] == "survivor"]
    deads = [r for r in all_results if r["class"] == "dead"]

    print(f"\n  Survivors: {len(survivors)}")
    ages = [r["age_days"] for r in survivors if r["age_days"]]
    if ages:
        print(f"    Age: {min(ages):.0f}d — {max(ages):.0f}d (median {sorted(ages)[len(ages)//2]:.0f}d)")
    mcaps = [r["market_cap"] for r in survivors if r["market_cap"]]
    if mcaps:
        print(f"    Mcap: ${min(mcaps):,.0f} — ${max(mcaps):,.0f}")
    convs = [r["conviction"] for r in survivors if r["conviction"] is not None]
    if convs:
        print(f"    Conviction: {min(convs):.1%} — {max(convs):.1%}")

    print(f"\n  Dead tokens: {len(deads)}")
    ages_d = [r["age_days"] for r in deads if r["age_days"]]
    if ages_d:
        print(f"    Age: {min(ages_d):.0f}d — {max(ages_d):.0f}d")
    last_txs = [r["hours_since_last_tx"] for r in deads if r["hours_since_last_tx"] is not None]
    if last_txs:
        print(f"    Last activity: {min(last_txs)/24:.0f}d — {max(last_txs)/24:.0f}d ago")

    # Discriminating features
    print("\n  Feature separation (survivor vs dead):")
    for feat in ["top1_pct", "top10_pct", "age_days", "hours_since_last_tx", "recent_swaps"]:
        s_vals = [r[feat] for r in survivors if r[feat] is not None]
        d_vals = [r[feat] for r in deads if r[feat] is not None]
        if s_vals and d_vals:
            s_med = sorted(s_vals)[len(s_vals)//2]
            d_med = sorted(d_vals)[len(d_vals)//2]
            sep = abs(s_med - d_med) / max(abs(s_med), abs(d_med), 0.001) * 100
            print(f"    {feat:25s}: survivor_med={s_med:>10.1f}  dead_med={d_med:>10.1f}  separation={sep:.0f}%")


if __name__ == "__main__":
    quick = "--quick" in sys.argv
    profile_only = "--profile-only" in sys.argv
    run_calibration(quick=quick, profile_only=profile_only)
