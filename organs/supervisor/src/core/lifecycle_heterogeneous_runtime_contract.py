"""Validate lifecycle registry roles against the heterogeneous runtime matrix."""

from __future__ import annotations

from typing import Any

import organs.supervisor.src.core.repo_topology_host as host


REGISTRY_PATH = host.WORKSPACE_ROOT / "infra/registry.json"
MATRIX_PATH = host.CONTRACTS_ROOT / "heterogeneous-runtime-matrix-v1.json"
RUST_CRATE_ROLES_PATH = host.CONTRACTS_ROOT / "rust-crate-roles-v1.json"


def validate_lifecycle_heterogeneous_runtime_contract() -> list[host.Finding]:
    findings: list[host.Finding] = []
    if not REGISTRY_PATH.exists():
        return findings
    if not MATRIX_PATH.exists():
        return [
            host.Finding(
                "critical",
                "lifecycle_heterogeneous_runtime_matrix_missing",
                host.relative(MATRIX_PATH),
                "heterogeneous runtime matrix must exist before lifecycle roles can be validated.",
            )
        ]

    registry = host.load_json(REGISTRY_PATH)
    matrix = host.load_json(MATRIX_PATH)
    lifecycle = (
        registry.get("lifecycle", {}).get("kernel_release", {})
        if isinstance(registry.get("lifecycle"), dict)
        else {}
    )
    role_map = {
        str(item.get("role")): item
        for item in matrix.get("roles", [])
        if isinstance(item, dict) and item.get("role")
    }
    declared_crates = {
        str(item.get("path"))
        for item in host.load_json(RUST_CRATE_ROLES_PATH).get("crates", [])
        if isinstance(item, dict) and item.get("path")
    }

    findings.extend(_validate_primary_roles(lifecycle, role_map))
    findings.extend(_validate_activation_requirements(lifecycle, role_map))
    findings.extend(_validate_runtime_artifacts(lifecycle, role_map, declared_crates))
    findings.extend(_validate_build_policy(lifecycle, declared_crates))
    return findings


def _validate_primary_roles(
    lifecycle: dict[str, Any],
    role_map: dict[str, dict[str, Any]],
) -> list[host.Finding]:
    findings: list[host.Finding] = []
    build_role = str(lifecycle.get("build_runner_role") or "")
    deploy_role = str(lifecycle.get("deploy_target_role") or "")
    expectations = {
        "build_runner_role": ("build_node", "build_and_publish", "build"),
        "deploy_target_role": ("kernel_node", "kernel_primary", "deploy"),
    }
    for field, (expected_class, expected_mode, expected_capability) in expectations.items():
        role = build_role if field == "build_runner_role" else deploy_role
        if not role:
            findings.append(
                host.Finding(
                    "critical",
                    "lifecycle_primary_role_missing",
                    f"infra/registry.json:lifecycle.kernel_release.{field}",
                    f"{field} must be declared.",
                )
            )
            continue
        item = role_map.get(role)
        if item is None:
            findings.append(
                host.Finding(
                    "critical",
                    "lifecycle_primary_role_unmapped",
                    f"infra/registry.json:lifecycle.kernel_release.{field}",
                    f"role {role!r} is not declared in heterogeneous-runtime-matrix-v1.json.",
                )
            )
            continue
        if item.get("class") != expected_class:
            findings.append(
                host.Finding(
                    "critical",
                    "lifecycle_primary_role_class_mismatch",
                    f"infra/registry.json:lifecycle.kernel_release.{field}",
                    f"role {role!r} must have class {expected_class!r}, got {item.get('class')!r}.",
                )
            )
        if item.get("activation_mode") != expected_mode:
            findings.append(
                host.Finding(
                    "critical",
                    "lifecycle_primary_role_activation_mode_mismatch",
                    f"infra/registry.json:lifecycle.kernel_release.{field}",
                    f"role {role!r} must have activation_mode {expected_mode!r}, got {item.get('activation_mode')!r}.",
                )
            )
        capabilities = item.get("capabilities", {}) if isinstance(item.get("capabilities"), dict) else {}
        if capabilities.get(expected_capability) is not True:
            findings.append(
                host.Finding(
                    "critical",
                    "lifecycle_primary_role_capability_mismatch",
                    f"infra/registry.json:lifecycle.kernel_release.{field}",
                    f"role {role!r} must declare capabilities.{expected_capability}=true.",
                )
            )
    return findings


def _validate_activation_requirements(
    lifecycle: dict[str, Any],
    role_map: dict[str, dict[str, Any]],
) -> list[host.Finding]:
    findings: list[host.Finding] = []
    activation_requirements = lifecycle.get("activation_requirements", {})
    if not isinstance(activation_requirements, dict):
        return findings
    for role, requirement in sorted(activation_requirements.items()):
        item = role_map.get(str(role))
        pointer = f"infra/registry.json:lifecycle.kernel_release.activation_requirements.{role}"
        if item is None:
            findings.append(
                host.Finding(
                    "critical",
                    "lifecycle_activation_requirement_role_unmapped",
                    pointer,
                    f"activation requirement role {role!r} is not declared in heterogeneous-runtime-matrix-v1.json.",
                )
            )
            continue
        local_agents = set(item.get("required_local_agents", []) or [])
        if "cynic-node" not in local_agents:
            findings.append(
                host.Finding(
                    "critical",
                    "lifecycle_activation_requirement_missing_cynic_node",
                    pointer,
                    f"role {role!r} must require cynic-node as a local agent.",
                )
            )
        if not isinstance(requirement, dict):
            findings.append(
                host.Finding(
                    "critical",
                    "lifecycle_activation_requirement_invalid",
                    pointer,
                    "activation requirement entry must be an object.",
                )
            )
    return findings


def _validate_runtime_artifacts(
    lifecycle: dict[str, Any],
    role_map: dict[str, dict[str, Any]],
    declared_crates: set[str],
) -> list[host.Finding]:
    findings: list[host.Finding] = []
    artifacts = lifecycle.get("runtime_artifacts", [])
    if not isinstance(artifacts, list):
        return findings
    allowed_virtual_targets = {"cortex", "all_runtime_nodes"}
    for index, item in enumerate(artifacts):
        if not isinstance(item, dict):
            continue
        pointer = f"infra/registry.json:lifecycle.kernel_release.runtime_artifacts[{index}]"
        for target in item.get("runtime_targets", []) or []:
            target_name = str(target)
            if target_name in allowed_virtual_targets:
                continue
            if target_name not in role_map:
                findings.append(
                    host.Finding(
                        "critical",
                        "lifecycle_runtime_artifact_target_unmapped",
                        pointer,
                        f"runtime target {target_name!r} is neither a virtual target nor a heterogeneous runtime role.",
                    )
                )
        build_targets = item.get("build_targets", []) or []
        for target in build_targets:
            target_name = str(target)
            if target_name.startswith("crates/") and target_name not in declared_crates:
                findings.append(
                    host.Finding(
                        "critical",
                        "lifecycle_runtime_artifact_build_target_unknown",
                        pointer,
                        f"build target {target_name!r} is not declared in rust-crate-roles-v1.json.",
                    )
                )
    return findings


def _validate_build_policy(
    lifecycle: dict[str, Any],
    declared_crates: set[str],
) -> list[host.Finding]:
    findings: list[host.Finding] = []
    build_policy = lifecycle.get("build_policy", {})
    artifact_targets = build_policy.get("artifact_targets", []) if isinstance(build_policy, dict) else []
    for target in artifact_targets or []:
        target_name = str(target)
        if target_name.startswith("crates/") and target_name not in declared_crates:
            findings.append(
                host.Finding(
                    "critical",
                    "lifecycle_build_policy_artifact_target_unknown",
                    "infra/registry.json:lifecycle.kernel_release.build_policy.artifact_targets",
                    f"artifact target {target_name!r} is not declared in rust-crate-roles-v1.json.",
                )
            )
    return findings
