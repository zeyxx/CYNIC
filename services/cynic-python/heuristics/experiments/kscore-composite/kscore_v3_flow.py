#!/usr/bin/env python3
"""Sovereign K-Score v3: Buy/sell flow analysis from token transactions.

v1: top-5 holder behavior → survivorship bias (DH=1.0 everywhere)
v2: random holder sampling → measures wealth, not conviction (ρ inverted)
v3: parse RECENT TRANSACTIONS of the token itself → who is buying vs selling?

Hypothesis: buy/sell ratio from recent token transactions correlates with conviction.
  - Strong conviction → more buyers than sellers (accumulation)
  - Weak conviction → more sellers than buyers (distribution)

Falsification: If ρ(buy_sell_ratio, conviction) < 0.3 → on-chain flow is noise.

Method: getTransactionsForAddress(MINT, full, desc, limit=100) → parse pre/postTokenBalances
  - Each tx that changes a non-pool wallet's token balance = buy or sell
  - buyer: wallet's token balance INCREASED
  - seller: wallet's token balance DECREASED

Cost: ~40 cr/token (getAsset 10 + age 10 + flow 10 + buffer).
Total 33 tokens: ~1,320 cr.

Usage:
    python3 kscore_v3_flow.py
    python3 kscore_v3_flow.py --limit 3
    python3 kscore_v3_flow.py --verbose   # show per-tx parsing
"""

import json
import os
import sys
import time
from dataclasses import dataclass, field, asdict
from typing import Optional, List, Dict, Set

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
HELIUS_REST = f"https://api-mainnet.helius-rpc.com/v0"

POOL_PROGRAMS: Set[str] = {
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
    "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",
    "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc",
    "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP",
    "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P",
    "39azUYFWPz3VHgKCf3VChUwbpURdCHRxjWVowf5jUJjg",
    "TSLvdd1pWpHVjahSpsvCXUbgwsL3JAcvokwaKt1eokM",
    "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4",
    "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB",
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1",
    "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo",
    "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB",
}


# ── DATA ──

@dataclass
class FlowAnalysis:
    mint: str
    symbol: str
    conviction: float
    conviction_tier: str

    # Transaction flow
    txs_fetched: int           # total txs returned
    txs_with_token_flow: int   # txs where our token changed hands
    time_span_hours: float     # oldest to newest tx in sample

    # Buy/sell direction
    unique_buyers: int         # wallets whose token balance increased
    unique_sellers: int        # wallets whose token balance decreased
    buy_sell_ratio: float      # buyers / sellers (>1 = accumulation)
    total_bought: float        # total tokens flowing to human wallets
    total_sold: float          # total tokens flowing from human wallets
    volume_buy_sell: float     # total_bought / total_sold

    # Activity metrics
    tx_per_hour: float         # txs / time_span
    unique_wallets: int        # distinct non-pool wallets involved
    avg_tx_size: float         # average token amount per transfer

    # Whale vs retail
    whale_buys: int            # buy txs > 1% of supply
    retail_buys: int           # buy txs < 0.01% of supply
    whale_sells: int
    retail_sells: int

    # Age
    token_age_days: Optional[float]
    longevity: float

    # Price / supply
    price_usd: Optional[float]
    supply_human: float
    decimals: int

    # Method used
    method: str                # "rpc_full" or "enhanced_rest"
    api_credits: int
    errors: List[str] = field(default_factory=list)


# ── API ──

_credits = 0
_calls = 0
VERBOSE = "--verbose" in sys.argv


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
            if VERBOSE:
                print(f"    RPC error: {data['error']}")
            return None
        time.sleep(0.12)
        return data.get("result")
    except Exception as e:
        if VERBOSE:
            print(f"    RPC exception: {e}")
        return None


def _rest_get(path: str, params: Optional[dict] = None) -> Optional[list]:
    """Enhanced Transactions REST API. Cost: 100 cr per call."""
    global _credits, _calls
    try:
        url = f"{HELIUS_REST}/{path}?api-key={HELIUS_API_KEY}"
        resp = requests.get(url, params=params or {}, timeout=15)
        _calls += 1
        _credits += 100
        if resp.status_code != 200:
            return None
        time.sleep(0.12)
        return resp.json()
    except Exception:
        return None


# ── FLOW PARSING ──

def parse_flow_from_rpc(mint: str, supply: int, decimals: int) -> Optional[Dict]:
    """Parse buy/sell flow from getTransactionsForAddress (full details).

    Returns dict with flow metrics, or None if endpoint doesn't return token data.
    """
    result = _rpc("getTransactionsForAddress", [
        mint,
        {
            "sortOrder": "desc",
            "limit": 100,
            "transactionDetails": "full",
            "encoding": "jsonParsed",
            "tokenAccounts": "balanceChanged",
            "maxSupportedTransactionVersion": 0,
        }
    ], credits=10)

    if not result or not isinstance(result, dict):
        return None

    txs = result.get("data", [])
    if not txs:
        return None

    return _extract_flow(txs, mint, supply, decimals, "rpc_full")


def parse_flow_from_rest(mint: str, supply: int, decimals: int) -> Optional[Dict]:
    """Fallback: parse buy/sell flow from Enhanced Transactions REST API.

    This API returns parsed tokenTransfers with fromUserAccount/toUserAccount.
    Cost: 100 cr per call.
    """
    txs = _rest_get(f"addresses/{mint}/transactions", {"limit": 100})
    if not txs or not isinstance(txs, list):
        return None

    return _extract_flow_from_enhanced(txs, mint, supply, decimals)


def _extract_flow(txs: List[Dict], mint: str, supply: int, decimals: int, method: str) -> Dict:
    """Extract buy/sell flow from full transaction data (RPC response)."""
    dec_factor = 10 ** decimals if decimals > 0 else 1

    buyers: Dict[str, float] = {}   # wallet → total tokens received
    sellers: Dict[str, float] = {}  # wallet → total tokens sent
    all_wallets: Set[str] = set()
    txs_with_flow = 0
    total_bought = 0.0
    total_sold = 0.0
    whale_buys = 0
    retail_buys = 0
    whale_sells = 0
    retail_sells = 0

    supply_human = supply / dec_factor if dec_factor > 0 else 1.0
    whale_threshold = supply_human * 0.01    # 1% of supply
    retail_threshold = supply_human * 0.0001  # 0.01% of supply

    timestamps: List[int] = []

    for tx_wrapper in txs:
        bt = tx_wrapper.get("blockTime", 0)
        if bt:
            timestamps.append(bt)

        meta = tx_wrapper.get("meta", {})
        if not meta:
            continue

        pre_bals = meta.get("preTokenBalances", [])
        post_bals = meta.get("postTokenBalances", [])

        # Build map: accountIndex → (mint, owner, pre_amount, post_amount)
        pre_map: Dict[int, Dict] = {}
        for b in pre_bals:
            if b.get("mint") == mint:
                idx = b.get("accountIndex", -1)
                owner = b.get("owner", "")
                ui = b.get("uiTokenAmount", {})
                amount = float(ui.get("uiAmount", 0) or 0)
                pre_map[idx] = {"owner": owner, "amount": amount}

        post_map: Dict[int, Dict] = {}
        for b in post_bals:
            if b.get("mint") == mint:
                idx = b.get("accountIndex", -1)
                owner = b.get("owner", "")
                ui = b.get("uiTokenAmount", {})
                amount = float(ui.get("uiAmount", 0) or 0)
                post_map[idx] = {"owner": owner, "amount": amount}

        # Compare pre vs post for each account index
        all_indices = set(list(pre_map.keys()) + list(post_map.keys()))
        tx_had_flow = False

        for idx in all_indices:
            pre = pre_map.get(idx, {})
            post = post_map.get(idx, {})

            owner = post.get("owner") or pre.get("owner", "")
            if not owner or owner in POOL_PROGRAMS:
                continue

            pre_amount = pre.get("amount", 0.0)
            post_amount = post.get("amount", 0.0)
            delta = post_amount - pre_amount

            if abs(delta) < 0.000001:
                continue

            tx_had_flow = True
            all_wallets.add(owner)

            if delta > 0:
                # BUYER: token balance increased
                buyers[owner] = buyers.get(owner, 0) + delta
                total_bought += delta
                if delta > whale_threshold:
                    whale_buys += 1
                elif delta < retail_threshold:
                    retail_buys += 1
            else:
                # SELLER: token balance decreased
                sellers[owner] = sellers.get(owner, 0) + abs(delta)
                total_sold += abs(delta)
                if abs(delta) > whale_threshold:
                    whale_sells += 1
                elif abs(delta) < retail_threshold:
                    retail_sells += 1

        if tx_had_flow:
            txs_with_flow += 1

    # Time span
    if len(timestamps) >= 2:
        time_span_hours = (max(timestamps) - min(timestamps)) / 3600.0
    else:
        time_span_hours = 1.0

    tx_per_hour = len(txs) / max(time_span_hours, 0.01)

    # Ratios
    n_buyers = len(buyers)
    n_sellers = len(sellers)
    buy_sell_ratio = n_buyers / n_sellers if n_sellers > 0 else (10.0 if n_buyers > 0 else 1.0)
    volume_ratio = total_bought / total_sold if total_sold > 0 else (10.0 if total_bought > 0 else 1.0)

    total_transfers = sum(buyers.values()) + sum(sellers.values())
    avg_size = total_transfers / max(n_buyers + n_sellers, 1)

    if VERBOSE:
        print(f"    RPC flow: {txs_with_flow}/{len(txs)} txs with token flow")
        print(f"    buyers={n_buyers} sellers={n_sellers} ratio={buy_sell_ratio:.2f}")
        print(f"    bought={total_bought:.2f} sold={total_sold:.2f} vol_ratio={volume_ratio:.2f}")

    return {
        "txs_fetched": len(txs),
        "txs_with_token_flow": txs_with_flow,
        "time_span_hours": time_span_hours,
        "unique_buyers": n_buyers,
        "unique_sellers": n_sellers,
        "buy_sell_ratio": buy_sell_ratio,
        "total_bought": total_bought,
        "total_sold": total_sold,
        "volume_buy_sell": volume_ratio,
        "tx_per_hour": tx_per_hour,
        "unique_wallets": len(all_wallets),
        "avg_tx_size": avg_size,
        "whale_buys": whale_buys,
        "retail_buys": retail_buys,
        "whale_sells": whale_sells,
        "retail_sells": retail_sells,
        "method": method,
    }


def _extract_flow_from_enhanced(txs: List[Dict], mint: str, supply: int, decimals: int) -> Dict:
    """Extract buy/sell flow from Enhanced Transactions REST (parsed format)."""
    dec_factor = 10 ** decimals if decimals > 0 else 1
    supply_human = supply / dec_factor if dec_factor > 0 else 1.0
    whale_threshold = supply_human * 0.01
    retail_threshold = supply_human * 0.0001

    buyers: Dict[str, float] = {}
    sellers: Dict[str, float] = {}
    all_wallets: Set[str] = set()
    txs_with_flow = 0
    total_bought = 0.0
    total_sold = 0.0
    whale_buys = 0
    retail_buys = 0
    whale_sells = 0
    retail_sells = 0
    timestamps: List[int] = []

    for tx in txs:
        ts = tx.get("timestamp", 0)
        if ts:
            timestamps.append(ts)

        token_transfers = tx.get("tokenTransfers", [])
        tx_had_flow = False

        for xfer in token_transfers:
            if xfer.get("mint") != mint:
                continue

            from_acct = xfer.get("fromUserAccount", "")
            to_acct = xfer.get("toUserAccount", "")
            amount = float(xfer.get("tokenAmount", 0))

            if amount <= 0:
                continue

            tx_had_flow = True

            # Buyer side (receiver, if not a pool)
            if to_acct and to_acct not in POOL_PROGRAMS:
                buyers[to_acct] = buyers.get(to_acct, 0) + amount
                total_bought += amount
                all_wallets.add(to_acct)
                if amount > whale_threshold:
                    whale_buys += 1
                elif amount < retail_threshold:
                    retail_buys += 1

            # Seller side (sender, if not a pool)
            if from_acct and from_acct not in POOL_PROGRAMS:
                sellers[from_acct] = sellers.get(from_acct, 0) + amount
                total_sold += amount
                all_wallets.add(from_acct)
                if amount > whale_threshold:
                    whale_sells += 1
                elif amount < retail_threshold:
                    retail_sells += 1

        if tx_had_flow:
            txs_with_flow += 1

    if len(timestamps) >= 2:
        time_span_hours = (max(timestamps) - min(timestamps)) / 3600.0
    else:
        time_span_hours = 1.0

    n_buyers = len(buyers)
    n_sellers = len(sellers)
    buy_sell_ratio = n_buyers / n_sellers if n_sellers > 0 else (10.0 if n_buyers > 0 else 1.0)
    volume_ratio = total_bought / total_sold if total_sold > 0 else (10.0 if total_bought > 0 else 1.0)
    total_transfers = sum(buyers.values()) + sum(sellers.values())
    avg_size = total_transfers / max(n_buyers + n_sellers, 1)

    if VERBOSE:
        print(f"    REST flow: {txs_with_flow}/{len(txs)} txs with token flow")
        print(f"    buyers={n_buyers} sellers={n_sellers} ratio={buy_sell_ratio:.2f}")

    return {
        "txs_fetched": len(txs),
        "txs_with_token_flow": txs_with_flow,
        "time_span_hours": time_span_hours,
        "unique_buyers": n_buyers,
        "unique_sellers": n_sellers,
        "buy_sell_ratio": buy_sell_ratio,
        "total_bought": total_bought,
        "total_sold": total_sold,
        "volume_buy_sell": volume_ratio,
        "tx_per_hour": len(txs) / max(time_span_hours, 0.01),
        "unique_wallets": len(all_wallets),
        "avg_tx_size": avg_size,
        "whale_buys": whale_buys,
        "retail_buys": retail_buys,
        "whale_sells": whale_sells,
        "retail_sells": retail_sells,
        "method": "enhanced_rest",
    }


# ── TOKEN INFO ──

def get_asset_info(mint: str) -> Dict:
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


def get_token_age(mint: str) -> Optional[float]:
    result = _rpc("getTransactionsForAddress", [
        mint, {"sortOrder": "asc", "limit": 1, "transactionDetails": "signatures"}
    ], credits=10)
    if result and isinstance(result, dict):
        data = result.get("data", [])
        if data:
            bt = data[0].get("blockTime")
            if bt:
                return (time.time() - bt) / 86400
    return None


# ── ANALYSIS ──

def analyze_token(mint: str, symbol: str, conviction: float, tier: str) -> FlowAnalysis:
    errors: List[str] = []
    credits_start = _credits

    # Asset info
    asset = get_asset_info(mint)
    supply = asset.get("supply", 0)
    supply_human = asset.get("supply_human", 0)
    decimals = asset.get("decimals", 0)
    price_usd = asset.get("price_usd")

    if supply == 0:
        errors.append("no supply")
        return _empty(mint, symbol, conviction, tier, errors, _credits - credits_start)

    # Age
    age_days = get_token_age(mint)
    longevity = min((age_days or 30) / 365.0, 1.0)

    # Flow: try RPC first (10 cr), fallback to REST (100 cr)
    flow = parse_flow_from_rpc(mint, supply, decimals)
    if not flow or flow["txs_with_token_flow"] == 0:
        if VERBOSE:
            print(f"    RPC returned {flow['txs_fetched'] if flow else 0} txs but 0 with token flow — trying REST")
        flow = parse_flow_from_rest(mint, supply, decimals)

    if not flow:
        errors.append("no flow data from RPC or REST")
        return _empty(mint, symbol, conviction, tier, errors, _credits - credits_start)

    return FlowAnalysis(
        mint=mint, symbol=symbol, conviction=conviction, conviction_tier=tier,
        txs_fetched=flow["txs_fetched"],
        txs_with_token_flow=flow["txs_with_token_flow"],
        time_span_hours=flow["time_span_hours"],
        unique_buyers=flow["unique_buyers"],
        unique_sellers=flow["unique_sellers"],
        buy_sell_ratio=flow["buy_sell_ratio"],
        total_bought=flow["total_bought"],
        total_sold=flow["total_sold"],
        volume_buy_sell=flow["volume_buy_sell"],
        tx_per_hour=flow["tx_per_hour"],
        unique_wallets=flow["unique_wallets"],
        avg_tx_size=flow["avg_tx_size"],
        whale_buys=flow["whale_buys"],
        retail_buys=flow["retail_buys"],
        whale_sells=flow["whale_sells"],
        retail_sells=flow["retail_sells"],
        token_age_days=age_days,
        longevity=longevity,
        price_usd=price_usd,
        supply_human=supply_human,
        decimals=decimals,
        method=flow["method"],
        api_credits=_credits - credits_start,
        errors=errors,
    )


def _empty(mint, sym, conv, tier, errors, credits):
    return FlowAnalysis(
        mint=mint, symbol=sym, conviction=conv, conviction_tier=tier,
        txs_fetched=0, txs_with_token_flow=0, time_span_hours=0,
        unique_buyers=0, unique_sellers=0, buy_sell_ratio=1.0,
        total_bought=0, total_sold=0, volume_buy_sell=1.0,
        tx_per_hour=0, unique_wallets=0, avg_tx_size=0,
        whale_buys=0, retail_buys=0, whale_sells=0, retail_sells=0,
        token_age_days=None, longevity=0, price_usd=None,
        supply_human=0, decimals=0, method="none",
        api_credits=credits, errors=errors,
    )


# ── CORRELATION ──

def spearman(x: List[float], y: List[float]) -> float:
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

def main():
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

    print("=" * 70)
    print("SOVEREIGN K-SCORE v3 — Buy/Sell Flow Analysis")
    print(f"Tokens: {len(tokens)}  |  Est: ~{len(tokens)*30} cr (RPC) or ~{len(tokens)*120} cr (REST fallback)")
    print("=" * 70)

    if dry_run:
        for t in tokens:
            print(f"  {t['symbol']:12s}  conv={t['conviction']:.3f}  [{t['conviction_tier']}]")
        return

    results: List[FlowAnalysis] = []

    for i, token in enumerate(tokens):
        print(f"\n[{i+1}/{len(tokens)}] {token['symbol']:12s} (conv={token['conviction']:.3f}, {token['conviction_tier']})")
        r = analyze_token(token["mint"], token["symbol"], token["conviction"], token["conviction_tier"])
        results.append(r)

        print(f"  method={r.method}  txs={r.txs_fetched} (flow={r.txs_with_token_flow})  "
              f"span={r.time_span_hours:.1f}h")
        print(f"  buyers={r.unique_buyers}  sellers={r.unique_sellers}  "
              f"B/S={r.buy_sell_ratio:.2f}  vol_B/S={r.volume_buy_sell:.2f}")
        print(f"  tx/h={r.tx_per_hour:.1f}  wallets={r.unique_wallets}  "
              f"avg_size={r.avg_tx_size:.0f}")
        print(f"  whale: buy={r.whale_buys} sell={r.whale_sells}  "
              f"retail: buy={r.retail_buys} sell={r.retail_sells}")
        if r.errors:
            for e in r.errors:
                print(f"  ⚠ {e}")

    # ── CORRELATIONS ──
    valid = [r for r in results if r.txs_with_token_flow > 0]

    print("\n" + "=" * 70)
    print(f"CORRELATIONS ({len(valid)}/{len(results)} with flow data)")
    print("=" * 70)

    if len(valid) < 5:
        print("Not enough data.")
        _save(results)
        return

    conv = [r.conviction for r in valid]

    signals = [
        ("buy_sell_ratio", [r.buy_sell_ratio for r in valid]),
        ("volume_buy_sell", [r.volume_buy_sell for r in valid]),
        ("unique_buyers", [float(r.unique_buyers) for r in valid]),
        ("unique_sellers", [float(r.unique_sellers) for r in valid]),
        ("unique_wallets", [float(r.unique_wallets) for r in valid]),
        ("tx_per_hour", [r.tx_per_hour for r in valid]),
        ("longevity", [r.longevity for r in valid]),
        ("whale_buy_ratio", [r.whale_buys / max(r.whale_buys + r.whale_sells, 1)
                             for r in valid]),
        ("retail_buy_ratio", [r.retail_buys / max(r.retail_buys + r.retail_sells, 1)
                              for r in valid]),
    ]

    print(f"\n  ρ > 0.5 = SOVEREIGN  |  ρ < 0.3 = NOISE\n")
    for name, vals in signals:
        rho = spearman(vals, conv)
        sig = "SOVEREIGN" if abs(rho) > 0.5 else ("weak" if abs(rho) > 0.3 else "NOISE")
        print(f"  {name:20s}  ρ = {rho:+.3f}  [{sig}]")

    # Per-tier
    print("\nPER-TIER:")
    for tier in ["strong", "mixed", "weak"]:
        td = [r for r in valid if r.conviction_tier == tier]
        if not td:
            continue
        bs = [r.buy_sell_ratio for r in td]
        vbs = [r.volume_buy_sell for r in td]
        tph = [r.tx_per_hour for r in td]
        print(f"\n  {tier.upper()} (n={len(td)}):")
        print(f"    buy_sell_ratio:  avg={sum(bs)/len(bs):.2f}  [{min(bs):.2f} - {max(bs):.2f}]")
        print(f"    volume_B/S:     avg={sum(vbs)/len(vbs):.2f}  [{min(vbs):.2f} - {max(vbs):.2f}]")
        print(f"    tx/hour:        avg={sum(tph)/len(tph):.1f}  [{min(tph):.1f} - {max(tph):.1f}]")

    # Scatter
    print("\nSCATTER:")
    for r in sorted(valid, key=lambda r: -r.conviction):
        tm = {"strong": "S", "mixed": "M", "weak": "W"}[r.conviction_tier]
        # Visual: bars for buy vs sell
        b = min(int(r.buy_sell_ratio * 5), 20)
        bar = "▓" * b if r.buy_sell_ratio >= 1 else "░" * min(int((1/r.buy_sell_ratio)*5), 20)
        print(f"  {r.symbol:12s} conv={r.conviction:.3f} [{tm}] "
              f"B/S={r.buy_sell_ratio:5.2f} {bar}  "
              f"vol={r.volume_buy_sell:5.2f} "
              f"w={r.unique_wallets:3d} tx/h={r.tx_per_hour:6.1f}")

    print(f"\nAPI COST: {_credits:,} credits ({_calls} calls)")
    _save(results)


def _save(results):
    out = os.path.join(os.path.dirname(__file__), "kscore_v3_flow_results.json")
    with open(out, "w") as f:
        json.dump({
            "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S"),
            "total": len(results),
            "valid": sum(1 for r in results if r.txs_with_token_flow > 0),
            "credits": _credits,
            "calls": _calls,
            "results": [asdict(r) for r in results],
        }, f, indent=2)
    print(f"Saved: {out}")


if __name__ == "__main__":
    main()
