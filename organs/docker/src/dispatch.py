#!/usr/bin/env python3
# Tier 2 INFRASTRUCTURE: organ-docker dispatch script.
"""Dispatch one organ-docker local-container task to the Hermes Agent executor."""

import json
import sys
from src.adapters.hermes_adapter import DefaultHermesAdapter

def main() -> int:
    adapter = DefaultHermesAdapter("docker")

    content = {
        "objective": "Organ Docker measurement-first audit of the local container surface",
        "domain": "docker",
        "targets": [
            "infra/docker/cynic-portal/docker-compose.yml",
            "infra/docker/cynic-portal/Dockerfile",
            "infra/systemd/cynic-portal.service",
        ],
        "actions": [
            "Measure the current container runtime against the documented local fallback surface.",
            "Report the exact drift, if any, with the smallest file-level explanation possible.",
            "Only propose a repo change if the drift is confined to the declared targets and can be fixed without widening scope.",
            "Post a concise observation to /observe with the measured state, even if no code change is needed.",
            "Do not touch datasets or curation artifacts.",
        ],
    }
    payload = {
        "kind": "hermes",
        "domain": "docker",
        "agent_id": "organ-docker-cron",
        "content": json.dumps(content, separators=(",", ":")),
    }

    return 0 if adapter.dispatch_task(payload) else 1

if __name__ == "__main__":
    sys.exit(main())
