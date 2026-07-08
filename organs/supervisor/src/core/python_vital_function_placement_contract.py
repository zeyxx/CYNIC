"""Validate the Python vital-function placement contract."""

from __future__ import annotations

import json
from pathlib import Path

import organs.supervisor.src.core.repo_topology_host as host


CONTRACT_PATH = host.CONTRACTS_ROOT / "python-vital-function-placement-v1.json"
RUST_CRATE_ROLES_PATH = host.CONTRACTS_ROOT / "rust-crate-roles-v1.json"

REQUIRED_GROUP_FIELDS = {
    "id",
    "status",
    "organ",
    "criticality",
    "target_crate",
    "target_component",
    "source_paths",
    "functions",
    "reason",
}
REQUIRED_TARGET_POLICY_FIELDS = {"target_crate", "expected_family", "expected_class", "why"}
ALLOWED_STATUSES = {
    "planned_migration",
    "active_migration_queue",
    "planned_removal_after_absorption",
    "absorbed",
}


def validate_python_vital_function_placement_contract() -> list[host.Finding]:
    if not CONTRACT_PATH.exists():
        return [
            host.Finding(
                "critical",
                "python_vital_function_placement_contract_missing",
                host.relative(CONTRACT_PATH),
                "python-vital-function-placement-v1.json must exist to formalize Python vital-function absorption targets.",
            )
        ]

    try:
        contract = host.load_json(CONTRACT_PATH)
    except json.JSONDecodeError as exc:
        return [
            host.Finding(
                "critical",
                "python_vital_function_placement_contract_invalid_json",
                host.relative(CONTRACT_PATH),
                f"{exc.msg} at line {exc.lineno} column {exc.colno}",
            )
        ]

    findings: list[host.Finding] = []
    contract_path = host.relative(CONTRACT_PATH)
    if contract.get("schema") != "cynic.python_vital_function_placement.v1":
        findings.append(
            host.Finding(
                "critical",
                "python_vital_function_placement_schema_invalid",
                contract_path,
                f"expected cynic.python_vital_function_placement.v1, got {contract.get('schema')!r}",
            )
        )

    groups = contract.get("function_groups")
    if not isinstance(groups, list):
        findings.append(
            host.Finding(
                "critical",
                "python_vital_function_groups_missing",
                contract_path,
                "function_groups must be a list.",
            )
        )
        return findings

    crate_roles = _declared_crates()
    target_policies = _target_policies(contract, contract_path, findings)
    seen_ids: set[str] = set()
    for index, group in enumerate(groups):
        pointer = f"{contract_path}#function_groups[{index}]"
        if not isinstance(group, dict):
            findings.append(
                host.Finding(
                    "critical",
                    "python_vital_function_group_invalid",
                    pointer,
                    "each function group must be an object.",
                )
            )
            continue
        missing = sorted(field for field in REQUIRED_GROUP_FIELDS if field not in group)
        for field in missing:
            findings.append(
                host.Finding(
                    "critical",
                    "python_vital_function_group_field_missing",
                    pointer,
                    f"function group must declare {field}.",
                )
            )
        group_id = group.get("id")
        if isinstance(group_id, str) and group_id:
            if group_id in seen_ids:
                findings.append(
                    host.Finding(
                        "critical",
                        "python_vital_function_group_duplicate",
                        pointer,
                        f"function group id {group_id!r} is declared more than once.",
                    )
                )
            seen_ids.add(group_id)
        status = group.get("status")
        if status not in ALLOWED_STATUSES:
            findings.append(
                host.Finding(
                    "critical",
                    "python_vital_function_group_status_invalid",
                    pointer,
                    f"status must be one of {sorted(ALLOWED_STATUSES)}, got {status!r}.",
                )
            )
        target_crate = group.get("target_crate")
        crate_role = crate_roles.get(target_crate) if isinstance(target_crate, str) else None
        if isinstance(target_crate, str) and target_crate not in crate_roles:
            findings.append(
                host.Finding(
                    "critical",
                    "python_vital_function_target_crate_unknown",
                    pointer,
                    f"target_crate {target_crate!r} is not declared in rust-crate-roles-v1.json.",
                )
            )
        elif isinstance(target_crate, str):
            if not isinstance(crate_role.get("python_absorption_authority"), dict):
                findings.append(
                    host.Finding(
                        "critical",
                        "python_vital_function_target_crate_not_absorption_authority",
                        pointer,
                        f"target_crate {target_crate!r} does not declare python_absorption_authority in rust-crate-roles-v1.json.",
                    )
                )
            policy = target_policies.get(target_crate)
            if policy is None:
                findings.append(
                    host.Finding(
                        "critical",
                        "python_vital_function_target_crate_policy_missing",
                        pointer,
                        f"target_crate {target_crate!r} is used by a function group but missing from target_crate_policy.",
                    )
                )
            else:
                expected_family = policy.get("expected_family")
                expected_class = policy.get("expected_class")
                actual_family = crate_role.get("family")
                actual_class = crate_role.get("class")
                if actual_family != expected_family:
                    findings.append(
                        host.Finding(
                            "critical",
                            "python_vital_function_target_family_mismatch",
                            pointer,
                            f"target_crate {target_crate!r} must have family {expected_family!r}, got {actual_family!r}.",
                        )
                    )
                if actual_class != expected_class:
                    findings.append(
                        host.Finding(
                            "critical",
                            "python_vital_function_target_class_mismatch",
                            pointer,
                            f"target_crate {target_crate!r} must have class {expected_class!r}, got {actual_class!r}.",
                        )
                    )
        for source in group.get("source_paths", []):
            if not isinstance(source, str) or not source:
                findings.append(
                    host.Finding(
                        "critical",
                        "python_vital_function_source_path_invalid",
                        pointer,
                        "source_paths entries must be non-empty strings.",
                    )
                )
                continue
            if not (host.WORKSPACE_ROOT / source).exists():
                findings.append(
                    host.Finding(
                        "warning",
                        "python_vital_function_source_path_missing_on_disk",
                        source,
                        "declared source path does not exist on disk.",
                    )
                )
        functions = group.get("functions")
        if not isinstance(functions, list) or not all(isinstance(item, str) and item for item in functions):
            findings.append(
                host.Finding(
                    "critical",
                    "python_vital_function_group_functions_invalid",
                    pointer,
                    "functions must be a non-empty list of function names.",
                )
            )
    return findings


def _declared_crates() -> dict[str, dict[str, object]]:
    try:
        payload = host.load_json(RUST_CRATE_ROLES_PATH)
    except Exception:
        return {}
    crates = payload.get("crates")
    if not isinstance(crates, list):
        return {}
    declared: dict[str, dict[str, object]] = {}
    for item in crates:
        if not isinstance(item, dict):
            continue
        path = item.get("path")
        if isinstance(path, str):
            declared[path] = item
    return declared


def _target_policies(
    contract: dict[str, object], contract_path: str, findings: list[host.Finding]
) -> dict[str, dict[str, object]]:
    policies = contract.get("target_crate_policy")
    if not isinstance(policies, list):
        findings.append(
            host.Finding(
                "critical",
                "python_vital_function_target_crate_policy_missing",
                contract_path,
                "target_crate_policy must be a list.",
            )
        )
        return {}
    by_target: dict[str, dict[str, object]] = {}
    for index, item in enumerate(policies):
        pointer = f"{contract_path}#target_crate_policy[{index}]"
        if not isinstance(item, dict):
            findings.append(
                host.Finding(
                    "critical",
                    "python_vital_function_target_crate_policy_invalid",
                    pointer,
                    "each target_crate_policy entry must be an object.",
                )
            )
            continue
        missing = sorted(field for field in REQUIRED_TARGET_POLICY_FIELDS if field not in item)
        for field in missing:
            findings.append(
                host.Finding(
                    "critical",
                    "python_vital_function_target_crate_policy_field_missing",
                    pointer,
                    f"target_crate_policy entry must declare {field}.",
                )
            )
        target_crate = item.get("target_crate")
        if isinstance(target_crate, str) and target_crate:
            by_target[target_crate] = item
    return by_target
