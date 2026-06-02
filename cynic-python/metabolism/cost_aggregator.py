"""
Tier 2 INFRASTRUCTURE: CYNIC Metabolic Cost Aggregator.

Reads cost_ledger.jsonl (cursor-based), fetches kernel /verdicts for token counts,
POSTs aggregated summary to kernel /observe domain=metabolism every 30min.

K15: cost_ledger (producer) → cost_aggregator (consumer) → kernel /observe (acts: updates /health)
Falsification: /observations?domain=metabolism must have entries within 30min of deploy.
"""
from __future__ import annotations

import json
import logging
import os
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any

log = logging.getLogger(__name__)

_raw_addr = os.environ.get("CYNIC_REST_ADDR", "")
KERNEL_ADDR = f"http://{_raw_addr}" if _raw_addr and not _raw_addr.startswith("http") else _raw_addr
KERNEL_KEY = os.environ.get("CYNIC_API_KEY", "")

LEDGER_PATH = Path(
    os.environ.get("CYNIC_COST_LEDGER",
                   str(Path.home() / ".cynic" / "metabolism" / "cost_ledger.jsonl"))
)
CURSOR_PATH = Path(str(LEDGER_PATH).replace(".jsonl", "_cursor.txt"))


def read_new_events(ledger_path: str, cursor: int) -> tuple[list[dict], int]:
    """Read events from ledger starting at byte cursor. Returns (events, new_cursor)."""
    path = Path(ledger_path)
    if not path.exists():
        return [], cursor
    events = []
    with open(path, "rb") as f:
        f.seek(cursor)
        while True:
            line = f.readline()
            if not line:
                break
            try:
                events.append(json.loads(line.decode()))
            except json.JSONDecodeError:
                pass
        new_cursor = f.tell()
    return events, new_cursor


def fetch_kernel_tokens(limit: int = 200) -> list[dict[str, Any]]:
    """Fetch recent verdicts from kernel, extract per-dog token counts."""
    if not KERNEL_ADDR or not KERNEL_KEY:
        return []
    results = []
    try:
        req = urllib.request.Request(
            f"{KERNEL_ADDR}/verdicts?limit={limit}",
            headers={"Authorization": f"Bearer {KERNEL_KEY}"},
        )
        with urllib.request.urlopen(req, timeout=10) as resp:
            verdicts = json.loads(resp.read().decode())
        if not isinstance(verdicts, list):
            return []
        for v in verdicts:
            for ds in v.get("dog_scores", []):
                pt = ds.get("prompt_tokens", 0)
                ct = ds.get("completion_tokens", 0)
                if pt > 0 or ct > 0:
                    results.append({
                        "provider": ds.get("dog_id", "unknown"),
                        "tokens_in": pt,
                        "tokens_out": ct,
                    })
    except Exception as exc:
        log.warning("fetch_kernel_tokens failed: %s", exc)
    return results


def compute_summary(
    events: list[dict],
    kernel_tokens: list[dict[str, Any]],
) -> dict[str, Any]:
    """Aggregate events + kernel tokens into a summary dict."""
    tokens_in = sum(e.get("tokens_in", 0) for e in events)
    tokens_out = sum(e.get("tokens_out", 0) for e in events)

    for kt in kernel_tokens:
        tokens_in += kt.get("tokens_in", 0)
        tokens_out += kt.get("tokens_out", 0)

    sovereign_calls: dict[str, int] = {}
    latencies: list[int] = []

    for e in events:
        if e.get("compute_class") == "external_api":
            provider = e.get("provider", "unknown")
            sovereign_calls[provider] = sovereign_calls.get(provider, 0) + 1
        lat = e.get("latency_ms", 0)
        if lat > 0:
            latencies.append(lat)

    latencies_sorted = sorted(latencies)
    n = len(latencies_sorted)
    p50 = latencies_sorted[n // 2] if n > 0 else 0
    import math as _math
    p99 = latencies_sorted[min(_math.ceil(n * 0.99) - 1, n - 1)] if n > 0 else 0

    return {
        "tokens_in": tokens_in,
        "tokens_out": tokens_out,
        "sovereign_calls": sovereign_calls,
        "latency_p50_ms": p50,
        "latency_p99_ms": p99,
        "event_count": len(events),
    }


def format_context(summary: dict[str, Any]) -> str:
    """Format summary as kernel context string."""
    sovereign_str = " ".join(
        f"{k}:{v}" for k, v in sorted(summary["sovereign_calls"].items())
    )
    return (
        f"tokens_in:{summary['tokens_in']} "
        f"tokens_out:{summary['tokens_out']} "
        f"sovereign_calls:{sum(summary['sovereign_calls'].values())} "
        f"[{sovereign_str}] "
        f"p50_latency_ms:{summary['latency_p50_ms']} "
        f"p99_latency_ms:{summary['latency_p99_ms']} "
        f"events:{summary['event_count']}"
    )


def post_to_kernel(summary: dict[str, Any]) -> bool:
    """POST aggregated summary to kernel /observe domain=metabolism."""
    if not KERNEL_ADDR or not KERNEL_KEY:
        log.warning("CYNIC_REST_ADDR/CYNIC_API_KEY not set — kernel post skipped")
        return False
    body = json.dumps({
        "tool": "cost_aggregator",
        "domain": "metabolism",
        "context": format_context(summary),
        "agent_id": "cost-aggregator",
        "tags": ["cost-flush"],
    }).encode()
    req = urllib.request.Request(
        f"{KERNEL_ADDR}/observe", data=body,
        headers={"Authorization": f"Bearer {KERNEL_KEY}", "Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            log.info("cost flush posted: HTTP %d — %s", resp.status, format_context(summary))
            return True
    except Exception as exc:
        log.warning("cost flush POST failed: %s", exc)
        return False


def run_flush() -> None:
    """Main flush cycle: read new events, aggregate, post to kernel, advance cursor."""
    ledger = str(LEDGER_PATH)
    cursor = int(CURSOR_PATH.read_text().strip()) if CURSOR_PATH.exists() else 0

    events, new_cursor = read_new_events(ledger, cursor)
    kernel_tokens = fetch_kernel_tokens()

    if not events and not kernel_tokens:
        log.info("cost_aggregator: nothing to flush")
        return

    summary = compute_summary(events, kernel_tokens)
    if post_to_kernel(summary):
        CURSOR_PATH.write_text(str(new_cursor))
        log.info("cost_aggregator: flushed %d events, cursor → %d", len(events), new_cursor)
    else:
        log.warning("cost_aggregator: flush failed — cursor not advanced")


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s: %(message)s")
    run_flush()
