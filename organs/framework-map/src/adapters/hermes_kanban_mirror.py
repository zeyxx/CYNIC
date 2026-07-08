"""Measure the current Hermes kanban to Kernel mirror coverage."""

from __future__ import annotations

import ast
from pathlib import Path
from typing import Any


KANBAN_MIRROR_SCHEMA = "cynic.hermes_kanban_kernel_mirror.v1"
MINIMUM_BLOODSTREAM_EVENTS = (
    "task_received",
    "kanban_created",
    "decompose_attempted",
    "kanban_status",
    "task_finished",
)
FULL_GRAPH_MIRROR_SURFACES = (
    "claim",
    "comment",
    "block",
    "unblock",
    "heartbeat",
    "link",
    "assign",
    "runs",
    "stats",
    "tail",
)


class HermesKanbanMirrorDiscovery:
    def discover(self, *, repo_root: Path, command_map: dict[str, Any] | None = None) -> dict[str, Any]:
        executor = repo_root / "services/ouroboros-runtime/src/ouroboros_runtime/core/adapters/hermes_kanban_executor.py"
        observed_events = _observed_events(executor)
        minimum_missing = [event for event in MINIMUM_BLOODSTREAM_EVENTS if event not in observed_events]
        observed_surfaces = _observed_graph_surfaces(observed_events)
        missing_graph_surfaces = [item for item in FULL_GRAPH_MIRROR_SURFACES if item not in observed_surfaces]
        kanban_subcommands = _kanban_subcommands(command_map)

        if minimum_missing:
            status = "missing"
        elif missing_graph_surfaces:
            status = "partial"
        else:
            status = "implemented"

        return {
            "schema": KANBAN_MIRROR_SCHEMA,
            "executor_path": str(executor),
            "status": status,
            "observed_events": observed_events,
            "minimum_bloodstream_events": list(MINIMUM_BLOODSTREAM_EVENTS),
            "missing_minimum_events": minimum_missing,
            "observed_graph_surfaces": observed_surfaces,
            "expected_graph_surfaces": list(FULL_GRAPH_MIRROR_SURFACES),
            "missing_graph_surfaces": missing_graph_surfaces,
            "upstream_kanban_subcommand_count": len(kanban_subcommands),
            "upstream_kanban_subcommands": kanban_subcommands,
        }


def _observed_events(path: Path) -> list[str]:
    if not path.exists():
        return []
    text = path.read_text(encoding="utf-8", errors="replace")
    try:
        tree = ast.parse(text, filename=str(path))
    except SyntaxError:
        return []
    events: set[str] = set()
    for node in ast.walk(tree):
        if not isinstance(node, ast.Call):
            continue
        func = node.func
        if not isinstance(func, ast.Attribute) or func.attr != "_observe_event":
            continue
        if len(node.args) < 4:
            continue
        event_node = node.args[3 - 1]
        if isinstance(event_node, ast.Constant) and isinstance(event_node.value, str):
            events.add(event_node.value)
    return sorted(events)


def _observed_graph_surfaces(events: list[str]) -> list[str]:
    observed: set[str] = set()
    for event in events:
        if event.startswith("kanban_"):
            suffix = event[len("kanban_") :]
            if suffix.startswith("terminal_"):
                continue
            observed.add(suffix)
        elif event in {"task_received", "task_finished"}:
            observed.add("runs")
    return sorted(observed)


def _kanban_subcommands(command_map: dict[str, Any] | None) -> list[str]:
    if not isinstance(command_map, dict):
        return []
    groups = command_map.get("command_groups", {})
    if not isinstance(groups, dict):
        return []
    execution = groups.get("agent_execution_and_coordination", {})
    if not isinstance(execution, dict):
        return []
    commands = execution.get("commands", {})
    if not isinstance(commands, dict):
        return []
    kanban = commands.get("kanban", {})
    if not isinstance(kanban, dict):
        return []
    return [str(item) for item in kanban.get("subcommands", []) if str(item).strip()]
