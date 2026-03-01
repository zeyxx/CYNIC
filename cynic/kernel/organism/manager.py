"""
Manager Role â€” Organism Agency and Risk Management.

The Manager is the "Will" of the organism. It has the power to veto
decisions from the community if they violate core axioms or if
confidence is too low for the assessed risk.
"""

from __future__ import annotations

import logging
import time
from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from cynic.kernel.core.unified_state import UnifiedJudgment

logger = logging.getLogger("cynic.kernel.organism.manager")


@dataclass
class ManagerDirective:
    """A recommendation from the Manager role."""

    should_execute: bool
    reasoning: str
    risk_level: str  # CAUTIOUS | MEASURED | AGGRESSIVE
    timestamp: float = field(default_factory=time.time)


class OrganismManager:
    """
    Orchestrates organism-level agency.

    Implements should_execute(decision) as required by Phase 3 Spec.
    """

    def __init__(self, confidence_provider: Any):
        self.confidence_provider = confidence_provider  # Usually OrganismState or Consciousness

    def should_execute(self, judgment: UnifiedJudgment) -> tuple[bool, str]:
        """
        Manager decides: Should we actually execute this decision?

        Logic:
        1. Axiom check: If any axiom score is critical (< 0.236), VETO.
        2. Confidence check: If confidence < 0.382 (phi^-2), VETO.
        3. Risk check: If BARK/GROWL but trying to ACT, VETO.
        """
        verdict = judgment.verdict
        confidence = judgment.confidence
        axiom_scores = judgment.axiom_scores or {}

        # 1. Axiom Safety Floor
        for axiom, score in axiom_scores.items():
            if score < 23.6:  # phi^-3 * 100
                return False, f"VETO: Axiom {axiom} is in critical state ({score:.1f})"

        # 2. Epistemic Humility (phi^-2 threshold)
        if confidence < 0.382:
            return False, f"VETO: Confidence too low ({confidence:.3f}) for autonomous action"

        # 3. Verdict Alignment
        if verdict in ("BARK", "GROWL"):
            return False, f"VETO: System disagrees with action (verdict={verdict})"

        return True, "SAFE: Decision meets all organism safety criteria"

    def propose_level(self, risk_profile: dict) -> str:
        """Determines how aggressively to pursue an action."""
        confidence = 0.5  # Default
        if hasattr(self.confidence_provider, "confidence_level"):
            confidence = self.confidence_provider.confidence_level

        if confidence > 0.60:
            return "AGGRESSIVE"
        elif confidence > 0.55:
            return "MEASURED"
        else:
            return "CAUTIOUS"
