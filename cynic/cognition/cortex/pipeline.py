"""
Judgment Pipeline — Context for one complete judgment cycle.

Extracted from orchestrator.py to reduce god object complexity.
"""
from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field
from typing import Any, Optional

from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.judgment import Cell, Judgment


@dataclass
class JudgmentPipeline:
    """
    Context for one complete judgment pipeline execution.

    Tracks timing, cost, and intermediate results for all 7 steps.
    """
    cell: Cell
    level: ConsciousnessLevel = ConsciousnessLevel.MACRO
    pipeline_id: str = field(default_factory=lambda: str(uuid.uuid4()))
    started_at: float = field(default_factory=time.perf_counter)

    # Step results
    dog_judgments: list = field(default_factory=list)
    consensus: Optional[Any] = None
    final_judgment: Optional[Judgment] = None
    action_executed: bool = False
    action_result: Optional[dict] = None
    learning_applied: bool = False  # Track C: whether learning signal was injected

    # Costs
    total_cost_usd: float = 0.0
    total_latency_ms: float = 0.0

    def elapsed_ms(self) -> float:
        return (time.perf_counter() - self.started_at) * 1000
