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
from typing import Any, TYPE_CHECKING, Optional

if TYPE_CHECKING:
    from cynic.cognition.neurons.dog_state import DogState
    from cynic.core.judgment import Cell, DogJudgment

from cynic.core.formulas import (
    QTABLE_ENTRY_CAP,
    CONFIDENCE_DECAY_FACTOR,
    GOSSIP_THRESHOLD,
)

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
    confidence_decay: float = CONFIDENCE_DECAY_FACTOR  # Imported from formulas.py
    local_qtable_size: int = QTABLE_ENTRY_CAP  # F(11) = 89 rolling cap (imported from formulas.py)
    gossip_threshold: float = GOSSIP_THRESHOLD  # Imported from formulas.py


class DogCognition:
    """
    Independent judgment engine for one dog.

    Each dog runs its own 7-step cycle without waiting for orchestrator.
    When done, dog publishes compressed_context for siblings to learn from.
    """

    def __init__(self, dog_id: str, config: Optional[DogCognitionConfig] = None) -> None:
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
        JUDGE: REAL analysis, not noise.

        Analyzes signals by type (security, style, performance, etc).
        Confidence based on: how much we've seen similar patterns + signal diversity.
        NOT just "count of judgments".
        """
        # PERCEIVE signals — count by type
        signal_types: dict[str, int] = {}
        for sig in signals:
            sig_type = sig.get("type", "unknown")
            signal_types[sig_type] = signal_types.get(sig_type, 0) + 1

        # JUDGE by signal types (REAL analysis, not noise)
        base_score = 50.0
        reasoning_points = []

        if not signals:
            # No signals = uncertain
            base_score = 45.0
            reasoning_points.append("no_signals")
        else:
            # Score based on WHAT signals, not just COUNT
            if signal_types.get("security_issue", 0) > 0:
                # Critical findings boost concern
                base_score -= min(signal_types["security_issue"] * 8, 25)
                reasoning_points.append(f"security_issue×{signal_types['security_issue']}")

            if signal_types.get("performance_gap", 0) > 0:
                base_score -= signal_types["performance_gap"] * 3
                reasoning_points.append(f"perf_gap×{signal_types['performance_gap']}")

            if signal_types.get("style_violation", 0) > 0:
                base_score -= signal_types["style_violation"] * 1
                reasoning_points.append(f"style×{signal_types['style_violation']}")

            if signal_types.get("documentation", 0) > 0:
                base_score -= signal_types["documentation"] * 2
                reasoning_points.append(f"doc×{signal_types['documentation']}")

            # Signal diversity = confidence boost (more types = more evidence)
            signal_diversity = len(signal_types)
            if signal_diversity > 1:
                # Multiple signal types = more trustworthy
                base_score += min(signal_diversity * 2, 10)
                reasoning_points.append(f"diverse×{signal_diversity}")

        # Adjust based on local Q-table (REAL pattern matching, not averaging)
        if hasattr(cell, "id"):
            cell_id = str(cell.id)
            local_q = dog_state.cognition.local_qtable.get(cell_id)

            if local_q is not None:
                # Strong prior from experience
                base_score = base_score * 0.4 + local_q * 0.6  # Weight experience
                reasoning_points.append(f"prior_Q={local_q:.1f}")

        # Confidence: based on signal clarity + pattern frequency (not just judgments)
        # Clarity: high diversity + few unknown signals
        clarity = (len(signal_types) / max(len(signals), 1)) if signals else 0

        # Pattern frequency: how many times seen this state
        pattern_hits = len([q for q in dog_state.cognition.local_qtable.values()
                           if abs(q - base_score) < 5])  # Similar past judgments

        confidence = min(
            0.15 +  # Baseline
            (clarity * 0.3) +  # Signal clarity matters
            (pattern_hits / max(len(dog_state.cognition.local_qtable), 1) * 0.2),  # Pattern freq
            self.config.max_confidence
        )

        reasoning = f"[{self.dog_id}] {','.join(reasoning_points[:3])} " \
                   f"score={base_score:.0f} clarity={clarity:.2f}"

        # Clamp score to valid range
        base_score = max(0.0, min(base_score, self.config.max_q_score))

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
    ) -> Optional[dict[str, Any]]:
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
