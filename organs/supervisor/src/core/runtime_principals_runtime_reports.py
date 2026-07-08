#!/usr/bin/env python3
"""Runtime principal status, plan and lifecycle report implementation.

Split out of ``runtime_principals_views_runtime`` so the bootstrap helpers stay
small and the report surface remains isolated.
"""

from __future__ import annotations

import os
import pwd
import subprocess
from pathlib import Path

from organs.contracts.src.project_root import resolve_repo_root
from organs.supervisor.src.core.infra_consumption_contract import organism_signs_of_life_contract
from organs.supervisor.src.core.runtime_principals_lifecycle_reports import RuntimeLifecycleReportContext
from organs.supervisor.src.core.runtime_principals_lifecycle_reports import lifecycle_activation_report as _lifecycle_activation_report
from organs.supervisor.src.core.runtime_principals_lifecycle_reports import lifecycle_operator_contract as _lifecycle_operator_contract
from organs.supervisor.src.core.runtime_principals_lifecycle_reports import lifecycle_profile_dry_run as _lifecycle_profile_dry_run
from organs.supervisor.src.core.runtime_principals_lifecycle_reports import lifecycle_profile_report as _lifecycle_profile_report
from organs.supervisor.src.core.runtime_principals_lifecycle_reports import mcp_freshness_report as _mcp_freshness_report
from organs.supervisor.src.core.runtime_principals_lifecycle_reports import run_capture as _run_capture_with_cwd
from organs.supervisor.src.core.runtime_principals_lifecycle_reports import tail as _tail_text
from organs.supervisor.src.core.runtime_principals_reports import execution_posture_report as _execution_posture_report
from organs.supervisor.src.core.runtime_principals_reports import runtime_command_catalog as _runtime_command_catalog
from organs.supervisor.src.core.runtime_principals_reports import runtime_proof_requirements as _runtime_proof_requirements
from organs.supervisor.src.core.runtime_principals_content import YOLO_ARGS
from organs.supervisor.src.core.runtime_principals_content import doctor_text as _doctor_text
from organs.supervisor.src.core.runtime_principals_content import plan_payload as _plan_payload
from organs.supervisor.src.entrypoints import runtime_principals_views_runtime as _base

CANONICAL_PROJECT_ROOT = _base.CANONICAL_PROJECT_ROOT
CYNIC_CLI = _base.CYNIC_CLI
CONTRACT = _base.CONTRACT
DEFAULT_UPDATE_COMMANDS = _base.DEFAULT_UPDATE_COMMANDS
GROUP_NAME = _base.GROUP_NAME
HOME_DIR = _base.HOME_DIR
LEGACY_SUDOERS = _base.LEGACY_SUDOERS
POLICY = _base.POLICY
LIFECYCLE_PROFILE_COMMANDS = _base.LIFECYCLE_PROFILE_COMMANDS
LIFECYCLE_PROFILE_DOMAINS = _base.LIFECYCLE_PROFILE_DOMAINS
LIFECYCLE_PROFILES = _base.LIFECYCLE_PROFILES
PLAN = _base.PLAN
PRIMARY_UPDATE_COMMANDS = _base.PRIMARY_UPDATE_COMMANDS
PROFILE_DEFAULTS = _base.PROFILE_DEFAULTS
PROJECT_ROOT = _base.PROJECT_ROOT
REPO_ACCESS_PROFILE = _base.REPO_ACCESS_PROFILE
REQUIRED_OPERATOR_BINARIES = _base.REQUIRED_OPERATOR_BINARIES
STATUS = _base.STATUS
USER_NAME = _base.USER_NAME
cynic_cli_body = _base.cynic_cli_body
_can_execute = _base._can_execute
_exists_group = _base._exists_group
_exists_user = _base._exists_user
_is_dir = _base._is_dir
_is_file = _base._is_file
_json_scalar_or_list = _base._json_scalar_or_list
_legacy_pvesh_sudoers_present = _base._legacy_pvesh_sudoers_present
_launcher_projection_ok = _base._launcher_projection_ok
_launcher_yolo_flags_present = _base._launcher_yolo_flags_present
_repo_access_manager = _base._repo_access_manager
_repo_access_payload = _base._repo_access_payload
_native_check_command = _base._native_check_command
_normalize_lifecycle_blocker = _base._normalize_lifecycle_blocker
_operator_home = _base._operator_home
_owner = _base._owner
_resolve_project_root = resolve_repo_root
_user_can_access_dir = _base._user_can_access_dir
discover_operator_binary = _base.discover_operator_binary
launcher_body = _base.launcher_body
profile_body = _base.profile_body


def _resolve_expected_cynic_cli() -> str:
    return cynic_cli_body()


def cynic_cli_projection_ok() -> bool | None:
    expected = _resolve_expected_cynic_cli()
    try:
        actual = CYNIC_CLI.read_text(encoding="utf-8")
    except FileNotFoundError:
        return False
    except PermissionError:
        return None
    return actual == expected


def status() -> dict[str, object]:
    requires_root_evidence = os.geteuid() != 0
    home_owner, home_group = _owner(HOME_DIR)
    user_exists = _exists_user(USER_NAME)
    group_exists = _exists_group(GROUP_NAME)
    home_exists = _is_dir(HOME_DIR)
    home_owner_ok = None if requires_root_evidence else home_owner == USER_NAME and home_group == GROUP_NAME
    codex_launcher_path = HOME_DIR / "bin/codex-yolo"
    antigravity_launcher_path = HOME_DIR / "bin/antigravity-yolo"
    hermes_launcher_path = HOME_DIR / "bin/hermes-yolo"
    codex_launcher = _can_execute(codex_launcher_path)
    antigravity_launcher = _can_execute(antigravity_launcher_path)
    hermes_launcher = _can_execute(hermes_launcher_path)
    codex_launcher_projection = _launcher_projection_ok("codex", codex_launcher_path)
    antigravity_launcher_projection = _launcher_projection_ok("agy", antigravity_launcher_path)
    hermes_launcher_projection = _launcher_projection_ok("hermes", hermes_launcher_path)
    codex_yolo_flags_present = _launcher_yolo_flags_present("codex", codex_launcher_path)
    antigravity_yolo_flags_present = _launcher_yolo_flags_present("agy", antigravity_launcher_path)
    hermes_yolo_flags_present = _launcher_yolo_flags_present("hermes", hermes_launcher_path)
    cynic_cli_exists = os.access(CYNIC_CLI, os.X_OK)
    cynic_cli_projection = cynic_cli_projection_ok()
    canonical_project_root_exists = _is_dir(CANONICAL_PROJECT_ROOT)
    canonical_project_root_accessible_by_cynic = _user_can_access_dir(USER_NAME, CANONICAL_PROJECT_ROOT)
    legacy_violation = _legacy_pvesh_sudoers_present()
    default_profile_exists = _is_file(PROFILE_DEFAULTS)
    repo_access = _repo_access_payload()
    execution_posture = _execution_posture_report(
        user_name=USER_NAME,
        home_dir=HOME_DIR,
        yolo_args=YOLO_ARGS,
        codex_launcher_projection=codex_launcher_projection,
        antigravity_launcher_projection=antigravity_launcher_projection,
        hermes_launcher_projection=hermes_launcher_projection,
        codex_yolo_flags_present=codex_yolo_flags_present,
        antigravity_yolo_flags_present=antigravity_yolo_flags_present,
        hermes_yolo_flags_present=hermes_yolo_flags_present,
        default_profile_exists=default_profile_exists,
    )
    runtime_ready = (not requires_root_evidence) and all(
        [
            user_exists,
            group_exists,
            home_exists,
            home_owner_ok is not False,
            codex_launcher is not False,
            antigravity_launcher is not False,
            codex_launcher_projection is not False,
            antigravity_launcher_projection is not False,
            hermes_launcher_projection is not False,
            codex_yolo_flags_present is not False,
            antigravity_yolo_flags_present is not False,
            hermes_yolo_flags_present is not False,
            cynic_cli_exists,
            cynic_cli_projection is not False,
            canonical_project_root_exists,
            canonical_project_root_accessible_by_cynic is not False,
            default_profile_exists,
            not legacy_violation,
        ]
    )
    proof_requirements = _runtime_proof_requirements(
        user_name=USER_NAME,
        group_name=GROUP_NAME,
        home_dir=HOME_DIR,
        cynic_cli=CYNIC_CLI,
        canonical_project_root=CANONICAL_PROJECT_ROOT,
        profile_defaults=PROFILE_DEFAULTS,
        requires_root_evidence=requires_root_evidence,
        user_exists=user_exists,
        group_exists=group_exists,
        home_exists=home_exists,
        home_owner_ok=home_owner_ok,
        codex_launcher=codex_launcher,
        antigravity_launcher=antigravity_launcher,
        hermes_launcher=hermes_launcher,
        codex_launcher_projection=codex_launcher_projection,
        antigravity_launcher_projection=antigravity_launcher_projection,
        hermes_launcher_projection=hermes_launcher_projection,
        codex_yolo_flags_present=codex_yolo_flags_present,
        antigravity_yolo_flags_present=antigravity_yolo_flags_present,
        hermes_yolo_flags_present=hermes_yolo_flags_present,
        cynic_cli_exists=cynic_cli_exists,
        cynic_cli_projection=cynic_cli_projection,
        canonical_project_root_exists=canonical_project_root_exists,
        canonical_project_root_accessible_by_cynic=canonical_project_root_accessible_by_cynic,
        default_profile_exists=default_profile_exists,
        legacy_violation=legacy_violation,
    )
    payload = {
        "schema": "cynic.runtime_principals.status.v1",
        "dedicated_user": USER_NAME,
        "current_user": pwd.getpwuid(os.getuid()).pw_name,
        "requires_root_evidence": requires_root_evidence,
        "root_evidence_command": "sudo cynic status --json",
        "contract_exists": _is_file(CONTRACT),
        "policy_projection_exists": _is_file(POLICY),
        "user_exists": user_exists,
        "group_exists": group_exists,
        "home_exists": home_exists,
        "home_owner": home_owner,
        "home_group": home_group,
        "home_owner_ok": home_owner_ok,
        "codex_launcher_exists": codex_launcher,
        "antigravity_launcher_exists": antigravity_launcher,
        "hermes_launcher_exists": hermes_launcher,
        "codex_launcher_projection_ok": codex_launcher_projection,
        "antigravity_launcher_projection_ok": antigravity_launcher_projection,
        "hermes_launcher_projection_ok": hermes_launcher_projection,
        "codex_yolo_flags_present": codex_yolo_flags_present,
        "agy_yolo_flags_present": antigravity_yolo_flags_present,
        "hermes_yolo_flags_present": hermes_yolo_flags_present,
        "launcher_visibility": "unknown" if None in {codex_launcher, antigravity_launcher, hermes_launcher} else "visible",
        "cynic_cli": str(CYNIC_CLI),
        "cynic_cli_exists": cynic_cli_exists,
        "cynic_cli_projection_ok": cynic_cli_projection,
        "cynic_cli_projection_command": "cynic bootstrap",
        "canonical_project_root": str(CANONICAL_PROJECT_ROOT),
        "canonical_project_root_exists": canonical_project_root_exists,
        "canonical_project_root_accessible_by_cynic": canonical_project_root_accessible_by_cynic,
        "default_profile": str(PROFILE_DEFAULTS),
        "default_profile_exists": default_profile_exists,
        "repo_access": repo_access,
        "legacy_pvesh_sudoers_violation": legacy_violation,
        "runtime_ready": runtime_ready,
        "runtime_evidence_status": "requires_root_evidence" if requires_root_evidence else ("ready" if runtime_ready else "not_ready"),
        "execution_posture": execution_posture,
        "proof_requirements": proof_requirements,
    }
    payload.update(_runtime_command_catalog())
    return payload


def plan() -> dict[str, object]:
    return _plan_payload(
        user_name=USER_NAME,
        home_dir=HOME_DIR,
        canonical_project_root=CANONICAL_PROJECT_ROOT,
        legacy_sudoers=LEGACY_SUDOERS,
        operator_cli=CYNIC_CLI,
        profile_defaults=PROFILE_DEFAULTS,
    )


def _ok(value: object) -> str:
    if value is True:
        return "ok"
    if value is None:
        return "unknown"
    return "missing"


def doctor_text() -> str:
    return _doctor_text(status())


def _lifecycle_context() -> RuntimeLifecycleReportContext:
    return RuntimeLifecycleReportContext(
        canonical_project_root=CANONICAL_PROJECT_ROOT,
        lifecycle_profile_domains=LIFECYCLE_PROFILE_DOMAINS,
        lifecycle_profile_commands=LIFECYCLE_PROFILE_COMMANDS,
        is_file=_is_file,
        normalize_lifecycle_blocker=_normalize_lifecycle_blocker,
    )


def _tail(text: str, limit: int = 1200) -> str:
    return _tail_text(text, limit)


def _run_capture(
    command: list[str],
    *,
    timeout: int = 15,
    stdout_limit: int | None = 1200,
    stderr_limit: int | None = 1200,
) -> dict[str, object]:
    return _run_capture_with_cwd(
        command,
        cwd=CANONICAL_PROJECT_ROOT,
        timeout=timeout,
        stdout_limit=stdout_limit,
        stderr_limit=stderr_limit,
    )


def mcp_freshness_report() -> dict[str, object]:
    return _mcp_freshness_report(_lifecycle_context())


def _mcp_freshness_proof_requirements(status_name: str, stdout: str, stderr: str) -> list[dict[str, object]]:
    from organs.supervisor.src.core.runtime_principals_lifecycle_reports import mcp_freshness_proof_requirements

    return mcp_freshness_proof_requirements(status_name, stdout, stderr)


def lifecycle_profile_report(profile: str) -> dict[str, object]:
    return _lifecycle_profile_report(profile, _lifecycle_context())


def lifecycle_operator_contract(profile: str) -> dict[str, object]:
    return _lifecycle_operator_contract(profile)


def lifecycle_profile_dry_run(profile: str) -> dict[str, object]:
    return _lifecycle_profile_dry_run(profile, _lifecycle_context())


def lifecycle_activation_report() -> dict[str, object]:
    return _lifecycle_activation_report(_lifecycle_context())
