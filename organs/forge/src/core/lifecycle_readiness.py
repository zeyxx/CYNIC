"""Lifecycle readiness helpers for organ-forge."""

from __future__ import annotations

import os
import subprocess

from organs.contracts.src.surface_contracts import json_object_or_default

from .entities import FleetProbe


def build_activation_evidence_gap_details(
    lifecycle: dict,
    build_node: dict,
    deploy_node: dict,
    *,
    env: dict | None = None,
    extra_role_nodes: dict | None = None,
) -> list[dict]:
    env = env or os.environ
    requirements = lifecycle.get("activation_requirements", {})
    role_nodes = {
        lifecycle["build_runner_role"]: build_node,
        lifecycle["deploy_target_role"]: deploy_node,
    }
    role_nodes.update(extra_role_nodes or {})
    gaps = []
    for role, requirement in requirements.items():
        node = role_nodes.get(role)
        if not node:
            gaps.append(
                {
                    "code": f"{role}:node_not_resolved",
                    "kind": "node",
                    "role": role,
                    "node": None,
                    "requirement": "resolved_node",
                }
            )
            continue
        network = node.get("network", {})
        for network_key in requirement.get("network", []):
            value = network.get(network_key)
            if not value:
                gaps.append(
                    {
                        "code": f"{node.get('name')}:{network_key}_missing",
                        "kind": "network",
                        "role": role,
                        "node": node.get("name"),
                        "requirement": network_key,
                        "reason": "missing_registry_value",
                    }
                )
                continue
            if isinstance(value, str) and value.startswith("<") and value.endswith(">"):
                env_name = value[1:-1]
                if not env.get(env_name):
                    gaps.append(
                        {
                            "code": f"{node.get('name')}:{network_key}_env_{env_name}_missing",
                            "kind": "network",
                            "role": role,
                            "node": node.get("name"),
                            "requirement": network_key,
                            "reason": "missing_env",
                            "env": env_name,
                        }
                    )
        service_names = {
            service.get("name")
            for service in node.get("services", [])
            if isinstance(service, dict)
        }
        node_service_evidence = (
            node.get("activation_evidence", {})
            .get("services", {})
        )
        for service_name in requirement.get("services", []):
            if service_name not in service_names:
                gaps.append(
                    {
                        "code": f"{node.get('name')}:service_{service_name}_missing",
                        "kind": "service",
                        "role": role,
                        "node": node.get("name"),
                        "requirement": service_name,
                        "reason": "missing_registry_service",
                    }
                )
                continue
            evidence = node_service_evidence.get(service_name, {})
            if not evidence.get("active"):
                gaps.append(
                    {
                        "code": f"{node.get('name')}:service_{service_name}_inactive",
                        "kind": "service",
                        "role": role,
                        "node": node.get("name"),
                        "requirement": service_name,
                        "reason": "missing_service_evidence",
                    }
                )
        runtime_paths = lifecycle.get("runtime_paths", {})
        runtime_path_proofs = lifecycle.get("runtime_path_proofs", {})
        node_runtime_evidence = (
            node.get("activation_evidence", {})
            .get("runtime_paths", {})
        )
        for path_name in requirement.get("filesystem", []):
            evidence = node_runtime_evidence.get(path_name, {})
            if not evidence.get("verified"):
                gaps.append(
                    {
                        "code": f"{node.get('name')}:runtime_path_{path_name}_unverified",
                        "kind": "filesystem",
                        "role": role,
                        "node": node.get("name"),
                        "requirement": path_name,
                        "reason": "missing_runtime_path_proof",
                        "path": runtime_paths.get(path_name),
                        "proof": runtime_path_proofs.get(path_name, {}),
                    }
                )
    return gaps


def activation_evidence_gaps(
    lifecycle: dict,
    build_node: dict,
    deploy_node: dict,
    *,
    env: dict | None = None,
    extra_role_nodes: dict | None = None,
) -> list[str]:
    return [
        gap["code"]
        for gap in build_activation_evidence_gap_details(
            lifecycle,
            build_node,
            deploy_node,
            env=env,
            extra_role_nodes=extra_role_nodes,
        )
    ]


def inspect_fleet(
    build_target: str,
    deploy_target: str,
    *,
    timeout: int = 10,
) -> dict:
    target_names = {build_target, deploy_target}
    try:
        result = subprocess.run(
            ["synapse", "status", "--json"],
            check=False,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    except FileNotFoundError:
        return FleetProbe.from_dict({"enabled": True, "status": "degraded", "error": "synapse_cli_unavailable", "targets": sorted(target_names)})
    except subprocess.TimeoutExpired:
        return FleetProbe.from_dict({"enabled": True, "status": "error", "error": "synapse_status_timeout", "targets": sorted(target_names)})
    except OSError as exc:
        return FleetProbe.from_dict({"enabled": True, "status": "error", "error": type(exc).__name__, "targets": sorted(target_names)})
    if result.returncode != 0:
        return FleetProbe.from_dict({"enabled": True, "status": "error", "error": "synapse_status_failed", "targets": sorted(target_names)})
    status = json_object_or_default(result.stdout)
    if not status:
        return FleetProbe.from_dict({"enabled": True, "status": "error", "error": "synapse_json_invalid", "targets": sorted(target_names)})
    peers = list((status.get("Peer") or {}).values())
    self_node = status.get("Self")
    if isinstance(self_node, dict):
        peers.append(self_node)
    by_name = {}
    for peer in peers:
        names = {
            str(peer.get("HostName", "")).lower(),
            str(peer.get("DNSName", "")).split(".", 1)[0].lower(),
        }
        for name in names:
            if name:
                by_name[name] = peer
    matches = {}
    missing = []
    for target in sorted(target_names):
        peer = by_name.get(target.lower())
        if not peer:
            missing.append(target)
            continue
        matches[target] = {
            "online": bool(peer.get("Online")),
            "os": peer.get("OS", "unknown"),
            "name": peer.get("HostName") or peer.get("DNSName", "").split(".", 1)[0],
        }
    return FleetProbe.from_dict({
        "enabled": True,
        "status": "ok",
        "missing": missing,
        "matches": matches,
        "targets": sorted(target_names),
        "mesh_running": status.get("BackendState") == "Running"
    })


def fleet_nodes_ready(
    lifecycle: dict,
    fleet_probe: dict,
    fleet_inventory_probe: dict,
) -> bool:
    requirements = lifecycle.get("fleet_requirements", {})
    if not requirements.get("required", False):
        return True
    if not fleet_inventory_probe:
        return False
    if fleet_inventory_probe.get("status") != "ok":
        return False
    summary = fleet_inventory_probe.get("summary", {})
    if isinstance(summary, dict):
        if summary.get("missing_nodes", 0) > 0 or summary.get("unexpected_nodes", 0) > 0:
            return False
    if requirements.get("provider") == "organ-fleet":
        return True
    if fleet_probe.get("status") != "ok":
        return False
    if fleet_probe.get("missing", []):
        return False
    if requirements.get("require_online", True):
        return all(
            peer.get("online") is True
            for peer in fleet_probe.get("matches", {}).values()
        )
    return True


def lifecycle_blockers(
    lifecycle: dict,
    build_node: dict,
    deploy_node: dict,
    *,
    activation_gap_details: list[dict],
    required_status: str | None,
    fleet_probe: dict,
    fleet_inventory_probe: dict,
    status_nodes: list[dict] | None = None,
) -> list[dict]:
    blockers = [
        {"category": "activation", **gap}
        for gap in activation_gap_details
    ]
    if required_status:
        nodes = status_nodes if status_nodes is not None else [build_node, deploy_node]
        seen_nodes = set()
        for node in nodes:
            node_name = node.get("name")
            if node_name in seen_nodes:
                continue
            seen_nodes.add(node_name)
            actual = node.get("status", "unknown")
            if actual != required_status:
                blockers.append(
                    {
                        "category": "status",
                        "kind": "node_status",
                        "node": node_name,
                        "role": node.get("role"),
                        "required_status": required_status,
                        "actual_status": actual,
                        "code": f"{node_name}:status_{actual}_not_{required_status}",
                    }
                )
    fleet_requirements = lifecycle.get("fleet_requirements", {})
    if fleet_requirements.get("required", False):
        provider = fleet_requirements.get("provider", "synapse")
        if provider == "synapse":
            fleet_status = fleet_probe.get("status", "not_run")
            if fleet_status != "ok":
                blocker = {
                    "category": "fleet",
                    "kind": "synapse_probe",
                    "provider": "synapse",
                    "required": "ok",
                    "actual": fleet_status,
                    "error": fleet_probe.get("error"),
                    "code": f"fleet_probe:{fleet_status}",
                    "resolver_organ": "organ-wireguard",
                    "resolver_consumer": "organ-forge-hermes-agent",
                }
                if fleet_probe.get("error") == "synapse_cli_unavailable":
                    blocker.update(
                        {
                            "operator_action": "activate organ-wireguard / synapse CLI on the registry build runner environment",
                            "command": "python3 organs/wireguard/src/entrypoints/cli.py sync-dns --registry infra/registry.json",
                        }
                    )
                blockers.append(blocker)
            for node_name in fleet_probe.get("missing", []):
                blockers.append(
                    {
                        "category": "fleet",
                        "kind": "synapse_peer",
                        "node": node_name,
                        "required": "present",
                        "actual": "missing",
                        "code": f"{node_name}:fleet_missing",
                    }
                )
            if fleet_requirements.get("require_online", True):
                for node_name, peer in fleet_probe.get("matches", {}).items():
                    if not peer.get("online"):
                        blockers.append(
                            {
                                "category": "fleet",
                                "kind": "synapse_peer",
                                "node": node_name,
                                "required": "online",
                                "actual": "offline",
                                "code": f"{node_name}:fleet_offline",
                            }
                        )
        inventory_status = fleet_inventory_probe.get("status", "not_run")
        if inventory_status != "ok":
            blockers.append(
                {
                    "category": "fleet",
                    "kind": "fleet_inventory",
                    "provider": "organ-fleet",
                    "required": "ok",
                    "actual": inventory_status,
                    "error": fleet_inventory_probe.get("error"),
                    "code": f"fleet_inventory:{inventory_status}",
                    "resolver_organ": "organ-fleet",
                    "resolver_consumer": "organ-forge-hermes-agent",
                    "command": "python3 organs/fleet/src/entrypoints/cli.py report --json --registry infra/registry.json",
                }
            )
        summary = fleet_inventory_probe.get("summary", {})
        if isinstance(summary, dict):
            for gap in summary.get("provider_evidence_gaps", []):
                blockers.append(
                    {
                        "category": "fleet",
                        "kind": "fleet_inventory_evidence_gap",
                        "provider": "fleet_inventory",
                        "code": str(gap),
                    }
                )
            for node_name in summary.get("declaration_missing_nodes", []):
                blockers.append(
                    {
                        "category": "fleet",
                        "kind": "fleet_inventory_node",
                        "node": str(node_name),
                        "required": "declared",
                        "actual": "missing",
                        "code": f"{node_name}:fleet_inventory_missing",
                    }
                )
            for node_name in summary.get("declaration_unexpected_nodes", []):
                blockers.append(
                    {
                        "category": "fleet",
                        "kind": "fleet_inventory_node",
                        "node": str(node_name),
                        "required": "declared",
                        "actual": "unexpected",
                        "code": f"{node_name}:fleet_inventory_unexpected",
                    }
                )
    return blockers
