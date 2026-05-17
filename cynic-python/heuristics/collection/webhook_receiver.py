#!/usr/bin/env python3
"""Helius Webhook Receiver — persists all token events + tracks credit cost.

Minimal HTTP server that:
1. Receives enhanced webhook POSTs from Helius
2. Persists each event to JSONL (one file per mint per day)
3. Tracks credit consumption in real-time (events/second/hour/day)
4. Exposes /metrics for cost monitoring

Data structure:
  webhook_data/
    {mint_short}/
      events_{YYYY-MM-DD}.jsonl     — raw events (append-only)
    _metrics/
      credits_{YYYY-MM-DD}.json     — daily credit tracking

Cost model: 1 credit per event received.

Usage:
    python3 webhook_receiver.py                  # start on port 3031
    python3 webhook_receiver.py --port 8090      # custom port
    python3 webhook_receiver.py --metrics        # show current cost metrics

Systemd: cynic-webhook-receiver.service
"""

import json
import os
import sys
import time
import threading
from datetime import datetime, timezone
from http.server import HTTPServer, BaseHTTPRequestHandler
from typing import Dict

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
DATA_DIR = os.path.join(SCRIPT_DIR, "webhook_data")
METRICS_DIR = os.path.join(DATA_DIR, "_metrics")

# ── CREDIT TRACKER ──

class CreditTracker:
    """Real-time credit consumption tracker."""

    def __init__(self):
        self._lock = threading.Lock()
        self._events_total = 0       # per-mint event count (for data analysis)
        self._deliveries_total = 0   # webhook deliveries = actual Helius credits
        self._deliveries_today = 0
        self._deliveries_this_hour = 0
        self._events_today = 0
        self._events_this_hour = 0
        self._events_this_minute = 0
        self._current_day = ""
        self._current_hour = -1
        self._current_minute = -1
        self._start_time = time.time()
        self._per_mint: Dict[str, int] = {}
        self._history: Dict[str, int] = {}  # date → events

        # Load existing metrics
        self._load_history()

    def _load_history(self) -> None:
        if not os.path.exists(METRICS_DIR):
            return
        for f in os.listdir(METRICS_DIR):
            if f.startswith("credits_") and f.endswith(".json"):
                date_str = f.replace("credits_", "").replace(".json", "")
                path = os.path.join(METRICS_DIR, f)
                try:
                    with open(path) as fh:
                        data = json.load(fh)
                        self._history[date_str] = data.get("events", 0)
                        self._events_total += data.get("events", 0)
                except (json.JSONDecodeError, IOError):
                    pass

    def record_delivery(self, num_txs: int = 1) -> None:
        """Record a webhook delivery (= 1 Helius credit per tx in the POST)."""
        with self._lock:
            self._deliveries_total += num_txs
            self._deliveries_today += num_txs
            self._deliveries_this_hour += num_txs

    def record_event(self, mint: str) -> None:
        """Record a per-mint event (for data analysis, NOT credit counting)."""
        now = datetime.now(timezone.utc)
        day = now.strftime("%Y-%m-%d")
        hour = now.hour
        minute = now.minute

        with self._lock:
            if day != self._current_day:
                self._save_daily_metrics()
                self._events_today = 0
                self._deliveries_today = 0
                self._current_day = day
            if hour != self._current_hour:
                self._events_this_hour = 0
                self._deliveries_this_hour = 0
                self._current_hour = hour
            if minute != self._current_minute:
                self._events_this_minute = 0
                self._current_minute = minute

            self._events_total += 1
            self._events_today += 1
            self._events_this_hour += 1
            self._events_this_minute += 1
            self._per_mint[mint] = self._per_mint.get(mint, 0) + 1

    def _save_daily_metrics(self) -> None:
        if not self._current_day or self._events_today == 0:
            return
        os.makedirs(METRICS_DIR, exist_ok=True)
        path = os.path.join(METRICS_DIR, f"credits_{self._current_day}.json")
        data = {
            "date": self._current_day,
            "events": self._events_today,
            "credits": self._events_today,  # 1:1 ratio
            "per_mint": dict(self._per_mint),
        }
        with open(path, "w") as f:
            json.dump(data, f, indent=2)

    def get_metrics(self) -> Dict:
        uptime_s = time.time() - self._start_time
        uptime_h = uptime_s / 3600

        with self._lock:
            # Credits = deliveries (what Helius actually charges)
            if uptime_h > 0:
                projected_daily = (self._deliveries_today / max(uptime_h, 0.01)) * 24
            else:
                projected_daily = 0

            projected_monthly = projected_daily * 30
            budget_pct = (projected_monthly / 10_000_000) * 100

            rate_per_s = self._deliveries_this_hour / 3600 if self._deliveries_this_hour > 0 else 0

            return {
                # CREDITS = deliveries (1 per webhook POST per tx)
                "credits_total": self._deliveries_total,
                "credits_today": self._deliveries_today,
                "credits_this_hour": self._deliveries_this_hour,
                "rate_per_second": round(rate_per_s, 2),
                "projected_daily": int(projected_daily),
                "projected_monthly": int(projected_monthly),
                "budget_pct_monthly": round(budget_pct, 3),
                # DATA = events persisted (for analysis, not billing)
                "events_total": self._events_total,
                "events_today": self._events_today,
                "events_this_hour": self._events_this_hour,
                # META
                "uptime_hours": round(uptime_h, 2),
                "mints_tracked": len(self._per_mint),
                "top_mints": sorted(self._per_mint.items(), key=lambda x: -x[1])[:5],
                "history_days": len(self._history),
            }

    def flush(self) -> None:
        with self._lock:
            self._save_daily_metrics()


tracker = CreditTracker()


# ── EVENT PERSISTENCE ──

def persist_event(event: Dict) -> None:
    """Persist a single webhook event to JSONL."""
    # Extract mint from tokenTransfers or accountData
    mints_seen = set()

    for xfer in event.get("tokenTransfers", []):
        mint = xfer.get("mint")
        if mint:
            mints_seen.add(mint)

    if not mints_seen:
        # Fallback: check accountData
        for acct in event.get("accountData", []):
            if acct.get("tokenBalanceChanges"):
                for tbc in acct["tokenBalanceChanges"]:
                    mint = tbc.get("mint")
                    if mint:
                        mints_seen.add(mint)

    today = datetime.now(timezone.utc).strftime("%Y-%m-%d")

    # Compact event: only keep what we need for conviction calculation
    compact = {
        "ts": event.get("timestamp", int(time.time())),
        "sig": event.get("signature", "")[:20],  # truncate for space
        "type": event.get("type", "UNKNOWN"),
        "transfers": [
            {
                "mint": x.get("mint", "")[:16],
                "from": x.get("fromUserAccount", ""),
                "to": x.get("toUserAccount", ""),
                "amount": x.get("tokenAmount", 0),
            }
            for x in event.get("tokenTransfers", [])
        ],
    }

    for mint in mints_seen:
        mint_short = mint[:16]
        mint_dir = os.path.join(DATA_DIR, mint_short)
        os.makedirs(mint_dir, exist_ok=True)

        events_path = os.path.join(mint_dir, f"events_{today}.jsonl")
        with open(events_path, "a") as f:
            f.write(json.dumps(compact) + "\n")

        tracker.record_event(mint_short)


# ── HTTP SERVER ──

class WebhookHandler(BaseHTTPRequestHandler):
    def do_POST(self):
        if self.path == "/webhook/helius":
            content_length = int(self.headers.get("Content-Length", 0))
            body = self.rfile.read(content_length)

            try:
                # Helius sends array of transactions (1 credit per tx)
                events = json.loads(body)
                if isinstance(events, list):
                    tracker.record_delivery(len(events))
                    for event in events:
                        persist_event(event)
                elif isinstance(events, dict):
                    tracker.record_delivery(1)
                    persist_event(events)

                self.send_response(200)
                self.end_headers()
                self.wfile.write(b"OK")
            except json.JSONDecodeError:
                self.send_response(400)
                self.end_headers()
                self.wfile.write(b"Invalid JSON")
        else:
            self.send_response(404)
            self.end_headers()

    def do_GET(self):
        if self.path == "/metrics":
            metrics = tracker.get_metrics()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps(metrics, indent=2).encode())
        elif self.path == "/health":
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b"OK")
        else:
            self.send_response(404)
            self.end_headers()

    def log_message(self, format, *args):
        # Suppress default logging (too noisy)
        pass


def show_metrics() -> None:
    """Show current cost metrics from stored data."""
    metrics = tracker.get_metrics()
    print("=" * 60)
    print("WEBHOOK CREDIT METRICS")
    print("=" * 60)
    print(f"  Credits total:      {metrics['credits_total']:>10,}")
    print(f"  Credits today:      {metrics['credits_today']:>10,}")
    print(f"  Credits this hour:  {metrics['credits_this_hour']:>10,}")
    print(f"  Rate:               {metrics['rate_per_second']:.2f} events/s")
    print(f"  Projected daily:    {metrics['projected_daily']:>10,} cr")
    print(f"  Projected monthly:  {metrics['projected_monthly']:>10,} cr")
    print(f"  Budget usage:       {metrics['budget_pct_monthly']:.3f}% of 10M/month")
    print(f"  Mints tracked:      {metrics['mints_tracked']}")
    print(f"  History:            {metrics['history_days']} days")
    if metrics['top_mints']:
        print(f"\n  Top mints by events:")
        for mint, count in metrics['top_mints']:
            print(f"    {mint}: {count:,}")


def main() -> None:
    if "--metrics" in sys.argv:
        show_metrics()
        return

    port = 3031
    if "--port" in sys.argv:
        idx = sys.argv.index("--port")
        if idx + 1 < len(sys.argv):
            port = int(sys.argv[idx + 1])

    os.makedirs(DATA_DIR, exist_ok=True)

    server = HTTPServer(("0.0.0.0", port), WebhookHandler)
    print(f"Webhook receiver listening on :{port}")
    print(f"  POST /webhook/helius  — receive events")
    print(f"  GET  /metrics         — credit consumption")
    print(f"  GET  /health          — liveness check")
    print(f"\nData: {DATA_DIR}/")
    print(f"Ctrl+C to stop\n")

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nShutting down...")
        tracker.flush()
        server.shutdown()


if __name__ == "__main__":
    main()
