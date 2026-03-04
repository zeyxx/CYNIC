"""
CYNIC Metabolic Orchestrator - The unified gateway for agentic actions.
Links Memory (ExperienceVault) with Physical Resources (MetabolicGovernor).
"""
from __future__ import annotations

import time
import logging
from typing import List, Any, Optional, Callable, Awaitable

from cynic.kernel.organism.experience import get_vault
from cynic.kernel.organism.metabolism.governor import MetabolicGovernor

logger = logging.getLogger("cynic.organism.orchestrator")

class MetabolicOrchestrator:
    def __init__(self, governor: Optional[MetabolicGovernor] = None):
        self.vault = get_vault()
        self.governor = governor or MetabolicGovernor()

    async def execute_with_learning(
        self, 
        axiom: str, 
        candidates: List[str],
        action_func: Callable[[str], Awaitable[Any]]
    ) -> Any:
        """
        Executes a task by selecting the best model, preparing resources, 
        and recording the outcome.
        """
        # 1. Selection based on experience
        best_dog = self.vault.get_best_dog_for(axiom, candidates)
        logger.info(f"Orchestrator: Selected {best_dog} for {axiom} task.")

        # 2. Somatic Preparation
        await self.governor.allocate(best_dog)

        # 3. Execution & Timing
        start_time = time.time()
        success = False
        result = None
        
        try:
            result = await action_func(best_dog)
            # Logic to determine success depends on the result type
            # For now, if no exception, we assume functional success 
            # (can be refined by the caller)
            success = True 
        except Exception as e:
            logger.error(f"Execution failed with {best_dog}: {e}")
            result = e
        finally:
            latency_ms = (time.time() - start_time) * 1000
            
            # 4. Synaptic Update (Learning)
            self.vault.record_experience(
                llm_id=best_dog, 
                axiom=axiom, 
                success=success, 
                latency_ms=latency_ms
            )
            logger.info(f"Orchestrator: Recorded experience for {best_dog}. Success={success}, Latency={latency_ms:.2f}ms")

        return result

# Global instance
_orchestrator: Optional[MetabolicOrchestrator] = None

def get_orchestrator() -> MetabolicOrchestrator:
    global _orchestrator
    if _orchestrator is None:
        _orchestrator = MetabolicOrchestrator()
    return _orchestrator
