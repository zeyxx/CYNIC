"""Validate heterogeneous runtime role matrix against the registry."""

from __future__ import annotations

from typing import Any

import organs.supervisor.src.core.repo_topology_host as host


CONTRACT_PATH = host.CONTRACTS_ROOT / "heterogeneous-runtime-matrix-v1.json"
REGISTRY_PATH = host.WORKSPACE_ROOT / "infra/registry.json"
EXPECTED_SCHEMA = "cynic.heterogeneous_runtime_matrix.v1"
REQUIRED_ROLE_FIELDS = {
    "role",
    "class",
    "organism_role",
    "bootstrap_target_stage",
    "required_local_agents",
    "authorized_local_actuators",
    "activation_mode",
    "required_network_planes",
    "required_dns_contracts",
    "required_proof_surfaces",
    "capabilities",
    "why",
}
REQUIRED_NODE_BINDING_FIELDS = {"node", "role", "machine_type"}
REQUIRED_CAPABILITY_FIELDS = {"build", "deploy", "recover"}


def validate_heterogeneous_runtime_matrix_contract() -> list[host.Finding]:
    findings: list[host.Finding] = []
    if not CONTRACT_PATH.exists():
        return [
            host.Finding(
                "critical",
                "heterogeneous_runtime_matrix_missing",
                host.relative(CONTRACT_PATH),
                "heterogeneous-runtime-matrix-v1.json must classify every registry node role.",
            )
        ]

    contract = host.load_json(CONTRACT_PATH)
    if contract.get("schema") != EXPECTED_SCHEMA:
        return [
            host.Finding(
                "critical",
                "unexpected_heterogeneous_runtime_matrix_schema",
                host.relative(CONTRACT_PATH),
                f"expected {EXPECTED_SCHEMA}, got {contract.get('schema')!r}",
            )
        ]

    findings.extend(_validate_authorities(contract))
    allowed_classes = set(contract.get("allowed_classes") or [])
    allowed_machine_types = set(contract.get("allowed_machine_types") or [])
    allowed_activation_modes = set(contract.get("allowed_activation_modes") or [])
    allowed_local_agents = set(contract.get("allowed_local_agents") or [])
    allowed_local_actuators = set(contract.get("allowed_local_actuators") or [])
    allowed_network_planes = set(contract.get("allowed_network_planes") or [])
    allowed_dns_contracts = set(contract.get("allowed_dns_contracts") or [])
    allowed_proof_surfaces = set(contract.get("allowed_proof_surfaces") or [])
    roles = contract.get("roles")
    if not isinstance(roles, list):
        return findings + [
            host.Finding(
                "critical",
                "heterogeneous_runtime_roles_missing",
                host.relative(CONTRACT_PATH),
                "roles must be a list.",
            )
        ]

    seen_roles: set[str] = set()
    declared_roles: set[str] = set()
    for index, item in enumerate(roles):
        pointer = f"{host.relative(CONTRACT_PATH)}:roles[{index}]"
        if not isinstance(item, dict):
            findings.append(
                host.Finding(
                    "critical",
                    "heterogeneous_runtime_role_invalid",
                    pointer,
                    "each roles entry must be an object.",
                )
            )
            continue
        for field in sorted(REQUIRED_ROLE_FIELDS):
            if not item.get(field):
                findings.append(
                    host.Finding(
                        "critical",
                        "heterogeneous_runtime_role_field_missing",
                        pointer,
                        f"roles entry must declare {field}.",
                    )
                )
        role = str(item.get("role") or "")
        if role:
            if role in seen_roles:
                findings.append(
                    host.Finding(
                        "critical",
                        "heterogeneous_runtime_role_duplicate",
                        role,
                        "role is declared more than once.",
                    )
                )
            seen_roles.add(role)
            declared_roles.add(role)
        role_class = str(item.get("class") or "")
        if role_class and role_class not in allowed_classes:
            findings.append(
                host.Finding(
                    "critical",
                    "heterogeneous_runtime_role_class_unknown",
                    pointer,
                    f"class {role_class!r} is not allowed.",
                )
            )
        activation_mode = str(item.get("activation_mode") or "")
        if activation_mode and activation_mode not in allowed_activation_modes:
            findings.append(
                host.Finding(
                    "critical",
                    "heterogeneous_runtime_activation_mode_unknown",
                    pointer,
                    f"activation_mode {activation_mode!r} is not allowed.",
                )
            )
        for actuator in item.get("authorized_local_actuators", []) or []:
            if str(actuator) not in allowed_local_actuators:
                findings.append(
                    host.Finding(
                        "critical",
                        "heterogeneous_runtime_local_actuator_unknown",
                        pointer,
                        f"authorized_local_actuators entry {actuator!r} is not allowed.",
                    )
                )
        for agent in item.get("required_local_agents", []) or []:
            if str(agent) not in allowed_local_agents:
                findings.append(
                    host.Finding(
                        "critical",
                        "heterogeneous_runtime_local_agent_unknown",
                        pointer,
                        f"required_local_agents entry {agent!r} is not allowed.",
                    )
                )
        for plane in item.get("required_network_planes", []) or []:
            if str(plane) not in allowed_network_planes:
                findings.append(
                    host.Finding(
                        "critical",
                        "heterogeneous_runtime_network_plane_unknown",
                        pointer,
                        f"required_network_planes entry {plane!r} is not allowed.",
                    )
                )
        for contract_name in item.get("required_dns_contracts", []) or []:
            if str(contract_name) not in allowed_dns_contracts:
                findings.append(
                    host.Finding(
                        "critical",
                        "heterogeneous_runtime_dns_contract_unknown",
                        pointer,
                        f"required_dns_contracts entry {contract_name!r} is not allowed.",
                    )
                )
        for proof in item.get("required_proof_surfaces", []) or []:
            if str(proof) not in allowed_proof_surfaces:
                findings.append(
                    host.Finding(
                        "critical",
                        "heterogeneous_runtime_proof_surface_unknown",
                        pointer,
                        f"required_proof_surfaces entry {proof!r} is not allowed.",
                    )
                )
        capabilities = item.get("capabilities")
        if not isinstance(capabilities, dict):
            findings.append(
                host.Finding(
                    "critical",
                    "heterogeneous_runtime_capabilities_invalid",
                    pointer,
                    "capabilities must be an object with build, deploy, and recover booleans.",
                )
            )
        else:
            for field in sorted(REQUIRED_CAPABILITY_FIELDS):
                value = capabilities.get(field)
                if not isinstance(value, bool):
                    findings.append(
                        host.Finding(
                            "critical",
                            "heterogeneous_runtime_capability_missing",
                            pointer,
                            f"capabilities.{field} must be a boolean.",
                        )
                    )

    registry = host.load_json(REGISTRY_PATH)
    registry_nodes = {
        str(item.get("name")): str(item.get("role"))
        for item in registry.get("nodes", [])
        if isinstance(item, dict) and item.get("name") and item.get("role")
    }
    registry_roles = set(registry_nodes.values())
    for role in sorted(registry_roles - declared_roles):
        findings.append(
            host.Finding(
                "critical",
                "heterogeneous_runtime_registry_role_unmapped",
                role,
                "registry node role must be classified in heterogeneous-runtime-matrix-v1.json.",
            )
        )

    findings.extend(
        _validate_node_bindings(
            contract=contract,
            registry_nodes=registry_nodes,
            allowed_machine_types=allowed_machine_types,
            declared_roles=declared_roles,
        )
    )

    totals = contract.get("totals", {})
    if totals.get("roles") != len(roles):
        findings.append(
            host.Finding(
                "critical",
                "heterogeneous_runtime_total_mismatch",
                host.relative(CONTRACT_PATH),
                f"totals.roles={totals.get('roles')!r} but contract contains {len(roles)} entries.",
            )
        )
    node_bindings = contract.get("node_bindings", [])
    if totals.get("node_bindings") != len(node_bindings):
        findings.append(
            host.Finding(
                "critical",
                "heterogeneous_runtime_node_binding_total_mismatch",
                host.relative(CONTRACT_PATH),
                f"totals.node_bindings={totals.get('node_bindings')!r} but contract contains {len(node_bindings)} entries.",
            )
        )

    return findings


def _validate_authorities(contract: dict[str, Any]) -> list[host.Finding]:
    findings: list[host.Finding] = []
    for name, value in sorted((contract.get("authorities") or {}).items()):
        if not isinstance(value, str) or not value:
            findings.append(
                host.Finding(
                    "critical",
                    "heterogeneous_runtime_authority_invalid",
                    host.relative(CONTRACT_PATH),
                    f"authority {name!r} must declare a non-empty path string.",
                )
            )
            continue
        path = host.WORKSPACE_ROOT / value
        if not path.exists():
            findings.append(
                host.Finding(
                    "critical",
                    "heterogeneous_runtime_authority_missing",
                    value,
                    f"authority {name!r} points to a path that does not exist.",
                )
            )
    return findings


def _validate_node_bindings(
    *,
    contract: dict[str, Any],
    registry_nodes: dict[str, str],
    allowed_machine_types: set[str],
    declared_roles: set[str],
) -> list[host.Finding]:
    findings: list[host.Finding] = []
    node_bindings = contract.get("node_bindings")
    if not isinstance(node_bindings, list):
        return [
            host.Finding(
                "critical",
                "heterogeneous_runtime_node_bindings_missing",
                host.relative(CONTRACT_PATH),
                "node_bindings must be a list that covers every registry node.",
            )
        ]

    seen_nodes: set[str] = set()
    bound_nodes: set[str] = set()
    for index, item in enumerate(node_bindings):
        pointer = f"{host.relative(CONTRACT_PATH)}:node_bindings[{index}]"
        if not isinstance(item, dict):
            findings.append(
                host.Finding(
                    "critical",
                    "heterogeneous_runtime_node_binding_invalid",
                    pointer,
                    "each node_bindings entry must be an object.",
                )
            )
            continue
        for field in sorted(REQUIRED_NODE_BINDING_FIELDS):
            if not item.get(field):
                findings.append(
                    host.Finding(
                        "critical",
                        "heterogeneous_runtime_node_binding_field_missing",
                        pointer,
                        f"node binding must declare {field}.",
                    )
                )
        node_name = str(item.get("node") or "")
        role_name = str(item.get("role") or "")
        machine_type = str(item.get("machine_type") or "")
        if node_name:
            if node_name in seen_nodes:
                findings.append(
                    host.Finding(
                        "critical",
                        "heterogeneous_runtime_node_binding_duplicate",
                        node_name,
                        "node is declared more than once in node_bindings.",
                    )
                )
            seen_nodes.add(node_name)
            bound_nodes.add(node_name)
            registry_role = registry_nodes.get(node_name)
            if registry_role is None:
                findings.append(
                    host.Finding(
                        "critical",
                        "heterogeneous_runtime_node_binding_unknown_node",
                        node_name,
                        "node binding does not match any registry node.",
                    )
                )
            elif role_name and registry_role != role_name:
                findings.append(
                    host.Finding(
                        "critical",
                        "heterogeneous_runtime_node_binding_role_mismatch",
                        node_name,
                        f"binding role {role_name!r} does not match registry role {registry_role!r}.",
                    )
                )
        if role_name and role_name not in declared_roles:
            findings.append(
                host.Finding(
                    "critical",
                    "heterogeneous_runtime_node_binding_unmapped_role",
                    pointer,
                    f"role {role_name!r} is not declared in roles.",
                )
            )
        if machine_type and machine_type not in allowed_machine_types:
            findings.append(
                host.Finding(
                    "critical",
                    "heterogeneous_runtime_node_binding_machine_type_unknown",
                    pointer,
                    f"machine_type {machine_type!r} is not allowed.",
                )
            )

    for node_name in sorted(set(registry_nodes) - bound_nodes):
        findings.append(
            host.Finding(
                "critical",
                "heterogeneous_runtime_registry_node_unbound",
                node_name,
                "every registry node must be bound in node_bindings.",
            )
        )
    return findings
