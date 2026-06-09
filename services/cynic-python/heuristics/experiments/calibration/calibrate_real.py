#!/usr/bin/env python3
"""Real-data calibration: CultScreener conviction → Helius enrichment → TokenScorer → accuracy.

This script bypasses the kernel /enrich/wallet endpoint and calls Helius directly.
Ground truth = CultScreener conviction tier (strong→HOWL, mixed→GROWL, weak→BARK).

Usage:
    python3 calibrate_real.py
    python3 calibrate_real.py --limit 10  # quick test

Requires:
    HELIUS_API_KEY in ~/.cynic-env or environment
    CULTSCREENER_API_KEY in .env or environment
"""

import os
import sys
import json
import time
import requests
from typing import Optional, Dict, List
from dataclasses import asdict

# Load env
def _load_env():
    for env_file in [
        os.path.join(os.path.dirname(__file__), '.env'),
        os.path.expanduser('~/.cynic-env'),
    ]:
        if os.path.exists(env_file):
            with open(env_file) as f:
                for line in f:
                    line = line.strip()
                    if line and not line.startswith('#') and '=' in line:
                        key, value = line.split('=', 1)
                        key = key.replace('export ', '').strip()
                        if key not in os.environ:
                            os.environ[key] = value.strip()

_load_env()

from cultscreener_client import CultScreenerClient, TokenConvictionData
from token_heuristics import TokenScorer, TokenMetrics, AxiomScores

HELIUS_API_KEY = os.getenv("HELIUS_API_KEY")
if not HELIUS_API_KEY:
    print("ERROR: HELIUS_API_KEY not set")
    sys.exit(1)

HELIUS_RPC = f"https://mainnet.helius-rpc.com/?api-key={HELIUS_API_KEY}"  # noqa: env var


def helius_rpc(method: str, params) -> Optional[dict]:
    """Call Helius JSON-RPC."""
    try:
        resp = requests.post(HELIUS_RPC, json={
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        }, timeout=30)
        data = resp.json()
        if "error" in data:
            return None
        return data.get("result")
    except Exception:
        return None


PUMPFUN_AUTHORITY = "TSLvdd1pWpHVjahSpsvCXUbgwsL3JAcvokwaKt1eokM"


def fetch_token_metrics(mint: str) -> Optional[TokenMetrics]:
    """Fetch on-chain metrics for a token from Helius.

    Uses:
    - getAsset (DAS) for authorities, supply info
    - getTokenLargestAccounts for holder distribution (relative to total supply)
    """
    # 1. Get asset metadata via DAS
    asset = helius_rpc("getAsset", {"id": mint})

    mint_authority_active = False
    freeze_authority_active = False
    supply_burned_pct = None
    origin_pump_fun = False
    total_supply = 0
    decimals = 0

    if asset:
        token_info = asset.get("token_info", {})

        # Authorities from token_info (the real source), not from metadata authorities
        mint_authority_active = token_info.get("mint_authority") is not None
        freeze_authority_active = token_info.get("freeze_authority") is not None

        total_supply = int(token_info.get("supply", 0))
        decimals = int(token_info.get("decimals", 0))

        # Pump.fun detection: authority address or mint suffix
        authorities = asset.get("authorities", [])
        for auth in authorities:
            if auth.get("address") == PUMPFUN_AUTHORITY:
                origin_pump_fun = True
                break
        if not origin_pump_fun and mint.endswith("pump"):
            origin_pump_fun = True

    # 2. Get top holders (relative to total supply, not just top-20 sum)
    holders_result = helius_rpc("getTokenLargestAccounts", [mint])

    holders = 0
    top1_pct = 0.0
    top10_pct = 0.0

    if holders_result and isinstance(holders_result, dict):
        accounts = holders_result.get("value", [])
        if accounts:
            amounts = [int(acc.get("amount", "0")) for acc in accounts]

            # Use total supply from DAS for accurate percentages
            denom = total_supply if total_supply > 0 else sum(amounts)
            if denom > 0:
                top1_pct = (amounts[0] / denom) * 100 if amounts else 0
                top10_amt = sum(amounts[:10])
                top10_pct = (top10_amt / denom) * 100

            holders = len(accounts)  # Min bound — real count higher

    if holders == 0 and not asset:
        return None

    return TokenMetrics(
        holders=max(holders, 1),
        top1_pct=top1_pct,
        top10_pct=top10_pct,
        herfindahl=None,
        age_hours=720,  # Default — would need tx history for real age
        mint_authority_active=mint_authority_active,
        freeze_authority_active=freeze_authority_active,
        lp_burned=False,
        lp_locked=False,
        supply_burned_pct=supply_burned_pct,
        origin_pump_fun=origin_pump_fun,
        exchange_listed=False,
        is_ecosystem_token=False,
        has_real_volume=False,
    )


def verdict_from_qscore(q_score: float) -> str:
    """Map Q-score to verdict (CYNIC thresholds)."""
    if q_score > 0.528:
        return "Howl"
    elif q_score > 0.382:
        return "Wag"
    elif q_score > 0.236:
        return "Growl"
    else:
        return "Bark"


def run_calibration(limit: int = 60):
    """Run full calibration pipeline."""
    print("=" * 60)
    print("CYNIC Token Calibration — Real Data")
    print("=" * 60)

    # 1. Fetch tokens from CultScreener
    print("\n[1/3] Fetching tokens from CultScreener...")
    client = CultScreenerClient()

    all_tokens: List[TokenConvictionData] = []

    strong = client.get_leaderboard(limit=min(limit // 3, 30), min_conviction=0.7)
    all_tokens.extend(strong)
    print(f"  Strong (>=0.7): {len(strong)} tokens")

    mixed_raw = client.get_leaderboard(limit=100, min_conviction=0.3)
    mixed = [t for t in mixed_raw if t.conviction < 0.7][:min(limit // 3, 20)]
    all_tokens.extend(mixed)
    print(f"  Mixed (0.3-0.7): {len(mixed)} tokens")

    weak_raw = client.get_leaderboard(limit=100, min_conviction=0.0)
    weak = [t for t in weak_raw if t.conviction < 0.3][:min(limit // 3, 20)]
    all_tokens.extend(weak)
    print(f"  Weak (<0.3): {len(weak)} tokens")

    print(f"  Total: {len(all_tokens)} tokens")

    # 2. Enrich and score each token
    print("\n[2/3] Enriching from Helius + scoring...")
    scorer = TokenScorer()
    results: List[Dict] = []

    for i, token in enumerate(all_tokens):
        symbol = token.symbol or token.mint[:8]
        sys.stdout.write(f"  [{i+1}/{len(all_tokens)}] {symbol:12s} (conv={token.conviction:.3f})...")
        sys.stdout.flush()

        metrics = fetch_token_metrics(token.mint)
        if not metrics:
            print(" SKIP (no data)")
            continue

        # Use CultScreener's holder count (real) instead of Helius top-20 limit
        if token.holders and token.holders > metrics.holders:
            metrics.holders = token.holders

        scores = scorer.score(metrics, token_name=token.symbol)
        predicted = verdict_from_qscore(scores.q_score)
        expected = token.to_verdict()

        match = "OK" if predicted == expected else "MISS"
        print(f" q={scores.q_score:.3f} -> {predicted:5s} (exp {expected:5s}) [{match}]")

        results.append({
            "mint": token.mint,
            "symbol": symbol,
            "conviction": token.conviction,
            "conviction_tier": token.conviction_tier.value,
            "expected_verdict": expected,
            "predicted_verdict": predicted,
            "q_score": scores.q_score,
            "axioms": {
                "fidelity": scores.fidelity,
                "phi": scores.phi,
                "verify": scores.verify,
                "culture": scores.culture,
                "burn": scores.burn,
                "sovereignty": scores.sovereignty,
            },
            "metrics": {
                "holders": metrics.holders,
                "top1_pct": round(metrics.top1_pct, 2),
                "top10_pct": round(metrics.top10_pct, 2),
                "mint_authority": metrics.mint_authority_active,
                "freeze_authority": metrics.freeze_authority_active,
                "origin_pump_fun": metrics.origin_pump_fun,
            },
            "match": predicted == expected,
        })

        time.sleep(0.3)  # Rate limit

    # 3. Analyze results
    print("\n" + "=" * 60)
    print("[3/3] CALIBRATION RESULTS")
    print("=" * 60)

    if not results:
        print("No results to analyze!")
        return

    total = len(results)
    correct = sum(1 for r in results if r["match"])
    accuracy = correct / total if total else 0

    print(f"\n  Overall Accuracy: {correct}/{total} = {accuracy:.1%}")

    # Per-tier breakdown
    print("\n  Per-tier:")
    for tier in ["strong", "mixed", "weak"]:
        tier_results = [r for r in results if r["conviction_tier"] == tier]
        if not tier_results:
            continue
        tier_correct = sum(1 for r in tier_results if r["match"])
        tier_acc = tier_correct / len(tier_results) if tier_results else 0
        print(f"    {tier:8s}: {tier_correct}/{len(tier_results)} = {tier_acc:.1%}")

    # Confusion matrix
    print("\n  Confusion (expected -> predicted):")
    from collections import Counter
    confusion = Counter()
    for r in results:
        confusion[(r["expected_verdict"], r["predicted_verdict"])] += 1

    for (expected, predicted), count in sorted(confusion.items()):
        marker = " OK" if expected == predicted else ""
        print(f"    {expected:5s} -> {predicted:5s}: {count}{marker}")

    # Q-score distribution per tier
    print("\n  Q-score distribution:")
    for tier in ["strong", "mixed", "weak"]:
        tier_results = [r for r in results if r["conviction_tier"] == tier]
        if not tier_results:
            continue
        q_scores = [r["q_score"] for r in tier_results]
        avg = sum(q_scores) / len(q_scores)
        mn, mx = min(q_scores), max(q_scores)
        print(f"    {tier:8s}: avg={avg:.3f} min={mn:.3f} max={mx:.3f} (n={len(q_scores)})")

    # Save results
    output_path = os.path.join(os.path.dirname(__file__), "calibration_results_real.json")
    with open(output_path, "w") as f:
        json.dump({
            "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S"),
            "total_tokens": total,
            "accuracy": accuracy,
            "results": results,
        }, f, indent=2)
    print(f"\n  Saved: {output_path}")

    # Worst misses
    misses = [r for r in results if not r["match"]]
    if misses:
        print(f"\n  MISSES ({len(misses)}) — review these:")
        for r in misses[:10]:
            print(f"    {r['symbol']:12s} conv={r['conviction']:.3f} "
                  f"exp={r['expected_verdict']:5s} got={r['predicted_verdict']:5s} "
                  f"q={r['q_score']:.3f} holders={r['metrics']['holders']} "
                  f"top1={r['metrics']['top1_pct']:.1f}%")


if __name__ == "__main__":
    limit = 60
    if "--limit" in sys.argv:
        idx = sys.argv.index("--limit")
        if idx + 1 < len(sys.argv):
            limit = int(sys.argv[idx + 1])

    run_calibration(limit=limit)
