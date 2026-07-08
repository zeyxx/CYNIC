"""Fleet node execution transport for organ-forge bootstrap."""

from __future__ import annotations

import json
import os
import shlex
import subprocess
from collections.abc import Callable


class FleetNodeExecCommandTransport:
    """Execute fleet_node_exec requests through a configured JSON wrapper command."""

    def __init__(
        self,
        argv: list[str],
        runner: Callable[..., subprocess.CompletedProcess] = subprocess.run,
    ):
        if not argv:
            raise RuntimeError("fleet node exec command argv missing")
        self.argv = argv
        self.runner = runner

    @classmethod
    def from_env(cls) -> "FleetNodeExecCommandTransport | None":
        command = os.environ.get("CYNIC_FLEET_NODE_EXEC_COMMAND", "").strip()
        if not command:
            return None
        return cls(shlex.split(command))

    def execute(
        self,
        *,
        node: str,
        command: str,
        timeout_seconds: int,
    ) -> dict:
        request = {
            "tool": "fleet_node_exec",
            "node": node,
            "command": command,
            "timeout_seconds": timeout_seconds,
            "purpose": "organ-forge lifecycle bootstrap",
        }
        completed = self.runner(
            self.argv,
            input=json.dumps(request, separators=(",", ":")),
            capture_output=True,
            text=True,
            timeout=timeout_seconds + 5,
            check=False,
        )
        if completed.returncode != 0:
            return {
                "status": "blocked",
                "ok": False,
                "transport": "fleet_node_exec_command",
                "returncode": completed.returncode,
                "stdout": completed.stdout,
                "stderr": completed.stderr,
            }
        try:
            payload = json.loads(completed.stdout or "{}")
        except json.JSONDecodeError as exc:
            return {
                "status": "blocked",
                "ok": False,
                "transport": "fleet_node_exec_command",
                "error": f"fleet node exec command returned non-JSON stdout: {exc}",
                "stdout": completed.stdout,
                "stderr": completed.stderr,
            }
        status = payload.get("status")
        return {
            **payload,
            "ok": status == "ok",
            "transport": payload.get("transport", "fleet_node_exec_command"),
        }
