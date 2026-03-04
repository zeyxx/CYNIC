"""
CYNIC Experience Vault - The Synaptic Memory of the Organism.
Uses Bayesian updating and UCB1 for exploration/exploitation.
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
from cynic.kernel.core.mathematics import ProbabilisticMathematics, RLMathematics

logger = logging.getLogger("cynic.organism.experience")

@dataclass
class DogPerformance:
    success_count: int = 0
    failure_count: int = 0
    total_latency_ms: float = 0.0
    # Bayesian Prior: we assume a 50% success rate with a weight of 2 trials before we know anything
    prior_prob: float = 0.5
    prior_weight: int = 2
    last_seen: str = field(default_factory=lambda: datetime.now().isoformat())

    @property
    def bayesian_score(self) -> float:
        """The true probability of success based on Bayesian conjugate updating."""
        total = self.success_count + self.failure_count
        return ProbabilisticMathematics.bayesian_update(
            self.prior_prob, self.prior_weight, self.success_count, total
        )

class ExperienceVault:
    def __init__(self, storage_path: Optional[str] = None):
        self.path = Path(storage_path or "audit/synaptic_weights.json")
        self.weights: Dict[str, Dict[str, DogPerformance]] = defaultdict(
            lambda: defaultdict(DogPerformance)
        )
        self._lock = asyncio.Lock()
        self.total_system_trials = 0
        self.load()

    def load(self) -> None:
        if self.path.exists():
            try:
                with open(self.path, "r") as f:
                    data = json.load(f)
                    self.total_system_trials = data.get("_meta_total_trials", 0)
                    for llm_id, axioms in data.items():
                        if llm_id.startswith("_meta"): continue
                        for axiom, stats in axioms.items():
                            self.weights[llm_id][axiom] = DogPerformance(**stats)
            except Exception as e:
                logger.error(f"Failed to load ExperienceVault: {e}")

    async def persist(self) -> None:
        async with self._lock:
            try:
                self.path.parent.mkdir(parents=True, exist_ok=True)
                serializable = {
                    llm_id: {axiom: asdict(perf) for axiom, perf in axioms.items()}
                    for llm_id, axioms in self.weights.items()
                }
                serializable["_meta_total_trials"] = self.total_system_trials
                with open(self.path, "w") as f:
                    json.dump(serializable, f, indent=2)
            except Exception as e:
                logger.error(f"Failed to persist ExperienceVault: {e}")

    async def record_experience(self, llm_id: str, axiom: str, success: bool, latency_ms: float) -> None:
        async with self._lock:
            self.total_system_trials += 1
            perf = self.weights[llm_id][axiom]
            if success: perf.success_count += 1
            else: perf.failure_count += 1
            perf.total_latency_ms += latency_ms
            perf.last_seen = datetime.now().isoformat()
        await self.persist()

    def get_best_dog_for(self, axiom: str, candidates: List[str]) -> str:
        """Selects the best Dog using UCB1 (Upper Confidence Bound)."""
        if not candidates: return "default"

        scored_candidates = []
        for c in candidates:
            perf = self.weights[c][axiom]
            model_trials = perf.success_count + perf.failure_count
            
            # UCB1 score balances the Bayesian average reward with the exploration bonus
            ucb_score = RLMathematics.ucb1_score(
                average_reward=perf.bayesian_score,
                total_system_trials=max(1, self.total_system_trials),
                model_trials=model_trials,
                exploration_constant=1.414 # sqrt(2) is standard for UCB1
            )
            scored_candidates.append((ucb_score, c))

        # Sort by UCB score descending
        scored_candidates.sort(key=lambda x: x[0], reverse=True)
        best_dog = scored_candidates[0][1]
        
        logger.debug(f"Vault: Selected {best_dog} via UCB1 (Score: {scored_candidates[0][0]:.4f})")
        return best_dog

_vault: Optional[ExperienceVault] = None
def get_vault() -> ExperienceVault:
    global _vault
    if _vault is None: _vault = ExperienceVault()
    return _vault
