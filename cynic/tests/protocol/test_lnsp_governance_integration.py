"""Test LNSP governance integration bridge."""
from __future__ import annotations

import pytest

from cynic.protocol.lnsp.governance_events import GovernanceProposalPayload
from cynic.protocol.lnsp.governance_integration import GovernanceLNSP
from cynic.protocol.lnsp.manager import LNSPManager


@pytest.mark.asyncio
async def test_governance_lnsp_initialization():
    """Test GovernanceLNSP initializes correctly."""
    manager = LNSPManager(instance_id="instance:governance", region="governance")
    bridge = GovernanceLNSP(manager)

    assert bridge.manager is manager
    assert bridge.manager.instance_id == "instance:governance"


@pytest.mark.asyncio
async def test_governance_lnsp_setup():
    """Test GovernanceLNSP setup initializes sensors and handlers."""
    manager = LNSPManager(instance_id="instance:governance", region="governance")
    bridge = GovernanceLNSP(manager)

    await bridge.setup()

    # Verify sensors registered
    assert "sensor:proposal" in {s.sensor_id for s in [bridge.proposal_sensor]}
    assert "sensor:vote" in {s.sensor_id for s in [bridge.vote_sensor]}

    # Verify handler registered
    assert "handler:governance" in manager.layer4.handlers


@pytest.mark.asyncio
async def test_governance_lnsp_process_proposal():
    """Test processing a governance proposal through LNSP."""
    manager = LNSPManager(instance_id="instance:governance", region="governance")
    bridge = GovernanceLNSP(manager)
    await bridge.setup()

    # Create proposal
    proposal = GovernanceProposalPayload(
        proposal_id="prop_001",
        title="Burn 10%",
        content="Reduce inflation by burning",
        submitter_id="user_123",
        community_id="dogecoin",
        submission_timestamp=1708982400.0,
        voting_period_hours=48,
    )

    # Add to sensor
    bridge.proposal_sensor.pending_payloads.append(proposal)

    # Process through LNSP
    await manager.run_cycle()

    # Verify observation was created
    obs = manager.layer1.ringbuffer.peek()
    assert obs is not None
    assert obs.payload["data"]["proposal_id"] == "prop_001"
