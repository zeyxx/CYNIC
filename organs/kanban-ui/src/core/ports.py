from abc import ABC, abstractmethod
from .entities import KanbanBoard

class IKanbanSource(ABC):
    @abstractmethod
    def fetch_tasks(self) -> KanbanBoard:
        pass

class IKanbanProjector(ABC):
    @abstractmethod
    def project(self, board: KanbanBoard) -> None:
        pass
