"""
CYNIC MasterDog — The Unified Sefirotic Intelligence.

A single class to rule all 11 Dogs. 
Reduces 5000+ LOC of duplication to one robust engine driven by a "DogSoul".

φ-Law: BURN — delete redundant code, keep the data essence.
"""
from __future__ import annotations

import asyncio
import logging
import time
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Set

from cynic.kernel.core.phi import (
    PHI_INV, MAX_CONFIDENCE, MAX_Q_SCORE, phi_bound_score,
    DOGS_QUORUM, DOG_PRIORITY, weighted_geometric_mean
)
from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.core.judgment import Cell, ConsensusResult
from cynic.kernel.core.event_bus import get_core_bus, Event, CoreEvent
from cynic.kernel.organism.brain.cognition.neurons.base import (
    LLMDog, DogCapabilities, DogHealth, DogJudgment,
    DogId, HealthStatus,
)

logger = logging.getLogger("cynic.kernel.brain.neurons.master")

def _compute_quorum(n: int) -> int:
    """PBFT formula: f = (n-1)//3, quorum = 2f+1."""
    if n < 4: return n
    f = (n - 1) // 3
    return 2 * f + 1

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
    Includes PBFT coordination logic for the CYNIC DogId.
    """

    def __init__(self, soul: DogSoul) -> None:
        super().__init__(soul.dog_id, task_type=soul.task_type)
        self.soul = soul
        self._lookups = 0
        self._errors = 0
        self._last_latencies: List[float] = []
        self._compressor = None

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
        """The core analysis loop: Expertise -> LLM -> Heuristic Fallback."""
        start = time.perf_counter()
        self._lookups += 1
        
        try:
            # 1. Attempt Expertise Plugin (Heuristic Specialist)
            if self.soul.expertise_fn:
                from cynic.kernel.organism.brain.cognition.neurons.expertise import call_expertise
                ext_result = await call_expertise(self.soul.expertise_fn, cell)
                if ext_result and ext_result.get("confidence", 0) > 0.3:
                    return self._create_judgment(cell, start, ext_result)

            # 2. Attempt LLM Judgment
            adapter = await self.get_llm()
            if adapter:
                judgment = await self._llm_path(cell, adapter, start)
                if self.dog_id == DogId.SAGE and self._compressor:
                    try:
                        self._compressor.boost(str(cell.content), judgment.q_score)
                    except Exception: pass
                return judgment
            
            # 3. Fallback to Heuristic
            return await self._heuristic_path(cell, start)
            
        except Exception as e:
            self._errors += 1
            logger.error(f"MasterDog[{self.dog_id}] failed: {e}")
            return self._error_judgment(cell, start, str(e))

    async def _llm_path(self, cell: Cell, adapter: Any, start: float) -> DogJudgment:
        from cynic.kernel.organism.brain.llm.temporal import temporal_judgment
        content = f"{self.soul.system_prompt}\n\nCONTEXT:\n{cell.content}"
        tj = await temporal_judgment(adapter, content)
        latency = (time.perf_counter() - start) * 1000
        self._last_latencies.append(latency)
        
        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=tj.phi_aggregate,
            confidence=tj.confidence,
            reasoning=f"*sniff* {self.dog_id} saw: {tj.reasoning[:200]}...",
            evidence=tj.to_dict(),
            latency_ms=latency,
            llm_id=tj.llm_id,
        )
        self.record_judgment(judgment)
        return judgment

    async def _heuristic_path(self, cell: Cell, start: float) -> DogJudgment:
        latency = (time.perf_counter() - start) * 1000
        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=50.0,
            confidence=0.2,
            reasoning=f"*head tilt* {self.dog_id} using baseline instincts.",
            evidence={"axioms": self.soul.axioms},
            latency_ms=latency,
        )
        self.record_judgment(judgment)
        return judgment

    # --- SAGE EXPERTISE (World-Maker) ---
    async def dream_facets(self, axiom: str, reality: str, registry: Any) -> dict[str, str]:
        if self.dog_id == DogId.SAGE:
            from cynic.kernel.organism.brain.cognition.neurons.expertise import dream_facets_expertise
            adapter = await self.get_llm()
            if adapter:
                return await dream_facets_expertise(adapter, axiom, reality, registry)
        return {}

    def set_compressor(self, compressor: Any) -> None:
        if self.dog_id == DogId.SAGE:
            self._compressor = compressor

    # --- PBFT CONSENSUS (CYNIC Role) ---
    async def pbft_run(self, cell: Cell, dog_judgments: List[DogJudgment]) -> ConsensusResult:
        """Coordinateur PBFT: agrège les votes des Dogs."""
        quorum = _compute_quorum(len(dog_judgments))
        votes = len(dog_judgments)
        
        if votes < quorum:
            return ConsensusResult(
                consensus=False, votes=votes, quorum=quorum,
                reason=f"Insufficient votes: {votes}/{quorum}",
                dog_judgments=[j.to_dict() for j in dog_judgments]
            )

        # Simple weighted aggregate
        scores = [j.q_score for j in dog_judgments]
        weights = [DOG_PRIORITY.get(j.dog_id, 1.0) for j in dog_judgments]
        
        final_q = weighted_geometric_mean(scores, weights)
        from cynic.kernel.core.axioms import verdict_from_q_score
        final_verdict = verdict_from_q_score(final_q).value

        return ConsensusResult(
            consensus=True,
            votes=votes,
            quorum=quorum,
            final_q_score=phi_bound_score(final_q),
            final_verdict=final_verdict,
            final_confidence=sum(j.confidence for j in dog_judgments)/max(votes, 1),
            dog_judgments=[j.to_dict() for j in dog_judgments]
        )

    def _create_judgment(self, cell: Cell, start: float, data: Dict[str, Any]) -> DogJudgment:
        latency = (time.perf_counter() - start) * 1000
        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=phi_bound_score(data.get("q_score", 50.0)),
            confidence=data.get("confidence", 0.2),
            reasoning=f"*sniff* {self.dog_id} expertise: {data.get('reasoning')}",
            evidence=data.get("evidence", {}),
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
            latency_p50_ms=sum(self._last_latencies) / max(len(self._last_latencies), 1) if self._last_latencies else 0.0,
            details=f"Soul: {self.soul.sefirot} | Errors: {self._errors}/{self._lookups}"
        )
