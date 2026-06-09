#!/usr/bin/env python3
"""Sovereign K-Score v2: behavioral signal via random holder sampling.

v1 flaw: top-5 holders always = diamond hands (survivorship bias).
v2 fix: sample RANDOM holders via DAS getTokenAccounts, measure dust ratio.

Hypothesis: CultScreener conviction correlates with holder health distribution.
- Strong conviction → few dust wallets, holders retain meaningful balances
- Weak conviction → many dust wallets, holders dumped to near-zero

Falsification: If Spearman ρ(dust_ratio, conviction) < 0.3 → hypothesis rejected.

Cost: ~1,400 credits (getAsset 10cr + getTokenAccounts 10cr + age 10cr) × 33 + buffer.

Usage:
    python3 kscore_sovereign_v2.py
    python3 kscore_sovereign_v2.py --limit 5
    python3 kscore_sovereign_v2.py --dry-run
"""

import json
import math
import os
import sys
import time
from dataclasses import dataclass, field, asdict
from typing import Optional, List, Dict

# ── ENV ──

def _load_env() -> None:
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

import requests

HELIUS_API_KEY = os.getenv("HELIUS_API_KEY")
if not HELIUS_API_KEY:
    print("ERROR: HELIUS_API_KEY not set")
    sys.exit(1)

HELIUS_RPC = f"https://mainnet.helius-rpc.com/?api-key={HELIUS_API_KEY}"

# ── KNOWN POOL / DEX PROGRAMS ──

POOL_PROGRAMS: set[str] = {
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",  # Raydium AMM
    "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",  # Raydium CLMM
    "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc",   # Orca Whirlpool
    "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP",  # Orca v1
    "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P",   # PumpFun bonding curve
    "39azUYFWPz3VHgKCf3VChUwbpURdCHRxjWVowf5jUJjg",  # PumpFun fee
    "TSLvdd1pWpHVjahSpsvCXUbgwsL3JAcvokwaKt1eokM",   # PumpFun migration
    "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4",   # Jupiter v6
    "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB",   # Jupiter v4
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1",  # Raydium v4
    "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo",   # Meteora DLMM
    "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB",  # Meteora pools
}


# ── DATA ──

@dataclass
class TokenAnalysis:
    """Analysis result for a single token."""
    mint: str
    symbol: str
    conviction_cultscreener: float
    conviction_tier: str

    # Holder distribution metrics
    total_holders_sampled: int
    pools_filtered: int
    real_holders: int       # balance > $1 USD (or > 0.1% of supply if no price)
    dust_holders: int       # balance < $0.01 USD (or < 0.001% of supply)
    zero_holders: int       # balance == 0
    active_holders: int     # balance > $1 AND < top 1% (not whales, not dust)

    # Derived signals
    dust_ratio: float       # dust_holders / (total - pools - zeros)
    organic_ratio: float    # real_holders / (total - pools - zeros)
    gini: float             # Gini coefficient of balance distribution (0=equal, 1=concentrated)

    # Age
    token_age_days: Optional[float]
    longevity: float        # min(age/365, 1.0)

    # Composite
    kscore_v2: float        # (1 - dust_ratio)^0.5 × organic_ratio^0.35 × longevity^0.15

    # Price
    price_usd: Optional[float]
    supply_human: float
    decimals: int

    # Quality
    api_credits_used: int
    errors: List[str] = field(default_factory=list)


# ── API ──

_credits = 0
_calls = 0


def _rpc(method: str, params, credits: int = 1) -> Optional[dict]:
    global _credits, _calls
    try:
        resp = requests.post(HELIUS_RPC, json={
            "jsonrpc": "2.0", "id": 1,
            "method": method, "params": params,
        }, timeout=30)
        data = resp.json()
        _calls += 1
        _credits += credits
        if "error" in data:
            return None
        time.sleep(0.12)  # 10 req/s rate limit
        return data.get("result")
    except Exception:
        return None


def get_token_accounts_sample(mint: str, pages: int = 3) -> List[Dict]:
    """Fetch random holder sample via DAS getTokenAccounts.

    Each page = 100 accounts, 10 credits.
    Returns: list of {owner, amount} dicts.
    """
    all_accounts: List[Dict] = []

    for page in range(1, pages + 1):
        result = _rpc("getTokenAccounts", {
            "mint": mint,
            "page": page,
            "limit": 100,
        }, credits=10)

        if not result:
            break

        accounts = result.get("token_accounts", [])
        if not accounts:
            break  # no more pages

        for acct in accounts:
            owner = acct.get("owner")
            amount = acct.get("amount")
            if owner and amount is not None:
                all_accounts.append({
                    "owner": owner,
                    "amount": int(amount) if isinstance(amount, (int, float)) else int(str(amount)),
                })

    return all_accounts


def get_token_age(mint: str) -> Optional[float]:
    """Token age in days. Cost: 10 cr."""
    result = _rpc("getTransactionsForAddress", [
        mint,
        {"sortOrder": "asc", "limit": 1, "transactionDetails": "signatures"}
    ], credits=10)
    if result and isinstance(result, dict):
        data = result.get("data", [])
        if data:
            bt = data[0].get("blockTime")
            if bt:
                return (time.time() - bt) / 86400
    return None


def get_asset_info(mint: str) -> Dict:
    """Get price, supply, decimals. Cost: 10 cr."""
    result = _rpc("getAsset", {"id": mint}, credits=10)
    if not result:
        return {}
    ti = result.get("token_info", {})
    price_info = ti.get("price_info", {})
    supply = int(ti.get("supply", 0))
    decimals = int(ti.get("decimals", 0))
    supply_human = supply / (10 ** decimals) if decimals > 0 else float(supply)
    return {
        "price_usd": price_info.get("price_per_token"),
        "supply": supply,
        "supply_human": supply_human,
        "decimals": decimals,
    }


# ── ANALYSIS ──

def compute_gini(values: List[float]) -> float:
    """Gini coefficient. 0 = perfect equality, 1 = one holder has everything."""
    if not values or len(values) < 2:
        return 0.0
    sorted_vals = sorted(values)
    n = len(sorted_vals)
    total = sum(sorted_vals)
    if total == 0:
        return 0.0
    cumulative = 0.0
    gini_sum = 0.0
    for i, v in enumerate(sorted_vals):
        cumulative += v
        gini_sum += (2 * (i + 1) - n - 1) * v
    return gini_sum / (n * total)


def analyze_token(
    mint: str,
    symbol: str,
    conviction: float,
    tier: str,
) -> TokenAnalysis:
    """Full analysis pipeline for one token."""
    errors: List[str] = []
    credits_start = _credits

    # 1. Asset info (price, supply, decimals)
    asset = get_asset_info(mint)
    price_usd = asset.get("price_usd")
    supply = asset.get("supply", 0)
    supply_human = asset.get("supply_human", 0)
    decimals = asset.get("decimals", 0)

    if supply == 0:
        errors.append("no supply data")
        return _empty(mint, symbol, conviction, tier, errors, _credits - credits_start)

    # 2. Token age
    age_days = get_token_age(mint)
    if age_days is None:
        errors.append("age fetch failed")
        age_days = 30.0

    longevity = min(age_days / 365.0, 1.0)

    # 3. Holder sample (3 pages × 100 = up to 300 random holders)
    accounts = get_token_accounts_sample(mint, pages=3)
    if not accounts:
        errors.append("no holder data from DAS")
        return _empty(mint, symbol, conviction, tier, errors, _credits - credits_start)

    # 4. Classify each holder
    dec_factor = 10 ** decimals if decimals > 0 else 1

    pools_filtered = 0
    zero_count = 0
    dust_count = 0
    real_count = 0
    active_count = 0
    balances_human: List[float] = []

    for acct in accounts:
        owner = acct["owner"]
        raw_amount = acct["amount"]

        # Filter pools
        if owner in POOL_PROGRAMS:
            pools_filtered += 1
            continue

        balance_human = raw_amount / dec_factor
        balance_usd = balance_human * price_usd if price_usd else None

        # Balance as % of supply
        balance_pct = (raw_amount / supply * 100) if supply > 0 else 0

        balances_human.append(balance_human)

        if raw_amount == 0:
            zero_count += 1
        elif balance_usd is not None:
            # USD-based classification
            if balance_usd < 0.01:
                dust_count += 1
            elif balance_usd >= 1.0:
                real_count += 1
                # Active = not a whale (< 1% of supply) and real balance
                if balance_pct < 1.0:
                    active_count += 1
            else:
                pass  # $0.01 - $1.00 = "micro" holder, neither dust nor real
        else:
            # No price → use supply % thresholds
            if balance_pct < 0.001:
                dust_count += 1
            elif balance_pct > 0.01:
                real_count += 1
                if balance_pct < 1.0:
                    active_count += 1

    # 5. Compute derived signals
    effective = len(accounts) - pools_filtered - zero_count
    dust_ratio = dust_count / effective if effective > 0 else 1.0
    organic_ratio = real_count / effective if effective > 0 else 0.0
    gini = compute_gini(balances_human)

    # 6. K-Score v2
    # Inverted dust_ratio: high dust = low score
    # organic_ratio: high organic = high score
    # longevity: old = higher score
    health = max(1.0 - dust_ratio, 0.01)
    kscore_v2 = (health ** 0.5) * (max(organic_ratio, 0.01) ** 0.35) * (max(longevity, 0.01) ** 0.15)

    return TokenAnalysis(
        mint=mint,
        symbol=symbol,
        conviction_cultscreener=conviction,
        conviction_tier=tier,
        total_holders_sampled=len(accounts),
        pools_filtered=pools_filtered,
        real_holders=real_count,
        dust_holders=dust_count,
        zero_holders=zero_count,
        active_holders=active_count,
        dust_ratio=dust_ratio,
        organic_ratio=organic_ratio,
        gini=gini,
        token_age_days=age_days,
        longevity=longevity,
        kscore_v2=kscore_v2,
        price_usd=price_usd,
        supply_human=supply_human,
        decimals=decimals,
        api_credits_used=_credits - credits_start,
        errors=errors,
    )


def _empty(mint: str, symbol: str, conv: float, tier: str,
           errors: List[str], credits: int) -> TokenAnalysis:
    return TokenAnalysis(
        mint=mint, symbol=symbol, conviction_cultscreener=conv, conviction_tier=tier,
        total_holders_sampled=0, pools_filtered=0, real_holders=0, dust_holders=0,
        zero_holders=0, active_holders=0, dust_ratio=1.0, organic_ratio=0.0, gini=0.0,
        token_age_days=None, longevity=0.0, kscore_v2=0.0,
        price_usd=None, supply_human=0, decimals=0,
        api_credits_used=credits, errors=errors,
    )


# ── CORRELATION ──

def spearman(x: List[float], y: List[float]) -> float:
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
            avg = (i + j) / 2.0 + 1.0
            for k in range(i, j + 1):
                r[indexed[k][0]] = avg
            i = j + 1
        return r

    rx, ry = ranks(x), ranks(y)
    d_sq = sum((a - b) ** 2 for a, b in zip(rx, ry))
    return 1.0 - (6.0 * d_sq) / (n * (n * n - 1))


# ── MAIN ──

def main() -> None:
    limit = 100
    dry_run = "--dry-run" in sys.argv
    if "--limit" in sys.argv:
        idx = sys.argv.index("--limit")
        if idx + 1 < len(sys.argv):
            limit = int(sys.argv[idx + 1])

    data_path = os.path.join(os.path.dirname(__file__), "calibration_results_real.json")
    with open(data_path) as f:
        calib = json.load(f)

    tokens = calib["results"][:limit]

    # Cost: getAsset(10) + getTokenAccounts×3pages(30) + age(10) = 50 cr/token
    est_cost = len(tokens) * 50

    print("=" * 70)
    print("SOVEREIGN K-SCORE v2 — Random Holder Sampling")
    print(f"Tokens: {len(tokens)}  |  Est. cost: ~{est_cost} cr  |  Plan: Developer")
    print("=" * 70)

    if dry_run:
        print("\n[DRY RUN]")
        for t in tokens:
            print(f"  {t['symbol']:12s}  conv={t['conviction']:.3f}  [{t['conviction_tier']}]")
        print(f"\nEstimated cost: ~{est_cost} credits")
        return

    results: List[TokenAnalysis] = []

    for i, token in enumerate(tokens):
        mint = token["mint"]
        symbol = token["symbol"]
        conv = token["conviction"]
        tier = token["conviction_tier"]

        print(f"\n[{i+1}/{len(tokens)}] {symbol:12s} (conv={conv:.3f}, {tier})")

        r = analyze_token(mint, symbol, conv, tier)
        results.append(r)

        eff = r.total_holders_sampled - r.pools_filtered - r.zero_holders
        print(f"  sampled={r.total_holders_sampled}  pools={r.pools_filtered}  zeros={r.zero_holders}")
        print(f"  dust={r.dust_holders} ({r.dust_ratio:.1%})  real={r.real_holders} ({r.organic_ratio:.1%})  "
              f"active={r.active_holders}  gini={r.gini:.3f}")
        print(f"  age={r.token_age_days:.0f}d  price={'$'+f'{r.price_usd:.8f}' if r.price_usd else 'N/A'}")
        print(f"  K_v2={r.kscore_v2:.3f}")
        if r.errors:
            for e in r.errors:
                print(f"  ⚠ {e}")

    # ── ANALYSIS ──
    valid = [r for r in results if r.total_holders_sampled > 0]

    print("\n" + "=" * 70)
    print(f"CORRELATION ANALYSIS ({len(valid)}/{len(results)} tokens)")
    print("=" * 70)

    if len(valid) < 5:
        print("Not enough data.")
        _save(results)
        return

    convictions = [r.conviction_cultscreener for r in valid]
    kscores = [r.kscore_v2 for r in valid]
    dust_ratios = [r.dust_ratio for r in valid]
    organic_ratios = [r.organic_ratio for r in valid]
    ginis = [r.gini for r in valid]
    longevities = [r.longevity for r in valid]
    active_counts = [float(r.active_holders) for r in valid]

    print("\nSPEARMAN ρ (vs CultScreener conviction):")
    print(f"  ρ > 0.5 = SOVEREIGN  |  ρ < 0.3 = NOISE")
    print()

    for name, vals in [
        ("kscore_v2", kscores),
        ("dust_ratio", dust_ratios),
        ("organic_ratio", organic_ratios),
        ("gini", ginis),
        ("longevity", longevities),
        ("active_holders", active_counts),
    ]:
        rho = spearman(vals, convictions)
        # dust_ratio is INVERSE (higher = worse), so expect negative ρ
        signal = "SOVEREIGN" if abs(rho) > 0.5 else ("weak" if abs(rho) > 0.3 else "NOISE")
        print(f"  {name:18s}  ρ = {rho:+.3f}  [{signal}]")

    # Per-tier distributions
    print("\nPER-TIER DISTRIBUTIONS:")
    for tier in ["strong", "mixed", "weak"]:
        td = [r for r in valid if r.conviction_tier == tier]
        if not td:
            continue
        dust = [r.dust_ratio for r in td]
        org = [r.organic_ratio for r in td]
        ks = [r.kscore_v2 for r in td]
        gi = [r.gini for r in td]
        print(f"\n  {tier.upper()} (n={len(td)}):")
        print(f"    dust_ratio:   avg={sum(dust)/len(dust):.3f}  [{min(dust):.3f} - {max(dust):.3f}]")
        print(f"    organic:      avg={sum(org)/len(org):.3f}  [{min(org):.3f} - {max(org):.3f}]")
        print(f"    gini:         avg={sum(gi)/len(gi):.3f}  [{min(gi):.3f} - {max(gi):.3f}]")
        print(f"    kscore_v2:    avg={sum(ks)/len(ks):.3f}  [{min(ks):.3f} - {max(ks):.3f}]")

    # Scatter
    print("\nSCATTER (conviction vs dust_ratio):")
    sorted_valid = sorted(valid, key=lambda r: -r.conviction_cultscreener)
    for r in sorted_valid:
        tm = {"strong": "S", "mixed": "M", "weak": "W"}[r.conviction_tier]
        dust_bar = "░" * min(int(r.dust_ratio * 40), 40)
        print(f"  {r.symbol:12s} conv={r.conviction_cultscreener:.3f} [{tm}] "
              f"dust={r.dust_ratio:.3f} {dust_bar}  "
              f"org={r.organic_ratio:.3f} K={r.kscore_v2:.3f}")

    print(f"\nAPI COST: {_credits:,} credits ({_calls} calls)")
    _save(results)


def _save(results: List[TokenAnalysis]) -> None:
    out = os.path.join(os.path.dirname(__file__), "kscore_v2_results.json")
    with open(out, "w") as f:
        json.dump({
            "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S"),
            "total_tokens": len(results),
            "valid_tokens": sum(1 for r in results if r.total_holders_sampled > 0),
            "total_credits": _credits,
            "total_calls": _calls,
            "results": [asdict(r) for r in results],
        }, f, indent=2)
    print(f"Saved: {out}")


if __name__ == "__main__":
    main()
