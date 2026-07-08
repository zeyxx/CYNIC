"""Unify organ convergence and Rust absorption into one boundary map."""

from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import Path
from typing import Any

from organs.supervisor.src.core.organ_convergence import build_convergence_report
from organs.supervisor.src.core.rust_migration_inventory import build_rust_migration_inventory


CANONICAL_CRATE_TARGETS = {
    "crates/cynic-kernel": "durable memory, tasks, observations, proof and storage semantics",
    "crates/cynic-mcp": "agent-facing tool contracts, topology validators and portable command surfaces",
    "crates/cynic-node": "host-local runtime, service and lifecycle probes",
    "crates/cynic-inference": "deterministic classification, mechanism analysis and judgment logic",
}

ORGAN_PRIORITY = {
    "organs/percolator/": ("high", "move Odysseum/PumpSwap research out; keep Percolator as bounty/security workflow only"),
    "organs/rag/": ("high", "ensure RAG reads Kernel/library memory and does not own durable truth"),
    "organs/solana/": ("high", "keep as sensing organ; graduate deterministic Pump/PumpSwap parsing into cynic-inference"),
    "organs/supervisor/": ("high", "keep Python observation; move stable validators into Rust crates"),
    "organs/twitter/": ("medium", "converge with communications membrane rather than remaining a transport silo"),
    "organs/discord/": ("medium", "converge with communications membrane rather than remaining a transport silo"),
    "organs/telegram/": ("medium", "converge with communications membrane rather than remaining a transport silo"),
    "organs/slack/": ("medium", "converge with communications membrane rather than remaining a transport silo"),
    "organs/freebox/": ("medium", "converge provider-specific networking into mesh/network membrane"),
    "organs/headscale/": ("medium", "converge provider-specific networking into mesh/network membrane"),
    "organs/wireguard/": ("medium", "converge provider-specific networking into mesh/network membrane"),
    "organs/jupyter/": ("medium", "move workspace surface out of organ biology unless it owns a daemon lifecycle"),
    "organs/kanban-ui/": ("medium", "move UI/workbench surface out of organ biology unless it owns a daemon lifecycle"),
    "organs/superteam/": ("medium", "treat as market/comms/workflow surface, not autonomous organ"),
    "organs/domain/": ("medium", "domain deployment should be absorbed or reforged behind exposure/runtime boundaries"),
    "organs/gasdf/": ("medium", "unclear role; define as relay service or burn/absorb"),
}


@dataclass(frozen=True)
class BoundaryAction:
    priority: str
    target: str
    layer: str
    action: str
    destination: str
    reason: str


def build_organ_crate_boundary_audit(root: Path) -> dict[str, Any]:
    organ_report = build_convergence_report(root)
    rust_report = build_rust_migration_inventory(root)
    actions = _organ_actions(organ_report) + _crate_actions(rust_report) + _crate_surface_actions(root)
    actions = sorted(actions, key=lambda item: (_priority_rank(item.priority), item.layer, item.target))
    first_wave = _first_wave(actions)

    return {
        "schema": "cynic.organ_crate_boundary_audit.v1",
        "status": "advisory",
        "principle": "Organs may sense and actuate; crates carry stable deterministic contracts. Do not solve organ chaos by creating more organs or more crates.",
        "counts": {
            "actions": len(actions),
            "high": sum(1 for item in actions if item.priority == "high"),
            "medium": sum(1 for item in actions if item.priority == "medium"),
            "low": sum(1 for item in actions if item.priority == "low"),
            "organs": organ_report["counts"]["organs"],
            "rust_absorption_candidates": rust_report["counts"]["rust_absorption_candidates"],
        },
        "canonical_crate_targets": CANONICAL_CRATE_TARGETS,
        "organ_families": organ_report["families"],
        "first_wave": [item.__dict__ for item in first_wave],
        "actions": [item.__dict__ for item in actions],
    }


def _organ_actions(organ_report: dict[str, Any]) -> list[BoundaryAction]:
    actions: list[BoundaryAction] = []
    for group, verb in (
        ("absorb_or_merge", "absorb_or_merge"),
        ("move_out_of_organs", "move_out_of_organs"),
        ("burn_or_archive", "burn_or_archive"),
    ):
        for item in organ_report["actions"][group]:
            target = item["organ"]
            priority, override = ORGAN_PRIORITY.get(target, ("low", item["reason"]))
            actions.append(
                BoundaryAction(
                    priority=priority,
                    target=target,
                    layer="organ",
                    action=verb,
                    destination=_organ_destination(item["action"]),
                    reason=override,
                )
            )

    for item in organ_report["actions"]["keep_as_organs_now"]:
        target = item["organ"]
        if target in ORGAN_PRIORITY:
            priority, reason = ORGAN_PRIORITY[target]
            actions.append(
                BoundaryAction(
                    priority=priority,
                    target=target,
                    layer="organ",
                    action="keep_with_boundary",
                    destination=target,
                    reason=reason,
                )
            )
    return actions


def _crate_actions(rust_report: dict[str, Any]) -> list[BoundaryAction]:
    actions: list[BoundaryAction] = []
    for destination, paths in rust_report["crate_absorption_targets"].items():
        for path in paths:
            priority = "high" if destination in {"crates/cynic-kernel", "crates/cynic-mcp"} else "medium"
            actions.append(
                BoundaryAction(
                    priority=priority,
                    target=path,
                    layer="rust_absorption",
                    action="graduate_stable_python_contract",
                    destination=destination,
                    reason=CANONICAL_CRATE_TARGETS.get(destination, "existing crate boundary"),
                )
            )
    return actions


def _crate_surface_actions(root: Path) -> list[BoundaryAction]:
    actions: list[BoundaryAction] = []
    crates_root = root / "crates"
    if not crates_root.exists():
        return actions
    for path in sorted(item for item in crates_root.iterdir() if item.is_dir()):
        rel = f"crates/{path.name}"
        if not (path / "Cargo.toml").exists():
            actions.append(
                BoundaryAction(
                    priority="medium",
                    target=rel,
                    layer="crate_surface",
                    action="quarantine_or_move",
                    destination="outside_crates_or_declared_contract",
                    reason="directory under crates without Cargo.toml must not masquerade as a Rust crate",
                )
            )
    return actions


def _organ_destination(action: str) -> str:
    return {
        "absorb_into_communications": "communications_membrane",
        "absorb_into_mesh_network": "mesh_network_membrane",
        "absorb_into_observation_analysis": "observation_analysis_membrane",
        "move_to_apps_or_workspaces": "apps_or_workspace_surface",
        "move_to_domain_workflow_or_apps": "domain_workflow_or_apps",
        "burn_or_absorb": "archive_or_existing_membrane",
    }.get(action, "existing_membrane")


def _priority_rank(priority: str) -> int:
    return {"high": 0, "medium": 1, "low": 2}.get(priority, 3)


def _first_wave(actions: list[BoundaryAction]) -> list[BoundaryAction]:
    preferred_targets = {
        "organs/percolator/",
        "organs/rag/",
        "organs/solana/",
        "organs/supervisor/",
        "organs/supervisor/src/core/infrastructure_mcp_contract.py",
        "organs/supervisor/src/core/surface_contracts.py",
        "organs/supervisor/src/core/organ_memory_contract.py",
        "organs/supervisor/src/core/repo_topology_host.py",
    }
    selected = [item for item in actions if item.target in preferred_targets]
    return sorted(selected, key=lambda item: (_priority_rank(item.priority), item.layer, item.target))


def audit_json(root: Path) -> str:
    return json.dumps(build_organ_crate_boundary_audit(root), indent=2, sort_keys=True)
