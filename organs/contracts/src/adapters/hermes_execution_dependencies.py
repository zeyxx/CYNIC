"""Aggregate Hermes execution dependencies across fleet inference and Synapse."""

from __future__ import annotations

from pathlib import Path
import json
import os
import subprocess
import sys
from typing import Any, Callable

from organs.contracts.src.adapters.kernel_http import kernel_json_request, normalize_kernel_base_url
from organs.contracts.src.adapters.hermes_fleet_backend_readiness import (
    probe_hermes_fleet_backend_readiness,
)
from organs.contracts.src.core.agent_framework_capabilities import utc_now_iso


HERMES_EXECUTION_DEPENDENCIES_SCHEMA = "cynic.hermes_execution_dependencies.v1"
HERMES_EXECUTION_DEPENDENCY_TASKS_SCHEMA = "cynic.hermes_execution_dependency_tasks.v1"
HERMES_EXECUTION_DEPENDENCY_DISPATCH_SCHEMA = "cynic.hermes_execution_dependency_dispatch.v1"


ReportProvider = Callable[[], dict[str, Any]]


def evaluate_hermes_execution_dependencies(
    *,
    backends_toml: Path,
    registry_json: Path | None = None,
    expected_backend_id: str = "qwen36-27b-gpu",
    timeout_secs: float = 5.0,
    observed_at: str | None = None,
    backend_readiness_provider: ReportProvider | None = None,
    dns_activation_status_provider: ReportProvider | None = None,
    executor_liveness_provider: ReportProvider | None = None,
) -> dict[str, Any]:
    backend = (
        backend_readiness_provider()
        if backend_readiness_provider
        else probe_hermes_fleet_backend_readiness(
            backends_toml=backends_toml,
            registry_json=registry_json,
            expected_backend_id=expected_backend_id,
            timeout_secs=timeout_secs,
            observed_at=observed_at,
        )
    )
    dns_status = (
        dns_activation_status_provider()
        if dns_activation_status_provider
        else _wireguard_dns_activation_status(resolve_dns=True)
    )
    executor_liveness = (
        executor_liveness_provider()
        if executor_liveness_provider
        else _hermes_executor_liveness()
    )
    blockers = _blockers(backend, dns_status, executor_liveness)
    return {
        "schema": HERMES_EXECUTION_DEPENDENCIES_SCHEMA,
        "observed_at": observed_at or utc_now_iso(),
        "mutates": False,
        "status": "ready" if not blockers else "blocked",
        "backend_id": expected_backend_id,
        "dependencies": {
            "fleet_backend_readiness": _summary(
                backend,
                keys=("schema", "status", "backend_id", "base_url", "expected_model", "violations", "next_actions"),
            ),
            "synapse_dns_activation": _summary(
                dns_status,
                keys=("schema", "status", "missing", "checks", "provider_status", "surface_contract"),
            ),
            "hermes_executor_liveness": executor_liveness,
        },
        "blockers": blockers,
        "next_actions": _next_actions(backend, dns_status, executor_liveness, blockers),
    }


def evaluate_hermes_execution_dependencies_from_args(args: list[str]) -> dict[str, Any]:
    backends_toml = Path("backends.toml")
    registry_json: Path | None = Path("infra/registry.json")
    expected_backend = "qwen36-27b-gpu"
    timeout_secs = 5.0
    index = 0
    while index < len(args):
        if args[index] == "--backends-toml" and index + 1 < len(args):
            backends_toml = Path(args[index + 1]).expanduser()
            index += 2
            continue
        if args[index] == "--registry" and index + 1 < len(args):
            registry_json = Path(args[index + 1]).expanduser()
            index += 2
            continue
        if args[index] == "--expected-backend" and index + 1 < len(args):
            expected_backend = args[index + 1]
            index += 2
            continue
        if args[index] == "--timeout-secs" and index + 1 < len(args):
            timeout_secs = float(args[index + 1])
            index += 2
            continue
        raise RuntimeError(
            "usage: hermes-execution-dependencies [--backends-toml backends.toml] [--registry infra/registry.json] [--expected-backend qwen36-27b-gpu] [--timeout-secs 5]"
        )
    return evaluate_hermes_execution_dependencies(
        backends_toml=backends_toml,
        registry_json=registry_json if registry_json and registry_json.exists() else None,
        expected_backend_id=expected_backend,
        timeout_secs=timeout_secs,
    )


def materialize_hermes_execution_dependency_tasks(
    *,
    dependencies: dict[str, Any],
    agent_id: str = "cortex-hermes-dependency-materializer",
    observed_at: str | None = None,
) -> dict[str, Any]:
    tasks = []
    for owner, actions in _actions_by_owner(dependencies.get("next_actions", [])).items():
        if owner not in {"organ-fleet", "cynic-host-gpu", "hermes-agent-executor"}:
            continue
        objective = _objective_for_owner(owner, actions)
        content = {
            "schema": "cynic.hermes_dependency_remediation_task.v1",
            "objective": objective,
            "source_schema": dependencies.get("schema"),
            "source_status": dependencies.get("status"),
            "blocked_backend": dependencies.get("backend_id"),
            "owner": owner,
            "actions": actions,
            "done_when": sorted({item.get("done_when", "") for item in actions if item.get("done_when")}),
            "proof_commands": _proof_commands(owner),
        }
        tasks.append(
            {
                "kind": "hermes",
                "domain": owner,
                "agent_id": agent_id,
                "content": json.dumps(content, sort_keys=True, separators=(",", ":")),
                "objective": objective,
            }
        )
    return {
        "schema": HERMES_EXECUTION_DEPENDENCY_TASKS_SCHEMA,
        "observed_at": observed_at or utc_now_iso(),
        "mutates": False,
        "source_schema": dependencies.get("schema"),
        "source_status": dependencies.get("status"),
        "task_count": len(tasks),
        "tasks": tasks,
    }


def materialize_hermes_execution_dependency_tasks_from_args(args: list[str]) -> dict[str, Any]:
    dependencies = evaluate_hermes_execution_dependencies_from_args(args)
    return materialize_hermes_execution_dependency_tasks(dependencies=dependencies)


def dispatch_hermes_execution_dependency_tasks(
    *,
    task_report: dict[str, Any],
    dispatcher: Any,
    dispatch: bool = False,
    observed_at: str | None = None,
) -> dict[str, Any]:
    results = []
    for task in task_report.get("tasks", []) if isinstance(task_report.get("tasks"), list) else []:
        if not isinstance(task, dict):
            continue
        if not dispatch:
            results.append(
                {
                    "status": "dry_run",
                    "kind": task.get("kind"),
                    "domain": task.get("domain"),
                    "objective": task.get("objective"),
                }
            )
            continue
        ok = bool(dispatcher.dispatch_task(task))
        results.append(
            {
                "status": "dispatched" if ok else "dispatch_failed",
                "kind": task.get("kind"),
                "domain": task.get("domain"),
                "objective": task.get("objective"),
            }
        )
    return {
        "schema": HERMES_EXECUTION_DEPENDENCY_DISPATCH_SCHEMA,
        "observed_at": observed_at or utc_now_iso(),
        "mutates": bool(dispatch),
        "mode": "dispatch" if dispatch else "dry_run",
        "source_schema": task_report.get("schema"),
        "task_count": len(results),
        "results": results,
        "status": "ok" if all(item["status"] in {"dry_run", "dispatched"} for item in results) else "failed",
    }


def dispatch_hermes_execution_dependency_tasks_from_args(args: list[str]) -> dict[str, Any]:
    dispatch = False
    filtered: list[str] = []
    for arg in args:
        if arg == "--dispatch":
            dispatch = True
        elif arg == "--dry-run":
            dispatch = False
        else:
            filtered.append(arg)
    task_report = materialize_hermes_execution_dependency_tasks_from_args(filtered)
    from organs.contracts.src.hermes_dispatch import HermesDispatcher

    return dispatch_hermes_execution_dependency_tasks(
        task_report=task_report,
        dispatcher=HermesDispatcher("hermes-execution-dependency-dispatch"),
        dispatch=dispatch,
    )


def _wireguard_dns_activation_status(*, resolve_dns: bool) -> dict[str, Any]:
    repo_root = _repo_root()
    cmd = [
        sys.executable,
        str(repo_root / "organs/fleet/src/entrypoints/cli.py"),
        "dns",
        "activation-status",
    ]
    if resolve_dns:
        cmd.append("--resolve-dns")
    result = subprocess.run(cmd, cwd=repo_root, text=True, capture_output=True, timeout=30, check=False)
    if not result.stdout.strip():
        return {
            "schema": "cynic.organ_fleet.dns_activation_status.v1",
            "status": "unavailable",
            "error": result.stderr[:1000],
        }
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        return {
            "schema": "cynic.organ_fleet.dns_activation_status.v1",
            "status": "invalid_json",
            "error": str(exc),
            "stdout": result.stdout[:1000],
        }


def _hermes_executor_liveness() -> dict[str, Any]:
    service_id = os.environ.get("HERMES_EXECUTOR_SERVICE_ID", "hermes-agent-executor")
    raw_addr = os.environ.get("CYNIC_REST_ADDR", "").strip()
    api_key = os.environ.get("CYNIC_API_KEY", "") or os.environ.get("CYNIC_ORGAN_KEY", "")
    if not raw_addr:
        return {
            "schema": "cynic.hermes_executor_liveness.v1",
            "status": "unknown",
            "service_id": service_id,
            "reason": "CYNIC_REST_ADDR_missing",
        }
    try:
        base_url = normalize_kernel_base_url(raw_addr, allow_empty=False)
    except RuntimeError as exc:
        return {
            "schema": "cynic.hermes_executor_liveness.v1",
            "status": "unknown",
            "service_id": service_id,
            "reason": str(exc),
        }

    status, text, payload = kernel_json_request("GET", base_url, "/services", api_key, timeout=10)
    if status != 200:
        return {
            "schema": "cynic.hermes_executor_liveness.v1",
            "status": "unknown",
            "service_id": service_id,
            "http_status": status,
            "error": text[:500],
        }

    services = payload if isinstance(payload, list) else payload.get("services", []) if isinstance(payload, dict) else []
    if not isinstance(services, list):
        services = []
    matching = [item for item in services if isinstance(item, dict) and item.get("service_id") == service_id]
    return {
        "schema": "cynic.hermes_executor_liveness.v1",
        "status": "registered" if matching else "not_registered",
        "service_id": service_id,
        "observed_services": len(services),
        "matching_services": matching,
    }


def _repo_root() -> Path:
    start = Path(__file__).resolve()
    for candidate in (start.parent, *start.parents):
        if (candidate / "organs").is_dir() and (candidate / "infra").is_dir():
            return candidate
    return Path.cwd()


def _summary(payload: dict[str, Any], *, keys: tuple[str, ...]) -> dict[str, Any]:
    return {key: payload.get(key) for key in keys if key in payload}


def _blockers(
    backend: dict[str, Any],
    dns_status: dict[str, Any],
    executor_liveness: dict[str, Any],
) -> list[dict[str, Any]]:
    blockers: list[dict[str, Any]] = []
    if executor_liveness.get("status") == "not_registered":
        blockers.append(
            {
                "kind": "hermes_executor_not_registered",
                "status": str(executor_liveness.get("status")),
                "service_id": str(executor_liveness.get("service_id", "hermes-agent-executor")),
            }
        )
    if backend.get("status") != "ready":
        blockers.append(
            {
                "kind": "fleet_backend_not_ready",
                "status": str(backend.get("status")),
                "violations": [item.get("kind") for item in backend.get("violations", []) if isinstance(item, dict)],
            }
        )
    if dns_status.get("status") != "active":
        blockers.append(
            {
                "kind": "synapse_dns_activation_not_active",
                "status": str(dns_status.get("status")),
                "missing": list(dns_status.get("missing", [])) if isinstance(dns_status.get("missing"), list) else [],
            }
        )
    return blockers


def _next_actions(
    backend: dict[str, Any],
    dns_status: dict[str, Any],
    executor_liveness: dict[str, Any],
    blockers: list[dict[str, Any]],
) -> list[dict[str, str]]:
    actions: list[dict[str, str]] = []
    if executor_liveness.get("status") == "not_registered":
        actions.append(
            {
                "owner": "hermes-agent-executor",
                "action": "start_or_repair_hermes_agent_executor",
                "command": "systemctl --user status hermes-agent-executor.service --no-pager || systemctl status hermes-agent-executor.service --no-pager",
                "done_when": "/services lists hermes-agent-executor as registered",
            }
        )
    missing = set(dns_status.get("missing", [])) if isinstance(dns_status.get("missing"), list) else set()
    if "canonical_dns_provider_available" in missing or "coredns_binary_available" in missing:
        actions.append(
            {
                "owner": "organ-fleet",
                "action": "install_canonical_coredns_provider",
                "command": "python3 organs/fleet/src/entrypoints/cli.py dns plan",
                "done_when": "dns activation-status reports canonical_dns_provider_available=true",
            }
        )
    if {"hosts_exists", "corefile_exists", "service_exists", "resolved_dropin_exists"} & missing:
        actions.append(
            {
                "owner": "organ-fleet",
                "action": "install_synapse_dns_activation_bundle",
                "command": "python3 organs/fleet/src/entrypoints/cli.py dns bundle --registry infra/registry.json --output-dir /tmp/cynic-synapse-dns",
                "done_when": "dns activation-status reports hosts/corefile/service/resolved_dropin present",
            }
        )
    for action in backend.get("next_actions", []):
        if isinstance(action, dict) and isinstance(action.get("action"), str):
            actions.append({str(key): str(value) for key, value in action.items() if isinstance(value, str)})
    if blockers and not actions:
        actions.append(
            {
                "owner": "cortex",
                "action": "inspect_hermes_dependency_blockers",
                "command": "python3 organs/contracts/src/entrypoints/cli.py hermes-execution-dependencies",
                "done_when": "status is ready",
            }
        )
    return actions


def _actions_by_owner(actions: Any) -> dict[str, list[dict[str, str]]]:
    grouped: dict[str, list[dict[str, str]]] = {}
    if not isinstance(actions, list):
        return grouped
    for action in actions:
        if not isinstance(action, dict):
            continue
        owner = str(action.get("owner", "")).strip()
        name = str(action.get("action", "")).strip()
        if not owner or not name:
            continue
        grouped.setdefault(owner, []).append(
            {str(key): str(value) for key, value in action.items() if isinstance(value, str)}
        )
    return grouped


def _objective_for_owner(owner: str, actions: list[dict[str, str]]) -> str:
    names = ", ".join(action["action"] for action in actions if action.get("action"))
    if owner == "organ-fleet":
        return f"Restore Synapse DNS and mesh prerequisites for sovereign Hermes execution: {names}"
    if owner == "cynic-host-gpu":
        return f"Restore sovereign GPU inference backend for Hermes execution: {names}"
    if owner == "hermes-agent-executor":
        return f"Restore Hermes Agent executor liveness for CYNIC task consumption: {names}"
    return f"Restore Hermes execution dependency for {owner}: {names}"


def _proof_commands(owner: str) -> list[str]:
    if owner == "organ-fleet":
        return [
            "python3 organs/fleet/src/entrypoints/cli.py dns activation-status --resolve-dns",
            "python3 organs/fleet/src/entrypoints/cli.py mesh status --resolve-dns --include-records",
            "python3 organs/contracts/src/entrypoints/cli.py hermes-execution-dependencies --backends-toml backends.toml --registry infra/registry.json --expected-backend qwen36-27b-gpu --timeout-secs 2",
        ]
    if owner == "cynic-host-gpu":
        return [
            "curl -fsS http://cynic-host-gpu.synapse.cynic:8080/v1/models",
            "python3 organs/contracts/src/entrypoints/cli.py hermes-execution-dependencies --backends-toml backends.toml --registry infra/registry.json --expected-backend qwen36-27b-gpu --timeout-secs 2",
        ]
    if owner == "hermes-agent-executor":
        return [
            "python3 organs/contracts/src/entrypoints/cli.py hermes-execution-dependencies --backends-toml backends.toml --registry infra/registry.json --expected-backend qwen36-27b-gpu --timeout-secs 2",
            "python3 organs/contracts/src/entrypoints/cli.py hermes-execution-dependency-tasks --backends-toml backends.toml --registry infra/registry.json --expected-backend qwen36-27b-gpu --timeout-secs 2",
        ]
    return [
        "python3 organs/contracts/src/entrypoints/cli.py hermes-execution-dependencies --backends-toml backends.toml --registry infra/registry.json --expected-backend qwen36-27b-gpu --timeout-secs 2",
    ]
