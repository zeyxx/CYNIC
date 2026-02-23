"""
CYNIC AlignmentSafetyChecker — Guardrail 2: Alignment validation

Validates that decisions conform to CYNIC's 5 axioms before ACT phase.

Prevents:
  1. Contradicting past judgments (FIDELITY violation)
  2. Unbalanced verdict distributions (PHI violation)
  3. Low-confidence high-impact actions (VERIFY violation)
  4. Novel violations of established patterns (CULTURE violation)
  5. Over-extraction or bloat (BURN violation)
"""
from __future__ import annotations

import logging
from dataclasses import dataclass
from typing import Any

from cynic.core.phi import fibonacci
from cynic.learning.qlearning import VERDICTS

logger = logging.getLogger("cynic.immune.alignment_checker")

# Axiom thresholds (φ-derived)
_MIN_CONFIDENCE_FOR_HIGH_IMPACT = 0.382  # φ⁻² — minimum for BARK decisions
_VERDICT_BALANCE_WINDOW = fibonacci(6)   # 8 judgments — check balance
_MAX_CONTRADICTIONS_BEFORE_BLOCK = 2     # Contradiction threshold


@dataclass
class AlignmentViolation:
    """Records an alignment violation and severity."""
    axiom: str                  # FIDELITY, PHI, VERIFY, CULTURE, BURN
    severity: str               # CRITICAL, WARNING, INFO
    reason: str                 # Explanation
    blocking: bool              # If True, blocks the decision
    recommendation: str         # Suggested fix


class AlignmentSafetyChecker:
    """
    Validates that decisions align with CYNIC's foundational axioms.

    Usage:
        checker = AlignmentSafetyChecker(window_size=8)
        violations = checker.check_alignment(judgment, decision, recent_judgments)
        if any(v.blocking for v in violations):
            # Reject or escalate the decision
    """

    def __init__(self, window_size: int = fibonacci(6)) -> None:
        """
        Args:
            window_size: Number of recent judgments to check for balance (default F(6)=8)
        """
        self._window_size = window_size
        self._recent_verdicts: list[str] = []

    def start(self) -> None:
        """Start alignment checker monitoring."""
        logger.info("AlignmentSafetyChecker started — validating axiom alignment")

    def check_alignment(
        self,
        judgment: Any,  # cynic.core.judgment.Judgment
        decision: dict[str, Any],  # From DecideAgent
        recent_judgments: list[Any],  # Past N judgments for history check
    ) -> list[AlignmentViolation]:
        """
        Check if decision aligns with CYNIC's 5 axioms.

        Args:
            judgment: Current judgment that triggered the decision
            decision: Decision dict with verdict, confidence, q_value, etc.
            recent_judgments: Past judgments for historical analysis

        Returns:
            List of AlignmentViolation objects (empty if all pass)
        """
        violations: list[AlignmentViolation] = []

        verdict = decision.get("verdict", "")
        confidence = float(decision.get("confidence", 0.0))
        q_value = float(decision.get("q_value", 0.0))

        # FIDELITY: Check for contradictions with past judgments
        fidelity_violations = self._check_fidelity(verdict, recent_judgments)
        violations.extend(fidelity_violations)

        # PHI: Check verdict distribution balance
        phi_violations = self._check_phi_balance(verdict)
        violations.extend(phi_violations)

        # VERIFY: Confidence high enough for the decision impact
        verify_violations = self._check_verification(verdict, confidence)
        violations.extend(verify_violations)

        # CULTURE: Check for established patterns being violated
        culture_violations = self._check_culture(verdict, recent_judgments)
        violations.extend(culture_violations)

        # BURN: Check for minimal/focused nature (not extractive)
        burn_violations = self._check_burn(decision)
        violations.extend(burn_violations)

        # Log violations
        for v in violations:
            level = "ERROR" if v.blocking else "WARNING"
            logger.log(
                logging.ERROR if v.blocking else logging.WARNING,
                f"[{v.axiom}] {level}: {v.reason}",
            )

        return violations

    def record_verdict(self, verdict: str) -> None:
        """Record a verdict for rolling balance check."""
        self._recent_verdicts.append(verdict)
        # Keep window size limited
        if len(self._recent_verdicts) > self._window_size:
            self._recent_verdicts.pop(0)

    # ── Private ────────────────────────────────────────────────────────

    def _check_fidelity(
        self,
        verdict: str,
        recent_judgments: list[Any],
    ) -> list[AlignmentViolation]:
        """Check for contradictions with recent judgment history (FIDELITY)."""
        violations = []

        if not recent_judgments or verdict not in VERDICTS:
            return violations

        # Count contradictions: current verdict vs last 3 judgments
        contradictions = 0
        for past_judgment in recent_judgments[-3:]:
            if hasattr(past_judgment, "verdict") and past_judgment.verdict != verdict:
                contradictions += 1

        if contradictions >= _MAX_CONTRADICTIONS_BEFORE_BLOCK:
            violations.append(
                AlignmentViolation(
                    axiom="FIDELITY",
                    severity="CRITICAL",
                    reason=f"Verdict contradicts {contradictions} of last 3 judgments",
                    blocking=True,
                    recommendation="Verify judgment data; escalate to human review",
                )
            )
        elif contradictions > 0:
            violations.append(
                AlignmentViolation(
                    axiom="FIDELITY",
                    severity="WARNING",
                    reason=f"Verdict contradicts {contradictions} recent judgment(s)",
                    blocking=False,
                    recommendation="Check for pattern drift",
                )
            )

        return violations

    def _check_phi_balance(self, verdict: str) -> list[AlignmentViolation]:
        """Check verdict distribution maintains φ balance (PHI)."""
        violations = []

        if not self._recent_verdicts or verdict not in VERDICTS:
            return violations

        # Count verdict frequencies in window
        verdict_counts = {v: self._recent_verdicts.count(v) for v in VERDICTS}
        current_count = verdict_counts.get(verdict, 0)

        # BARK should be ≤ φ⁻² × window (38.2% of verdicts)
        # Other verdicts should be ≤ φ⁻¹ × window (61.8% of verdicts)
        bark_limit = int(self._window_size * 0.382)
        other_limit = int(self._window_size * 0.618)

        if verdict == "BARK" and current_count >= bark_limit:
            violations.append(
                AlignmentViolation(
                    axiom="PHI",
                    severity="WARNING",
                    reason=f"BARK at {current_count}/{self._window_size} exceeds φ⁻² threshold",
                    blocking=False,
                    recommendation="Consider escalating to GROWL/WAG if less critical",
                )
            )

        return violations

    def _check_verification(self, verdict: str, confidence: float) -> list[AlignmentViolation]:
        """Check confidence supports the decision impact (VERIFY)."""
        violations = []

        # BARK (critical) requires high confidence
        if verdict == "BARK" and confidence < _MIN_CONFIDENCE_FOR_HIGH_IMPACT:
            violations.append(
                AlignmentViolation(
                    axiom="VERIFY",
                    severity="CRITICAL",
                    reason=f"BARK verdict requires confidence ≥ {_MIN_CONFIDENCE_FOR_HIGH_IMPACT:.1%}, got {confidence:.1%}",
                    blocking=True,
                    recommendation="Lower to GROWL or improve confidence before proceeding",
                )
            )

        # GROWL requires moderate confidence
        if verdict == "GROWL" and confidence < 0.3:
            violations.append(
                AlignmentViolation(
                    axiom="VERIFY",
                    severity="WARNING",
                    reason=f"GROWL verdict has low confidence: {confidence:.1%}",
                    blocking=False,
                    recommendation="Verify evidence before acting",
                )
            )

        return violations

    def _check_culture(
        self,
        verdict: str,
        recent_judgments: list[Any],
    ) -> list[AlignmentViolation]:
        """Check decision respects established patterns (CULTURE)."""
        violations = []

        # Novel verdicts (never seen before in recent history) warrant caution
        if recent_judgments and all(
            verdict != getattr(j, "verdict", "") for j in recent_judgments[-5:]
        ):
            # All 5 recent verdicts differ from current
            if verdict in {"BARK", "GROWL"}:  # Only flag for critical verdicts
                violations.append(
                    AlignmentViolation(
                        axiom="CULTURE",
                        severity="WARNING",
                        reason=f"Verdict {verdict} is novel (not seen in last 5 judgments)",
                        blocking=False,
                        recommendation="Verify this is intentional pattern shift",
                    )
                )

        return violations

    def _check_burn(self, decision: dict[str, Any]) -> list[AlignmentViolation]:
        """Check decision is minimal and focused (BURN)."""
        violations = []

        # Action prompt length — extract bloat detector
        action_prompt = decision.get("action_prompt", "")
        if len(action_prompt) > 1000:
            violations.append(
                AlignmentViolation(
                    axiom="BURN",
                    severity="WARNING",
                    reason=f"Action prompt too long ({len(action_prompt)} chars > 1000)",
                    blocking=False,
                    recommendation="Truncate to core directive only",
                )
            )

        # Recommended action should be one of VERDICTS (minimal)
        recommended_action = decision.get("recommended_action", "")
        if recommended_action not in VERDICTS:
            violations.append(
                AlignmentViolation(
                    axiom="BURN",
                    severity="WARNING",
                    reason=f"Recommended action '{recommended_action}' not a standard verdict",
                    blocking=False,
                    recommendation="Use standard verdict actions",
                )
            )

        return violations

    # ---- Stats --------------------------------------------------------

    def stats(self) -> dict[str, Any]:
        """Return checker statistics."""
        return {
            "recent_verdict_count": len(self._recent_verdicts),
            "recent_verdicts": self._recent_verdicts.copy(),
            "window_size": self._window_size,
        }
