"""CYNIC-specific integration summary for external framework maps."""

from __future__ import annotations

from pathlib import Path
from typing import Any


FRAMEWORK_CYNIC_INTEGRATION_SCHEMA = "cynic.framework_cynic_integration.v1"


def cynic_integration_from_reports(
    *,
    repo_root: Path,
    framework: str,
    capabilities: dict[str, Any],
    source_inventory: dict[str, Any],
    command_map: dict[str, Any] | None = None,
    skill_bridge: dict[str, Any] | None = None,
    kanban_mirror: dict[str, Any] | None = None,
) -> dict[str, Any]:
    capability_ids = {
        str(item.get("id", ""))
        for item in capabilities.get("capabilities", [])
        if isinstance(item, dict)
    }
    command_names = {
        str(item.get("name", ""))
        for item in source_inventory.get("cli_commands", [])
        if isinstance(item, dict)
    }
    if isinstance(command_map, dict):
        command_names.update(str(item).strip() for item in command_map.get("root_cli_commands", []) if str(item).strip())
    tool_names = {
        str(item.get("name", ""))
        for item in source_inventory.get("tools", [])
        if isinstance(item, dict)
    }

    bridges = [
        _bridge(
            "execution_bloodstream",
            status=_status(
                _exists(repo_root, "services/ouroboros-runtime/src/ouroboros_runtime/core/adapters/hermes_kanban_executor.py"),
                True,
            ),
            evidence=[
                "services/ouroboros-runtime/src/ouroboros_runtime/core/adapters/hermes_kanban_executor.py",
                "docs/organs/HERMES_AGENT.md",
            ],
            rationale="Hermes execution must emit bloodstream events and normalized results instead of remaining a black box.",
        ),
        _bridge(
            "repo_coordination_claims",
            status=_status(
                _exists(repo_root, "services/coordination-runtime/src/cynic_coord_runtime/adapters/kernel_rest.py"),
                True,
            ),
            evidence=[
                "services/coordination-runtime/src/cynic_coord_runtime/adapters/kernel_rest.py",
                "docs/api/openapi.json",
                "docs/organs/HERMES_AGENT.md",
            ],
            rationale="Repo-affecting Hermes tasks must register, claim, and release through the Kernel coordination bus.",
        ),
        _bridge(
            "fleet_model_governance",
            status=_status(
                _exists(repo_root, "organs/contracts/src/adapters/hermes_fleet_model.py")
                and _exists(repo_root, "services/ouroboros-runtime/src/ouroboros_runtime/core/adapters/hermes_fleet_config_actuator.py"),
                True,
            ),
            evidence=[
                "organs/contracts/src/adapters/hermes_fleet_model.py",
                "organs/contracts/src/adapters/hermes_fleet_config_projection.py",
                "services/ouroboros-runtime/src/ouroboros_runtime/core/adapters/hermes_fleet_config_actuator.py",
            ],
            rationale="Hermes provider/model state must resolve to a fleet backend declared by CYNIC, not arbitrary local picker state.",
        ),
        _bridge(
            "framework_mapping_ssot",
            status=_status(
                _exists(repo_root, "organs/framework-map/src/entrypoints/cli.py"),
                True,
            ),
            evidence=[
                "organs/framework-map/src/entrypoints/cli.py",
                "organs/contracts/src/adapters/hermes_agent_capabilities.py",
            ],
            rationale="Hermes capability mapping should be generated from source and binary evidence, not maintained as prose only.",
        ),
        _bridge(
            "browser_to_prospect_batch",
            status=_status(
                _exists(repo_root, "organs/prospect/src/core/browser_results.py")
                and _exists(repo_root, "organs/prospect/src/core/browser_tasks.py"),
                "browser_computer_use" in capability_ids,
            ),
            evidence=[
                "organs/prospect/src/core/browser_tasks.py",
                "organs/prospect/src/core/browser_results.py",
            ],
            rationale="Browser evidence must convert into structured prospect batches with canonical public website URLs.",
        ),
        _bridge(
            "mcp_exposure_governance",
            status=_status(
                _exists(repo_root, "infra/agent-exposure.json")
                and _exists(repo_root, "crates/cynic-mcp/src/proxy.rs"),
                "mcp" in command_names or "mcp_tooling" in capability_ids,
            ),
            evidence=[
                "infra/agent-exposure.json",
                "crates/cynic-mcp/src/proxy.rs",
            ],
            rationale="Hermes MCP and plugin surfaces must be filtered by the organism exposure SSOT.",
        ),
        _bridge(
            "command_map_generation",
            status="implemented" if isinstance(command_map, dict) and command_map else "missing",
            evidence=[
                "organs/framework-map/src/adapters/hermes_command_map_discovery.py",
                "docs/ops/hermes-agent-command-map-2026-07-03.json",
            ],
            rationale="Hermes command mapping should be materialized from the installed CLI and source checkout instead of staying a static doc snapshot.",
        ),
        _bridge(
            "kanban_kernel_graph_mirror",
            status=_kanban_mirror_status("kanban" in command_names, kanban_mirror),
            evidence=[
                "organs/framework-map/src/adapters/hermes_kanban_mirror.py",
                "services/ouroboros-runtime/src/ouroboros_runtime/core/adapters/hermes_kanban_executor.py",
                "docs/organs/HERMES_AGENT.md",
            ],
            rationale="Execution events exist, but the full Hermes kanban graph is not mirrored into Kernel-native task/comment/blocker state.",
        ),
        _bridge(
            "skill_bridge",
            status=_skill_bridge_status("skill_lifecycle" in capability_ids, skill_bridge),
            evidence=[
                "organs/framework-map/src/adapters/hermes_skill_bridge.py",
                "docs/organs/HERMES_AGENT.md",
                "organs/twitter/" + "state/SKILL.md",
            ],
            rationale="CYNIC organ skills and Hermes skills still lack a canonical import/export and curation bridge.",
        ),
        _bridge(
            "organ_scoped_tool_policy",
            status="partial" if tool_names else "missing",
            evidence=[
                "infra/agent-policy.toml",
                "docs/organs/HERMES_AGENT.md",
            ],
            rationale="Filesystem and process policy exists, but organ-scoped Hermes tool allowlists are not yet materialized as a formal SSOT.",
        ),
    ]

    high_priority_gaps = [gap for gap in (_gap(bridge) for bridge in bridges) if gap is not None]

    return {
        "schema": FRAMEWORK_CYNIC_INTEGRATION_SCHEMA,
        "framework": framework,
        "command_surface": {
            "root_command_count": len(command_names),
            "root_commands": sorted(command_names),
            "tool_count": len(tool_names),
            "tool_names_sample": sorted(tool_names)[:80],
            "toolset_count": int(source_inventory.get("toolset_count", 0) or 0),
            "provider_count": int(source_inventory.get("provider_count", 0) or 0),
            "plugin_count": int(source_inventory.get("plugin_count", 0) or 0),
            "optional_mcp_count": int(source_inventory.get("optional_mcp_count", 0) or 0),
        },
        "bridges": bridges,
        "implemented_bridge_count": sum(1 for item in bridges if item["status"] == "implemented"),
        "partial_bridge_count": sum(1 for item in bridges if item["status"] == "partial"),
        "missing_bridge_count": sum(1 for item in bridges if item["status"] == "missing"),
        "high_priority_gaps": high_priority_gaps,
    }


def _bridge(bridge_id: str, *, status: str, evidence: list[str], rationale: str) -> dict[str, Any]:
    return {
        "id": bridge_id,
        "status": status,
        "evidence": evidence,
        "rationale": rationale,
    }


def _gap(bridge: dict[str, Any]) -> dict[str, Any] | None:
    status = bridge.get("status")
    if status not in {"missing", "partial"}:
        return None
    priority = "p1" if status == "missing" else "p2"
    return {
        "id": bridge.get("id"),
        "priority": priority,
        "status": status,
        "why_it_blocks_cashflow": bridge.get("rationale"),
        "evidence": bridge.get("evidence", []),
    }


def _exists(repo_root: Path, relative: str) -> bool:
    return (repo_root / relative).exists()


def _status(condition: bool, relevant: bool) -> str:
    if not relevant:
        return "not_applicable"
    return "implemented" if condition else "missing"


def _skill_bridge_status(relevant: bool, skill_bridge: dict[str, Any] | None) -> str:
    if not relevant:
        return "not_applicable"
    if not isinstance(skill_bridge, dict) or not skill_bridge:
        return "missing"
    if int(skill_bridge.get("repo_operational_skill_count", 0) or 0) > 0:
        return "implemented"
    return "partial"


def _kanban_mirror_status(relevant: bool, kanban_mirror: dict[str, Any] | None) -> str:
    if not relevant:
        return "not_applicable"
    if not isinstance(kanban_mirror, dict) or not kanban_mirror:
        return "missing"
    return str(kanban_mirror.get("status") or "missing")
