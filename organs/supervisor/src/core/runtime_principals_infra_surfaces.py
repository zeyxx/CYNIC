"""Runtime principal infra surface report helpers."""

from __future__ import annotations

from organs.contracts.src.surface_contracts import (
    ReportSurfaceSpec,
    SurfaceRegistry,
    attach_surface_contract,
    json_stdout_or_envelope,
)
from organs.supervisor.src.core.runtime_principals_surface import infra_next_checks as _infra_next_checks
from organs.supervisor.src.core.runtime_principals_surface import infra_proof_requirements as _infra_proof_requirements
from organs.supervisor.src.core.runtime_principals_surface import infra_report_text
from organs.supervisor.src.entrypoints import runtime_principals_views_runtime_reports as _reports

_run_capture = _reports._run_capture
_tail = _reports._tail
LIFECYCLE_PROFILES = _reports.LIFECYCLE_PROFILES
lifecycle_activation_report = _reports.lifecycle_activation_report
lifecycle_profile_report = _reports.lifecycle_profile_report
mcp_freshness_report = _reports.mcp_freshness_report
status = _reports.status

STORAGE_PRIMARY_SURFACE = ReportSurfaceSpec(schema="cynic.storage_primary_activation.status.v1", command="cynic storage status")
BRIDGE_SURFACE = ReportSurfaceSpec(schema="cynic.lxc_exec_bridge.activation_status.v1", command="cynic bridge status")
FLEET_SURFACE = ReportSurfaceSpec(schema="cynic.fleet.status.v1", command="cynic fleet status")
MESH_SURFACE = ReportSurfaceSpec(schema="cynic.infrastructure_mcp.network_mesh_status.v1", command="cynic mesh status")
INFRA_SURFACE_REGISTRY = SurfaceRegistry(
    reports=(STORAGE_PRIMARY_SURFACE, BRIDGE_SURFACE, FLEET_SURFACE, MESH_SURFACE),
)


def storage_primary_report() -> dict[str, object]:
    result = _run_capture(
        ["python3", "organs/keep/src/entrypoints/cli.py", "storage-primary", "status"],
        timeout=15,
        stdout_limit=None,
    )
    payload = json_stdout_or_envelope(result, schema="cynic.storage_primary_activation.status.v1")
    payload["exit_code"] = result["exit_code"]
    payload["mutates"] = False
    return attach_surface_contract(payload, STORAGE_PRIMARY_SURFACE)


def storage_primary_plan() -> dict[str, object]:
    result = _run_capture(
        ["python3", "organs/keep/src/entrypoints/cli.py", "storage-primary", "plan"],
        timeout=15,
        stdout_limit=None,
    )
    payload = json_stdout_or_envelope(
        result,
        schema="cynic.storage_primary_activation.bundle.v1",
        stdout_transform=_tail,
    )
    payload["exit_code"] = result["exit_code"]
    payload["command"] = "cynic storage plan"
    payload["fallback_command"] = "python3 organs/keep/src/entrypoints/cli.py storage-primary plan"
    payload["mutates_mempool"] = False
    return attach_surface_contract(payload, STORAGE_PRIMARY_SURFACE)


def storage_primary_bundle(extra_args: list[str] | None = None) -> dict[str, object]:
    extra_args = extra_args or []
    command = [
        "python3",
        "organs/keep/src/entrypoints/cli.py",
        "storage-primary",
        "bundle",
        *extra_args,
    ]
    result = _run_capture(command, timeout=20, stdout_limit=None)
    payload = json_stdout_or_envelope(
        result,
        schema="cynic.storage_primary_activation.bundle.v1",
        stdout_transform=_tail,
    )
    payload["exit_code"] = result["exit_code"]
    payload["command"] = " ".join(["cynic", "storage", "bundle", *extra_args]).strip()
    payload["fallback_command"] = " ".join(command)
    payload["mutates_mempool"] = False
    return attach_surface_contract(payload, STORAGE_PRIMARY_SURFACE)


def storage_primary_status(extra_args: list[str] | None = None) -> dict[str, object]:
    extra_args = extra_args or []
    if not extra_args:
        payload = storage_primary_report()
        payload["fallback_command"] = "python3 organs/keep/src/entrypoints/cli.py storage-primary status"
    else:
        command = [
            "python3",
            "organs/keep/src/entrypoints/cli.py",
            "storage-primary",
            "status",
            *extra_args,
        ]
        result = _run_capture(command, timeout=20, stdout_limit=None)
        payload = json_stdout_or_envelope(
            result,
            schema="cynic.storage_primary_activation.status.v1",
            stdout_transform=_tail,
        )
        payload["exit_code"] = result["exit_code"]
        payload["fallback_command"] = " ".join(command)
    payload["command"] = " ".join(["cynic", "storage", "status", *extra_args]).strip()
    payload["mutates_mempool"] = False
    attach_surface_contract(payload, STORAGE_PRIMARY_SURFACE)
    payload["next_checks"] = _infra_next_checks(payload, json_command="cynic storage status --json")
    payload.setdefault("proof_requirements", _infra_proof_requirements(payload, json_command="cynic storage status --json"))
    return payload


def lxc_exec_bridge_plan() -> dict[str, object]:
    result = _run_capture(
        ["python3", "organs/fleet/src/entrypoints/cli.py", "lxc-exec-bridge", "install-plan"],
        timeout=15,
        stdout_limit=None,
    )
    payload = json_stdout_or_envelope(
        result,
        schema="cynic.lxc_exec_bridge.install_plan.v1",
        stdout_transform=_tail,
    )
    payload["exit_code"] = result["exit_code"]
    payload["command"] = "cynic bridge plan"
    payload["fallback_command"] = "python3 organs/fleet/src/entrypoints/cli.py lxc-exec-bridge install-plan"
    payload["mutates_mempool"] = False
    return attach_surface_contract(payload, BRIDGE_SURFACE)


def lxc_exec_bridge_status() -> dict[str, object]:
    result = _run_capture(
        ["python3", "organs/fleet/src/entrypoints/cli.py", "lxc-exec-bridge", "activation-status"],
        timeout=15,
        stdout_limit=None,
    )
    payload = json_stdout_or_envelope(
        result,
        schema="cynic.lxc_exec_bridge.activation_status.v1",
        stdout_transform=_tail,
    )
    payload["exit_code"] = result["exit_code"]
    payload["command"] = "cynic bridge status"
    payload["fallback_command"] = "python3 organs/fleet/src/entrypoints/cli.py lxc-exec-bridge activation-status"
    payload["mutates_mempool"] = False
    attach_surface_contract(payload, BRIDGE_SURFACE)
    payload["next_checks"] = _infra_next_checks(payload, json_command="cynic bridge status --json")
    payload["proof_requirements"] = _infra_proof_requirements(payload, json_command="cynic bridge status --json")
    return payload


def fleet_status(extra_args: list[str] | None = None) -> dict[str, object]:
    extra_args = extra_args or []
    command = [
        "python3",
        "organs/fleet/src/entrypoints/cli.py",
        "report",
        "--json",
        "--registry",
        "infra/registry.json",
        *extra_args,
    ]
    result = _run_capture(command, timeout=20, stdout_limit=None)
    payload = json_stdout_or_envelope(
        result,
        schema="cynic.fleet.status.v1",
        stdout_transform=_tail,
    )
    payload["schema"] = payload.get("schema", "cynic.fleet.status.v1")
    payload["exit_code"] = result["exit_code"]
    payload["command"] = " ".join(["cynic", "fleet", "status", *extra_args]).strip()
    payload["fallback_command"] = " ".join(command)
    payload["mutates_mempool"] = False
    attach_surface_contract(payload, FLEET_SURFACE)
    payload["next_checks"] = _infra_next_checks(payload, json_command="cynic fleet status --json")
    payload["proof_requirements"] = _infra_proof_requirements(payload, json_command="cynic fleet status --json")
    return payload


def mesh_status(extra_args: list[str] | None = None) -> dict[str, object]:
    extra_args = extra_args or []
    command = [
        "python3",
        "organs/fleet/src/entrypoints/cli.py",
        "mesh",
        "status",
        *extra_args,
    ]
    result = _run_capture(command, timeout=20, stdout_limit=None)
    payload = json_stdout_or_envelope(
        result,
        schema="cynic.infrastructure_mcp.network_mesh_status.v1",
        stdout_transform=_tail,
    )
    payload["exit_code"] = result["exit_code"]
    payload["command"] = " ".join(["cynic", "mesh", "status", *extra_args]).strip()
    payload["fallback_command"] = " ".join(command)
    payload["mutates_mempool"] = False
    attach_surface_contract(payload, MESH_SURFACE)
    payload["next_checks"] = _infra_next_checks(payload, json_command="cynic mesh status --json")
    payload["proof_requirements"] = _infra_proof_requirements(payload, json_command="cynic mesh status --json")
    return payload
