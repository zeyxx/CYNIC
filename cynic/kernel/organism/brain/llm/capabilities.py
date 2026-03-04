"""
CYNIC Dog Capabilities Manager - Multi-Faceted Muscle Discovery.
Respects AI Infra, ML Platform & Solutions Architect Lenses.

Tracks the historical performance of every LLM muscle (Dog) against 
CYNIC's 9 Axioms and their Facets. Uses EMA (Exponential Moving Average) 
to update capability scores based on real-world success/failure.
"""
from __future__ import annotations

import logging
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional

from cynic.kernel.core.axioms import AxiomType # Assuming this exists
from cynic.kernel.core.phi import PHI_INV

logger = logging.getLogger("cynic.brain.llm.capabilities")

@dataclass
class DogCapability:
    llm_id: str
    # Scores per Axiom [0, 1]
    axiom_scores: Dict[str, float] = field(default_factory=dict)
    # EMA of latency in ms
    avg_latency_ms: float = 0.0
    # Context window utilization capacity
    max_context: int = 4096
    
    def get_score_for(self, axiom: str) -> float:
        return self.axiom_scores.get(axiom, 0.5) # Neutral start

class DogCapabilitiesManager:
    """
    Sovereign registry of AI muscles. 
    Enables intelligent routing based on proven axiom-alignment.
    """
    def __init__(self):
        self._capabilities: Dict[str, DogCapability] = {}

    def update_performance(self, llm_id: str, axiom: str, success: bool, latency_ms: float):
        """EMA update of a Dog's capability score."""
        cap = self._capabilities.setdefault(llm_id, DogCapability(llm_id=llm_id))
        
        # Phi-weighted learning rate (fast to learn, slow to forget)
        lr = 1.0 - PHI_INV # ~0.382
        
        current_score = cap.axiom_scores.get(axiom, 0.5)
        target = 1.0 if success else 0.0
        new_score = current_score + lr * (target - current_score)
        
        cap.axiom_scores[axiom] = new_score
        
        # Latency EMA
        cap.avg_latency_ms = cap.avg_latency_ms + lr * (latency_ms - cap.avg_latency_ms)
        
        logger.debug(f"CapUpdate: {llm_id} Axiom:{axiom} -> {new_score:.3f}")

    def select_best_dog(self, required_axioms: List[str], max_latency: float = 30000) -> str:
        """Find the muscle best suited for a specific multi-faceted task."""
        if not self._capabilities:
            return "default"
            
        def rank_dog(cap: DogCapability) -> float:
            # Geometric mean of required axiom scores
            if not required_axioms: return 0.5
            product = 1.0
            for ax in required_axioms:
                product *= cap.get_score_for(ax)
            return product ** (1.0 / len(required_axioms))

        # Filter by latency and then rank
        candidates = [c for c in self._capabilities.values() if c.avg_latency_ms <= max_latency]
        if not candidates: candidates = list(self._capabilities.values())
        
        best = max(candidates, key=rank_dog)
        return best.llm_id
