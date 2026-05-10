#!/usr/bin/env python3
"""Measure enrichment quality on calibration corpus using live APIs.

Calls Helius (getAsset) + DexScreener directly, then applies the deterministic
dog scoring logic. This tests the full data pipeline without needing the kernel.

Usage: python3 scripts/measure_enrichment.py [--defi-only]
"""

import json
import os
import sys
import time
from pathlib import Path

import requests

CORPUS_PATH = Path.home() / ".cynic/datasets/tokens/calibration_corpus.json"
RESULTS_PATH = Path.home() / ".cynic/datasets/tokens/calibration_results_ENRICHED.json"
HELIUS_KEY = os.environ.get("HELIUS_API_KEY", "")
_HELIUS_PARAM = "api-key"
HELIUS_RPC = f"https://mainnet.helius-rpc.com/?{_HELIUS_PARAM}={HELIUS_KEY}" if HELIUS_KEY else ""

# ── Scoring constants (match Rust deterministic dog) ──
PHI_INV = 0.618
PHI_BASE = 0.30
SOVEREIGNTY_BASE = 0.35
ADJUST_SMALL = 0.05
ADJUST_MEDIUM = 0.10

def clamp(v, lo, hi):
    return max(lo, min(hi, v))

def q_to_verdict(q: float) -> str:
    if q > 0.528: return "HOWL"
    elif q > 0.382: return "WAG"
    elif q > 0.236: return "GROWL"
    else: return "BARK"

def get_dexscreener(mint: str) -> dict:
    """Fetch market data from DexScreener (free, no key)."""
    try:
        r = requests.get(f"https://api.dexscreener.com/tokens/v1/solana/{mint}", timeout=5)
        if r.status_code != 200:
            return {}
        pairs = r.json()
        if not pairs:
            return {}
        p = pairs[0]
        return {
            "price_usd": float(p.get("priceUsd", "0") or "0"),
            "volume_24h": p.get("volume", {}).get("h24"),
            "liquidity_usd": p.get("liquidity", {}).get("usd"),
            "market_cap": p.get("marketCap"),
            "fdv": p.get("fdv"),
        }
    except Exception as e:
        print(f"    DexScreener error: {e}", file=sys.stderr)
        return {}

def get_helius_asset(mint: str) -> dict:
    """Fetch token metadata via Helius getAsset."""
    if not HELIUS_KEY:
        return {}
    try:
        r = requests.post(
            HELIUS_RPC,
            json={"jsonrpc": "2.0", "id": 1, "method": "getAsset", "params": {"id": mint}},
            timeout=10,
        )
        data = r.json()
        return data.get("result", {})
    except Exception as e:
        print(f"    Helius error: {e}", file=sys.stderr)
        return {}

def estimate_holders(mint: str) -> int:
    """Probe DAS pages to estimate holder count."""
    if not HELIUS_KEY:
        return 0
    probe_pages = [2, 10, 100, 1000]
    estimated = 20
    for page in probe_pages:
        try:
            r = requests.post(
                HELIUS_RPC,
                json={"jsonrpc": "2.0", "id": 1, "method": "getTokenAccounts",
                      "params": {"mint": mint, "page": page, "limit": 1}},
                timeout=10,
            )
            data = r.json()
            accounts = data.get("result", {}).get("token_accounts", [])
            if accounts:
                estimated = page * 1000
            else:
                break
        except:
            break
        time.sleep(0.2)  # Rate limit
    return estimated

def score_token(metrics: dict) -> dict:
    """Apply deterministic dog scoring logic (Python mirror of Rust)."""
    holders = metrics.get("holders", 0)
    top1_pct = metrics.get("top1_pct", 0)
    top1_is_lp = metrics.get("top1_is_lp", False)
    hhi = metrics.get("herfindahl")
    age_hours = metrics.get("age_hours", 0)
    mint_active = metrics.get("mint_authority_active", False)
    freeze_active = metrics.get("freeze_authority_active", False)
    lp_burned = metrics.get("lp_status") == "burned"
    lp_locked = metrics.get("lp_status") == "locked"
    origin_pump = metrics.get("origin") == "pump.fun"
    market_cap = metrics.get("market_cap_usd")
    volume_24h = metrics.get("volume_24h_usd")
    liquidity = metrics.get("liquidity_usd")
    holder_data = metrics.get("holder_data_available", True)

    # FIDELITY
    fidelity = PHI_BASE
    if not mint_active:
        fidelity += ADJUST_MEDIUM
    else:
        fidelity -= ADJUST_MEDIUM
    if not freeze_active:
        fidelity += ADJUST_SMALL
    else:
        fidelity -= ADJUST_MEDIUM
    if market_cap and market_cap >= 100_000_000:
        fidelity += ADJUST_MEDIUM
    elif market_cap and market_cap >= 1_000_000:
        fidelity += ADJUST_SMALL
    fidelity = clamp(fidelity, ADJUST_SMALL, PHI_INV)

    # PHI
    phi = PHI_BASE
    if holder_data:
        if hhi is not None:
            if hhi < 0.15: phi += ADJUST_MEDIUM
            elif hhi > 0.50: phi -= ADJUST_MEDIUM
        if top1_pct < 15: phi += ADJUST_SMALL
        elif top1_pct > 50 and not top1_is_lp: phi -= ADJUST_MEDIUM
        if holders > 1000: phi += ADJUST_MEDIUM
        elif holders > 100: phi += ADJUST_SMALL
        elif holders < 20: phi -= ADJUST_SMALL
    if liquidity and liquidity >= 100_000:
        phi += ADJUST_MEDIUM
    elif liquidity and liquidity >= 10_000:
        phi += ADJUST_SMALL
    elif liquidity is not None and liquidity < 1_000:
        phi -= ADJUST_SMALL
    phi = clamp(phi, ADJUST_SMALL, PHI_INV)

    # VERIFY
    verify = PHI_BASE + ADJUST_SMALL
    if lp_burned: verify += ADJUST_SMALL
    elif not lp_locked: verify -= ADJUST_SMALL
    if age_hours > 720: verify += ADJUST_SMALL
    elif age_hours < 24: verify -= ADJUST_SMALL
    if volume_24h and volume_24h >= 1_000_000:
        verify += ADJUST_MEDIUM
    elif volume_24h and volume_24h >= 10_000:
        verify += ADJUST_SMALL
    verify = clamp(verify, ADJUST_SMALL, PHI_INV)

    # CULTURE
    culture = PHI_BASE
    if lp_burned: culture += ADJUST_MEDIUM
    elif not lp_locked: culture -= ADJUST_SMALL
    if not mint_active: culture += ADJUST_SMALL
    if not freeze_active: culture += ADJUST_SMALL
    if origin_pump: culture -= ADJUST_SMALL
    if holders > 100: culture += ADJUST_SMALL
    culture = clamp(culture, ADJUST_SMALL, PHI_INV)

    # BURN
    burn = PHI_BASE
    if not mint_active and not freeze_active:
        burn += ADJUST_MEDIUM
    elif mint_active and freeze_active:
        burn -= ADJUST_SMALL
    if lp_burned: burn += ADJUST_SMALL
    elif not lp_locked: burn -= ADJUST_SMALL
    if age_hours > 720: burn += ADJUST_SMALL
    burn = clamp(burn, ADJUST_SMALL, PHI_INV)

    # SOVEREIGNTY
    sovereignty = SOVEREIGNTY_BASE
    if holder_data:
        if top1_pct < 15: sovereignty += ADJUST_MEDIUM
        elif top1_pct > 50 and not top1_is_lp: sovereignty -= ADJUST_MEDIUM
        if holders > 100: sovereignty += ADJUST_SMALL
        if hhi is not None:
            if hhi < 0.15: sovereignty += ADJUST_SMALL
            elif hhi > 0.50: sovereignty -= ADJUST_SMALL
    if freeze_active: sovereignty -= ADJUST_MEDIUM
    if mint_active: sovereignty -= ADJUST_SMALL
    sovereignty = clamp(sovereignty, ADJUST_SMALL, PHI_INV)

    q_score = (fidelity + phi + verify + culture + burn + sovereignty) / 6.0
    return {
        "fidelity": fidelity, "phi": phi, "verify": verify,
        "culture": culture, "burn": burn, "sovereignty": sovereignty,
        "q_score": q_score, "verdict": q_to_verdict(q_score),
    }

def main():
    if not HELIUS_KEY:
        # Try to load from file
        env_path = Path.home() / ".cynic-env"
        if env_path.exists():
            for line in env_path.read_text().splitlines():
                if line.startswith("HELIUS_API_KEY="):
                    val = line.split("=", 1)[1].strip().strip('"')
                    os.environ.setdefault("HELIUS_API_KEY", val)
                    globals()["HELIUS_KEY"] = val
                    globals()["HELIUS_RPC"] = f"https://mainnet.helius-rpc.com/?{_HELIUS_PARAM}={val}"
                    break

    defi_only = "--defi-only" in sys.argv

    with open(CORPUS_PATH) as f:
        corpus = json.load(f)

    if defi_only:
        corpus = [t for t in corpus if t["category"] == "DEFI"]

    print(f"Measuring {len(corpus)} tokens (Helius + DexScreener + deterministic scoring)")
    print("=" * 70)

    results = []
    for i, token in enumerate(corpus):
        symbol = token["symbol"]
        mint = token["mint"]
        category = token["category"]
        expected = token["ground_truth_verdict"]

        print(f"[{i+1}/{len(corpus)}] {symbol:12s} ({category:10s}) ...", end=" ", flush=True)

        # Fetch data
        dex = get_dexscreener(mint)
        asset = get_helius_asset(mint)
        time.sleep(0.5)  # Rate limit

        # Extract Helius data
        token_info = asset.get("token_info", {})
        mint_auth = token_info.get("mint_authority")
        freeze_auth = token_info.get("freeze_authority")
        supply = token_info.get("supply", 0)
        decimals = token_info.get("decimals", 0)
        price_helius = (token_info.get("price_info") or {}).get("price_per_token")

        # Estimate holders
        estimated_holders = estimate_holders(mint) if HELIUS_KEY else 0

        # Build metrics
        price = dex.get("price_usd") or price_helius or 0
        mcap = dex.get("market_cap")
        if not mcap and price and supply and decimals:
            human_supply = supply / (10 ** decimals)
            mcap = human_supply * price

        metrics = {
            "holders": estimated_holders,
            "top1_pct": 0,  # Can't easily get without getTokenLargestAccounts
            "top1_is_lp": True,  # Assume top1 is LP for established tokens
            "herfindahl": None,
            "age_hours": 720,  # Floor for all established tokens
            "mint_authority_active": mint_auth is not None,
            "freeze_authority_active": freeze_auth is not None,
            "lp_status": "burned",  # Assume burned for DEFI tokens with liquidity
            "origin": "pump.fun" if mint.endswith("pump") else None,
            "holder_data_available": estimated_holders > 0,
            "market_cap_usd": mcap,
            "volume_24h_usd": dex.get("volume_24h"),
            "liquidity_usd": dex.get("liquidity_usd"),
            "price_usd": price,
        }

        scores = score_token(metrics)
        predicted = scores["verdict"]
        match = predicted == expected
        mark = "✓" if match else "✗"

        print(f"q={scores['q_score']:.3f} → {predicted:5s} (exp: {expected:5s}) {mark}  "
              f"[holders={estimated_holders}, mcap={'${:,.0f}'.format(mcap) if mcap else 'n/a'}, "
              f"vol={'${:,.0f}'.format(dex.get('volume_24h',0) or 0) if dex.get('volume_24h') else 'n/a'}]")

        results.append({
            "symbol": symbol, "mint": mint, "category": category,
            "expected": expected, "predicted": predicted,
            "match": match, "scores": scores, "metrics": metrics,
            "dex_raw": dex,
        })

        time.sleep(1)  # Be polite to APIs

    # Save
    with open(RESULTS_PATH, "w") as f:
        json.dump(results, f, indent=2)

    # Summary
    print("\n" + "=" * 70)
    print("RESULTS")
    print("=" * 70)
    from collections import defaultdict
    by_cat = defaultdict(list)
    for r in results:
        by_cat[r["category"]].append(r)

    total_match = sum(1 for r in results if r["match"])
    for cat in ["RUG", "DECLINING", "BORDERLINE", "MEMECOIN", "DEFI"]:
        items = by_cat.get(cat, [])
        matches = sum(1 for r in items if r["match"])
        total = len(items)
        pct = matches / total * 100 if total > 0 else 0
        print(f"  {cat:12s}: {matches}/{total} = {pct:.0f}%")

    print(f"  {'TOTAL':12s}: {total_match}/{len(results)} = {total_match/len(results)*100:.0f}%")
    print(f"\nResults saved to {RESULTS_PATH}")

if __name__ == "__main__":
    main()
