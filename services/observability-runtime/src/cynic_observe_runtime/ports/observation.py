from abc import ABC, abstractmethod


class ObservationPort(ABC):
    @abstractmethod
    def observe(self, domain: str, content: dict, agent_id: str) -> bool:
        """Emit a structured observation to the kernel /observe endpoint."""
        pass
