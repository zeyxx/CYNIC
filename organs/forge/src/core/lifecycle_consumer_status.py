"""Lifecycle mempool consumer status helpers for organ-forge."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
from pathlib import Path
from typing import Any


def _tasks(payload: dict | None) -> list[dict]:
    tasks = (payload or {}).get("tasks", [])
    return tasks if isinstance(tasks, list) else []


def _task_content(task: dict) -> dict:
    raw = task.get("content", "{}")
    if isinstance(raw, dict):
        return raw
    if not isinstance(raw, str):
        return {}
    try:
        parsed = json.loads(raw)
    except json.JSONDecodeError:
        return {}
    return parsed if isinstance(parsed, dict) else {}


def _intent_counts(tasks: list[dict]) -> dict[str, int]:
    counts: dict[str, int] = {}
    for task in tasks:
        content = _task_content(task)
        key = content.get("intent") or content.get("action", {}).get("action") or "unknown"
        counts[str(key)] = counts.get(str(key), 0) + 1
    return dict(sorted(counts.items()))


def build_lifecycle_consumer_status_report(
    *,
    pending_payload: dict,
    completed_payload: dict | None,
    kind: str,
    domain: str,
    consumer_agent_id: str = "organ-forge-controller",
    runtime_probe: dict[str, Any] | None = None,
) -> dict[str, Any]:
    pending = _tasks(pending_payload)
    completed = _tasks(completed_payload)
    unclaimed = [task for task in pending if not task.get("claimed_by")]
    claimed = [task for task in pending if task.get("claimed_by")]
    ages = [
        int(task.get("age_seconds", 0))
        for task in pending
        if isinstance(task.get("age_seconds", 0), int)
    ]
    oldest_age = max(ages) if ages else 0
    corrected_mcp_reload = [
        task.get("id")
        for task in pending
        if _task_content(task).get("request_profile") == "mcp-reload"
        or _task_content(task).get("intent") == "build-and-reload-mcp"
    ]
    status = "ok"
    consumer_state = "idle"
    next_action = "No pending lifecycle tasks for organ-forge."
    probe = runtime_probe or {}
    runtime_ready = probe.get("status") == "ok"
    if pending and unclaimed:
        status = "degraded"
        consumer_state = "not_claiming"
        next_action = (
            "Start or repair the organ-forge mempool consumer on the registry build runner; "
            "pending organ-forge tasks are not being claimed."
        )
    elif pending:
        consumer_state = "working"
        next_action = "Wait for claimed tasks to complete, then verify completed read-back."
    elif not runtime_ready:
        status = "degraded"
        consumer_state = "runtime_unavailable"
        next_action = probe.get(
            "operator_action",
            "Restore the cynic runtime user session and build toolchain before relying on organ-forge.",
        )

    return {
        "schema": "cynic.lifecycle_consumer_status.v1",
        "status": status,
        "kind": kind,
        "domain": domain,
        "consumer_agent_id": consumer_agent_id,
        "consumer_state": consumer_state,
        "pending_count": len(pending),
        "completed_sample_count": len(completed),
        "claimed_count": len(claimed),
        "unclaimed_count": len(unclaimed),
        "oldest_pending_age_seconds": oldest_age,
        "intent_counts": _intent_counts(pending),
        "mcp_reload_pending_task_ids": [
            task_id for task_id in corrected_mcp_reload if isinstance(task_id, str)
        ],
        "sample_pending_task_ids": [
            task.get("id") for task in pending[:10] if isinstance(task.get("id"), str)
        ],
        "next_action": next_action,
        "runtime_probe": probe,
        "runtime_notes": {
            "expected_systemd_unit": "organ-forge-hermes-agent.service",
            "expected_controller": "organs.forge.controllers.lifecycle_agent.LifecycleAgentController",
            "intent_executor_status": (
                "LifecycleAgentController polls organ-forge plus configured lifecycle domains, "
                "executes build-and-reload-mcp with MCP freshness read-back, and executes "
                "classify-release-matter-for-sovereign-forgejo-proposal as a deterministic "
                "Forgejo proposal classification. Unknown lifecycle intents remain structured "
                "blockers; intent tasks must not be re-dispatched through BuildDelegationService."
            ),
        },
    }


def probe_local_consumer_runtime(
    *,
    runtime_user: str = "cynic",
    runtime_home: str | None = None,
) -> dict[str, Any]:
    home = Path(runtime_home or Path.home()).expanduser()
    loginctl = _run_command(["loginctl", "list-users"])
    loginctl_stdout = loginctl.get("stdout", "")
    user_active = False
    for line in loginctl_stdout.splitlines()[1:]:
        columns = line.split()
        if len(columns) >= 2 and columns[1] == runtime_user:
            user_active = True
            break

    daemon_probe = _probe_manual_consumer_process(runtime_home=home)
    cargo_path = _resolve_runtime_cargo(home)
    cargo_available = cargo_path is not None
    env_file = home / ".config" / "cynic" / "env"
    systemd_unit = home / ".config" / "systemd" / "user" / "hermes-agent-executor.service"
    problems: list[str] = []
    if not user_active and daemon_probe.get("running") is not True:
        problems.append("runtime_user_not_active")
    if not cargo_available:
        problems.append("cargo_missing")
    if not env_file.exists():
        problems.append("runtime_env_missing")
    if not systemd_unit.exists():
        problems.append("runtime_unit_missing")

    if not problems:
        status = "ok"
        operator_action = "Runtime consumer session and build toolchain appear present."
    else:
        status = "blocked"
        operator_action = (
            "Activate the cynic user runtime, ensure hermes-agent-executor.service is installed, "
            "and install the Rust cargo toolchain in the runtime home."
        )

    return {
        "schema": "cynic.lifecycle_consumer_runtime_probe.v1",
        "status": status,
        "runtime_user": runtime_user,
        "runtime_home": str(home),
        "runtime_user_active": user_active,
        "daemon_probe": daemon_probe,
        "cargo_available": cargo_available,
        "cargo_path": cargo_path,
        "runtime_env_exists": env_file.exists(),
        "runtime_unit_exists": systemd_unit.exists(),
        "loginctl": loginctl,
        "problems": problems,
        "operator_action": operator_action,
    }


def _run_command(command: list[str]) -> dict[str, Any]:
    try:
        completed = subprocess.run(
            command,
            capture_output=True,
            text=True,
            timeout=15,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        return {
            "command": command,
            "returncode": None,
            "stdout": getattr(exc, "stdout", "") or "",
            "stderr": str(exc),
        }
    return {
        "command": command,
        "returncode": completed.returncode,
        "stdout": completed.stdout,
        "stderr": completed.stderr,
    }


def _resolve_runtime_cargo(home: Path) -> str | None:
    path_bin = shutil.which("cargo")
    if path_bin:
        return path_bin
    for candidate in (
        home / ".cargo" / "bin" / "cargo",
        home / ".local" / "bin" / "cargo",
        home / "bin" / "cargo",
    ):
        if candidate.exists() and os.access(candidate, os.X_OK):
            return str(candidate)
    return None


def _probe_manual_consumer_process(*, runtime_home: Path) -> dict[str, Any]:
    command = [
        "pgrep",
        "-a",
        "-u",
        str(os.getuid()),
        "-f",
        f"{runtime_home}/.cynic/repos/CYNIC/organs/forge/src/entrypoints/cli.py daemon",
    ]
    result = _run_command(command)
    lines = [line for line in result.get("stdout", "").splitlines() if line.strip()]
    return {
        "command": command,
        "running": bool(lines),
        "matches": lines[:10],
        "returncode": result.get("returncode"),
    }
