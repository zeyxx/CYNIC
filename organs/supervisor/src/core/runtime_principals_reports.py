"""Pure report builders for runtime principal status surfaces."""

from __future__ import annotations

from pathlib import Path
from typing import Any


def proof_status(value: object) -> str:
    if value is True:
        return "satisfied"
    if value is None:
        return "unknown"
    return "missing"


def execution_posture_report(
    *,
    user_name: str,
    home_dir: Path,
    yolo_args: dict[str, list[str]],
    codex_launcher_projection: bool | None,
    antigravity_launcher_projection: bool | None,
    hermes_launcher_projection: bool | None,
    codex_yolo_flags_present: bool | None,
    antigravity_yolo_flags_present: bool | None,
    hermes_yolo_flags_present: bool | None,
    default_profile_exists: bool,
) -> dict[str, object]:
    agents = {
        "codex": {
            "command": "cynic codex",
            "runtime_user": user_name,
            "launcher": str(home_dir / "bin/codex-yolo"),
            "yolo_flags": yolo_args["codex"],
            "launcher_projection_ok": codex_launcher_projection,
            "yolo_flags_present": codex_yolo_flags_present,
        },
        "agy": {
            "command": "cynic agy",
            "runtime_user": user_name,
            "launcher": str(home_dir / "bin/antigravity-yolo"),
            "yolo_flags": yolo_args["agy"],
            "launcher_projection_ok": antigravity_launcher_projection,
            "yolo_flags_present": antigravity_yolo_flags_present,
        },
        "hermes": {
            "command": "cynic hermes",
            "runtime_user": user_name,
            "launcher": str(home_dir / "bin/hermes-yolo"),
            "yolo_flags": yolo_args["hermes"],
            "launcher_projection_ok": hermes_launcher_projection,
            "yolo_flags_present": hermes_yolo_flags_present,
        },
    }
    posture_evidence = [
        default_profile_exists,
        codex_launcher_projection,
        antigravity_launcher_projection,
        hermes_launcher_projection,
        codex_yolo_flags_present,
        antigravity_yolo_flags_present,
        hermes_yolo_flags_present,
    ]
    if any(item is None for item in posture_evidence):
        native_default: bool | None = None
    else:
        native_default = all(item is True for item in posture_evidence)
    return {
        "name": "bounded_yolo",
        "runtime_user": user_name,
        "native_default": native_default,
        "scope": "canonical_checkout",
        "destructive_authority": "passworded_or_mcp_helper_only",
        "agents": agents,
    }


def runtime_proof_requirements(
    *,
    user_name: str,
    group_name: str,
    home_dir: Path,
    cynic_cli: Path,
    canonical_project_root: Path,
    profile_defaults: Path,
    requires_root_evidence: bool,
    user_exists: bool,
    group_exists: bool,
    home_exists: bool,
    home_owner_ok: bool | None,
    codex_launcher: bool | None,
    antigravity_launcher: bool | None,
    hermes_launcher: bool | None,
    codex_launcher_projection: bool | None,
    antigravity_launcher_projection: bool | None,
    hermes_launcher_projection: bool | None,
    codex_yolo_flags_present: bool | None,
    antigravity_yolo_flags_present: bool | None,
    hermes_yolo_flags_present: bool | None,
    cynic_cli_exists: bool,
    cynic_cli_projection: bool | None,
    canonical_project_root_exists: bool,
    canonical_project_root_accessible_by_cynic: bool | None,
    default_profile_exists: bool,
    legacy_violation: bool,
) -> list[dict[str, Any]]:
    return [
        {
            "check": "root_evidence_context",
            "status": "missing" if requires_root_evidence else "satisfied",
            "command": "sudo cynic status --json",
            "expected": "runtime status collected with root evidence so ownership and dedicated-user access are observable",
            "observed": "non_root" if requires_root_evidence else "root",
        },
        {
            "check": "dedicated_user_exists",
            "status": proof_status(user_exists),
            "command": "cynic bootstrap",
            "expected": f"user {user_name} exists",
            "observed": user_exists,
        },
        {
            "check": "dedicated_group_exists",
            "status": proof_status(group_exists),
            "command": "cynic bootstrap",
            "expected": f"group {group_name} exists",
            "observed": group_exists,
        },
        {
            "check": "dedicated_home_owned",
            "status": proof_status(home_owner_ok),
            "command": "cynic bootstrap",
            "expected": f"{home_dir} is owned by {user_name}:{group_name}",
            "observed": "unknown_without_root" if home_owner_ok is None else home_owner_ok,
        },
        {
            "check": "dedicated_home_exists",
            "status": proof_status(home_exists),
            "command": "cynic bootstrap",
            "expected": f"{home_dir} exists",
            "observed": home_exists,
        },
        {
            "check": "canonical_checkout_accessible_by_dedicated_user",
            "status": proof_status(canonical_project_root_accessible_by_cynic),
            "command": "cynic bootstrap",
            "expected": f"{user_name} can traverse/read the canonical checkout",
            "observed": "unknown_without_root"
            if canonical_project_root_accessible_by_cynic is None
            else canonical_project_root_accessible_by_cynic,
        },
        {
            "check": "codex_yolo_launcher_exists",
            "status": proof_status(codex_launcher),
            "command": "cynic bootstrap",
            "expected": f"{home_dir}/bin/codex-yolo is executable",
            "observed": "unknown_without_root" if codex_launcher is None else codex_launcher,
        },
        {
            "check": "codex_bounded_yolo_projection_fresh",
            "status": proof_status(codex_launcher_projection),
            "command": "cynic bootstrap",
            "expected": "codex launcher matches generated bounded_yolo projection",
            "observed": "unknown_permission_denied" if codex_launcher_projection is None else codex_launcher_projection,
        },
        {
            "check": "codex_yolo_flags_present",
            "status": proof_status(codex_yolo_flags_present),
            "command": "cynic bootstrap",
            "expected": "codex launcher includes --dangerously-bypass-approvals-and-sandbox",
            "observed": "unknown_permission_denied" if codex_yolo_flags_present is None else codex_yolo_flags_present,
        },
        {
            "check": "agy_yolo_launcher_exists",
            "status": proof_status(antigravity_launcher),
            "command": "cynic bootstrap",
            "expected": f"{home_dir}/bin/antigravity-yolo is executable",
            "observed": "unknown_without_root" if antigravity_launcher is None else antigravity_launcher,
        },
        {
            "check": "agy_bounded_yolo_projection_fresh",
            "status": proof_status(antigravity_launcher_projection),
            "command": "cynic bootstrap",
            "expected": "agy launcher matches generated bounded_yolo projection",
            "observed": "unknown_permission_denied" if antigravity_launcher_projection is None else antigravity_launcher_projection,
        },
        {
            "check": "agy_yolo_flags_present",
            "status": proof_status(antigravity_yolo_flags_present),
            "command": "cynic bootstrap",
            "expected": "agy launcher includes --dangerously-skip-permissions",
            "observed": "unknown_permission_denied" if antigravity_yolo_flags_present is None else antigravity_yolo_flags_present,
        },
        {
            "check": "hermes_yolo_launcher_exists",
            "status": proof_status(hermes_launcher),
            "command": "cynic bootstrap",
            "expected": f"{home_dir}/bin/hermes-yolo is executable",
            "observed": "unknown_without_root" if hermes_launcher is None else hermes_launcher,
        },
        {
            "check": "hermes_bounded_yolo_projection_fresh",
            "status": proof_status(hermes_launcher_projection),
            "command": "cynic bootstrap",
            "expected": "hermes launcher matches generated bounded_yolo projection",
            "observed": "unknown_permission_denied" if hermes_launcher_projection is None else hermes_launcher_projection,
        },
        {
            "check": "hermes_yolo_flags_present",
            "status": proof_status(hermes_yolo_flags_present),
            "command": "cynic bootstrap",
            "expected": "hermes launcher uses default bounded_yolo invocation",
            "observed": "unknown_permission_denied" if hermes_yolo_flags_present is None else hermes_yolo_flags_present,
        },
        {
            "check": "operator_cli_installed",
            "status": proof_status(cynic_cli_exists),
            "command": "cynic bootstrap",
            "expected": f"{cynic_cli} is executable",
            "observed": cynic_cli_exists,
        },
        {
            "check": "operator_cli_projection_fresh",
            "status": proof_status(cynic_cli_projection),
            "command": "cynic bootstrap",
            "expected": f"{cynic_cli} matches the generated cynic wrapper contract",
            "observed": "unknown_permission_denied" if cynic_cli_projection is None else cynic_cli_projection,
        },
        {
            "check": "canonical_project_root_exists",
            "status": proof_status(canonical_project_root_exists),
            "command": "cynic status --json",
            "expected": "canonical project root exists",
            "observed": str(canonical_project_root),
        },
        {
            "check": "default_profile_installed",
            "status": proof_status(default_profile_exists),
            "command": "cynic bootstrap",
            "expected": f"{profile_defaults} exists",
            "observed": default_profile_exists,
        },
        {
            "check": "legacy_raw_provider_sudo_removed",
            "status": "missing" if legacy_violation else "satisfied",
            "command": "cynic bootstrap",
            "expected": "legacy raw pvesh NOPASSWD sudoers is absent",
            "observed": legacy_violation,
        },
    ]


def runtime_command_catalog() -> dict[str, object]:
    return {
        "launch_commands": {
            "codex": "cynic codex",
            "agy": "cynic agy",
            "hermes": "cynic hermes",
        },
        "update_commands": {
            "primary": "cynic update",
            "codex": "codex update",
            "agy": "agy update",
            "hermes": "make hermes-update",
        },
        "runtime_commands": {
            "status": "cynic status",
            "status_json": "cynic status --json",
            "doctor": "cynic doctor",
            "plan": "cynic plan",
            "shell": "cynic shell",
        },
        "lifecycle_commands": {
            "actions": {
                "inspect": "cynic actions",
                "inspect_json": "cynic actions --json",
            },
            "activation": {
                "dispatch": "cynic activate",
                "dispatch_json": "cynic activate --json",
            },
            "bootstrap_request": {
                "inspect": "cynic bootstrap-request --dry-run",
                "dispatch": "cynic bootstrap-request",
                "dispatch_json": "cynic bootstrap-request --json",
            },
            "mcp_reload": {
                "inspect": "cynic mcp-reload --dry-run",
                "dispatch": "cynic mcp-reload",
                "dispatch_json": "cynic mcp-reload --json",
            },
            "gate_2": {
                "inspect": "cynic gate-2 --dry-run",
                "dispatch": "cynic gate-2",
                "dispatch_json": "cynic gate-2 --json",
            },
            "deploy": {
                "inspect": "cynic deploy --dry-run",
                "dispatch": "cynic deploy",
                "dispatch_json": "cynic deploy --json",
            },
        },
        "storage_commands": {
            "plan": "cynic storage plan",
            "bundle": "cynic storage bundle --output-dir <dir>",
            "status": "cynic storage status",
            "status_json": "cynic storage status --json",
        },
        "bridge_commands": {
            "plan": "cynic bridge plan",
            "status": "cynic bridge status",
            "status_json": "cynic bridge status --json",
        },
        "fleet_commands": {
            "status": "cynic fleet status",
            "status_json": "cynic fleet status --json",
        },
        "mesh_commands": {
            "status": "cynic mesh status",
            "status_json": "cynic mesh status --json",
            "status_with_dns": "cynic mesh status --resolve-dns --include-records",
        },
    }
