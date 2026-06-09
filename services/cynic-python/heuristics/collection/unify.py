#!/usr/bin/env python3
"""Unify — merge all token-analysis sources into unified profiles.

Tier 2 INFRASTRUCTURE: Reads MANIFESTs to discover data sources, merges
holder snapshots + market data + conviction + outcomes into a single
unified profile per token per date.

K15 Consumer: ground truth validation, Dog calibration, trajectory analysis.

Usage:
    python3 unify.py                    # unify today's data
    python3 unify.py --date 2026-05-19  # unify specific date
    python3 unify.py --all              # unify all available dates
    python3 unify.py --status           # show available data sources
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Optional


__version__ = "0.1.0"

SCHEMA_VERSION = 2


def find_data_root() -> Path:
    """Find the data directory."""
    data_dir = Path(__file__).resolve().parent.parent / "data"
    if data_dir.exists():
        return data_dir
    print("ERROR: Cannot find data directory", file=sys.stderr)
    sys.exit(1)


def load_market_snapshot(data_root: Path, date_str: str) -> dict[str, dict[str, Any]]:
    """Load market snapshot for a given date, indexed by mint."""
    path = data_root / "market_snapshots" / f"market_snapshot_{date_str}.json"
    if not path.exists():
        return {}

    try:
        with open(path) as f:
            data = json.load(f)
        tokens = data.get("tokens", [])
        return {t["mint"]: t for t in tokens if "mint" in t}
    except (json.JSONDecodeError, OSError):
        return {}


def load_holder_snapshots(data_root: Path, date_str: str) -> dict[str, dict[str, Any]]:
    """Load holder metadata for a given date, indexed by mint."""
    snapshots_dir = data_root / "snapshots"
    if not snapshots_dir.exists():
        return {}

    result: dict[str, dict[str, Any]] = {}
    for mint_dir in snapshots_dir.iterdir():
        if not mint_dir.is_dir():
            continue

        metadata_path = mint_dir / f"metadata_{date_str}.json"
        holders_path = mint_dir / f"holders_{date_str}.json"

        if not metadata_path.exists():
            continue

        try:
            with open(metadata_path) as f:
                metadata = json.load(f)

            holder_count = 0
            if holders_path.exists():
                with open(holders_path) as f:
                    holders_data = json.load(f)
                if isinstance(holders_data, dict):
                    holder_count = len(holders_data)
                elif isinstance(holders_data, list):
                    holder_count = len(holders_data)

            mint = metadata.get("mint", "")
            if mint:
                result[mint] = {
                    **metadata,
                    "holder_count_actual": holder_count,
                    "has_holders": holder_count > 0,
                }
        except (json.JSONDecodeError, OSError):
            continue

    return result


def load_token_profiles(data_root: Path) -> dict[str, dict[str, Any]]:
    """Load enriched token profiles, indexed by mint."""
    path = data_root / "token_profiles.jsonl"
    if not path.exists():
        return {}

    result: dict[str, dict[str, Any]] = {}
    try:
        with open(path) as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                record = json.loads(line)
                mint = record.get("mint", "")
                if mint:
                    result[mint] = record
    except (json.JSONDecodeError, OSError):
        pass

    return result


def load_outcomes(data_root: Path, date_str: str) -> dict[str, list[dict[str, Any]]]:
    """Load outcome records for tokens judged on a given date, indexed by mint."""
    outcomes_dir = data_root / "outcomes"
    if not outcomes_dir.exists():
        return {}

    result: dict[str, list[dict[str, Any]]] = {}
    for f in outcomes_dir.glob("outcomes_*.jsonl"):
        try:
            with open(f) as fh:
                for line in fh:
                    line = line.strip()
                    if not line:
                        continue
                    record = json.loads(line)
                    if record.get("judgment_date") == date_str:
                        mint = record.get("mint", "")
                        if mint:
                            result.setdefault(mint, []).append(record)
        except (json.JSONDecodeError, OSError):
            continue

    return result


def load_trajectory(data_root: Path, mint_short: str, date_str: str) -> Optional[dict[str, Any]]:
    """Load trajectory classification for a mint on a given date."""
    path = data_root / "snapshots" / mint_short / f"trajectory_{date_str}.json"
    if not path.exists():
        return None
    try:
        with open(path) as f:
            return json.load(f)
    except (json.JSONDecodeError, OSError):
        return None


def get_available_dates(data_root: Path) -> list[str]:
    """Get all dates for which we have market snapshots."""
    snapshots_dir = data_root / "market_snapshots"
    if not snapshots_dir.exists():
        return []

    dates: list[str] = []
    for f in sorted(snapshots_dir.glob("market_snapshot_*.json")):
        date_str = f.stem.replace("market_snapshot_", "")
        dates.append(date_str)
    return dates


def unify_date(data_root: Path, date_str: str, output_dir: Path) -> int:
    """Unify all sources for a given date. Returns count of unified profiles."""
    market = load_market_snapshot(data_root, date_str)
    holders = load_holder_snapshots(data_root, date_str)
    profiles = load_token_profiles(data_root)
    outcomes = load_outcomes(data_root, date_str)

    # Collect all known mints for this date
    all_mints = set(market.keys()) | set(holders.keys())
    if not all_mints:
        print(f"  {date_str}: no data")
        return 0

    output_dir.mkdir(parents=True, exist_ok=True)
    out_path = output_dir / f"profiles_{date_str}.jsonl"

    count = 0
    with open(out_path, "w") as f:
        for mint in sorted(all_mints):
            m = market.get(mint, {})
            h = holders.get(mint, {})
            p = profiles.get(mint, {})
            o = outcomes.get(mint, [])

            # Find mint short key (first 16 chars) for trajectory lookup
            mint_short = mint[:16]
            # Try to find actual directory
            for d in (data_root / "snapshots").iterdir() if (data_root / "snapshots").exists() else []:
                if d.is_dir() and mint.startswith(d.name):
                    mint_short = d.name
                    break

            traj = load_trajectory(data_root, mint_short, date_str)

            unified = {
                "schema_version": SCHEMA_VERSION,
                "date": date_str,
                "unified_at": datetime.now(timezone.utc).isoformat(),
                # Identity
                "mint": mint,
                "symbol": m.get("symbol") or h.get("symbol") or p.get("symbol", "?"),
                "name": p.get("name", ""),
                # Market (from GeckoTerminal snapshot)
                "price_usd": m.get("price_usd") or m.get("price_today"),
                "volume_24h": m.get("volume_24h"),
                "market_cap_usd": m.get("market_cap_usd"),
                "fdv_usd": m.get("fdv_usd"),
                "reserve_usd": m.get("reserve_usd"),
                "pool_address": m.get("pool_address"),
                "pool_created_at": m.get("pool_created_at"),
                # Price changes (from market snapshot historical)
                "price_change_7d": m.get("price_change_7d"),
                "price_change_14d": m.get("price_change_14d"),
                "price_change_30d": m.get("price_change_30d"),
                # Holders (from Helius snapshot)
                "holder_count": h.get("holder_count") or h.get("holder_count_actual", 0),
                "has_holders": h.get("has_holders", False),
                # Conviction (from sovereign profiler / CultScreener)
                "conviction": m.get("conviction") or p.get("conviction_cultscreener"),
                "conviction_tier": m.get("conviction_tier") or p.get("conviction_tier"),
                # Token metadata (from enriched profile)
                "token_age_days": p.get("token_age_days"),
                "origin_pump_fun": p.get("origin_pump_fun"),
                "mint_authority_active": p.get("mint_authority_active"),
                "freeze_authority_active": p.get("freeze_authority_active"),
                # Flow (from enriched profile)
                "buy_sell_ratio": p.get("flow", {}).get("buy_sell_ratio") if isinstance(p.get("flow"), dict) else None,
                "unique_wallets": p.get("flow", {}).get("unique_wallets") if isinstance(p.get("flow"), dict) else None,
                # Trajectory (from daily classification)
                "trajectory_class": traj.get("class") if traj else None,
                "trajectory_conviction_delta": traj.get("conviction_delta") if traj else None,
                # Outcomes (from outcome collector — may be empty if not yet due)
                "outcomes": [
                    {
                        "window_days": oc.get("window_days"),
                        "price_change_pct": oc.get("price_change_pct"),
                        "survived": oc.get("survived"),
                    }
                    for oc in sorted(o, key=lambda x: x.get("window_days", 0))
                ] if o else [],
                # Sources present (for data quality tracking)
                "sources": {
                    "market": bool(m),
                    "holders": bool(h) and h.get("has_holders", False),
                    "profile": bool(p),
                    "trajectory": traj is not None,
                    "outcomes": bool(o),
                },
            }

            f.write(json.dumps(unified) + "\n")
            count += 1

    print(f"  {date_str}: {count} profiles -> {out_path}")
    return count


def cmd_status(data_root: Path) -> None:
    """Show available data sources and coverage."""
    dates = get_available_dates(data_root)
    print(f"Available dates: {len(dates)}")
    for d in dates:
        print(f"  {d}")

    # Count data sources
    profiles_path = data_root / "token_profiles.jsonl"
    profile_count = 0
    if profiles_path.exists():
        with open(profiles_path) as f:
            profile_count = sum(1 for line in f if line.strip())

    outcomes_dir = data_root / "outcomes"
    outcome_count = 0
    if outcomes_dir.exists():
        for f in outcomes_dir.glob("outcomes_*.jsonl"):
            with open(f) as fh:
                outcome_count += sum(1 for line in fh if line.strip())

    snapshot_dirs = list((data_root / "snapshots").iterdir()) if (data_root / "snapshots").exists() else []
    holder_dirs = [d for d in snapshot_dirs if d.is_dir()]

    print(f"\nData sources:")
    print(f"  Market snapshots: {len(dates)} dates")
    print(f"  Holder snapshots: {len(holder_dirs)} mints")
    print(f"  Token profiles: {profile_count} entries")
    print(f"  Outcomes: {outcome_count} records")

    # Check unified output
    unified_dir = data_root / "unified"
    if unified_dir.exists():
        unified_files = list(unified_dir.glob("profiles_*.jsonl"))
        print(f"  Unified profiles: {len(unified_files)} dates")
    else:
        print(f"  Unified profiles: 0 (directory not created yet)")


def main() -> int:
    parser = argparse.ArgumentParser(description="Unify all token-analysis sources")
    parser.add_argument("--date", type=str, help="Unify specific date (YYYY-MM-DD)")
    parser.add_argument("--all", action="store_true", help="Unify all available dates")
    parser.add_argument("--status", action="store_true", help="Show available data sources")
    args = parser.parse_args()

    data_root = find_data_root()
    output_dir = data_root / "unified"

    if args.status:
        cmd_status(data_root)
        return 0

    if args.all:
        dates = get_available_dates(data_root)
        if not dates:
            print("No market snapshots found.")
            return 0
        total = 0
        for d in dates:
            total += unify_date(data_root, d, output_dir)
        print(f"\nTotal: {total} profiles across {len(dates)} dates")
    else:
        date_str = args.date or datetime.now(timezone.utc).strftime("%Y-%m-%d")
        unify_date(data_root, date_str, output_dir)

    return 0


if __name__ == "__main__":
    sys.exit(main())
