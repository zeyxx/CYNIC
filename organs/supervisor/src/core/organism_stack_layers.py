"""Local layer entities for the L0-L7 organism stack."""

from __future__ import annotations

import os
from pathlib import Path

from organs.supervisor.src.core.organism_stack_utils import read_text


def membrane_entities(runtime: dict[str, object]) -> list[dict[str, object]]:
    status = str(runtime.get("runtime_evidence_status") or ("ready" if runtime.get("runtime_ready") else "unknown"))
    health = "ok" if runtime.get("runtime_ready") else "degraded"
    return [
        {
            "layer": "L1",
            "entity": "dedicated-cynic-runtime",
            "kind": "runtime_principal",
            "desired": "cortex CLIs run through dedicated organism user and projected cynic CLI",
            "observed": status,
            "health": health,
            "owner": "axioms/runtime-principals-v1.json",
            "next_action": runtime.get("root_evidence_command", "cynic status"),
        }
    ]


def circulation_entities() -> list[dict[str, object]]:
    rest_addr = os.environ.get("CYNIC_REST_ADDR", "").strip()
    return [
        {
            "layer": "L5",
            "entity": "kernel-bus",
            "kind": "coordination_memory",
            "desired": "tasks, claims, observations and health with read-back",
            "observed": "CYNIC_REST_ADDR set" if rest_addr else "CYNIC_REST_ADDR missing in current environment",
            "health": "unknown" if rest_addr else "degraded",
            "owner": "crates/cynic-kernel",
            "next_action": "cynic organism --json",
        }
    ]


def soma_entities(manifests: list[dict[str, object]]) -> list[dict[str, object]]:
    names = {str(item.get("name")) for item in manifests}
    return [
        {
            "layer": "L6",
            "entity": "organ-supervisor",
            "kind": "convergence_loop",
            "desired": "compute L0-L7 desired/observed drift and next actions",
            "observed": "present" if "organ-supervisor" in names else "manifest not found",
            "health": "ok" if "organ-supervisor" in names else "degraded",
            "owner": "organs/supervisor",
            "next_action": "cynic organism",
        },
        {
            "layer": "L6",
            "entity": "organ-forge",
            "kind": "lifecycle_orchestrator",
            "desired": "claim, build, validate, dispatch and observe lifecycle work",
            "observed": "present" if "organ-forge" in names else "manifest not found",
            "health": "ok" if "organ-forge" in names else "degraded",
            "owner": "organs/forge",
            "next_action": "cynic actions",
        },
    ]


def recursion_entities() -> list[dict[str, object]]:
    return [
        {
            "layer": "L6",
            "entity": "l0-l7-recursive-convergence",
            "kind": "recursion_contract",
            "desired": "each layer exposes desired, observed, health, owner, next_action and proof for the layer above",
            "observed": "organism_stack_report emits entities, patterns, actions and dispatchable_requests",
            "health": "ok",
            "owner": "organs/supervisor/src/core/organism_stack.py",
            "next_action": "cynic converge --json",
        }
    ]


def cortex_entities(root: Path) -> list[dict[str, object]]:
    codex_config = root / ".codex" / "config.toml"
    text = read_text(codex_config)
    projected = "run_cynic_mcp.py" in text and "run_browser_mcp.py" in text
    return [
        {
            "layer": "L7",
            "entity": "codex-cortex",
            "kind": "cortex_adapter",
            "desired": "MCP-first cortex adapter projected from infra/agent-exposure.json",
            "observed": "projection aligned" if projected else "projection drift",
            "health": "ok" if projected else "blocked",
            "owner": ".codex/config.toml",
            "next_action": "PYTHONPATH=. python3 organs/exposure/src/entrypoints/cli.py apply",
        }
    ]
