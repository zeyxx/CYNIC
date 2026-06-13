from typing import List, Dict, Any
from abc import ABC, abstractmethod
from src.core.entities import Container, DockerState

class DockerPort(ABC):
    @abstractmethod
    def get_all_containers(self) -> List[Container]:
        pass

    @abstractmethod
    def heal_service(self, compose_dir_name: str) -> bool:
        pass

class StatePort(ABC):
    @abstractmethod
    def save_state(self, state: DockerState) -> None:
        pass

class DispatcherPort(ABC):
    @abstractmethod
    def dispatch_task(self, payload: Dict[str, Any]) -> bool:
        pass
