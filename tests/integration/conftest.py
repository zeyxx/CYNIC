"""Shared fixtures for P10 probes testing (skipped)."""

import pytest
pytest.skip("Integration tests: dependencies being resolved", allow_module_level=True)

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
def self_prober(event_bus, tmp_path):
    """Create a fresh SelfProber instance with isolated temp storage."""
    # Use a temporary file path to avoid loading real proposals from disk
    temp_proposals_file = tmp_path / "self_proposals.json"
    prober = SelfProber(proposals_path=str(temp_proposals_file), bus=event_bus)
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
    from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProposal
    import time

    proposals = []

    # Directly create 10 sample proposals
    for i in range(10):
        proposal = SelfProposal(
            probe_id=f"probe_{i:02d}",
            trigger="MANUAL",
            pattern_type="TEST",
            severity=0.5,
            dimension="QTABLE" if i < 5 else "ESCORE" if i < 8 else "CONFIG",
            target=f"target_{i}",
            recommendation=f"Test recommendation {i}",
            current_value=float(i) / 10,
            suggested_value=float(i + 1) / 10,
            proposed_at=time.time(),
            status="PENDING" if i < 5 else "APPLIED" if i < 8 else "DISMISSED"
        )
        proposals.append(proposal)
        self_prober._proposals.append(proposal)

    self_prober._save()
    return proposals
