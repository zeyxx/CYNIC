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
import urllib.request
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
DATA_DIR = os.path.join(SCRIPT_DIR, "..", "data")
SNAPSHOTS_DIR = os.path.join(DATA_DIR, "snapshots")
WATCHLIST_PATH = os.path.join(SCRIPT_DIR, "watchlist.json")
CALIB_PATH = os.path.join(DATA_DIR, "calibration_results_real.json")

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

    # Find closest snapshot to target_date (skip empty snapshots — API failures)
    def _find_valid_snapshot(target: "date") -> Optional[str]:
        """Find a non-empty snapshot closest to target date, ±2 days."""
        candidates = [target]
        for offset in range(1, 3):
            candidates.append(target + timedelta(days=offset))
            candidates.append(target - timedelta(days=offset))
        for d in candidates:
            path = os.path.join(mint_dir, f"holders_{d.isoformat()}.json")
            if os.path.exists(path):
                with open(path) as f:
                    data = json.load(f)
                if len(data) > 0:  # skip empty snapshots (API failure)
                    return path
        return None

    past_path = _find_valid_snapshot(target_date)
    if past_path is None:
        return None

    # Load both snapshots
    with open(today_path) as f:
        today_holders = json.load(f)
    if len(today_holders) == 0:
        return None  # today's snapshot is also empty
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
    cache_dir = os.path.join(DATA_DIR, "helius_cache")
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


# ── TRAJECTORY CLASSIFICATION ──

def classify_trajectory(windows: Dict[int, Optional[Dict]]) -> Dict:
    """Classify token trajectory from multi-window conviction decay curve.

    The decay curve shows how much conviction is LOST between short and long windows.
    Short-term retention (3d) is always >= long-term (30d) — what matters is the RATE.

    Classes:
      DEAD:     30d conviction < 1% (no one stays)
      DYING:    decay > 30% between shortest and longest window (steep churn)
      DECLINING: decay 15-30% (moderate churn)
      STABLE:   decay < 15% (diamond hands)
      UNKNOWN:  insufficient data (<2 windows)
    """
    # Collect (window, conviction) pairs
    points = []
    for w in sorted(windows.keys()):
        if windows[w] is not None:
            points.append((w, windows[w]["conviction"]))

    if len(points) < 2:
        return {"class": "UNKNOWN", "points": len(points), "decay": 0.0, "conviction_delta": 0.0, "conviction_30d": None}

    short_conv = points[0][1]    # e.g. 3d conviction (highest)
    long_conv = points[-1][1]    # e.g. 30d conviction (lowest)
    decay = short_conv - long_conv  # how much is lost over the window span

    # Decay rate per day
    day_span = points[-1][0] - points[0][0]
    decay_rate = decay / day_span if day_span > 0 else 0.0

    if long_conv < 0.01:
        traj_class = "DEAD"
    elif decay > 0.30:
        traj_class = "DYING"
    elif decay > 0.15:
        traj_class = "DECLINING"
    else:
        traj_class = "STABLE"

    return {
        "class": traj_class,
        "decay": round(decay, 4),
        "conviction_delta": round(decay, 4),  # alias for unify.py compatibility
        "decay_rate": round(decay_rate, 5),
        "conviction_3d": short_conv,
        "conviction_30d": long_conv,
        "points": len(points),
    }


def cmd_trajectory() -> None:
    """Classify token trajectories and POST to kernel /observe."""
    tokens = get_tracked_tokens()
    windows = [3, 7, 14, 30]

    print(f"{'='*70}")
    print("TOKEN TRAJECTORY — decay curve classification")
    print(f"{'='*70}")
    print(f"\n{'Token':12s} {'Class':13s} {'3d':>5s} {'7d':>5s} {'14d':>5s} {'30d':>5s}  {'decay':>10s}")
    print("-" * 70)

    today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    results = []
    for token in tokens:
        mint = token["mint"]
        symbol = token["symbol"]

        window_data: Dict[int, Optional[Dict]] = {}
        for w in windows:
            window_data[w] = compute_conviction_from_snapshots(mint, w)

        traj = classify_trajectory(window_data)
        traj["mint"] = mint
        traj["symbol"] = symbol
        # Persist trajectory file for unify.py (K15: producer→consumer contract)
        traj_path = os.path.join(SNAPSHOTS_DIR, mint[:16], f"trajectory_{today}.json")
        os.makedirs(os.path.dirname(traj_path), exist_ok=True)
        with open(traj_path, "w") as f:
            json.dump(traj, f)
        results.append(traj)

        # Format conviction values
        conv_strs = []
        for w in windows:
            if window_data[w] is not None:
                conv_strs.append(f"{window_data[w]['conviction']:.2f}")
            else:
                conv_strs.append("   - ")

        class_icon = {"DYING": "!", "DEAD": "X", "DECLINING": "~", "STABLE": "=", "UNKNOWN": "?"}.get(traj["class"], "?")
        print(f"{symbol:12s} [{class_icon}] {traj['class']:10s} {conv_strs[0]:>5s} {conv_strs[1]:>5s} {conv_strs[2]:>5s} {conv_strs[3]:>5s}  decay={traj['decay']:>+.3f}")

    # Summary
    classes = {}
    for r in results:
        classes[r["class"]] = classes.get(r["class"], 0) + 1
    print(f"\nDistribution: {classes}")

    # POST to kernel /observe
    cynic_addr = os.getenv("CYNIC_REST_ADDR")
    cynic_key = os.getenv("CYNIC_API_KEY")
    if cynic_addr and cynic_key:
        try:
            observation = {
                "tool": "trajectory_cron",
                "target": "token-trajectory",
                "domain": "token-analysis",
                "status": "measured",
                "context": json.dumps({
                    "date": datetime.now(timezone.utc).strftime("%Y-%m-%d"),
                    "tokens": len(results),
                    "distribution": classes,
                    "trajectories": [
                        {"symbol": r["symbol"], "mint": r["mint"], "class": r["class"],
                         "decay": r["decay"], "conviction_30d": r["conviction_30d"]}
                        for r in results
                    ],
                }),
                "tags": ["trajectory", "cron", "sovereign"],
                "consumer": "deterministic-dog",
                "action": "token trajectory feeds conviction decay into judgment pipeline",
                "confidence": "observed",
            }
            resp = requests.post(
                f"http://{cynic_addr}/observe",
                json=observation,
                headers={"Authorization": f"Bearer {cynic_key}", "Content-Type": "application/json"},
                timeout=5,
            )
            print(f"\nKernel POST: {resp.status_code}")
        except Exception as e:
            print(f"\nKernel POST failed (non-blocking): {e}")
    else:
        print("\nKernel POST skipped (CYNIC_REST_ADDR/CYNIC_API_KEY not set)")


# ── MARKET SNAPSHOT (GeckoTerminal) ──


def cmd_market() -> None:
    """Snapshot price/volume/mcap from GeckoTerminal for all tracked tokens.

    Cost: 0 (free API, no key). Rate limit: 10 req/min.
    Persists to data/market_snapshots/market_snapshot_{YYYY-MM-DD}.json.
    """
    tokens = get_tracked_tokens()
    today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    out_dir = os.path.join(os.path.dirname(DATA_DIR), "data", "market_snapshots")
    os.makedirs(out_dir, exist_ok=True)
    out_path = os.path.join(out_dir, "market_snapshot_" + today + ".json")

    if os.path.exists(out_path):
        with open(out_path) as f:
            existing = json.load(f)
        if len(existing.get("tokens", [])) >= len(tokens) * 0.8:
            print("Market snapshot already exists for " + today + ". Skip.")
            return

    results: list = []
    errors: list = []

    for i, tok in enumerate(tokens):
        mint = tok["mint"]
        symbol = tok.get("symbol", "?").strip()
        print("[" + str(i + 1) + "/" + str(len(tokens)) + "] " + symbol + "...", end=" ", flush=True)

        try:
            url = "https://api.geckoterminal.com/api/v2/networks/solana/tokens/" + mint + "/pools?page=1"
            req = urllib.request.Request(url, headers={"Accept": "application/json", "User-Agent": "CYNIC/1.0"})
            with urllib.request.urlopen(req, timeout=15) as resp:
                data = json.loads(resp.read())

            pools = data.get("data", [])
            if not pools:
                print("no pool")
                errors.append({"symbol": symbol, "mint": mint, "error": "no pool"})
                time.sleep(7)
                continue

            attrs = pools[0]["attributes"]
            snapshot = {
                "mint": mint,
                "symbol": symbol,
                "date": today,
                "price_usd": float(attrs.get("base_token_price_usd") or 0),
                "volume_24h": float((attrs.get("volume_usd") or {}).get("h24") or 0),
                "reserve_usd": float(attrs.get("reserve_in_usd") or 0),
                "fdv_usd": float(attrs.get("fdv_usd") or 0),
                "market_cap_usd": float(attrs.get("market_cap_usd") or 0),
            }
            results.append(snapshot)
            print("$" + format(snapshot["price_usd"], ".6f") + " vol=$" + format(snapshot["volume_24h"], ".0f"))
            time.sleep(7)

        except Exception as e:
            print("ERROR: " + str(e))
            errors.append({"symbol": symbol, "mint": mint, "error": str(e)})
            time.sleep(10)

    with open(out_path, "w") as f:
        json.dump({"date": today, "tokens": results, "errors": errors}, f, indent=2)
    print("\nSaved " + str(len(results)) + " tokens to " + out_path)


# ── MAIN ──


def main() -> None:
    if "--market" in sys.argv:
        cmd_market()
    elif "--trajectory" in sys.argv:
        cmd_trajectory()
    elif "--compute" in sys.argv:
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
