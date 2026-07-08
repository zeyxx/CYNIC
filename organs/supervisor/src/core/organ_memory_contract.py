#!/usr/bin/env python3
"""Legacy Python mirror for organ-local memory classification validation.

The stable deterministic authority is migrating into `crates/cynic-kernel`.
Keep this module as a compatibility surface until supervisor wrappers consume
the Rust validator directly.
"""

from __future__ import annotations

import json
import subprocess
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

from organs.contracts.src.project_root import resolve_repo_root


PROJECT_ROOT = resolve_repo_root(Path(__file__).resolve())
CONTRACT_PATH = PROJECT_ROOT / "axioms" / "organ-memory-v1.json"
TRACKED_STATE_PATHS = ("organs/*/state/*", "infra/organ-anvil/*")
REQUIRED_CLASSES = {
    "raw",
    "normalized",
    "warehouse",
    "ccm",
    "outbox",
    "deadletter",
    "cache",
    "artifact",
    "checkpoint",
    "local_journal",
    "local_snapshot",
    "desired_projection",
    "local_contract",
    "operator_view",
    "local_doc",
}
REQUIRED_CLASSIFICATION_FIELDS = {
    "path",
    "organ",
    "class",
    "authority",
    "backend",
    "promotion_path",
    "next_action",
}
LOCAL_MEMORY_PREFIXES = ("organs/", "infra/organ-anvil/")


@dataclass
class Finding:
    severity: str
    kind: str
    path: str
    detail: str


def load_contract(path: Path = CONTRACT_PATH) -> dict[str, Any]:
    with path.open(encoding="utf-8") as handle:
        value = json.load(handle)
    return value if isinstance(value, dict) else {}


def git_ls_files(*pathspecs: str, root: Path = PROJECT_ROOT) -> list[str]:
    command = ["git", "-C", str(root), "ls-files", *pathspecs]
    output = subprocess.check_output(command, text=True)
    return [line for line in output.splitlines() if line]


def validate_organ_memory_contract(
    contract: dict[str, Any] | None = None,
    *,
    tracked_paths: list[str] | None = None,
    contract_path: Path = CONTRACT_PATH,
) -> list[Finding]:
    findings: list[Finding] = []
    if contract is None:
        if not contract_path.exists():
            return [
                Finding(
                    "critical",
                    "organ_memory_contract_missing",
                    str(contract_path.relative_to(PROJECT_ROOT)),
                    "organ-memory-v1 must classify organ-local state before local stores can be trusted.",
                )
            ]
        contract = load_contract(contract_path)

    if contract.get("schema") != "cynic.organ_memory.v1":
        findings.append(
            Finding(
                "critical",
                "unexpected_organ_memory_schema",
                _contract_path(contract_path),
                f"expected cynic.organ_memory.v1, got {contract.get('schema')!r}",
            )
        )

    declared_classes = set(contract.get("storage_classes", {}).keys())
    missing_classes = sorted(REQUIRED_CLASSES - declared_classes)
    if missing_classes:
        findings.append(
            Finding(
                "critical",
                "organ_memory_classes_missing",
                _contract_path(contract_path),
                f"missing storage classes: {', '.join(missing_classes)}",
            )
        )

    classifications = contract.get("state_path_classifications")
    if not isinstance(classifications, list):
        findings.append(
            Finding(
                "critical",
                "organ_memory_classifications_missing",
                _contract_path(contract_path),
                "state_path_classifications must be a list of tracked local state paths and their memory role.",
            )
        )
        classifications = []

    authorities = set(contract.get("authority_levels", {}).keys())
    classified_paths: set[str] = set()
    for entry in classifications:
        if not isinstance(entry, dict):
            findings.append(
                Finding(
                    "critical",
                    "organ_memory_classification_invalid",
                    _contract_path(contract_path),
                    "every state_path_classifications entry must be an object.",
                )
            )
            continue
        path = str(entry.get("path", ""))
        missing = sorted(field for field in REQUIRED_CLASSIFICATION_FIELDS if not entry.get(field))
        if missing:
            findings.append(
                Finding(
                    "critical",
                    "organ_memory_classification_metadata_missing",
                    path or _contract_path(contract_path),
                    f"classification is missing required fields: {', '.join(missing)}",
                )
            )
        if path in classified_paths:
            findings.append(
                Finding(
                    "critical",
                    "organ_memory_path_classified_twice",
                    path,
                    "a local memory path must have exactly one classification.",
                )
            )
        classified_paths.add(path)
        if entry.get("class") not in declared_classes:
            findings.append(
                Finding(
                    "critical",
                    "organ_memory_unknown_class",
                    path,
                    f"class {entry.get('class')!r} is not declared in storage_classes.",
                )
            )
        if entry.get("authority") not in authorities:
            findings.append(
                Finding(
                    "critical",
                    "organ_memory_unknown_authority",
                    path,
                    f"authority {entry.get('authority')!r} is not declared in authority_levels.",
                )
            )
        if path.startswith(LOCAL_MEMORY_PREFIXES) and entry.get("authority") == "organism_canonical":
            findings.append(
                Finding(
                    "critical",
                    "local_memory_claims_global_authority",
                    path,
                    "organ-local state may buffer, compress, or project; it must not be organism-canonical.",
                )
            )

    if tracked_paths is None:
        tracked_paths = git_ls_files(*TRACKED_STATE_PATHS)
    ignored_docs = {"README.md", "SKILL.md"}
    for path in sorted(tracked_paths):
        if Path(path).name in ignored_docs:
            continue
        if path not in classified_paths:
            findings.append(
                Finding(
                    "critical",
                    "tracked_local_memory_without_classification",
                    path,
                    "tracked organ-local state must be classified as cache, journal, outbox, warehouse, artifact, or legacy debt.",
                )
            )
    return findings


def report() -> dict[str, Any]:
    findings = validate_organ_memory_contract()
    return {
        "schema": "cynic.organ_memory_validation.v1",
        "ok": not any(item.severity == "critical" for item in findings),
        "findings": [asdict(item) for item in findings],
    }


def _contract_path(path: Path) -> str:
    try:
        return str(path.relative_to(PROJECT_ROOT))
    except ValueError:
        return str(path)


def main() -> int:
    payload = report()
    print(json.dumps(payload, indent=2, sort_keys=True))
    return 0 if payload["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
