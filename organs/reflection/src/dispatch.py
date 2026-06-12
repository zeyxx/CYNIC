#!/usr/bin/env python3
# Tier 2 INFRASTRUCTURE: organ-reflection dispatch script.
"""Dispatch organ-reflection tasks to the Hermes Agent executor for Askesis."""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent.parent / "scripts"))
from hermes_dispatch import HermesDispatcher

PROJECT_DIR = Path(__file__).resolve().parent.parent
ORGANISM_STATE_FILE = PROJECT_DIR / "state" / "organism_state.json"
HUMAN_STATE_FILE = PROJECT_DIR / "state" / "human_state.json"

def get_poh(file_path: Path) -> dict | None:
    if not file_path.exists():
        return None
    try:
        with open(file_path, "r") as f:
            return json.load(f)
    except json.JSONDecodeError:
        return None

def dispatch_organism_askesis(dispatcher: HermesDispatcher) -> bool:
    poh = get_poh(ORGANISM_STATE_FILE)
    if not poh or not poh.get("recent_events"):
        return True # Nothing to do

    content = {
        "objective": "Perform Organism Askesis (Cognitive Reflection) on the Organism Proof-of-History.",
        "domain": "organ-reflection-organism",
        "targets": [
            "organs/reflection/state/organism_state.json",
            "organs/reflection/state/organism_askesis.jsonl",
            "docs/SOUL.md"
        ],
        "actions": [
            "Read the unified Organism PoH from organism_state.json.",
            "Analyze the events to detect behavioral patterns, repeated failures, or successes of the CYNIC agents.",
            "Write a cognitive reflection and append it to organism_askesis.jsonl.",
            "If a profound pattern is detected, propose an update to docs/SOUL.md.",
            "Post a concise insight to /observe."
        ],
    }
    
    payload = {
        "kind": "hermes",
        "domain": "organ-reflection",
        "agent_id": "organ-reflection-cron-organism",
        "content": json.dumps(content, separators=(",", ":")),
    }
    return dispatcher.dispatch_task(payload)

def dispatch_human_askesis(dispatcher: HermesDispatcher) -> bool:
    poh = get_poh(HUMAN_STATE_FILE)
    if not poh or not poh.get("recent_events"):
        return True # Nothing to do

    content = {
        "objective": "Perform Human Askesis (Cognitive Reflection) on the Human Proof-of-History.",
        "domain": "organ-reflection-human",
        "targets": [
            "organs/reflection/state/human_state.json",
            "organs/reflection/state/human_askesis.jsonl",
            "docs/HUMAN_PROFILE.md"
        ],
        "actions": [
            "Read the Human PoH from human_state.json.",
            "Analyze the human's actions, commands, and chat logs to deduce their intent, mood, and long-term goals.",
            "Write a cognitive reflection and append it to human_askesis.jsonl.",
            "If a major shift in user behavior is detected, update docs/HUMAN_PROFILE.md.",
            "Post a concise insight to /observe."
        ],
    }
    
    payload = {
        "kind": "hermes",
        "domain": "organ-reflection",
        "agent_id": "organ-reflection-cron-human",
        "content": json.dumps(content, separators=(",", ":")),
    }
    return dispatcher.dispatch_task(payload)


def main() -> int:
    dispatcher = HermesDispatcher("organ-reflection")
    key = dispatcher.kernel_key()
    if not key:
        print("organ-reflection dispatch skipped: kernel API key missing", file=sys.stderr)
        return 1

    ok1 = dispatch_organism_askesis(dispatcher)
    ok2 = dispatch_human_askesis(dispatcher)
    
    return 0 if (ok1 and ok2) else 1

if __name__ == "__main__":
    raise SystemExit(main())
