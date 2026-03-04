"""
CYNIC Experience Vault - The Synaptic Memory of the Organism.
Stores and evaluates the axiomatic performance of different LLM Dogs.
Thread-safe and strictly typed for industrial use.
"""

from __future__ import annotations

import asyncio
import json
import logging
from collections import defaultdict
from dataclasses import asdict, dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional

logger = logging.getLogger("cynic.organism.experience")


@dataclass
class DogPerformance:
    success_count: int = 0
    failure_count: int = 0
    total_latency_ms: float = 0.0
    last_seen: str = field(default_factory=lambda: datetime.now().isoformat())

    @property
    def score(self) -> float:
        """Calculates a Phi-bounded score based on success rate and latency."""
        total = self.success_count + self.failure_count
        if total == 0:
            return 0.5  # Neutral starting point
        rate = self.success_count / total
        # Penalty for high latency (heuristic: 1000ms is standard)
        # We normalize by assuming 5000ms is a 'very slow' task
        latency_factor = max(0.1, 1.0 - (self.total_latency_ms / (total * 5000)))
        return float(rate * latency_factor)


class ExperienceVault:
    """
    Persistent store for LLM performance across the 5 Axioms.
    Uses async locks to prevent memory corruption during parallel campaigns.
    """

    def __init__(self, storage_path: Optional[str] = None):
        self.path = Path(storage_path or "audit/synaptic_weights.json")
        self.weights: Dict[str, Dict[str, DogPerformance]] = defaultdict(
            lambda: defaultdict(DogPerformance)
        )
        self._lock = asyncio.Lock()
        self.load()

    def load(self) -> None:
        """Loads synaptic weights from disk."""
        if self.path.exists():
            try:
                with open(self.path, "r") as f:
                    data = json.load(f)
                    for llm_id, axioms in data.items():
                        for axiom, stats in axioms.items():
                            self.weights[llm_id][axiom] = DogPerformance(**stats)
                logger.info(f"ExperienceVault: Loaded memory from {self.path}")
            except Exception as e:
                logger.error(f"Failed to load ExperienceVault: {e}")

    async def persist(self) -> None:
        """Saves current weights to disk with async safety."""
        async with self._lock:
            try:
                self.path.parent.mkdir(parents=True, exist_ok=True)
                serializable = {
                    llm_id: {axiom: asdict(perf) for axiom, perf in axioms.items()}
                    for llm_id, axioms in self.weights.items()
                }
                with open(self.path, "w") as f:
                    json.dump(serializable, f, indent=2)
            except Exception as e:
                logger.error(f"Failed to persist ExperienceVault: {e}")

    async def record_experience(
        self, llm_id: str, axiom: str, success: bool, latency_ms: float
    ) -> None:
        """Updates the synaptic weight for a specific Dog and Axiom."""
        async with self._lock:
            perf = self.weights[llm_id][axiom]
            if success:
                perf.success_count += 1
            else:
                perf.failure_count += 1
            perf.total_latency_ms += latency_ms
            perf.last_seen = datetime.now().isoformat()

        # Persist after update
        await self.persist()

    def get_best_dog_for(self, axiom: str, candidates: List[str]) -> str:
        """Selects the most reliable Dog for a given axiomatic task."""
        if not candidates:
            return "default"

        # Sort candidates by their calculated score for the given axiom
        # If unknown, score defaults to 0.5 via property
        scored = sorted(
            candidates, key=lambda c: self.weights[c][axiom].score, reverse=True
        )
        return str(scored[0])


# Global instance for the kernel
_vault: Optional[ExperienceVault] = None


def get_vault() -> ExperienceVault:
    global _vault
    if _vault is None:
        _vault = ExperienceVault()
    return _vault
