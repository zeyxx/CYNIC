"""
CYNIC MasterDog — The Unified Sefirotic Intelligence.

A single class to rule all 11 Dogs. 
Reduces 5000+ LOC of duplication to one robust engine driven by a "DogSoul".

φ-Law: BURN — delete redundant code, keep the data essence.
"""
from __future__ import annotations

import logging
import time
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Set

from cynic.kernel.core.phi import PHI_INV, MAX_CONFIDENCE, phi_bound_score
from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.core.judgment import Cell
from cynic.kernel.organism.brain.cognition.neurons.base import (
    LLMDog, DogCapabilities, DogHealth, DogJudgment,
    DogId, HealthStatus,
)

logger = logging.getLogger("cynic.kernel.brain.neurons.master")

@dataclass
class DogSoul:
    """The configuration-based essence of a Dog."""
    dog_id: DogId
    sefirot: str
    task_type: str
    axioms: List[str]
    system_prompt: str
    heuristic_prompt: str
    
    # Behavior tunables
    temperature: float = 0.618
    consciousness_min: ConsciousnessLevel = ConsciousnessLevel.MICRO
    supported_realities: Set[str] = field(default_factory=lambda: {"CODE", "SOLANA", "MARKET", "SOCIAL", "HUMAN", "CYNIC", "COSMOS"})
    
    # Specialized logic (optional plugins)
    expertise_fn: Optional[str] = None 

class MasterDog(LLMDog):
    """
    Unified engine for all Dogs.
    Uses a DogSoul to define its personality and judgment criteria.
    """

    def __init__(self, soul: DogSoul) -> None:
        super().__init__(soul.dog_id, task_type=soul.task_type)
        self.soul = soul
        self._lookups = 0
        self._errors = 0
        self._last_latencies: List[float] = []

    def get_capabilities(self) -> DogCapabilities:
        return DogCapabilities(
            dog_id=self.soul.dog_id,
            sefirot=self.soul.sefirot,
            consciousness_min=self.soul.consciousness_min,
            uses_llm=True,
            supported_realities=self.soul.supported_realities,
            supported_analyses={"PERCEIVE", "JUDGE", "DECIDE", "ACT", "LEARN", "ACCOUNT", "EMERGE"},
            technology=f"MasterDog Engine (Soul: {self.soul.task_type})",
            max_concurrent=4,
        )

    async def analyze(self, cell: Cell, **kwargs: Any) -> DogJudgment:
        """The core analysis loop: Heuristic -> LLM -> Validation."""
        start = time.perf_counter()
        self._lookups += 1
        
        try:
            # 1. Attempt LLM Judgment (The "Dream" path)
            adapter = await self.get_llm()
            if adapter:
                return await self._llm_path(cell, adapter, start)
            
            # 2. Fallback to Heuristic (The "Instinct" path)
            return await self._heuristic_path(cell, start)
            
        except Exception as e:
            self._errors += 1
            logger.error(f"MasterDog[{self.dog_id}] analysis failed: {e}", exc_info=True)
            return self._error_judgment(cell, start, str(e))

    async def _llm_path(self, cell: Cell, adapter: Any, start: float) -> DogJudgment:
        """Unified LLM judgment path."""
        from cynic.kernel.organism.brain.llm.temporal import temporal_judgment
        
        # Inject soul into the content
        content = f"{self.soul.system_prompt}\n\nCONTEXT:\n{cell.content}"
        tj = await temporal_judgment(adapter, content)
        
        latency = (time.perf_counter() - start) * 1000
        self._last_latencies.append(latency)
        
        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=tj.phi_aggregate,
            confidence=tj.confidence,
            reasoning=f"*sniff* {self.dog_id} saw through 7 perspectives: {tj.reasoning[:200]}...",
            evidence=tj.to_dict(),
            latency_ms=latency,
            llm_id=tj.llm_id,
        )
        self.record_judgment(judgment)
        return judgment

    async def _heuristic_path(self, cell: Cell, start: float) -> DogJudgment:
        """Fast, LLM-less fallback using soul axioms."""
        # TODO: Implement basic keyword/pattern matching based on soul.heuristic_prompt
        latency = (time.perf_counter() - start) * 1000
        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=50.0, # Neutral
            confidence=0.2, # Low trust
            reasoning=f"*head tilt* {self.dog_id} heuristic: No LLM available, using baseline instincts.",
            evidence={"axioms": self.soul.axioms},
            latency_ms=latency,
        )
        self.record_judgment(judgment)
        return judgment

    def _error_judgment(self, cell: Cell, start: float, error: str) -> DogJudgment:
        return DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=0.0,
            confidence=0.0,
            reasoning=f"*whimper* {self.dog_id} failed: {error}",
            evidence={"error": error},
            latency_ms=(time.perf_counter() - start) * 1000,
        )

    async def health_check(self) -> DogHealth:
        status = HealthStatus.HEALTHY if self._errors / max(self._lookups, 1) < 0.1 else HealthStatus.DEGRADED
        return DogHealth(
            dog_id=self.dog_id,
            status=status,
            latency_p50_ms=sum(self._last_latencies) / max(len(self._last_latencies), 1),
            details=f"Soul: {self.soul.sefirot} | Errors: {self._errors}/{self._lookups}"
        )
