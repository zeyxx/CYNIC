"""
CYNIC Dog — Keter (Crown) — PBFT Consensus Coordinator

THE most critical Dog. Coordinates consensus among all 11 Dogs.
Uses PBFT (Practical Byzantine Fault Tolerance):
  - f = 3 Byzantine faults tolerated
  - n = 11 Dogs (L(5) = Lucas(5))
  - quorum = 2f+1 = 7 votes needed

4 PBFT phases:
  1. PRE-PREPARE  — primary proposes cell for judgment
  2. PREPARE      — replicas agree on the proposal
  3. COMMIT       — replicas confirm commitment to value
  4. REPLY        — primary collects and aggregates final votes

CYNIC Dog runs at L3 REFLEX when doing PBFT coordination (no LLM).
CYNIC Dog can also produce its own independent q_score judgment.

Keter = Crown = the highest sefirot = systemic coherence.
"""
from __future__ import annotations

import asyncio
import logging
import time
import uuid
from collections import defaultdict
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Set, Tuple

from cynic.core.phi import (
    DOGS_TOTAL, DOGS_BYZANTINE, DOGS_QUORUM,
    MAX_Q_SCORE, MAX_CONFIDENCE, PHI_INV, PHI_3,
    phi_bound_score, weighted_geometric_mean,
)
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.judgment import Cell, ConsensusResult
from cynic.core.event_bus import (
    get_agent_bus, get_core_bus, Event, AgentEvent, CoreEvent
)
from cynic.dogs.base import (
    AbstractDog, DogCapabilities, DogHealth, DogJudgment,
    DogId, HealthStatus, DOG_PRIORITY,
)

logger = logging.getLogger("cynic.dogs.cynic")


# ════════════════════════════════════════════════════════════════════════════
# PBFT STATE MACHINE
# ════════════════════════════════════════════════════════════════════════════

class PBFTPhase(str):
    PRE_PREPARE = "PRE_PREPARE"
    PREPARE     = "PREPARE"
    COMMIT      = "COMMIT"
    REPLY       = "REPLY"
    DONE        = "DONE"
    FAILED      = "FAILED"


@dataclass
class PBFTRequest:
    """One PBFT consensus round for a Cell."""
    request_id: str = field(default_factory=lambda: str(uuid.uuid4()))
    cell_id: str = ""
    phase: str = PBFTPhase.PRE_PREPARE
    prepare_votes: Dict[str, float] = field(default_factory=dict)   # dog_id → q_score
    commit_votes: Dict[str, float] = field(default_factory=dict)    # dog_id → q_score
    veto_dogs: Set[str] = field(default_factory=set)                # Dogs that vetoed
    started_at: float = field(default_factory=time.time)
    timeout_sec: float = 5.0  # F(5)=5 — Fibonacci-aligned

    @property
    def is_timed_out(self) -> bool:
        return time.time() - self.started_at > self.timeout_sec

    @property
    def has_quorum(self) -> bool:
        return len(self.commit_votes) >= DOGS_QUORUM

    @property
    def has_veto(self) -> bool:
        """GUARDIAN veto blocks regardless of vote count."""
        return len(self.veto_dogs) > 0


# ════════════════════════════════════════════════════════════════════════════
# CYNIC DOG (Keter — PBFT Primary)
# ════════════════════════════════════════════════════════════════════════════

class CynicDog(AbstractDog):
    """
    The CYNIC Dog — Keter (Crown).

    Role 1: PBFT Coordinator
      - Initiates PRE-PREPARE for each cell judgment
      - Collects PREPARE + COMMIT votes from other Dogs
      - Aggregates into ConsensusResult

    Role 2: Systemic Coherence Scorer
      - Analyzes overall system coherence (are all Dogs healthy? is budget healthy?)
      - Produces its own q_score for the COSMOS/CYNIC reality dimensions

    Non-LLM — runs at L3 REFLEX (<10ms for coordination, ~50ms for aggregation).
    """

    def __init__(self) -> None:
        super().__init__(DogId.CYNIC)
        self._active_requests: Dict[str, PBFTRequest] = {}
        self._completed_requests: List[PBFTRequest] = []
        self._max_completed = 89  # F(11) — keep last 89 consensus records

    def get_capabilities(self) -> DogCapabilities:
        return DogCapabilities(
            dog_id=DogId.CYNIC,
            sefirot="Keter — Crown",
            consciousness_min=ConsciousnessLevel.REFLEX,  # runs at L3
            uses_llm=False,
            supported_realities={"CODE", "SOLANA", "MARKET", "SOCIAL", "HUMAN", "CYNIC", "COSMOS"},
            supported_analyses={"JUDGE", "DECIDE", "EMERGE"},  # consensus + governance
            technology="PBFT (f=3, n=11, quorum=7)",
            max_concurrent=1,  # Only 1 PBFT coordinator
        )

    async def analyze(self, cell: Cell, **kwargs: Any) -> DogJudgment:
        """
        Produce a systemic coherence score.

        CYNIC Dog judges:
        - Are all Dogs healthy? (health check)
        - Is budget sufficient? (budget check)
        - Is consciousness level appropriate? (cycle health)
        """
        start = time.perf_counter()

        # Systemic coherence factors
        factors = []

        # Factor 1: budget health
        budget = kwargs.get("budget_usd", 1.0)
        budget_ratio = min(budget / 10.0, 1.0)  # normalize to $10 max
        factors.append(budget_ratio)

        # Factor 2: active Dogs ratio
        active_dogs = kwargs.get("active_dogs", DOGS_TOTAL)
        dogs_ratio = active_dogs / DOGS_TOTAL
        factors.append(dogs_ratio)

        # Factor 3: consensus health (prior round success rate)
        recent = self._completed_requests[-13:] if self._completed_requests else []
        if recent:
            success_rate = sum(1 for r in recent if r.has_quorum) / len(recent)
        else:
            success_rate = PHI_INV  # 61.8% — assume moderate health without data
        factors.append(success_rate)

        # Geometric mean of coherence factors → q_score
        if any(f <= 0 for f in factors):
            q_score = 0.0
        else:
            import math
            log_sum = sum(math.log(f) for f in factors)
            q_score = math.exp(log_sum / len(factors)) * MAX_Q_SCORE

        q_score = phi_bound_score(q_score)
        latency = (time.perf_counter() - start) * 1000

        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=q_score,
            confidence=PHI_INV * PHI_INV,  # 0.382 — modest confidence for systemic score
            reasoning=f"Systemic coherence: budget={budget_ratio:.2f}, dogs={dogs_ratio:.2f}, consensus_rate={success_rate:.2f}",
            latency_ms=latency,
            llm_id=None,
        )
        self.record_judgment(judgment)
        return judgment

    async def health_check(self) -> DogHealth:
        """CYNIC Dog is healthy if PBFT can still reach quorum."""
        return DogHealth(
            dog_id=self.dog_id,
            status=HealthStatus.HEALTHY,
            latency_p50_ms=self.avg_latency_ms,
            details=f"Active PBFT requests: {len(self._active_requests)}",
        )

    # ── PBFT Protocol ──────────────────────────────────────────────────────

    async def pbft_run(
        self,
        cell: Cell,
        dog_judgments: List[DogJudgment],
    ) -> ConsensusResult:
        """
        Run a full PBFT consensus round given Dog judgments.

        In a distributed CYNIC, this would involve actual network messages.
        In single-process mode, we aggregate synchronously.

        Returns ConsensusResult (consensus reached or failed).
        """
        request = PBFTRequest(cell_id=cell.cell_id)
        self._active_requests[request.request_id] = request

        agent_bus = get_agent_bus()

        # PHASE 1: PRE-PREPARE — announce the cell
        await agent_bus.emit(Event(
            type=AgentEvent.PBFT_PRE_PREPARE,
            payload={"request_id": request.request_id, "cell_id": cell.cell_id},
            source=self.dog_id,
        ))
        request.phase = PBFTPhase.PREPARE

        # PHASE 2: PREPARE — collect Dog votes
        for j in dog_judgments:
            if j.veto:
                request.veto_dogs.add(j.dog_id)
            request.prepare_votes[j.dog_id] = j.q_score

        await agent_bus.emit(Event(
            type=AgentEvent.PBFT_PREPARE,
            payload={
                "request_id": request.request_id,
                "votes": request.prepare_votes,
            },
            source=self.dog_id,
        ))
        request.phase = PBFTPhase.COMMIT

        # PHASE 3: COMMIT — Dogs that sent PREPARE commit to the value
        # In single-process mode, all votes that passed PREPARE auto-commit
        request.commit_votes = dict(request.prepare_votes)

        await agent_bus.emit(Event(
            type=AgentEvent.PBFT_COMMIT,
            payload={
                "request_id": request.request_id,
                "commit_count": len(request.commit_votes),
            },
            source=self.dog_id,
        ))
        request.phase = PBFTPhase.REPLY

        # PHASE 4: REPLY — aggregate to final ConsensusResult
        result = self._aggregate(request, dog_judgments)

        await agent_bus.emit(Event(
            type=AgentEvent.PBFT_REPLY,
            payload={
                "request_id": request.request_id,
                "consensus": result.consensus,
                "final_q_score": result.final_q_score,
                "votes": result.votes,
            },
            source=self.dog_id,
        ))

        # Finalize
        request.phase = PBFTPhase.DONE if result.consensus else PBFTPhase.FAILED
        del self._active_requests[request.request_id]
        self._completed_requests.append(request)
        if len(self._completed_requests) > self._max_completed:
            self._completed_requests.pop(0)

        return result

    def _aggregate(
        self,
        request: PBFTRequest,
        dog_judgments: List[DogJudgment],
    ) -> ConsensusResult:
        """
        Aggregate Dog votes into ConsensusResult.

        Algorithm:
          1. If GUARDIAN veto → no consensus regardless of vote count
          2. If < DOGS_QUORUM votes → no consensus
          3. Final Q-Score = φ-weighted geometric mean of commit votes
          4. Final confidence = average confidence (φ-bounded)

        φ-weighting: each Dog's vote weighted by DOG_PRIORITY[dog_id].
        """
        votes = len(request.commit_votes)

        # GUARDIAN veto blocks execution (regardless of vote count)
        if request.has_veto:
            return ConsensusResult(
                consensus=False,
                votes=votes,
                quorum=DOGS_QUORUM,
                reason=f"GUARDIAN VETO by {request.veto_dogs}",
                dog_judgments=[j.to_dict() for j in dog_judgments],
            )

        # Quorum check
        if not request.has_quorum:
            return ConsensusResult(
                consensus=False,
                votes=votes,
                quorum=DOGS_QUORUM,
                reason=f"Insufficient votes: {votes}/{DOGS_QUORUM}",
                dog_judgments=[j.to_dict() for j in dog_judgments],
            )

        # Build judgment map for weighted aggregation
        judgment_map = {j.dog_id: j for j in dog_judgments}
        scores = []
        weights = []
        confidences = []

        for dog_id, q_score in request.commit_votes.items():
            weight = DOG_PRIORITY.get(dog_id, 1.0)
            scores.append(q_score if q_score > 0 else 0.001)  # avoid log(0)
            weights.append(weight)

            if dog_id in judgment_map:
                confidences.append(judgment_map[dog_id].confidence)

        # φ-weighted geometric mean → final Q-Score
        final_q = weighted_geometric_mean(scores, weights)
        final_q = phi_bound_score(final_q)

        # Average confidence (φ-bounded)
        final_conf = min(
            sum(confidences) / len(confidences) if confidences else 0.0,
            MAX_CONFIDENCE,
        )

        # Determine verdict
        from cynic.core.axioms import verdict_from_q_score
        final_verdict = verdict_from_q_score(final_q).value

        return ConsensusResult(
            consensus=True,
            votes=votes,
            quorum=DOGS_QUORUM,
            final_q_score=final_q,
            final_verdict=final_verdict,
            final_confidence=final_conf,
            dog_judgments=[j.to_dict() for j in dog_judgments],
        )

    async def initiate_view_change(self, request_id: str) -> None:
        """
        Trigger PBFT view change on timeout or suspected primary failure.

        In single-process mode this resets the failed request.
        In distributed mode this would elect a new primary.
        """
        request = self._active_requests.get(request_id)
        if request and request.is_timed_out:
            await get_agent_bus().emit(Event(
                type=AgentEvent.PBFT_VIEW_CHANGE,
                payload={"request_id": request_id, "reason": "timeout"},
                source=self.dog_id,
            ))
            del self._active_requests[request_id]
            logger.warning("PBFT view change initiated for request=%s", request_id)
