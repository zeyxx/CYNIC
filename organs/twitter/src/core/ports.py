from abc import ABC, abstractmethod
from .entities import TwitterSurfaceState

class ITwitterPerceiver(ABC):
    @abstractmethod
    def perceive(self) -> TwitterSurfaceState:
        pass

class IStateStorage(ABC):
    @abstractmethod
    def save_state(self, state: TwitterSurfaceState) -> None:
        pass
