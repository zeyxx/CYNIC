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
