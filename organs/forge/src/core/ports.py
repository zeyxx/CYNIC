from abc import ABC, abstractmethod
from typing import AsyncGenerator

from .entities import RepoState

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
