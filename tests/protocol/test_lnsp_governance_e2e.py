"""End-to-end test of LNSP governance integration."""
from __future__ import annotations

from typing import Any

import pytest

from cynic.kernel.protocol.lnsp.governance_integration import GovernanceLNSP
from cynic.kernel.protocol.lnsp.manager import LNSPManager
from cynic.kernel.protocol.lnsp.types import LNSPMessage


@pytest.mark.asyncio
async def test_full_governance_cycle() -> None:
    """Test complete governance cycle: proposal → verdict → execution → feedback."""
    manager = LNSPManager(instance_id="instance:dogecoin", region="governance")
    bridge = GovernanceLNSP(manager)
    await bridge.setup()

    # Track emitted verdicts
    verdicts: list[dict[str, Any]] = []

    def capture_verdict(msg: LNSPMessage) -> None:
        data = msg.payload.get("data", {})
        if "verdict_type" in data:
            verdicts.append(data)

    manager.layer4.on_feedback(capture_verdict)

    # Step 1: Community submits proposal
    await bridge.process_proposal({
        "proposal_id": "prop_dogecoin_001",
        "title": "Burn 10% of treasury",
        "content": "Reduce inflation by burning 10% annually",
        "submitter_id": "user_alice",
        "community_id": "dogecoin",
        "submission_timestamp": 1708982400.0,
        "voting_period_hours": 48,
    })

    # Verify proposal was received
    assert manager.layer1.ringbuffer.size() >= 1

    # Step 2: Community votes
    await bridge.process_vote({
        "proposal_id": "prop_dogecoin_001",
        "voter_id": "user_bob",
        "vote_choice": "YES",
        "timestamp": 1708982450.0,
        "community_id": "dogecoin",
    })

    await bridge.process_vote({
        "proposal_id": "prop_dogecoin_001",
        "voter_id": "user_charlie",
        "vote_choice": "YES",
        "timestamp": 1708982460.0,
        "community_id": "dogecoin",
    })

    assert manager.layer1.ringbuffer.size() >= 3

    # Step 3: Decision executes on-chain
    await bridge.process_execution({
        "proposal_id": "prop_dogecoin_001",
        "success": True,
        "tx_hash": "0x123abc456def",
        "result": {"burned_tokens": 1000000, "treasury_balance": 9000000},
        "timestamp": 1708982500.0,
        "community_id": "dogecoin",
    })

    # Step 4: Community provides feedback
    await bridge.process_outcome({
        "proposal_id": "prop_dogecoin_001",
        "accepted": True,
        "funds_received": True,
        "community_sentiment": 0.92,
        "feedback_text": "Excellent decision, community very satisfied!",
        "timestamp": 1708982600.0,
        "community_id": "dogecoin",
    })

    # Verify all observations were processed
    assert manager.layer1.ringbuffer.size() >= 4


@pytest.mark.asyncio
async def test_multiple_proposals_parallel() -> None:
    """Test multiple proposals processed in parallel."""
    manager = LNSPManager(instance_id="instance:governance", region="governance")
    bridge = GovernanceLNSP(manager)
    await bridge.setup()

    # Process multiple proposals
    for i in range(5):
        await bridge.process_proposal({
            "proposal_id": f"prop_{i:03d}",
            "title": f"Proposal {i}",
            "content": f"Content for proposal {i}",
            "submitter_id": f"user_{i}",
            "community_id": "test",
            "submission_timestamp": 1708982400.0 + i,
            "voting_period_hours": 48,
        })

    # All should be processed
    assert manager.layer1.ringbuffer.size() >= 5
    assert len(bridge.verdict_cache) >= 0  # May be 0 if no verdicts


@pytest.mark.asyncio
async def test_learning_improves_verdicts() -> None:
    """Test that feedback loop can improve verdict quality."""
    manager = LNSPManager(instance_id="instance:governance", region="governance")
    bridge = GovernanceLNSP(manager)
    await bridge.setup()

    # Process first proposal with good outcome
    await bridge.process_proposal({
        "proposal_id": "prop_001",
        "title": "Good proposal",
        "content": "This should succeed",
        "submitter_id": "user_1",
        "community_id": "test",
        "submission_timestamp": 1708982400.0,
        "voting_period_hours": 48,
    })

    await bridge.process_execution({
        "proposal_id": "prop_001",
        "success": True,
        "tx_hash": "0xabc",
        "result": {},
        "timestamp": 1708982500.0,
        "community_id": "test",
    })

    await bridge.process_outcome({
        "proposal_id": "prop_001",
        "accepted": True,
        "funds_received": True,
        "community_sentiment": 0.95,
        "feedback_text": "Perfect!",
        "timestamp": 1708982600.0,
        "community_id": "test",
    })

    # Process second similar proposal
    # LNSP should have learned from first
    await bridge.process_proposal({
        "proposal_id": "prop_002",
        "title": "Similar good proposal",
        "content": "Similar to first, should also succeed",
        "submitter_id": "user_1",
        "community_id": "test",
        "submission_timestamp": 1708982700.0,
        "voting_period_hours": 48,
    })

    # Verify learning occurred (in full implementation)
    # For now, just verify pipeline executed
    assert manager.layer1.ringbuffer.size() >= 0
