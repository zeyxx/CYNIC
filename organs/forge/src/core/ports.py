from abc import ABC, abstractmethod
from typing import AsyncGenerator

from .entities import RepoState


class CoordinationPort(ABC):
    @abstractmethod
    def register_agent(self, agent_id: str, intent: str, agent_type: str) -> dict:
        pass

    @abstractmethod
    def who(self) -> dict:
        pass

    @abstractmethod
    def claim(self, agent_id: str, target: str) -> dict:
        pass

    @abstractmethod
    def release(self, agent_id: str) -> dict:
        pass

    @abstractmethod
    def observe(self, event: dict) -> dict:
        pass

    @abstractmethod
    def dispatch_agent_task(self, task: dict) -> dict:
        pass

    @abstractmethod
    def list_pending_agent_tasks(self, kind: str, domain: str | None = None) -> list[dict]:
        pass

    @abstractmethod
    def list_completed_agent_tasks(self, kind: str, domain: str | None = None) -> list[dict]:
        pass

    @abstractmethod
    def update_agent_task_result(self, task_id: str, kind: str, domain: str, result: dict) -> dict:
        pass


class LifecycleDispatchPort(ABC):
    @abstractmethod
    def payloads(self, profile: str) -> dict:
        pass

    @abstractmethod
    def dispatch(self, profile: str) -> dict:
        pass


class InfraRegistryPort(ABC):
    @abstractmethod
    def read(self) -> dict:
        pass

    @abstractmethod
    def validate(self) -> list[dict]:
        pass


class ObservationPort(ABC):
    @abstractmethod
    def observe(self, event: dict) -> dict:
        pass

class IGitPerceiver(ABC):
    @abstractmethod
    def gather_state(self) -> RepoState:
        pass
        
    @abstractmethod
    def extract_patch(self) -> str:
        """Extracts the current workspace diff as a git patch."""
        pass

class IStateStorage(ABC):
    @abstractmethod
    def save_state(self, state: RepoState) -> None:
        pass
    
    @abstractmethod
    def load_state(self) -> RepoState:
        pass

class IHealthAuditor(ABC):
    @abstractmethod
    def audit(self, state: RepoState) -> RepoState:
        pass

class IKernelClient(ABC):
    @abstractmethod
    async def dispatch_task_and_stream(self, intent: str, patch: str) -> AsyncGenerator[str, None]:
        """
        Dispatches a task via the CYNIC Kernel and yields streaming logs.
        Raises an exception if the task fails.
        """
        pass
