"""Classify supervisor responsibilities that should graduate into Rust."""

from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import Path
from typing import Any


SUPERVISOR_REL = Path("organs/supervisor")
PYTHON_ROOTS = (
    SUPERVISOR_REL / "src" / "core",
    SUPERVISOR_REL / "controllers",
    SUPERVISOR_REL / "src" / "controllers",
)

KERNEL_TERMS = (
    "kernel",
    "memory",
    "observe",
    "observation",
    "proof",
    "storage",
    "task",
)
MCP_TERMS = (
    "api",
    "exposure",
    "mcp",
    "openapi",
    "surface",
    "tool",
)
NODE_TERMS = (
    "host",
    "infra",
    "lifecycle",
    "node",
    "principal",
    "runtime",
    "service",
)
INFERENCE_TERMS = (
    "convergence",
    "domain",
    "inference",
    "organ_layer",
    "organ_shape",
    "solana",
)
STATIC_CONTRACT_TERMS = (
    "contract",
    "topology",
    "inventory",
    "validate",
    "validator",
    "policy",
)


@dataclass(frozen=True)
class MigrationCandidate:
    path: str
    stability: str
    current_role: str
    recommended_home: str
    reason: str


def build_rust_migration_inventory(root: Path) -> dict[str, Any]:
    candidates = [_classify_python_file(root, path) for path in _supervisor_python_files(root)]
    rust_queue = [candidate for candidate in candidates if candidate.recommended_home.startswith("crates/")]
    python_queue = [candidate for candidate in candidates if candidate.recommended_home == "organs/supervisor"]

    return {
        "schema": "cynic.supervisor_rust_migration_inventory.v1",
        "status": "advisory",
        "principle": "Supervisor may observe in Python, but stable contracts and critical validators should graduate into existing Rust crates instead of becoming a Python authority.",
        "counts": {
            "python_files_scanned": len(candidates),
            "rust_absorption_candidates": len(rust_queue),
            "keep_in_supervisor_python": len(python_queue),
        },
        "crate_absorption_targets": _group_by_home(rust_queue),
        "keep_in_supervisor_python": [candidate.__dict__ for candidate in python_queue],
        "migration_queue": [candidate.__dict__ for candidate in rust_queue],
    }


def _supervisor_python_files(root: Path) -> list[Path]:
    files: list[Path] = []
    for rel_root in PYTHON_ROOTS:
        abs_root = root / rel_root
        if not abs_root.exists():
            continue
        files.extend(
            path
            for path in abs_root.rglob("*.py")
            if path.name != "__init__.py" and "__pycache__" not in path.parts
        )
    return sorted(files)


def _classify_python_file(root: Path, path: Path) -> MigrationCandidate:
    rel = _relative(root, path)
    stem = path.stem.replace("-", "_")
    text = path.read_text(encoding="utf-8", errors="replace")
    signals = " ".join((rel, stem, text[:4096])).lower()

    if stem == "rust_migration_inventory":
        return MigrationCandidate(
            path=rel,
            stability="exploratory_probe",
            current_role="python_observation_logic",
            recommended_home="organs/supervisor",
            reason="this file is the temporary scanner that produces the migration queue",
        )

    if _is_stable_contract(stem, signals):
        home = _recommended_crate(stem, signals)
        return MigrationCandidate(
            path=rel,
            stability="stable_contract_candidate",
            current_role="supervisor_python_validator",
            recommended_home=home,
            reason="contract-like deterministic validation should become a Rust module once the rule stops moving",
        )

    if "entrypoint" in rel or "/controllers/" in rel:
        return MigrationCandidate(
            path=rel,
            stability="orchestration_surface",
            current_role="python_controller",
            recommended_home="organs/supervisor",
            reason="controller and CLI glue may stay in Python while it delegates stable checks to Rust",
        )

    return MigrationCandidate(
        path=rel,
        stability="exploratory_probe",
        current_role="python_observation_logic",
        recommended_home="organs/supervisor",
        reason="scan, synthesis and discovery logic can remain mutable inside the supervisor organ",
    )


def _is_stable_contract(stem: str, signals: str) -> bool:
    return any(term in stem for term in STATIC_CONTRACT_TERMS) or (
        "schema" in signals and ("finding" in signals or "validate" in signals)
    )


def _recommended_crate(stem: str, signals: str) -> str:
    haystack = f"{stem} {signals}"
    if any(term in haystack for term in KERNEL_TERMS):
        return "crates/cynic-kernel"
    if any(term in haystack for term in MCP_TERMS):
        return "crates/cynic-mcp"
    if any(term in haystack for term in NODE_TERMS):
        return "crates/cynic-node"
    if any(term in haystack for term in INFERENCE_TERMS):
        return "crates/cynic-inference"
    return "crates/cynic-mcp"


def _group_by_home(candidates: list[MigrationCandidate]) -> dict[str, list[str]]:
    grouped: dict[str, list[str]] = {}
    for candidate in candidates:
        grouped.setdefault(candidate.recommended_home, []).append(candidate.path)
    return {home: sorted(paths) for home, paths in sorted(grouped.items())}


def _relative(root: Path, path: Path) -> str:
    try:
        return path.relative_to(root).as_posix()
    except ValueError:
        return path.as_posix()


def inventory_json(root: Path) -> str:
    return json.dumps(build_rust_migration_inventory(root), indent=2, sort_keys=True)
