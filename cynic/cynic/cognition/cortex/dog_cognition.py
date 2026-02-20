"""
DogCognition — Mini-Orchestrator enabling autonomous dog judgment

Each dog is a mini-CYNIC that can:
  1. PERCEIVE: Observe signals in its domain (no gossip needed yet)
  2. JUDGE: Run local analysis (fast, domain-specific)
  3. DECIDE: Create verdict (bounded by φ, domain-aware)
  4. ACT: Execute local action (if needed)
  5. LEARN: Update local Q-table from feedback
  6. RESIDUAL: Detect anomalies in domain
  7. EVOLVE: Adjust strategy based on residuals

This removes the orchestrator bottleneck:
  - Old: Dog perceives → sends to Orchestrator → Orchestrator judges all dogs
  - New: Dog perceives, judges locally, gossips summary

Result: Cost ∝ log(N) instead of N
"""
from __future__ import annotations

import time
import logging
from dataclasses import dataclass
from typing import Any, TYPE_CHECKING

if TYPE_CHECKING:
    from cynic.cognition.neurons.dog_state import DogState
    from cynic.core.judgment import Cell, DogJudgment

logger = logging.getLogger("cynic.cognition.cortex.dog_cognition")


# ═══════════════════════════════════════════════════════════════════════════
# DOGCOGNITION — INDEPENDENT JUDGMENT ENGINE
# ═══════════════════════════════════════════════════════════════════════════


@dataclass
class DogCognitionConfig:
    """Configuration for a dog's independent judgment."""
    max_q_score: float = 100.0
    min_confidence: float = 0.0
    max_confidence: float = 0.618  # φ-bounded
    confidence_decay: float = 0.95  # Older judgments fade in confidence
    local_qtable_size: int = 89  # F(11) rolling cap
    gossip_threshold: float = 0.5  # Only gossip if confidence > 50%


class DogCognition:
    """
    Independent judgment engine for one dog.

    Each dog runs its own 7-step cycle without waiting for orchestrator.
    When done, dog publishes compressed_context for siblings to learn from.
    """

    def __init__(self, dog_id: str, config: DogCognitionConfig | None = None) -> None:
        self.dog_id = dog_id
        self.config = config or DogCognitionConfig()
        self._judgment_count = 0
        self._error_count = 0
        self._last_error = ""

    async def judge_cell(self, cell: Cell, dog_state: DogState) -> DogJudgment:
        """
        Independent judgment: Perceive → Judge → Decide → Act

        Dog analyzes cell in its domain WITHOUT orchestrator involvement.
        Returns judgment immediately, then learns from feedback asynchronously.

        **Returns**: DogJudgment with:
          - q_score: domain-specific quality [0, 100]
          - confidence: φ-bounded [0, 0.618]
          - reasoning: why this dog thinks X
        """
        start_ms = time.time() * 1000

        try:
            # STEP 1: PERCEIVE — What signals are available?
            signals = self._perceive_domain(dog_state, cell)

            # STEP 2: JUDGE — Analyze with local expertise
            q_score, confidence, reasoning = await self._judge_domain(
                dog_state, cell, signals
            )

            # STEP 3: DECIDE — Create verdict
            verdict = self._decide_verdict(q_score)

            # STEP 4: ACT — Execute domain-specific action (if needed)
            await self._act_domain(dog_state, cell, verdict)

            # STEP 5: LEARN — Update local Q-table
            await self._learn_from_judgment(dog_state, cell, q_score)

            # STEP 6: RESIDUAL — Detect anomalies
            residual = self._detect_residual(dog_state, q_score, confidence)

            # STEP 7: EVOLVE — Adjust strategy
            if residual:
                await self._evolve_strategy(dog_state, residual)

            # Update counters
            self._judgment_count += 1
            dog_state.cognition.judgment_count += 1
            dog_state.cognition.last_verdict = verdict
            dog_state.cognition.last_q_score = q_score
            dog_state.last_judgment_at = time.time()

            # Build judgment (simulating DogJudgment structure)
            judgment = type("DogJudgment", (), {
                "dog_id": self.dog_id,
                "cell_id": getattr(cell, "id", "unknown"),
                "q_score": q_score,
                "confidence": confidence,
                "verdict": verdict,  # Add as top-level field
                "reasoning": reasoning,
                "evidence": {
                    "signals": len(signals),
                    "residual": bool(residual),
                },
                "latency_ms": time.time() * 1000 - start_ms,
                "cost_usd": 0.0,  # Local judgment has no LLM cost
                "llm_id": None,
                "veto": False,
            })()

            logger.debug(
                f"[{self.dog_id}] Judged {getattr(cell, 'id', '?')}: "
                f"Q={q_score:.1f} conf={confidence:.3f} v={verdict}"
            )

            return judgment

        except Exception as e:
            self._error_count += 1
            self._last_error = str(e)
            logger.error(f"[{self.dog_id}] Judgment failed: {e}", exc_info=True)
            raise

    def _perceive_domain(
        self, dog_state: DogState, cell: Cell
    ) -> list[dict[str, Any]]:
        """PERCEIVE: Gather domain-specific signals."""
        signals = []

        # Each dog perceives different things based on domain
        # SAGE perceives: knowledge graphs, RDF triples
        # ANALYST perceives: type errors, formal properties
        # GUARDIAN perceives: security anomalies
        # etc.

        # For now, collect whatever signals are in dog_state.senses
        signals.extend(dog_state.senses.observed_signals[-5:])  # Last 5 signals
        return signals

    async def _judge_domain(
        self, dog_state: DogState, cell: Cell, signals: list[dict[str, Any]]
    ) -> tuple[float, float, str]:
        """
        JUDGE: Analyze with local expertise.

        Domain-specific judgment based on:
        - What signals are available
        - What patterns the dog has learned
        - Local Q-table experience
        """
        # Start with neutral score
        base_score = 50.0

        # Boost score if we have relevant signals
        if signals:
            base_score += min(len(signals) * 5.0, 20.0)

        # Adjust based on local Q-table experience
        if hasattr(cell, "id"):
            local_q = dog_state.cognition.local_qtable.get(str(cell.id), 50.0)
            base_score = (base_score + local_q) / 2  # Average with experience

        # Calculate confidence: higher if we've judged similar things before
        judgment_freq = dog_state.cognition.judgment_count
        confidence = min(0.2 + (judgment_freq * 0.01), self.config.max_confidence)

        # Apply decay: older judgments less confident
        if dog_state.cognition.confidence_history:
            avg_confidence = sum(dog_state.cognition.confidence_history) / len(
                dog_state.cognition.confidence_history
            )
            confidence = confidence * 0.7 + avg_confidence * 0.3

        reasoning = f"[{self.dog_id}] Analyzed with {len(signals)} signals, " \
                   f"experience={judgment_freq} judgments"

        return base_score, min(confidence, self.config.max_confidence), reasoning

    def _decide_verdict(self, q_score: float) -> str:
        """DECIDE: Classify q_score into verdict."""
        # φ-based thresholds (same as organism)
        if q_score >= 82.0:
            return "HOWL"
        elif q_score >= 61.8:
            return "WAG"
        elif q_score >= 38.2:
            return "GROWL"
        else:
            return "BARK"

    async def _act_domain(
        self, dog_state: DogState, cell: Cell, verdict: str
    ) -> None:
        """ACT: Execute domain-specific action if verdict warrants."""
        if verdict in ("BARK", "GROWL"):
            # Queue action for review (don't auto-execute)
            action = {
                "dog_id": self.dog_id,
                "cell_id": getattr(cell, "id", "unknown"),
                "verdict": verdict,
                "timestamp": time.time(),
                "status": "PENDING",
            }
            dog_state.metabolism.pending_actions.append(action)

    async def _learn_from_judgment(
        self, dog_state: DogState, cell: Cell, q_score: float
    ) -> None:
        """LEARN: Update local Q-table and confidence history."""
        cell_id = str(getattr(cell, "id", "unknown"))

        # Update local Q-table (temporal difference learning)
        old_q = dog_state.cognition.local_qtable.get(cell_id, 50.0)
        alpha = 0.038  # Learning rate (same as organism's QTable)
        new_q = old_q + alpha * (q_score - old_q)
        dog_state.cognition.local_qtable[cell_id] = new_q

        # Keep Q-table size bounded (F(11)=89)
        if len(dog_state.cognition.local_qtable) > self.config.local_qtable_size:
            # Remove oldest entries
            items = list(dog_state.cognition.local_qtable.items())
            dog_state.cognition.local_qtable = dict(
                items[-self.config.local_qtable_size :]
            )

        # Update confidence history
        dog_state.cognition.confidence_history.append(
            min(0.2 + (self._judgment_count * 0.01), self.config.max_confidence)
        )
        if len(dog_state.cognition.confidence_history) > 21:  # F(8) rolling window
            dog_state.cognition.confidence_history.pop(0)

    def _detect_residual(
        self, dog_state: DogState, q_score: float, confidence: float
    ) -> dict[str, Any] | None:
        """RESIDUAL: Detect anomalies that signal learning gaps."""
        # Residual if we're low-confidence about a low-score judgment (uncertain + bad)
        if q_score < 38.2 and confidence < 0.3:
            return {
                "type": "SPIKE",  # Sudden uncertainty
                "q_score": q_score,
                "confidence": confidence,
                "dog_id": self.dog_id,
            }

        # Residual if we've judged same cell many times (pattern instability)
        if (
            dog_state.cognition.local_qtable
            and self._judgment_count > 10
            and len(set(dog_state.cognition.local_qtable.values())) > 5
        ):
            return {
                "type": "INSTABILITY",  # Inconsistent on same cell
                "variance": max(dog_state.cognition.local_qtable.values())
                - min(dog_state.cognition.local_qtable.values()),
                "dog_id": self.dog_id,
            }

        return None

    async def _evolve_strategy(
        self, dog_state: DogState, residual: dict[str, Any]
    ) -> None:
        """EVOLVE: Adjust judgment strategy based on residuals."""
        # Record residual for learning
        dog_state.memory.residual_cases.append(residual)

        # Adjust confidence decay if residuals detected
        if residual["type"] == "SPIKE":
            self.config.confidence_decay = max(0.90, self.config.confidence_decay - 0.01)

        logger.debug(f"[{self.dog_id}] Evolution triggered by {residual['type']}")

    def health_check(self) -> dict[str, Any]:
        """Return health metrics for this dog's cognition."""
        total = self._judgment_count + self._error_count
        error_rate = (self._error_count / total * 100) if total > 0 else 0.0

        return {
            "dog_id": self.dog_id,
            "judgment_count": self._judgment_count,
            "error_count": self._error_count,
            "error_rate_pct": round(error_rate, 2),
            "last_error": self._last_error,
            "healthy": error_rate < 5.0,
        }
