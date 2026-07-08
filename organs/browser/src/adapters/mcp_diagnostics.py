"""Diagnostic adapters for organ-browser MCP readiness."""

from __future__ import annotations

import json
import select
import subprocess
from pathlib import Path

from organs.contracts.src.surface_contracts import json_loads_or_default


def first_active_user_unit(root: Path, units: list[str]) -> str | None:
    """Return the first active user systemd unit from a candidate list."""
    for unit in units:
        try:
            proc = subprocess.run(
                ["systemctl", "--user", "is-active", unit],
                cwd=root,
                text=True,
                capture_output=True,
                timeout=3,
                check=False,
            )
        except Exception:
            continue
        if proc.returncode == 0:
            return unit
    return None


def check_mcp_protocol(root: Path, wrapper: Path) -> bool:
    """Probe a stdio MCP wrapper with initialize and tools/list."""
    process = None
    try:
        process = subprocess.Popen(
            [str(wrapper)],
            cwd=root,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

        def send(payload: dict) -> None:
            assert process.stdin
            process.stdin.write(json.dumps(payload) + "\n")
            process.stdin.flush()

        def receive(timeout: float = 3.0) -> dict | None:
            assert process.stdout
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
                    "clientInfo": {"name": "diagnose-browser-mcp", "version": "0.1"},
                },
            }
        )
        initialized = receive()
        send({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}})
        send({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})
        tools = receive()
        names = {
            item.get("name")
            for item in (tools or {}).get("result", {}).get("tools", [])
        }
        return bool((initialized or {}).get("result")) and "browser_status" in names
    except Exception:
        return False
    finally:
        if process is not None:
            try:
                process.terminate()
                process.wait(timeout=1)
            except Exception:
                try:
                    process.kill()
                except Exception:
                    pass
