from __future__ import annotations

from pathlib import Path
from typing import Any

import organs.supervisor.src.core.repo_topology_host as host


CONTRACT_PATH = host.WORKSPACE_ROOT / "axioms" / "agentic-runtime-topology-v1.json"
EXPECTED_SCHEMA = "cynic.agentic_runtime_topology.v1"


def validate_agentic_runtime_topology_contract() -> list[host.Finding]:
    findings: list[host.Finding] = []
    if not CONTRACT_PATH.exists():
        return [
            host.Finding(
                "critical",
                "agentic_runtime_topology_contract_missing",
                host.relative(CONTRACT_PATH),
                "agentic-runtime-topology-v1.json must exist to declare canonical agentic roles and runtime classes.",
            )
        ]

    contract = host.load_json(CONTRACT_PATH)
    if contract.get("schema") != EXPECTED_SCHEMA:
        findings.append(
            host.Finding(
                "critical",
                "unexpected_agentic_runtime_topology_schema",
                host.relative(CONTRACT_PATH),
                f"expected {EXPECTED_SCHEMA}, got {contract.get('schema')!r}",
            )
        )
        return findings

    findings.extend(_validate_authorities(contract))
    findings.extend(_validate_roles(contract))
    findings.extend(_validate_runtime_classes(contract))
    findings.extend(_validate_minimum_runtime_set(contract))
    findings.extend(_validate_mapping_pressure(contract))
    findings.extend(_validate_forbidden_runtime_shapes(contract))
    return findings


def _validate_authorities(contract: dict[str, Any]) -> list[host.Finding]:
    findings: list[host.Finding] = []
    for name, value in sorted((contract.get("authorities") or {}).items()):
        if not isinstance(value, str) or not value:
            findings.append(
                host.Finding(
                    "critical",
                    "agentic_runtime_authority_invalid",
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
                    "agentic_runtime_authority_missing",
                    value,
                    f"authority {name!r} points to a path that does not exist.",
                )
            )
    return findings


def _validate_roles(contract: dict[str, Any]) -> list[host.Finding]:
    findings: list[host.Finding] = []
    roles = contract.get("agentic_roles")
    if not isinstance(roles, list) or not roles:
        return [
            host.Finding(
                "critical",
                "agentic_roles_missing",
                host.relative(CONTRACT_PATH),
                "agentic-runtime-topology-v1.json must declare agentic_roles.",
            )
        ]
    seen: set[str] = set()
    for index, item in enumerate(roles):
        path = f"{host.relative(CONTRACT_PATH)}:agentic_roles[{index}]"
        role_id = str(item.get("id") or "")
        if not role_id:
            findings.append(host.Finding("critical", "agentic_role_id_missing", path, "agentic role entries must declare id."))
            continue
        if role_id in seen:
            findings.append(host.Finding("critical", "agentic_role_duplicate", path, f"agentic role {role_id!r} is declared more than once."))
        seen.add(role_id)
        if not item.get("class"):
            findings.append(host.Finding("critical", "agentic_role_class_missing", path, f"agentic role {role_id!r} must declare class."))
        for field in ("allowed_responsibilities", "forbidden_responsibilities"):
            value = item.get(field)
            if not isinstance(value, list) or not value:
                findings.append(host.Finding("critical", "agentic_role_responsibilities_missing", path, f"agentic role {role_id!r} must declare non-empty {field}."))
    return findings


def _validate_runtime_classes(contract: dict[str, Any]) -> list[host.Finding]:
    findings: list[host.Finding] = []
    runtimes = contract.get("runtime_classes")
    if not isinstance(runtimes, list) or not runtimes:
        return [
            host.Finding(
                "critical",
                "runtime_classes_missing",
                host.relative(CONTRACT_PATH),
                "agentic-runtime-topology-v1.json must declare runtime_classes.",
            )
        ]
    seen: set[str] = set()
    for index, item in enumerate(runtimes):
        path = f"{host.relative(CONTRACT_PATH)}:runtime_classes[{index}]"
        runtime_id = str(item.get("id") or "")
        if not runtime_id:
            findings.append(host.Finding("critical", "runtime_class_id_missing", path, "runtime class entries must declare id."))
            continue
        if runtime_id in seen:
            findings.append(host.Finding("critical", "runtime_class_duplicate", path, f"runtime class {runtime_id!r} is declared more than once."))
        seen.add(runtime_id)
        if item.get("class") != "shared_runtime_framework":
            findings.append(host.Finding("critical", "runtime_class_invalid_class", path, f"runtime class {runtime_id!r} must use class='shared_runtime_framework'."))
        for field in ("allowed_responsibilities", "forbidden_responsibilities"):
            value = item.get(field)
            if not isinstance(value, list) or not value:
                findings.append(host.Finding("critical", "runtime_class_responsibilities_missing", path, f"runtime class {runtime_id!r} must declare non-empty {field}."))
    return findings


def _validate_minimum_runtime_set(contract: dict[str, Any]) -> list[host.Finding]:
    findings: list[host.Finding] = []
    declared = {
        str(item.get("id"))
        for item in contract.get("runtime_classes", [])
        if isinstance(item, dict) and item.get("id")
    }
    minimum = contract.get("minimum_runtime_set")
    if not isinstance(minimum, list) or not minimum:
        return [
            host.Finding(
                "critical",
                "minimum_runtime_set_missing",
                host.relative(CONTRACT_PATH),
                "agentic-runtime-topology-v1.json must declare minimum_runtime_set.",
            )
        ]
    for runtime_id in minimum:
        if str(runtime_id) not in declared:
            findings.append(
                host.Finding(
                    "critical",
                    "minimum_runtime_unknown",
                    host.relative(CONTRACT_PATH),
                    f"minimum runtime {runtime_id!r} is not declared in runtime_classes.",
                )
            )
    return findings


def _validate_mapping_pressure(contract: dict[str, Any]) -> list[host.Finding]:
    findings: list[host.Finding] = []
    runtime_ids = {
        str(item.get("id"))
        for item in contract.get("runtime_classes", [])
        if isinstance(item, dict) and item.get("id")
    }
    role_ids = {
        str(item.get("id"))
        for item in contract.get("agentic_roles", [])
        if isinstance(item, dict) and item.get("id")
    }
    for index, item in enumerate(contract.get("current_mapping_pressure", [])):
        path = str(item.get("path") or f"current_mapping_pressure[{index}]")
        fs_path = host.WORKSPACE_ROOT / path
        if not fs_path.exists():
            findings.append(
                host.Finding(
                    "warning",
                    "agentic_runtime_mapping_pressure_path_missing",
                    path,
                    "current_mapping_pressure points to a path that does not exist.",
                )
            )
        primary_runtime = str(item.get("declared_primary_runtime_class") or "")
        if primary_runtime and primary_runtime not in runtime_ids:
            findings.append(
                host.Finding(
                    "critical",
                    "agentic_runtime_mapping_pressure_unknown_runtime",
                    path,
                    f"declared_primary_runtime_class {primary_runtime!r} is not declared in runtime_classes.",
                )
            )
        for runtime_id in item.get("secondary_runtime_classes", []) or []:
            if str(runtime_id) not in runtime_ids:
                findings.append(
                    host.Finding(
                        "critical",
                        "agentic_runtime_mapping_pressure_unknown_secondary_runtime",
                        path,
                        f"secondary runtime {runtime_id!r} is not declared in runtime_classes.",
                    )
                )
        for role_id in item.get("declared_roles", []) or []:
            if str(role_id) not in role_ids:
                findings.append(
                    host.Finding(
                        "critical",
                        "agentic_runtime_mapping_pressure_unknown_role",
                        path,
                        f"declared role {role_id!r} is not declared in agentic_roles.",
                    )
                )
    return findings


def _validate_forbidden_runtime_shapes(contract: dict[str, Any]) -> list[host.Finding]:
    findings: list[host.Finding] = []
    rules = contract.get("canonical_runtime_root_rules") or {}
    forbidden = rules.get("forbidden_shared_runtime_shapes") or []
    if "services/**/organs/**" not in forbidden:
        return findings

    services_root = host.WORKSPACE_ROOT / "services"
    if not services_root.exists():
        return findings

    seen: set[str] = set()
    for organ_dir in services_root.rglob("organs"):
        rel = host.relative(organ_dir)
        if rel in seen:
            continue
        seen.add(rel)
        findings.append(
            host.Finding(
                "warning",
                "forbidden_shared_runtime_shape_present",
                rel,
                "shared runtime frameworks must not live under services/**/organs/**; move them to explicit services/<runtime>/ roots and keep only compatibility shims during migration.",
            )
        )
    return findings
