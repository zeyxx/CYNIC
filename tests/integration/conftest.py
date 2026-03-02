"""Shared fixtures for P10 probes testing."""

import pytest
from unittest.mock import MagicMock, AsyncMock
from cynic.kernel.core.event_bus import EventBus
from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
from cynic.kernel.organism.brain.cognition.cortex.proposal_executor import ProposalExecutor


@pytest.fixture
def event_bus():
    """Create a fresh EventBus for each test."""
    return EventBus(bus_id="test_bus", instance_id="test_instance")


@pytest.fixture
def self_prober(event_bus):
    """Create a fresh SelfProber instance."""
    prober = SelfProber(bus=event_bus)
    # Load existing proposals from disk
    return prober


@pytest.fixture
def mock_executor(event_bus):
    """Mock ProposalExecutor for testing."""
    executor = MagicMock(spec=ProposalExecutor)

    # Classify all proposals as LOW_RISK for testing
    async def mock_execute(proposal):
        result = MagicMock()
        result.proposal_id = proposal.probe_id
        result.dimension = proposal.dimension
        result.success = True
        result.message = "Executed"
        result.error_message = None
        result.old_value = proposal.current_value
        result.new_value = proposal.suggested_value
        return result

    executor.classify_risk = MagicMock(return_value="LOW_RISK")
    executor.execute = AsyncMock(side_effect=mock_execute)

    return executor


@pytest.fixture
def prober_with_executor(self_prober, mock_executor):
    """SelfProber with mock executor injected."""
    self_prober.set_executor(mock_executor)
    return self_prober


@pytest.fixture
def sample_proposals(self_prober):
    """Generate 10 sample proposals with mixed statuses."""
    proposals = []

    # Generate 10 proposals
    for i in range(10):
        new_proposals = self_prober.analyze(
            trigger="MANUAL",
            pattern_type="TEST",
            severity=0.5
        )
        if new_proposals:
            proposals.extend(new_proposals)

    # Assign statuses
    for i, proposal in enumerate(proposals[:10]):
        if i < 5:
            proposal.status = "PENDING"
        elif i < 8:
            proposal.status = "APPLIED"
        else:
            proposal.status = "DISMISSED"
        self_prober._save()

    return proposals[:10]
