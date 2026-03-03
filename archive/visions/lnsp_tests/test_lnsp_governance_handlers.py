"""Test governance handlers for LNSP integration."""

from __future__ import annotations

import pytest

from cynic.kernel.protocol.lnsp.governance_handlers import GovernanceVerdictHandler
from cynic.kernel.protocol.lnsp.messages import create_judgment
from cynic.kernel.protocol.lnsp.types import JudgmentType, VerdictType


@pytest.mark.asyncio
async def test_handler_convert_bark_verdict():
    """Test handler converts BARK verdict to APPROVED."""
    handler = GovernanceVerdictHandler("handler:governance")

    verdict = create_judgment(
        judgment_type=JudgmentType.STATE_EVALUATION,
        data={
            "proposal_id": "prop_001",
            "verdict_type": VerdictType.BARK.value,
            "q_score": 0.87,
            "axiom_scores": {
                "fidelity": 0.95,
                "phi": 0.82,
                "verify": 0.91,
                "culture": 0.88,
                "burn": 0.95,
            },
            "reasoning": "All axioms strong",
        },
        source="judge:central",
    )

    success, result = await handler.handle(verdict)
    assert success is True
    assert result["verdict"] == "APPROVED"
    assert result["confidence"] == 0.87


@pytest.mark.asyncio
async def test_handler_convert_wag_verdict():
    """Test handler converts WAG verdict to TENTATIVE_APPROVE."""
    handler = GovernanceVerdictHandler("handler:governance")

    verdict = create_judgment(
        judgment_type=JudgmentType.STATE_EVALUATION,
        data={
            "proposal_id": "prop_002",
            "verdict_type": VerdictType.WAG.value,
            "q_score": 0.72,
            "axiom_scores": {
                "fidelity": 0.75,
                "phi": 0.70,
                "verify": 0.75,
                "culture": 0.70,
                "burn": 0.75,
            },
        },
        source="judge:central",
    )

    success, result = await handler.handle(verdict)
    assert success is True
    assert result["verdict"] == "TENTATIVE_APPROVE"


@pytest.mark.asyncio
async def test_handler_convert_growl_verdict():
    """Test handler converts GROWL verdict to CAUTION."""
    handler = GovernanceVerdictHandler("handler:governance")

    verdict = create_judgment(
        judgment_type=JudgmentType.STATE_EVALUATION,
        data={
            "proposal_id": "prop_003",
            "verdict_type": VerdictType.GROWL.value,
            "q_score": 0.55,
            "axiom_scores": {},
        },
        source="judge:central",
    )

    success, result = await handler.handle(verdict)
    assert success is True
    assert result["verdict"] == "CAUTION"


@pytest.mark.asyncio
async def test_handler_convert_howl_verdict():
    """Test handler converts HOWL verdict to REJECT."""
    handler = GovernanceVerdictHandler("handler:governance")

    verdict = create_judgment(
        judgment_type=JudgmentType.STATE_EVALUATION,
        data={
            "proposal_id": "prop_004",
            "verdict_type": VerdictType.HOWL.value,
            "q_score": 0.25,
            "axiom_scores": {},
        },
        source="judge:central",
    )

    success, result = await handler.handle(verdict)
    assert success is True
    assert result["verdict"] == "REJECT"
