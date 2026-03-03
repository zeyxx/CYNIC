"""
CYNIC MasterDog " The Unified Sefirotic Intelligence.

A single class to rule all 11 Dogs.
Reduces 5000+ LOC of duplication to one robust engine driven by a "DogSoul".

-Law: BURN " delete redundant code, keep the data essence.
"""

from __future__ import annotations

import logging
import time
from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Any, Optional

if TYPE_CHECKING:
    from cynic.kernel.core.event_bus import EventBus

from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.core.judgment import Cell, ConsensusResult
from cynic.kernel.core.phi import (
    DOG_PRIORITY,
    phi_bound_score,
    weighted_geometric_mean,
)
from cynic.kernel.organism.brain.cognition.neurons.base import (
    DogCapabilities,
    DogHealth,
    DogId,
    DogJudgment,
    HealthStatus,
    LLMDog,
)

logger = logging.getLogger("cynic.kernel.brain.neurons.master")


def _compute_quorum(n: int) -> int:
    """PBFT formula: f = (n-1)//3, quorum = 2f+1."""
    if n < 4:
        return n
    f = (n - 1) // 3
    return 2 * f + 1


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
    supported_realities: set[str] = field(
        default_factory=lambda: {
            "CODE",
            "SOLANA",
            "MARKET",
            "SOCIAL",
            "HUMAN",
            "CYNIC",
            "COSMOS",
        }
    )

    # Specialized logic (optional plugins)
    expertise_fn: str | None = None


class MasterDog(LLMDog):
    """
    Unified engine for all Dogs.
    Uses a DogSoul to define its personality and judgment criteria.
    Includes PBFT coordination logic for the CYNIC DogId.
    """

    def __init__(
        self,
        soul: DogSoul,
        bus: Optional["EventBus"] = None,
        vascular: Optional["VascularSystem"] = None,
    ) -> None:
        """Initialize MasterDog with a DogSoul configuration.

        Args:
            soul: DogSoul configuration defining personality, axioms, and prompts
            bus: Event bus for emitting observations. REQUIRED.
            vascular: Vascular system for multimodal IO and acceleration. OPTIONAL.
        """
        super().__init__(
            soul.dog_id, task_type=soul.task_type, bus=bus, vascular=vascular
        )
        self.soul = soul
        self._lookups = 0
        self._errors = 0
        self._last_latencies: list[float] = []
        self._compressor = None

    def get_capabilities(self) -> DogCapabilities:
        return DogCapabilities(
            dog_id=self.soul.dog_id,
            sefirot=self.soul.sefirot,
            consciousness_min=self.soul.consciousness_min,
            uses_llm=True,
            supported_realities=self.soul.supported_realities,
            supported_analyses={
                "PERCEIVE",
                "JUDGE",
                "DECIDE",
                "ACT",
                "LEARN",
                "ACCOUNT",
                "EMERGE",
            },
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
                from cynic.kernel.organism.brain.cognition.neurons.expertise import (
                    call_expertise,
                )

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
                    except Exception as _e:
                        logger.debug(f"Silenced: {_e}")
                return judgment

            # 3. Fallback to Heuristic
            return await self._heuristic_path(cell, start)

        except Exception as e:
            self._errors += 1
            logger.error(f"MasterDog[{self.dog_id}] failed: {e}")
            return self._error_judgment(cell, start, str(e))

    async def _llm_path(self, cell: Cell, adapter: Any, start: float) -> DogJudgment:
        from cynic.kernel.organism.brain.llm.temporal import temporal_judgment

        # 1. Handle Multimodal Data if present
        multimodal_data = []
        if cell.multimodal_packet_id and self.vascular:
            # We look for the packet in the perception buffer
            # In a real scenario, we might want a more persistent lookup
            packets = (
                await self.vascular.perception.flush()
            )  # Simplified: get all for now
            for p in packets:
                if p.packet_id == cell.multimodal_packet_id:
                    multimodal_data.append(p)
                    # Put back others
                else:
                    await self.vascular.perception.push(p)

        # 2. Execute Temporal Analysis
        content = f"{self.soul.system_prompt}\n\nCONTEXT:\n{cell.content}"
        tj = await temporal_judgment(adapter, content, multimodal_data=multimodal_data)

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
    async def dream_facets(
        self, axiom: str, reality: str, registry: Any
    ) -> dict[str, str]:
        if self.dog_id == DogId.SAGE:
            from cynic.kernel.organism.brain.cognition.neurons.expertise import (
                dream_facets_expertise,
            )

            adapter = await self.get_llm()
            if adapter:
                return await dream_facets_expertise(adapter, axiom, reality, registry)
        return {}

    def set_compressor(self, compressor: Any) -> None:
        if self.dog_id == DogId.SAGE:
            self._compressor = compressor

    # --- phi-BFT CONSENSUS (CYNIC Coordinator Role) ---
    async def phi_bft_run(
        self, cell: Cell, dog_judgments: list[DogJudgment]
    ) -> ConsensusResult:
        """
        phi-BFT: PHI-weighted Byzantine Fault Tolerance.

        Logic:
        1. Quorum Check: Requires 2f+1 votes (7/11).
        2. Priority Weighting: Each dog's vote is multiplied by its Sefirotic priority (phi^n).
        3. Harmonic Aggregation: Balanced mean that resists extreme outliers unless high priority.
        4. Veto: If high-priority dog (GUARDIAN) says BARK, final q_score is capped.
        """
        from cynic.kernel.core.axioms import verdict_from_q_score

        votes = len(dog_judgments)
        # f = (n-1)//3 = 3. Quorum = 2f+1 = 7.
        quorum = _compute_quorum(votes) if votes < 11 else 7

        if votes < quorum:
            return ConsensusResult(
                consensus=False,
                votes=votes,
                quorum=quorum,
                reason=f"Quorum failure: {votes}/{quorum} dogs responded.",
                dog_judgments=[j.to_dict() for j in dog_judgments],
            )

        # 1. Extraction des scores et poids
        scores = []
        weights = []
        veto_active = False

        for j in dog_judgments:
            priority = DOG_PRIORITY.get(j.dog_id, 1.0)
            scores.append(max(j.q_score, 0.1))
            weights.append(priority)

            # 2. Veto Logic (Guardian/Cynic high confidence BARK)
            if (
                j.dog_id in ["GUARDIAN", "CYNIC"]
                and j.q_score < 38.2
                and j.confidence > 0.5
            ):
                veto_active = True

        # 3. Weighted Aggregation (phi-weighted geometric mean)
        final_q = weighted_geometric_mean(scores, weights)

        # 4. Apply Veto
        if veto_active:
            final_q = min(final_q, 38.1)  # Force into BARK/GROWL territory
            logger.warning("phi-BFT: Veto triggered by high-priority dog.")

        final_verdict = verdict_from_q_score(final_q).value

        return ConsensusResult(
            consensus=True,
            votes=votes,
            quorum=quorum,
            final_q_score=phi_bound_score(final_q),
            final_verdict=final_verdict,
            final_confidence=sum(j.confidence for j in dog_judgments) / max(votes, 1),
            dog_judgments=[j.to_dict() for j in dog_judgments],
        )

    def _create_judgment(
        self, cell: Cell, start: float, data: dict[str, Any]
    ) -> DogJudgment:
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
        status = (
            HealthStatus.HEALTHY
            if self._errors / max(self._lookups, 1) < 0.1
            else HealthStatus.DEGRADED
        )
        return DogHealth(
            dog_id=self.dog_id,
            status=status,
            latency_p50_ms=sum(self._last_latencies) / max(len(self._last_latencies), 1)
            if self._last_latencies
            else 0.0,
            details=f"Soul: {self.soul.sefirot} | Errors: {self._errors}/{self._lookups}",
        )
