#!/usr/bin/env python3
"""Dispatch one organ-discord surface task to the Hermes Agent executor."""
from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent.parent / "scripts"))
from hermes_dispatch import HermesDispatcher


def main() -> int:
    dispatcher = HermesDispatcher("organ-discord")
    content = {
        "objective": "Organ Discord runtime perception and community opportunity triage",
        "domain": "discord",
        "targets": [
            "organs/discord/MANIFEST.yaml",
            "organs/discord/src/config.py",
            "organs/discord/src/passive_proxy.py",
            "organs/discord/src/pipeline.py",
            "organs/discord/state/SKILL.md",
            "infra/systemd/discord-passive-proxy.service",
        ],
        "actions": [
            "Check Discord passive capture configuration drift and secret handling.",
            "Surface Superteam France bounty, grant, infra, and community signals.",
            "Keep passive ingestion scoped to explicit channel allowlists.",
            "Post structured observations to /observe when Discord runtime state changes.",
            "Do not mutate raw Discord datasets in repo tasks.",
        ],
    }
    payload = {
        "kind": "hermes",
        "domain": "discord",
        "agent_id": "organ-discord-cron",
        "content": json.dumps(content, separators=(",", ":")),
    }
    return 0 if dispatcher.dispatch_task(payload) else 1


if __name__ == "__main__":
    raise SystemExit(main())
