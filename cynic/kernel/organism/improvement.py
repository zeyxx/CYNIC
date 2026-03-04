"""
CYNIC Improvement Cortex - Mathematical Self-Optimization.
Analyzes benchmarks to refine the organism's synaptic weights and model selection.
"""
from __future__ import annotations

import logging
from typing import Dict, List, Optional
from cynic.kernel.organism.benchmarking import get_benchmarker
from cynic.kernel.organism.experience import get_vault

logger = logging.getLogger("cynic.organism.improvement")

class ImprovementCortex:
    """
    The 'Prefrontal Cortex' of CYNIC.
    Translates raw performance numbers into refined Synaptic Scores.
    """
    def __init__(self):
        self.benchmarker = get_benchmarker()
        self.vault = get_vault()

    async def consolidate_learning(self):
        """
        Analyzes recent benchmarks and updates the ExperienceVault.
        This is the mathematical feedback loop.
        """
        logger.info("ImprovementCortex: Starting synaptic consolidation...")
        
        # In a real scenario, we would iterate over all models/axioms
        # Here we refine the scores based on Bayesian Updates and Multi-Dimensional Phi
        for llm_id, axioms in self.vault.weights.items():
            for axiom, perf in axioms.items():
                avg_stats = self.benchmarker.get_average_performance(llm_id, axiom)
                
                # Update the Bayesian Prior based on empirical risk (Mohri)
                # If the L2 Phi quality score is consistently high, we strengthen the prior
                empirical_phi = avg_stats["avg_phi"]
                
                if empirical_phi > 0.0:
                    # Move the prior probability towards the true empirical Phi
                    learning_rate = 0.1
                    perf.prior_prob = (perf.prior_prob * (1 - learning_rate)) + (empirical_phi * learning_rate)
                    
                    logger.info(f"ImprovementCortex: Bayesan shift for {llm_id}:{axiom} -> Prior now {perf.prior_prob:.4f} (Empirical Phi: {empirical_phi:.4f})")
        
        await self.vault.persist()
        logger.info("ImprovementCortex: Consolidation complete.")

# Global instance
_cortex: Optional[ImprovementCortex] = None

def get_improvement_cortex() -> ImprovementCortex:
    global _cortex
    if _cortex is None:
        _cortex = ImprovementCortex()
    return _cortex
