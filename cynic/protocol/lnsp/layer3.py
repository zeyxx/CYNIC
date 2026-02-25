"""Layer 3: Judge with Axiom Evaluation and Verdict Emission

Layer 3 transforms aggregated state (Layer 2) into φ-bounded verdicts through
axiom evaluation. The Judge coordinates axiom scorers, computes geometric-mean
Q-Scores, determines verdicts, and emits judgments to Layer 4 handlers.

Components:
- RoutingRule: Defines what agents should observe
- Layer3: Judge that evaluates state and emits verdicts
"""
from __future__ import annotations

import math
from collections.abc import Callable
from dataclasses import dataclass, field
from typing import Any

from .axioms import AxiomEvaluator
from .messages import create_judgment
from .types import JudgmentType, Layer, LNSPMessage, VerdictType


@dataclass
class RoutingRule:
    """Rule for routing verdicts to specific agents.

    Routing rules determine which agents should receive which verdicts,
    allowing intelligent filtering and distribution of judgments.

    Attributes:
        target_agent_type: Type of agent (e.g., "Dog", "Handler", "Sensor")
        target_agent_id: Specific agent ID to route to
        observable_types: Types of observables to show (e.g., ["JUDGMENT"])
        verdict_filter: Verdicts to send (e.g., ["HOWL", "GROWL"])
        axiom_filter: Specific axioms to show, or None for all
        emergence_only: Only route emergent patterns (default False)
        priority: Rule priority ("high", "medium", "low", default "medium")
        confidence: Rule confidence score (default 0.618, φ)
    """

    target_agent_type: str
    target_agent_id: str
    observable_types: list[str] = field(default_factory=lambda: ["JUDGMENT"])
    verdict_filter: list[str] = field(default_factory=lambda: ["HOWL", "GROWL", "WAG", "BARK"])
    axiom_filter: list[str] | None = None
    emergence_only: bool = False
    priority: str = "medium"
    confidence: float = 0.618

    def matches(self, verdict: VerdictType, agent_id: str | None = None) -> bool:
        """Check if this rule applies to a verdict.

        Args:
            verdict: The verdict to check
            agent_id: Optional agent ID to check against target

        Returns:
            True if rule matches, False otherwise
        """
        # Check agent ID if provided
        if agent_id is not None and agent_id != self.target_agent_id:
            return False

        # Check verdict is in filter
        if verdict.value not in self.verdict_filter:
            return False

        return True


class Layer3:
    """Layer 3 Judge for axiom evaluation and verdict emission.

    The Judge coordinates axiom evaluators, computes Q-Scores, determines
    verdicts, and emits judgment messages to subscribed Layer 4 handlers.

    Attributes:
        judge_id: Unique identifier for this judge
        axiom_evaluators: Dict mapping axiom names to AxiomEvaluator instances
        routing_rules: List of RoutingRule instances for agent filtering
        subscribers: List of Layer 4 callbacks to receive judgments
    """

    def __init__(self, judge_id: str) -> None:
        """Initialize Layer 3 Judge.

        Args:
            judge_id: Unique identifier for this judge
        """
        self.judge_id = judge_id
        self.axiom_evaluators: dict[str, AxiomEvaluator] = {}
        self.routing_rules: list[RoutingRule] = []
        self.subscribers: list[Callable[[LNSPMessage], None]] = []

    def register_axiom(self, evaluator: AxiomEvaluator) -> None:
        """Register an axiom evaluator.

        Args:
            evaluator: AxiomEvaluator instance to register
        """
        self.axiom_evaluators[evaluator.axiom_name] = evaluator

    def add_routing_rule(self, rule: RoutingRule) -> None:
        """Add a routing rule for agent filtering.

        Args:
            rule: RoutingRule instance to add
        """
        self.routing_rules.append(rule)

    def subscribe(self, callback: Callable[[LNSPMessage], None]) -> None:
        """Subscribe a callback to receive judgment messages.

        Callbacks are called synchronously as judgments are emitted.

        Args:
            callback: Callable that takes an LNSPMessage judgment
        """
        self.subscribers.append(callback)

    async def judge(self, aggregated_state: LNSPMessage) -> LNSPMessage | None:
        """Judge aggregated state and emit verdict.

        Process:
        1. Check message is Layer 2 (AGGREGATED)
        2. Extract state data from payload
        3. Score each axiom
        4. Compute geometric-mean Q-Score (φ-weighted)
        5. Determine verdict based on Q-Score thresholds
        6. Create judgment message
        7. Emit to subscribers
        8. Return judgment message

        Args:
            aggregated_state: Layer 2 aggregated state message

        Returns:
            Layer 3 judgment message, or None if not Layer 2 message

        Verdict Thresholds (Q-Score based):
        - Q < 0.4: HOWL (problem)
        - Q 0.4-0.6: GROWL (caution)
        - Q 0.6-0.8: WAG (healthy)
        - Q >= 0.8: BARK (excellent)
        """
        # Check layer
        if aggregated_state.header.layer != Layer.AGGREGATED:
            return None

        # Extract state data from payload (exclude metadata fields)
        state = {
            k: v for k, v in aggregated_state.payload.items()
            if k not in ["aggregation_type", "based_on"]
        }

        # Score each axiom
        axiom_scores: dict[str, float] = {}
        for axiom_name, evaluator in self.axiom_evaluators.items():
            score = await evaluator.score(state)
            axiom_scores[axiom_name] = max(0.0, min(score, 1.0))  # Clamp to [0, 1]

        # Compute Q-Score using geometric mean, φ-weighted
        q_score = self._compute_q_score(list(axiom_scores.values()))

        # Determine verdict
        verdict = self._determine_verdict(q_score)

        # Create judgment message
        judgment = create_judgment(
            judgment_type=JudgmentType.STATE_EVALUATION,
            verdict=verdict,
            q_score=q_score,
            confidence=0.618,  # φ
            axiom_scores=axiom_scores,
            data={
                "state_summary": state,
                "axiom_count": len(axiom_scores),
            },
            source=self.judge_id,
            target="EXECUTIVE",
            based_on=[aggregated_state.header.message_id],
            instance_id=aggregated_state.metadata.instance_id,
            region=aggregated_state.metadata.region,
        )

        # Emit to subscribers
        for callback in self.subscribers:
            callback(judgment)

        return judgment

    def _compute_q_score(self, scores: list[float]) -> float:
        """Compute φ-weighted geometric mean Q-Score.

        Q-Score uses geometric mean to emphasize average performance:
        geometric_mean = exp(sum(ln(score_i + 0.001)) / n)

        Then φ-weighted (multiply by golden ratio conjugate):
        q_score = geometric_mean * 0.618

        Finally clamped to [0, 1].

        Args:
            scores: List of axiom scores (0.0-1.0)

        Returns:
            Q-Score as float in [0.0, 1.0]
        """
        if not scores:
            return 0.5  # Neutral if no axioms

        # Add epsilon to avoid log(0)
        epsilon = 0.001
        adjusted_scores = [s + epsilon for s in scores]

        # Geometric mean: exp(mean(ln(x)))
        try:
            log_sum = sum(math.log(s) for s in adjusted_scores)
            geometric_mean = math.exp(log_sum / len(adjusted_scores))
        except (ValueError, ZeroDivisionError):
            return 0.5  # Neutral on calculation error

        # φ-weight (golden ratio conjugate: φ⁻¹ = 0.618)
        phi = 0.618
        q_score = geometric_mean * phi

        # Clamp to [0, 1]
        return max(0.0, min(q_score, 1.0))

    def _determine_verdict(self, q_score: float) -> VerdictType:
        """Determine verdict based on Q-Score thresholds.

        Args:
            q_score: Computed Q-Score (0.0-1.0)

        Returns:
            VerdictType enum value

        Thresholds (φ-weighted, max ~0.618):
        - Q < 0.2: HOWL (problem)
        - Q 0.2-0.4: GROWL (caution)
        - Q 0.4-0.6: WAG (healthy)
        - Q >= 0.6: BARK (excellent)
        """
        if q_score < 0.2:
            return VerdictType.HOWL
        elif q_score < 0.4:
            return VerdictType.GROWL
        elif q_score < 0.6:
            return VerdictType.WAG
        else:
            return VerdictType.BARK

    def get_rules_for_agent(self, agent_id: str) -> list[RoutingRule]:
        """Get applicable routing rules for an agent.

        Args:
            agent_id: Agent ID to find rules for

        Returns:
            List of RoutingRule instances for this agent
        """
        return [
            rule for rule in self.routing_rules
            if rule.target_agent_id == agent_id
        ]

    def stats(self) -> dict[str, Any]:
        """Return current statistics about Layer 3 state.

        Returns:
            Dict with keys:
                - judge_id: Judge identifier
                - axiom_count: Number of registered axioms
                - routing_rule_count: Number of routing rules
                - subscriber_count: Number of subscribed callbacks
        """
        return {
            "judge_id": self.judge_id,
            "axiom_count": len(self.axiom_evaluators),
            "routing_rule_count": len(self.routing_rules),
            "subscriber_count": len(self.subscribers),
        }
