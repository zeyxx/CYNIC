"""Test governance sensors for LNSP integration."""

from __future__ import annotations

import pytest

from cynic.kernel.protocol.lnsp.governance_events import (
    GovernanceExecutionPayload,
    GovernanceOutcomePayload,
    GovernanceProposalPayload,
    GovernanceVotePayload,
)
from cynic.kernel.protocol.lnsp.governance_sensors import (
    ExecutionSensor,
    OutcomeSensor,
    ProposalSensor,
    VoteSensor,
)
from cynic.kernel.protocol.lnsp.types import Layer, ObservationType


@pytest.mark.asyncio
async def test_proposal_sensor_observe():
    """Test ProposalSensor converts proposal to observation."""
    sensor = ProposalSensor("sensor:proposal")
    payload = GovernanceProposalPayload(
        proposal_id="prop_001",
        title="Burn 10%",
        content="Reduce inflation",
        submitter_id="user_123",
        community_id="dogecoin",
        submission_timestamp=1708982400.0,
        voting_period_hours=48,
    )
    sensor.pending_payloads.append(payload)

    obs = await sensor.observe()
    assert obs is not None
    assert obs.header.layer == Layer.RAW
    assert obs.payload["observation_type"] == ObservationType.HUMAN_INPUT.value
    assert obs.payload["data"]["proposal_id"] == "prop_001"


@pytest.mark.asyncio
async def test_vote_sensor_observe():
    """Test VoteSensor converts vote to observation."""
    sensor = VoteSensor("sensor:vote")
    payload = GovernanceVotePayload(
        proposal_id="prop_001",
        voter_id="user_456",
        vote_choice="YES",
        timestamp=1708982500.0,
        community_id="dogecoin",
    )
    sensor.pending_payloads.append(payload)

    obs = await sensor.observe()
    assert obs is not None
    assert obs.header.layer == Layer.RAW
    assert obs.payload["observation_type"] == ObservationType.HUMAN_INPUT.value


@pytest.mark.asyncio
async def test_execution_sensor_observe():
    """Test ExecutionSensor converts execution to observation."""
    sensor = ExecutionSensor("sensor:execution")
    payload = GovernanceExecutionPayload(
        proposal_id="prop_001",
        success=True,
        tx_hash="0xabc",
        result={"burned": 1000000},
        timestamp=1708982600.0,
        community_id="dogecoin",
    )
    sensor.pending_payloads.append(payload)

    obs = await sensor.observe()
    assert obs is not None
    assert obs.header.layer == Layer.RAW
    assert obs.payload["observation_type"] == ObservationType.ACTION_RESULT.value


@pytest.mark.asyncio
async def test_outcome_sensor_observe():
    """Test OutcomeSensor converts outcome to observation."""
    sensor = OutcomeSensor("sensor:outcome")
    payload = GovernanceOutcomePayload(
        proposal_id="prop_001",
        accepted=True,
        funds_received=True,
        community_sentiment=0.89,
        feedback_text="Great!",
        timestamp=1708982700.0,
        community_id="dogecoin",
    )
    sensor.pending_payloads.append(payload)

    obs = await sensor.observe()
    assert obs is not None
    assert obs.payload["observation_type"] == ObservationType.ECOSYSTEM_EVENT.value


@pytest.mark.asyncio
async def test_sensors_return_none_when_empty():
    """Test sensors return None when no payloads."""
    sensor = ProposalSensor("sensor:proposal")
    obs = await sensor.observe()
    assert obs is None
