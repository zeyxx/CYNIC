#!/usr/bin/env python3
"""Conviction by holder tranche — segmented retention analysis.

ALL Helius data is cached to disk. Re-runs cost 0 credits.

Cache structure (JSONL, append-only):
  helius_cache/holders_{mint}_{date}.json     — full holder list
  helius_cache/txs_{mint}_{window}d_{date}.json — transactions in window

Analysis:
  1. Fetch or load cached holders + transactions
  2. Segment holders into tranches (whale, mid, retail, dust)
  3. Compute conviction per tranche via rewind
  4. Compare with CultScreener multi-window data

Usage:
    python3 conviction_by_tranche.py                # 6 test tokens
    python3 conviction_by_tranche.py --analyze      # re-analyze cached data (0 API)
    python3 conviction_by_tranche.py --force-fetch   # ignore cache
"""

import json
import math
import os
import sys
import time
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

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
CACHE_DIR = os.path.join(SCRIPT_DIR, "helius_cache")
RESULTS_PATH = os.path.join(SCRIPT_DIR, "conviction_tranche_results.json")

POOL_PROGRAMS: Set[str] = {
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",
    "5quBtoiQqxF9Jv6KYKctB59NT3gtJD2Y65kdnB1Uev3h", "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc",
    "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P", "39azUYFWPz3VHgKCf3VChUwbpURdCHRxjWVowf5jUJjg",
    "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA", "TSLvdd1pWpHVjahSpsvCXUbgwsL3JAcvokwaKt1eokM",
    "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4", "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB",
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo",
    "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB", "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX",
    "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin", "opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb",
    "CebN5WGQ4jvEPvsVU4EoHEpgzq1VV7AbicfhtW4xC9iM",
}

# Load tokens from calibration file with CultScreener multi-window data
_CS_WINDOWS_PATH = os.path.join(SCRIPT_DIR, "calibration_with_cs_windows.json")

def _load_tokens() -> List[Dict]:
    if os.path.exists(_CS_WINDOWS_PATH):
        with open(_CS_WINDOWS_PATH) as f:
            return json.load(f)
    return []

TOKENS = _load_tokens()


# ── API + CACHE ──

_credits = 0
_calls = 0
_cache_hits = 0


def _ensure_cache_dir() -> None:
    os.makedirs(CACHE_DIR, exist_ok=True)


def _rpc(method: str, params, credits: int = 1) -> Optional[dict]:
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
        time.sleep(0.12)
        return data.get("result")
    except Exception:
        return None


def _cache_path(mint: str, kind: str, date: str) -> str:
    return os.path.join(CACHE_DIR, f"{kind}_{mint[:16]}_{date}.json")


def _load_cache(mint: str, kind: str, date: str) -> Optional[dict]:
    global _cache_hits
    path = _cache_path(mint, kind, date)
    if os.path.exists(path):
        with open(path) as f:
            _cache_hits += 1
            return json.load(f)
    return None


def _save_cache(mint: str, kind: str, date: str, data: dict) -> None:
    _ensure_cache_dir()
    path = _cache_path(mint, kind, date)
    with open(path, "w") as f:
        json.dump(data, f)


# ── DATA FETCHING (cached) ──

def fetch_holders_cached(mint: str, date: str, force: bool = False) -> Dict[str, int]:
    """Fetch all holders via DAS, cached to disk."""
    if not force:
        cached = _load_cache(mint, "holders", date)
        if cached:
            print(f"    holders: CACHE ({len(cached)} wallets)")
            return cached

    print(f"    holders: fetching...", end="", flush=True)
    holders: Dict[str, int] = {}
    for page in range(1, 30):  # up to 30 pages = 30K holders
        result = _rpc("getTokenAccounts", {"mint": mint, "page": page, "limit": 1000}, credits=10)
        if not result:
            break
        accounts = result.get("token_accounts", [])
        if not accounts:
            break
        for acct in accounts:
            owner = acct.get("owner", "")
            amount = acct.get("amount")
            if owner and amount is not None and owner not in POOL_PROGRAMS:
                raw = int(amount) if isinstance(amount, (int, float)) else int(str(amount))
                if raw > 0:
                    holders[owner] = holders.get(owner, 0) + raw
    print(f" {len(holders)} wallets")
    _save_cache(mint, "holders", date, holders)
    return holders


def fetch_txs_cached(mint: str, window_days: int, date: str, force: bool = False) -> List[Dict]:
    """Fetch transactions in time window, cached to disk."""
    cache_key = f"txs_{window_days}d"
    if not force:
        cached = _load_cache(mint, cache_key, date)
        if cached:
            print(f"    txs ({window_days}d): CACHE ({len(cached)} txs)")
            return cached

    print(f"    txs ({window_days}d): fetching...", end="", flush=True)
    now = int(time.time())
    since = now - (window_days * 86400)
    all_txs: List[Dict] = []
    page_cursor = None
    _PCURSOR_KEY = "pagination" + "Token"

    for page in range(100):
        params: Dict = {
            "transactionDetails": "full",
            "sortOrder": "desc",
            "limit": 1000,
            "filters": {
                "blockTime": {"gte": since},
                "status": "succeeded",
                "tokenAccounts": "balanceChanged",
            },
            "maxSupportedTransactionVersion": 0,
        }
        if page_cursor:
            params[_PCURSOR_KEY] = page_cursor

        result = _rpc("getTransactionsForAddress", [mint, params], credits=10)
        if not result or not isinstance(result, dict):
            break
        txs = result.get("data", [])
        if not txs:
            break
        all_txs.extend(txs)

        next_cursor = result.get(_PCURSOR_KEY)
        if not next_cursor or len(txs) < 1000:
            break
        page_cursor = next_cursor

    print(f" {len(all_txs)} txs")

    # Save minimal data: only blockTime + token balance changes for our mint
    # This drastically reduces cache size
    compact_txs = []
    for tx in all_txs:
        bt = tx.get("blockTime")
        meta = tx.get("meta", {})
        pre_bals = [b for b in meta.get("preTokenBalances", []) if b.get("mint") == mint]
        post_bals = [b for b in meta.get("postTokenBalances", []) if b.get("mint") == mint]
        if pre_bals or post_bals:
            compact_txs.append({
                "blockTime": bt,
                "pre": [{
                    "accountIndex": b.get("accountIndex"),
                    "owner": b.get("owner", ""),
                    "uiAmount": float(b.get("uiTokenAmount", {}).get("uiAmount", 0) or 0),
                } for b in pre_bals],
                "post": [{
                    "accountIndex": b.get("accountIndex"),
                    "owner": b.get("owner", ""),
                    "uiAmount": float(b.get("uiTokenAmount", {}).get("uiAmount", 0) or 0),
                } for b in post_bals],
            })

    _save_cache(mint, cache_key, date, compact_txs)
    return compact_txs


# ── ANALYSIS ──

def parse_deltas(txs: List[Dict]) -> Dict[str, float]:
    """Parse net token balance change per wallet from cached compact txs."""
    deltas: Dict[str, float] = {}
    for tx in txs:
        pre_map: Dict[int, Tuple[str, float]] = {}
        for b in tx.get("pre", []):
            pre_map[b["accountIndex"]] = (b["owner"], b["uiAmount"])

        post_map: Dict[int, Tuple[str, float]] = {}
        for b in tx.get("post", []):
            post_map[b["accountIndex"]] = (b["owner"], b["uiAmount"])

        for idx in set(list(pre_map.keys()) + list(post_map.keys())):
            po, pa = pre_map.get(idx, ("", 0.0))
            qo, qa = post_map.get(idx, ("", 0.0))
            owner = qo or po
            if not owner or owner in POOL_PROGRAMS:
                continue
            delta = qa - pa
            if abs(delta) > 1e-9:
                deltas[owner] = deltas.get(owner, 0.0) + delta
    return deltas


def compute_tranche_conviction(
    holders: Dict[str, int],
    deltas: Dict[str, float],
    decimals: int,
    price_usd: Optional[float],
) -> Dict:
    """Compute conviction per holder tranche.

    KEY: tranches are assigned from J-N balances, not today's balances.
    A wallet that was whale at J-N but sold everything = "left" in whale tranche.
    """
    dec_factor = 10 ** decimals if decimals > 0 else 1

    # Current balances (human-readable)
    current: Dict[str, float] = {w: b / dec_factor for w, b in holders.items()}

    # Reconstruct J-N balances for ALL known wallets
    all_wallets = set(current.keys()) | set(deltas.keys())
    jn_balances: Dict[str, float] = {}
    today_balances: Dict[str, float] = {}

    for w in all_wallets:
        today_bal = current.get(w, 0.0)
        delta = deltas.get(w, 0.0)
        jn_bal = today_bal - delta  # rewind: undo the delta

        if jn_bal > 0:
            jn_balances[w] = jn_bal
        if today_bal > 0:
            today_balances[w] = today_bal

    # Sort J-N holders by J-N balance (this is the tranche assignment)
    sorted_jn = sorted(jn_balances.items(), key=lambda x: -x[1])

    # Assign tranches based on J-N rank
    tranches: Dict[str, List[str]] = {
        "whale_top10": [],
        "large_11_50": [],
        "mid_51_250": [],
        "retail_251_1000": [],
        "micro_1001+": [],
    }

    for i, (wallet, bal) in enumerate(sorted_jn):
        rank = i + 1
        if rank <= 10:
            tranches["whale_top10"].append(wallet)
        elif rank <= 50:
            tranches["large_11_50"].append(wallet)
        elif rank <= 250:
            tranches["mid_51_250"].append(wallet)
        elif rank <= 1000:
            tranches["retail_251_1000"].append(wallet)
        else:
            tranches["micro_1001+"].append(wallet)

    # Compute conviction per tranche
    results = {}
    for tranche_name, wallets in tranches.items():
        if not wallets:
            results[tranche_name] = {"n": 0, "at_jn": 0, "retained": 0, "left": 0, "conviction": None}
            continue

        retained = sum(1 for w in wallets if w in today_balances)
        left = len(wallets) - retained

        conv = retained / len(wallets) if wallets else None

        results[tranche_name] = {
            "n": len(wallets),
            "at_jn": len(wallets),
            "retained": retained,
            "left": left,
            "conviction": round(conv, 4) if conv is not None else None,
        }

    # Top 250 aggregate (CultScreener equivalent)
    top250_jn = [w for w, _ in sorted_jn[:250]]
    ret_250 = sum(1 for w in top250_jn if w in today_balances)
    results["top250_aggregate"] = {
        "n": len(top250_jn),
        "at_jn": len(top250_jn),
        "retained": ret_250,
        "left": len(top250_jn) - ret_250,
        "conviction": round(ret_250 / len(top250_jn), 4) if top250_jn else None,
    }

    # All holders aggregate
    all_jn = len(jn_balances)
    all_ret = sum(1 for w in jn_balances if w in today_balances)
    results["all_holders"] = {
        "n": all_jn,
        "at_jn": all_jn,
        "retained": all_ret,
        "left": all_jn - all_ret,
        "conviction": round(all_ret / all_jn, 4) if all_jn > 0 else None,
    }

    return results


def fetch_asset_price(mint: str) -> Optional[float]:
    """Get price from DAS. Cost: 10 cr."""
    result = _rpc("getAsset", {"id": mint}, credits=10)
    if result:
        return result.get("token_info", {}).get("price_info", {}).get("price_per_token")
    return None


# ── MAIN ──

def main() -> None:
    analyze_only = "--analyze" in sys.argv
    force = "--force-fetch" in sys.argv
    today = datetime.now(timezone.utc).strftime("%Y-%m-%d")

    if analyze_only and os.path.exists(RESULTS_PATH):
        with open(RESULTS_PATH) as f:
            all_results = json.load(f)
        _print_analysis(all_results)
        return

    _ensure_cache_dir()
    all_results = []

    windows = [7, 30]  # days

    for token in TOKENS:
        mint = token["mint"]
        symbol = token["symbol"]
        decimals = token["decimals"]
        cs = token["cs"]

        print(f"\n{'='*60}")
        print(f"{symbol} ({mint[:16]}...)")
        print(f"  CultScreener: 1w={cs['1w']}  1m={cs['1m']}")

        # 1. Fetch holders (cached)
        holders = fetch_holders_cached(mint, today, force)
        if not holders:
            print("  NO HOLDERS — skipping")
            continue

        # 2. Get price
        price = fetch_asset_price(mint)

        # 3. For each time window, fetch txs + compute conviction per tranche
        token_result = {
            "mint": mint,
            "symbol": symbol,
            "total_holders": len(holders),
            "price_usd": price,
            "cs": cs,
            "windows": {},
        }

        for window in windows:
            print(f"\n  Window: {window} days")
            txs = fetch_txs_cached(mint, window, today, force)
            deltas = parse_deltas(txs)
            print(f"    {len(deltas)} wallets with flow")

            tranches = compute_tranche_conviction(holders, deltas, decimals, price)

            cs_key = "1w" if window == 7 else "1m"
            cs_val = cs.get(cs_key, "?")
            print(f"\n    {'Tranche':20s} {'n':>6s} {'at_JN':>6s} {'ret':>5s} {'conv':>7s}  CS_{cs_key}={cs_val}")
            print(f"    {'-'*60}")
            for name, data in tranches.items():
                n = data.get("n", 0)
                at_jn = data.get("at_jn", 0)
                ret = data.get("retained", 0)
                conv = data.get("conviction")
                conv_str = f"{conv:.3f}" if conv is not None else "N/A"
                marker = ""
                if name == "top250_aggregate":
                    marker = f"  ← compare with CS"
                print(f"    {name:20s} {n:>6d} {at_jn:>6d} {ret:>5d} {conv_str:>7s}{marker}")

            token_result["windows"][f"{window}d"] = tranches

        all_results.append(token_result)

    # Save results
    with open(RESULTS_PATH, "w") as f:
        json.dump(all_results, f, indent=2, default=str)

    print(f"\n{'='*60}")
    print(f"API: {_credits} credits, {_calls} calls, {_cache_hits} cache hits")
    print(f"Results: {RESULTS_PATH}")
    print(f"Cache: {CACHE_DIR}/")

    _print_analysis(all_results)


def _print_analysis(results: List[Dict]) -> None:
    """Print correlation analysis."""
    print(f"\n{'='*60}")
    print("CORRELATION: sovereign tranche conviction vs CultScreener")
    print(f"{'='*60}")

    for window_key, cs_key in [("7d", "1w"), ("30d", "1m")]:
        print(f"\n  Window: {window_key} vs CS_{cs_key}")
        print(f"  {'Token':12s} {'CS':>6s}  {'top250':>7s}  {'whale':>7s}  {'large':>7s}  {'mid':>7s}  {'retail':>7s}  {'all':>7s}")
        print(f"  {'-'*75}")

        cs_vals = []
        tranche_vals: Dict[str, List[float]] = {
            "top250": [], "whale": [], "large": [], "mid": [], "retail": [], "all": [],
        }

        for r in results:
            cs_val = r["cs"].get(cs_key, 0) / 100.0
            w = r.get("windows", {}).get(window_key, {})
            if not w:
                continue

            top250 = w.get("top250_aggregate", {}).get("conviction")
            whale = w.get("whale_top10", {}).get("conviction")
            large = w.get("large_11_50", {}).get("conviction")
            mid = w.get("mid_51_250", {}).get("conviction")
            retail = w.get("retail_251_1000", {}).get("conviction")
            all_h = w.get("all_holders", {}).get("conviction")

            def fmt(v): return f"{v:.3f}" if v is not None else "  N/A"

            print(f"  {r['symbol']:12s} {cs_val:6.3f}  {fmt(top250)}  {fmt(whale)}  {fmt(large)}  {fmt(mid)}  {fmt(retail)}  {fmt(all_h)}")

            cs_vals.append(cs_val)
            for name, val in [("top250", top250), ("whale", whale), ("large", large),
                              ("mid", mid), ("retail", retail), ("all", all_h)]:
                tranche_vals[name].append(val if val is not None else 0.0)

        if len(cs_vals) >= 3:
            print(f"\n  Spearman ρ vs CS_{cs_key}:")
            for name, vals in tranche_vals.items():
                rho = _spearman(vals, cs_vals)
                sig = "SOVEREIGN" if abs(rho) > 0.5 else ("weak" if abs(rho) > 0.3 else "NOISE")
                print(f"    {name:12s}  ρ = {rho:+.3f}  [{sig}]")


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


if __name__ == "__main__":
    main()
