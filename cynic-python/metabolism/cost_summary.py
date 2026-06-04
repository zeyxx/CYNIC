#!/usr/bin/env python3
"""
cost_summary — Human-readable metabolic cost report.

Tier 2 INFRASTRUCTURE: CLI report over cost_ledger.jsonl.

K15 Consumer: human-routed alert — surfaces top token/sovereign consumers
Systemd: none (CLI, invoked manually or by agents)
Promotion date: 2026-06-02

Usage:
    python3 cost_summary.py
    python3 cost_summary.py --since 24h
    python3 cost_summary.py --session cortex-abc123
    python3 cost_summary.py --feature spike_detector --since 7d
"""
from __future__ import annotations

import argparse
import json
import sys as _sys
import os as _os
from collections import defaultdict
from datetime import datetime, timedelta, timezone
from pathlib import Path

_sys.path.insert(0, str(Path(__file__).parent.parent))
from metabolism.cost_tracker import percentile as _pct


def parse_since(since: str) -> datetime:
    now = datetime.now(timezone.utc)
    if since.endswith("h"):
        return now - timedelta(hours=float(since[:-1]))
    if since.endswith("d"):
        return now - timedelta(days=float(since[:-1]))
    raise ValueError(f"Unknown since format: {since!r} (use e.g. 24h or 7d)")


def load_events(
    ledger: Path,
    since: datetime | None = None,
    session: str | None = None,
    feature: str | None = None,
) -> list[dict]:
    if not ledger.exists():
        return []
    events = []
    with open(ledger) as f:
        for line in f:
            try:
                e = json.loads(line)
            except json.JSONDecodeError:
                continue
            if since:
                ts_str = e.get("ts", "")
                if ts_str:
                    try:
                        ts = datetime.fromisoformat(ts_str.replace("Z", "+00:00"))
                        if ts < since:
                            continue
                    except ValueError:
                        pass
            if session and e.get("session_id") != session:
                continue
            if feature and e.get("feature_id") != feature:
                continue
            events.append(e)
    return events


def print_summary(events: list[dict], label: str) -> None:
    if not events:
        print(f"{label}: no events found")
        return

    tokens_in = sum(e.get("tokens_in", 0) for e in events)
    tokens_out = sum(e.get("tokens_out", 0) for e in events)
    sovereign: dict[str, int] = defaultdict(int)
    feature_tokens: dict[str, int] = defaultdict(int)
    feature_sovereign: dict[str, int] = defaultdict(int)
    latencies = [e["latency_ms"] for e in events if e.get("latency_ms", 0) > 0]

    for e in events:
        fid = e.get("feature_id", "unknown")
        feature_tokens[fid] += e.get("tokens_in", 0) + e.get("tokens_out", 0)
        if e.get("compute_class") == "external_api":
            sovereign[e.get("provider", "unknown")] += 1
            feature_sovereign[fid] += 1

    latencies.sort()
    p50 = _pct(latencies, 0.50)
    p99 = _pct(latencies, 0.99)

    total_sovereign = sum(sovereign.values())
    total_tokens = tokens_in + tokens_out

    print(f"\n{label}")
    print(f"  Events   : {len(events)}")
    print(f"  Tokens   : in={tokens_in:,}  out={tokens_out:,}  total={total_tokens:,}")
    if sovereign:
        parts = "  ".join(f"{k}×{v}" for k, v in sorted(sovereign.items(), key=lambda x: -x[1]))
        print(f"  Sovereign: {parts}")
    print(f"  Latency  : P50={p50}ms  P99={p99}ms")

    if total_tokens > 0:
        top_token_fid = max(feature_tokens, key=lambda k: feature_tokens[k])
        pct = int(feature_tokens[top_token_fid] / total_tokens * 100)
        print(f"  Top token cost : {top_token_fid} {pct}% of tokens")
    if total_sovereign > 0:
        top_sov_fid = max(feature_sovereign, key=lambda k: feature_sovereign[k])
        pct = int(feature_sovereign[top_sov_fid] / total_sovereign * 100)
        print(f"  Top sovereign  : {top_sov_fid} {pct}% of external calls")
    print()


def main() -> None:
    parser = argparse.ArgumentParser(description="CYNIC metabolic cost summary")
    parser.add_argument("--since", default="24h", help="Time window (e.g. 24h, 7d)")
    parser.add_argument("--session", help="Filter by session_id")
    parser.add_argument("--feature", help="Filter by feature_id")
    parser.add_argument(
        "--ledger",
        default=str(Path.home() / ".cynic" / "metabolism" / "cost_ledger.jsonl"),
    )
    args = parser.parse_args()

    since_dt = parse_since(args.since)
    ledger = Path(args.ledger)
    events = load_events(ledger, since=since_dt, session=args.session, feature=args.feature)

    label_parts = [f"last {args.since}"]
    if args.session:
        label_parts.append(f"session={args.session}")
    if args.feature:
        label_parts.append(f"feature={args.feature}")
    print_summary(events, "Cost summary — " + ", ".join(label_parts))


if __name__ == "__main__":
    main()
