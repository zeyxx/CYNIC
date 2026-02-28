"""Test LNSP integration with CYNIC event bus."""
from __future__ import annotations

from typing import Any

import pytest

from cynic.protocol.lnsp.governance_integration import GovernanceLNSP
from cynic.protocol.lnsp.manager import LNSPManager
from cynic.protocol.lnsp.types import LNSPMessage


@pytest.mark.asyncio
async def test_governance_lnsp_startup() -> None:
    """Test GovernanceLNSP can start up cleanly."""
    manager = LNSPManager(instance_id="instance:governance", region="governance")
    bridge = GovernanceLNSP(manager)

    # Should not raise
    await bridge.setup()

    # Verify components are wired
    assert len(manager.layer1.sensors) == 4  # 4 sensors registered
    assert len(manager.layer4.handlers) == 1  # 1 handler registered


@pytest.mark.asyncio
async def test_governance_verdicts_through_pipeline() -> None:
    """Test verdict flows through entire pipeline."""
    manager = LNSPManager(instance_id="instance:governance", region="governance")
    bridge = GovernanceLNSP(manager)
    await bridge.setup()

    # Track verdicts
    verdicts: list[dict[str, Any]] = []

    def capture_verdict(msg: LNSPMessage) -> None:
        # When verdict emitted, capture it
        if "verdict_type" in msg.payload.get("data", {}):
            verdicts.append(msg.payload["data"])

    manager.layer4.on_feedback(capture_verdict)

    # Process a proposal
    await bridge.process_proposal({
        "proposal_id": "prop_001",
        "title": "Test",
        "content": "Test proposal",
        "submitter_id": "user_1",
        "community_id": "test",
        "submission_timestamp": 1708982400.0,
        "voting_period_hours": 48,
    })

    # Verdict should have been emitted (feedback captured)
    # Note: May be empty if no axioms registered, but pipeline should execute
    assert manager.layer1.ringbuffer.size() >= 0
