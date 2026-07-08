"""MCP stdio client for the canonical fleet_node_exec tool."""

from __future__ import annotations

import json
import os
import select
import shlex
import subprocess
import time
from pathlib import Path
from typing import Any

from organs.contracts.src.surface_contracts import json_loads_or_default


class McpFleetNodeExecClient:
    def __init__(
        self,
        argv: list[str],
        cwd: str | Path,
        popen=subprocess.Popen,
    ):
        if not argv:
            raise RuntimeError("MCP stdio command argv missing")
        self.argv = argv
        self.cwd = str(cwd)
        self.popen = popen

    @classmethod
    def from_env(cls, project_root: str | Path) -> "McpFleetNodeExecClient":
        command = os.environ.get("CYNIC_MCP_STDIO_COMMAND", "").strip()
        if command:
            argv = shlex.split(command)
        else:
            argv = ["python3", ".cortex/mcp/run_cynic_mcp.py"]
        return cls(argv, project_root)

    def call_fleet_node_exec(self, request: dict) -> dict:
        node = request.get("node")
        command = request.get("command")
        if not isinstance(node, str) or not node:
            raise RuntimeError("fleet_node_exec wrapper request node missing")
        if not isinstance(command, str) or not command:
            raise RuntimeError("fleet_node_exec wrapper request command missing")
        timeout_seconds = int(request.get("timeout_seconds", 30))
        purpose = request.get("purpose") or "organ-forge lifecycle bootstrap"
        arguments = {
            "node": node,
            "command": command,
            "purpose": purpose,
            "timeout_seconds": timeout_seconds,
            "agent_id": request.get("agent_id", "organ-forge-fleet-node-exec-wrapper"),
        }
        return self._call_tool("fleet_node_exec", arguments, timeout_seconds + 10)

    def _call_tool(self, name: str, arguments: dict[str, Any], timeout_seconds: int) -> dict:
        process = None
        try:
            process = self.popen(
                self.argv,
                cwd=self.cwd,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                bufsize=1,
            )

            def send(payload: dict) -> None:
                if process is None or process.stdin is None:
                    raise RuntimeError("MCP process stdin unavailable")
                process.stdin.write(json.dumps(payload, separators=(",", ":")) + "\n")
                process.stdin.flush()

            def receive(timeout: float) -> dict | None:
                if process is None or process.stdout is None:
                    raise RuntimeError("MCP process stdout unavailable")
                ready, _, _ = select.select([process.stdout], [], [], timeout)
                if not ready:
                    return None
                payload = json_loads_or_default(process.stdout.readline(), None)
                return payload if isinstance(payload, dict) else None

            send(
                {
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {},
                        "clientInfo": {
                            "name": "organ-forge-fleet-node-exec-wrapper",
                            "version": "0.1",
                        },
                    },
                }
            )
            initialized = receive(5)
            if not initialized or initialized.get("error"):
                return self._blocked("mcp_initialize_failed", initialized, process)
            send({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}})
            send(
                {
                    "jsonrpc": "2.0",
                    "id": 2,
                    "method": "tools/call",
                    "params": {"name": name, "arguments": arguments},
                }
            )
            response = receive(float(timeout_seconds))
            if not response:
                return self._blocked("mcp_tool_timeout", None, process)
            if response.get("error"):
                return self._blocked("mcp_tool_error", response, process)
            return self._extract_tool_payload(response, process)
        except subprocess.TimeoutExpired as exc:
            return {"status": "blocked", "ok": False, "error": f"MCP timeout: {exc}"}
        except Exception as exc:
            return {"status": "blocked", "ok": False, "error": str(exc)}
        finally:
            if process is not None:
                try:
                    process.terminate()
                    process.wait(timeout=2)
                except Exception:
                    try:
                        process.kill()
                    except Exception:
                        pass

    def _extract_tool_payload(self, response: dict, process) -> dict:
        content = response.get("result", {}).get("content", [])
        if not content:
            return self._blocked("mcp_tool_missing_content", response, process)
        text = content[0].get("text", "{}")
        payload = json_loads_or_default(text, {})
        if not isinstance(payload, dict):
            return self._blocked("mcp_tool_non_object_payload", response, process)
        status = payload.get("status")
        return {
            **payload,
            "ok": status == "ok",
            "transport": payload.get("transport", "mcp_stdio_fleet_node_exec"),
        }

    def _drain_available_text(self, stream, *, max_wait_seconds: float = 0.05, max_lines: int = 32) -> str:
        if stream is None:
            return ""
        chunks: list[str] = []
        deadline = time.monotonic() + max_wait_seconds
        while len(chunks) < max_lines and time.monotonic() < deadline:
            try:
                ready, _, _ = select.select([stream], [], [], max(0.0, deadline - time.monotonic()))
            except Exception:
                break
            if not ready:
                break
            line = stream.readline()
            if not line:
                break
            chunks.append(line)
        return "".join(chunks)

    def _blocked(self, error: str, response: dict | None, process) -> dict:
        stderr = ""
        if process is not None and process.stderr is not None:
            try:
                stderr = self._drain_available_text(process.stderr)
            except Exception:
                stderr = ""
        return {
            "status": "blocked",
            "ok": False,
            "error": error,
            "response": response,
            "stderr": stderr,
            "transport": "mcp_stdio_fleet_node_exec",
        }
