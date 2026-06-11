from abc import ABC, abstractmethod
from .entities import ProofOfHistory

class IPoHAggregator(ABC):
    @abstractmethod
    def aggregate_organism(self) -> ProofOfHistory:
        pass

    @abstractmethod
    def aggregate_human(self) -> ProofOfHistory:
        pass

class IStateStorage(ABC):
    @abstractmethod
    def save_poh_snapshot(self, poh: ProofOfHistory) -> None:
        pass
