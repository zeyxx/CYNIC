#!/usr/bin/env python3
# Tier 2 INFRASTRUCTURE: organ-twitter dispatch script.
"""Dispatch one organ-twitter task to the Hermes Agent executor."""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent.parent / "scripts"))
from hermes_dispatch import HermesDispatcher

PROJECT_DIR = Path(__file__).resolve().parent.parent
STATE_FILE = PROJECT_DIR / "state" / "state.json"

def get_state() -> dict | None:
    if not STATE_FILE.exists():
        return None
    try:
        with open(STATE_FILE, "r") as f:
            return json.load(f)
    except json.JSONDecodeError:
        return None

def main() -> int:
    dispatcher = HermesDispatcher("organ-twitter")
    key = dispatcher.kernel_key()
    if not key:
        print("organ-twitter dispatch skipped: kernel API key missing", file=sys.stderr)
        return 1

    state = get_state()
    if not state:
        print("organ-twitter dispatch skipped: no state available", file=sys.stderr)
        return 0

    content = {
        "objective": "Perform Twitter Organ Lifecycle loop (Explorer/Analyst).",
        "domain": "organ-twitter",
        "targets": [
            "organs/twitter/state/state.json",
            "organs/twitter/state/dataset.jsonl",
            "organs/twitter/SKILL.md"
        ],
        "actions": [
            "Read the Twitter surface state from state.json.",
            "If Proxy is offline, run in Analyst Mode: analyze historical dataset.jsonl and find patterns.",
            "If Proxy is online, run in Explorer Mode: check for high-signal tweets and curate them.",
            "Update SKILL.md with new patterns and evidence.",
            "Post a concise insight to /observe."
        ],
    }
    
    payload = {
        "kind": "hermes",
        "domain": "organ-twitter",
        "agent_id": "organ-twitter-cron",
        "content": json.dumps(content, separators=(",", ":")),
    }
    
    if not dispatcher.dispatch_task(payload):
        return 1

    return 0

if __name__ == "__main__":
    raise SystemExit(main())
