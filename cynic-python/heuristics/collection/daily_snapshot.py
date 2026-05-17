#!/usr/bin/env python3
"""Daily Token Snapshot — sovereign conviction accumulator.

Runs daily via cron. Persists holder snapshots to build temporal conviction
without transaction rewind (2 real snapshots → direct comparison).

Architecture:
  snapshots/
    {mint_short}/
      holders_{YYYY-MM-DD}.json     — full holder list {owner: balance_raw}
      metadata_{YYYY-MM-DD}.json    — price, supply, holder_count

After 7+ days: compare holders_today vs holders_7d_ago → sovereign conviction.
After 30+ days: multi-window decay curve (3d/7d/14d/30d).

Tokens tracked: calibration set (33) + any mints in watchlist.json.

Cost: ~40 cr/token/day (getTokenAccounts 3 pages + getAsset).
  33 tokens × 40 cr = ~1,320 cr/day = 0.013% of Developer plan.

Usage:
    python3 daily_snapshot.py                  # snapshot all tracked tokens
    python3 daily_snapshot.py --compute        # compute conviction from stored snapshots (0 API)
    python3 daily_snapshot.py --status         # show snapshot coverage
    python3 daily_snapshot.py --add-mint <m>   # add mint to watchlist

Cron (systemd timer or crontab):
    0 6 * * * cd /home/user/Bureau/CYNIC && python3 cynic-python/heuristics/daily_snapshot.py
"""

import json
import os
import sys
import time
from datetime import datetime, timezone, timedelta
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

HELIUS_RPC = "https://mainnet.helius-rpc.com/"
HELIUS_HEADERS = {"Content-Type": "application/json", "Authorization": f"Bearer {HELIUS_API_KEY}"}

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
SNAPSHOTS_DIR = os.path.join(SCRIPT_DIR, "snapshots")
WATCHLIST_PATH = os.path.join(SCRIPT_DIR, "watchlist.json")
CALIB_PATH = os.path.join(SCRIPT_DIR, "calibration_results_real.json")

POOL_PROGRAMS: Set[str] = {
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",
    "5quBtoiQqxF9Jv6KYKctB59NT3gtJD2Y65kdnB1Uev3h", "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc",
    "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P", "39azUYFWPz3VHgKCf3VChUwbpURdCHRxjWVowf5jUJjg",
    "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA", "TSLvdd1pWpHVjahSpsvCXUbgwsL3JAcvokwaKt1eokM",
    "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4", "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB",
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo",
    "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB",
    "CebN5WGQ4jvEPvsVU4EoHEpgzq1VV7AbicfhtW4xC9iM",
}

_credits = 0
_calls = 0


# ── API ──

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


# ── SNAPSHOT ──

def snapshot_token(mint: str, date_str: str, max_pages: int = 20) -> Dict:
    """Snapshot one mint: holders + metadata. Persisted to disk."""
    mint_dir = os.path.join(SNAPSHOTS_DIR, mint[:16])
    os.makedirs(mint_dir, exist_ok=True)

    holders_path = os.path.join(mint_dir, f"holders_{date_str}.json")
    meta_path = os.path.join(mint_dir, f"metadata_{date_str}.json")

    # Skip if already snapshotted today
    if os.path.exists(holders_path) and os.path.exists(meta_path):
        with open(holders_path) as f:
            holders = json.load(f)
        with open(meta_path) as f:
            meta = json.load(f)
        return {"cached": True, "holders": len(holders), "meta": meta}

    # Fetch holders (paginated)
    holders: Dict[str, int] = {}
    for page in range(1, max_pages + 1):
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

    # Fetch metadata
    meta = {"mint": mint, "date": date_str, "holder_count": len(holders)}
    asset = _rpc("getAsset", {"id": mint}, credits=10)
    if asset:
        ti = asset.get("token_info", {})
        price_info = ti.get("price_info", {})
        meta["price_usd"] = price_info.get("price_per_token")
        meta["supply"] = int(ti.get("supply", 0))
        meta["decimals"] = int(ti.get("decimals", 0))
        meta["symbol"] = asset.get("content", {}).get("metadata", {}).get("symbol")
        meta["name"] = asset.get("content", {}).get("metadata", {}).get("name")

    # Persist
    with open(holders_path, "w") as f:
        json.dump(holders, f)
    with open(meta_path, "w") as f:
        json.dump(meta, f, indent=2)

    return {"cached": False, "holders": len(holders), "meta": meta}


# ── CONVICTION COMPUTATION (from stored snapshots, 0 API) ──

def compute_conviction_from_snapshots(mint: str, window_days: int = 7) -> Optional[Dict]:
    """Compare snapshot from N days ago with today's snapshot.

    Returns conviction metrics or None if insufficient data.
    """
    mint_dir = os.path.join(SNAPSHOTS_DIR, mint[:16])
    if not os.path.exists(mint_dir):
        return None

    today = datetime.now(timezone.utc).date()
    target_date = today - timedelta(days=window_days)

    # Find today's snapshot
    today_path = os.path.join(mint_dir, f"holders_{today.isoformat()}.json")
    if not os.path.exists(today_path):
        return None

    # Find closest snapshot to target_date
    past_path = os.path.join(mint_dir, f"holders_{target_date.isoformat()}.json")
    if not os.path.exists(past_path):
        # Try ±1 day
        for offset in range(1, 3):
            for delta in [timedelta(days=offset), timedelta(days=-offset)]:
                alt_date = target_date + delta
                alt_path = os.path.join(mint_dir, f"holders_{alt_date.isoformat()}.json")
                if os.path.exists(alt_path):
                    past_path = alt_path
                    break
            if os.path.exists(past_path):
                break
        else:
            return None  # No past snapshot found

    # Load both snapshots
    with open(today_path) as f:
        today_holders = json.load(f)
    with open(past_path) as f:
        past_holders = json.load(f)

    # Compute conviction: holders in past that are still present today
    retained = set(past_holders.keys()) & set(today_holders.keys())
    left = set(past_holders.keys()) - set(today_holders.keys())
    new = set(today_holders.keys()) - set(past_holders.keys())

    total_past = len(past_holders)
    conviction = len(retained) / total_past if total_past > 0 else 0

    return {
        "mint": mint,
        "window_days": window_days,
        "date_today": today.isoformat(),
        "date_past": os.path.basename(past_path).replace("holders_", "").replace(".json", ""),
        "holders_past": total_past,
        "holders_today": len(today_holders),
        "retained": len(retained),
        "left": len(left),
        "new": len(new),
        "conviction": round(conviction, 4),
    }


# ── TOKEN LIST ──

def get_tracked_tokens() -> List[Dict]:
    """Load tokens to track: calibration set + watchlist."""
    tokens = []

    # Calibration set
    if os.path.exists(CALIB_PATH):
        with open(CALIB_PATH) as f:
            calib = json.load(f)
        for r in calib.get("results", []):
            tokens.append({"mint": r["mint"], "symbol": r.get("symbol", "?"), "source": "calibration"})

    # Watchlist
    if os.path.exists(WATCHLIST_PATH):
        with open(WATCHLIST_PATH) as f:
            watchlist = json.load(f)
        for t in watchlist:
            if t["mint"] not in {x["mint"] for x in tokens}:
                tokens.append({"mint": t["mint"], "symbol": t.get("symbol", "?"), "source": "watchlist"})

    return tokens


# ── HISTORICAL BACKFILL ──

def backfill_from_cache(mint: str) -> int:
    """Import existing helius_cache data into snapshots/ format.

    The helius_cache/ already has holder data from today's runs.
    This converts it to the daily snapshot format for continuity.
    Returns number of snapshots created.
    """
    cache_dir = os.path.join(SCRIPT_DIR, "helius_cache")
    if not os.path.exists(cache_dir):
        return 0

    created = 0
    # Look for cached holder files
    for fname in os.listdir(cache_dir):
        if fname.startswith(f"holders_{mint[:16]}") and fname.endswith(".json"):
            # Extract date from filename: holders_{mint16}_{date}.json
            parts = fname.replace(".json", "").split("_")
            if len(parts) >= 3:
                date_str = parts[-1]
                # Load and convert
                cache_path = os.path.join(cache_dir, fname)
                with open(cache_path) as f:
                    holders = json.load(f)

                mint_dir = os.path.join(SNAPSHOTS_DIR, mint[:16])
                os.makedirs(mint_dir, exist_ok=True)
                snap_path = os.path.join(mint_dir, f"holders_{date_str}.json")

                if not os.path.exists(snap_path):
                    with open(snap_path, "w") as f:
                        json.dump(holders, f)
                    created += 1

    return created


# ── COMMANDS ──

def cmd_snapshot() -> None:
    """Main: take daily snapshot of all tracked tokens."""
    today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    tokens = get_tracked_tokens()

    print(f"{'='*60}")
    print(f"DAILY SNAPSHOT — {today}")
    print(f"Tokens: {len(tokens)}  |  Est. cost: ~{len(tokens) * 40} cr")
    print(f"{'='*60}")

    os.makedirs(SNAPSHOTS_DIR, exist_ok=True)

    for i, token in enumerate(tokens):
        mint = token["mint"]
        symbol = token["symbol"]
        result = snapshot_token(mint, today)

        status = "CACHE" if result.get("cached") else "FETCH"
        holders = result.get("holders", 0)
        price = result.get("meta", {}).get("price_usd")
        price_str = f"${price:.8f}" if price else "N/A"

        print(f"  [{i+1}/{len(tokens)}] {symbol:12s} [{status}] {holders:>6,} holders  {price_str}")

    print(f"\nAPI: {_credits} credits, {_calls} calls")
    print(f"Snapshots: {SNAPSHOTS_DIR}/")


def cmd_compute() -> None:
    """Compute conviction from stored snapshots (0 API calls)."""
    tokens = get_tracked_tokens()
    windows = [3, 7, 14, 30]

    print(f"{'='*60}")
    print("CONVICTION FROM SNAPSHOTS (0 API calls)")
    print(f"{'='*60}")

    print(f"\n{'Token':12s}", end="")
    for w in windows:
        print(f" {w:>3d}d", end="")
    print("  holders_today")
    print("-" * 60)

    for token in tokens:
        mint = token["mint"]
        symbol = token["symbol"]

        print(f"{symbol:12s}", end="")
        any_data = False

        for w in windows:
            result = compute_conviction_from_snapshots(mint, w)
            if result:
                any_data = True
                print(f" {result['conviction']:.2f}", end="")
            else:
                print(f"   - ", end="")

        # Today's holder count
        today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
        today_path = os.path.join(SNAPSHOTS_DIR, mint[:16], f"holders_{today}.json")
        if os.path.exists(today_path):
            with open(today_path) as f:
                h = json.load(f)
            print(f"  {len(h):>8,}")
        else:
            print(f"  {'N/A':>8s}")


def cmd_status() -> None:
    """Show snapshot coverage per token."""
    tokens = get_tracked_tokens()

    print(f"{'='*60}")
    print("SNAPSHOT STATUS")
    print(f"{'='*60}")

    total_snapshots = 0
    for token in tokens:
        mint = token["mint"]
        symbol = token["symbol"]
        mint_dir = os.path.join(SNAPSHOTS_DIR, mint[:16])

        if not os.path.exists(mint_dir):
            print(f"  {symbol:12s} — no snapshots")
            continue

        holder_files = sorted([f for f in os.listdir(mint_dir) if f.startswith("holders_")])
        total_snapshots += len(holder_files)

        if holder_files:
            first = holder_files[0].replace("holders_", "").replace(".json", "")
            last = holder_files[-1].replace("holders_", "").replace(".json", "")
            span = len(holder_files)
            print(f"  {symbol:12s} {span:>3d} snapshots  [{first} → {last}]")
        else:
            print(f"  {symbol:12s} — no holder snapshots")

    print(f"\nTotal: {total_snapshots} snapshots across {len(tokens)} tokens")


def cmd_add_mint(mint: str) -> None:
    """Add a mint to the watchlist."""
    watchlist = []
    if os.path.exists(WATCHLIST_PATH):
        with open(WATCHLIST_PATH) as f:
            watchlist = json.load(f)

    if any(t["mint"] == mint for t in watchlist):
        print(f"Already in watchlist: {mint}")
        return

    # Fetch symbol
    asset = _rpc("getAsset", {"id": mint}, credits=10)
    symbol = "?"
    if asset:
        symbol = asset.get("content", {}).get("metadata", {}).get("symbol", "?")

    watchlist.append({"mint": mint, "symbol": symbol})
    with open(WATCHLIST_PATH, "w") as f:
        json.dump(watchlist, f, indent=2)
    print(f"Added: {symbol} ({mint})")


# ── MAIN ──

def main() -> None:
    if "--compute" in sys.argv:
        cmd_compute()
    elif "--status" in sys.argv:
        cmd_status()
    elif "--add-mint" in sys.argv:
        idx = sys.argv.index("--add-mint")
        if idx + 1 < len(sys.argv):
            cmd_add_mint(sys.argv[idx + 1])
    elif "--backfill-cache" in sys.argv:
        # Import existing helius_cache data into snapshots format
        tokens = get_tracked_tokens()
        total = 0
        for t in tokens:
            n = backfill_from_cache(t["mint"])
            if n:
                total += n
                print(f"  {t['symbol']:12s} — imported {n} snapshots from cache")
        print(f"\nTotal imported: {total}")
    else:
        cmd_snapshot()


if __name__ == "__main__":
    main()
