"""
Universal AI Infra Layer - Model Optimization Interface.
Respects AI Infra & ML Platform Lenses.

Defines the contract for pluggable optimizers (like Helion).
Allows the organism to improve its cognition without being tied to a specific engine.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any, Dict


@dataclass
class OptimizationResult:
    """Standardized result of a model optimization cycle."""

    optimizer_id: str
    baseline_latency_ms: float
    optimized_latency_ms: float
    speedup_factor: float
    weights_hash: str
    metadata: Dict[str, Any] = field(default_factory=dict)


class ModelOptimizer(ABC):
    """
    Abstract contract for cognitive optimization.
    Helion will implement this during the hackathon.
    """

    @property
    @abstractmethod
    def optimizer_id(self) -> str:
        """Unique identifier for this optimizer."""
        pass

    @abstractmethod
    async def optimize(self, q_table_snapshot: Dict[str, Any]) -> OptimizationResult:
        """
        Take raw judgments and transform them into optimized model weights.
        """
        pass

    @abstractmethod
    async def benchmark(self) -> Dict[str, float]:
        """
        Compare current performance against PyTorch/TensorFlow baselines.
        """
        pass
