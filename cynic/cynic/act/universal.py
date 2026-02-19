"""
CYNIC UniversalActuator — Strategy Registry (T33)

Pattern: Strategy Registry — every actuator type registers itself.
UniversalActuator dispatches ACT_REQUESTED payloads to the matching strategy.

Built-in strategies:
  BashActuator    — executes a command list via asyncio.create_subprocess_exec
  GitReadActuator — reads git status, log, diff (read-only, safe)

Architecture:
  ACT_REQUESTED -> UniversalActuator.dispatch(payload)
               -> _registry[action_type].execute(payload)
               -> returns ActResult(success, output, error)

Adding a strategy:
    ua = UniversalActuator()
    ua.register("my_type", MyActuator())

phi-safety: Each actuator output is capped at F(13)=233 chars (BURN axiom).
"""
from __future__ import annotations

import asyncio
import logging
import sys
import time
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional

from cynic.core.phi import fibonacci

logger = logging.getLogger("cynic.act.universal")

_MAX_OUTPUT = fibonacci(13)  # 233 chars


# ── ActResult ─────────────────────────────────────────────────────────────────

@dataclass
class ActResult:
    """Result of one actuator execution."""
    action_type: str
    success:     bool
    output:      str = ""
    error:       str = ""
    duration_ms: float = 0.0
    timestamp:   float = field(default_factory=time.time)

    def to_dict(self) -> dict[str, Any]:
        return {
            "action_type": self.action_type,
            "success":     self.success,
            "output":      self.output,
            "error":       self.error,
            "duration_ms": round(self.duration_ms, 1),
            "timestamp":   self.timestamp,
        }


# ── Base Actuator ─────────────────────────────────────────────────────────────

class BaseActuator:
    name: str = "base"

    async def execute(self, payload: dict[str, Any]) -> ActResult:
        raise NotImplementedError(f"{self.__class__.__name__}.execute() not implemented")


# ── BashActuator ──────────────────────────────────────────────────────────────

class BashActuator(BaseActuator):
    """
    Execute a command list safely (no shell injection).

    Payload keys:
        args:    List[str]  — command + arguments, e.g. ["git", "status"]
        cwd:     str        — working directory (optional)
        timeout: float      — seconds (default 30.0, max 60.0)

    Uses create_subprocess_exec (no shell=True) — injection-safe.
    """

    name = "bash"

    async def execute(self, payload: dict[str, Any]) -> ActResult:
        args: list[str] = payload.get("args") or []
        if not args:
            return ActResult("bash", success=False, error="Empty args list")

        cwd     = payload.get("cwd") or None
        timeout = min(float(payload.get("timeout", 30.0)), 60.0)

        t0 = time.perf_counter()
        try:
            proc = await asyncio.create_subprocess_exec(
                *[str(a) for a in args],
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
                cwd=cwd,
            )
            stdout, stderr = await asyncio.wait_for(
                proc.communicate(), timeout=timeout
            )
            duration_ms = (time.perf_counter() - t0) * 1000
            out = stdout.decode("utf-8", errors="replace")[:_MAX_OUTPUT] if stdout else ""
            err = stderr.decode("utf-8", errors="replace")[:_MAX_OUTPUT] if stderr else ""
            success = (proc.returncode == 0)
            logger.info(
                "BashActuator: rc=%d dur=%.0fms args=%s",
                proc.returncode, duration_ms, args[:3],
            )
            return ActResult("bash", success=success, output=out, error=err,
                             duration_ms=duration_ms)
        except TimeoutError:
            return ActResult("bash", success=False,
                             error=f"Timed out after {timeout}s",
                             duration_ms=(time.perf_counter() - t0) * 1000)
        except Exception as exc:
            return ActResult("bash", success=False, error=str(exc),
                             duration_ms=(time.perf_counter() - t0) * 1000)


# ── GitReadActuator ───────────────────────────────────────────────────────────

class GitReadActuator(BaseActuator):
    """
    Read-only git operations: status, log, diff, show, branch.

    Payload keys:
        subcommand: "status" | "log" | "diff" | "show" | "branch"
        cwd:        working directory (optional)
        args:       additional args (safe list, no write flags)
    """

    name = "git_read"

    ALLOWED_SUBCOMMANDS = frozenset({"status", "log", "diff", "show", "branch"})
    _BLOCKED_ARGS       = frozenset({"--force", "-f", "--hard", "--push", "--delete"})

    async def execute(self, payload: dict[str, Any]) -> ActResult:
        subcommand = (payload.get("subcommand") or "status").strip().lower()
        if subcommand not in self.ALLOWED_SUBCOMMANDS:
            return ActResult(
                "git_read", success=False,
                error=f"Subcommand not allowed: {subcommand!r}. "
                      f"Allowed: {sorted(self.ALLOWED_SUBCOMMANDS)}"
            )

        extra_args: list[str] = [str(a) for a in (payload.get("args") or [])]
        for arg in extra_args:
            if arg in self._BLOCKED_ARGS:
                return ActResult("git_read", success=False,
                                 error=f"Write arg blocked: {arg!r}")

        cwd = payload.get("cwd") or None
        cmd = ["git", subcommand] + extra_args

        t0 = time.perf_counter()
        try:
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
                cwd=cwd,
            )
            stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=15.0)
            duration_ms = (time.perf_counter() - t0) * 1000
            out = stdout.decode("utf-8", errors="replace")[:_MAX_OUTPUT] if stdout else ""
            err = stderr.decode("utf-8", errors="replace")[:_MAX_OUTPUT] if stderr else ""
            logger.info("GitReadActuator: git %s rc=%d", subcommand, proc.returncode)
            return ActResult("git_read", success=(proc.returncode == 0),
                             output=out, error=err, duration_ms=duration_ms)
        except Exception as exc:
            return ActResult("git_read", success=False, error=str(exc),
                             duration_ms=(time.perf_counter() - t0) * 1000)


# ── UniversalActuator ─────────────────────────────────────────────────────────

class UniversalActuator:
    """
    Strategy Registry: dispatches action payloads to registered actuators.

    Built-in strategies: "bash", "git_read".
    Register custom strategies via ua.register(name, actuator).
    """

    def __init__(self) -> None:
        self._registry: dict[str, BaseActuator] = {}
        self._results:  list[ActResult] = []
        self._max_history = fibonacci(11)  # 89

        self.register("bash",     BashActuator())
        self.register("git_read", GitReadActuator())

    def register(self, action_type: str, actuator: BaseActuator) -> None:
        self._registry[action_type] = actuator
        logger.info("UniversalActuator: registered strategy '%s'", action_type)

    def strategies(self) -> list[str]:
        return list(self._registry.keys())

    async def dispatch(self, payload: dict[str, Any]) -> ActResult:
        action_type = (payload.get("action_type") or "bash").lower()
        actuator = self._registry.get(action_type)
        if actuator is None:
            return ActResult(
                action_type, success=False,
                error=f"No strategy for action_type={action_type!r}. "
                      f"Available: {self.strategies()}",
            )
        result = await actuator.execute(payload)
        self._results.append(result)
        if len(self._results) > self._max_history:
            self._results = self._results[-self._max_history:]
        return result

    def recent_results(self, n: int = 10) -> list[dict[str, Any]]:
        return [r.to_dict() for r in self._results[-n:]]

    def stats(self) -> dict[str, Any]:
        total   = len(self._results)
        success = sum(1 for r in self._results if r.success)
        return {
            "strategies":     self.strategies(),
            "total_executed": total,
            "success_count":  success,
            "error_count":    total - success,
        }
