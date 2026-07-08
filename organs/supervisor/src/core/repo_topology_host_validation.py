#!/usr/bin/env python3
"""Pure validation helpers used by the topology-host façade."""

from __future__ import annotations

import json
import os
import fnmatch
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable

import yaml
from organs.contracts.src.surface_contracts import (
    read_json_file_or_default,
    read_toml_file_with_loader,
    read_yaml_file_with_loader,
)
from organs.supervisor.src.core import repo_topology_host_constants as host_constants


@dataclass
class Finding:
    severity: str
    kind: str
    path: str
    detail: str


def load_json(path: Path) -> Any:
    return read_json_file_or_default(path, {})


def load_yaml(path: Path) -> Any:
    return read_yaml_file_with_loader(path, yaml)


def relative(path: Path, workspace_root: Path) -> str:
    try:
        return str(path.relative_to(workspace_root))
    except ValueError:
        return str(path)


def git_ls_files(workspace_root: Path, *pathspecs: str) -> list[str]:
    command = ["git", "-C", str(workspace_root), "ls-files", *pathspecs]
    output = subprocess.check_output(command, text=True)
    return [line for line in output.splitlines() if line]


def git_ls_untracked(workspace_root: Path, *pathspecs: str) -> list[str]:
    command = [
        "git",
        "-C",
        str(workspace_root),
        "ls-files",
        "--others",
        "--exclude-standard",
        *pathspecs,
    ]
    output = subprocess.check_output(command, text=True)
    return [line for line in output.splitlines() if line]


def path_matches_any(path: str, patterns: set[str]) -> bool:
    return any(fnmatch.fnmatchcase(path, pattern) for pattern in patterns)


def is_generated_dependency_directory(path: Path) -> bool:
    return path.name in host_constants.GENERATED_DEPENDENCY_DIRECTORY_NAMES


def iter_immediate_organ_dirs(organs_root: Path) -> list[Path]:
    if not organs_root.exists():
        return []
    return sorted(
        path
        for path in organs_root.iterdir()
        if path.is_dir() and not is_generated_dependency_directory(path)
    )


def validate_json_contracts(
    contracts_root: Path,
    relative_fn: Callable[[Path], str],
    *,
    load_json_fn: Callable[[Path], Any] = load_json,
) -> list[Finding]:
    findings: list[Finding] = []
    if not contracts_root.exists():
        return [
            Finding(
                "critical",
                "contracts_root_missing",
                relative_fn(contracts_root),
                "axioms must exist because machine-readable contracts live there.",
            )
        ]

    for path in sorted(contracts_root.rglob("*.json")):
        try:
            load_json_fn(path)
        except json.JSONDecodeError as exc:
            findings.append(
                Finding(
                    "critical",
                    "invalid_json_contract",
                    relative_fn(path),
                    f"{exc.msg} at line {exc.lineno} column {exc.colno}",
                )
            )
    return findings


def validate_schema_driven_contracts(
    contracts_root: Path,
    workspace_root: Path,
    relative_fn: Callable[[Path], str],
    *,
    load_json_fn: Callable[[Path], Any] = load_json,
) -> list[Finding]:
    findings: list[Finding] = []
    if not contracts_root.exists():
        return findings

    try:
        import jsonschema
    except ImportError:
        return findings

    for path in sorted(contracts_root.rglob("*.json")):
        rel = relative_fn(path)
        if rel.startswith("axioms/schemas/"):
            continue
        try:
            payload = load_json_fn(path)
        except json.JSONDecodeError:
            continue

        schema_ref = payload.get("$schema")
        if not schema_ref:
            continue

        schema_path = (path.parent / schema_ref).resolve()
        if not schema_path.exists():
            schema_path = (workspace_root / schema_ref).resolve()

        if not schema_path.exists():
            findings.append(
                Finding(
                    "critical",
                    "json_schema_missing",
                    rel,
                    f"Declares $schema {schema_ref!r} but the schema file does not exist.",
                )
            )
            continue

        try:
            schema_payload = load_json_fn(schema_path)
            jsonschema.validate(instance=payload, schema=schema_payload)
        except Exception as exc:
            findings.append(
                Finding(
                    "critical",
                    "json_schema_validation_failed",
                    rel,
                    f"Failed validation against schema {schema_ref!r}: {exc}",
                )
            )
    return findings


def validate_data_constitution_rules(
    *,
    workspace_root: Path,
    organs_root: Path,
    relative_fn: Callable[[Path], str],
    organ_maturity_inventory_path: Path,
    organ_manifest_schema_path: Path,
    skip_schema_validation_statuses: list[str] | set[str] = ("quarantined", "legacy_blocked", "burn_candidate"),
    load_json_fn: Callable[[Path], Any] = load_json,
    load_yaml_fn: Callable[[Path], Any] = load_yaml,
) -> list[Finding]:
    findings: list[Finding] = []
    data_constitution_path = workspace_root / "axioms/data-constitution-v1.json"
    if not data_constitution_path.exists():
        return findings
    try:
        data_constitution = load_json_fn(data_constitution_path)
    except Exception:
        return findings

    rules = data_constitution.get("data_rules", {})
    state_isolation = rules.get("state_isolation_policy", {})

    organ_names = []
    if organs_root.exists():
        organ_names = [p.name for p in iter_immediate_organ_dirs(organs_root)]

    for organ_dir in iter_immediate_organ_dirs(organs_root):
        organ_name = organ_dir.name
        for path in sorted(organ_dir.rglob("*.py")):
            if "state/" in relative_fn(path) or "tests/" in relative_fn(path) or ".venv/" in relative_fn(path) or "node_modules/" in relative_fn(path):
                continue
            try:
                content = path.read_text(encoding="utf-8")
            except Exception:
                continue

            for other_organ in organ_names:
                if other_organ == organ_name:
                    continue
                patterns = [
                    f"organs.{other_organ}.state",
                    f"organs/{other_organ}/state",
                    f"from organs.{other_organ} import state",
                ]
                for pat in patterns:
                    if pat in content:
                        findings.append(
                            Finding(
                                "critical",
                                "state_isolation_violation",
                                relative_fn(path),
                                f"Violates Loi d'Isolation d'État: accesses state of other organ {other_organ!r} via {pat!r}.",
                            )
                        )

    membrane_policy = rules.get("membrane_policy", {})
    requires_manifest_schemas = membrane_policy.get("requires_manifest_schemas", True)

    if requires_manifest_schemas and organ_maturity_inventory_path.exists():
        if not organ_manifest_schema_path.exists():
            findings.append(
                Finding(
                    "critical",
                    "organ_manifest_schema_missing",
                    relative_fn(organ_manifest_schema_path),
                    "Data constitution requires manifest schemas; the central organ manifest schema must exist.",
                )
            )
            return findings
        try:
            manifest_schema = load_json_fn(organ_manifest_schema_path)
        except json.JSONDecodeError as exc:
            findings.append(
                Finding(
                    "critical",
                    "organ_manifest_schema_invalid_json",
                    relative_fn(organ_manifest_schema_path),
                    f"{exc.msg} at line {exc.lineno} column {exc.colno}",
                )
            )
            return findings
        try:
            inventory = load_json_fn(organ_maturity_inventory_path)
            organs_list = inventory.get("organs", [])
        except Exception:
            organs_list = []

        for organ_entry in organs_list:
            organ_path = organ_entry.get("path")
            maturity = organ_entry.get("maturity")
            if maturity in skip_schema_validation_statuses:
                continue

            actual_dir = workspace_root / organ_path
            if not actual_dir.exists():
                continue
            manifest_path = actual_dir / "MANIFEST.yaml"
            if not manifest_path.exists():
                findings.append(
                    Finding(
                        "critical",
                        "organ_manifest_missing_for_schema_validation",
                        organ_path,
                        "Organ is subject to manifest schema validation but has no MANIFEST.yaml.",
                    )
                )
                continue
            try:
                manifest_payload = load_yaml_fn(manifest_path) or {}
            except yaml.YAMLError as exc:
                findings.append(
                    Finding(
                        "critical",
                        "organ_manifest_yaml_invalid_for_schema_validation",
                        relative_fn(manifest_path),
                        str(exc),
                    )
                )
                continue
            required = set(manifest_schema.get("required", []))
            missing = sorted(field for field in required if not manifest_payload.get(field))
            if missing:
                findings.append(
                    Finding(
                        "critical",
                        "organ_manifest_schema_required_field_missing",
                        relative_fn(manifest_path),
                        f"MANIFEST.yaml must satisfy organ-manifest-v1 required fields: {', '.join(missing)}.",
                    )
                )
            allowed_statuses = set(manifest_schema.get("properties", {}).get("status", {}).get("enum", []))
            status_value = str(manifest_payload.get("status") or "").lower()
            if allowed_statuses and status_value not in allowed_statuses:
                findings.append(
                    Finding(
                        "critical",
                        "organ_manifest_schema_status_unknown",
                        organ_path,
                        f"MANIFEST.yaml status {status_value!r} is not declared by organ-manifest-v1.",
                    )
                )

    return findings


def validate_draft_contract_governance(
    *,
    governed_contract_paths: list[Path],
    required_fields: set[str],
    relative_fn: Callable[[Path], str],
    load_json_fn: Callable[[Path], Any] = load_json,
) -> list[Finding]:
    findings: list[Finding] = []
    for path in governed_contract_paths:
        if not path.exists():
            continue
        try:
            contract = load_json_fn(path)
        except json.JSONDecodeError:
            continue
        if not isinstance(contract, dict) or contract.get("status") != "draft":
            continue
        for field in sorted(required_fields):
            if not contract.get(field):
                findings.append(
                    Finding(
                        "critical",
                        "draft_contract_governance_missing",
                        relative_fn(path),
                        f"draft contracts must declare {field}.",
                    )
                )
    return findings


def load_toml_with_tomllib(tomllib: Any, path: Path) -> dict[str, Any]:
    return read_toml_file_with_loader(path, tomllib)


def declared_paths(contract: dict[str, Any], key: str) -> set[str]:
    return {
        str(item.get("path", "")).rstrip("/") + "/"
        for item in contract.get(key, [])
        if item.get("path")
    }


def validate_exception_metadata(
    contract: dict[str, Any],
    key: str,
    required: set[str],
    *,
    directory_paths: bool = False,
) -> list[Finding]:
    findings: list[Finding] = []
    for index, item in enumerate(contract.get(key, [])):
        path = item.get("path")
        item_path = str(path or f"{key}[{index}]")
        for field in sorted(required):
            if not item.get(field):
                findings.append(
                    Finding(
                        "critical",
                        "topology_exception_metadata_missing",
                        item_path,
                        f"{key} entries must declare {field}.",
                    )
                )
        if directory_paths and path and not str(path).endswith("/"):
            findings.append(
                Finding(
                    "critical",
                    "topology_exception_path_not_directory",
                    item_path,
                    f"{key} paths must end with '/' to avoid ambiguous prefix matching.",
                )
            )
    return findings


def runtime_state_exception_patterns(contract: dict[str, Any]) -> set[str]:
    patterns: set[str] = set()
    for item in contract.get("runtime_state_commit_exceptions", []):
        path = item.get("path")
        if path:
            patterns.add(str(path))
        for path in item.get("paths", []):
            patterns.add(str(path))
        for pattern in item.get("patterns", []):
            patterns.add(str(pattern))
    return patterns


def git_metadata_writable(workspace_root: Path) -> bool:
    return os.access(workspace_root / ".git", os.W_OK)
