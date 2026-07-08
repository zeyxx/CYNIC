"""Deterministic organ convergence audit."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import json
import re
from typing import Any


COMMUNICATION_ORGANS = {"discord", "slack", "telegram", "twitter"}
MESH_ORGANS = {"freebox", "headscale", "wireguard"}
WORKSPACE_ORGANS = {"jupyter", "kanban-ui", "superteam"}
BUSINESS_WORKFLOW_ORGANS = {"percolator", "prospect"}
OBSERVATION_SURFACES = {"framework-map", "oracle", "prompt-enricher", "radar", "reflection", "telemetry"}
CORE_ORGANS = {"contracts", "exposure", "forge", "supervisor"}
RUNTIME_ORGANS = {"fleet", "lifecycle"}
STORAGE_ORGANS = {"keep", "rag"}
DOMAIN_LIBRARY_ORGANS = {"solana"}


@dataclass(frozen=True)
class OrganSummary:
    name: str
    path: str
    id: str | None
    domain: str | None
    status: str | None
    files: int
    has_core: bool
    has_adapters: bool
    has_entrypoints: bool


def build_convergence_report(root: Path) -> dict[str, Any]:
    organs = [_summarize_organ(path) for path in sorted((root / "organs").iterdir()) if path.is_dir()]
    families = _families(organs)
    actions = _actions(organs)
    return {
        "schema": "cynic.organ_convergence.v1",
        "status": "advisory",
        "principle": "An organ is a durable bounded subsystem, not a folder theme. Prefer absorbing capabilities into existing organs over creating new organs.",
        "counts": {
            "organs": len(organs),
            "families": len(families),
            "keep_as_organs_now": len(actions["keep_as_organs_now"]),
            "absorb_or_merge": len(actions["absorb_or_merge"]),
            "move_out_of_organs": len(actions["move_out_of_organs"]),
            "burn_or_archive": len(actions["burn_or_archive"]),
        },
        "families": families,
        "actions": actions,
        "organ_summaries": [organ.__dict__ for organ in organs],
    }


def _summarize_organ(path: Path) -> OrganSummary:
    manifest = path / "MANIFEST.yaml"
    text = manifest.read_text(encoding="utf-8", errors="replace") if manifest.exists() else ""
    return OrganSummary(
        name=path.name,
        path=f"organs/{path.name}/",
        id=_field(text, "id") or _field(text, "name"),
        domain=_field(text, "domain"),
        status=(_field(text, "status") or None),
        files=sum(1 for item in path.rglob("*") if item.is_file()),
        has_core=(path / "src" / "core").is_dir(),
        has_adapters=(path / "src" / "adapters").is_dir(),
        has_entrypoints=(path / "src" / "entrypoints").is_dir(),
    )


def _field(text: str, name: str) -> str | None:
    match = re.search(rf"^{name}:\s*(.+?)\s*$", text, flags=re.MULTILINE)
    return match.group(1).strip().strip('"').strip("'") if match else None


def _families(organs: list[OrganSummary]) -> dict[str, list[str]]:
    names = {organ.name for organ in organs}
    return {
        "governance_control": sorted(names & CORE_ORGANS),
        "runtime_convergence": sorted(names & RUNTIME_ORGANS),
        "storage_memory": sorted(names & STORAGE_ORGANS),
        "mesh_network": sorted(names & MESH_ORGANS),
        "communications": sorted(names & COMMUNICATION_ORGANS),
        "observation_analysis": sorted(names & OBSERVATION_SURFACES),
        "domain_libraries": sorted(names & DOMAIN_LIBRARY_ORGANS),
        "workspaces": sorted(names & WORKSPACE_ORGANS),
        "business_workflows": sorted(names & BUSINESS_WORKFLOW_ORGANS),
        "unclassified": sorted(
            names
            - CORE_ORGANS
            - RUNTIME_ORGANS
            - STORAGE_ORGANS
            - MESH_ORGANS
            - COMMUNICATION_ORGANS
            - OBSERVATION_SURFACES
            - DOMAIN_LIBRARY_ORGANS
            - WORKSPACE_ORGANS
            - BUSINESS_WORKFLOW_ORGANS
        ),
    }


def _actions(organs: list[OrganSummary]) -> dict[str, list[dict[str, str]]]:
    names = {organ.name for organ in organs}
    keep = [
        _action(name, "keep", "structural nucleus or active bounded subsystem")
        for name in sorted(names & (CORE_ORGANS | RUNTIME_ORGANS | STORAGE_ORGANS | {"browser", "solana"}))
    ]
    absorb = [
        _action(name, "absorb_into_communications", "transport-specific social organs should converge behind one communications membrane")
        for name in sorted(names & COMMUNICATION_ORGANS)
    ]
    absorb += [
        _action(name, "absorb_into_mesh_network", "provider-specific mesh organs should converge behind one network membrane")
        for name in sorted(names & MESH_ORGANS)
    ]
    absorb += [
        _action(name, "absorb_into_observation_analysis", "small analysis surfaces should become capabilities of a single sensing/analysis organ unless independently scheduled")
        for name in sorted(names & OBSERVATION_SURFACES)
    ]
    move = [
        _action(name, "move_to_apps_or_workspaces", "operator UI/workbench surfaces do not need full organ biology unless they own a daemon lifecycle")
        for name in sorted(names & WORKSPACE_ORGANS)
    ]
    move += [
        _action(name, "move_to_domain_workflow_or_apps", "business workflows should be product/workflow modules unless they own persistent sensory-motor runtime")
        for name in sorted(names & BUSINESS_WORKFLOW_ORGANS)
    ]
    burn = [
        _action(name, "burn_or_absorb", "role is too narrow or unclear for an independent organ")
        for name in sorted(names & {"gasdf", "domain"})
    ]
    return {
        "keep_as_organs_now": keep,
        "absorb_or_merge": absorb,
        "move_out_of_organs": move,
        "burn_or_archive": burn,
    }


def _action(name: str, action: str, reason: str) -> dict[str, str]:
    return {
        "organ": f"organs/{name}/",
        "action": action,
        "reason": reason,
    }


def report_json(root: Path) -> str:
    return json.dumps(build_convergence_report(root), indent=2, sort_keys=True)
