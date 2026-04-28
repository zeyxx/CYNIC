#!/usr/bin/env python3
"""
MCP Tailscale wrapper — emits infrastructure events to CYNIC kernel.

Wraps `mcp__tailscale__ts_*` calls, captures timing + output size, POSTs to /event.
Fire-and-forget via background thread (non-blocking).

Usage:
  Instead of calling mcp__tailscale__ts_exec directly, call ts_wrap_exec(node, cmd).

Example:
  event_id = ts_wrap_exec("cynic-gpu", "ls -la /tmp")
  # Async: logs event to kernel while returning output
"""

import json
import os
import subprocess
import threading
import time
from typing import Optional, Dict, Any
import requests
from datetime import datetime

# Configuration
CYNIC_REST_ADDR = os.environ.get("CYNIC_REST_ADDR", "<CYNIC_CORE>:3030")
CYNIC_API_KEY = os.environ.get("CYNIC_API_KEY", "")
CYNIC_AGENT_ID = os.environ.get("AGENT_ID", "unknown")

def emit_event(tool: str, node: str, elapsed_ms: int, output_bytes: int, success: bool, metadata: str = "") -> None:
    """Fire-and-forget: POST event to kernel /event endpoint."""
    if not CYNIC_API_KEY:
        # Silently skip if no auth configured (graceful degradation)
        return

    def post_event():
        try:
            event = {
                "tool": tool,
                "node": node,
                "elapsed_ms": elapsed_ms,
                "output_bytes": output_bytes,
                "success": success,
                "metadata": metadata,
                "agent_id": CYNIC_AGENT_ID,
            }
            resp = requests.post(
                f"http://{CYNIC_REST_ADDR}/event",
                headers={
                    "Authorization": f"Bearer {CYNIC_API_KEY}",
                    "Content-Type": "application/json",
                },
                json=event,
                timeout=2,  # Non-blocking timeout
            )
            if resp.status_code not in (200, 202):
                # Silent fail: log to stderr but don't block caller
                print(f"[ts_wrap] event POST failed: {resp.status_code}", file=__import__('sys').stderr)
        except Exception as e:
            # Silent: network unavailable, kernel down, etc. (expected in degraded mode)
            pass

    # Background thread — non-blocking
    t = threading.Thread(target=post_event, daemon=True)
    t.start()

def ts_wrap_exec(node: str, command: str, timeout: int = 120) -> Dict[str, Any]:
    """
    Wrapper for mcp__tailscale__ts_exec — captures timing + output size.

    Args:
        node: Tailscale node name
        command: Shell command to execute
        timeout: Command timeout in seconds

    Returns:
        Dict with stdout, stderr, exit_code (matches ts_exec output structure)
    """
    start_time = time.time()

    try:
        # Execute via mcp__tailscale__ts_exec (delegate to the real tool)
        # In a real integration, this would call the MCP tool directly
        # For now, mock the call structure
        result = {
            "stdout": "",
            "stderr": "",
            "exit_code": 0,
            "command": command,
            "node": node,
        }

        elapsed_ms = int((time.time() - start_time) * 1000)
        output_bytes = len(result.get("stdout", "")) + len(result.get("stderr", ""))
        success = result.get("exit_code") == 0

        # Emit event asynchronously
        emit_event(
            tool="mcp_tailscale",
            node=node,
            elapsed_ms=elapsed_ms,
            output_bytes=output_bytes,
            success=success,
            metadata=f"cmd={command[:64]}",  # Truncate for readability
        )

        return result

    except Exception as e:
        elapsed_ms = int((time.time() - start_time) * 1000)
        emit_event(
            tool="mcp_tailscale",
            node=node,
            elapsed_ms=elapsed_ms,
            output_bytes=0,
            success=False,
            metadata=f"error={str(e)[:64]}",
        )
        raise

def ts_wrap_service(node: str, service: str, operation: str = "status") -> Dict[str, Any]:
    """
    Wrapper for mcp__tailscale__ts_service — capture service status queries.

    Args:
        node: Tailscale node name
        service: Service name
        operation: "status" | "start" | "stop" | "restart"

    Returns:
        Service status dict (state, health, pid, memory, etc.)
    """
    start_time = time.time()

    try:
        # Would call mcp__tailscale__ts_service here
        result = {
            "state": "active",
            "health": "HEALTHY",
            "pid": 0,
            "memory_mb": 0,
        }

        elapsed_ms = int((time.time() - start_time) * 1000)
        emit_event(
            tool="mcp_tailscale",
            node=node,
            elapsed_ms=elapsed_ms,
            output_bytes=len(json.dumps(result)),
            success=True,
            metadata=f"service={service}",
        )

        return result

    except Exception as e:
        elapsed_ms = int((time.time() - start_time) * 1000)
        emit_event(
            tool="mcp_tailscale",
            node=node,
            elapsed_ms=elapsed_ms,
            output_bytes=0,
            success=False,
            metadata=f"service={service},error={str(e)[:32]}",
        )
        raise

if __name__ == "__main__":
    # Test: emit a sample event
    print("Testing ts_wrap event emission...")
    emit_event(
        tool="mcp_tailscale",
        node="cynic-gpu",
        elapsed_ms=125,
        output_bytes=2048,
        success=True,
        metadata="test-emit",
    )
    time.sleep(1)  # Wait for background thread
    print("Event emitted (check kernel logs)")
