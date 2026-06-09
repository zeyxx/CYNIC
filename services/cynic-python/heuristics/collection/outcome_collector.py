#!/usr/bin/env python3
"""Outcome Collector — re-query GeckoTerminal at T+7/14/30 for judged tokens.

Tier 2 INFRASTRUCTURE: Collects ground truth outcomes by comparing
judgment-time prices with actual prices at defined intervals.

K15 Consumer: unified profiles (unify.py), ground truth validation.
Systemd: token-snapshot.service (appended as ExecStartPost after wiring).

Usage:
    python3 outcome_collector.py                # collect outcomes for all due tokens
    python3 outcome_collector.py --dry-run      # show what would be collected
    python3 outcome_collector.py --status       # show collection status
"""

from __future__ import annotations

import json
import os
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone, timedelta
from pathlib import Path
from typing import Any, Optional


__version__ = "0.1.0"

# Outcome windows (days after judgment)
OUTCOME_WINDOWS = [7, 14, 30]

# GeckoTerminal rate limit: 10 req/min → 7s between requests
RATE_LIMIT_SECONDS = 7


def find_data_root() -> Path:
    """Find the data directory by reading MANIFESTs or falling back to known path."""
    # Try to find via MANIFEST
    project_root = Path(__file__).resolve().parents[3]  # cynic-python/heuristics/collection/ → project root
    manifest_path = Path(__file__).resolve().parent / "MANIFEST.yaml"

    # Fallback: known relative path
    data_dir = Path(__file__).resolve().parent.parent / "data"
    if data_dir.exists():
        return data_dir

    # Try project root
    alt = project_root / "cynic-python" / "heuristics" / "data"
    if alt.exists():
        return alt

    print(f"ERROR: Cannot find data directory", file=sys.stderr)
    sys.exit(1)


def load_market_snapshots(data_root: Path) -> dict[str, dict[str, Any]]:
    """Load all market snapshots indexed by date."""
    snapshots_dir = data_root / "market_snapshots"
    snapshots: dict[str, dict[str, Any]] = {}

    if not snapshots_dir.exists():
        return snapshots

    for f in sorted(snapshots_dir.glob("market_snapshot_*.json")):
        date_str = f.stem.replace("market_snapshot_", "")
        try:
            with open(f) as fh:
                data = json.load(fh)
                snapshots[date_str] = data
        except (json.JSONDecodeError, OSError) as e:
            print(f"  WARN: Cannot read {f.name}: {e}", file=sys.stderr)

    return snapshots


def load_existing_outcomes(outcomes_dir: Path) -> dict[str, dict[str, Any]]:
    """Load existing outcome records indexed by key (mint:judgment_date:window)."""
    existing: dict[str, dict[str, Any]] = {}

    if not outcomes_dir.exists():
        return existing

    for f in sorted(outcomes_dir.glob("outcomes_*.jsonl")):
        try:
            with open(f) as fh:
                for line in fh:
                    line = line.strip()
                    if not line:
                        continue
                    record = json.loads(line)
                    key = f"{record['mint']}:{record['judgment_date']}:{record['window_days']}"
                    existing[key] = record
        except (json.JSONDecodeError, OSError, KeyError):
            pass

    return existing


def find_due_outcomes(snapshots: dict[str, dict[str, Any]],
                      existing: dict[str, dict[str, Any]],
                      today: datetime) -> list[dict[str, Any]]:
    """Find tokens that need outcome collection (judgment date + window = today or past)."""
    due: list[dict[str, Any]] = []

    for date_str, snapshot in snapshots.items():
        try:
            judgment_date = datetime.strptime(date_str, "%Y-%m-%d").replace(tzinfo=timezone.utc)
        except ValueError:
            continue

        tokens = snapshot.get("tokens", [])
        for token in tokens:
            mint = token.get("mint", "")
            if not mint:
                continue

            for window in OUTCOME_WINDOWS:
                due_date = judgment_date + timedelta(days=window)
                if due_date.date() > today.date():
                    continue  # Not due yet

                key = f"{mint}:{date_str}:{window}"
                if key in existing:
                    continue  # Already collected

                due.append({
                    "mint": mint,
                    "symbol": token.get("symbol", "?"),
                    "judgment_date": date_str,
                    "window_days": window,
                    "due_date": due_date.strftime("%Y-%m-%d"),
                    "price_at_judgment": token.get("price_usd", 0),
                    "volume_at_judgment": token.get("volume_24h", 0),
                    "mcap_at_judgment": token.get("market_cap_usd", 0),
                    "conviction_at_judgment": token.get("conviction", None),
                    "conviction_tier_at_judgment": token.get("conviction_tier", None),
                })

    return due


def query_geckoterminal(mint: str, max_retries: int = 3) -> Optional[dict[str, Any]]:
    """Query GeckoTerminal for current price/volume/mcap of a Solana token.

    Distinguishes transient from permanent errors (P18). HTTP 429 (rate-limit)
    and 503 (overloaded) are transient → retry with exponential backoff, honoring
    Retry-After when present. Permanent errors (400/404/422) return None immediately.
    On retry exhaustion the token stays 'due', so a later run re-attempts it — no
    silent data loss. Incident: a 429 cascade truncated collection at 23/45 because
    the caller's rate-limit sleep was skipped on the error path (2026-05-30).
    """
    url = f"https://api.geckoterminal.com/api/v2/networks/solana/tokens/{mint}/pools?page=1"
    req = urllib.request.Request(url, headers={
        "Accept": "application/json",
        "User-Agent": "CYNIC/1.0",
    })

    backoff = 15
    for attempt in range(max_retries + 1):
        try:
            with urllib.request.urlopen(req, timeout=15) as resp:
                data = json.loads(resp.read())

            pools = data.get("data", [])
            if not pools:
                # No pool = the token has no live liquidity = DEAD. This is a real outcome,
                # NOT a fetch failure — return zeros so it is recorded as survived=false.
                # Conflating this with API errors (returning None → skipped) biased the
                # ground truth to survivors only (126/126 survived=true, 2026-05-30).
                return {
                    "price_usd": 0.0,
                    "volume_24h": 0.0,
                    "reserve_usd": 0.0,
                    "fdv_usd": 0.0,
                    "market_cap_usd": 0.0,
                    "no_pool": True,
                }

            attrs = pools[0]["attributes"]
            return {
                "price_usd": float(attrs.get("base_token_price_usd") or 0),
                "volume_24h": float((attrs.get("volume_usd") or {}).get("h24") or 0),
                "reserve_usd": float(attrs.get("reserve_in_usd") or 0),
                "fdv_usd": float(attrs.get("fdv_usd") or 0),
                "market_cap_usd": float(attrs.get("market_cap_usd") or 0),
            }
        except urllib.error.HTTPError as e:
            if e.code in (429, 503) and attempt < max_retries:
                retry_after = e.headers.get("Retry-After")
                hinted = int(retry_after) if (retry_after and retry_after.isdigit()) else 0
                # Never wait below the exponential floor — a `Retry-After: 0` must not
                # defeat the backoff and re-trigger the 429 cascade (2026-05-30).
                wait = max(hinted, backoff)
                print(f"  {e.code} on {mint[:8]}..., backoff {wait}s "
                      f"(attempt {attempt + 1}/{max_retries})", file=sys.stderr)
                time.sleep(wait)
                backoff *= 2
                continue
            # Permanent (400/404/422) or retries exhausted.
            print(f"  ERROR querying {mint[:8]}...: {e}", file=sys.stderr)
            return None
        except Exception as e:
            print(f"  ERROR querying {mint[:8]}...: {e}", file=sys.stderr)
            return None
    return None


def collect_outcomes(due: list[dict[str, Any]], outcomes_dir: Path,
                     dry_run: bool = False) -> int:
    """Collect outcome data for due tokens. Returns count of collected outcomes."""
    if not due:
        print("No outcomes due for collection.")
        return 0

    # Group by mint to minimize API calls (same mint, different windows)
    by_mint: dict[str, list[dict[str, Any]]] = {}
    for item in due:
        by_mint.setdefault(item["mint"], []).append(item)

    today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    outcomes_dir.mkdir(parents=True, exist_ok=True)
    out_path = outcomes_dir / f"outcomes_{today}.jsonl"

    collected = 0
    total_mints = len(by_mint)

    for i, (mint, items) in enumerate(by_mint.items()):
        symbol = items[0].get("symbol", "?")
        windows = [item["window_days"] for item in items]
        print(f"[{i+1}/{total_mints}] {symbol} (windows: {windows})...", end=" ", flush=True)

        if dry_run:
            print(f"DRY RUN — would query GeckoTerminal")
            collected += len(items)
            continue

        current = query_geckoterminal(mint)
        if current is None:
            print("no data")
            # Hold the rate-limit cadence even on failure — skipping it here caused a
            # 429 cascade that truncated collection at 23/45 (2026-05-30).
            time.sleep(RATE_LIMIT_SECONDS)
            continue

        for item in items:
            price_at_j = item["price_at_judgment"]
            price_now = current["price_usd"]

            # Compute outcome metrics
            price_change = 0.0
            if price_at_j > 0:
                price_change = (price_now - price_at_j) / price_at_j

            volume_at_j = item["volume_at_judgment"]
            volume_now = current["volume_24h"]
            volume_change = 0.0
            if volume_at_j > 0:
                volume_change = (volume_now - volume_at_j) / volume_at_j

            outcome = {
                # v2 (2026-05-30): added `no_pool` + dead tokens now recorded (survived=false).
                # v1 rows omit `no_pool` and contain survivors only — consumers must filter by version (P17).
                "schema_version": 2,
                "collected_at": datetime.now(timezone.utc).isoformat(),
                "mint": mint,
                "symbol": symbol,
                "judgment_date": item["judgment_date"],
                "window_days": item["window_days"],
                "collection_date": today,
                # Judgment-time data
                "price_at_judgment": price_at_j,
                "volume_at_judgment": volume_at_j,
                "mcap_at_judgment": item["mcap_at_judgment"],
                "conviction_at_judgment": item["conviction_at_judgment"],
                "conviction_tier_at_judgment": item["conviction_tier_at_judgment"],
                # Current data
                "price_current": price_now,
                "volume_current": volume_now,
                "mcap_current": current["market_cap_usd"],
                "reserve_current": current["reserve_usd"],
                # Outcome metrics
                "price_change_pct": round(price_change, 6),
                "volume_change_pct": round(volume_change, 6),
                "no_pool": bool(current.get("no_pool", False)),
                "survived": price_now > 0 and current.get("reserve_usd", 0) > 100,
            }

            with open(out_path, "a") as f:
                f.write(json.dumps(outcome) + "\n")

            collected += 1

        price_str = f"${current['price_usd']:.6f}" if current else "?"
        print(f"{price_str} ({len(items)} outcomes)")
        time.sleep(RATE_LIMIT_SECONDS)

    print(f"\nCollected {collected} outcomes → {out_path}")
    return collected


def cmd_status(data_root: Path, outcomes_dir: Path) -> None:
    """Show outcome collection status."""
    snapshots = load_market_snapshots(data_root)
    existing = load_existing_outcomes(outcomes_dir)
    today = datetime.now(timezone.utc)
    due = find_due_outcomes(snapshots, existing, today)

    print(f"Market snapshots: {len(snapshots)} dates")
    print(f"Existing outcomes: {len(existing)} records")
    print(f"Due for collection: {len(due)} records")

    if due:
        # Group by window
        by_window: dict[int, int] = {}
        for item in due:
            w = item["window_days"]
            by_window[w] = by_window.get(w, 0) + 1
        for w in sorted(by_window):
            print(f"  T+{w}: {by_window[w]} tokens")

    # Show coverage
    total_possible = 0
    for date_str, snapshot in snapshots.items():
        judgment_date = datetime.strptime(date_str, "%Y-%m-%d").replace(tzinfo=timezone.utc)
        n_tokens = len(snapshot.get("tokens", []))
        for window in OUTCOME_WINDOWS:
            if (judgment_date + timedelta(days=window)).date() <= today.date():
                total_possible += n_tokens

    coverage = len(existing) / total_possible if total_possible > 0 else 0
    print(f"Coverage: {len(existing)}/{total_possible} ({coverage:.1%})")


def main() -> int:
    import argparse
    parser = argparse.ArgumentParser(description="Outcome Collector — ground truth at T+7/14/30")
    parser.add_argument("--dry-run", action="store_true", help="Show what would be collected")
    parser.add_argument("--status", action="store_true", help="Show collection status")
    args = parser.parse_args()

    data_root = find_data_root()
    outcomes_dir = data_root / "outcomes"

    if args.status:
        cmd_status(data_root, outcomes_dir)
        return 0

    snapshots = load_market_snapshots(data_root)
    existing = load_existing_outcomes(outcomes_dir)
    today = datetime.now(timezone.utc)
    due = find_due_outcomes(snapshots, existing, today)

    if not due:
        print("No outcomes due. Next collection window not reached yet.")
        return 0

    collected = collect_outcomes(due, outcomes_dir, dry_run=args.dry_run)
    return 0 if collected >= 0 else 1


if __name__ == "__main__":
    sys.exit(main())
