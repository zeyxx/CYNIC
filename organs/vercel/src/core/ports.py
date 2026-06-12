from typing import List, Dict, Any
from abc import ABC, abstractmethod
from src.core.entities import Deployment, VercelState

class VercelPort(ABC):
    @abstractmethod
    def fetch_deployments(self) -> List[Deployment]:
        pass

class StatePort(ABC):
    @abstractmethod
    def save_state(self, state: VercelState) -> None:
        pass

class DispatcherPort(ABC):
    @abstractmethod
    def dispatch_task(self, payload: Dict[str, Any]) -> bool:
        pass
