# Tier 3
from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Any, Optional, Tuple

if TYPE_CHECKING:
    from ..compute_guard import ComputeGuard
    from ..coalition import CoalitionBoundary


class ExecutorPort(ABC):
    @abstractmethod
    def execute(
        self,
        task: dict,
        organ_dir: str,
        guard: Optional["ComputeGuard"] = None,
        coalition: Optional["CoalitionBoundary"] = None,
        coalition_agent_id: str = "unknown",
    ) -> Tuple[Optional[str], Optional[str]]:
        """Execute the task and return (result, error_msg).

        Args:
            task:              Kernel task dict (id, objective, domain, actions).
            organ_dir:         Organ working directory.
            guard:             Optional ComputeGuard for Niveau 1 circuit breaker.
                               Executor should call guard.tick() at polling boundaries.
                               If None, executor manages its own timeout.
            coalition:         Optional CoalitionBoundary for Option A validation.
                               If set, the executor must validate any data-generating
                               operations against this boundary before producing output.
            coalition_agent_id: Agent identity for coalition violation tracing.
        """
        pass