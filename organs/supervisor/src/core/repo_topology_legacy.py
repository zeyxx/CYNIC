#!/usr/bin/env python3
"""Repo topology checks for transitional wrappers, legacy references, and organ/mutation drift."""

from __future__ import annotations

import fnmatch
import os
import re
import subprocess
from pathlib import Path
from typing import Any

import organs.supervisor.src.core.repo_topology_host as host
from organs.supervisor.src.core.repo_topology_entrypoints import (
    validate_active_document_script_entrypoint_policy,
    validate_entrypoint_thin_membrane_policy,
    validate_generated_dependency_directory_policy,
    validate_prohibited_script_entry_files,
    validate_top_level_source_file_policy,
    validate_tracked_executable_file_policy,
)


def validate_current_transitional_paths(contract: dict[str, Any]) -> list[host.Finding]:
    findings: list[host.Finding] = []
    declared = contract.get("current_transitional_paths", [])
    for item in declared:
        path_value = str(item.get("path", ""))
        for field in sorted(host.TRANSITIONAL_PATH_REQUIRED_FIELDS):
            if not item.get(field):
                findings.append(
                    host.Finding(
                        "critical",
                        "transitional_path_metadata_missing",
                        path_value or "current_transitional_paths",
                        f"current_transitional_paths entries must declare {field}.",
                    )
                )
        if not path_value:
            continue
        path = host.WORKSPACE_ROOT / path_value
        if not path.exists():
            findings.append(
                host.Finding(
                    "medium",
                    "transitional_path_stale",
                    path_value,
                    "Declared transitional path does not exist; remove the exception or create the bounded replacement.",
                )
            )
    return findings


def validate_closed_legacy_paths(contract: dict[str, Any]) -> list[host.Finding]:
    findings: list[host.Finding] = []
    required = {"path", "owner", "status", "replacement", "expiry_condition"}
    declared = contract.get("closed_legacy_paths", [])
    if not isinstance(declared, list):
        return [
            host.Finding(
                "critical",
                "closed_legacy_paths_invalid",
                "closed_legacy_paths",
                "closed_legacy_paths must be a list of closed legacy root metadata.",
            )
        ]

    for item in declared:
        if not isinstance(item, dict):
            findings.append(
                host.Finding(
                    "critical",
                    "closed_legacy_path_invalid",
                    "closed_legacy_paths",
                    "closed_legacy_paths entries must be objects.",
                )
            )
            continue

        path_value = str(item.get("path", ""))
        for field in sorted(required):
            if not item.get(field):
                findings.append(
                    host.Finding(
                        "critical",
                        "closed_legacy_path_metadata_missing",
                        path_value or "closed_legacy_paths",
                        f"closed_legacy_paths entries must declare {field}.",
                    )
                )
        if str(item.get("status", "")) != "legacy_blocked":
            findings.append(
                host.Finding(
                    "critical",
                    "closed_legacy_path_status_invalid",
                    path_value or "closed_legacy_paths",
                    "closed_legacy_paths entries must use status=legacy_blocked.",
                )
            )
        if not path_value:
            continue

        path = host.WORKSPACE_ROOT / path_value
        if not path.exists():
            continue
        for child in sorted(path.rglob("*")):
            if child.is_file():
                findings.append(
                    host.Finding(
                        "critical",
                        "closed_legacy_path_contains_file",
                        host.relative(child),
                        "Closed legacy paths must not contain files; reforge the entry as an organ entrypoint, make fragment, infra contract, or MCP operation.",
                    )
                )
    return findings


def transitional_wrapper_exception_paths(contract: dict[str, Any]) -> set[str]:
    return {
        str(item.get("path"))
        for item in contract.get("transitional_wrapper_file_exceptions", [])
        if item.get("path")
    }


def validate_transitional_wrapper_file_exceptions(contract: dict[str, Any]) -> list[host.Finding]:
    findings: list[host.Finding] = []
    findings.extend(
        host.validate_exception_metadata(
            contract,
            "transitional_wrapper_file_exceptions",
            {"path", "status", "owner", "classification", "replacement", "expiry_condition"},
        )
    )
    allowed = transitional_wrapper_exception_paths(contract)
    try:
        untracked = sorted(
            path
            for path in host.git_ls_untracked(*host.PROVIDER_MUTATION_ROOTS)
            if path.endswith(host.TRANSITIONAL_WRAPPER_SUFFIXES)
        )
    except (subprocess.CalledProcessError, FileNotFoundError) as exc:
        return [
            host.Finding(
                "critical",
                "transitional_wrapper_scan_failed",
                ".",
                f"unable to query git untracked transitional wrappers: {exc}",
            )
        ]

    for path in untracked:
        if path not in allowed:
            findings.append(
                host.Finding(
                    "critical",
                    "unclassified_new_transitional_wrapper",
                    path,
                    "New wrappers in transitional roots must declare transitional_wrapper_file_exceptions metadata or move behind an organ entrypoint/MCP operation.",
                )
            )

    for path in sorted(allowed):
        if not (host.WORKSPACE_ROOT / path).exists():
            findings.append(
                host.Finding(
                    "medium",
                    "transitional_wrapper_exception_stale",
                    path,
                    "Transitional wrapper exception does not match an existing file.",
                )
            )
    return findings


def misplaced_organ_exception_paths(contract: dict[str, Any]) -> set[str]:
    return {
        str(item.get("path", "")).rstrip("/") + "/"
        for item in contract.get("misplaced_organ_exceptions", [])
        if item.get("path")
    }


def validate_misplaced_organ_exceptions(contract: dict[str, Any]) -> list[host.Finding]:
    findings: list[host.Finding] = []
    findings.extend(
        host.validate_exception_metadata(
            contract,
            "misplaced_organ_exceptions",
            host.MISPLACED_ORGAN_REQUIRED_FIELDS,
            directory_paths=True,
        )
    )
    allowed = misplaced_organ_exception_paths(contract)
    scripts_root = host.WORKSPACE_ROOT / "scripts"
    if not scripts_root.exists():
        return findings

    for manifest in sorted(scripts_root.rglob("MANIFEST.yaml")):
        organ_dir = host.relative(manifest.parent).rstrip("/") + "/"
        if organ_dir not in allowed:
            findings.append(
                host.Finding(
                    "critical",
                    "organ_manifest_under_transitional_scripts",
                    host.relative(manifest),
                    "MANIFEST.yaml under scripts/ means an organ is hiding in a transitional wrapper root; move it under organs/ or declare misplaced_organ_exceptions metadata.",
                )
            )

    for path in sorted(allowed):
        if not (host.WORKSPACE_ROOT / path / "MANIFEST.yaml").exists():
            findings.append(
                host.Finding(
                    "medium",
                    "misplaced_organ_exception_stale",
                    path,
                    "Misplaced organ exception does not match a scripts/*/MANIFEST.yaml.",
                )
            )
    return findings


def iter_repo_text_files() -> list[Path]:
    ignored_parts = {".git", ".venv", "__pycache__", "node_modules", "target"}
    files: list[Path] = []
    for path in host.WORKSPACE_ROOT.rglob("*"):
        if not path.is_file():
            continue
        rel_parts = set(path.relative_to(host.WORKSPACE_ROOT).parts)
        if rel_parts & ignored_parts:
            continue
        if path.suffix not in host.LEGACY_HERMES_X_SCAN_SUFFIXES:
            continue
        files.append(path)
    return files


def validate_no_active_legacy_hermes_x_references() -> list[host.Finding]:
    findings: list[host.Finding] = []
    for path in sorted(iter_repo_text_files()):
        rel = host.relative(path)
        if host.path_matches_any(rel, host.LEGACY_HERMES_X_REFERENCE_ALLOWED_GLOBS):
            continue
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except (FileNotFoundError, UnicodeDecodeError):
            continue
        for line_no, line in enumerate(lines, start=1):
            if host.LEGACY_HERMES_X_PATH_MARKER in line:
                findings.append(
                    host.Finding(
                        "critical",
                        "active_legacy_hermes_x_reference",
                        f"{rel}:{line_no}",
                        "Active source surfaces must not reference scripts/hermes-x; use organ-twitter paths or a documented historical/legacy exception.",
                    )
                )
    return findings


def validate_legacy_hermes_x_wrapper_contract() -> list[host.Finding]:
    findings: list[host.Finding] = []
    root = host.WORKSPACE_ROOT / host.LEGACY_HERMES_X_ROOT
    if not root.exists():
        return findings

    for path in sorted(root.rglob("*.py")):
        rel = host.relative(path)
        if path.name.startswith("__init__"):
            continue
        try:
            source = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue

        if host.LEGACY_HERMES_X_FUNC_RE.search(source):
            findings.append(
                host.Finding(
                    "critical",
                    "legacy_hermes_x_runtime_implementation",
                    rel,
                    "scripts/hermes-x should only contain compatibility shims; move runtime implementations to organs/twitter/src.",
                )
            )

        if host.LEGACY_HERMES_X_WRAPPER_MARKER not in source and rel not in {
            "scripts/hermes-x/analysis/__init__.py",
            "scripts/hermes-x/consumers/__init__.py",
            "scripts/hermes-x/core/__init__.py",
            "scripts/hermes-x/data_architecture/__init__.py",
            "scripts/hermes-x/tests/__init__.py",
        }:
            findings.append(
                host.Finding(
                    "critical",
                    "legacy_hermes_x_wrapper_missing_twitter_redirect",
                    rel,
                    "compatibility wrappers under scripts/hermes-x must import from organs/twitter/src.* rather than implement logic inline.",
                )
            )

    return findings


def validate_no_infra_hermes_x_direct_refs() -> list[host.Finding]:
    findings: list[host.Finding] = []
    infra_root = host.WORKSPACE_ROOT / "infra"
    if not infra_root.exists():
        return findings

    for path in sorted(infra_root.rglob("*")):
        if not path.is_file() or path.suffix not in host.LEGACY_HERMES_X_INFRA_SCAN_SUFFIXES:
            continue
        rel = host.relative(path)
        try:
            for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
                if host.LEGACY_HERMES_X_DOMAIN_ALIAS_RE.search(line):
                    findings.append(
                        host.Finding(
                            "medium",
                            "legacy_hermes_x_domain_alias",
                            f"{rel}:{line_no}",
                            "Active infra must not carry explicit legacy hermes-x domain filters in service env.",
                        )
                    )
                if host.INFRA_HERMES_X_REF_MARKER in line:
                    findings.append(
                        host.Finding(
                            "critical",
                            "infra_legacy_hermes_x_ref",
                            f"{rel}:{line_no}",
                            "systemd/infra should not target scripts/hermes-x runtime paths; route via organs/twitter entrypoints.",
                        )
                    )
        except UnicodeDecodeError:
            continue

    return findings


def validate_legacy_hermes_x_systemd_migration() -> list[host.Finding]:
    findings: list[host.Finding] = []
    service_roots = [host.WORKSPACE_ROOT / "infra" / "systemd", host.WORKSPACE_ROOT / "organs"]
    for root in service_roots:
        if not root.exists():
            continue

        for path in sorted(root.rglob("*.service")):
            rel = host.relative(path)
            if path.name.startswith(host.LEGACY_HERMES_X_SYSTEMD_PREFIX):
                findings.append(
                    host.Finding(
                        "medium",
                        "legacy_hermes_x_systemd_filename",
                        rel,
                        "Systemd unit name still uses legacy hermes-x-* namespace; migrate to organ-twitter canonical naming or document why it remains.",
                    )
                )

            try:
                lines = path.read_text(encoding="utf-8").splitlines()
            except UnicodeDecodeError:
                continue

            for line_no, line in enumerate(lines, start=1):
                stripped = line.strip()
                if not stripped or stripped.startswith("#"):
                    continue
                match = host.LEGACY_HERMES_X_SYSTEMD_DEP_RE.match(stripped)
                if not match:
                    continue
                deps = match.group(2)
                for token in deps.replace(",", " ").split():
                    if host.LEGACY_HERMES_X_UNIT_TOKEN_RE.search(token):
                        findings.append(
                            host.Finding(
                                "medium",
                                "legacy_hermes_x_systemd_dependency",
                                f"{rel}:{line_no}",
                                "Systemd dependency graph still targets legacy hermes-x-* unit names; migrate or add a canonical alias policy.",
                            )
                        )

    return findings


def validate_runtime_state_commit_exceptions(contract: dict[str, Any]) -> list[host.Finding]:
    findings: list[host.Finding] = []
    findings.extend(
        host.validate_exception_metadata(
            contract,
            "runtime_state_commit_exceptions",
            {"status", "owner", "commit_policy", "replacement", "expiry_condition"},
        )
    )
    allowed = host.runtime_state_exception_patterns(contract)
    try:
        tracked = [
            path
            for path in host.git_ls_files()
            if any(fnmatch.fnmatchcase(path, pattern) for pattern in host.RUNTIME_STATE_GLOBS)
        ]
    except (subprocess.CalledProcessError, FileNotFoundError) as exc:
        return [
            host.Finding(
                "critical",
                "tracked_runtime_state_scan_failed",
                ".",
                f"unable to query git tracked runtime state: {exc}",
            )
        ]

    for path in sorted(tracked):
        if not host.path_matches_any(path, allowed):
            findings.append(
                host.Finding(
                    "critical",
                    "tracked_runtime_state_without_exception",
                    path,
                    "Tracked runtime state must be declared in runtime_state_commit_exceptions or removed from source control.",
                )
            )

    for pattern in sorted(allowed):
        if not any(fnmatch.fnmatchcase(path, pattern) for path in tracked):
            findings.append(
                host.Finding(
                    "medium",
                    "runtime_state_exception_stale",
                    pattern,
                    "Runtime state exception does not match any tracked file.",
                )
            )
    return findings


def validate_local_data_runtime_outputs_ignored() -> list[host.Finding]:
    try:
        visible = host.git_ls_untracked("data")
    except (subprocess.CalledProcessError, FileNotFoundError) as exc:
        return [
            host.Finding(
                "critical",
                "local_data_runtime_output_scan_failed",
                "data/",
                f"unable to query git untracked data files: {exc}",
            )
        ]

    findings: list[host.Finding] = []
    for path in visible:
        if any(fnmatch.fnmatchcase(path, pattern) for pattern in host.LOCAL_DATA_RUNTIME_OUTPUT_GLOBS):
            findings.append(
                host.Finding(
                    "critical",
                    "local_data_runtime_output_visible",
                    path,
                    "Local data-store runtime outputs must stay ignored unless promoted to curated data, reports, fixtures, or an emergence manifest.",
                )
            )
    return findings


def provider_mutation_exception_paths(contract: dict[str, Any]) -> set[str]:
    return {
        str(item.get("path"))
        for item in contract.get("provider_mutation_script_exceptions", [])
        if item.get("path")
    }


def validate_provider_mutation_script_exceptions(contract: dict[str, Any]) -> list[host.Finding]:
    findings: list[host.Finding] = []
    findings.extend(
        host.validate_exception_metadata(
            contract,
            "provider_mutation_script_exceptions",
            {"path", "status", "owner", "replacement", "guard", "expiry_condition"},
        )
    )
    allowed = provider_mutation_exception_paths(contract)
    try:
        tracked = sorted(
            set(host.git_ls_files(*host.PROVIDER_MUTATION_ROOTS))
            | set(host.git_ls_untracked(*host.PROVIDER_MUTATION_ROOTS))
            | {path for path in allowed if (host.WORKSPACE_ROOT / path).exists()}
        )
    except (subprocess.CalledProcessError, FileNotFoundError) as exc:
        return [
            host.Finding(
                "critical",
                "provider_mutation_script_scan_failed",
                ".",
                f"unable to query git tracked provider scripts: {exc}",
            )
        ]

    for path in sorted(tracked):
        if not path.endswith(host.PROVIDER_MUTATION_SUFFIXES):
            continue
        full_path = host.WORKSPACE_ROOT / path
        if not full_path.exists():
            continue
        try:
            content = full_path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            content = full_path.read_text(encoding="utf-8", errors="ignore")
        if host.PROVIDER_MUTATION_RE.search(content) and path not in allowed:
            findings.append(
                host.Finding(
                    "critical",
                    "provider_mutation_script_without_exception",
                    path,
                    "Provider mutation scripts in transitional paths need provider_mutation_script_exceptions metadata or must be migrated behind MCP/organ adapters.",
                )
            )

    for path in sorted(allowed):
        if not (host.WORKSPACE_ROOT / path).exists():
            findings.append(
                host.Finding(
                    "medium",
                    "provider_mutation_exception_stale",
                    path,
                    "Provider mutation exception does not match a tracked file.",
                )
            )
    return findings


def validate_no_hardcoded_deployment_projection_paths() -> list[host.Finding]:
    findings: list[host.Finding] = []
    projection_roots = [
        host.WORKSPACE_ROOT / "infra" / "systemd",
    ]
    projection_roots.extend(
        sorted((host.WORKSPACE_ROOT / "organs").glob("*/systemd"))
    )

    for root in projection_roots:
        if not root.exists():
            continue
        for path in sorted(root.rglob("*")):
            if not path.is_file() or path.suffix not in {".service", ".timer", ".sh", ".conf"}:
                continue
            rel = host.relative(path)
            try:
                for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
                    stripped = line.strip()
                    if not stripped or stripped.startswith("#"):
                        continue
                    if host.DEPLOYMENT_PROJECTION_PATH_RE.search(stripped):
                        findings.append(
                            host.Finding(
                                "critical",
                                "hardcoded_deployment_projection_path",
                                f"{rel}:{line_no}",
                                "Deployment projections must use registry/runtime placeholders (@CYNIC_PROJECT_ROOT@ or explicit env vars), not host paths.",
                            )
                        )
            except UnicodeDecodeError:
                continue
    return findings
