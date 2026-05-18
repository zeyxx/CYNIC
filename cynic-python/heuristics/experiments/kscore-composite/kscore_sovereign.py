#!/usr/bin/env python3
"""Sovereign K-Score: behavioral signal from Helius on-chain data.

Goal: Compute our own conviction proxy (K-Score) for 33 calibration tokens,
then measure correlation with CultScreener conviction (Spearman ρ).

If ρ > 0.5 → we have a sovereign behavioral signal, drop CultScreener dependency.
If ρ < 0.3 → static + behavioral on-chain data insufficient, need different approach.

Cost: ~2,050 credits (0.02% of Developer plan 10M/month).

K-Score formula (HolDex v8):
  K = DiamondHands^0.5 × OrganicGrowth^0.35 × Longevity^0.15

  DiamondHands = (accumulators + holders) / analyzed_count
  OrganicGrowth = real_holders / total_holders (real = balance > $1 USD)
  Longevity = min(token_age_days / 365, 1.0)

Usage:
    python3 kscore_sovereign.py
    python3 kscore_sovereign.py --limit 5   # quick test on 5 tokens
    python3 kscore_sovereign.py --dry-run    # show plan, no API calls
"""

import json
import os
import sys
import time
from dataclasses import dataclass, field, asdict
from typing import Optional, List, Dict, Tuple

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
    print("ERROR: HELIUS_API_KEY not set (check ~/.cynic-env)")
    sys.exit(1)

HELIUS_RPC = f"https://mainnet.helius-rpc.com/?api-key={HELIUS_API_KEY}"

# ── KNOWN POOL / DEX ADDRESSES (filter from holder analysis) ──

POOL_PROGRAMS: set[str] = {
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",  # Raydium AMM
    "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",  # Raydium CLMM
    "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc",   # Orca Whirlpool
    "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP",  # Orca v1
    "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P",   # PumpFun bonding curve
    "39azUYFWPz3VHgKCf3VChUwbpURdCHRxjWVowf5jUJjg",  # PumpFun fee
    "TSLvdd1pWpHVjahSpsvCXUbgwsL3JAcvokwaKt1eokM",   # PumpFun migration authority
    "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4",   # Jupiter v6
    "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB",   # Jupiter v4
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1",  # Raydium v4
    "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo",   # Meteora DLMM
    "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB",  # Meteora pools
}

# Known exchange cold wallets (partial list)
EXCHANGE_WALLETS: set[str] = {
    "5tzFkiKscjHsFCaxGtPiSbkXYnXhzGfJMg6Q5qgzMEm4",  # Binance
    "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",  # FTX (defunct but still holds)
}


# ── DATA STRUCTURES ──

@dataclass
class HolderClassification:
    """Classification of a single holder's behavior."""
    wallet: str
    balance_raw: int
    balance_pct: float  # % of total supply
    net_flow_30d: float  # positive = buying, negative = selling
    tx_count_30d: int
    classification: str  # accumulator | holder | reducer | extractor
    is_pool: bool


@dataclass
class KScoreResult:
    """K-Score computation result for a single token."""
    mint: str
    symbol: str
    conviction_cultscreener: float
    conviction_tier: str

    # K-Score pillars
    diamond_hands: float  # 0-1
    organic_growth: float  # 0-1
    longevity: float  # 0-1
    kscore: float  # composite

    # Raw data
    token_age_days: Optional[float]
    total_holders_sampled: int
    real_holders: int  # balance > $1
    holders_analyzed: int
    accumulators: int
    holders_passive: int
    reducers: int
    extractors: int
    pools_filtered: int
    price_usd: Optional[float]

    # Data quality
    holder_data_available: bool
    age_data_available: bool
    behavior_data_available: bool
    api_credits_used: int
    errors: List[str] = field(default_factory=list)


# ── HELIUS API ──

_credits_used = 0
_calls_made = 0


def helius_rpc(method: str, params, credits: int = 1) -> Optional[dict]:
    """Call Helius JSON-RPC with rate limiting and cost tracking."""
    global _credits_used, _calls_made

    try:
        resp = requests.post(HELIUS_RPC, json={
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        }, timeout=30)
        data = resp.json()
        _calls_made += 1
        _credits_used += credits

        if "error" in data:
            err = data["error"]
            msg = err.get("message", str(err)) if isinstance(err, dict) else str(err)
            return None

        # Rate limit: 10 req/s on Developer plan
        time.sleep(0.12)
        return data.get("result")

    except Exception as e:
        return None


def get_token_largest_accounts(mint: str) -> Optional[List[Dict]]:
    """Get top 20 holders by balance. Cost: 1 cr."""
    result = helius_rpc("getTokenLargestAccounts", [mint], credits=1)
    if result and isinstance(result, dict):
        return result.get("value", [])
    return None


def get_multiple_accounts(addresses: List[str]) -> Optional[List[Dict]]:
    """Batch resolve account addresses to parsed data. Cost: 1 cr."""
    result = helius_rpc("getMultipleAccounts", [
        addresses,
        {"encoding": "jsonParsed"}
    ], credits=1)
    if result and isinstance(result, dict):
        return result.get("value", [])
    return None


def get_token_age(mint: str) -> Optional[float]:
    """Get token age in days via oldest transaction. Cost: 10 cr."""
    result = helius_rpc("getTransactionsForAddress", [
        mint,
        {
            "sortOrder": "asc",
            "limit": 1,
            "transactionDetails": "signatures",
        }
    ], credits=10)

    if result and isinstance(result, dict):
        data = result.get("data", [])
        if data:
            block_time = data[0].get("blockTime")
            if block_time:
                now = time.time()
                age_days = (now - block_time) / 86400
                return age_days
    return None


def get_asset_price(mint: str) -> Optional[float]:
    """Get token price via DAS getAsset. Cost: 10 cr."""
    result = helius_rpc("getAsset", {"id": mint}, credits=10)
    if result:
        price_info = result.get("token_info", {}).get("price_info", {})
        return price_info.get("price_per_token")
    return None


def get_holder_transactions(wallet: str, limit: int = 50) -> Optional[List[Dict]]:
    """Get recent transactions for a wallet with token balance changes.

    Cost: 10 cr per 100 txs returned.
    """
    result = helius_rpc("getTransactionsForAddress", [
        wallet,
        {
            "sortOrder": "desc",
            "limit": limit,
            "transactionDetails": "full",
            "encoding": "jsonParsed",
            "tokenAccounts": "balanceChanged",
            "maxSupportedTransactionVersion": 0,
        }
    ], credits=10)

    if result and isinstance(result, dict):
        return result.get("data", [])
    return None


# ── K-SCORE COMPUTATION ──

def resolve_token_account_owners(token_accounts: List[Dict]) -> Dict[str, str]:
    """Resolve token account addresses → owner wallet addresses.

    Returns: {token_account_address: owner_wallet_address}
    """
    addresses = [acc.get("address", "") for acc in token_accounts if acc.get("address")]
    if not addresses:
        return {}

    # Batch in groups of 100
    mapping: Dict[str, str] = {}
    for i in range(0, len(addresses), 100):
        batch = addresses[i:i+100]
        accounts = get_multiple_accounts(batch)
        if not accounts:
            continue

        for addr, acct_data in zip(batch, accounts):
            if acct_data and isinstance(acct_data, dict):
                parsed = acct_data.get("data", {})
                if isinstance(parsed, dict):
                    parsed_info = parsed.get("parsed", {})
                    if isinstance(parsed_info, dict):
                        info = parsed_info.get("info", {})
                        owner = info.get("owner")
                        if owner:
                            mapping[addr] = owner

    return mapping


def classify_holder(
    wallet: str,
    current_balance: int,
    total_supply: int,
    mint: str,
    decimals: int,
) -> HolderClassification:
    """Classify a holder's behavior based on recent transaction history.

    Looks at token balance changes in last 50 transactions to determine
    if the wallet is accumulating, holding, reducing, or extracting.
    """
    balance_pct = (current_balance / total_supply * 100) if total_supply > 0 else 0

    # Check if this is a pool/DEX address
    if wallet in POOL_PROGRAMS or wallet in EXCHANGE_WALLETS:
        return HolderClassification(
            wallet=wallet,
            balance_raw=current_balance,
            balance_pct=balance_pct,
            net_flow_30d=0,
            tx_count_30d=0,
            classification="pool",
            is_pool=True,
        )

    txs = get_holder_transactions(wallet, limit=50)
    if not txs:
        # No tx data → assume passive holder
        return HolderClassification(
            wallet=wallet,
            balance_raw=current_balance,
            balance_pct=balance_pct,
            net_flow_30d=0,
            tx_count_30d=0,
            classification="holder",
            is_pool=False,
        )

    # Parse token balance changes for our specific mint
    net_flow = 0.0
    tx_count = 0
    cutoff = time.time() - (30 * 86400)  # 30 days ago

    for tx_wrapper in txs:
        block_time = tx_wrapper.get("blockTime", 0)
        if block_time and block_time < cutoff:
            continue  # older than 30 days

        tx_data = tx_wrapper.get("transaction", {})
        meta = tx_wrapper.get("meta", {})
        if not meta:
            continue

        pre_balances = meta.get("preTokenBalances", [])
        post_balances = meta.get("postTokenBalances", [])

        # Find our mint in pre and post balances
        pre_amount = 0
        post_amount = 0

        for bal in pre_balances:
            if bal.get("mint") == mint and bal.get("owner") == wallet:
                ui_amount = bal.get("uiTokenAmount", {})
                pre_amount = float(ui_amount.get("uiAmount", 0) or 0)

        for bal in post_balances:
            if bal.get("mint") == mint and bal.get("owner") == wallet:
                ui_amount = bal.get("uiTokenAmount", {})
                post_amount = float(ui_amount.get("uiAmount", 0) or 0)

        delta = post_amount - pre_amount
        if abs(delta) > 0:
            net_flow += delta
            tx_count += 1

    # Classify based on net flow relative to current balance
    current_ui = current_balance / (10 ** decimals) if decimals > 0 else current_balance

    if current_ui == 0:
        classification = "extractor"
    elif tx_count == 0:
        classification = "holder"  # no recent activity = passive holder
    elif net_flow > 0:
        classification = "accumulator"  # net buying
    elif net_flow > -(current_ui * 0.5):
        classification = "reducer"  # sold less than 50%
    else:
        classification = "extractor"  # sold more than 50%

    return HolderClassification(
        wallet=wallet,
        balance_raw=current_balance,
        balance_pct=balance_pct,
        net_flow_30d=net_flow,
        tx_count_30d=tx_count,
        classification=classification,
        is_pool=False,
    )


def compute_kscore(
    mint: str,
    symbol: str,
    conviction: float,
    conviction_tier: str,
    total_supply: int,
    decimals: int,
) -> KScoreResult:
    """Compute sovereign K-Score for a single token.

    Pipeline:
    1. Get top holders → filter pools → classify behavior (DiamondHands)
    2. Get price → count real holders (OrganicGrowth)
    3. Get token age (Longevity)
    4. Combine: K = DH^0.5 × OG^0.35 × L^0.15
    """
    errors: List[str] = []
    credits_start = _credits_used

    # ── 1. Top holders ──
    top_accounts = get_token_largest_accounts(mint)
    holder_data_available = top_accounts is not None and len(top_accounts or []) > 0

    if not holder_data_available:
        errors.append("getTokenLargestAccounts failed")
        return _empty_result(mint, symbol, conviction, conviction_tier, errors, _credits_used - credits_start)

    # Resolve token accounts → owner wallets
    owner_map = resolve_token_account_owners(top_accounts)

    # ── 2. Token age ──
    age_days = get_token_age(mint)
    age_data_available = age_days is not None

    if not age_data_available:
        errors.append("token age fetch failed")
        age_days = 30.0  # fallback: assume 30 days

    # ── 3. Price (for organic growth calculation) ──
    price_usd = get_asset_price(mint)

    # ── 4. Classify top holders ──
    classifications: List[HolderClassification] = []
    pools_filtered = 0
    real_holders = 0

    for acct in (top_accounts or []):
        token_acct_addr = acct.get("address", "")
        balance_raw = int(acct.get("amount", "0"))
        owner = owner_map.get(token_acct_addr)

        if not owner:
            errors.append(f"could not resolve owner for {token_acct_addr[:8]}...")
            continue

        # Check if pool
        if owner in POOL_PROGRAMS:
            pools_filtered += 1
            continue

        # Count real holders (balance > $1 USD)
        if price_usd and decimals > 0:
            balance_usd = (balance_raw / (10 ** decimals)) * price_usd
            if balance_usd > 1.0:
                real_holders += 1
        else:
            real_holders += 1  # assume real if no price data

        # Only analyze behavior for top 5 non-pool holders
        if len(classifications) < 5:
            c = classify_holder(owner, balance_raw, total_supply, mint, decimals)
            classifications.append(c)

    behavior_data_available = len(classifications) > 0
    analyzed = len(classifications)

    # ── 5. Compute K-Score pillars ──

    # DiamondHands: (accumulators + passive holders) / analyzed
    accumulators = sum(1 for c in classifications if c.classification == "accumulator")
    holders_passive = sum(1 for c in classifications if c.classification == "holder")
    reducers = sum(1 for c in classifications if c.classification == "reducer")
    extractors = sum(1 for c in classifications if c.classification == "extractor")

    diamond_hands = (accumulators + holders_passive) / analyzed if analyzed > 0 else 0.5

    # OrganicGrowth: real_holders / total_sampled
    total_sampled = len(top_accounts or []) - pools_filtered
    organic_growth = real_holders / total_sampled if total_sampled > 0 else 0.5

    # Longevity: min(age_days / 365, 1.0)
    longevity = min((age_days or 30) / 365.0, 1.0)

    # Composite: K = DH^0.5 × OG^0.35 × L^0.15
    kscore = (diamond_hands ** 0.5) * (organic_growth ** 0.35) * (longevity ** 0.15)

    return KScoreResult(
        mint=mint,
        symbol=symbol,
        conviction_cultscreener=conviction,
        conviction_tier=conviction_tier,
        diamond_hands=diamond_hands,
        organic_growth=organic_growth,
        longevity=longevity,
        kscore=kscore,
        token_age_days=age_days,
        total_holders_sampled=total_sampled,
        real_holders=real_holders,
        holders_analyzed=analyzed,
        accumulators=accumulators,
        holders_passive=holders_passive,
        reducers=reducers,
        extractors=extractors,
        pools_filtered=pools_filtered,
        price_usd=price_usd,
        holder_data_available=holder_data_available,
        age_data_available=age_data_available,
        behavior_data_available=behavior_data_available,
        api_credits_used=_credits_used - credits_start,
        errors=errors,
    )


def _empty_result(mint: str, symbol: str, conviction: float, tier: str,
                  errors: List[str], credits: int) -> KScoreResult:
    return KScoreResult(
        mint=mint, symbol=symbol,
        conviction_cultscreener=conviction, conviction_tier=tier,
        diamond_hands=0, organic_growth=0, longevity=0, kscore=0,
        token_age_days=None, total_holders_sampled=0, real_holders=0,
        holders_analyzed=0, accumulators=0, holders_passive=0,
        reducers=0, extractors=0, pools_filtered=0, price_usd=None,
        holder_data_available=False, age_data_available=False,
        behavior_data_available=False, api_credits_used=credits,
        errors=errors,
    )


# ── CORRELATION ──

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


# ── MAIN ──

def main() -> None:
    # Parse args
    limit = 100
    dry_run = "--dry-run" in sys.argv
    if "--limit" in sys.argv:
        idx = sys.argv.index("--limit")
        if idx + 1 < len(sys.argv):
            limit = int(sys.argv[idx + 1])

    # Load calibration data
    data_path = os.path.join(os.path.dirname(__file__), "calibration_results_real.json")
    with open(data_path) as f:
        calib = json.load(f)

    tokens = calib["results"][:limit]

    print("=" * 70)
    print("SOVEREIGN K-SCORE CALIBRATION")
    print(f"Tokens: {len(tokens)}  |  Plan: Developer  |  Budget: ~{len(tokens) * 62} cr")
    print("=" * 70)

    if dry_run:
        print("\n[DRY RUN] Would process:")
        for t in tokens:
            print(f"  {t['symbol']:12s}  conv={t['conviction']:.3f}  [{t['conviction_tier']}]")
        print(f"\nEstimated cost: ~{len(tokens) * 62} credits")
        return

    # We need total_supply and decimals per token.
    # Fetch via getAsset (already used for price).
    # To avoid double-fetching, we'll get price + supply in the same call.

    results: List[KScoreResult] = []

    for i, token in enumerate(tokens):
        mint = token["mint"]
        symbol = token["symbol"]
        conviction = token["conviction"]
        tier = token["conviction_tier"]

        print(f"\n[{i+1}/{len(tokens)}] {symbol:12s} (conv={conviction:.3f}, {tier})")

        # Get supply info from getAsset
        asset = helius_rpc("getAsset", {"id": mint}, credits=10)
        total_supply = 0
        decimals = 0
        if asset:
            ti = asset.get("token_info", {})
            total_supply = int(ti.get("supply", 0))
            decimals = int(ti.get("decimals", 0))

        if total_supply == 0:
            print(f"  ⚠ no supply data — skipping")
            results.append(_empty_result(mint, symbol, conviction, tier,
                                         ["no supply data"], 10))
            continue

        # Compute K-Score
        result = compute_kscore(mint, symbol, conviction, tier, total_supply, decimals)
        results.append(result)

        # Print summary
        print(f"  K={result.kscore:.3f}  DH={result.diamond_hands:.2f}  "
              f"OG={result.organic_growth:.2f}  L={result.longevity:.2f}")
        print(f"  holders: {result.holders_analyzed} analyzed  "
              f"(acc={result.accumulators} hold={result.holders_passive} "
              f"red={result.reducers} ext={result.extractors})  "
              f"pools={result.pools_filtered}")
        if result.token_age_days:
            print(f"  age: {result.token_age_days:.1f} days  "
                  f"price: ${result.price_usd:.8f}" if result.price_usd else
                  f"  age: {result.token_age_days:.1f} days  price: N/A")
        if result.errors:
            for err in result.errors:
                print(f"  ⚠ {err}")

    # ── ANALYSIS ──
    valid = [r for r in results if r.behavior_data_available]

    print("\n" + "=" * 70)
    print(f"RESULTS ({len(valid)}/{len(results)} tokens with behavioral data)")
    print("=" * 70)

    if len(valid) < 3:
        print("Not enough data for correlation analysis.")
        _save_results(results)
        return

    # Spearman correlations
    convictions = [r.conviction_cultscreener for r in valid]
    kscores = [r.kscore for r in valid]
    diamond_hands_vals = [r.diamond_hands for r in valid]
    organic_vals = [r.organic_growth for r in valid]
    longevity_vals = [r.longevity for r in valid]

    print("\nSPEARMAN ρ (vs CultScreener conviction):")
    print(f"  ρ > 0.5 = SOVEREIGN SIGNAL | ρ < 0.3 = NOISE")
    print()

    for name, vals in [
        ("kscore", kscores),
        ("diamond_hands", diamond_hands_vals),
        ("organic_growth", organic_vals),
        ("longevity", longevity_vals),
    ]:
        rho = spearman_rank(vals, convictions)
        signal = "SOVEREIGN" if abs(rho) > 0.5 else ("weak" if abs(rho) > 0.3 else "NOISE")
        print(f"  {name:16s}  ρ = {rho:+.3f}  [{signal}]")

    # Per-tier K-Score distributions
    print("\nK-SCORE BY TIER:")
    for tier in ["strong", "mixed", "weak"]:
        tier_data = [r for r in valid if r.conviction_tier == tier]
        if not tier_data:
            continue
        ks = [r.kscore for r in tier_data]
        print(f"  {tier:8s} (n={len(tier_data)}): "
              f"avg={sum(ks)/len(ks):.3f}  "
              f"min={min(ks):.3f}  max={max(ks):.3f}")

    # Separability test
    strong_k = [r.kscore for r in valid if r.conviction_tier == "strong"]
    weak_k = [r.kscore for r in valid if r.conviction_tier == "weak"]
    if strong_k and weak_k:
        overlap = max(0, min(max(strong_k), max(weak_k)) - max(min(strong_k), min(weak_k)))
        total_range = max(max(strong_k), max(weak_k)) - min(min(strong_k), min(weak_k))
        overlap_pct = (overlap / total_range * 100) if total_range > 0 else 100
        print(f"\n  SEPARABILITY: strong [{min(strong_k):.3f}-{max(strong_k):.3f}] vs "
              f"weak [{min(weak_k):.3f}-{max(weak_k):.3f}]  overlap={overlap_pct:.0f}%")

    # Holder behavior scatter
    print("\nHOLDER BEHAVIOR SCATTER:")
    sorted_valid = sorted(valid, key=lambda r: -r.conviction_cultscreener)
    for r in sorted_valid:
        tier_mark = {"strong": "S", "mixed": "M", "weak": "W"}[r.conviction_tier]
        bar = "█" * int(r.kscore * 20)
        print(f"  {r.symbol:12s} conv={r.conviction_cultscreener:.3f} [{tier_mark}] "
              f"K={r.kscore:.3f} {bar}  "
              f"DH={r.diamond_hands:.2f} OG={r.organic_growth:.2f} L={r.longevity:.2f}")

    # Cost summary
    print(f"\nAPI COST: {_credits_used:,} credits ({_calls_made} calls)")

    _save_results(results)


def _save_results(results: List[KScoreResult]) -> None:
    """Save results to JSON for further analysis."""
    output_path = os.path.join(os.path.dirname(__file__), "kscore_calibration_results.json")
    serializable = []
    for r in results:
        d = asdict(r)
        serializable.append(d)

    with open(output_path, "w") as f:
        json.dump({
            "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S"),
            "total_tokens": len(results),
            "valid_tokens": sum(1 for r in results if r.behavior_data_available),
            "total_credits_used": _credits_used,
            "total_api_calls": _calls_made,
            "results": serializable,
        }, f, indent=2)
    print(f"\nSaved: {output_path}")


if __name__ == "__main__":
    main()
