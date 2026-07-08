"""Hermes kanban graph and execution result contracts."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any


HERMES_KANBAN_GRAPH_EVENT_SCHEMA = "cynic.hermes_kanban_graph_event.v1"
HERMES_EXECUTION_RESULT_SCHEMA = "cynic.hermes.execution_result.v1"

VALID_HERMES_KANBAN_EVENTS = frozenset(
    {
        "task_received",
        "fleet_model_checked",
        "fleet_model_mismatch",
        "execution_dependencies_checked",
        "execution_dependencies_blocked",
        "execution_dependencies_remediation_bypass",
        "kanban_create_failed",
        "kanban_created",
        "decompose_attempted",
        "kanban_status",
        "kanban_completed",
        "kanban_terminal_non_success",
        "task_finished",
    }
)


def _required_text(value: Any, field: str) -> str:
    text = str(value or "").strip()
    if not text:
        raise RuntimeError(f"{field} missing")
    return text


@dataclass(frozen=True)
class HermesKanbanGraphEvent:
    task_id: str
    task_domain: str
    event: str
    executor: str = "hermes-kanban"
    hermes_kanban_task_id: str | None = None
    status: str | None = None
    payload: dict[str, Any] | None = None

    def __post_init__(self) -> None:
        _required_text(self.task_id, "hermes kanban graph event task_id")
        _required_text(self.task_domain, "hermes kanban graph event task_domain")
        _required_text(self.executor, "hermes kanban graph event executor")
        if self.event not in VALID_HERMES_KANBAN_EVENTS:
            raise RuntimeError(f"unsupported hermes kanban graph event: {self.event}")

    def to_dict(self) -> dict[str, Any]:
        data = {
            "schema": HERMES_KANBAN_GRAPH_EVENT_SCHEMA,
            "task_id": self.task_id,
            "task_domain": self.task_domain,
            "executor": self.executor,
            "event": self.event,
        }
        if self.hermes_kanban_task_id:
            data["hermes_kanban_task_id"] = self.hermes_kanban_task_id
        if self.status:
            data["status"] = self.status
        if self.payload:
            data["payload"] = self.payload
        return data


@dataclass(frozen=True)
class HermesExecutionResult:
    task_id: str
    domain: str
    executor: str
    hermes_kanban_task_id: str
    status: str
    output: str

    def __post_init__(self) -> None:
        _required_text(self.task_id, "hermes execution result task_id")
        _required_text(self.domain, "hermes execution result domain")
        _required_text(self.executor, "hermes execution result executor")
        _required_text(self.hermes_kanban_task_id, "hermes execution result hermes_kanban_task_id")
        _required_text(self.status, "hermes execution result status")

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema": HERMES_EXECUTION_RESULT_SCHEMA,
            "task_id": self.task_id,
            "domain": self.domain,
            "executor": self.executor,
            "hermes_kanban_task_id": self.hermes_kanban_task_id,
            "status": self.status,
            "output": self.output,
        }


def example_hermes_kanban_graph_event() -> dict[str, Any]:
    return HermesKanbanGraphEvent(
        task_id="agent_tasks:demo",
        task_domain="organ-forge",
        event="kanban_status",
        hermes_kanban_task_id="hk-demo",
        status="in_progress",
        payload={"source": "example"},
    ).to_dict()


def example_hermes_execution_result() -> dict[str, Any]:
    return HermesExecutionResult(
        task_id="agent_tasks:demo",
        domain="organ-forge",
        executor="hermes-kanban",
        hermes_kanban_task_id="hk-demo",
        status="completed",
        output="example output",
    ).to_dict()
