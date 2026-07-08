#!/usr/bin/env python3
"""Repo topology policy checks and validation aggregation."""

from __future__ import annotations

import json
import os
import re
import subprocess
from dataclasses import asdict
from pathlib import Path
from typing import Any

import organs.supervisor.src.core.repo_topology_host as host
from organs.supervisor.src.core.infra_consumption_contract import validate_infra_consumption_contract
from organs.supervisor.src.core.agentic_runtime_topology_contract import (
    validate_agentic_runtime_topology_contract,
)
from organs.supervisor.src.core.anti_polling_transition_contract import (
    validate_anti_polling_transition_contract,
)
from organs.supervisor.src.core.organism_physics_contract import (
    validate_organism_physics_contract,
)
from organs.supervisor.src.core.node_network_interfaces_contract import (
    validate_node_network_interfaces_contract,
)
from organs.supervisor.src.core.python_vital_function_placement_contract import (
    validate_python_vital_function_placement_contract,
)
from organs.supervisor.src.core.repo_topology_documents import (
    validate_document_theme_inventory_contract,
    validate_evidence_capsule_contract,
    validate_theme_topology_contract,
)
from organs.supervisor.src.core.repo_topology_legacy import (
    validate_active_document_script_entrypoint_policy,
    validate_closed_legacy_paths,
    validate_current_transitional_paths,
    validate_generated_dependency_directory_policy,
    validate_legacy_hermes_x_systemd_migration,
    validate_legacy_hermes_x_wrapper_contract,
    validate_local_data_runtime_outputs_ignored,
    validate_misplaced_organ_exceptions,
    validate_no_active_legacy_hermes_x_references,
    validate_no_hardcoded_deployment_projection_paths,
    validate_no_infra_hermes_x_direct_refs,
    validate_prohibited_script_entry_files,
    validate_provider_mutation_script_exceptions,
    validate_runtime_state_commit_exceptions,
    validate_entrypoint_thin_membrane_policy,
    validate_top_level_source_file_policy,
    validate_tracked_executable_file_policy,
    validate_transitional_wrapper_file_exceptions,
)
from organs.supervisor.src.core.git_membrane import (
    GitMembraneClosurePolicy,
    GitMembraneClosureService,
)
from organs.supervisor.src.core.organ_shape import OrganShapeService
from organs.supervisor.src.core.runtime_principals_contract import RuntimePrincipalsContractService
from organs.supervisor.src.core.topology_contract import TopologyContractService


def allowed_organ_maturity_statuses(contract: dict[str, Any]) -> set[str]:
    policy = contract.get("organ_maturity_policy", {})
    statuses = policy.get("allowed_statuses")
    if statuses:
        return {str(item).lower() for item in statuses}
    manifest_statuses = (
        contract.get("organ_shape", {})
        .get("manifest_policy", {})
        .get("allowed_statuses", [])
    )
    return {str(item).lower() for item in manifest_statuses}


def validate_organ_maturity_policy(contract: dict[str, Any]) -> list[host.Finding]:
    findings: list[host.Finding] = []
    policy = contract.get("organ_maturity_policy")
    if not isinstance(policy, dict):
        return [
            host.Finding(
                "critical",
                "organ_maturity_policy_missing",
                host.relative(host.CONTRACT_PATH),
                "repo-topology-v1 must declare organ_maturity_policy so organ states use one vocabulary.",
            )
        ]
    for field in ("allowed_statuses", "status_meanings", "minimum_exception_fields", "shape_exception_fields", "rule"):
        if not policy.get(field):
            findings.append(
                host.Finding(
                    "critical",
                    "organ_maturity_policy_metadata_missing",
                    host.relative(host.CONTRACT_PATH),
                    f"organ_maturity_policy must declare {field}.",
                )
            )

    allowed = allowed_organ_maturity_statuses(contract)
    for status in allowed:
        if status not in {str(key).lower() for key in policy.get("status_meanings", {}).keys()}:
            findings.append(
                host.Finding(
                    "critical",
                    "organ_maturity_status_meaning_missing",
                    host.relative(host.CONTRACT_PATH),
                    f"organ maturity status {status!r} must have a status_meanings entry.",
                )
            )

    for key in ("organ_boundary_exceptions", "organ_shape_exceptions"):
        for item in contract.get(key, []):
            status = item.get("status")
            path = str(item.get("path") or key)
            if status and str(status).lower() not in allowed:
                findings.append(
                    host.Finding(
                        "critical",
                        "organ_maturity_status_unknown",
                        path,
                        f"{key} status {status!r} is not declared in organ_maturity_policy.allowed_statuses.",
                    )
                )
    return findings


def validate_organ_maturity_inventory_contract(contract: dict[str, Any]) -> list[host.Finding]:
    findings: list[host.Finding] = []
    if not host.ORGAN_MATURITY_INVENTORY_PATH.exists():
        return [
            host.Finding(
                "critical",
                "organ_maturity_inventory_missing",
                host.relative(host.ORGAN_MATURITY_INVENTORY_PATH),
                "organ-maturity-inventory-v1.json must classify every immediate organ directory.",
            )
        ]

    try:
        inventory = host.load_json(host.ORGAN_MATURITY_INVENTORY_PATH)
    except json.JSONDecodeError as exc:
        return [
            host.Finding(
                "critical",
                "organ_maturity_inventory_invalid_json",
                host.relative(host.ORGAN_MATURITY_INVENTORY_PATH),
                f"{exc.msg} at line {exc.lineno} column {exc.colno}",
            )
        ]

    if inventory.get("schema") != "cynic.organ_maturity_inventory.v1":
        findings.append(
            host.Finding(
                "critical",
                "unexpected_organ_maturity_inventory_schema",
                host.relative(host.ORGAN_MATURITY_INVENTORY_PATH),
                f"expected cynic.organ_maturity_inventory.v1, got {inventory.get('schema')!r}",
            )
        )

    for name, value in sorted(inventory.get("authorities", {}).items()):
        authority = host.WORKSPACE_ROOT / value
        if not authority.exists():
            findings.append(
                host.Finding(
                    "critical",
                    "organ_maturity_inventory_authority_missing",
                    host.relative(authority),
                    f"authority {name!r} is declared by organ-maturity-inventory-v1 but does not exist.",
                )
            )

    organs = inventory.get("organs")
    if not isinstance(organs, list):
        findings.append(
            host.Finding(
                "critical",
                "organ_maturity_inventory_organs_missing",
                host.relative(host.ORGAN_MATURITY_INVENTORY_PATH),
                "organ-maturity-inventory-v1 must declare organs as a list.",
            )
        )
        organs = []

    maturity_transition_policy = inventory.get("maturity_transition_policy") or {}
    if not isinstance(maturity_transition_policy, dict):
        findings.append(
            host.Finding(
                "critical",
                "organ_maturity_inventory_transition_policy_invalid",
                host.relative(host.ORGAN_MATURITY_INVENTORY_PATH),
                "maturity_transition_policy must be an object keyed by non-terminal organ maturity.",
            )
        )
        maturity_transition_policy = {}
    allowed = allowed_organ_maturity_statuses(contract)
    maturity_order = inventory.get("maturity_order")
    if not isinstance(maturity_order, list) or not maturity_order:
        findings.append(
            host.Finding(
                "critical",
                "organ_maturity_inventory_order_missing",
                host.relative(host.ORGAN_MATURITY_INVENTORY_PATH),
                "maturity_order must declare the progression of organ maturity states.",
            )
        )
        maturity_order = []
    ordered_statuses = [str(item) for item in maturity_order if isinstance(item, str)]
    if len(ordered_statuses) != len(maturity_order) or len(set(ordered_statuses)) != len(ordered_statuses):
        findings.append(
            host.Finding(
                "critical",
                "organ_maturity_inventory_order_invalid",
                host.relative(host.ORGAN_MATURITY_INVENTORY_PATH),
                "maturity_order must contain unique non-empty string statuses.",
            )
        )
    expected_order = [
        "burn_candidate",
        "legacy_blocked",
        "quarantined",
        "embryonic",
        "transitional",
        "canonical_partial",
        "canonical",
    ]
    if ordered_statuses != expected_order:
        findings.append(
            host.Finding(
                "critical",
                "organ_maturity_inventory_order_invalid_sequence",
                host.relative(host.ORGAN_MATURITY_INVENTORY_PATH),
                "maturity_order must preserve the canonical progression burn_candidate -> ... -> canonical.",
            )
        )
    allowed_statuses = set(allowed)
    try:
        untracked_organs = set(host.git_ls_untracked("organs"))
    except (subprocess.CalledProcessError, FileNotFoundError) as exc:
        return [
            host.Finding(
                "critical",
                "organ_maturity_inventory_git_scan_failed",
                "organs/",
                f"unable to query git untracked organ files: {exc}",
            )
        ]
    seen_paths: set[str] = set()
    inventory_paths: set[str] = set()
    by_maturity: dict[str, int] = {}
    for index, item in enumerate(organs):
        item_path = str(item.get("path") or f"organs[{index}]")
        for field in sorted(host.ORGAN_MATURITY_INVENTORY_REQUIRED_FIELDS):
            if not item.get(field):
                findings.append(
                    host.Finding(
                        "critical",
                        "organ_maturity_inventory_metadata_missing",
                        item_path,
                        f"organs entries must declare {field}.",
                    )
                )
        path_value = item.get("path")
        if path_value:
            path = str(path_value).rstrip("/") + "/"
            inventory_paths.add(path)
            if path in seen_paths:
                findings.append(
                    host.Finding(
                        "critical",
                        "organ_maturity_inventory_duplicate_path",
                        path,
                        "Each immediate organ directory must appear only once in organ-maturity-inventory-v1.",
                    )
                )
            seen_paths.add(path)
            if not path.startswith("organs/") or path.count("/") != 2:
                findings.append(
                    host.Finding(
                        "critical",
                        "organ_maturity_inventory_path_not_immediate_organ",
                        path,
                        "Organ maturity inventory entries must target immediate organs/<role>/ directories.",
                    )
                )
            if not (host.WORKSPACE_ROOT / path).is_dir():
                findings.append(
                    host.Finding(
                        "critical",
                        "organ_maturity_inventory_path_missing",
                        path,
                        "Inventory entry points to an organ directory that does not exist.",
                    )
                )
        maturity = item.get("maturity")
        if maturity:
            maturity_value = str(maturity).lower()
            by_maturity[maturity_value] = by_maturity.get(maturity_value, 0) + 1
            if allowed and maturity_value not in allowed:
                findings.append(
                    host.Finding(
                        "critical",
                        "organ_maturity_inventory_status_unknown",
                        item_path,
                        f"maturity {maturity!r} is not declared in organ_maturity_policy.allowed_statuses.",
                    )
                )
            if maturity_value in host.ORGAN_MATURITY_NON_TERMINAL_STATUSES:
                transition = maturity_transition_policy.get(maturity_value)
                if not isinstance(transition, dict):
                    findings.append(
                        host.Finding(
                            "critical",
                            "organ_maturity_inventory_transition_policy_missing",
                            item_path,
                            f"maturity {maturity!r} must have a maturity_transition_policy entry.",
                        )
                    )
                else:
                    promotes_to = str(transition.get("promotes_to") or "")
                    if not promotes_to:
                        findings.append(
                            host.Finding(
                                "critical",
                                "organ_maturity_inventory_transition_target_missing",
                                item_path,
                                f"maturity {maturity!r} must declare promotes_to.",
                            )
                        )
                    elif promotes_to not in allowed_statuses:
                        findings.append(
                            host.Finding(
                                "critical",
                                "organ_maturity_inventory_transition_target_unknown",
                                item_path,
                                f"maturity_transition_policy[{maturity_value!r}].promotes_to must name an allowed maturity status.",
                            )
                        )
                    for field in sorted(host.ORGAN_MATURITY_TRANSITION_REQUIRED_FIELDS):
                        if not transition.get(field):
                            findings.append(
                                host.Finding(
                                    "critical",
                                    "organ_maturity_inventory_transition_policy_metadata_missing",
                                    item_path,
                                    f"maturity_transition_policy[{maturity_value!r}] must declare {field}.",
                                )
                            )
            if path_value and maturity_value == "canonical":
                path = str(path_value).rstrip("/") + "/"
                if any(item.startswith(path) for item in untracked_organs):
                    findings.append(
                        host.Finding(
                            "critical",
                            "canonical_organ_has_untracked_files",
                            path,
                            "Canonical organs must have a closed Git membrane; use canonical_partial or transitional until local files are tracked or removed.",
                        )
                    )
            if path_value:
                path = str(path_value).rstrip("/") + "/"
                evidence = item.get("evidence", [])
                shape_exception_paths = {
                    str(exception.get("path", "")).rstrip("/") + "/"
                    for exception in contract.get("organ_shape_exceptions", [])
                    if exception.get("path")
                }
                if "organ_shape_exceptions" in evidence and path not in shape_exception_paths:
                    findings.append(
                        host.Finding(
                            "critical",
                            "organ_maturity_inventory_stale_shape_exception_evidence",
                            path,
                            "Organ maturity evidence cites organ_shape_exceptions but no current shape exception exists for this organ.",
                        )
                    )
                if any(untracked_item.startswith(path) for untracked_item in untracked_organs) and host.ORGAN_UNTRACKED_BODY_EVIDENCE not in evidence:
                    findings.append(
                        host.Finding(
                            "critical",
                            "organ_untracked_body_missing_maturity_evidence",
                            path,
                            f"Organs with visible untracked body files must declare {host.ORGAN_UNTRACKED_BODY_EVIDENCE!r} in organ-maturity-inventory-v1 evidence.",
                        )
                    )

    actual_organs = set()
    if host.ORGANS_ROOT.exists():
        actual_organs = {
            host.relative(path).rstrip("/") + "/"
            for path in host.ORGANS_ROOT.iterdir()
            if path.is_dir()
        }
    for path in sorted(actual_organs - inventory_paths):
        findings.append(
            host.Finding(
                "critical",
                "organ_missing_maturity_inventory",
                path,
                "Every immediate organ directory must appear in organ-maturity-inventory-v1.",
            )
        )
    for path in sorted(inventory_paths - actual_organs):
        findings.append(
            host.Finding(
                "critical",
                "organ_maturity_inventory_stale_path",
                path,
                "Organ maturity inventory contains a path that is not an immediate organ directory.",
            )
        )

    totals = inventory.get("totals", {})
    if totals:
        declared_count = totals.get("organs")
        if declared_count is not None and declared_count != len(organs):
            findings.append(
                host.Finding(
                    "critical",
                    "organ_maturity_inventory_total_mismatch",
                    host.relative(host.ORGAN_MATURITY_INVENTORY_PATH),
                    f"totals.organs={declared_count!r} but inventory contains {len(organs)} entries.",
                )
            )
        declared_by_maturity = totals.get("by_maturity", {})
        if declared_by_maturity and {str(key): value for key, value in declared_by_maturity.items()} != by_maturity:
            findings.append(
                host.Finding(
                    "critical",
                    "organ_maturity_inventory_maturity_total_mismatch",
                    host.relative(host.ORGAN_MATURITY_INVENTORY_PATH),
                    f"totals.by_maturity does not match actual inventory counts {by_maturity}.",
                )
            )

    return findings


def validate_python_organ_authority_inventory_contract() -> list[host.Finding]:
    findings: list[host.Finding] = []
    path = host.PYTHON_ORGAN_AUTHORITY_INVENTORY_PATH
    if not path.exists():
        return [
            host.Finding(
                "critical",
                "python_organ_authority_inventory_missing",
                host.relative(path),
                "python-organ-authority-inventory-v1.json must classify every immediate Python organ.",
            )
        ]

    try:
        inventory = host.load_json(path)
    except json.JSONDecodeError as exc:
        return [
            host.Finding(
                "critical",
                "python_organ_authority_inventory_invalid_json",
                host.relative(path),
                f"{exc.msg} at line {exc.lineno} column {exc.colno}",
            )
        ]

    if inventory.get("schema") != "cynic.python_organ_authority_inventory.v1":
        findings.append(
            host.Finding(
                "critical",
                "unexpected_python_organ_authority_inventory_schema",
                host.relative(path),
                f"expected cynic.python_organ_authority_inventory.v1, got {inventory.get('schema')!r}",
            )
        )

    allowed = set(inventory.get("allowed_classifications", []))
    expected_allowed = {
        "python_membrane",
        "python_transitional_vital",
        "python_legacy_duplication",
    }
    if allowed != expected_allowed:
        findings.append(
            host.Finding(
                "critical",
                "python_organ_authority_allowed_classifications_invalid",
                host.relative(path),
                f"allowed_classifications must equal {sorted(expected_allowed)}.",
            )
        )

    organs = inventory.get("organs")
    if not isinstance(organs, list):
        findings.append(
            host.Finding(
                "critical",
                "python_organ_authority_organs_missing",
                host.relative(path),
                "python-organ-authority-inventory-v1 must declare organs as a list.",
            )
        )
        organs = []

    rust_roles = host.load_json(host.WORKSPACE_ROOT / "axioms/rust-crate-roles-v1.json")
    known_crates = {
        str(item.get("path"))
        for item in rust_roles.get("crates", [])
        if isinstance(item, dict) and item.get("path")
    }

    vital_contract = host.load_json(host.WORKSPACE_ROOT / "axioms/python-vital-function-placement-v1.json")
    vital_organs = {
        f"organs/{str(item.get('organ')).replace('organ-', '')}/"
        for item in vital_contract.get("function_groups", [])
        if isinstance(item, dict) and item.get("organ")
    }

    inventory_paths: set[str] = set()
    by_classification: dict[str, int] = {}
    for index, item in enumerate(organs):
        item_path = str(item.get("path") or f"organs[{index}]")
        for field in ("path", "classification", "rust_authority_targets", "reason"):
            if item.get(field) in (None, ""):
                findings.append(
                    host.Finding(
                        "critical",
                        "python_organ_authority_metadata_missing",
                        item_path,
                        f"inventory entries must declare {field}.",
                    )
                )

        rel = str(item.get("path") or "")
        if rel:
            if rel in inventory_paths:
                findings.append(
                    host.Finding(
                        "critical",
                        "python_organ_authority_duplicate_path",
                        rel,
                        "Each immediate Python organ must appear exactly once in python-organ-authority-inventory-v1.json.",
                    )
                )
            inventory_paths.add(rel)
            organ_dir = host.WORKSPACE_ROOT / rel
            if not organ_dir.exists():
                findings.append(
                    host.Finding(
                        "critical",
                        "python_organ_authority_path_missing",
                        rel,
                        "Inventory entry points to an organ path that does not exist.",
                    )
                )

        classification = item.get("classification")
        if classification not in expected_allowed:
            findings.append(
                host.Finding(
                    "critical",
                    "python_organ_authority_classification_unknown",
                    item_path,
                    f"classification {classification!r} is not allowed.",
                )
            )
        else:
            by_classification[str(classification)] = by_classification.get(str(classification), 0) + 1

        targets = item.get("rust_authority_targets", [])
        if not isinstance(targets, list):
            findings.append(
                host.Finding(
                    "critical",
                    "python_organ_authority_targets_invalid",
                    item_path,
                    "rust_authority_targets must be a list.",
                )
            )
            targets = []
        for target in targets:
            if target not in known_crates:
                findings.append(
                    host.Finding(
                        "critical",
                        "python_organ_authority_target_unknown",
                        f"{item_path}:{target}",
                        "rust_authority_targets must reference declared crate paths from rust-crate-roles-v1.json.",
                    )
                )

        if rel in vital_organs and classification == "python_membrane":
            findings.append(
                host.Finding(
                    "critical",
                    "python_organ_authority_vital_organ_misclassified",
                    rel,
                    "Organs with declared Python vital function groups must not be classified as python_membrane.",
                )
            )
        if rel and classification == "python_legacy_duplication":
            organ_dir = host.WORKSPACE_ROOT / rel
            active_legacy_paths = [organ_dir / "src" / "core" / "use_cases.py"]
            if (organ_dir / "src" / "controllers").exists():
                active_legacy_paths.extend((organ_dir / "src" / "controllers").rglob("*.py"))
            if (organ_dir / "src" / "adapters").exists():
                active_legacy_paths.extend((organ_dir / "src" / "adapters").glob("*netlink*.py"))
                active_legacy_paths.extend((organ_dir / "src" / "adapters").glob("*surreal*.py"))
            active_legacy_paths = [path for path in active_legacy_paths if path.exists()]
            for legacy_path in active_legacy_paths:
                findings.append(
                    host.Finding(
                        "critical",
                        "python_legacy_duplication_active_logic_present",
                        host.relative(legacy_path),
                        "Organs classified as python_legacy_duplication must not keep controllers, active use cases, or provider-specific actuation adapters.",
                    )
                )

    immediate_python_organs = {
        f"{host.relative(path)}/"
        for path in sorted(host.ORGANS_ROOT.iterdir())
        if path.is_dir() and (path / "src").exists() and any(path.joinpath("src").rglob("*.py"))
    }
    for rel in sorted(immediate_python_organs - inventory_paths):
        findings.append(
            host.Finding(
                "critical",
                "python_organ_authority_missing_inventory",
                rel,
                "Every immediate Python organ must be classified in python-organ-authority-inventory-v1.json.",
            )
        )
    for rel in sorted(inventory_paths - immediate_python_organs):
        findings.append(
            host.Finding(
                "critical",
                "python_organ_authority_stale_path",
                rel,
                "Inventory entry is not an immediate Python organ path.",
            )
        )

    totals = inventory.get("totals", {})
    if totals.get("organs") != len(organs):
        findings.append(
            host.Finding(
                "critical",
                "python_organ_authority_total_mismatch",
                host.relative(path),
                f"totals.organs={totals.get('organs')!r} but inventory contains {len(organs)} entries.",
            )
        )
    normalized_counts = {
        classification: by_classification.get(classification, 0)
        for classification in sorted(expected_allowed)
    }
    if totals.get("by_classification") != normalized_counts:
        findings.append(
            host.Finding(
                "critical",
                "python_organ_authority_classification_total_mismatch",
                host.relative(path),
                f"totals.by_classification does not match actual counts {normalized_counts}.",
            )
        )

    return findings


def git_membrane_closure_service() -> GitMembraneClosureService:
    return GitMembraneClosureService(
        workspace_root=host.WORKSPACE_ROOT,
        plan_path=host.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
        policy=GitMembraneClosurePolicy(
            target_required_fields=host.GIT_MEMBRANE_CLOSURE_TARGET_REQUIRED_FIELDS,
            target_classifications=host.GIT_MEMBRANE_CLOSURE_TARGET_CLASSIFICATIONS,
            wave_required_fields=host.GIT_MEMBRANE_CLOSURE_WAVE_REQUIRED_FIELDS,
            execution_required_fields=host.GIT_MEMBRANE_CLOSURE_EXECUTION_REQUIRED_FIELDS,
            proof_required_fields=host.GIT_MEMBRANE_CLOSURE_PROOF_REQUIRED_FIELDS,
        ),
        load_json=host.load_json,
        relative=host.relative,
        git_ls_untracked=host.git_ls_untracked,
        git_metadata_writable_probe=host.git_metadata_writable,
        finding=host.Finding,
    )


def validate_git_membrane_closure_plan_contract() -> list[host.Finding]:
    return git_membrane_closure_service().validate_plan_contract()


def git_membrane_closure_summary(selected_wave: int | None = None) -> dict[str, Any]:
    return git_membrane_closure_service().summary(selected_wave=selected_wave)


def git_matter_report() -> dict[str, Any]:
    return git_membrane_closure_service().matter_report()


def git_metadata_writable() -> bool:
    return os.access(host.WORKSPACE_ROOT / ".git", os.W_OK)


def git_membrane_closure_commands(selected_wave: int | None = None) -> dict[str, Any]:
    return git_membrane_closure_service().commands(selected_wave=selected_wave)


def git_membrane_closure_preflight() -> dict[str, Any]:
    return git_membrane_closure_service().preflight()


def git_membrane_closure_check(selected_wave: int | None = None) -> dict[str, Any]:
    return git_membrane_closure_service().check(selected_wave=selected_wave)


def zero_exception_report() -> dict[str, Any]:
    contract = host.load_json(host.CONTRACT_PATH)
    try:
        organ_inventory = host.load_json(host.ORGAN_MATURITY_INVENTORY_PATH)
    except (FileNotFoundError, json.JSONDecodeError):
        organ_inventory = {"organs": []}

    noncanonical_organs = [
        {
            "path": item.get("path"),
            "maturity": item.get("maturity"),
            "next_action": item.get("next_action"),
        }
        for item in organ_inventory.get("organs", [])
        if item.get("maturity") != "canonical"
    ]
    exception_groups = {
        "organ_boundary_exceptions": contract.get("organ_boundary_exceptions", []),
        "organ_shape_exceptions": contract.get("organ_shape_exceptions", []),
        "current_transitional_paths": contract.get("current_transitional_paths", []),
        "transitional_wrapper_file_exceptions": contract.get("transitional_wrapper_file_exceptions", []),
        "misplaced_organ_exceptions": contract.get("misplaced_organ_exceptions", []),
        "runtime_state_commit_exceptions": contract.get("runtime_state_commit_exceptions", []),
        "provider_mutation_script_exceptions": contract.get("provider_mutation_script_exceptions", []),
    }
    exception_count = sum(len(value) for value in exception_groups.values())
    god_objects: list[dict[str, Any]] = []
    for pattern in host.GOD_OBJECT_SCAN_GLOBS:
        for path in sorted(host.WORKSPACE_ROOT.glob(pattern)):
            if not path.is_file():
                continue
            try:
                line_count = len(path.read_text(encoding="utf-8").splitlines())
            except UnicodeDecodeError:
                continue
            if line_count > host.GOD_OBJECT_LINE_LIMIT:
                god_objects.append(
                    {
                        "path": host.relative(path),
                        "lines": line_count,
                        "limit": host.GOD_OBJECT_LINE_LIMIT,
                        "replacement": "split into organ src/core, src/adapters, src/entrypoints and focused tests",
                    }
                )
    return {
        "schema": "cynic.zero_exception_topology_report.v1",
        "ok": not noncanonical_organs and exception_count == 0 and not god_objects,
        "noncanonical_organs_count": len(noncanonical_organs),
        "noncanonical_organs": noncanonical_organs,
        "exception_count": exception_count,
        "exception_groups": exception_groups,
        "god_object_count": len(god_objects),
        "god_objects": god_objects,
    }


def organ_shape_service() -> OrganShapeService:
    return OrganShapeService(
        workspace_root=host.WORKSPACE_ROOT,
        organs_root=host.ORGANS_ROOT,
        contract_path=host.CONTRACT_PATH,
        manifest_script_reference_suffixes=host.MANIFEST_SCRIPT_REFERENCE_SUFFIXES,
        manifest_untracked_script_allowed_statuses=host.MANIFEST_UNTRACKED_SCRIPT_ALLOWED_STATUSES,
        manifest_untracked_script_required_fields=host.MANIFEST_UNTRACKED_SCRIPT_REQUIRED_FIELDS,
        load_yaml=host.load_yaml,
        relative=host.relative,
        git_ls_files=host.git_ls_files,
        git_ls_untracked=host.git_ls_untracked,
        declared_paths=host.declared_paths,
        validate_exception_metadata=host.validate_exception_metadata,
        finding=host.Finding,
    )


def validate_organ_boundaries(contract: dict[str, Any]) -> list[host.Finding]:
    return organ_shape_service().validate_organ_boundaries(contract)


def validate_organ_manifest_policy(contract: dict[str, Any]) -> list[host.Finding]:
    return organ_shape_service().validate_organ_manifest_policy(contract)


def validate_organ_manifest_state_paths(contract: dict[str, Any]) -> list[host.Finding]:
    return organ_shape_service().validate_organ_manifest_state_paths(contract)


def manifest_script_reference_path(organ_dir: Path, name: str) -> str | None:
    return organ_shape_service().manifest_script_reference_path(organ_dir, name)


def is_git_tracked_path(path: str, tracked: set[str]) -> bool:
    return OrganShapeService.is_git_tracked_path(path, tracked)


def manifest_untracked_script_status_finding(
    item: dict[str, Any],
    manifest: Path,
    index: int,
    rel: str,
) -> host.Finding | None:
    return organ_shape_service().manifest_untracked_script_status_finding(item, manifest, index, rel)


def validate_manifest_script_references() -> list[host.Finding]:
    return organ_shape_service().validate_manifest_script_references()


def has_python_sources(path: Path) -> bool:
    return OrganShapeService.has_python_sources(path)


def validate_python_organ_shape(contract: dict[str, Any]) -> list[host.Finding]:
    return organ_shape_service().validate_python_organ_shape(contract)


def validate() -> list[host.Finding]:
    findings: list[host.Finding] = []
    findings.extend(host.validate_json_contracts())
    findings.extend(host.validate_draft_contract_governance())
    findings.extend(host.validate_runtime_principals_contract())
    findings.extend(validate_infra_consumption_contract())
    findings.extend(validate_theme_topology_contract())
    findings.extend(validate_document_theme_inventory_contract())
    findings.extend(validate_evidence_capsule_contract())
    findings.extend(validate_agentic_runtime_topology_contract())
    findings.extend(validate_anti_polling_transition_contract())
    findings.extend(validate_organism_physics_contract())
    findings.extend(validate_node_network_interfaces_contract())
    findings.extend(validate_python_vital_function_placement_contract())
    findings.extend(host.validate_meta_layer_contract())
    findings.extend(host.validate_api_layer_alignment_contract())
    findings.extend(host.validate_openapi_route_contract())
    findings.extend(host.validate_api_topology_contract())
    findings.extend(host.validate_infrastructure_mcp_contract())
    findings.extend(host.validate_lxc_exec_bridge_contract())
    findings.extend(host.validate_storage_primary_activation_contract())
    topology_findings = host.validate_topology_contract()
    findings.extend(topology_findings)
    findings.extend(host.validate_organ_layer_topology_contract())
    findings.extend(host.validate_multiscale_organism_property_map_contract())
    if not any(item.kind in {"repo_topology_contract_missing", "unexpected_repo_topology_schema"} for item in topology_findings):
        contract = host.load_json(host.CONTRACT_PATH)
        findings.extend(host.validate_contract_file_policy(contract))
        findings.extend(host.validate_schema_driven_contracts())
        findings.extend(host.validate_data_constitution_rules())
        findings.extend(host.validate_rust_hexagonal_crate_policy(contract))
        findings.extend(validate_organ_maturity_policy(contract))
        findings.extend(validate_organ_maturity_inventory_contract(contract))
        findings.extend(validate_python_organ_authority_inventory_contract())
        findings.extend(host.validate_service_surface_inventory_contract())
        findings.extend(host.validate_diagnostic_surface_inventory_contract())
        findings.extend(host.validate_implementation_surface_inventory_contract())
        findings.extend(host.validate_infra_surface_inventory_contract())
        findings.extend(host.validate_data_control_surface_inventory_contract())
        findings.extend(validate_git_membrane_closure_plan_contract())
        findings.extend(validate_current_transitional_paths(contract))
        findings.extend(validate_closed_legacy_paths(contract))
        findings.extend(validate_prohibited_script_entry_files(contract))
        findings.extend(validate_tracked_executable_file_policy(contract))
        findings.extend(validate_top_level_source_file_policy(contract))
        findings.extend(validate_generated_dependency_directory_policy(contract))
        findings.extend(validate_entrypoint_thin_membrane_policy(contract))
        findings.extend(validate_active_document_script_entrypoint_policy(contract))
        findings.extend(validate_transitional_wrapper_file_exceptions(contract))
        findings.extend(validate_misplaced_organ_exceptions(contract))
        findings.extend(validate_organ_boundaries(contract))
        findings.extend(validate_organ_manifest_policy(contract))
        findings.extend(validate_organ_manifest_state_paths(contract))
        findings.extend(validate_manifest_script_references())
        findings.extend(validate_python_organ_shape(contract))
        findings.extend(validate_runtime_state_commit_exceptions(contract))
        findings.extend(validate_local_data_runtime_outputs_ignored())
        findings.extend(validate_provider_mutation_script_exceptions(contract))
        findings.extend(validate_no_active_legacy_hermes_x_references())
        findings.extend(validate_legacy_hermes_x_wrapper_contract())
        findings.extend(validate_no_infra_hermes_x_direct_refs())
        findings.extend(validate_legacy_hermes_x_systemd_migration())
        findings.extend(validate_no_hardcoded_deployment_projection_paths())
    findings.extend(validate_mcp_no_absolute_project_paths())
    return findings


def validate_mcp_no_absolute_project_paths() -> list[host.Finding]:
    findings: list[host.Finding] = []
    if not host.MCP_SRC_ROOT.exists():
        return findings

    forbidden = re.compile(r"(/" + r"home/[^\\\"'\\s]+|/" + r"Users/[^\\\"'\\s]+|Bureau/|privacy-ethereum-framework)")
    for path in sorted(host.MCP_SRC_ROOT.rglob("*.rs")):
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            if forbidden.search(line):
                findings.append(
                    host.Finding(
                        "critical",
                        "hardcoded_project_path_in_mcp",
                        f"{host.relative(path)}:{line_no}",
                        "portable MCP proxy code must use explicit parameters or CYNIC_PROJECT_ROOT, not host/project-specific paths.",
                    )
                )
    return findings
