"""Lifecycle activation evidence helpers for organ-forge."""

from __future__ import annotations

import json
import os
from copy import deepcopy
from datetime import datetime, timezone
from pathlib import Path

from organs.contracts.src.surface_contracts import json_object_or_raise, read_json_file_or_raise
from .project_root import resolve_project_root_from_script


def _default_registry_path() -> Path:
    return resolve_project_root_from_script(__file__) / "infra" / "registry.json"


def _load_registry(path: str | Path | None = None) -> dict:
    target_path = Path(path) if path else _default_registry_path()
    return read_json_file_or_raise(target_path, RuntimeError(f"failed to load registry: {target_path}"))


def _save_registry(path: str | Path, registry: dict) -> None:
    target_path = Path(path)
    tmp_path = target_path.with_name(f".{target_path.name}.tmp")
    tmp_path.write_text(json.dumps(registry, indent=2, ensure_ascii=False) + "\n")
    os.replace(tmp_path, target_path)


def build_lifecycle_service_evidence_patch(
    node_name: str,
    registry: dict | None = None,
    service_states: dict | None = None,
) -> dict:
    if not node_name:
        raise RuntimeError("node_name is required")
    registry = registry or _load_registry()
    from ..adapters.registry_adapter import validate_lifecycle_policy

    validate_lifecycle_policy(registry)
    lifecycle = registry["lifecycle"]["kernel_release"]
    node = next(
        (item for item in registry.get("nodes", []) if item.get("name") == node_name),
        None,
    )
    if not node:
        raise RuntimeError(f"node {node_name} is not in registry")
    role = node.get("role")
    required_services = (
        lifecycle.get("activation_requirements", {})
        .get(role, {})
        .get("services", [])
    )
    if not required_services:
        raise RuntimeError(f"node {node_name} has no lifecycle services")

    if service_states is None:
        service_states = {}
        env_payload = os.environ.get("CYNIC_LIFECYCLE_SERVICE_STATES_JSON")
        if not env_payload:
            env_payload = os.environ.get("CYNIC_SERVICE_STATES")
        if env_payload:
            try:
                service_states = json_object_or_raise(
                    env_payload,
                    RuntimeError("CYNIC_LIFECYCLE_SERVICE_STATES_JSON must be a JSON object"),
                )
            except RuntimeError as exc:
                if str(exc) == "CYNIC_LIFECYCLE_SERVICE_STATES_JSON must be a JSON object":
                    raise
                raise RuntimeError(
                    "CYNIC_LIFECYCLE_SERVICE_STATES_JSON must be a JSON object"
                ) from exc

    declared_services = {
        service.get("name")
        for service in node.get("services", [])
        if isinstance(service, dict)
    }
    service_states = service_states or {}
    normalized_service_states: dict[str, dict] = {}
    for service_name, state in service_states.items():
        if isinstance(state, dict):
            normalized_service_states[service_name] = state
            continue
        if isinstance(state, bool):
            normalized_service_states[service_name] = {"active": state}
            continue
        raise RuntimeError(
            f"service_states[{service_name!r}] must be an object or a boolean"
        )
    service_states = normalized_service_states
    checked_at = (
        datetime.now(timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z")
    )
    services = {}
    checks = []
    for service_name in required_services:
        state = service_states.get(service_name, {})
        declared = service_name in declared_services
        active = bool(state.get("active", False))
        checked_service_at = state.get("checked_at", checked_at)
        services[service_name] = {
            "declared": declared,
            "active": active,
            "checked_at": checked_service_at,
            "source": state.get("source", "lifecycle_service_evidence_patch"),
        }
        checks.append(
            {
                "node": node_name,
                "role": role,
                "service": service_name,
                "declared": declared,
                "active": active,
                "ok": declared and active,
            }
        )
    ok = all(check["ok"] for check in checks)
    return {
        "status": "ok" if ok else "blocked",
        "lifecycle": "kernel_release",
        "node": node_name,
        "role": role,
        "checks": checks,
        "activation_evidence_patch": {
            "node": node_name,
            "activation_evidence": {
                "services": services,
            },
        },
    }


def apply_lifecycle_activation_evidence_patch(
    patch: dict,
    registry: dict | None = None,
) -> dict:
    registry = registry or _load_registry()
    from ..adapters.registry_adapter import validate_lifecycle_policy

    validate_lifecycle_policy(registry)

    if not isinstance(patch, dict):
        raise RuntimeError("activation evidence patch must be a JSON object")
    wrapped_patch = patch.get("activation_evidence_patch")
    if "activation_evidence" not in patch and wrapped_patch:
        if not isinstance(wrapped_patch, dict):
            raise RuntimeError("activation_evidence_patch must be an object")
        patch = {
            "node": wrapped_patch.get("node", patch.get("node")),
            "activation_evidence": wrapped_patch.get("activation_evidence", {}),
        }
    node_name = patch.get("node")
    if not isinstance(node_name, str) or not node_name:
        raise RuntimeError("activation evidence patch node missing")

    node_index = next(
        (
            index
            for index, node in enumerate(registry.get("nodes", []))
            if node.get("name") == node_name
        ),
        None,
    )
    if node_index is None:
        raise RuntimeError(
            f"activation evidence patch node is not in registry: {node_name}"
        )

    lifecycle = registry["lifecycle"]["kernel_release"]
    node = registry["nodes"][node_index]
    role = node.get("role")
    required_paths = set(
        lifecycle.get("activation_requirements", {})
        .get(role, {})
        .get("filesystem", [])
    )
    evidence = patch.get("activation_evidence", {})
    runtime_paths = evidence.get("runtime_paths", {})
    services = evidence.get("services", {})
    if not isinstance(runtime_paths, dict):
        raise RuntimeError("activation evidence patch runtime_paths must be an object")
    if not isinstance(services, dict):
        raise RuntimeError("activation evidence patch services must be an object")
    if not runtime_paths and not services:
        raise RuntimeError("activation evidence patch runtime_paths or services missing")

    errors = []
    for path_name, path_evidence in runtime_paths.items():
        if path_name not in required_paths:
            errors.append(f"runtime path {path_name} is not required by node role {role}")
            continue
        if not isinstance(path_evidence, dict):
            errors.append(f"runtime path {path_name} evidence must be an object")
            continue
        expected_value = lifecycle.get("runtime_paths", {}).get(path_name)
        expected_proof = lifecycle.get("runtime_path_proofs", {}).get(path_name, {})
        if path_evidence.get("value") != expected_value:
            errors.append(f"runtime path {path_name} value does not match registry")
        if path_evidence.get("proof_kind") != expected_proof.get("kind"):
            errors.append(f"runtime path {path_name} proof_kind does not match registry")
        if not isinstance(path_evidence.get("verified"), bool):
            errors.append(f"runtime path {path_name} verified must be boolean")
        if not isinstance(path_evidence.get("marker_exists"), bool):
            errors.append(f"runtime path {path_name} marker_exists must be boolean")
        if not isinstance(path_evidence.get("sentinels_verified"), bool):
            errors.append(f"runtime path {path_name} sentinels_verified must be boolean")
        if not isinstance(path_evidence.get("verified_at"), str) or not path_evidence.get(
            "verified_at"
        ):
            errors.append(f"runtime path {path_name} verified_at missing")
        if path_evidence.get("verified") and (
            not path_evidence.get("marker_exists")
            or not path_evidence.get("sentinels_verified")
        ):
            errors.append(
                f"runtime path {path_name} cannot be verified without marker and sentinels"
            )
    required_services = set(
        lifecycle.get("activation_requirements", {})
        .get(role, {})
        .get("services", [])
    )
    declared_services = {
        service.get("name")
        for service in node.get("services", [])
        if isinstance(service, dict)
    }
    for service_name, service_evidence in services.items():
        if service_name not in required_services:
            errors.append(f"service {service_name} is not required by node role {role}")
            continue
        if service_name not in declared_services:
            errors.append(f"service {service_name} is not declared by node {node_name}")
            continue
        if not isinstance(service_evidence, dict):
            errors.append(f"service {service_name} evidence must be an object")
            continue
        if not isinstance(service_evidence.get("declared"), bool):
            errors.append(f"service {service_name} declared must be boolean")
        if not isinstance(service_evidence.get("active"), bool):
            errors.append(f"service {service_name} active must be boolean")
        if not isinstance(service_evidence.get("checked_at"), str) or not service_evidence.get(
            "checked_at"
        ):
            errors.append(f"service {service_name} checked_at missing")
        if service_evidence.get("active") and not service_evidence.get("declared"):
            errors.append(f"service {service_name} cannot be active without declared=true")
    if errors:
        raise RuntimeError("; ".join(errors))

    merged = deepcopy(registry)
    target = merged["nodes"][node_index]
    target.setdefault("activation_evidence", {}).setdefault("runtime_paths", {}).update(
        runtime_paths
    )
    target.setdefault("activation_evidence", {}).setdefault("services", {}).update(services)
    return {
        "status": "ok",
        "node": node_name,
        "role": role,
        "accepted_runtime_paths": sorted(runtime_paths),
        "accepted_services": sorted(services),
        "registry": merged,
    }


def persist_lifecycle_activation_evidence_patch(
    patch: dict,
    registry_path: str | Path | None = None,
) -> dict:
    target_path = Path(registry_path) if registry_path else _default_registry_path()
    registry = _load_registry(target_path)
    result = apply_lifecycle_activation_evidence_patch(patch, registry)
    _save_registry(target_path, result["registry"])
    return {
        "status": "ok",
        "registry_path": str(target_path),
        "node": result["node"],
        "role": result["role"],
        "accepted_runtime_paths": result["accepted_runtime_paths"],
        "accepted_services": result["accepted_services"],
    }


def activate_lifecycle_registry_node(
    node_name: str,
    registry: dict | None = None,
) -> dict:
    if not node_name:
        raise RuntimeError("node_name is required")
    registry = registry or _load_registry()
    from ..adapters.kernel_adapter import KernelRestClient
    from ..adapters.registry_adapter import validate_lifecycle_policy

    validate_lifecycle_policy(registry)
    lifecycle = registry["lifecycle"]["kernel_release"]
    required_status = lifecycle.get("required_node_status")
    if required_status != "active":
        raise RuntimeError("lifecycle required_node_status must be active")

    nodes = registry.get("nodes", [])
    node_index = next(
        (
            index
            for index, node in enumerate(nodes)
            if node.get("name") == node_name
        ),
        None,
    )
    if node_index is None:
        raise RuntimeError(f"node {node_name} is not in registry")
    node = nodes[node_index]
    lifecycle_roles = {
        lifecycle["build_runner_role"],
        lifecycle["deploy_target_role"],
    }
    role = node.get("role")
    if role not in lifecycle_roles:
        raise RuntimeError(f"node {node_name} is not part of lifecycle kernel_release")

    diagnostic = KernelRestClient.__new__(KernelRestClient)
    diagnostic.registry = registry
    diagnostic.lifecycle = lifecycle
    diagnostic.build_node = next(
        item for item in nodes if item.get("role") == lifecycle["build_runner_role"]
    )
    diagnostic.deploy_node = next(
        item
        for item in nodes
        if item.get("role") == lifecycle["deploy_target_role"]
        and item.get(lifecycle.get("deploy_target_flag", "deploy_target")) is True
    )
    node_gaps = [
        gap
        for gap in diagnostic._activation_evidence_gap_details()
        if gap.get("node") == node_name
    ]
    if node_gaps:
        raise RuntimeError(
            "node activation evidence is incomplete "
            f"({'; '.join(gap['code'] for gap in node_gaps)})"
        )

    activated_at = (
        datetime.now(timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z")
    )
    merged = deepcopy(registry)
    target = merged["nodes"][node_index]
    previous_status = target.get("status", "unknown")
    target["status"] = required_status
    target["status_updated_at"] = activated_at
    target["status_source"] = "lifecycle_node_activation"
    return {
        "status": "ok",
        "node": node_name,
        "role": role,
        "from_status": previous_status,
        "to_status": required_status,
        "registry": merged,
    }


def persist_lifecycle_registry_node_activation(
    node_name: str,
    registry_path: str | Path | None = None,
) -> dict:
    target_path = Path(registry_path) if registry_path else _default_registry_path()
    registry = _load_registry(target_path)
    result = activate_lifecycle_registry_node(node_name, registry)
    _save_registry(target_path, result["registry"])
    return {
        "status": "ok",
        "registry_path": str(target_path),
        "node": result["node"],
        "role": result["role"],
        "from_status": result["from_status"],
        "to_status": result["to_status"],
    }
