import dataclasses
from typing import List
from datetime import datetime, timezone

@dataclasses.dataclass
class JupyterState:
    timestamp: str
    status: str
    active_kernels: int
    recent_notebooks: List[str]
    memory_usage_mb: float

    @staticmethod
    def create(status: str, active_kernels: int) -> 'JupyterState':
        return JupyterState(
            timestamp=datetime.now(timezone.utc).isoformat(),
            status=status,
            active_kernels=active_kernels,
            recent_notebooks=[],
            memory_usage_mb=0.0
        )
