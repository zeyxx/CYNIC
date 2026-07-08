"""Registry-backed constellation view for lifecycle orchestration."""

from __future__ import annotations

from collections import defaultdict
from typing import Any


def _node_services(node: dict[str, Any]) -> list[str]:
    return [
        service.get("name")
        for service in node.get("services", []) or []
        if isinstance(service, dict) and service.get("name")
    ]


def _service_evidence(node: dict[str, Any], service: str) -> dict[str, Any]:
    evidence = node.get("activation_evidence", {}).get("services", {})
    value = evidence.get(service, {})
    return value if isinstance(value, dict) else {}


def _runtime_path_ready(node: dict[str, Any], path_name: str) -> bool:
    evidence = node.get("activation_evidence", {}).get("runtime_paths", {})
    value = evidence.get(path_name, {})
    return bool(isinstance(value, dict) and value.get("verified"))


def _node_lifecycle_role(node: dict[str, Any], lifecycle: dict[str, Any]) -> str | None:
    role = node.get("role")
    if role == lifecycle.get("build_runner_role"):
        return "build_runner"
    if role == lifecycle.get("deploy_target_role") and node.get(
        lifecycle.get("deploy_target_flag", "deploy_target"),
        False,
    ):
        return "deploy_target"
    if role in lifecycle.get("activation_requirements", {}):
        return str(role)
    return None


def _activation_status(node: dict[str, Any], lifecycle: dict[str, Any]) -> dict[str, Any]:
    role = node.get("role")
    requirement = lifecycle.get("activation_requirements", {}).get(role, {})
    service_status = {
        service: bool(_service_evidence(node, service).get("active"))
        for service in requirement.get("services", [])
    }
    runtime_path_status = {
        path_name: _runtime_path_ready(node, path_name)
        for path_name in requirement.get("filesystem", [])
    }
    network_status = {
        key: bool(node.get("network", {}).get(key))
        for key in requirement.get("network", [])
    }
    checks = [*service_status.values(), *runtime_path_status.values(), *network_status.values()]
    return {
        "required": bool(requirement),
        "ready": all(checks) if checks else bool(requirement),
        "network": network_status,
        "runtime_paths": runtime_path_status,
        "services": service_status,
    }


def build_lifecycle_constellation_report(registry: dict[str, Any]) -> dict[str, Any]:
    lifecycle = registry.get("lifecycle", {}).get("kernel_release", {})
    nodes = [
        node
        for node in registry.get("nodes", []) or []
        if isinstance(node, dict) and node.get("name")
    ]
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    lifecycle_nodes: list[dict[str, Any]] = []

    for node in nodes:
        constellation = node.get("constellation") or "unclassified"
        lifecycle_role = _node_lifecycle_role(node, lifecycle)
        item = {
            "name": node.get("name"),
            "aliases": node.get("aliases", []),
            "role": node.get("role"),
            "lifecycle_role": lifecycle_role,
            "status": node.get("status", "unknown"),
            "network": node.get("network", {}),
            "services": _node_services(node),
            "activation": _activation_status(node, lifecycle),
            "energy_profile": node.get("energy_profile", {}),
        }
        grouped[str(constellation)].append(item)
        if lifecycle_role:
            lifecycle_nodes.append({"constellation": constellation, **item})

    constellations = [
        {
            "name": name,
            "node_count": len(items),
            "active_nodes": sum(1 for item in items if item.get("status") == "active"),
            "roles": sorted({str(item.get("role")) for item in items if item.get("role")}),
            "nodes": sorted(items, key=lambda item: str(item.get("name"))),
        }
        for name, items in sorted(grouped.items())
    ]

    lifecycle_ready = all(
        node.get("status") == lifecycle.get("required_node_status", "active")
        and node.get("activation", {}).get("ready")
        for node in lifecycle_nodes
    )
    return {
        "schema": "cynic.lifecycle_constellation.v1",
        "status": "ok" if lifecycle_ready else "blocked",
        "lifecycle": "kernel_release",
        "orchestrator_organ": lifecycle.get("orchestrator_organ"),
        "required_node_status": lifecycle.get("required_node_status"),
        "constellation_count": len(constellations),
        "node_count": len(nodes),
        "lifecycle_nodes_ready": lifecycle_ready,
        "lifecycle_nodes": sorted(lifecycle_nodes, key=lambda item: str(item.get("name"))),
        "constellations": constellations,
    }
