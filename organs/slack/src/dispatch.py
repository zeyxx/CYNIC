#!/usr/bin/env python3
"""
organ-slack — Dispatch
Hexagonal entrypoint for the Slack organ.

Lifecycle:
  1. Poll SurrealDB mempool for observations tagged domain=slack or with a
     target channel ID (routable via registry.json).
  2. Route each observation to the correct Slack channel via router.py.
  3. Send via sender.py (the only Slack HTTP boundary).
  4. POST /observe back to the Kernel with status=delivered|failed.

Can also be called directly (CLI mode) to send a one-shot message:
  python dispatch.py --route stage_privacy_ethereum --text "Hello Caplogy"
"""

from __future__ import annotations

import argparse
import json
import logging
import os
import sys
import urllib.request
import urllib.error
from pathlib import Path
from datetime import datetime, timezone

# Allow running from any working directory
sys.path.insert(0, str(Path(__file__).resolve().parents[4] / "scripts"))

from .router import resolve_channel, list_routes
from .sender import send, format_observation

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [organ-slack] %(levelname)s %(message)s",
    datefmt="%Y-%m-%dT%H:%M:%SZ",
)
log = logging.getLogger("organ-slack")

KERNEL_REST_ADDR = os.environ.get("CYNIC_REST_ADDR", "http://localhost:3030")
CYNIC_API_KEY = os.environ.get("CYNIC_API_KEY", "")


# ── Kernel observe callback ──────────────────────────────────────────────────

def _kernel_observe(context: str, status: str, domain: str = "slack") -> None:
    """Fire-and-forget observation back to the Kernel."""
    payload = json.dumps({
        "tool": "organ-slack",
        "domain": domain,
        "context": context,
        "status": status,
        "tags": ["organ-slack", "relay"],
    }).encode()
    req = urllib.request.Request(
        f"{KERNEL_REST_ADDR}/observe",
        data=payload,
        headers={
            "Authorization": f"Bearer {CYNIC_API_KEY}",
            "Content-Type": "application/json",
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=5):
            pass
    except Exception as exc:
        log.warning("Kernel observe failed (non-blocking): %s", exc)


# ── Core dispatch logic ──────────────────────────────────────────────────────

def dispatch_observation(observation: dict) -> bool:
    """
    Route and send one observation dict to Slack.
    Returns True on success, False on failure.
    """
    routing_key = observation.get("domain", "kernel_ops")
    channel_id = observation.get("target", "")

    # If target is already a raw channel ID (starts with C), use it directly
    if not (channel_id.startswith("C") and len(channel_id) > 8):
        try:
            channel_id = resolve_channel(routing_key)
        except RuntimeError as exc:
            log.error("Routing failed: %s", exc)
            return False

    text, blocks = format_observation(observation)

    try:
        send(channel_id, text, blocks)
        log.info("Delivered to %s (route=%s)", channel_id, routing_key)
        _kernel_observe(
            context=f"Delivered: {text[:120]}",
            status="delivered",
            domain=routing_key,
        )
        return True
    except RuntimeError as exc:
        log.error("Send failed: %s", exc)
        _kernel_observe(
            context=f"Failed: {exc}",
            status="failed",
            domain=routing_key,
        )
        return False


def dispatch_text(routing_key: str, text: str) -> bool:
    """Convenience wrapper: route + send plain text."""
    return dispatch_observation({
        "domain": routing_key,
        "context": text,
        "tool": "cli",
        "project": "cynic",
    })


# ── CLI ──────────────────────────────────────────────────────────────────────

def _cli() -> int:
    parser = argparse.ArgumentParser(description="organ-slack CLI")
    sub = parser.add_subparsers(dest="cmd")

    # send command
    send_p = sub.add_parser("send", help="Send a message to a routed channel")
    send_p.add_argument("--route", default="kernel_ops",
                        help="Routing key from registry.json (default: kernel_ops)")
    send_p.add_argument("--text", required=True, help="Message text")

    # routes command (diagnostics)
    sub.add_parser("routes", help="Print resolved routing table")

    args = parser.parse_args()

    if args.cmd == "send":
        ok = dispatch_text(args.route, args.text)
        return 0 if ok else 1

    if args.cmd == "routes":
        routes = list_routes()
        for key, channel in routes.items():
            print(f"  {key:<30} → {channel}")
        return 0

    parser.print_help()
    return 1


if __name__ == "__main__":
    raise SystemExit(_cli())
