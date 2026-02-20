"""
ClaudeCodeRunner — CYNIC spawns Claude Code autonomously.

CYNIC is the BRAIN. Claude Code is the HANDS.
CYNIC launches `claude --sdk-url ws://localhost:PORT/ws/sdk` as a subprocess,
waits for it to connect via SDK_SESSION_STARTED, sends the task, waits for
SDK_RESULT_RECEIVED, and returns the result.

No human needed. CYNIC does it entirely.

This closes the PERCEIVE → JUDGE → DECIDE → ACT loop.

Security note: subprocess is launched with a hardcoded argument list (no shell=True,
no string interpolation of untrusted input). This is equivalent to execFile, not exec.
The sdk_url is derived from our own port constant, not from user-provided data.
"""
from __future__ import annotations

import asyncio
import json
import logging
import subprocess
import uuid
from typing import Any, Optional


from cynic.core.event_bus import CoreEvent

logger = logging.getLogger("cynic.metabolism.runner")

# Timeout constants (seconds)
CONNECT_TIMEOUT = 30.0          # wait for claude to call /ws/sdk
DEFAULT_TASK_TIMEOUT = 300.0    # wait for result message
SDK_PATH = "/ws/sdk"

# Claude Code CLI command — hardcoded, no shell interpolation
_CLAUDE_BIN = "claude"
_CLAUDE_FLAGS = ["--print", "--output-format", "stream-json", "--input-format", "stream-json"]

# Default model for CYNIC's own autonomous tasks.
# Haiku: fast, cheap, sufficient for known-pattern tasks (debug/refactor/test).
# Override per-call via execute(model="claude-sonnet-4-6") for complex analysis.
MODEL_HAIKU = "claude-haiku-4-5-20251001"
MODEL_DEFAULT = MODEL_HAIKU


def _build_claude_cmd(sdk_url: str, resume_session_id: str | None = None) -> list:
    """
    Build the claude CLI command as a list (safe — no shell=True).
    sdk_url is always 'ws://localhost:{port}/ws/sdk' — controlled by CYNIC, not user input.
    resume_session_id: Claude's internal session ID from a previous system/init message.
    """
    cmd = [_CLAUDE_BIN, "--sdk-url", sdk_url] + _CLAUDE_FLAGS
    if resume_session_id:
        cmd += ["--resume", resume_session_id]
    return cmd


class ClaudeCodeRunner:
    """
    Autonomous Claude Code subprocess launcher.

    When CYNIC decides to ACT, it spawns claude in SDK mode pointing to its
    own WebSocket server. Claude executes the task (with every tool call
    judged by GUARDIAN), and CYNIC collects the result for learning.

    Design:
    - Register bus listeners BEFORE spawning (prevents race condition)
    - Graceful degradation if claude binary not found
    - Subprocess terminated in finally block on every exit path
    - Two separate timeouts: connect (CONNECT_TIMEOUT) and task timeout
    """

    def __init__(self, bus, sessions_registry: dict, port: int = 8765):
        self._bus = bus
        self._sessions = sessions_registry
        self._port = port
        self._running: dict[str, Any] = {}  # exec_id → process handle

    async def execute(
        self,
        prompt: str,
        cwd: str | None = None,
        model: str | None = MODEL_DEFAULT,
        timeout: float = DEFAULT_TASK_TIMEOUT,
        resume_session_id: str | None = None,
    ) -> dict[str, Any]:
        """
        Execute a task via Claude Code --sdk-url.

        Returns on success:
            {"success": True, "session_id": str, "cli_session_id": str,
             "cost_usd": float, "total_cost_usd": float, "exec_id": str}

        Returns on failure:
            {"success": False, "error": str, "exec_id": str}
        """
        exec_id = str(uuid.uuid4())[:8]
        sdk_url = f"ws://localhost:{self._port}{SDK_PATH}"

        # ── Shared state across inner handlers ─────────────────────────────
        session_ready = asyncio.Event()
        loop = asyncio.get_running_loop()
        result_future: asyncio.Future = loop.create_future()
        target_session_id: str | None = None

        # ── Register listeners BEFORE spawning ─────────────────────────────
        async def _on_session_started(event) -> None:
            nonlocal target_session_id
            if not session_ready.is_set():
                target_session_id = event.payload.get("session_id")
                session_ready.set()

        async def _on_result_received(event) -> None:
            if (event.payload.get("session_id") == target_session_id
                    and not result_future.done()):
                result_future.set_result(event.payload)

        self._bus.on(CoreEvent.SDK_SESSION_STARTED, _on_session_started)
        self._bus.on(CoreEvent.SDK_RESULT_RECEIVED, _on_result_received)

        proc = None
        try:
            # ── Launch Claude Code in headless SDK mode ─────────────────────
            # Safe: list form, no shell=True, url is always localhost:{port}
            cmd = _build_claude_cmd(sdk_url, resume_session_id=resume_session_id)
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                cwd=cwd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            self._running[exec_id] = proc
            logger.info(
                "*ears perk* Spawned claude --sdk-url (pid=%d exec=%s)",
                proc.pid, exec_id,
            )

            # ── Wait for session to connect ─────────────────────────────────
            await asyncio.wait_for(session_ready.wait(), timeout=CONNECT_TIMEOUT)

            session = self._sessions.get(target_session_id)
            if session is None:
                raise RuntimeError(
                    f"Session {target_session_id} not in registry after connect"
                )

            # Tag session with task prompt for telemetry (runner path)
            if hasattr(session, "_task_prompt"):
                session._task_prompt = prompt

            # ── Optional model routing ──────────────────────────────────────
            if model and model != session.model:
                set_model_msg = {
                    "type": "control_response",
                    "response": {
                        "subtype": "success",
                        "request_id": str(uuid.uuid4()),
                        "response": {"subtype": "set_model", "model": model},
                    },
                }
                await session.ws.send_text(json.dumps(set_model_msg) + "\n")
                logger.debug("*sniff* Model routed → %s (exec=%s)", model, exec_id)

            # ── Send the task ───────────────────────────────────────────────
            task_msg = {
                "type": "user",
                "message": {"role": "user", "content": prompt},
                "parent_tool_use_id": None,
                "session_id": target_session_id,
            }
            await session.ws.send_text(json.dumps(task_msg) + "\n")
            logger.info(
                "*tail wag* Task sent (exec=%s): %s...",
                exec_id, prompt[:80],
            )

            # ── Wait for result ─────────────────────────────────────────────
            payload = await asyncio.wait_for(result_future, timeout=timeout)
            # cli_session_id from session registry (captured at system/init)
            completed_session = self._sessions.get(target_session_id)
            cli_sid = getattr(completed_session, "cli_session_id", "") if completed_session else ""
            return {
                "success": not payload.get("is_error", False),
                "session_id": target_session_id,
                "cli_session_id": cli_sid,
                "cost_usd": payload.get("cost_usd", 0.0),
                "total_cost_usd": payload.get("total_cost_usd", 0.0),
                "exec_id": exec_id,
            }

        except FileNotFoundError:
            logger.warning(
                "*head tilt* claude binary not found — install Claude Code CLI"
            )
            return {
                "success": False,
                "error": "claude binary not found",
                "exec_id": exec_id,
            }

        except TimeoutError:
            logger.error("*GROWL* Claude Code timed out (exec=%s)", exec_id)
            return {"success": False, "error": "timeout", "exec_id": exec_id}

        except Exception as exc:
            logger.error("*GROWL* Runner error (exec=%s): %s", exec_id, exc)
            return {"success": False, "error": str(exc), "exec_id": exec_id}

        finally:
            self._bus.off(CoreEvent.SDK_SESSION_STARTED, _on_session_started)
            self._bus.off(CoreEvent.SDK_RESULT_RECEIVED, _on_result_received)
            # Terminate subprocess if still running
            if proc is not None and proc.returncode is None:
                try:
                    proc.terminate()
                    await asyncio.wait_for(proc.wait(), timeout=5.0)
                except Exception:
                    try:
                        proc.kill()
                    except Exception:
                        pass
            self._running.pop(exec_id, None)

    def stats(self) -> dict[str, Any]:
        return {
            "running_processes": len(self._running),
            "sdk_port": self._port,
            "sdk_path": SDK_PATH,
        }

    async def shutdown(self) -> None:
        """Terminate all running claude processes on kernel shutdown."""
        for exec_id, proc in list(self._running.items()):
            if hasattr(proc, "returncode") and proc.returncode is None:
                try:
                    proc.terminate()
                    logger.info("*yawn* Terminated claude (exec=%s)", exec_id)
                except Exception:
                    pass
        self._running.clear()
