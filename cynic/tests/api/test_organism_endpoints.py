"""
Tests for organism state API endpoints.

Phase3-T2: GET /api/organism/state/snapshot
Phase3-T3/T4/T5: GET /api/organism/consciousness, /dogs, /actions
"""
from __future__ import annotations

import pytest
from fastapi.testclient import TestClient
from unittest.mock import Mock, MagicMock, patch
import time

from cynic.api.server import app
from cynic.api.state import set_app_container, AppContainer
from cynic.api.models.organism_state import (
    StateSnapshotResponse,
    ConsciousnessResponse,
    DogsResponse,
    ActionsResponse,
)


@pytest.fixture
def mock_organism():
    """Create a mock organism with minimal required attributes."""
    mock_org = MagicMock()

    # Mock metabolic system (for consciousness level)
    mock_org.metabolic = MagicMock()
    mock_org.metabolic.scheduler = MagicMock()
    mock_org.metabolic.scheduler.current_lod = 1  # MICRO

    # Mock cognition system
    mock_org.cognition = MagicMock()
    mock_org.cognition.orchestrator = MagicMock()
    mock_org.cognition.orchestrator.dogs = {"dog1": MagicMock(), "dog2": MagicMock()}

    mock_org.cognition.qtable = MagicMock()
    mock_org.cognition.qtable._q_table = {
        "state1": {"WAG": 0.5},
        "state2": {"GROWL": 0.3},
    }

    mock_org.cognition.residual_detector = MagicMock()
    mock_org.cognition.residual_detector._residuals = [
        {"score": 0.1},
        {"score": 0.2},
    ]

    # Mock memory system
    mock_org.memory = MagicMock()
    mock_org.memory.action_proposer = MagicMock()
    mock_org.memory.action_proposer.pending = lambda: [
        {"action_id": "action1"},
    ]

    return mock_org


@pytest.fixture
def client(mock_organism):
    """FastAPI test client with mocked organism."""
    # Create mock container
    container = AppContainer(
        organism=mock_organism,
        instance_id="test-12345",
        guidance_path="/tmp/guidance.json",
        started_at=time.time(),
    )

    # Set the container in the app state
    set_app_container(container)

    # Return test client
    return TestClient(app)


def test_get_organism_state_snapshot(client):
    """GET /api/organism/state/snapshot returns StateSnapshotResponse."""
    response = client.get("/api/organism/state/snapshot")
    assert response.status_code == 200, f"Expected 200, got {response.status_code}: {response.text}"

    data = response.json()

    # Verify all required fields are present
    assert "timestamp" in data, "Missing 'timestamp' field"
    assert "consciousness_level" in data, "Missing 'consciousness_level' field"
    assert "judgment_count" in data, "Missing 'judgment_count' field"
    assert "dog_count" in data, "Missing 'dog_count' field"
    assert "qtable_entries" in data, "Missing 'qtable_entries' field"
    assert "residuals_count" in data, "Missing 'residuals_count' field"
    assert "pending_actions_count" in data, "Missing 'pending_actions_count' field"

    # Verify field types
    assert isinstance(data["timestamp"], (int, float)), "timestamp should be numeric"
    assert isinstance(data["consciousness_level"], str), "consciousness_level should be string"
    assert isinstance(data["judgment_count"], int), "judgment_count should be int"
    assert isinstance(data["dog_count"], int), "dog_count should be int"
    assert isinstance(data["qtable_entries"], int), "qtable_entries should be int"
    assert isinstance(data["residuals_count"], int), "residuals_count should be int"
    assert isinstance(data["pending_actions_count"], int), "pending_actions_count should be int"

    # Verify consciousness_level is one of valid values
    assert data["consciousness_level"] in ["REFLEX", "MICRO", "MACRO", "META"], \
        f"consciousness_level '{data['consciousness_level']}' not in valid levels"

    # Verify counts are non-negative
    assert data["judgment_count"] >= 0, "judgment_count should be >= 0"
    assert data["dog_count"] >= 0, "dog_count should be >= 0"
    assert data["qtable_entries"] >= 0, "qtable_entries should be >= 0"
    assert data["residuals_count"] >= 0, "residuals_count should be >= 0"
    assert data["pending_actions_count"] >= 0, "pending_actions_count should be >= 0"

    # Verify response can be parsed as StateSnapshotResponse
    snapshot = StateSnapshotResponse(**data)
    assert snapshot is not None
    assert snapshot.timestamp > 0
    assert snapshot.dog_count == 2  # We created 2 dogs in mock
    assert snapshot.qtable_entries == 2  # We created 2 q-table entries
    assert snapshot.residuals_count == 2  # We created 2 residuals
    assert snapshot.pending_actions_count == 1  # We created 1 pending action


def test_get_organism_consciousness(client):
    """GET /api/organism/consciousness returns consciousness level."""
    response = client.get("/api/organism/consciousness")
    assert response.status_code == 200, f"Expected 200, got {response.status_code}: {response.text}"

    data = response.json()

    # Verify required field
    assert "level" in data, "Missing 'level' field"

    # Verify field type
    assert isinstance(data["level"], str), "level should be string"

    # Verify consciousness_level is one of valid values
    assert data["level"] in ["REFLEX", "MICRO", "MACRO", "META"], \
        f"level '{data['level']}' not in valid levels"

    # Verify response can be parsed as ConsciousnessResponse
    consciousness = ConsciousnessResponse(**data)
    assert consciousness is not None
    assert consciousness.level == "MICRO"  # mock has scheduler.current_lod=1


def test_get_organism_dogs(client):
    """GET /api/organism/dogs returns all dogs."""
    response = client.get("/api/organism/dogs")
    assert response.status_code == 200, f"Expected 200, got {response.status_code}: {response.text}"

    data = response.json()

    # Verify required fields
    assert "dogs" in data, "Missing 'dogs' field"
    assert "count" in data, "Missing 'count' field"

    # Verify field types
    assert isinstance(data["dogs"], dict), "dogs should be dict"
    assert isinstance(data["count"], int), "count should be int"

    # Verify count matches dogs dict
    assert data["count"] == len(data["dogs"]), "count should match number of dogs"

    # Verify response can be parsed as DogsResponse
    dogs_resp = DogsResponse(**data)
    assert dogs_resp is not None
    assert dogs_resp.count == 2  # mock has 2 dogs


def test_get_organism_actions(client):
    """GET /api/organism/actions returns pending actions."""
    response = client.get("/api/organism/actions")
    assert response.status_code == 200, f"Expected 200, got {response.status_code}: {response.text}"

    data = response.json()

    # Verify required fields
    assert "actions" in data, "Missing 'actions' field"
    assert "count" in data, "Missing 'count' field"

    # Verify field types
    assert isinstance(data["actions"], list), "actions should be list"
    assert isinstance(data["count"], int), "count should be int"

    # Verify count matches actions list
    assert data["count"] == len(data["actions"]), "count should match number of actions"

    # Verify response can be parsed as ActionsResponse
    actions_resp = ActionsResponse(**data)
    assert actions_resp is not None
    assert actions_resp.count == 1  # mock has 1 action
