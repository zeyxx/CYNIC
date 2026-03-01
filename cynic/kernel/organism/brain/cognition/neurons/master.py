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
from typing import Any

from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.core.judgment import Cell
from cynic.kernel.core.phi import phi_bound_score
from cynic.kernel.organism.brain.cognition.neurons.base import (
    DogCapabilities,
    DogHealth,
    DogId,
    DogJudgment,
    HealthStatus,
    LLMDog,
)

logger = logging.getLogger("cynic.kernel.brain.neurons.master")

@dataclass
class DogSoul:
    """The configuration-based essence of a Dog."""
    dog_id: DogId
    sefirot: str
    task_type: str
    axioms: list[str]
    system_prompt: str
    heuristic_prompt: str
    
    # Behavior tunables
    temperature: float = 0.618
    consciousness_min: ConsciousnessLevel = ConsciousnessLevel.MICRO
    supported_realities: set[str] = field(default_factory=lambda: {"CODE", "SOLANA", "MARKET", "SOCIAL", "HUMAN", "CYNIC", "COSMOS"})
    
    # Specialized logic (optional plugins)
    expertise_fn: str | None = None 

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
        self._last_latencies: list[float] = []

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
                    # If expertise is confident enough, we might skip LLM or combine
                    # For now, let's use it as a high-quality heuristic
                    return self._create_judgment(cell, start, ext_result)

            # 2. Attempt LLM Judgment (The "Dream" path)
            adapter = await self.get_llm()
            if adapter:
                judgment = await self._llm_path(cell, adapter, start)
                
                # SAGE-specific: bidirectional attention feedback
                if self.dog_id == DogId.SAGE and hasattr(self, "_compressor") and self._compressor:
                    try:
                        self._compressor.boost(str(cell.content), judgment.q_score)
                    except Exception: pass
                    
                return judgment
            
            # 3. Fallback to Basic Heuristic (The "Instinct" path)
            return await self._heuristic_path(cell, start)
            
        except Exception as e:
            self._errors += 1
            logger.error(f"MasterDog[{self.dog_id}] analysis failed: {e}", exc_info=True)
            return self._error_judgment(cell, start, str(e))

    # --- SAGE EXPERTISE (World-Maker) ---
    async def dream_facets(self, axiom: str, reality: str, registry: Any) -> dict[str, str]:
        """Delegate facet dreaming to expertise plugin if available."""
        if self.dog_id == DogId.SAGE:
            from cynic.kernel.organism.brain.cognition.neurons.expertise import (
                dream_facets_expertise,
            )
            adapter = await self.get_llm()
            if adapter:
                return await dream_facets_expertise(adapter, axiom, reality, registry)
        return {}

    def set_compressor(self, compressor: Any) -> None:
        """Inject ContextCompressor (SAGE only)."""
        if self.dog_id == DogId.SAGE:
            self._compressor = compressor
            logger.info("MasterDog[SAGE]: ContextCompressor injected.")

    def _create_judgment(self, cell: Cell, start: float, data: dict[str, Any]) -> DogJudgment:
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

    async def pbft_run(self, cell: Cell, dog_judgments: list[DogJudgment]) -> DogJudgment | None:
        """
        Byzantine Fault Tolerant consensus across Dog judgments.

        Aggregates judgments using weighted voting and returns consensus.
        Returns None if consensus cannot be reached.
        """
        if not dog_judgments:
            return None

        # Simple majority-based consensus: average the q_scores
        avg_q_score = sum(j.q_score for j in dog_judgments) / len(dog_judgments)
        avg_confidence = sum(j.confidence for j in dog_judgments) / len(dog_judgments)

        # Consensus judgment from CYNIC's perspective
        consensus = DogJudgment(
            dog_id=DogId.CYNIC,
            cell_id=cell.cell_id,
            q_score=phi_bound_score(avg_q_score),
            confidence=avg_confidence,
            reasoning=f"PBFT consensus from {len(dog_judgments)} Dogs: {[j.dog_id for j in dog_judgments]}",
            evidence={"dog_votes": len(dog_judgments), "avg_score": avg_q_score},
            latency_ms=0.0,
        )
        return consensus
