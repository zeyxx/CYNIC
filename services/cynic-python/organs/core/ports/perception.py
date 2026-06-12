# Tier 3
from abc import ABC, abstractmethod
from typing import Optional, List, Dict, Tuple

class KernelPort(ABC):
    @abstractmethod
    def poll_tasks(self, limit: int = 1) -> List[Dict]:
        pass

    @abstractmethod
    def claim_task(self, task: dict) -> bool:
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
    def coord_claim(self, task: dict) -> Tuple[str, List[str], Optional[str]]:
        pass
        
    @abstractmethod
    def coord_release(self, agent_id: str):
        pass
