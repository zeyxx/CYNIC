from abc import ABC, abstractmethod
from typing import List
from src.domain.entities import Target, Observation

class SensorPort(ABC):
    """
    Inbound Port: Defines how the radar interacts with external targets.
    Implemented by Adapters (HttpSensor, RpcSensor, etc.)
    """
    @abstractmethod
    async def perceive(self, target: Target) -> Observation:
        pass

class ObservationRepositoryPort(ABC):
    """
    Outbound Port: Defines how observations are stored.
    Implemented by Adapters (JsonFileRepository, SurrealDBRepository)
    """
    @abstractmethod
    def save(self, observation: Observation) -> None:
        pass
