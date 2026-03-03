"""
Judgment Pipeline  Context for one complete judgment cycle (DAG State).

Extracted from orchestrator.py to reduce god object complexity.
Uses Pydantic V2 for strict data lineage and immutability.
"""

from __future__ import annotations

import time
import uuid
from typing import Any, Optional

from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.core.judgment import Cell, Judgment
from cynic.kernel.organism.brain.cognition.neurons.base import DogJudgment
from pydantic import BaseModel, ConfigDict, Field


class JudgmentPipeline(BaseModel):
    """
    Immutable snapshot of a judgment cycle's progress.
    Each stage of the DAG produces a new evolved instance.
    """
    model_config = ConfigDict(frozen=True, extra="allow")

    cell: Cell
    level: ConsciousnessLevel = ConsciousnessLevel.MACRO
    fractal_depth: int = 1
    pipeline_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    trace_id: str = Field(default_factory=lambda: f"TR-{uuid.uuid4().hex[:6].upper()}")
    started_at: float = Field(default_factory=time.perf_counter)

    # Step results (immutable tuples/dicts)
    dog_judgments: tuple[DogJudgment, ...] = Field(default_factory=tuple)
    consensus: Any | None = None
    final_judgment: Judgment | None = None
    action_executed: bool = False
    action_result: Optional[dict[str, Any]] = None
    learning_applied: bool = False

    # Costs & Latency
    total_cost_usd: float = 0.0
    total_latency_ms: float = 0.0

    def elapsed_ms(self) -> float:
        return (time.perf_counter() - self.started_at) * 1000

    def evolve(self, **kwargs: Any) -> JudgmentPipeline:
        """Functional update: returns a NEW immutable instance with updates applied."""
        # Pydantic V2 model_copy with update is the standard way
        return self.model_copy(update=kwargs)
