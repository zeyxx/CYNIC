from abc import ABC, abstractmethod
from typing import Optional


class HermesKanbanGraphPort(ABC):
    @abstractmethod
    def publish_hermes_kanban_event(
        self,
        *,
        task_id: str,
        task_domain: str,
        event: str,
        agent_id: str,
        hermes_kanban_task_id: Optional[str] = None,
        status: Optional[str] = None,
        payload: Optional[dict] = None,
    ) -> bool:
        """Publish a normalized Hermes Kanban graph event."""
        pass
