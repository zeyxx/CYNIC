# Tier 3
from abc import ABC, abstractmethod
from typing import Optional, List, Dict
from cynic_observe_runtime.ports.observation import ObservationPort
from cynic_reliability_runtime.ports.service_liveness import ServiceLivenessPort


class KernelPort(ObservationPort, ServiceLivenessPort, ABC):
    @abstractmethod
    def wait_for_tasks(self, timeout_secs: float, limit: int = 1) -> List[Dict]:
        pass

    @abstractmethod
    def poll_tasks(self, limit: int = 1) -> List[Dict]:
        pass

    @abstractmethod
    def claim_task(self, task: dict, agent_id: str) -> bool:
        pass

    @abstractmethod
    def complete_task(self, task: dict, result: Optional[str], error: Optional[str]) -> bool:
        pass

    @abstractmethod
    def release_task(self, task: dict, success: bool):
        pass

    @abstractmethod
    def check_soma_gate(self, task_id: str) -> dict:
        pass

    @abstractmethod
    def coord_register(self, agent_id: str, agent_type: str, intent: str) -> Optional[str]:
        pass
        
    @abstractmethod
    def coord_release(self, agent_id: str):
        pass

    @abstractmethod
    def coord_claim_targets(self, agent_id: str, targets: List[str]) -> Optional[str]:
        pass
