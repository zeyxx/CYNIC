from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

from organs.contracts.src.project_root import resolve_repo_root
from organs.contracts.src.surface_contracts import json_object_or_default


PROFILE_DOMAINS = {
    "bootstrap": "bootstrap-request",
    "mcp-reload": "mcp-runtime-request",
    "gate-2": "gate-2-request",
    "deploy": "deploy-request",
}


def _repo_root() -> Path:
    return resolve_repo_root(Path(__file__).resolve())


def _subprocess_env(repo_root: Path) -> dict[str, str]:
    env = os.environ.copy()
    env["CYNIC_PROJECT_ROOT"] = str(repo_root)
    pythonpath = env.get("PYTHONPATH", "")
    entries = [str(repo_root)]
    if pythonpath:
        entries.append(pythonpath)
    env["PYTHONPATH"] = os.pathsep.join(entries)
    return env


def _run_python_command(cmd: list[str], parse_error: str) -> dict[str, object]:
    repo_root = _repo_root()
    result = subprocess.run(
        cmd,
        cwd=repo_root,
        env=_subprocess_env(repo_root),
        capture_output=True,
        text=True,
        check=False,
    )
    return json_object_or_default(
        result.stdout,
        {"error": parse_error, "stdout": result.stdout, "stderr": result.stderr},
    )


def _run_lifecycle(args: list[str]) -> dict[str, object]:
    command = "lifecycle-blockers" if "--dry-run" in args else "lifecycle-activation-dispatch"
    cmd = [sys.executable, "organs/forge/src/entrypoints/cli.py", command]
    return _run_python_command(cmd, "Failed to parse lifecycle output")

def _run_profile_payloads(profile: str) -> dict[str, object]:
    domain = PROFILE_DOMAINS.get(profile, f"{profile}-request")
    cmd = [
        sys.executable,
        "organs/forge/src/entrypoints/cli.py",
        "lifecycle-profile-payloads",
        profile,
        domain,
        "cortex-runtime-principals",
    ]
    return _run_python_command(cmd, "Failed to parse profile output")


def _run_profile_dispatch(profile: str) -> dict[str, object]:
    cmd = [sys.executable, "organs/forge/src/entrypoints/cli.py", "delegate-profile", profile, "--json"]
    return _run_python_command(cmd, "Failed to parse profile dispatch output")

def activation_operator_contract() -> dict[str, object]:
    return _run_lifecycle(["--dry-run", "--json"])

def activate_lifecycle() -> dict[str, object]:
    return _run_lifecycle(["--json"])

def activation_dry_run() -> dict[str, object]:
    return _run_lifecycle(["--dry-run", "--json"])

def activate() -> dict[str, object]:
    return activate_lifecycle()

def request_lifecycle_profile(profile: str) -> dict[str, object]:
    return _run_profile_dispatch(profile)

def lifecycle_profile_dry_run(profile: str) -> dict[str, object]:
    return _run_profile_payloads(profile)
