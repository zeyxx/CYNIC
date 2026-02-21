"""
Test Pydantic response models for organism state API endpoints.

TDD approach: Tests written first, models implemented after.
All models MUST be frozen=True for immutability.
"""

import pytest
from datetime import datetime
from pydantic import ValidationError


def test_state_snapshot_response_model():
    """Test StateSnapshotResponse model creation and validation."""
    from cynic.api.models.organism_state import StateSnapshotResponse

    # Create a snapshot
    snapshot = StateSnapshotResponse(
        timestamp=datetime.now().timestamp(),
        consciousness_level="MICRO",
        judgment_count=42,
        dog_count=11,
        qtable_entries=256,
        residuals_count=3,
        pending_actions_count=2,
    )

    # Verify fields
    assert isinstance(snapshot.timestamp, float)
    assert snapshot.consciousness_level == "MICRO"
    assert snapshot.judgment_count == 42
    assert snapshot.dog_count == 11
    assert snapshot.qtable_entries == 256
    assert snapshot.residuals_count == 3
    assert snapshot.pending_actions_count == 2

    # Verify frozen (immutable) - Pydantic v2 raises ValidationError
    with pytest.raises(ValidationError):
        snapshot.judgment_count = 50


def test_consciousness_response_model():
    """Test ConsciousnessResponse model creation."""
    from cynic.api.models.organism_state import ConsciousnessResponse

    consciousness = ConsciousnessResponse(level="MACRO")

    assert consciousness.level == "MACRO"

    # Verify frozen - Pydantic v2 raises ValidationError
    with pytest.raises(ValidationError):
        consciousness.level = "META"


def test_dogs_response_model():
    """Test DogsResponse model with nested DogStatus."""
    from cynic.api.models.organism_state import DogsResponse, DogStatus

    # Create individual dog statuses
    guardian = DogStatus(
        q_score=0.78,
        verdict="HOWL",
        confidence=0.55,
        activity="judging",
    )

    analyst = DogStatus(
        q_score=0.64,
        verdict="WAG",
        confidence=0.42,
        activity="idle",
    )

    # Create response with dogs
    dogs_response = DogsResponse(
        dogs={
            "guardian": guardian,
            "analyst": analyst,
        },
        count=2,
    )

    # Verify structure
    assert dogs_response.count == 2
    assert "guardian" in dogs_response.dogs
    assert "analyst" in dogs_response.dogs
    assert dogs_response.dogs["guardian"].verdict == "HOWL"
    assert dogs_response.dogs["analyst"].q_score == 0.64

    # Verify frozen - Pydantic v2 raises ValidationError
    with pytest.raises(ValidationError):
        dogs_response.count = 5


def test_actions_response_model():
    """Test ActionsResponse model with nested ProposedAction."""
    from cynic.api.models.organism_state import ActionsResponse, ProposedAction

    # Create actions
    action1 = ProposedAction(
        action_id="ACT-001",
        action_type="INVESTIGATE",
        priority=1,
        description="Critical issue detected in parser",
    )

    action2 = ProposedAction(
        action_id="ACT-002",
        action_type="MONITOR",
        priority=3,
        description=None,  # Optional
    )

    # Create response
    actions_response = ActionsResponse(
        actions=[action1, action2],
        count=2,
    )

    # Verify structure
    assert actions_response.count == 2
    assert len(actions_response.actions) == 2
    assert actions_response.actions[0].action_type == "INVESTIGATE"
    assert actions_response.actions[0].priority == 1
    assert actions_response.actions[1].description is None

    # Verify frozen - Pydantic v2 raises ValidationError
    with pytest.raises(ValidationError):
        actions_response.count = 10


def test_account_status_response_model():
    """Test AccountStatusResponse model with account metrics."""
    from cynic.api.models.organism_state import AccountStatusResponse

    # Create account status
    account = AccountStatusResponse(
        timestamp=datetime.now().timestamp(),
        balance_usd=10.0,
        spent_usd=2.5,
        budget_remaining_usd=7.5,
        learn_rate=0.618,
        reputation=78.5,
    )

    # Verify fields
    assert isinstance(account.timestamp, float)
    assert account.balance_usd == 10.0
    assert account.spent_usd == 2.5
    assert account.budget_remaining_usd == 7.5
    assert account.learn_rate == 0.618
    assert account.reputation == 78.5

    # Verify frozen - Pydantic v2 raises ValidationError
    with pytest.raises(ValidationError):
        account.balance_usd = 20.0


def test_escore_response_model():
    """Test EScoreResponse model with per-agent reputation scores."""
    from cynic.api.models.organism_state import EScoreResponse, AgentScore

    # Create per-agent scores
    sage_score = AgentScore(
        agent_id="SAGE",
        burn=75.0,
        build=82.0,
        judge=88.0,
        run=90.0,
        social=65.0,
        graph=70.0,
        hold=60.0,
        total=78.5,
    )

    guardian_score = AgentScore(
        agent_id="GUARDIAN",
        burn=70.0,
        build=75.0,
        judge=85.0,
        run=92.0,
        social=72.0,
        graph=68.0,
        hold=62.0,
        total=76.2,
    )

    # Create E-Score response
    escore_response = EScoreResponse(
        timestamp=datetime.now().timestamp(),
        agents=[sage_score, guardian_score],
        count=2,
    )

    # Verify structure
    assert escore_response.count == 2
    assert len(escore_response.agents) == 2
    assert escore_response.agents[0].agent_id == "SAGE"
    assert escore_response.agents[0].total == 78.5
    assert escore_response.agents[1].agent_id == "GUARDIAN"
    assert escore_response.agents[1].judge == 85.0

    # Verify frozen - Pydantic v2 raises ValidationError
    with pytest.raises(ValidationError):
        escore_response.count = 5
