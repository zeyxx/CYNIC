"""
CYNIC DecisionValidator — Integration layer for all guardrails

Chains guardrails in sequence:
  1. PowerLimiter.check_available() → blocks if overloaded
  2. AlignmentChecker.check_alignment() → blocks if axioms violated
  3. HumanApprovalGate.requires_approval() → gates execution
  4. TransparencyAuditTrail.record_*() → audit trail

Single entry point for decision validation. All guardrails must pass
before decision proceeds to ACT phase.

Raises BlockedDecision if any guardrail blocks, with full context
for escalation/human review.
"""
from __future__ import annotations

import logging
from dataclasses import dataclass
from typing import Any, Optional

from cynic.immune.power_limiter import PowerLimiter
from cynic.immune.alignment_checker import AlignmentSafetyChecker
from cynic.immune.human_approval_gate import HumanApprovalGate
from cynic.immune.transparency_audit import TransparencyAuditTrail

logger = logging.getLogger("cynic.cognition.cortex.decision_validator")


class BlockedDecision(Exception):
    """Decision was blocked by a guardrail."""

    def __init__(
        self,
        reason: str,
        guardrail: str,
        blocking_rule: str,
        recommendation: str,
    ):
        self.reason = reason
        self.guardrail = guardrail
        self.blocking_rule = blocking_rule
        self.recommendation = recommendation
        super().__init__(self.reason)


@dataclass
class ValidatedDecision:
    """A decision that has passed all guardrail checks."""

    decision_id: str
    verdict: str
    confidence: float
    action_prompt: str
    approved_by_human: bool = False
    audit_record_id: Optional[str] = None


class DecisionValidator:
    """
    Integration layer that chains all guardrails.

    Validates decisions sequentially:
      1. System load (PowerLimiter)
      2. Axiom alignment (AlignmentChecker)
      3. Human approval gates (HumanApprovalGate)
      4. Record in audit trail (TransparencyAuditTrail)

    All guardrails must pass for decision to proceed.

    Usage:
        validator = DecisionValidator(
            power_limiter=limiter,
            alignment_checker=checker,
            human_gate=gate,
            audit_trail=trail,
        )

        try:
            validated = await validator.validate_decision(
                decision={"verdict": "BARK", ...},
                judgment=judgment_obj,
                recent_judgments=[...],
                scheduler=scheduler,
            )
            # Decision passed all checks
            await execute_action(validated)
        except BlockedDecision as e:
            # Decision blocked
            logger.error(f"Decision blocked: {e.guardrail} — {e.reason}")
            # Human will review via approval gate
    """

    def __init__(
        self,
        power_limiter: PowerLimiter,
        alignment_checker: AlignmentSafetyChecker,
        human_gate: HumanApprovalGate,
        audit_trail: TransparencyAuditTrail,
    ):
        """
        Args:
            power_limiter: Resource monitoring guardrail
            alignment_checker: Axiom validation guardrail
            human_gate: High-impact decision gating
            audit_trail: Immutable decision recording
        """
        self._power_limiter = power_limiter
        self._alignment_checker = alignment_checker
        self._human_gate = human_gate
        self._audit_trail = audit_trail

    async def validate_decision(
        self,
        decision: dict[str, Any],
        judgment: Any,  # cynic.core.judgment.Judgment
        recent_judgments: list[Any],
        scheduler: Any,  # ConsciousnessRhythm
    ) -> ValidatedDecision:
        """
        Validate a decision against all guardrails.

        Args:
            decision: From DecideAgent.decide_for_judgment()
            judgment: The judgment that triggered the decision
            recent_judgments: Past judgments for alignment check
            scheduler: ConsciousnessRhythm for PowerLimiter

        Returns:
            ValidatedDecision if all guardrails pass

        Raises:
            BlockedDecision if any guardrail blocks
        """
        import uuid

        verdict = decision.get("verdict", "")
        confidence = float(decision.get("confidence", 0.0))
        q_score = float(decision.get("q_score", 0.0))
        action_prompt = decision.get("action_prompt", "")
        judgment_id = decision.get("judgment_id", "")

        decision_id = str(uuid.uuid4())

        # Step 1: POWER LIMITER — Check system capacity
        if not self._power_limiter.check_available(scheduler):
            recommended_level = self._power_limiter.recommended_level(scheduler)
            raise BlockedDecision(
                reason="System overloaded — cannot accept more work",
                guardrail="PowerLimiter",
                blocking_rule=f"CPU/Memory/Queue limit exceeded, recommend {recommended_level.name}",
                recommendation="Escalate to lower consciousness level or wait for system recovery",
            )

        # Step 2: ALIGNMENT CHECKER — Validate against 5 axioms
        alignment_violations = self._alignment_checker.check_alignment(
            judgment, decision, recent_judgments
        )

        blocking_violations = [v for v in alignment_violations if v.blocking]
        if blocking_violations:
            violation_reasons = [f"{v.axiom}: {v.reason}" for v in blocking_violations]
            raise BlockedDecision(
                reason=f"Axiom violations: {'; '.join(violation_reasons)}",
                guardrail="AlignmentChecker",
                blocking_rule=f"{len(blocking_violations)} critical axiom violations",
                recommendation="; ".join([v.recommendation for v in blocking_violations]),
            )

        # Step 3: TRANSPARENCY AUDIT — Record decision in audit trail
        audit_record = self._audit_trail.record_decision(
            judgment_id=judgment_id,
            verdict=verdict,
            confidence=confidence,
            q_score=q_score,
        )

        # Record alignment results
        self._audit_trail.record_alignment_check(
            audit_record.record_id,
            [v.__dict__ for v in alignment_violations],
        )

        # Record action recommendation
        self._audit_trail.record_decision_recommendation(
            audit_record.record_id,
            recommended_action=decision.get("recommended_action", ""),
            action_prompt=action_prompt,
        )

        # Step 4: HUMAN APPROVAL GATE — Check if human approval needed
        if self._human_gate.requires_approval(decision, alignment_violations):
            # Create approval request
            approval_request = self._human_gate.create_approval_request(
                record_id=audit_record.record_id,
                verdict=verdict,
                confidence=confidence,
                q_score=q_score,
                action_prompt=action_prompt,
                reason=f"High-impact {verdict} decision requires human approval",
                blocking_violations=[v.axiom for v in alignment_violations],
                risk_level="CRITICAL" if verdict == "BARK" else "HIGH",
            )

            logger.warning(
                f"Decision gated for human approval: {approval_request.request_id}"
            )

            raise BlockedDecision(
                reason=f"Decision requires human approval (ID: {approval_request.request_id})",
                guardrail="HumanApprovalGate",
                blocking_rule="High-impact decision gating",
                recommendation=f"Human must review and approve via /approve/{approval_request.request_id}",
            )

        # Record power limiter check
        self._power_limiter.record_judgment()

        # Record verdict for balance tracking
        self._alignment_checker.record_verdict(verdict)

        logger.info(
            f"Decision validated: {verdict} (conf={confidence:.1%}, audit={audit_record.record_id})"
        )

        return ValidatedDecision(
            decision_id=decision_id,
            verdict=verdict,
            confidence=confidence,
            action_prompt=action_prompt,
            approved_by_human=False,
            audit_record_id=audit_record.record_id,
        )

    def stats(self) -> dict[str, Any]:
        """Return validation statistics from all guardrails."""
        return {
            "power_limiter": self._power_limiter.stats(),
            "alignment_checker": self._alignment_checker.stats(),
            "human_gate": self._human_gate.stats(),
            "audit_trail": self._audit_trail.stats(),
        }
