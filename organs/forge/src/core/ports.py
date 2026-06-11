from abc import ABC, abstractmethod
from .entities import RepoState

class IGitPerceiver(ABC):
    @abstractmethod
    def gather_state(self) -> RepoState:
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
