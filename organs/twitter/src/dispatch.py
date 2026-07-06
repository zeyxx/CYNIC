#!/usr/bin/env python3
# Tier 2 INFRASTRUCTURE: organ-twitter dispatch script.
"""Dispatch one organ-twitter task to the Hermes Agent executor."""

from __future__ import annotations

import json
from pathlib import Path
from organs.contracts.src.hermes_dispatch import HermesDispatcher
from organs.contracts.src.surface_contracts import read_json_file_or_default

PROJECT_DIR = Path(__file__).resolve().parent.parent
STATE_FILE = PROJECT_DIR / "state" / "state.json"

def get_state() -> dict | None:
    if not STATE_FILE.exists():
        return None
    try:
        state = read_json_file_or_default(STATE_FILE, {})
        return state if isinstance(state, dict) else None
    except Exception:
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
            "organs/twitter/state/context_items.jsonl",
            "organs/twitter/state/context/latest.md",
            "organs/twitter/state/SKILL.md"
        ],
        "actions": [
            "Read the Twitter surface state from state.json.",
            "If state/context/latest.md exists, use it as the cortex context bundle.",
            "If no bundle exists but context_items.jsonl has items, run organs/twitter/src/entrypoints/cli.py bundle.",
            "If Proxy is offline, run Analyst Mode over context_items.jsonl and historical dataset.jsonl.",
            "If Proxy is online, run Explorer Mode: check for high-signal tweets and curate them into context_items.jsonl.",
            "Update state/SKILL.md only with durable operating patterns and evidence.",
            "Post a concise JSON observation to /observe."
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
