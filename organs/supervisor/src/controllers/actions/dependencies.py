from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

from organs.contracts.src.project_root import resolve_repo_root
from organs.supervisor.src.entrypoints.runtime_principals_views_runtime_reports import (
    PRIMARY_UPDATE_COMMANDS,
    discover_operator_binary,
)

_CODEX_SOFT_UPDATE_FAILURE_MARKERS: tuple[str, ...] = (
    "Could not detect the Codex installation method",
)


def _is_soft_failure(name: str, completed: subprocess.CompletedProcess[str]) -> bool:
    if name != "codex":
        return False
    combined_output = "".join(
        filter(None, [completed.stdout or "", completed.stderr or ""])
    ).strip()
    return any(marker in combined_output for marker in _CODEX_SOFT_UPDATE_FAILURE_MARKERS)


def _run_update_command(name: str, command: list[str], env: dict[str, str] | None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        command,
        check=False,
        env=env,
        text=True,
        capture_output=True,
    )


def _resolve_update_command(name: str, command: list[str]) -> list[str] | None:
    if len(command) < 2:
        return None

    requested_binary = command[0]
    resolved = discover_operator_binary(requested_binary)
    if resolved is None:
        resolved = Path(shutil.which(requested_binary)) if shutil.which(requested_binary) else None

    if resolved is None:
        return None

    if name == "hermes" and command == ["make", "hermes-update"]:
        repo_root = resolve_repo_root(Path(__file__).resolve())
        return [str(resolved), "-C", str(repo_root), "hermes-update"]

    return [str(resolved), *command[1:]]


def _prepare_update_command(name: str, command: list[str]) -> tuple[list[str] | None, dict[str, str] | None]:
    resolved = _resolve_update_command(name, command)
    if resolved is None:
        return None, None

    return resolved, None


def update_primary_dependencies() -> dict[str, object]:
    results: dict[str, object] = {}
    for name, command in PRIMARY_UPDATE_COMMANDS.items():
        resolved_command, env = _prepare_update_command(name, list(command))
        if resolved_command is None:
            results[name] = {"status": "missing", "command": command}
            continue

        completed = _run_update_command(name, resolved_command, env)
        if completed.returncode != 0 and _is_soft_failure(name, completed):
            results[name] = {
                "status": "ok",
                "command": command,
                "returncode": completed.returncode,
                "result": "soft-failed and skipped",
                "message": "Soft failure ignored: install method is unavailable in this runtime."
                " Continue with dedicated runtime update path.",
            }
            continue

        results[name] = {
            "status": "ok" if completed.returncode == 0 else "failed",
            "command": command,
            "returncode": completed.returncode,
        }
    return {"results": results}


def update_dry_run() -> dict[str, object]:
    results: dict[str, object] = {}
    for name, command in PRIMARY_UPDATE_COMMANDS.items():
        resolved = _resolve_update_command(name, list(command))
        results[name] = {
            "status": "ok" if resolved is not None else "missing",
            "command": command,
            "dry_run": True,
        }
    return {"results": results}
