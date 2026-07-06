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
import logging
import os

import requests

from organs.contracts.src.adapters.kernel_observe import post_kernel_observe
from .router import resolve_coordination_channel, list_routes
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
    payload = {
        "tool": "organ-slack",
        "domain": domain,
        "context": context,
        "status": status,
        "tags": ["organ-slack", "relay"],
    }
    result = post_kernel_observe(
        request_post=requests.post,
        kernel_url=KERNEL_REST_ADDR,
        auth_key=CYNIC_API_KEY,
        payload=payload,
        timeout=5,
    )
    if not result.posted:
        log.warning("Kernel observe failed (non-blocking): %s", result.error or result.status_code)


# ── Core dispatch logic ──────────────────────────────────────────────────────

def dispatch_observation(observation: dict) -> bool:
    """
    Route and send one observation dict to Slack.
    Returns True on success, False on failure.
    """
    routing_key = observation.get("domain", "kernel_ops")
    channel_id = observation.get("target", "")
    payload = observation.get("value")

    # If target is already a raw channel ID (starts with C), use it directly.
    if not (isinstance(channel_id, str) and channel_id.startswith("C") and len(channel_id) > 8):
        channel_id = ""

    thread_id = None
    if isinstance(payload, dict):
        thread_id = payload.get("thread_id")
        thread_ts = payload.get("thread_ts")
    else:
        thread_ts = None

    try:
        if not channel_id:
            # Coordination payloads have project/workstream context in value.coordination.
            project = ""
            workstream = None
            if isinstance(payload, dict):
                project = payload.get("project", "")
                workstream = payload.get("workstream") or payload.get("thread", None)
                if isinstance(workstream, str) and not workstream:
                    workstream = None
            channel_id = resolve_coordination_channel(
                project=project or routing_key,
                workstream=workstream,
                thread_id=thread_id if isinstance(thread_id, str) else None,
                fallback_key=routing_key,
            )
        if not isinstance(payload, dict):
            payload = {}
    except RuntimeError as exc:
        log.error("Routing failed: %s", exc)
        return False

    text, blocks = format_observation(observation)
    thread_ts_opt = None
    if isinstance(thread_ts, str):
        thread_ts_opt = thread_ts

    try:
        send(channel_id, text, blocks, thread_ts=thread_ts_opt)
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
