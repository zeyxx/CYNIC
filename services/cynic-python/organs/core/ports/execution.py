# Tier 3
from abc import ABC, abstractmethod
from typing import Tuple, Optional

class ExecutorPort(ABC):
    @abstractmethod
    def execute(self, task: dict, organ_dir: str) -> Tuple[Optional[str], Optional[str]]:
        """Execute the task and return (result, error_msg)"""
        pass
