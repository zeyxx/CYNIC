#!/usr/bin/env python3
"""Runtime principal activation helpers for CYNIC cortex CLIs.

This entrypoint owns the dedicated `cynic` user incarnation. It is non-mutating
unless called with `apply` or `install-defaults`.
"""

from __future__ import annotations

import argparse
import json
import os
import pwd
import grp
import shutil
import subprocess
import sys
import urllib.error
import urllib.request
from pathlib import Path

try:
    from organs.supervisor.src.entrypoints._bootstrap import bootstrap_repo_path
except ImportError:
    from _bootstrap import bootstrap_repo_path


PROJECT_ROOT = bootstrap_repo_path()
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

CYNIC_RUNTIME_ROOT = PROJECT_ROOT / ".cynic"
CYNIC_RUNTIME_PYTHON_ROOT = CYNIC_RUNTIME_ROOT / "python-runtime" / "bin"

from organs.supervisor.src.core.runtime_principals_content import (
    YOLO_ARGS,
    cynic_cli_body as _cynic_cli_body,
    doctor_text as _doctor_text,
    launcher_body as _launcher_body,
    plan_payload as _plan_payload,
    profile_body as _profile_body,
)
from organs.supervisor.src.core.infra_consumption_contract import organism_signs_of_life_contract
from organs.supervisor.src.core.runtime_principals_repo_access import RepoAccessManager


CONTRACT = PROJECT_ROOT / "axioms/runtime-principals-v1.json"
POLICY = PROJECT_ROOT / "infra/agent-policy.toml"
USER_NAME = os.environ.get("CYNIC_RUNTIME_USER", "cynic")
GROUP_NAME = os.environ.get("CYNIC_RUNTIME_GROUP", USER_NAME)
HOME_DIR = Path(os.environ.get("CYNIC_RUNTIME_HOME", "/" + f"home/{USER_NAME}"))
CANONICAL_PROJECT_ROOT = Path(os.environ.get("CYNIC_PROJECT_ROOT", str(PROJECT_ROOT)))
LEGACY_SUDOERS = Path("/etc/sudoers.d/cynic")
PROFILE_DEFAULTS = Path(os.environ.get("CYNIC_PROFILE_DEFAULTS", "/etc/profile.d/cynic-agents.sh"))
CYNIC_CLI = Path(os.environ.get("CYNIC_OPERATOR_CLI", "/usr/local/bin/cynic"))
REPO_ACCESS_PROFILE = Path(os.environ.get("CYNIC_REPO_ACCESS_PROFILE", str(PROJECT_ROOT / "infra/repo-access-profile.json")))
REQUIRED_OPERATOR_BINARIES = {"codex": "codex", "agy": "agy", "hermes": "hermes"}
PRIMARY_UPDATE_COMMANDS = {
    "codex": ["codex", "update"],
    "agy": ["agy", "update"],
    "hermes": ["make", "hermes-update"],
}
DEFAULT_UPDATE_COMMANDS = PRIMARY_UPDATE_COMMANDS
LIFECYCLE_PROFILES = {
    "bootstrap_request": "bootstrap-request",
    "mcp_reload": "mcp-reload",
    "gate_2": "gate-2",
    "deploy": "deploy",
}
LIFECYCLE_PROFILE_COMMANDS = {
    "bootstrap-request": "bootstrap",
    "mcp-reload": "mcp-reload",
    "gate-2": "gate-2",
    "deploy": "deploy",
}
LIFECYCLE_PROFILE_DOMAINS = {
    "bootstrap-request": "bootstrap-request",
    "mcp-reload": "mcp-runtime-request",
    "gate-2": "gate-2-request",
    "deploy": "deploy-request",
}
STATUS = "status"
PLAN = "plan"


def _repo_access_manager() -> RepoAccessManager:
    operator_user = os.environ.get("SUDO_USER")
    if not operator_user or operator_user == "root":
        operator_user, _ = _owner(CANONICAL_PROJECT_ROOT)
    if operator_user == USER_NAME:
        operator_user = None
    if not operator_user:
        operator_user = None
    canonical_root = CANONICAL_PROJECT_ROOT if _is_dir(CANONICAL_PROJECT_ROOT) else None
    return RepoAccessManager(
        dedicated_user=USER_NAME,
        dedicated_group=GROUP_NAME,
        profile=REPO_ACCESS_PROFILE,
        operator_user=operator_user,
        canonical_project_root=canonical_root,
    )


def _repo_access_payload() -> dict[str, object]:
    return _repo_access_manager().report()


def _exists_user(name: str) -> bool:
    try:
        pwd.getpwnam(name)
        return True
    except KeyError:
        return False


def _exists_group(name: str) -> bool:
    try:
        grp.getgrnam(name)
        return True
    except KeyError:
        return False


def _owner(path: Path) -> tuple[str, str]:
    try:
        stat = path.stat()
        return pwd.getpwuid(stat.st_uid).pw_name, grp.getgrgid(stat.st_gid).gr_name
    except (FileNotFoundError, KeyError, PermissionError):
        return "", ""


def _is_dir(path: Path) -> bool:
    try:
        return path.is_dir()
    except PermissionError:
        return False


def _is_file(path: Path) -> bool:
    try:
        return path.is_file()
    except PermissionError:
        return False


def _json_scalar_or_list(value: object) -> bool:
    if value is None or isinstance(value, (str, int, float, bool)):
        return True
    if isinstance(value, list):
        return all(_json_scalar_or_list(item) for item in value)
    return False


def _native_check_command(command: object) -> str | None:
    if not isinstance(command, str):
        return None
    command_map = {
        "python3 organs/fleet/src/entrypoints/cli.py report --json --registry infra/registry.json": "cynic fleet status",
        "python3 organs/fleet/src/entrypoints/cli.py lxc-exec-bridge activation-status": "cynic bridge status",
        "python3 organs/fleet/src/entrypoints/cli.py mesh status": "cynic mesh status",
        "python3 organs/keep/src/entrypoints/cli.py storage-primary status": "cynic storage status",
    }
    return command_map.get(command)


def _normalize_lifecycle_blocker(blocker: dict[str, object]) -> dict[str, object]:
    normalized = {key: value for key, value in blocker.items() if _json_scalar_or_list(value)}
    native_check = _native_check_command(normalized.get("command"))
    if native_check:
        normalized["native_check_command"] = native_check
    code = str(normalized.get("code") or "")
    if normalized.get("kind") == "fleet_inventory_evidence_gap" and code.startswith("active_without_runtime_proof:"):
        normalized.setdefault("node", code.split(":", 1)[1])
        normalized.setdefault("resolver_organ", "organ-fleet")
        normalized.setdefault("resolver_consumer", "organ-forge-hermes-agent")
        normalized.setdefault("native_check_command", "cynic fleet status")
    return normalized


def _can_execute(path: Path) -> bool | None:
    try:
        path.stat()
    except PermissionError:
        return None
    except FileNotFoundError:
        return False
    return os.access(path, os.X_OK)


def _legacy_pvesh_sudoers_present() -> bool:
    try:
        return "/usr/bin/pvesh" in LEGACY_SUDOERS.read_text(encoding="utf-8")
    except (FileNotFoundError, PermissionError):
        return False


def _user_can_access_dir(user: str, path: Path) -> bool | None:
    if os.geteuid() != 0:
        return None
    result = subprocess.run(["sudo", "-u", user, "test", "-d", str(path)], check=False)
    return result.returncode == 0


def _operator_home() -> Path | None:
    operator_user = os.environ.get("SUDO_USER")
    if not operator_user or operator_user == "root":
        operator_user, _ = _owner(CANONICAL_PROJECT_ROOT)
    if not operator_user or operator_user in {"", "root", USER_NAME}:
        return None
    try:
        return Path(pwd.getpwnam(operator_user).pw_dir)
    except KeyError:
        return None


def discover_operator_binary(binary: str) -> Path | None:
    operator_home = _operator_home()
    candidates: list[Path] = []
    if operator_home:
        candidates.extend(
            [
                operator_home / ".local/bin" / binary,
                operator_home / "bin" / binary,
            ]
        )
    candidates.extend([CYNIC_RUNTIME_PYTHON_ROOT / binary])
    candidates.extend(
        [
            CANONICAL_PROJECT_ROOT / ".cynic" / "python-runtime" / "bin" / binary,
            PROJECT_ROOT / ".cynic" / "python-runtime" / "bin" / binary,
        ]
    )
    found = shutil.which(binary)
    if found:
        candidates.append(Path(found))
    for candidate in candidates:
        try:
            if candidate.exists():
                return candidate
        except PermissionError:
            continue
    return None


def launcher_body(binary: str) -> str:
    return _launcher_body(binary, CANONICAL_PROJECT_ROOT)


def cynic_cli_body() -> str:
    return _cynic_cli_body(USER_NAME, PROJECT_ROOT, CANONICAL_PROJECT_ROOT)


def profile_body() -> str:
    return _profile_body()


def cynic_cli_projection_ok() -> bool | None:
    expected = cynic_cli_body()
    try:
        actual = CYNIC_CLI.read_text(encoding="utf-8")
    except FileNotFoundError:
        return False
    except PermissionError:
        return None
    return actual == expected


def _launcher_projection_ok(binary: str, path: Path) -> bool | None:
    expected = launcher_body(binary)
    try:
        actual = path.read_text(encoding="utf-8")
    except FileNotFoundError:
        return False
    except PermissionError:
        return None
    return actual == expected


def _launcher_yolo_flags_present(binary: str, path: Path) -> bool | None:
    flags = YOLO_ARGS.get(binary, [])
    try:
        actual = path.read_text(encoding="utf-8")
    except FileNotFoundError:
        return False
    except PermissionError:
        return None
    return all(flag in actual for flag in flags)
