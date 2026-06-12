#!/usr/bin/env python3
# Tier 2 INFRASTRUCTURE: organ-vercel dispatch script.
"""Dispatch one organ-vercel deployment-surface task to the Hermes Agent executor."""

import json
import sys
from src.adapters.hermes_adapter import DefaultHermesAdapter

def main() -> int:
    adapter = DefaultHermesAdapter("vercel")

    content = {
        "objective": "Organ Vercel measurement-first audit of the public edge surface",
        "domain": "vercel",
        "targets": [
            "packages/cynic-ui/vercel.json",
            "packages/talaria-demo/vercel.json",
            "packages/talaria-landing/vercel.json",
            "infra/talaria/runtime-surfaces.json",
        ],
        "actions": [
            "Measure the public edge configuration against the recorded runtime surfaces.",
            "Report the exact drift, if any, with the smallest file-level explanation possible.",
            "Only propose a repo change if the drift is confined to the declared targets and can be fixed without widening scope.",
            "Post a concise observation to /observe with the measured state, even if no code change is needed.",
            "Do not touch datasets or curation artifacts.",
        ],
    }
    payload = {
        "kind": "hermes",
        "domain": "vercel",
        "agent_id": "organ-vercel-cron",
        "content": json.dumps(content, separators=(",", ":")),
    }

    return 0 if adapter.dispatch_task(payload) else 1

if __name__ == "__main__":
    sys.exit(main())
