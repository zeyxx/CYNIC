#!/usr/bin/env python3
# Tier 2 INFRASTRUCTURE: organ-telegram dispatch script.
"""Dispatch one organ-telegram messaging-surface task to the Hermes Agent executor."""

from __future__ import annotations

import json
import sys
from pathlib import Path

# Ensure the scripts directory is in sys.path
sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent.parent / \"scripts\"))
from hermes_dispatch import HermesDispatcher


def main() -> int:
    dispatcher = HermesDispatcher("organ-telegram")

    content = {
        "objective": "Organ Telegram runtime perception and config drift repair",
        "domain": "telegram",
        "targets": [
            "infra/systemd/telegram-listener.service",
            "cynic-python/organs/telegram/MANIFEST.yaml",
            "cynic-python/organs/telegram/config.py",
            "cynic-python/organs/telegram/listener.py",
        ],
        "actions": [
            "Compare Telegram runtime metadata against the repository state.",
            "Detect listener config drift, secret handling issues, or public/ops ambiguity.",
            "Apply the smallest repo change that restores reproducible Telegram deployment for organ-local files.",
            "Post a concise observation to /observe when Telegram runtime state changes.",
            "Do not mutate the shared runtime map or message datasets in the periodic task; surface shared-map drift for explicit follow-up.",
        ],
    }
    payload = {
        "kind": "hermes",
        "domain": "telegram",
        "agent_id": "organ-telegram-cron",
        "content": json.dumps(content, separators=(",", ":")),
    }

    return 0 if dispatcher.dispatch_task(payload) else 1


if __name__ == "__main__":
    raise SystemExit(main())
