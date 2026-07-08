#!/usr/bin/env python3
"""JSON wrapper for organ-forge fleet_node_exec transport."""

from __future__ import annotations

import json
import sys
from pathlib import Path

PROJECT_ROOT_CANDIDATE = Path(__file__).resolve().parents[4]
if str(PROJECT_ROOT_CANDIDATE) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT_CANDIDATE))

from organs.contracts.src.project_root import resolve_repo_root
from organs.contracts.src.surface_contracts import json_object_or_raise
from organs.forge.src.adapters.mcp_fleet_node_exec_client import McpFleetNodeExecClient


def run(raw_stdin: str) -> dict:
    request = json_object_or_raise(
        raw_stdin,
        RuntimeError("fleet_node_exec wrapper stdin must be a JSON object"),
    )
    project_root = resolve_repo_root(PROJECT_ROOT_CANDIDATE)
    client = McpFleetNodeExecClient.from_env(project_root)
    return client.call_fleet_node_exec(request)


def main() -> int:
    try:
        result = run(sys.stdin.read())
        print(json.dumps(result, separators=(",", ":")))
        return 0 if result.get("status") == "ok" else 1
    except Exception as exc:
        print(
            json.dumps(
                {
                    "status": "blocked",
                    "ok": False,
                    "error": str(exc),
                    "transport": "mcp_stdio_fleet_node_exec",
                },
                separators=(",", ":"),
            )
        )
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
