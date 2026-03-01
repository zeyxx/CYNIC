"""Governance handlers for LNSP integration."""

from __future__ import annotations

from typing import Any

from cynic.kernel.protocol.lnsp.layer4 import Handler
from cynic.kernel.protocol.lnsp.types import LNSPMessage, VerdictType


class GovernanceVerdictHandler(Handler):
    """Converts LNSP verdicts to CYNIC JUDGMENT_CREATED events."""

    async def handle(self, verdict: LNSPMessage) -> tuple[bool, Any]:
        """Execute governance verdict.

        Converts LNSP verdict type to CYNIC verdict string:
        - BARK (Q >= 0.8) -> APPROVED
        - WAG (Q 0.6-0.8) -> TENTATIVE_APPROVE
        - GROWL (Q 0.4-0.6) -> CAUTION
        - HOWL (Q < 0.4) -> REJECT
        """
        try:
            data = verdict.payload.get("data", {})
            verdict_type_str = data.get("verdict_type")
            q_score = data.get("q_score", 0.0)
            proposal_id = data.get("proposal_id", "unknown")

            # Map verdict type to CYNIC verdict string
            verdict_map = {
                VerdictType.BARK.value: "APPROVED",
                VerdictType.WAG.value: "TENTATIVE_APPROVE",
                VerdictType.GROWL.value: "CAUTION",
                VerdictType.HOWL.value: "REJECT",
            }

            cynic_verdict = verdict_map.get(verdict_type_str, "UNKNOWN")

            return (
                True,
                {
                    "verdict": cynic_verdict,
                    "confidence": q_score,
                    "proposal_id": proposal_id,
                    "verdict_type": verdict_type_str,
                    "axiom_scores": data.get("axiom_scores", {}),
                    "reasoning": data.get("reasoning", ""),
                },
            )
        except Exception as e:
            return (False, {"error": str(e)})
