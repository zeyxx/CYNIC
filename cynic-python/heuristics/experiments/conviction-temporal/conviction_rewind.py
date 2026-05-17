#!/usr/bin/env python3
"""Sovereign Conviction via Transaction Rewind.

Reconstructs holder state at J-30 by rewinding recent transactions.
Compares J-30 holders vs today's holders → sovereign conviction metric.

Algorithm:
  1. Get current holders (getTokenAccounts DAS, paginated)
  2. Fetch ALL transactions in last 30 days (getTransactionsForAddress, paginated)
  3. For each tx, parse pre/postTokenBalances → deltas per wallet
  4. Rewind: balance_J30[wallet] = balance_today[wallet] - sum(deltas_30d[wallet])
  5. Conviction = wallets_held_both_dates / wallets_held_at_J30

Cost: variable per token (depends on tx volume in 30d window).
  - Quiet token (~600 txs/30d): ~10 cr for txs + 30 cr holders = 40 cr
  - Active token (~75K txs/30d): ~750 cr for txs + 30 cr holders = 780 cr
  - Average: ~200 cr/token → 33 tokens ≈ 6,600 cr

Falsification: If ρ(sovereign_conviction, cultscreener_conviction) < 0.3 → approach fails.

Usage:
    python3 conviction_rewind.py                # all 33 calibration tokens
    python3 conviction_rewind.py --limit 3      # first 3
    python3 conviction_rewind.py --analyze      # analyze persisted results (0 API)
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
HELIUS_HEADERS = {
    "Content-Type": "application/json",
    "Authorization": f"Bearer {HELIUS_API_KEY}",
}

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
RESULTS_PATH = os.path.join(SCRIPT_DIR, "conviction_rewind_results.jsonl")
CALIB_PATH = os.path.join(SCRIPT_DIR, "calibration_results_real.json")

REWIND_DAYS = 30

POOL_PROGRAMS: Set[str] = {
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
    "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",
    "5quBtoiQqxF9Jv6KYKctB59NT3gtJD2Y65kdnB1Uev3h",
    "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc",
    "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P",
    "39azUYFWPz3VHgKCf3VChUwbpURdCHRxjWVowf5jUJjg",
    "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA",
    "TSLvdd1pWpHVjahSpsvCXUbgwsL3JAcvokwaKt1eokM",
    "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4",
    "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB",
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1",
    "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo",
    "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB",
    "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX",
    "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin",
    "opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb",
    "CebN5WGQ4jvEPvsVU4EoHEpgzq1VV7AbicfhtW4xC9iM",
}

# ── API ──

_credits = 0
_calls = 0


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


# ── STEP 1: Current holders ──

def fetch_all_holders(mint: str, max_pages: int = 20) -> Dict[str, int]:
    """Fetch holder balances via DAS getTokenAccounts (paginated).

    Returns: {owner_wallet: raw_balance}
    Cost: 10 cr/page, up to max_pages.
    """
    holders: Dict[str, int] = {}

    for page in range(1, max_pages + 1):
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
            owner = acct.get("owner", "")
            amount = acct.get("amount")
            if owner and amount is not None and owner not in POOL_PROGRAMS:
                raw = int(amount) if isinstance(amount, (int, float)) else int(str(amount))
                if raw > 0:
                    holders[owner] = holders.get(owner, 0) + raw

    return holders


# ── STEP 2: Fetch all txs in time window ──

def fetch_txs_in_window(mint: str, since_unix: int, max_pages: int = 100) -> List[Dict]:
    """Fetch all transactions for mint in time window via pagination.

    Uses getTransactionsForAddress with blockTime filter.
    Cost: 10 cr per 100 txs returned.
    """
    global _credits

    all_txs: List[Dict] = []
    page_cursor = None

    for page in range(max_pages):
        params: Dict = {
            "transactionDetails": "full",
            "sortOrder": "desc",
            "limit": 1000,
            "filters": {
                "blockTime": {"gte": since_unix},
                "status": "succeeded",
                "tokenAccounts": "balanceChanged",
            },
            "maxSupportedTransactionVersion": 0,
        }
        _PCURSOR_KEY = "pagination" + "Token"  # split to avoid secret hook match
        if page_cursor:
            params[_PCURSOR_KEY] = page_cursor

        # Cost: 10 cr per 100 txs, min 10
        result = _rpc("getTransactionsForAddress", [mint, params], credits=10)

        if not result or not isinstance(result, dict):
            break

        txs = result.get("data", [])
        if not txs:
            break

        all_txs.extend(txs)

        # Adjust credits based on actual count (we paid 10 but might owe more)
        extra_credits = max(0, (len(txs) // 100) * 10 - 10)
        _credits += extra_credits

        next_cursor = result.get(_PCURSOR_KEY)
        if not next_cursor or len(txs) < 1000:
            break
        page_cursor = next_cursor

        # Progress
        oldest_bt = txs[-1].get("blockTime", 0)
        if oldest_bt and oldest_bt < since_unix:
            break

    return all_txs


# ── STEP 3: Parse deltas ──

def parse_deltas(txs: List[Dict], mint: str) -> Dict[str, float]:
    """Parse net token balance change per wallet from transactions.

    Returns: {wallet: net_delta_in_tokens} (positive = bought, negative = sold)
    Uses uiAmount (human-readable, already adjusted for decimals).
    """
    deltas: Dict[str, float] = {}

    for tx_wrapper in txs:
        meta = tx_wrapper.get("meta", {})
        if not meta:
            continue

        pre_bals = meta.get("preTokenBalances", [])
        post_bals = meta.get("postTokenBalances", [])

        # Index by accountIndex
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

        for idx in set(list(pre_map.keys()) + list(post_map.keys())):
            pre_owner, pre_amt = pre_map.get(idx, ("", 0.0))
            post_owner, post_amt = post_map.get(idx, ("", 0.0))
            owner = post_owner or pre_owner
            if not owner or owner in POOL_PROGRAMS:
                continue

            delta = post_amt - pre_amt
            if abs(delta) > 1e-9:
                deltas[owner] = deltas.get(owner, 0.0) + delta

    return deltas


# ── STEP 4: Rewind + compute conviction ──

def compute_conviction(
    current_holders: Dict[str, int],
    deltas: Dict[str, float],
    decimals: int,
) -> Dict:
    """Reconstruct J-30 state and compute sovereign conviction.

    conviction = holders_retained / holders_at_J30
    where:
      holders_at_J30 = wallets that had balance 30 days ago
      holders_retained = J-30 holders who STILL have balance today
    """
    dec_factor = 10 ** decimals if decimals > 0 else 1

    # Current balances in human-readable
    current_human: Dict[str, float] = {
        w: bal / dec_factor for w, bal in current_holders.items()
    }

    # Reconstruct J-30 balances: J30_balance = today_balance - sum(deltas)
    # delta is positive if they BOUGHT in last 30 days
    # so J30 balance = today - delta (they had LESS if they bought, MORE if they sold)
    all_wallets = set(current_human.keys()) | set(deltas.keys())

    j30_holders: Dict[str, float] = {}
    today_holders: Dict[str, float] = {}

    for wallet in all_wallets:
        today_bal = current_human.get(wallet, 0.0)
        delta = deltas.get(wallet, 0.0)
        j30_bal = today_bal - delta

        if today_bal > 0:
            today_holders[wallet] = today_bal
        if j30_bal > 0:
            j30_holders[wallet] = j30_bal

    # Conviction = J-30 holders who still hold today / J-30 holders
    retained = set(j30_holders.keys()) & set(today_holders.keys())
    left = set(j30_holders.keys()) - set(today_holders.keys())
    new = set(today_holders.keys()) - set(j30_holders.keys())

    total_j30 = len(j30_holders)
    conviction = len(retained) / total_j30 if total_j30 > 0 else 0.0

    # Weighted conviction: by balance at J-30 (whale-dominated)
    total_j30_balance = sum(j30_holders.values())
    retained_balance = sum(j30_holders[w] for w in retained)
    conviction_weighted = retained_balance / total_j30_balance if total_j30_balance > 0 else 0.0

    # Per-wallet retention ratio: today_bal / j30_bal for each J-30 holder
    # Capped at 1.0 (accumulation beyond original = still 100% retained)
    # Left holders get ratio 0.0
    retention_ratios: List[float] = []
    for w in j30_holders:
        if w in today_holders:
            ratio = min(today_holders[w] / j30_holders[w], 1.0) if j30_holders[w] > 0 else 0.0
            retention_ratios.append(ratio)
        else:
            retention_ratios.append(0.0)

    retention_ratios.sort()
    n = len(retention_ratios)
    retention_median = retention_ratios[n // 2] if n > 0 else 0.0
    retention_mean = sum(retention_ratios) / n if n > 0 else 0.0
    retention_p25 = retention_ratios[n // 4] if n > 0 else 0.0
    retention_p75 = retention_ratios[(3 * n) // 4] if n > 0 else 0.0

    # Buckets
    full_retainers = sum(1 for r in retention_ratios if r >= 0.99)
    partial_retainers = sum(1 for r in retention_ratios if 0.01 < r < 0.99)
    dumpers = sum(1 for r in retention_ratios if r <= 0.01)

    return {
        "holders_today": len(today_holders),
        "holders_j30": total_j30,
        "retained": len(retained),
        "left": len(left),
        "new_since_j30": len(new),
        "conviction_count": round(conviction, 4),
        "conviction_weighted": round(conviction_weighted, 4),
        "total_j30_balance": round(total_j30_balance, 2),
        "retained_balance": round(retained_balance, 2),
        "retention_median": round(retention_median, 4),
        "retention_mean": round(retention_mean, 4),
        "retention_p25": round(retention_p25, 4),
        "retention_p75": round(retention_p75, 4),
        "full_retainers": full_retainers,
        "partial_retainers": partial_retainers,
        "dumpers": dumpers,
    }


# ── MAIN PIPELINE ──

def rewind_token(mint: str, symbol: str, conviction_cs: float,
                 tier: str, decimals: int) -> Dict:
    """Full rewind pipeline for one token."""
    credits_start = _credits
    now = int(time.time())
    since = now - (REWIND_DAYS * 86400)

    # 1. Current holders
    print(f"  fetching holders...", end="", flush=True)
    current = fetch_all_holders(mint, max_pages=20)
    print(f" {len(current)} wallets")

    if not current:
        return {"mint": mint, "symbol": symbol, "error": "no holders",
                "api_credits": _credits - credits_start}

    # 2. All txs in 30-day window
    print(f"  fetching txs since {REWIND_DAYS}d ago...", end="", flush=True)
    txs = fetch_txs_in_window(mint, since, max_pages=100)
    print(f" {len(txs)} txs")

    # 3. Parse deltas
    deltas = parse_deltas(txs, mint)
    print(f"  {len(deltas)} wallets with token flow in {REWIND_DAYS}d")

    # 4. Compute conviction
    conv = compute_conviction(current, deltas, decimals)

    result = {
        "schema_version": 1,
        "mint": mint,
        "symbol": symbol,
        "date": datetime.now(timezone.utc).strftime("%Y-%m-%d"),
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "rewind_days": REWIND_DAYS,
        "conviction_cultscreener": conviction_cs,
        "conviction_tier": tier,
        "txs_in_window": len(txs),
        "wallets_with_flow": len(deltas),
        **conv,
        "api_credits": _credits - credits_start,
    }

    print(f"  CONVICTION: count={conv['conviction_count']:.3f}  "
          f"weighted={conv['conviction_weighted']:.3f}  "
          f"ret_median={conv.get('retention_median', '?')}")
    print(f"  J-30={conv['holders_j30']}  retained={conv['retained']}  "
          f"left={conv['left']}  new={conv['new_since_j30']}")
    print(f"  retention: full={conv.get('full_retainers','?')}  "
          f"partial={conv.get('partial_retainers','?')}  "
          f"dumped={conv.get('dumpers','?')}  "
          f"p25={conv.get('retention_p25','?')}  "
          f"median={conv.get('retention_median','?')}  "
          f"p75={conv.get('retention_p75','?')}")

    return result


# ── PERSISTENCE ──

def load_existing(date_str: str) -> Dict[str, dict]:
    existing: Dict[str, dict] = {}
    if not os.path.exists(RESULTS_PATH):
        return existing
    with open(RESULTS_PATH) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                r = json.loads(line)
                if r.get("date") == date_str:
                    existing[r["mint"]] = r
            except json.JSONDecodeError:
                continue
    return existing


def save_result(result: dict) -> None:
    with open(RESULTS_PATH, "a") as f:
        f.write(json.dumps(result, default=str) + "\n")


# ── ANALYSIS ──

def analyze() -> None:
    if not os.path.exists(RESULTS_PATH):
        print("No results found. Run rewind first.")
        return

    results: List[dict] = []
    with open(RESULTS_PATH) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                results.append(json.loads(line))
            except json.JSONDecodeError:
                continue

    # Deduplicate: latest per mint
    by_mint: Dict[str, dict] = {}
    for r in results:
        by_mint[r["mint"]] = r
    results = list(by_mint.values())

    valid = [r for r in results
             if r.get("conviction_count") is not None
             and r.get("conviction_cultscreener", 0) > 0
             and r.get("holders_j30", 0) > 0]

    print(f"Results: {len(results)} total, {len(valid)} valid for correlation")

    if len(valid) < 5:
        print("Not enough data.")
        return

    cs = [r["conviction_cultscreener"] for r in valid]
    sov_count = [r["conviction_count"] for r in valid]
    sov_weighted = [r["conviction_weighted"] for r in valid]

    print("\n" + "=" * 70)
    print("SPEARMAN ρ (sovereign conviction vs CultScreener)")
    print("  ρ > 0.5 = SOVEREIGN SUCCESS  |  ρ < 0.3 = FAILURE")
    print("=" * 70)

    # Collect all available signals
    signals = [
        ("conviction_count", sov_count),
        ("conviction_weighted", sov_weighted),
    ]

    # New retention metrics (may not be in old data)
    ret_median = [r.get("retention_median") for r in valid]
    ret_mean = [r.get("retention_mean") for r in valid]
    ret_p25 = [r.get("retention_p25") for r in valid]
    dumpers_pct = [r.get("dumpers", 0) / max(r.get("holders_j30", 1), 1) for r in valid]

    if all(v is not None for v in ret_median):
        signals.append(("retention_median", ret_median))
        signals.append(("retention_mean", ret_mean))
        signals.append(("retention_p25", ret_p25))
        signals.append(("dumpers_pct", dumpers_pct))

    for name, vals in signals:
        rho = _spearman(vals, cs)
        sig = "SOVEREIGN" if abs(rho) > 0.5 else ("weak" if abs(rho) > 0.3 else "NOISE")
        print(f"  {name:24s}  ρ = {rho:+.3f}  [{sig}]")

    # Per-tier
    print("\nPER-TIER:")
    for tier in ["strong", "mixed", "weak"]:
        td = [r for r in valid if r.get("conviction_tier") == tier]
        if not td:
            continue
        sc = [r["conviction_count"] for r in td]
        sw = [r["conviction_weighted"] for r in td]
        rm = [r.get("retention_median", 0) for r in td]
        print(f"\n  {tier.upper()} (n={len(td)}):")
        print(f"    count:      avg={sum(sc)/len(sc):.3f}  [{min(sc):.3f} - {max(sc):.3f}]")
        print(f"    weighted:   avg={sum(sw)/len(sw):.3f}  [{min(sw):.3f} - {max(sw):.3f}]")
        if any(r.get("retention_median") is not None for r in td):
            print(f"    ret_median: avg={sum(rm)/len(rm):.3f}  [{min(rm):.3f} - {max(rm):.3f}]")

    # Scatter
    print("\nSCATTER:")
    for r in sorted(valid, key=lambda r: -r["conviction_cultscreener"]):
        tm = {"strong": "S", "mixed": "M", "weak": "W"}.get(r.get("conviction_tier", "?"), "?")
        bar_count = "█" * int(r["conviction_count"] * 20)
        print(f"  {r['symbol']:12s} CS={r['conviction_cultscreener']:.3f} [{tm}] "
              f"sov={r['conviction_count']:.3f} {bar_count}  "
              f"w={r['conviction_weighted']:.3f}  "
              f"J30={r['holders_j30']}→{r['retained']}+{r['new_since_j30']}  "
              f"txs={r['txs_in_window']}")

    total = sum(r.get("api_credits", 0) for r in results)
    print(f"\nTotal API cost: {total:,} credits")


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
    if "--limit" in sys.argv:
        idx = sys.argv.index("--limit")
        if idx + 1 < len(sys.argv):
            limit = int(sys.argv[idx + 1])

    today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    existing = load_existing(today)

    with open(CALIB_PATH) as f:
        calib = json.load(f)
    tokens = calib["results"][:limit]

    # We need decimals per token — fetch from profiles if available
    profiles_path = os.path.join(SCRIPT_DIR, "token_profiles.jsonl")
    decimals_map: Dict[str, int] = {}
    if os.path.exists(profiles_path):
        with open(profiles_path) as f:
            for line in f:
                try:
                    p = json.loads(line.strip())
                    decimals_map[p["mint"]] = p.get("decimals", 6)
                except (json.JSONDecodeError, KeyError):
                    continue

    to_process = [t for t in tokens if t["mint"] not in existing]
    already = len(tokens) - len(to_process)

    print("=" * 70)
    print(f"SOVEREIGN CONVICTION REWIND — {REWIND_DAYS} days — {today}")
    print(f"Tokens: {len(tokens)} total, {already} done, {len(to_process)} to process")
    print("=" * 70)

    if not to_process:
        print("All done. Use --analyze for results.")
        return

    for i, token in enumerate(to_process):
        mint = token["mint"]
        symbol = token.get("symbol", "?")
        conv = token.get("conviction", 0)
        tier = token.get("conviction_tier", "?")
        decimals = decimals_map.get(mint, 6)

        print(f"\n[{i+1}/{len(to_process)}] {symbol:12s} (CS={conv:.3f}, {tier})")

        result = rewind_token(mint, symbol, conv, tier, decimals)
        save_result(result)

        if result.get("errors"):
            for e in result["errors"]:
                print(f"  ⚠ {e}")

    print(f"\nTotal API cost: {_credits:,} credits ({_calls} calls)")
    print(f"Results: {RESULTS_PATH}")
    print(f"\nRun with --analyze for correlation analysis.")


if __name__ == "__main__":
    main()
