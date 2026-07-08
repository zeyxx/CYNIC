"""Topology metabolism for the L0-L7 organism stack."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from organs.supervisor.src.core.organism_stack_utils import (
    is_ignored_duplicate_scan_path,
    read_text,
    read_text_without_docstrings,
    rel,
)


def topology_summary(entities: list[dict[str, object]]) -> dict[str, object]:
    by_kind: dict[str, int] = {}
    by_layer: dict[str, int] = {}
    unhealthy_by_layer: dict[str, int] = {}
    for entity in entities:
        kind = str(entity.get("kind") or "unknown")
        layer = str(entity.get("layer") or "unknown")
        by_kind[kind] = by_kind.get(kind, 0) + 1
        by_layer[layer] = by_layer.get(layer, 0) + 1
        if entity.get("health") in {"blocked", "degraded"}:
            unhealthy_by_layer[layer] = unhealthy_by_layer.get(layer, 0) + 1
    return {
        "by_layer": dict(sorted(by_layer.items())),
        "unhealthy_by_layer": dict(sorted(unhealthy_by_layer.items())),
        "domain_topologies": by_kind.get("domain_topology", 0),
        "kernel_surfaces": by_kind.get("kernel_surface_topology", 0),
        "node_role_topologies": by_kind.get("node_role_topology", 0),
        "async_muscles": by_kind.get("async_muscle", 0),
        "supply_chains": by_kind.get("supply_chain_topology", 0),
        "recursion_contracts": by_kind.get("recursion_contract", 0),
    }


def node_entities(registry: dict[str, Any]) -> list[dict[str, object]]:
    entities: list[dict[str, object]] = []
    for node in registry.get("nodes", []) if isinstance(registry.get("nodes"), list) else []:
        if not isinstance(node, dict):
            continue
        entities.append(
            {
                "layer": "L0",
                "entity": str(node.get("name") or "unknown-node"),
                "kind": "node",
                "desired": str(node.get("role") or "declared"),
                "observed": str(node.get("status") or node.get("status_source") or "registry_declared"),
                "health": "ok" if node.get("status") == "active" else "unknown",
                "owner": "infra/registry.json",
                "next_action": "cynic fleet status",
            }
        )
    return entities


def node_role_topology_entities(registry: dict[str, Any]) -> list[dict[str, object]]:
    roles: dict[str, list[str]] = {}
    for node in registry.get("nodes", []) if isinstance(registry.get("nodes"), list) else []:
        if not isinstance(node, dict):
            continue
        role = str(node.get("role") or "unknown")
        roles.setdefault(role, []).append(str(node.get("name") or "unknown-node"))
    return [
        {
            "layer": "L0",
            "entity": f"node-role:{role}",
            "kind": "node_role_topology",
            "desired": "role has explicit registry nodes and no hidden placement",
            "observed": ",".join(sorted(names)),
            "health": "ok" if names else "degraded",
            "owner": "infra/registry.json:nodes",
            "next_action": "cynic fleet status",
        }
        for role, names in sorted(roles.items())
    ]


def synapse_entities(registry: dict[str, Any]) -> list[dict[str, object]]:
    network_policy = registry.get("network_policy", {}) if isinstance(registry.get("network_policy"), dict) else {}
    desired = "synapse mesh and registry IPAM"
    if network_policy:
        desired = str(network_policy.get("description") or desired)
    return [
        {
            "layer": "L2",
            "entity": "synapse-mesh",
            "kind": "mesh",
            "desired": desired,
            "observed": "requires mesh probe",
            "health": "unknown",
            "owner": "infra/registry.json:network_policy",
            "next_action": "cynic mesh status",
        }
    ]


def kernel_surface_entities(root: Path) -> list[dict[str, object]]:
    surfaces = [
        {
            "entity": "kernel-surface:/agent-tasks",
            "desired": "durable async task mempool with dispatch, claim, result and pending/completed read-back",
            "owner": "organs/contracts/src/adapters/agent_tasks.py",
            "next_action": "cynic list-pending-agent-tasks",
            "proof_files": agent_task_contract_files(root),
            "capabilities": ["dispatch", "list_pending", "list_completed", "claim", "result", "read_back"],
        },
        {
            "entity": "kernel-surface:/observe",
            "desired": "shared sensory observation intake through one observation transport port",
            "owner": "organs/contracts/src/adapters/kernel_observe.py",
            "next_action": "cynic observe --dry-run",
            "proof_files": observe_contract_files(root),
        },
        {
            "entity": "kernel-surface:/judge",
            "desired": "kernel judgment requests use the same Kernel HTTP membrane as other cortex/organ calls",
            "owner": "organs/contracts/src/adapters/kernel_http.py",
            "next_action": "cynic health",
            "proof_files": kernel_surface_contract_files(root, "/judge"),
        },
        {
            "entity": "kernel-surface:/health",
            "desired": "health/readiness probes use the same Kernel HTTP membrane and do not silently target localhost",
            "owner": "organs/contracts/src/adapters/kernel_http.py",
            "next_action": "cynic health",
            "proof_files": kernel_surface_contract_files(root, "/health"),
        },
        {
            "entity": "kernel-surface:/coord",
            "desired": "registration, claim, who and release endpoints synchronize cortex and organs through one coordination bus",
            "owner": "organs/contracts/src/hermes_dispatch.py",
            "next_action": "cynic coord who",
            "proof_files": kernel_surface_contract_files(root, "/coord/"),
        },
    ]
    return [
        {
            "layer": "L3",
            "entity": item["entity"],
            "kind": "kernel_surface_topology",
            "desired": item["desired"],
            "observed": f"contract_files={len(item['proof_files'])}",
            "health": "ok" if item["proof_files"] else "degraded",
            "owner": item["owner"],
            "next_action": item["next_action"],
            "proof": {
                "sample_contract_files": item["proof_files"][:12],
                "capabilities": item.get("capabilities", []),
            },
        }
        for item in surfaces
    ]


def domain_topology_entities(manifests: list[dict[str, object]]) -> list[dict[str, object]]:
    domains: dict[str, dict[str, list[str]]] = {}
    for manifest in manifests:
        domain = str(manifest.get("domain") or "").strip()
        if not domain:
            continue
        role = str(manifest.get("domain_role") or "primary").lower()
        name = str(manifest.get("name"))
        topology = domains.setdefault(domain, {"primary": [], "sensor": [], "consumer": [], "adapter": [], "other": []})
        topology.setdefault(role if role in topology else "other", []).append(name)
    entities: list[dict[str, object]] = []
    for domain, topology in sorted(domains.items()):
        primaries = topology.get("primary", [])
        observed = ", ".join(
            f"{role}={','.join(sorted(names)) or 'none'}"
            for role, names in topology.items()
            if names or role == "primary"
        )
        entities.append(
            {
                "layer": "L4",
                "entity": f"domain:{domain}",
                "kind": "domain_topology",
                "desired": "one primary owner; secondary organs declare sensor/consumer/adapter roles",
                "observed": observed,
                "health": "ok" if len(primaries) == 1 else "degraded",
                "owner": "organs/*/MANIFEST.yaml:domain",
                "next_action": "cynic converge --json",
                "topology": topology,
            }
        )
    return entities


def async_muscle_entities(root: Path, registry: dict[str, Any]) -> list[dict[str, object]]:
    lifecycle = registry.get("lifecycle", {}) if isinstance(registry.get("lifecycle"), dict) else {}
    kernel_release = lifecycle.get("kernel_release", {}) if isinstance(lifecycle.get("kernel_release"), dict) else {}
    profiles = kernel_release.get("request_profiles", {}) if isinstance(kernel_release.get("request_profiles"), dict) else {}
    profile_names = sorted(str(name) for name in profiles)
    async_files = async_contract_files(root)
    agent_task_files = agent_task_contract_files(root)
    observed_parts = []
    if profile_names:
        observed_parts.append(f"profiles={','.join(profile_names)}")
    observed_parts.append(f"contract_files={len(async_files)}")
    observed_parts.append(f"agent_task_contract_files={len(agent_task_files)}")
    return [
        {
            "layer": "L5",
            "entity": "kernel-lifecycle-mempool",
            "kind": "async_muscle",
            "desired": "all release/build/deploy work enters the Kernel mempool with claim, result and read-back",
            "observed": "; ".join(observed_parts),
            "health": "ok" if profile_names else "degraded",
            "owner": "infra/registry.json:lifecycle.kernel_release.request_profiles",
            "next_action": "cynic lifecycle payloads --profile gate-2",
            "proof": {
                "profiles": profile_names,
                "sample_contract_files": async_files[:12],
                "sample_agent_task_contract_files": agent_task_files[:12],
            },
        },
        {
            "layer": "L5",
            "entity": "organ-observation-heartbeats",
            "kind": "async_muscle",
            "desired": "organs emit structured observations/heartbeats through shared transport ports",
            "observed": f"contract_files={len(async_files)}",
            "health": "ok" if async_files else "degraded",
            "owner": "organs/contracts/src/adapters/kernel_observe.py",
            "next_action": "cynic converge --json",
            "proof": {
                "sample_contract_files": async_files[:12],
            },
        },
    ]


def supply_chain_topology_entities(registry: dict[str, Any]) -> list[dict[str, object]]:
    lifecycle = kernel_release_lifecycle(registry)
    targets = lifecycle_target_resolution(registry)
    build = targets.get("build_runner")
    deploy = targets.get("deploy_target")
    required = str(lifecycle.get("required_node_status") or "")
    build_name = str(build.get("name") if build else "unresolved")
    deploy_name = str(deploy.get("name") if deploy else "unresolved")
    build_status = str(build.get("status") if build else "missing")
    deploy_status = str(deploy.get("status") if deploy else "missing")
    healthy = bool(build and deploy and (not required or build_status == required and deploy_status == required))
    return [
        {
            "layer": "L5",
            "entity": "supply-chain:kernel-release",
            "kind": "supply_chain_topology",
            "desired": "cortex requests go to mempool; build runner builds; kernel primary deploys; hypervisor never hosts kernel directly",
            "observed": f"build_runner={build_name}:{build_status}; deploy_target={deploy_name}:{deploy_status}",
            "health": "ok" if healthy else "degraded",
            "owner": "infra/registry.json:lifecycle.kernel_release",
            "next_action": "cynic lifecycle diagnose",
            "proof": {
                "build_runner_role": lifecycle.get("build_runner_role"),
                "deploy_target_role": lifecycle.get("deploy_target_role"),
                "deploy_target_flag": lifecycle.get("deploy_target_flag"),
                "required_node_status": required,
            },
        }
    ]


def duplicate_domains(manifests: list[dict[str, object]]) -> dict[str, list[str]]:
    by_domain: dict[str, list[str]] = {}
    primary_by_domain: dict[str, list[str]] = {}
    for item in manifests:
        domain = str(item.get("domain") or "").strip()
        if not domain:
            continue
        name = str(item.get("name"))
        by_domain.setdefault(domain, []).append(name)
        if _is_primary_domain_role(str(item.get("domain_role") or "")):
            primary_by_domain.setdefault(domain, []).append(name)
    conflicts: dict[str, list[str]] = {}
    for domain, names in by_domain.items():
        if len(names) <= 1:
            continue
        primaries = primary_by_domain.get(domain, [])
        if len(primaries) != 1:
            conflicts[domain] = names
    return conflicts


def domains_without_primary(manifests: list[dict[str, object]]) -> dict[str, list[str]]:
    by_domain: dict[str, list[str]] = {}
    primary_by_domain: dict[str, list[str]] = {}
    for item in manifests:
        domain = str(item.get("domain") or "").strip()
        if not domain:
            continue
        name = str(item.get("name"))
        by_domain.setdefault(domain, []).append(name)
        if _is_primary_domain_role(str(item.get("domain_role") or "")):
            primary_by_domain.setdefault(domain, []).append(name)
    return {
        domain: sorted(names)
        for domain, names in sorted(by_domain.items())
        if not primary_by_domain.get(domain)
    }


def _is_primary_domain_role(role: str) -> bool:
    normalized = role.strip().lower()
    if not normalized:
        return True
    non_primary_markers = (
        "sensor",
        "adapter",
        "consumer",
        "bridge",
        "proxy",
        "helper",
        "relay",
        "shim",
    )
    return not any(marker in normalized for marker in non_primary_markers)


def lifecycle_target_gaps(registry: dict[str, Any]) -> list[dict[str, object]]:
    lifecycle = kernel_release_lifecycle(registry)
    nodes = [node for node in registry.get("nodes", []) if isinstance(node, dict)] if isinstance(registry.get("nodes"), list) else []
    build_role = str(lifecycle.get("build_runner_role") or "")
    deploy_role = str(lifecycle.get("deploy_target_role") or "")
    deploy_flag = str(lifecycle.get("deploy_target_flag") or "")
    required = str(lifecycle.get("required_node_status") or "")
    build_nodes = [node for node in nodes if node.get("role") == build_role]
    deploy_nodes = [node for node in nodes if node.get("role") == deploy_role]
    flagged_deploy_nodes = [node for node in deploy_nodes if deploy_flag and node.get(deploy_flag) is True]
    gaps: list[dict[str, object]] = []
    if len(build_nodes) != 1:
        gaps.append({"role": build_role or "build_runner", "problem": "expected exactly one build runner", "count": len(build_nodes)})
    if len(deploy_nodes) != 1:
        gaps.append({"role": deploy_role or "kernel_primary", "problem": "expected exactly one deploy role node", "count": len(deploy_nodes)})
    if deploy_flag and len(flagged_deploy_nodes) != 1:
        gaps.append({"role": deploy_role or "kernel_primary", "problem": f"expected exactly one {deploy_flag}=true", "count": len(flagged_deploy_nodes)})
    for role, role_nodes in ((build_role, build_nodes), (deploy_role, deploy_nodes)):
        for node in role_nodes:
            if required and node.get("status") != required:
                gaps.append(
                    {
                        "role": role,
                        "node": node.get("name"),
                        "problem": "required status mismatch",
                        "observed": node.get("status"),
                        "required": required,
                    }
                )
    return gaps


def async_readback_gaps(root: Path) -> list[str]:
    dispatch_markers = (
        "/agent-tasks",
        "cynic_dispatch_agent_task",
        "cynic_lifecycle_dispatch",
        "delegate-profile",
        "lifecycle-profile-payloads",
    )
    proof_markers = (
        "read_back",
        "completed",
        "pending",
        "idempotency_key",
        "cynic_update_agent_task_result",
        "/result",
        "kernel_agent_task_visible(",
        "agent_task_visible(",
    )
    gaps: list[str] = []
    for path in sorted((root / "organs").glob("*/src/**/*.py")):
        if is_ignored_duplicate_scan_path(path):
            continue
        text = read_text_without_docstrings(path)
        if any(marker in text for marker in dispatch_markers) and not any(marker in text for marker in proof_markers):
            gaps.append(rel(root, path))
    return gaps


def async_contract_files(root: Path) -> list[str]:
    markers = (
        "post_kernel_observe(",
        "post_observe_payload(",
        "agent_task_visible(",
        "task_id_in_agent_task_listings(",
        "cynic_lifecycle_dispatch",
        "cynic_update_agent_task_result",
        "read_back",
        "idempotency_key",
    )
    files: list[str] = []
    for path in sorted((root / "organs").glob("*/src/**/*.py")):
        if is_ignored_duplicate_scan_path(path):
            continue
        text = read_text(path)
        if any(marker in text for marker in markers):
            files.append(rel(root, path))
    return files


def agent_task_contract_files(root: Path) -> list[str]:
    markers = (
        "from organs.contracts.src.adapters.agent_tasks import",
        "HermesDispatcher",
        "KernelTaskClient",
        "agent_task_visible(",
        "agent_task_collection_path(",
        "agent_task_claim_path(",
        "agent_task_result_path(",
        "agent_task_result_payload(",
        "task_id_in_agent_task_listings(",
    )
    files: list[str] = []
    for path in sorted((root / "organs").glob("**/*.py")):
        if is_ignored_duplicate_scan_path(path):
            continue
        text = read_text(path)
        if any(marker in text for marker in markers):
            files.append(rel(root, path))
    return files


def observe_contract_files(root: Path) -> list[str]:
    markers = (
        "from organs.contracts.src.adapters.kernel_observe import",
        "post_kernel_observe(",
        "post_observe_payload(",
    )
    files: list[str] = []
    for path in sorted((root / "organs").glob("**/*.py")):
        if is_ignored_duplicate_scan_path(path):
            continue
        text = read_text(path)
        if any(marker in text for marker in markers):
            files.append(rel(root, path))
    return files


def kernel_surface_contract_files(root: Path, surface: str) -> list[str]:
    files: list[str] = []
    helper_markers = ()
    if surface == "/coord/":
        helper_markers = (
            "coord_register_path(",
            "coord_who_path(",
            "coord_claim_path(",
            "coord_release_path(",
        )
    for path in sorted((root / "organs").glob("**/*.py")):
        if is_ignored_duplicate_scan_path(path):
            continue
        text = read_text(path)
        surface_present = surface in text or any(marker in text for marker in helper_markers)
        if surface_present and (
            "from organs.contracts.src.adapters.kernel_http import" in text
            or "kernel_json_request(" in text
            or "kernel_url(" in text
            or "normalize_kernel_base_url(" in text
            or any(marker in text for marker in helper_markers)
        ):
            files.append(rel(root, path))
    return files


def kernel_http_membrane_fragmentation(root: Path) -> list[str]:
    local_membrane_markers = (
        "CYNIC_REST_ADDR",
        "CYNIC_API_KEY",
        "CYNIC_ALLOW_LOCAL_KERNEL",
        "Bearer ",
        "Authorization",
        "urlparse(",
    )
    transport_markers = (
        "requests.",
        "httpx.",
        "urllib.request",
        "urllib.error",
        "urlopen(",
    )
    kernel_surface_markers = (
        "/observe",
        "/judge",
        "/coord/",
        "/health",
        "/state-history",
        "/agent-tasks",
        "/proof-of-history",
    )
    canonical_markers = (
        "from organs.contracts.src.adapters.kernel_http import",
        "kernel_auth_headers(",
        "kernel_json_request(",
        "normalize_kernel_base_url(",
        "kernel_url(",
        "post_kernel_observe(",
        "post_observe_payload(",
        "KernelTransport(",
    )
    fragments: list[str] = []
    for path in sorted((root / "organs").glob("**/*.py")):
        if is_ignored_duplicate_scan_path(path):
            continue
        text = read_text_without_docstrings(path)
        has_kernel_membrane = (
            "CYNIC_REST_ADDR" in text
            and any(marker in text for marker in local_membrane_markers)
        ) or ("Bearer " in text and "Authorization" in text and "CYNIC_API_KEY" in text)
        has_transport = any(marker in text for marker in transport_markers)
        has_kernel_surface = any(marker in text for marker in kernel_surface_markers)
        if has_kernel_membrane and has_transport and has_kernel_surface and not any(marker in text for marker in canonical_markers):
            fragments.append(rel(root, path))
    return fragments


def observe_contract_fragmentation(root: Path) -> list[str]:
    direct_markers = (
        '"/observe"',
        "'/observe'",
        '}/observe"',
        "}/observe'",
    )
    canonical_markers = (
        "from organs.contracts.src.adapters.kernel_observe import",
        "post_kernel_observe(",
        "post_observe_payload(",
        "kernel_url(",
    )
    fragments: list[str] = []
    for path in sorted((root / "organs").glob("**/*.py")):
        if is_ignored_duplicate_scan_path(path):
            continue
        text = read_text_without_docstrings(path)
        if any(marker in text for marker in direct_markers) and not any(marker in text for marker in canonical_markers):
            fragments.append(rel(root, path))
    return fragments


def mempool_contract_fragmentation(root: Path) -> list[str]:
    direct_markers = (
        '"/agent-tasks',
        "'/agent-tasks",
        "/agent-tasks/completed",
    )
    canonical_markers = (
        "from organs.contracts.src.adapters.agent_tasks import",
        "from organs.contracts.src.hermes_dispatch import HermesDispatcher",
        "from .kernel_task_client import KernelTaskClient",
        "KernelTaskClient(",
        "agent_task_visible(",
        "agent_task_collection_path(",
        "agent_task_claim_path(",
        "agent_task_result_path(",
        "agent_task_result_payload(",
        "task_id_in_agent_task_listings(",
    )
    fragments: list[str] = []
    for path in sorted((root / "organs").glob("**/*.py")):
        if is_ignored_duplicate_scan_path(path):
            continue
        text = read_text_without_docstrings(path)
        if any(marker in text for marker in direct_markers) and not any(marker in text for marker in canonical_markers):
            fragments.append(rel(root, path))
    return fragments


def kernel_release_lifecycle(registry: dict[str, Any]) -> dict[str, Any]:
    lifecycle = registry.get("lifecycle", {}) if isinstance(registry.get("lifecycle"), dict) else {}
    kernel_release = lifecycle.get("kernel_release", {}) if isinstance(lifecycle.get("kernel_release"), dict) else {}
    return kernel_release


def lifecycle_target_resolution(registry: dict[str, Any]) -> dict[str, dict[str, Any] | None]:
    lifecycle = kernel_release_lifecycle(registry)
    nodes = [node for node in registry.get("nodes", []) if isinstance(node, dict)] if isinstance(registry.get("nodes"), list) else []
    build_role = str(lifecycle.get("build_runner_role") or "")
    deploy_role = str(lifecycle.get("deploy_target_role") or "")
    deploy_flag = str(lifecycle.get("deploy_target_flag") or "")
    build = next((node for node in nodes if node.get("role") == build_role), None)
    deploy_candidates = [node for node in nodes if node.get("role") == deploy_role]
    flagged = [node for node in deploy_candidates if deploy_flag and node.get(deploy_flag) is True]
    deploy = flagged[0] if len(flagged) == 1 else deploy_candidates[0] if len(deploy_candidates) == 1 else None
    return {"build_runner": build, "deploy_target": deploy}
