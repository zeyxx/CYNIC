"""Compatibility mirror for infra consumption contract validation.

This is intentionally focused on lifecycle/runtime surfaces first. It prevents
the organism's core execution path from drifting back to cortex-local builds,
hardcoded operator paths, or ad-hoc dependency installers.

Stable deterministic authority is migrating into crates/cynic-mcp. Keep this
module as a Python compatibility mirror until remaining callers are ported.
"""

from __future__ import annotations

import fnmatch
import json
import re
from pathlib import Path
from typing import Any

from organs.contracts.src.surface_contracts import read_json_file_or_default
from organs.contracts.src.project_root import resolve_repo_root
from organs.supervisor.src.core.repo_topology_host import Finding, relative


PROJECT_ROOT = resolve_repo_root(Path(__file__).resolve())
CONTRACT_PATH = PROJECT_ROOT / "axioms/infra-consumption-v1.json"


def load_contract(path: Path = CONTRACT_PATH) -> dict[str, Any]:
    return read_json_file_or_default(path, {})


def _iter_scoped_files(contract: dict[str, Any]) -> list[Path]:
    paths: set[Path] = set()
    for pattern in contract.get("scan_scope", []):
        for match in PROJECT_ROOT.glob(str(pattern)):
            if match.is_file():
                paths.add(match)
    return sorted(paths)


def _is_allowed_literal(line: str, contract: dict[str, Any]) -> bool:
    return any(str(literal) in line for literal in contract.get("allowed_literals", []))


def _compiled_forbidden_patterns(contract: dict[str, Any]) -> list[tuple[dict[str, Any], re.Pattern[str]]]:
    compiled: list[tuple[dict[str, Any], re.Pattern[str]]] = []
    for rule in contract.get("forbidden_patterns", []):
        compiled.append((rule, re.compile(str(rule["pattern"]))))
    return compiled


def _validate_forbidden_on_cortex_coverage(contract: dict[str, Any], path: Path) -> list[Finding]:
    findings: list[Finding] = []
    patterns = _compiled_forbidden_patterns(contract)
    for forbidden in contract.get("native_lifecycle", {}).get("forbidden_on_cortex", []):
        sample = str(forbidden)
        if not any(pattern.search(sample) for _, pattern in patterns):
            findings.append(
                Finding(
                    "critical",
                    "infra_consumption_forbidden_on_cortex_uncovered",
                    relative(path),
                    f"native_lifecycle.forbidden_on_cortex entry is not covered by forbidden_patterns: {sample}",
                )
            )
    return findings


def validate_infra_consumption_contract(path: Path = CONTRACT_PATH) -> list[Finding]:
    findings: list[Finding] = []
    if not path.is_file():
        return [
            Finding(
                "critical",
                "infra_consumption_contract_missing",
                relative(path),
                "axioms/infra-consumption-v1.json must define how organs consume infrastructure.",
            )
        ]

    try:
        contract = load_contract(path)
    except json.JSONDecodeError as exc:
        return [
            Finding(
                "critical",
                "infra_consumption_contract_invalid_json",
                relative(path),
                f"invalid JSON: {exc}",
            )
        ]

    if contract.get("schema") != "cynic.infra_consumption.v1":
        findings.append(
            Finding(
                "critical",
                "unexpected_infra_consumption_schema",
                relative(path),
                f"expected cynic.infra_consumption.v1, got {contract.get('schema')!r}",
            )
        )
    if contract.get("status") != "normative":
        findings.append(
            Finding(
                "critical",
                "infra_consumption_not_normative",
                relative(path),
                "infra consumption must be normative before it gates organism lifecycle surfaces.",
            )
        )

    for field in ["capability_roles", "required_ports", "native_lifecycle", "service_placement", "scan_scope"]:
        if not contract.get(field):
            findings.append(
                Finding(
                    "critical",
                    "infra_consumption_required_field_missing",
                    relative(path),
                    f"infra consumption contract must declare {field}.",
                )
            )

    service_placement = contract.get("service_placement", {})
    if service_placement.get("surrealdb", {}).get("role") != "storage_primary":
        findings.append(
            Finding(
                "critical",
                "surrealdb_not_bound_to_storage_primary",
                relative(path),
                "SurrealDB must be consumed through the storage_primary role.",
            )
        )

    findings.extend(_validate_forbidden_on_cortex_coverage(contract, path))

    for rule, pattern in _compiled_forbidden_patterns(contract):
        for scoped_path in _iter_scoped_files(contract):
            try:
                lines = scoped_path.read_text(encoding="utf-8").splitlines()
            except UnicodeDecodeError:
                continue
            for line_no, line in enumerate(lines, start=1):
                if _is_allowed_literal(line, contract):
                    continue
                if pattern.search(line):
                    findings.append(
                        Finding(
                            str(rule.get("severity", "critical")),
                            f"infra_consumption_forbidden_{rule['id']}",
                            f"{relative(scoped_path)}:{line_no}",
                            str(rule.get("detail", "forbidden infrastructure consumption pattern.")),
                        )
                    )

    return findings


def organism_signs_of_life_contract(path: Path = CONTRACT_PATH) -> dict[str, Any]:
    contract = load_contract(path)
    return {
        "schema": "cynic.organism_signs_of_life_contract.v1",
        "human_command": contract["organism_signs_of_life"]["human_command"],
        "machine_sources": contract["organism_signs_of_life"]["machine_sources"],
        "must_report": contract["organism_signs_of_life"]["must_report"],
    }
