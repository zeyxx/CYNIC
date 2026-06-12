from abc import ABC, abstractmethod
from typing import Tuple
from .entities import JupyterState

class JupyterPort(ABC):
    @abstractmethod
    def fetch_status(self) -> Tuple[str, int]:
        """Returns (status: str, active_kernels: int)"""
        pass

class StatePort(ABC):
    @abstractmethod
    def save_state(self, state: JupyterState) -> None:
        pass

class KernelObserverPort(ABC):
    @abstractmethod
    def observe(self, state: JupyterState) -> None:
        pass
