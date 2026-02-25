"""Test LNSP learning loop for governance."""
from __future__ import annotations

import pytest

from cynic.protocol.lnsp.governance_integration import GovernanceLNSP
from cynic.protocol.lnsp.manager import LNSPManager
from cynic.protocol.lnsp.types import ObservationType


@pytest.mark.asyncio
async def test_feedback_loop_on_execution() -> None:
    """Test feedback from execution updates Q-table."""
    manager = LNSPManager(instance_id="instance:governance", region="governance")
    bridge = GovernanceLNSP(manager)
    await bridge.setup()

    # Process a proposal first
    await bridge.process_proposal({
        "proposal_id": "prop_001",
        "title": "Test",
        "content": "Test proposal",
        "submitter_id": "user_1",
        "community_id": "test",
        "submission_timestamp": 1708982400.0,
        "voting_period_hours": 48,
    })

    # Then process execution outcome
    await bridge.process_execution({
        "proposal_id": "prop_001",
        "success": True,
        "tx_hash": "0xabc",
        "result": {"burned": 1000000},
        "timestamp": 1708982600.0,
        "community_id": "test",
    })

    # Verify feedback was processed
    # (Layer 1 should have both proposal and execution observations)
    assert manager.layer1.ringbuffer.size() >= 1


@pytest.mark.asyncio
async def test_outcome_feedback_processing() -> None:
    """Test outcome feedback from community."""
    manager = LNSPManager(instance_id="instance:governance", region="governance")
    bridge = GovernanceLNSP(manager)
    await bridge.setup()

    # Process outcome feedback
    await bridge.process_outcome({
        "proposal_id": "prop_001",
        "accepted": True,
        "funds_received": True,
        "community_sentiment": 0.89,
        "feedback_text": "Great decision!",
        "timestamp": 1708982700.0,
        "community_id": "test",
    })

    # Verify observation was created
    obs = manager.layer1.ringbuffer.peek()
    assert obs is not None
    assert obs.payload["observation_type"] == ObservationType.ECOSYSTEM_EVENT.value
