#!/usr/bin/env python3
"""Sovereign Token Profiler — on-chain behavioral signal via mint-level flow analysis.

Collects and PERSISTS comprehensive token profiles from Helius:
1. Metadata (getAsset DAS) — name, supply, price, authorities
2. Token age (getTransactionsForAddress, asc, signatures)
3. Buy/sell flow (getTransactionsForAddress, desc, full, 1000 txs)
4. Top holders + pool filtering (getTokenLargestAccounts + getMultipleAccounts)
5. Holder distribution sample (getTokenAccounts DAS, 3 pages)

All data persisted to JSONL (append-only, one line per token per run).
Never re-fetches a token already profiled today.

Cost: ~152 cr/token. 33 tokens = ~5,000 cr (0.05% of Developer plan).

Usage:
    python3 sovereign_profiler.py                    # profile all 33 calibration tokens
    python3 sovereign_profiler.py --limit 3          # first 3 only
    python3 sovereign_profiler.py --mint <address>   # single token
    python3 sovereign_profiler.py --analyze          # analyze persisted data (0 API calls)
"""

import json
import os
import sys
import time
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from typing import Optional, List, Dict, Set, Tuple

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

HELIUS_RPC = "https://mainnet.helius-rpc.com/"
HELIUS_HEADERS = {"Content-Type": "application/json", "Authorization": f"Bearer {HELIUS_API_KEY}"}

# Paths
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
DATA_DIR = os.path.join(SCRIPT_DIR, "..", "data")
PROFILES_PATH = os.path.join(DATA_DIR, "token_profiles.jsonl")
CALIB_PATH = os.path.join(DATA_DIR, "calibration_results_real.json")

# ── POOL PROGRAMS ──

POOL_PROGRAMS: Set[str] = {
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",  # Raydium AMM
    "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",  # Raydium CLMM
    "5quBtoiQqxF9Jv6KYKctB59NT3gtJD2Y65kdnB1Uev3h",  # Raydium
    "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc",   # Orca Whirlpool
    "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P",   # PumpFun bonding curve
    "39azUYFWPz3VHgKCf3VChUwbpURdCHRxjWVowf5jUJjg",  # PumpFun fee
    "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA",   # PumpFun AMM
    "TSLvdd1pWpHVjahSpsvCXUbgwsL3JAcvokwaKt1eokM",   # PumpFun migration
    "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4",   # Jupiter v6
    "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB",   # Jupiter v4
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1",  # Raydium v4
    "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo",   # Meteora DLMM
    "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB",  # Meteora pools
    "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX",    # OpenBook/Serum
    "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin",  # Serum v3
    "opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb",   # OpenBook v2
}

# Known pool wallet addresses (not program owners but specific wallets)
KNOWN_POOL_WALLETS: Set[str] = {
    "CebN5WGQ4jvEPvsVU4EoHEpgzq1VV7AbicfhtW4xC9iM",  # PumpFun Fee
}

# Cache for pool checks (address → is_pool)
_pool_cache: Dict[str, bool] = {}


# ── API LAYER ──

_credits = 0
_calls = 0


def _rpc(method: str, params, credits: int = 1) -> Optional[dict]:
    """Helius JSON-RPC call with rate limiting."""
    global _credits, _calls
    try:
        resp = requests.post(HELIUS_RPC, json={
            "jsonrpc": "2.0", "id": 1,
            "method": method, "params": params,
        }, headers=HELIUS_HEADERS, timeout=30)
        data = resp.json()
        _calls += 1
        _credits += credits
        if "error" in data:
            return None
        time.sleep(0.12)  # 10 req/s rate limit
        return data.get("result")
    except Exception:
        return None


# ── DATA COLLECTION ──

def fetch_asset(mint: str) -> Dict:
    """getAsset DAS — metadata, price, supply, authorities. Cost: 10 cr."""
    result = _rpc("getAsset", {"id": mint}, credits=10)
    if not result:
        return {}

    ti = result.get("token_info", {})
    price_info = ti.get("price_info", {})
    supply_raw = int(ti.get("supply", 0))
    decimals = int(ti.get("decimals", 0))
    supply_human = supply_raw / (10 ** decimals) if decimals > 0 else float(supply_raw)

    content = result.get("content", {}).get("metadata", {})
    authorities = result.get("authorities", [])
    is_pump = any(a.get("address") == "TSLvdd1pWpHVjahSpsvCXUbgwsL3JAcvokwaKt1eokM"
                  for a in authorities) or mint.endswith("pump")

    return {
        "name": content.get("name"),
        "symbol": content.get("symbol"),
        "supply_raw": supply_raw,
        "supply_human": supply_human,
        "decimals": decimals,
        "price_usd": price_info.get("price_per_token"),
        "mint_authority_active": ti.get("mint_authority") is not None,
        "freeze_authority_active": ti.get("freeze_authority") is not None,
        "origin_pump_fun": is_pump,
    }


def fetch_token_age(mint: str) -> Optional[float]:
    """Token age in days via oldest transaction. Cost: 10 cr."""
    result = _rpc("getTransactionsForAddress", [mint, {
        "sortOrder": "asc",
        "limit": 1,
        "transactionDetails": "signatures",
    }], credits=10)
    if result and isinstance(result, dict):
        data = result.get("data", [])
        if data:
            bt = data[0].get("blockTime")
            if bt:
                return (time.time() - bt) / 86400
    return None


def fetch_buy_sell_flow(mint: str, supply_raw: int, decimals: int,
                        limit: int = 1000) -> Dict:
    """Buy/sell flow from last N transactions of the token.

    Cost: 10 cr per 100 txs returned (min 10 cr).
    For 1000 txs = ~100 cr max.
    """
    credits_est = max(10, (limit // 100) * 10)
    result = _rpc("getTransactionsForAddress", [mint, {
        "transactionDetails": "full",
        "sortOrder": "desc",
        "limit": limit,
        "filters": {
            "status": "succeeded",
            "tokenAccounts": "balanceChanged",
        },
        "maxSupportedTransactionVersion": 0,
    }], credits=credits_est)

    if not result or not isinstance(result, dict):
        return {"error": "no data", "txs_fetched": 0}

    txs = result.get("data", [])
    dec_factor = 10 ** decimals if decimals > 0 else 1
    supply_human = supply_raw / dec_factor if dec_factor > 0 else 1.0

    # Parse all token balance changes
    buyers: Dict[str, float] = {}   # wallet → total tokens received
    sellers: Dict[str, float] = {}  # wallet → total tokens sent
    pool_wallets: Set[str] = set()
    timestamps: List[int] = []
    txs_with_flow = 0

    for tx_wrapper in txs:
        bt = tx_wrapper.get("blockTime")
        if bt:
            timestamps.append(bt)

        meta = tx_wrapper.get("meta", {})
        if not meta:
            continue

        pre_bals = meta.get("preTokenBalances", [])
        post_bals = meta.get("postTokenBalances", [])

        # Index by accountIndex for our mint
        pre_map: Dict[int, Tuple[str, float]] = {}
        for b in pre_bals:
            if b.get("mint") == mint:
                idx = b.get("accountIndex", -1)
                owner = b.get("owner", "")
                amt = float(b.get("uiTokenAmount", {}).get("uiAmount", 0) or 0)
                pre_map[idx] = (owner, amt)

        post_map: Dict[int, Tuple[str, float]] = {}
        for b in post_bals:
            if b.get("mint") == mint:
                idx = b.get("accountIndex", -1)
                owner = b.get("owner", "")
                amt = float(b.get("uiTokenAmount", {}).get("uiAmount", 0) or 0)
                post_map[idx] = (owner, amt)

        all_indices = set(list(pre_map.keys()) + list(post_map.keys()))
        tx_had_flow = False

        for idx in all_indices:
            pre_owner, pre_amt = pre_map.get(idx, ("", 0.0))
            post_owner, post_amt = post_map.get(idx, ("", 0.0))
            owner = post_owner or pre_owner
            if not owner:
                continue

            delta = post_amt - pre_amt
            if abs(delta) < 1e-9:
                continue

            # Identify pools by checking cache or program owner
            is_pool = _is_pool(owner)
            if is_pool:
                pool_wallets.add(owner)
                continue

            tx_had_flow = True
            if delta > 0:
                buyers[owner] = buyers.get(owner, 0) + delta
            else:
                sellers[owner] = sellers.get(owner, 0) + abs(delta)

        if tx_had_flow:
            txs_with_flow += 1

    # Compute metrics
    n_buyers = len(buyers)
    n_sellers = len(sellers)
    total_bought = sum(buyers.values())
    total_sold = sum(sellers.values())

    buy_sell_ratio = n_buyers / n_sellers if n_sellers > 0 else (10.0 if n_buyers > 0 else 1.0)
    volume_ratio = total_bought / total_sold if total_sold > 0 else (10.0 if total_bought > 0 else 1.0)

    # Time span
    time_span_hours = (max(timestamps) - min(timestamps)) / 3600.0 if len(timestamps) >= 2 else 0.0
    tx_per_hour = len(txs) / max(time_span_hours, 0.01) if time_span_hours > 0 else 0.0

    # Whale vs retail thresholds
    whale_threshold = supply_human * 0.01    # > 1% of supply
    retail_threshold = supply_human * 0.0001  # < 0.01% of supply

    whale_buys = sum(1 for v in buyers.values() if v > whale_threshold)
    whale_sells = sum(1 for v in sellers.values() if v > whale_threshold)
    retail_buys = sum(1 for v in buyers.values() if v < retail_threshold)
    retail_sells = sum(1 for v in sellers.values() if v < retail_threshold)

    # Unique wallets
    all_wallets = set(buyers.keys()) | set(sellers.keys())
    # Wallets that both buy AND sell (flippers)
    flippers = set(buyers.keys()) & set(sellers.keys())

    return {
        "txs_fetched": len(txs),
        "txs_with_flow": txs_with_flow,
        "time_span_hours": round(time_span_hours, 2),
        "tx_per_hour": round(tx_per_hour, 2),
        "unique_buyers": n_buyers,
        "unique_sellers": n_sellers,
        "buy_sell_ratio": round(buy_sell_ratio, 4),
        "total_bought": round(total_bought, 4),
        "total_sold": round(total_sold, 4),
        "volume_ratio": round(volume_ratio, 4),
        "unique_wallets": len(all_wallets),
        "flippers": len(flippers),
        "pool_wallets_seen": len(pool_wallets),
        "whale_buys": whale_buys,
        "whale_sells": whale_sells,
        "retail_buys": retail_buys,
        "retail_sells": retail_sells,
        "oldest_tx_time": min(timestamps) if timestamps else None,
        "newest_tx_time": max(timestamps) if timestamps else None,
    }


def fetch_top_holders(mint: str) -> Dict:
    """Top 20 holders + pool filtering. Cost: 2 cr."""
    accounts = _rpc("getTokenLargestAccounts", [mint], credits=1)
    if not accounts or not isinstance(accounts, dict):
        return {"error": "getTokenLargestAccounts failed"}

    token_accounts = accounts.get("value", [])
    if not token_accounts:
        return {"error": "no holders returned"}

    # Resolve token accounts → owner wallets
    addresses = [a.get("address", "") for a in token_accounts if a.get("address")]
    owner_map = _resolve_owners(addresses)

    holders = []
    pools_filtered = 0

    for acct in token_accounts:
        addr = acct.get("address", "")
        amount_raw = int(acct.get("amount", "0"))
        decimals = acct.get("decimals", 0)
        amount_human = float(acct.get("uiAmountString", "0") or "0")
        owner = owner_map.get(addr, "unknown")

        is_pool = _is_pool(owner)
        if is_pool:
            pools_filtered += 1

        holders.append({
            "token_account": addr,
            "owner": owner,
            "amount_raw": amount_raw,
            "amount_human": amount_human,
            "is_pool": is_pool,
        })

    return {
        "holders": holders,
        "total_accounts": len(token_accounts),
        "pools_filtered": pools_filtered,
    }


def fetch_holder_sample(mint: str, pages: int = 3) -> Dict:
    """Random holder sample via DAS. Cost: 10 cr/page."""
    all_accounts: List[Dict] = []

    for page in range(1, pages + 1):
        result = _rpc("getTokenAccounts", {
            "mint": mint,
            "page": page,
            "limit": 1000,
        }, credits=10)
        if not result:
            break
        accounts = result.get("token_accounts", [])
        if not accounts:
            break
        for acct in accounts:
            owner = acct.get("owner")
            amount = acct.get("amount")
            if owner and amount is not None:
                all_accounts.append({
                    "owner": owner,
                    "amount": int(amount) if isinstance(amount, (int, float)) else int(str(amount)),
                })

    return {
        "sampled": len(all_accounts),
        "pages_fetched": min(pages, len(all_accounts) // 1000 + 1) if all_accounts else 0,
        "accounts": all_accounts,
    }


# ── POOL DETECTION ──

def _resolve_owners(token_account_addresses: List[str]) -> Dict[str, str]:
    """Batch resolve token account → owner wallet. Cost: 1 cr per 100."""
    mapping: Dict[str, str] = {}
    for i in range(0, len(token_account_addresses), 100):
        batch = token_account_addresses[i:i+100]
        result = _rpc("getMultipleAccounts", [batch, {"encoding": "jsonParsed"}], credits=1)
        if not result or not isinstance(result, dict):
            continue
        values = result.get("value", [])
        for addr, acct in zip(batch, values):
            if acct and isinstance(acct, dict):
                parsed = acct.get("data", {})
                if isinstance(parsed, dict):
                    info = parsed.get("parsed", {}).get("info", {})
                    owner = info.get("owner")
                    if owner:
                        mapping[addr] = owner
    return mapping


def _is_pool(address: str) -> bool:
    """Check if address is a known pool/DEX program or cached pool wallet."""
    if address in POOL_PROGRAMS or address in KNOWN_POOL_WALLETS:
        return True
    return _pool_cache.get(address, False)


# ── PERSISTENCE ──

def load_existing_profiles(date_str: str) -> Dict[str, dict]:
    """Load profiles already collected today (avoid re-fetching)."""
    existing: Dict[str, dict] = {}
    if not os.path.exists(PROFILES_PATH):
        return existing
    with open(PROFILES_PATH) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                profile = json.loads(line)
                if profile.get("date") == date_str:
                    existing[profile["mint"]] = profile
            except json.JSONDecodeError:
                continue
    return existing


def save_profile(profile: dict) -> None:
    """Append one profile to JSONL."""
    with open(PROFILES_PATH, "a") as f:
        f.write(json.dumps(profile, default=str) + "\n")


# ── PROFILE BUILDER ──

def build_profile(mint: str, symbol: str = "", conviction: float = 0.0,
                  conviction_tier: str = "") -> dict:
    """Full profiling pipeline for one token."""
    errors: List[str] = []
    credits_start = _credits

    # 1. Asset metadata
    asset = fetch_asset(mint)
    if not asset or asset.get("supply_raw", 0) == 0:
        errors.append("no asset data")
        return {"mint": mint, "symbol": symbol, "error": "no asset data",
                "date": _today(), "credits": _credits - credits_start}

    supply_raw = asset["supply_raw"]
    decimals = asset["decimals"]
    price_usd = asset.get("price_usd")

    # 2. Token age
    age_days = fetch_token_age(mint)
    if age_days is None:
        errors.append("age fetch failed")

    # 3. Buy/sell flow (the main behavioral signal)
    flow = fetch_buy_sell_flow(mint, supply_raw, decimals, limit=1000)

    # 4. Top holders + pool filtering
    top = fetch_top_holders(mint)

    # 5. Holder distribution sample
    sample = fetch_holder_sample(mint, pages=3)

    # Compute holder distribution metrics from sample
    holder_metrics = _compute_holder_metrics(
        sample.get("accounts", []), supply_raw, decimals, price_usd
    )

    # Build profile
    profile = {
        "schema_version": 1,
        "mint": mint,
        "symbol": symbol or asset.get("symbol", ""),
        "name": asset.get("name", ""),
        "date": _today(),
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "conviction_cultscreener": conviction,
        "conviction_tier": conviction_tier,

        # Asset metadata
        "supply_human": asset["supply_human"],
        "decimals": decimals,
        "price_usd": price_usd,
        "mint_authority_active": asset.get("mint_authority_active", False),
        "freeze_authority_active": asset.get("freeze_authority_active", False),
        "origin_pump_fun": asset.get("origin_pump_fun", False),

        # Age
        "token_age_days": round(age_days, 2) if age_days else None,

        # Buy/sell flow
        "flow": flow,

        # Top holders
        "top_holders": top,

        # Holder distribution
        "holder_sample": {
            "sampled": sample.get("sampled", 0),
            **holder_metrics,
        },

        # Cost tracking
        "api_credits": _credits - credits_start,
        "api_calls": _calls,
        "errors": errors,
    }

    return profile


def _compute_holder_metrics(accounts: List[Dict], supply_raw: int,
                            decimals: int, price_usd: Optional[float]) -> Dict:
    """Compute distribution metrics from holder sample."""
    if not accounts:
        return {"real_holders": 0, "dust_holders": 0, "zero_holders": 0}

    dec_factor = 10 ** decimals if decimals > 0 else 1
    pools = 0
    zeros = 0
    dust = 0
    real = 0
    balances: List[float] = []

    for acct in accounts:
        owner = acct["owner"]
        raw = acct["amount"]

        if owner in POOL_PROGRAMS or owner in KNOWN_POOL_WALLETS:
            pools += 1
            continue

        balance_human = raw / dec_factor
        balances.append(balance_human)

        if raw == 0:
            zeros += 1
        elif price_usd:
            usd = balance_human * price_usd
            if usd < 0.01:
                dust += 1
            elif usd >= 1.0:
                real += 1
        else:
            pct = (raw / supply_raw * 100) if supply_raw > 0 else 0
            if pct < 0.001:
                dust += 1
            elif pct > 0.01:
                real += 1

    effective = len(accounts) - pools - zeros
    dust_ratio = dust / effective if effective > 0 else 0
    organic_ratio = real / effective if effective > 0 else 0

    # Gini
    gini = _gini(balances) if balances else 0.0

    return {
        "pools": pools,
        "zero_holders": zeros,
        "dust_holders": dust,
        "real_holders": real,
        "effective": effective,
        "dust_ratio": round(dust_ratio, 4),
        "organic_ratio": round(organic_ratio, 4),
        "gini": round(gini, 4),
    }


def _gini(values: List[float]) -> float:
    if not values or len(values) < 2:
        return 0.0
    s = sorted(values)
    n = len(s)
    total = sum(s)
    if total == 0:
        return 0.0
    cum = 0.0
    g = 0.0
    for i, v in enumerate(s):
        cum += v
        g += (2 * (i + 1) - n - 1) * v
    return g / (n * total)


def _today() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%d")


# ── ANALYSIS (reads persisted data, 0 API calls) ──

def analyze() -> None:
    """Analyze persisted profiles — correlations, distributions, insights."""
    if not os.path.exists(PROFILES_PATH):
        print("No profiles found. Run profiler first.")
        return

    profiles: List[dict] = []
    with open(PROFILES_PATH) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                profiles.append(json.loads(line))
            except json.JSONDecodeError:
                continue

    # Deduplicate: keep latest per mint
    by_mint: Dict[str, dict] = {}
    for p in profiles:
        by_mint[p["mint"]] = p
    profiles = list(by_mint.values())

    # Filter to those with flow data
    valid = [p for p in profiles if p.get("flow", {}).get("txs_with_flow", 0) > 0
             and p.get("conviction_cultscreener", 0) > 0]

    print(f"Profiles: {len(profiles)} total, {len(valid)} with flow + conviction data")

    if len(valid) < 5:
        print("Not enough data for correlation analysis.")
        return

    # Extract signals
    conv = [p["conviction_cultscreener"] for p in valid]
    bsr = [p["flow"]["buy_sell_ratio"] for p in valid]
    vr = [p["flow"]["volume_ratio"] for p in valid]
    ub = [float(p["flow"]["unique_buyers"]) for p in valid]
    us = [float(p["flow"]["unique_sellers"]) for p in valid]
    uw = [float(p["flow"]["unique_wallets"]) for p in valid]
    tph = [p["flow"]["tx_per_hour"] for p in valid]
    age = [p.get("token_age_days") or 30 for p in valid]
    dust = [p.get("holder_sample", {}).get("dust_ratio", 0) for p in valid]
    organic = [p.get("holder_sample", {}).get("organic_ratio", 0) for p in valid]
    gini = [p.get("holder_sample", {}).get("gini", 0) for p in valid]
    flippers = [float(p["flow"].get("flippers", 0)) for p in valid]

    print("\n" + "=" * 70)
    print("SPEARMAN ρ (vs CultScreener conviction)")
    print("  ρ > 0.5 = SOVEREIGN  |  ρ < 0.3 = NOISE")
    print("=" * 70)

    signals = [
        ("buy_sell_ratio", bsr),
        ("volume_ratio", vr),
        ("unique_buyers", ub),
        ("unique_sellers", us),
        ("unique_wallets", uw),
        ("tx_per_hour", tph),
        ("flippers", flippers),
        ("token_age_days", age),
        ("dust_ratio", dust),
        ("organic_ratio", organic),
        ("gini", gini),
    ]

    for name, vals in signals:
        rho = _spearman(vals, conv)
        sig = "SOVEREIGN" if abs(rho) > 0.5 else ("weak" if abs(rho) > 0.3 else "NOISE")
        print(f"  {name:20s}  ρ = {rho:+.3f}  [{sig}]")

    # Per-tier distributions
    print("\nPER-TIER FLOW:")
    for tier in ["strong", "mixed", "weak"]:
        td = [p for p in valid if p.get("conviction_tier") == tier]
        if not td:
            continue
        bs = [p["flow"]["buy_sell_ratio"] for p in td]
        vrs = [p["flow"]["volume_ratio"] for p in td]
        tphs = [p["flow"]["tx_per_hour"] for p in td]
        print(f"\n  {tier.upper()} (n={len(td)}):")
        print(f"    buy_sell_ratio:  avg={sum(bs)/len(bs):.2f}  [{min(bs):.2f} - {max(bs):.2f}]")
        print(f"    volume_ratio:    avg={sum(vrs)/len(vrs):.2f}  [{min(vrs):.2f} - {max(vrs):.2f}]")
        print(f"    tx/hour:         avg={sum(tphs)/len(tphs):.1f}  [{min(tphs):.1f} - {max(tphs):.1f}]")

    # Scatter
    print("\nSCATTER:")
    for p in sorted(valid, key=lambda p: -p["conviction_cultscreener"]):
        tm = {"strong": "S", "mixed": "M", "weak": "W"}.get(p.get("conviction_tier", "?"), "?")
        f = p["flow"]
        b = min(int(f["buy_sell_ratio"] * 5), 20)
        bar = "▓" * b if f["buy_sell_ratio"] >= 1 else "░" * min(int(5 / max(f["buy_sell_ratio"], 0.01)), 20)
        print(f"  {p.get('symbol','?'):12s} conv={p['conviction_cultscreener']:.3f} [{tm}] "
              f"B/S={f['buy_sell_ratio']:5.2f} {bar}  "
              f"vol={f['volume_ratio']:6.2f}  w={f['unique_wallets']:3d}  tx/h={f['tx_per_hour']:6.1f}")

    # Total cost
    total_cr = sum(p.get("api_credits", 0) for p in profiles)
    print(f"\nTotal API cost across all profiles: {total_cr:,} credits")


def _spearman(x: List[float], y: List[float]) -> float:
    n = len(x)
    if n < 3:
        return 0.0
    def ranks(vals):
        indexed = sorted(enumerate(vals), key=lambda t: t[1])
        r = [0.0] * n
        i = 0
        while i < n:
            j = i
            while j < n - 1 and indexed[j+1][1] == indexed[j][1]:
                j += 1
            avg = (i + j) / 2.0 + 1.0
            for k in range(i, j+1):
                r[indexed[k][0]] = avg
            i = j + 1
        return r
    rx, ry = ranks(x), ranks(y)
    d_sq = sum((a-b)**2 for a, b in zip(rx, ry))
    return 1.0 - (6.0 * d_sq) / (n * (n*n - 1))


# ── MAIN ──

def main() -> None:
    if "--analyze" in sys.argv:
        analyze()
        return

    limit = 100
    single_mint = None
    if "--limit" in sys.argv:
        idx = sys.argv.index("--limit")
        if idx + 1 < len(sys.argv):
            limit = int(sys.argv[idx + 1])
    if "--mint" in sys.argv:
        idx = sys.argv.index("--mint")
        if idx + 1 < len(sys.argv):
            single_mint = sys.argv[idx + 1]

    today = _today()
    existing = load_existing_profiles(today)

    # Load calibration tokens
    if single_mint:
        tokens = [{"mint": single_mint, "symbol": "?", "conviction": 0, "conviction_tier": "?"}]
    else:
        with open(CALIB_PATH) as f:
            calib = json.load(f)
        tokens = calib["results"][:limit]

    # Skip already profiled today
    to_profile = [t for t in tokens if t["mint"] not in existing]
    already = len(tokens) - len(to_profile)

    print("=" * 70)
    print(f"SOVEREIGN TOKEN PROFILER — {today}")
    print(f"Tokens: {len(tokens)} total, {already} already profiled today, {len(to_profile)} to fetch")
    print(f"Est. cost: ~{len(to_profile) * 152} credits")
    print("=" * 70)

    if not to_profile:
        print("\nAll tokens already profiled today. Use --analyze to see results.")
        return

    for i, token in enumerate(to_profile):
        mint = token["mint"]
        symbol = token.get("symbol", "?")
        conv = token.get("conviction", 0)
        tier = token.get("conviction_tier", "?")

        print(f"\n[{i+1}/{len(to_profile)}] {symbol:12s} (conv={conv:.3f}, {tier})")

        profile = build_profile(mint, symbol, conv, tier)
        save_profile(profile)

        # Print summary
        f = profile.get("flow", {})
        if f.get("txs_with_flow", 0) > 0:
            print(f"  flow: {f['txs_fetched']} txs ({f['txs_with_flow']} with token flow)  "
                  f"span={f['time_span_hours']:.1f}h")
            print(f"  buyers={f['unique_buyers']}  sellers={f['unique_sellers']}  "
                  f"B/S={f['buy_sell_ratio']:.2f}  vol={f['volume_ratio']:.2f}")
            print(f"  wallets={f['unique_wallets']}  flippers={f.get('flippers',0)}  "
                  f"pools={f['pool_wallets_seen']}")
        else:
            print(f"  flow: no token flow detected")

        hs = profile.get("holder_sample", {})
        print(f"  holders: sampled={hs.get('sampled',0)}  real={hs.get('real_holders',0)}  "
              f"dust={hs.get('dust_holders',0)} ({hs.get('dust_ratio',0):.1%})")
        print(f"  age={profile.get('token_age_days','?')}d  "
              f"price={'$'+str(profile.get('price_usd','N/A')) if profile.get('price_usd') else 'N/A'}  "
              f"credits={profile.get('api_credits',0)}")

        if profile.get("errors"):
            for e in profile["errors"]:
                print(f"  ⚠ {e}")

    print(f"\nTotal API cost: {_credits:,} credits ({_calls} calls)")
    print(f"Profiles saved to: {PROFILES_PATH}")
    print(f"\nRun `python3 {os.path.basename(__file__)} --analyze` for correlation analysis.")


if __name__ == "__main__":
    main()
