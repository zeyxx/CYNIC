"""Validate canonical node-network interfaces for Python->Rust absorption."""

from __future__ import annotations

from pathlib import Path
from typing import Any

import organs.supervisor.src.core.repo_topology_host as host


CONTRACT_PATH = host.CONTRACTS_ROOT / "node-network-interfaces-v1.json"
RUST_CRATE_ROLES_PATH = host.CONTRACTS_ROOT / "rust-crate-roles-v1.json"
PYTHON_VITAL_FUNCTION_PATH = host.CONTRACTS_ROOT / "python-vital-function-placement-v1.json"
EXPECTED_SCHEMA = "cynic.node_network_interfaces.v1"
REQUIRED_PORT_FIELDS = {
    "id",
    "responsibility",
    "rust_trait",
    "current_python_authority",
    "absorbs_function_groups",
}


def validate_node_network_interfaces_contract() -> list[host.Finding]:
    findings: list[host.Finding] = []
    if not CONTRACT_PATH.exists():
        return [
            host.Finding(
                "critical",
                "node_network_interfaces_contract_missing",
                host.relative(CONTRACT_PATH),
                "node-network-interfaces-v1.json must exist to declare canonical node network ports.",
            )
        ]

    contract = host.load_json(CONTRACT_PATH)
    contract_path = host.relative(CONTRACT_PATH)
    if contract.get("schema") != EXPECTED_SCHEMA:
        findings.append(
            host.Finding(
                "critical",
                "unexpected_node_network_interfaces_schema",
                contract_path,
                f"expected {EXPECTED_SCHEMA}, got {contract.get('schema')!r}",
            )
        )
        return findings

    findings.extend(_validate_authorities(contract))
    findings.extend(_validate_target_crate(contract))
    findings.extend(_validate_ports(contract))
    return findings


def _validate_authorities(contract: dict[str, Any]) -> list[host.Finding]:
    findings: list[host.Finding] = []
    for name, value in sorted((contract.get("authorities") or {}).items()):
        if not isinstance(value, str) or not value:
            findings.append(
                host.Finding(
                    "critical",
                    "node_network_interface_authority_invalid",
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
                    "node_network_interface_authority_missing",
                    value,
                    f"authority {name!r} points to a path that does not exist.",
                )
            )
    return findings


def _validate_target_crate(contract: dict[str, Any]) -> list[host.Finding]:
    findings: list[host.Finding] = []
    target = contract.get("target_crate")
    if not isinstance(target, dict):
        return [
            host.Finding(
                "critical",
                "node_network_target_crate_missing",
                host.relative(CONTRACT_PATH),
                "target_crate must declare the canonical Rust crate for network ports.",
            )
        ]
    target_path = str(target.get("path") or "")
    if not target_path:
        findings.append(
            host.Finding(
                "critical",
                "node_network_target_crate_path_missing",
                host.relative(CONTRACT_PATH),
                "target_crate.path is required.",
            )
        )
        return findings
    crates = _declared_crates()
    item = crates.get(target_path)
    if item is None:
        findings.append(
            host.Finding(
                "critical",
                "node_network_target_crate_unknown",
                host.relative(CONTRACT_PATH),
                f"target_crate.path {target_path!r} is not declared in rust-crate-roles-v1.json.",
            )
        )
        return findings
    for field in ("family", "class"):
        expected = target.get(field)
        actual = item.get(field)
        if expected != actual:
            findings.append(
                host.Finding(
                    "critical",
                    f"node_network_target_crate_{field}_mismatch",
                    host.relative(CONTRACT_PATH),
                    f"target_crate.{field} must be {actual!r} for {target_path!r}, got {expected!r}.",
                )
            )
    return findings


def _validate_ports(contract: dict[str, Any]) -> list[host.Finding]:
    findings: list[host.Finding] = []
    ports = contract.get("ports")
    if not isinstance(ports, list) or not ports:
        return [
            host.Finding(
                "critical",
                "node_network_ports_missing",
                host.relative(CONTRACT_PATH),
                "node-network-interfaces-v1.json must declare ports.",
            )
        ]
    valid_groups = _declared_vital_function_groups()
    seen_ids: set[str] = set()
    for index, item in enumerate(ports):
        pointer = f"{host.relative(CONTRACT_PATH)}:ports[{index}]"
        if not isinstance(item, dict):
            findings.append(
                host.Finding(
                    "critical",
                    "node_network_port_invalid",
                    pointer,
                    "each ports entry must be an object.",
                )
            )
            continue
        for field in sorted(REQUIRED_PORT_FIELDS):
            if not item.get(field):
                findings.append(
                    host.Finding(
                        "critical",
                        "node_network_port_field_missing",
                        pointer,
                        f"ports entry must declare {field}.",
                    )
                )
        port_id = str(item.get("id") or "")
        if port_id:
            if port_id in seen_ids:
                findings.append(
                    host.Finding(
                        "critical",
                        "node_network_port_duplicate",
                        pointer,
                        f"port {port_id!r} is declared more than once.",
                    )
                )
            seen_ids.add(port_id)
        rust_trait = item.get("rust_trait")
        if isinstance(rust_trait, str) and rust_trait:
            trait_file = rust_trait.split("::", 1)[0]
            path = host.WORKSPACE_ROOT / trait_file
            if not path.exists():
                findings.append(
                    host.Finding(
                        "critical",
                        "node_network_port_trait_path_missing",
                        pointer,
                        f"rust_trait path {trait_file!r} does not exist.",
                    )
                )
        for authority in item.get("current_python_authority", []) or []:
            if not isinstance(authority, str) or not authority:
                findings.append(
                    host.Finding(
                        "critical",
                        "node_network_port_python_authority_invalid",
                        pointer,
                        "current_python_authority entries must be non-empty strings.",
                    )
                )
                continue
            if not (host.WORKSPACE_ROOT / authority).exists():
                findings.append(
                    host.Finding(
                        "warning",
                        "node_network_port_python_authority_missing",
                        authority,
                        "declared current_python_authority path does not exist on disk.",
                    )
                )
        for group_id in item.get("absorbs_function_groups", []) or []:
            if str(group_id) not in valid_groups:
                findings.append(
                    host.Finding(
                        "critical",
                        "node_network_port_unknown_function_group",
                        pointer,
                        f"absorbs_function_groups entry {group_id!r} is not declared in python-vital-function-placement-v1.json.",
                    )
                )
    return findings


def _declared_crates() -> dict[str, dict[str, object]]:
    payload = host.load_json(RUST_CRATE_ROLES_PATH)
    crates = payload.get("crates")
    if not isinstance(crates, list):
        return {}
    return {
        str(item.get("path")): item
        for item in crates
        if isinstance(item, dict) and isinstance(item.get("path"), str)
    }


def _declared_vital_function_groups() -> set[str]:
    payload = host.load_json(PYTHON_VITAL_FUNCTION_PATH)
    groups = payload.get("function_groups")
    if not isinstance(groups, list):
        return set()
    return {
        str(item.get("id"))
        for item in groups
        if isinstance(item, dict) and isinstance(item.get("id"), str)
    }
