#!/usr/bin/env python3
# Tier 2 INFRASTRUCTURE: organ-x dispatch script.
"""Dispatch one organ-x X-surface task to the Hermes Agent executor."""

from __future__ import annotations

import json
import sys
from pathlib import Path

# Ensure the scripts directory is in sys.path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from hermes_dispatch import HermesDispatcher


def main() -> int:
    dispatcher = HermesDispatcher("organ-x")

    content = {
        "objective": "Organ X runtime perception and drift repair",
        "domain": "hermes-x",
        "targets": [
            "infra/systemd/hermes-data-organism.service",
            "infra/systemd/hermes-data-organism.timer",
            "scripts/hermes_data_organism.py",
        ],
        "actions": [
            "Compare the X surface metadata against the repository and runtime state.",
            "Detect drift in ingest, curation, or runtime ownership.",
            "Apply the smallest repo change that keeps the X surface reproducible for organ-local files.",
            "Post a concise observation to /observe when X runtime state changes.",
            "Do not mutate the shared runtime map or curation artifacts in the periodic task; surface shared-map drift for explicit follow-up.",
        ],
    }
    payload = {
        "kind": "hermes",
        "domain": "hermes-x",
        "agent_id": "organ-x-cron",
        "content": json.dumps(content, separators=(",", ":")),
    }

    return 0 if dispatcher.dispatch_task(payload) else 1


if __name__ == "__main__":
    raise SystemExit(main())
