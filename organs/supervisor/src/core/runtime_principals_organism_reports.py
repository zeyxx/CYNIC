"""Runtime principal organism reporting implementation.

This module owns the heavier reporting surface for organism and activation
signals so the compatibility facade can stay thin.
"""

from __future__ import annotations

import json
import os
from pathlib import Path
import urllib.error
import urllib.request

from organs.contracts.src.adapters.kernel_http import kernel_auth_headers, kernel_url
from organs.forge.src.adapters.kernel_adapter import normalize_kernel_base_url
from organs.forge.src.adapters.kernel_bus_probe_adapter import diagnose_kernel_http_failure
from organs.forge.src.adapters.kernel_task_client import KernelTaskClient
from organs.supervisor.src.core.infra_consumption_contract import organism_signs_of_life_contract
from organs.contracts.src.surface_contracts import (
    ReportSurfaceSpec,
    SurfaceRegistry,
    attach_surface_contract,
    json_object_or_default,
)
from organs.contracts.src.adapters.env import load_required_env_from_default_file
from organs.supervisor.src.core.organism_text_sections import (
    append_pattern_evolution,
    append_topology_summary,
)
from organs.supervisor.src.core.runtime_principals_organism import (
    activation_dispatch_plan as _activation_dispatch_plan,
    activation_proof_requirements as _activation_proof_requirements,
    normalize_activation_action as _normalize_activation_action,
)
from organs.supervisor.src.core.organism_stack import organism_stack_report
from organs.supervisor.src.core.runtime_principals_organism_text import (
    activation_dispatch_text as _activation_dispatch_text,
    lifecycle_profile_request_text as _lifecycle_profile_request_text,
    organism_actions_text as _organism_actions_text,
    organism_text as _organism_text,
)
from organs.supervisor.src.entrypoints.runtime_principals_views_runtime_reports import (
    LIFECYCLE_PROFILES,
    _tail,
    _normalize_lifecycle_blocker,
    lifecycle_activation_report,
    lifecycle_profile_report,
    mcp_freshness_report,
    status,
)
from organs.supervisor.src.entrypoints.runtime_principals_views_sequence import (
    _organism_blocker_check,
    _organism_next_checks,
    _organism_operator_sequence,
    _order_next_checks_by_operator_sequence,
)
from organs.supervisor.src.entrypoints.runtime_principals_views_surface import (
    fleet_status,
    lxc_exec_bridge_status,
    mesh_status,
    storage_primary_status,
)


ORGANISM_ACTIONS_SURFACE = ReportSurfaceSpec(
    schema="cynic.organism.actions.v1",
    command="cynic actions",
    mutates=False,
    mutates_mempool=False,
)
ORGANISM_CONVERGE_SURFACE = ReportSurfaceSpec(
    schema="cynic.organism.convergence.v1",
    command="cynic converge",
    mutates=False,
    mutates_mempool=False,
)
ORGANISM_SURFACE = ReportSurfaceSpec(
    schema="cynic.organism.signs_of_life.v1",
    command="cynic organism",
    mutates=False,
    mutates_mempool=False,
)
ORGANISM_SURFACE_REGISTRY = SurfaceRegistry(
    reports=(ORGANISM_ACTIONS_SURFACE, ORGANISM_CONVERGE_SURFACE, ORGANISM_SURFACE),
)


def local_validation_report(project_root: object) -> dict[str, object]:
    root = Path(str(project_root))
    markers = {
        "gate_0": root / ".gate-0",
        "gate_1": root / ".gate-1",
    }
    rust_sources = [
        path
        for path in (root / "crates").rglob("*.rs")
        if "target" not in path.parts
    ] if (root / "crates").is_dir() else []
    newest_rust_mtime = max((int(path.stat().st_mtime) for path in rust_sources), default=None)
    marker_status: dict[str, object] = {}
    missing: list[str] = []
    for name, path in markers.items():
        exists = path.is_file()
        marker_status[name] = {
            "path": str(path),
            "exists": exists,
            **({"mtime": int(path.stat().st_mtime)} if exists else {}),
        }
        if not exists:
            missing.append(name)
    gate_1_mtime = marker_status["gate_1"].get("mtime")
    gate_1_fresh = (
        marker_status["gate_1"]["exists"]
        and (newest_rust_mtime is None or int(gate_1_mtime) >= newest_rust_mtime)
    )
    if marker_status["gate_1"]["exists"] and not gate_1_fresh:
        missing.append("gate_1_fresh")
    return {
        "schema": "cynic.local_validation.status.v1",
        "status": "ready" if not missing else "blocked",
        "mutates": False,
        "markers": marker_status,
        "missing": missing,
        "newest_rust_source_mtime": newest_rust_mtime,
        "commands": {
            "gate_0": "make gate-0",
            "gate_1": "make gate-1",
            "check": "make check",
        },
        "proof_requirements": [
            {
                "check": "gate_0_marker",
                "command": "make gate-0",
                "expected": ".gate-0 exists after Level 0 local validation",
                "observed": marker_status["gate_0"]["exists"],
                "status": "satisfied" if marker_status["gate_0"]["exists"] else "missing",
            },
            {
                "check": "gate_1_marker",
                "command": "make gate-1",
                "expected": ".gate-1 exists after Level 1 local validation",
                "observed": marker_status["gate_1"]["exists"],
                "status": "satisfied" if marker_status["gate_1"]["exists"] else "missing",
            },
            {
                "check": "gate_1_fresh",
                "command": "make gate-1",
                "expected": ".gate-1 is newer than Rust sources",
                "observed": gate_1_fresh,
                "status": "satisfied" if gate_1_fresh else "missing",
            },
        ],
    }


def kernel_health_report() -> dict[str, object]:
    rest_addr = os.environ.get("CYNIC_REST_ADDR", "").strip()
    if not rest_addr:
        return {"status": "unknown", "reason": "CYNIC_REST_ADDR is not set"}
    url = kernel_url(rest_addr, "/health")
    request = urllib.request.Request(url)
    api_key = os.environ.get("CYNIC_API_KEY")
    if api_key:
        for key, value in kernel_auth_headers(api_key, content_type=False).items():
            request.add_header(key, value)
    try:
        with urllib.request.urlopen(request, timeout=5) as response:
            body = response.read().decode("utf-8")
    except (urllib.error.URLError, TimeoutError) as exc:
        diagnosis = diagnose_kernel_http_failure(
            base_url=normalize_kernel_base_url(rest_addr),
            path="/health",
            timeout_seconds=2,
        )
        return {
            "status": "unreachable",
            "endpoint": url,
            "error": str(exc),
            "diagnosis": diagnosis,
        }
    payload = json_object_or_default(body, {"raw": _tail(body)})
    return {"status": "ok", "endpoint": url, "payload": payload}


def organism_actions_report(*, run_probes: bool = True) -> dict[str, object]:
    if run_probes:
        activation = lifecycle_activation_report()
    else:
        activation = {
            "schema": "cynic.lifecycle.activation_summary.v1",
            "status": "unknown",
            "activation_request_summary": [],
            "blockers": [],
            "mutates_mempool": False,
        }
    raw_actions = activation.get("activation_request_summary", [])
    actions = [
        _normalize_activation_action(item)
        for item in raw_actions
        if isinstance(item, dict)
    ]
    raw_blockers = activation.get("blockers", [])
    blockers = [
        _normalize_lifecycle_blocker(item)
        for item in raw_blockers
        if isinstance(item, dict)
    ]
    dispatch_command = activation.get("activation_dispatch_command", "cynic activate")
    dispatch_plan = _activation_dispatch_plan(actions, dispatch_command)
    proof_requirements = _activation_proof_requirements(actions, activation, dispatch_command)
    return {
        "schema": "cynic.organism.actions.v1",
        "mutates": False,
        "mutates_mempool": False,
        **attach_surface_contract({}, ORGANISM_ACTIONS_SURFACE),
        "status": activation.get("status", "unknown"),
        "action_count": len(actions),
        "activation_request_count": len(actions),
        "actions": actions,
        "blockers": blockers,
        "dispatch_command": dispatch_command,
        "dispatch_mutates_mempool": True,
        "dispatch_plan": dispatch_plan,
        "consumers": dispatch_plan["consumers"],
        "coord_targets": dispatch_plan["coord_targets"],
        "proof_requirements": proof_requirements,
        "source_schema": activation.get("schema"),
    }


def organism_actions_text(report: dict[str, object] | None = None) -> str:
    return _organism_actions_text(report or organism_actions_report())


def organism_convergence_report(*, run_probes: bool = True) -> dict[str, object]:
    report = organism_report(run_probes=run_probes)
    stack = report.get("stack", {})
    if not isinstance(stack, dict):
        stack = {}
    actions = stack.get("actions", [])
    requests = stack.get("dispatchable_requests", [])
    patterns = stack.get("patterns", [])
    entities = stack.get("entities", [])
    topology_summary = stack.get("topology_summary", {})
    pattern_evolution = stack.get("pattern_evolution", {})
    return {
        "schema": "cynic.organism.convergence.v1",
        "mutates": False,
        "mutates_mempool": False,
        **attach_surface_contract({}, ORGANISM_CONVERGE_SURFACE),
        "status": stack.get("status", "unknown"),
        "action_count": len(actions) if isinstance(actions, list) else 0,
        "dispatchable_request_count": len(requests) if isinstance(requests, list) else 0,
        "pattern_count": len(patterns) if isinstance(patterns, list) else 0,
        "topology_summary": topology_summary if isinstance(topology_summary, dict) else {},
        "pattern_evolution": pattern_evolution if isinstance(pattern_evolution, dict) else {},
        "living_topology": _living_topology_projection(entities if isinstance(entities, list) else []),
        "actions": actions if isinstance(actions, list) else [],
        "dispatchable_requests": requests if isinstance(requests, list) else [],
        "patterns": patterns if isinstance(patterns, list) else [],
        "operator_sequence": [
            {"step": "inspect", "command": "cynic converge --dry-run", "mutates_mempool": False},
            {"step": "restore_bus", "command": "cynic mcp-reload", "mutates_mempool": True},
            {"step": "observe", "command": "cynic organism --json", "mutates_mempool": False},
        ],
        "done_when": [
            "all dispatchable_requests have either been sent through Kernel read-back or replaced by narrower blockers",
            "cynic organism --json reports fewer blocked/degraded L0-L7 entities",
        ],
    }


def organism_convergence_text(report: dict[str, object] | None = None) -> str:
    payload = report or organism_convergence_report()
    lines = [
        "CYNIC organism convergence",
        f"Status: {payload.get('status', 'unknown')}",
        f"Actions: {payload.get('action_count', 0)}",
        f"Dispatchable requests: {payload.get('dispatchable_request_count', 0)}",
        f"Patterns: {payload.get('pattern_count', 0)}",
    ]
    append_topology_summary(lines, payload.get("topology_summary"), title="Topology:")
    append_pattern_evolution(lines, payload.get("pattern_evolution"))
    living = payload.get("living_topology")
    if isinstance(living, dict):
        domains = living.get("domains")
        if isinstance(domains, list) and domains:
            lines.append("Domains:")
            for item in domains[:8]:
                if isinstance(item, dict):
                    lines.append(f"  - {item.get('entity')}: {item.get('observed')}")
            if len(domains) > 8:
                lines.append(f"  - ... {len(domains) - 8} more")
        surfaces = living.get("kernel_surfaces")
        if isinstance(surfaces, list) and surfaces:
            lines.append("Kernel surfaces:")
            for item in surfaces[:6]:
                if isinstance(item, dict):
                    lines.append(f"  - {item.get('entity')}: {item.get('observed')}")
        supply = living.get("supply_chains")
        if isinstance(supply, list) and supply:
            lines.append("Supply-chain:")
            for item in supply[:4]:
                if isinstance(item, dict):
                    lines.append(f"  - {item.get('entity')}: {item.get('observed')}")
        muscles = living.get("async_muscles")
        if isinstance(muscles, list) and muscles:
            lines.append("Async muscles:")
            for item in muscles[:4]:
                if isinstance(item, dict):
                    lines.append(f"  - {item.get('entity')}: {item.get('observed')}")
    requests = payload.get("dispatchable_requests", [])
    if isinstance(requests, list) and requests:
        lines.append("Dispatchable:")
        for request in requests[:8]:
            if not isinstance(request, dict):
                continue
            lines.append(f"  - {request.get('domain')} -> {request.get('consumer')} p={request.get('priority')}")
            targets = request.get("coord_targets")
            if isinstance(targets, list) and targets:
                lines.append(f"    targets: {', '.join(str(item) for item in targets[:6])}")
            caused_by = request.get("caused_by_pattern_ids")
            if isinstance(caused_by, list) and caused_by:
                lines.append(f"    caused-by: {', '.join(str(item) for item in caused_by[:4])}")
    patterns = payload.get("patterns", [])
    if isinstance(patterns, list) and patterns:
        for pattern in patterns[:8]:
            if not isinstance(pattern, dict):
                continue
            lines.append(
                f"Pattern {pattern.get('pattern_id', pattern.get('pattern'))}: "
                f"{pattern.get('severity')} -> {pattern.get('resolution')}"
            )
            if pattern.get("pattern") != "utility_duplicate_pattern":
                continue
            entities = pattern.get("entities")
            if not isinstance(entities, dict):
                continue
            lines.append("Utility duplicates:")
            for name, paths in list(entities.items())[:6]:
                if isinstance(paths, list):
                    sample = ", ".join(str(path) for path in paths[:2])
                    suffix = f" (+{len(paths) - 2})" if len(paths) > 2 else ""
                    lines.append(f"  - {name}: {sample}{suffix}")
    actions = payload.get("actions", [])
    if isinstance(actions, list) and actions:
        lines.append("Actions:")
        for action in actions[:10]:
            if not isinstance(action, dict):
                continue
            flags = []
            if action.get("mutates_mempool"):
                flags.append("mutates mempool")
            if action.get("mutates_files"):
                flags.append("writes files")
            suffix = f" [{', '.join(flags)}]" if flags else ""
            lines.append(f"  - {action.get('layer')} {action.get('entity')}: {action.get('action')}{suffix}")
    lines.append("Machine output: cynic converge --json")
    return "\n".join(lines) + "\n"


def _living_topology_projection(entities: list[object]) -> dict[str, list[dict[str, object]]]:
    projected = {
        "domains": [],
        "kernel_surfaces": [],
        "node_roles": [],
        "supply_chains": [],
        "async_muscles": [],
        "recursion_contracts": [],
    }
    kind_map = {
        "domain_topology": "domains",
        "kernel_surface_topology": "kernel_surfaces",
        "node_role_topology": "node_roles",
        "supply_chain_topology": "supply_chains",
        "async_muscle": "async_muscles",
        "recursion_contract": "recursion_contracts",
    }
    for entity in entities:
        if not isinstance(entity, dict):
            continue
        bucket = kind_map.get(str(entity.get("kind") or ""))
        if not bucket:
            continue
        projected[bucket].append(
            {
                "entity": entity.get("entity"),
                "health": entity.get("health"),
                "observed": entity.get("observed"),
                "owner": entity.get("owner"),
                "next_action": entity.get("next_action"),
            }
        )
    return projected


def organism_convergence_dispatch_report(*, run_probes: bool = True) -> dict[str, object]:
    convergence = organism_convergence_report(run_probes=run_probes)
    requests = convergence.get("dispatchable_requests", [])
    if not isinstance(requests, list):
        requests = []
    dispatchable = [item for item in requests if isinstance(item, dict)]
    kernel_env = _load_cynic_env_if_needed()
    base_url = os.environ.get("CYNIC_REST_ADDR", "").strip()
    api_key = os.environ.get("CYNIC_API_KEY", "").strip()
    if not base_url or not api_key:
        return {
            "schema": "cynic.organism.convergence_dispatch.v1",
            "status": "blocked",
            "mutates": False,
            "mutates_mempool": False,
            "reason": "CYNIC_REST_ADDR and CYNIC_API_KEY are required for convergence dispatch",
            "requested": len(dispatchable),
            "dispatched": 0,
            "skipped_existing": 0,
            "dispatchable_requests": dispatchable,
            "read_back": "not_attempted",
            "inspect_command": "cynic converge --dry-run",
            "kernel_environment": kernel_env,
        }
    client = KernelTaskClient(normalize_kernel_base_url(base_url), api_key)
    try:
        pending = client.list_agent_tasks(kind="hermes", domain="organism-convergence", completed=False)
        completed = client.list_agent_tasks(kind="hermes", domain="organism-convergence", completed=True)
    except Exception as exc:
        return {
            "schema": "cynic.organism.convergence_dispatch.v1",
            "status": "blocked",
            "mutates": False,
            "mutates_mempool": False,
            "reason": f"kernel task read failed: {_tail(str(exc))}",
            "requested": len(dispatchable),
            "dispatched": 0,
            "skipped_existing": 0,
            "dispatchable_requests": dispatchable,
            "read_back": "not_attempted",
            "inspect_command": "cynic converge --dry-run",
            "kernel_environment": kernel_env,
        }
    existing = _convergence_tasks_by_idempotency_key(
        [
            *pending.get("tasks", []),
            *completed.get("tasks", []),
        ]
    )
    dispatched: list[dict[str, object]] = []
    skipped_existing: list[dict[str, object]] = []
    failed_requests: list[dict[str, object]] = []
    responses: list[dict[str, object]] = []
    for request in dispatchable:
        key = _request_idempotency_key(request)
        if key and key in existing:
            skipped_existing.append(
                {
                    "task_id": existing[key].get("id"),
                    "idempotency_key": key,
                    "reason": "existing_pending_or_completed",
                    "consumer": request.get("consumer"),
                }
            )
            continue
        try:
            response = client.post_agent_task(request)
            task_id = response.get("task_id") or response.get("id")
            if not task_id:
                raise RuntimeError(f"convergence task dispatch response missing task_id: {response}")
            client.assert_agent_task_visible(
                str(task_id),
                kind=str(request.get("kind", "hermes")),
                domain=str(request.get("domain", "organism-convergence")),
            )
        except Exception as exc:
            failed_requests.append(
                {
                    "idempotency_key": key,
                    "consumer": request.get("consumer"),
                    "priority": request.get("priority"),
                    "coord_targets": request.get("coord_targets", []),
                    "error": _tail(str(exc)),
                }
            )
            continue
        responses.append(response)
        dispatched.append(
            {
                "task_id": task_id,
                "idempotency_key": key,
                "consumer": request.get("consumer"),
                "priority": request.get("priority"),
                "coord_targets": request.get("coord_targets", []),
            }
        )
        if key:
            existing[key] = {"id": task_id}
    status = "ok"
    if failed_requests and dispatched:
        status = "partial"
    elif failed_requests:
        status = "blocked"
    return {
        "schema": "cynic.organism.convergence_dispatch.v1",
        "status": status,
        "mutates": False,
        "mutates_mempool": bool(dispatched),
        "requested": len(dispatchable),
        "dispatched": len(dispatched),
        "skipped_existing": len(skipped_existing),
        "failed": len(failed_requests),
        "responses": responses,
        "dispatched_tasks": dispatched,
        "skipped_existing_tasks": skipped_existing,
        "failed_requests": failed_requests,
        "task_ids": [item.get("task_id") for item in dispatched],
        "inspect_command": "cynic converge --dry-run",
        "proof_commands": ["cynic converge --json", "cynic organism --json"],
        "read_back": "each dispatched task id was visible in pending or completed Kernel task lists"
        if not failed_requests
        else "failed requests did not prove Kernel read-back",
        "kernel_environment": kernel_env,
    }


def organism_convergence_dispatch_text(report: dict[str, object]) -> str:
    lines = [
        "CYNIC organism convergence dispatch",
        f"Status: {report.get('status', 'unknown')}",
        f"Requested: {report.get('requested', 0)}",
        f"Dispatched: {report.get('dispatched', 0)}",
        f"Skipped existing: {report.get('skipped_existing', 0)}",
        f"Failed: {report.get('failed', 0)}",
    ]
    reason = report.get("reason")
    if reason:
        lines.append(f"Reason: {reason}")
    dispatched = report.get("dispatched_tasks", [])
    if isinstance(dispatched, list) and dispatched:
        lines.append("Task ids:")
        for item in dispatched:
            if isinstance(item, dict):
                lines.append(f"  - {item.get('task_id')}: {item.get('idempotency_key')}")
    skipped = report.get("skipped_existing_tasks", [])
    if isinstance(skipped, list) and skipped:
        lines.append("Existing:")
        for item in skipped[:8]:
            if isinstance(item, dict):
                lines.append(f"  - {item.get('task_id')}: {item.get('idempotency_key')}")
    failed = report.get("failed_requests", [])
    if isinstance(failed, list) and failed:
        lines.append("Failed:")
        for item in failed[:8]:
            if isinstance(item, dict):
                lines.append(f"  - {item.get('idempotency_key')}: {item.get('error')}")
    lines.append("Machine output: cynic converge --dispatch --json")
    return "\n".join(lines) + "\n"


def _convergence_tasks_by_idempotency_key(tasks: list[object]) -> dict[str, dict[str, object]]:
    result: dict[str, dict[str, object]] = {}
    for task in tasks:
        if not isinstance(task, dict):
            continue
        key = _task_idempotency_key(task)
        if key:
            result[key] = task
    return result


def _task_idempotency_key(task: dict[str, object]) -> str | None:
    content = task.get("content")
    if isinstance(content, str):
        try:
            content = json.loads(content)
        except json.JSONDecodeError:
            content = {}
    if isinstance(content, dict):
        key = content.get("idempotency_key")
        return str(key) if key else None
    return None


def _load_cynic_env_if_needed() -> dict[str, object]:
    return load_required_env_from_default_file(("CYNIC_REST_ADDR", "CYNIC_API_KEY"))


def _request_idempotency_key(request: dict[str, object]) -> str | None:
    content = request.get("content")
    if isinstance(content, str):
        try:
            content = json.loads(content)
        except json.JSONDecodeError:
            content = {}
    if isinstance(content, dict):
        key = content.get("idempotency_key")
        return str(key) if key else None
    return None


def activation_dispatch_text(report: dict[str, object]) -> str:
    return _activation_dispatch_text(report)


def lifecycle_profile_request_text(report: dict[str, object]) -> str:
    return _lifecycle_profile_request_text(report)


def organism_report(*, run_probes: bool = True) -> dict[str, object]:
    runtime = status()
    local_validation = local_validation_report(runtime.get("canonical_project_root", os.getcwd()))
    stack = organism_stack_report(runtime.get("canonical_project_root", os.getcwd()), runtime)
    contract = organism_signs_of_life_contract()
    if run_probes:
        mcp = mcp_freshness_report()
        storage = storage_primary_status()
        bridge = lxc_exec_bridge_status()
        fleet = fleet_status()
        mesh = mesh_status(["--resolve-dns", "--include-records"])
        kernel = kernel_health_report()
        lifecycle = {name: lifecycle_profile_report(profile) for name, profile in LIFECYCLE_PROFILES.items()}
        activation = lifecycle_activation_report()
    else:
        mcp = {"status": "unknown"}
        storage = {"status": "unknown"}
        bridge = {"status": "unknown"}
        fleet = {"status": "unknown"}
        mesh = {"status": "unknown"}
        kernel = {"status": "unknown"}
        lifecycle = {name: {"profile": profile, "status": "unknown"} for name, profile in LIFECYCLE_PROFILES.items()}
        activation = {
            "schema": "cynic.lifecycle.activation_summary.v1",
            "status": "unknown",
            "activation_request_summary": [],
            "blockers": [],
            "mutates_mempool": False,
        }
    blockers: list[dict[str, object]] = []
    if local_validation.get("status") != "ready":
        blockers.append(
            {
                "surface": "local_validation",
                "status": local_validation.get("status"),
                "missing": local_validation.get("missing", []),
                "check": "make check",
                "next_checks": ["make check"],
                "proof_requirements": local_validation.get("proof_requirements", []),
            }
        )
    if run_probes:
        if not runtime["runtime_ready"]:
            runtime_next_checks = []
            if runtime.get("cynic_cli_projection_ok") is False:
                runtime_next_checks.append(runtime.get("cynic_cli_projection_command", "cynic bootstrap"))
            blockers.append(
                {
                    "surface": "cortex_runtime",
                    "status": runtime.get("runtime_evidence_status", "not_ready"),
                    "check": runtime.get("root_evidence_command", "cynic status"),
                    "proof_requirements": runtime.get("proof_requirements", []),
                    **({"next_checks": runtime_next_checks} if runtime_next_checks else {}),
                }
            )
    else:
        runtime_next_checks = []
        if runtime.get("cynic_cli_projection_ok") is False:
            runtime_next_checks.append(runtime.get("cynic_cli_projection_command", "cynic bootstrap"))
        blockers.append(
            {
                "surface": "cortex_runtime",
                "status": runtime.get(
                    "runtime_evidence_status",
                    "ready" if runtime["runtime_ready"] else "not_ready",
                ),
                "check": runtime.get("root_evidence_command", "cynic status"),
                "proof_requirements": runtime.get("proof_requirements", []),
                **({"next_checks": runtime_next_checks} if runtime_next_checks else {}),
            }
        )
    if mcp.get("status") != "fresh":
        blockers.append(
            {
                "surface": "mcp_freshness",
                "status": mcp.get("status"),
                "check": "cynic mcp-reload --dry-run",
                "action": mcp.get("canonical_action"),
                "mcp_dispatch": mcp.get("mcp_dispatch"),
                "proof_requirements": mcp.get("proof_requirements", []),
            }
        )
    if storage.get("status") != "active":
        blockers.append(
            {
                "surface": "storage_primary",
                "status": storage.get("status"),
                "missing": storage.get("missing", []),
                "check": "cynic storage status",
                "next_checks": _organism_next_checks(storage.get("next_checks", [])),
                "proof_requirements": storage.get("proof_requirements", []),
                "dependency_proofs": {
                    "bridge": bridge.get("proof_requirements", []),
                    "fleet": fleet.get("proof_requirements", []),
                    "mesh": mesh.get("proof_requirements", []),
                },
            }
        )
    if kernel.get("status") not in {"ok", "unknown"}:
        blockers.append(
            {
                "surface": "kernel_health",
                "status": kernel.get("status"),
                "check": "cynic organism --json",
                "endpoint": kernel.get("endpoint"),
                "error": kernel.get("error"),
                **({"diagnosis": kernel.get("diagnosis")} if kernel.get("diagnosis") else {}),
            }
        )
    for name, report in lifecycle.items():
        if report.get("status") != "ok":
            surface = f"lifecycle.{name}"
            blockers.append({"surface": surface, "status": report.get("status"), "check": _organism_blocker_check(surface)})
    if activation.get("status") not in {"ok", "ready"}:
        activation_actions = [
            _normalize_activation_action(item)
            for item in activation.get("activation_request_summary", [])
            if isinstance(item, dict)
        ]
        activation_dispatch_command = activation.get("activation_dispatch_command", "cynic activate")
        blockers.append(
            {
                "surface": "lifecycle_activation",
                "status": activation.get("status"),
                "activation_request_count": activation.get("activation_request_count"),
                "check": "cynic actions",
                "next_checks": ["cynic actions", "cynic activate --dry-run"],
                "proof_requirements": _activation_proof_requirements(
                    activation_actions,
                    activation,
                    activation_dispatch_command,
                ),
                "dispatch_plan": _activation_dispatch_plan(activation_actions, activation_dispatch_command),
            }
        )
    next_checks = _organism_next_checks(
        [
            blocker.get("check")
            for blocker in blockers
            if isinstance(blocker, dict)
        ]
        + [
            command
            for blocker in blockers
            if isinstance(blocker, dict) and isinstance(blocker.get("next_checks"), list)
            for command in blocker["next_checks"]
        ]
    )
    operator_sequence = _organism_operator_sequence(blockers, next_checks)
    next_checks = _order_next_checks_by_operator_sequence(next_checks, operator_sequence)
    return {
        "schema": "cynic.organism.signs_of_life.v1",
        "mutates": False,
        **attach_surface_contract({}, ORGANISM_SURFACE),
        "runtime": runtime,
        "stack": stack,
        "local_validation": local_validation,
        "kernel": kernel,
        "storage_primary": storage,
        "bridge": bridge,
        "fleet": fleet,
        "mesh": mesh,
        "mcp_freshness": mcp,
        "lifecycle": lifecycle,
        "lifecycle_activation": activation,
        "actions": {
            "schema": "cynic.organism.actions.v1",
            "mutates": False,
            "status": activation.get("status", "unknown"),
            "action_count": len(
                [
                    item
                    for item in activation.get("activation_request_summary", [])
                    if isinstance(item, dict)
                ]
            ),
            "dispatch_command": activation.get("activation_dispatch_command", "cynic activate"),
            "dispatch_mutates_mempool": True,
        },
        "contract": contract,
        "blockers": blockers,
        "next_checks": next_checks,
        "operator_sequence": operator_sequence,
    }


def organism_text(report: dict[str, object] | None = None) -> str:
    return _organism_text(report or organism_report())
