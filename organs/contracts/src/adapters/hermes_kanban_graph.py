"""Helper builders for Hermes kanban graph events and execution results."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
from typing import Any

try:
    from organs.contracts.src.core.hermes_kanban_graph import (
        HermesExecutionResult,
        HermesKanbanGraphEvent,
    )
except ModuleNotFoundError:
    _core_path = Path(__file__).resolve().parent.parent / "core" / "hermes_kanban_graph.py"
    spec = importlib.util.spec_from_file_location("cynic_contracts_hermes_kanban_graph_core", _core_path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"unable to load Hermes kanban graph core contract from {_core_path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    HermesExecutionResult = module.HermesExecutionResult
    HermesKanbanGraphEvent = module.HermesKanbanGraphEvent


def build_hermes_kanban_graph_event(
    *,
    task_id: str,
    task_domain: str,
    event: str,
    hermes_kanban_task_id: str | None = None,
    status: str | None = None,
    payload: dict[str, Any] | None = None,
) -> dict[str, Any]:
    return HermesKanbanGraphEvent(
        task_id=task_id,
        task_domain=task_domain,
        event=event,
        hermes_kanban_task_id=hermes_kanban_task_id,
        status=status,
        payload=payload,
    ).to_dict()


def build_hermes_execution_result(
    *,
    task_id: str,
    domain: str,
    executor: str,
    hermes_kanban_task_id: str,
    status: str,
    output: str,
) -> dict[str, Any]:
    return HermesExecutionResult(
        task_id=task_id,
        domain=domain,
        executor=executor,
        hermes_kanban_task_id=hermes_kanban_task_id,
        status=status,
        output=output,
    ).to_dict()
