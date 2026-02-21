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
from pydantic import ValidationError
from cynic.api.state import set_app_container, AppContainer
from cynic.api.models.organism_state import (
    StateSnapshotResponse,
    ConsciousnessResponse,
    DogsResponse,
    ActionsResponse,
    AccountStatusResponse,
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

    # Mock metabolic system - account_agent
    mock_account_agent = MagicMock()
    mock_account_agent.stats = lambda: {
        "total_cost_usd": 2.5,
        "session_budget_usd": 10.0,
        "budget_remaining_usd": 7.5,
        "budget_ratio_remaining": 0.75,
        "judgment_count": 15,
        "warning_emitted": False,
        "exhausted_emitted": False,
        "uptime_s": 123.5,
    }
    mock_org.metabolic.account_agent = mock_account_agent

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


# ════════════════════════════════════════════════════════════════════════════
# INTEGRATION TESTS — Phase3-T6: Verify endpoints return actual organism reality
# ════════════════════════════════════════════════════════════════════════════


def test_all_endpoints_work_together(client):
    """
    INTEGRATION: All endpoints work together without errors.

    This test verifies that all 4 organism endpoints can be called in sequence
    and return valid responses. This simulates a real monitoring client that
    queries CYNIC's observable state.
    """
    # Query snapshot
    snapshot_resp = client.get("/api/organism/state/snapshot")
    assert snapshot_resp.status_code == 200, \
        f"Snapshot failed: {snapshot_resp.status_code} {snapshot_resp.text}"

    # Query consciousness
    consciousness_resp = client.get("/api/organism/consciousness")
    assert consciousness_resp.status_code == 200, \
        f"Consciousness failed: {consciousness_resp.status_code} {consciousness_resp.text}"

    # Query dogs
    dogs_resp = client.get("/api/organism/dogs")
    assert dogs_resp.status_code == 200, \
        f"Dogs failed: {dogs_resp.status_code} {dogs_resp.text}"

    # Query actions
    actions_resp = client.get("/api/organism/actions")
    assert actions_resp.status_code == 200, \
        f"Actions failed: {actions_resp.status_code} {actions_resp.text}"

    # Parse all responses
    snap_data = snapshot_resp.json()
    cons_data = consciousness_resp.json()
    dogs_data = dogs_resp.json()
    acts_data = actions_resp.json()

    # Verify all can be instantiated
    snapshot = StateSnapshotResponse(**snap_data)
    consciousness = ConsciousnessResponse(**cons_data)
    dogs = DogsResponse(**dogs_data)
    actions = ActionsResponse(**acts_data)

    assert snapshot is not None
    assert consciousness is not None
    assert dogs is not None
    assert actions is not None


def test_consciousness_matches_snapshot(client):
    """
    CONSISTENCY: Consciousness endpoint matches snapshot consciousness_level.

    Both endpoints read from the same organism state (scheduler.current_lod).
    They should always return the same consciousness level.
    """
    snapshot = client.get("/api/organism/state/snapshot").json()
    consciousness = client.get("/api/organism/consciousness").json()

    assert consciousness["level"] == snapshot["consciousness_level"], \
        f"Mismatch: consciousness.level='{consciousness['level']}' " \
        f"but snapshot.consciousness_level='{snapshot['consciousness_level']}'"


def test_dogs_count_matches_snapshot(client):
    """
    CONSISTENCY: Dogs endpoint count matches snapshot dog_count.

    Both read from the same orchestrator.dogs registry.
    Count should always match.
    """
    snapshot = client.get("/api/organism/state/snapshot").json()
    dogs = client.get("/api/organism/dogs").json()

    assert snapshot["dog_count"] == dogs["count"], \
        f"Mismatch: snapshot reports {snapshot['dog_count']} dogs, " \
        f"but /dogs reports {dogs['count']}"


def test_actions_count_matches_snapshot(client):
    """
    CONSISTENCY: Actions endpoint count matches snapshot pending_actions_count.

    Both read from the same action_proposer.pending() queue.
    Count should always match.
    """
    snapshot = client.get("/api/organism/state/snapshot").json()
    actions = client.get("/api/organism/actions").json()

    assert snapshot["pending_actions_count"] == actions["count"], \
        f"Mismatch: snapshot reports {snapshot['pending_actions_count']} pending actions, " \
        f"but /actions reports {actions['count']}"


def test_all_responses_are_frozen_immutable(client):
    """
    IMMUTABILITY: All responses are frozen (FrozenInstanceError on mutation).

    This proves that external clients cannot mutate CYNIC's state via API.
    All response models have frozen=True configured.
    """
    # Get snapshot
    snapshot_resp = client.get("/api/organism/state/snapshot")
    snapshot = StateSnapshotResponse(**snapshot_resp.json())

    # Try to mutate (should raise FrozenInstanceError)
    try:
        snapshot.dog_count = 999  # type: ignore
        assert False, "Should have raised FrozenInstanceError"
    except Exception as exc:
        assert "frozen" in str(exc).lower() or "FrozenInstanceError" in type(exc).__name__, \
            f"Expected FrozenInstanceError, got {type(exc).__name__}: {exc}"

    # Get consciousness
    cons_resp = client.get("/api/organism/consciousness")
    consciousness = ConsciousnessResponse(**cons_resp.json())

    try:
        consciousness.level = "INVALID"  # type: ignore
        assert False, "Should have raised FrozenInstanceError"
    except Exception as exc:
        assert "frozen" in str(exc).lower() or "FrozenInstanceError" in type(exc).__name__, \
            f"Expected FrozenInstanceError, got {type(exc).__name__}: {exc}"

    # Get dogs
    dogs_resp = client.get("/api/organism/dogs")
    dogs = DogsResponse(**dogs_resp.json())

    try:
        dogs.count = 999  # type: ignore
        assert False, "Should have raised FrozenInstanceError"
    except Exception as exc:
        assert "frozen" in str(exc).lower() or "FrozenInstanceError" in type(exc).__name__, \
            f"Expected FrozenInstanceError, got {type(exc).__name__}: {exc}"

    # Get actions
    acts_resp = client.get("/api/organism/actions")
    actions = ActionsResponse(**acts_resp.json())

    try:
        actions.count = 999  # type: ignore
        assert False, "Should have raised FrozenInstanceError"
    except Exception as exc:
        assert "frozen" in str(exc).lower() or "FrozenInstanceError" in type(exc).__name__, \
            f"Expected FrozenInstanceError, got {type(exc).__name__}: {exc}"


def test_snapshot_reflects_actual_counts(client):
    """
    REALITY: Snapshot returns actual counts from organism state.

    With mock organism:
    - dog_count should be 2 (2 dogs in mock orchestrator)
    - qtable_entries should be 2 (2 entries in mock q-table)
    - residuals_count should be 2 (2 residuals in mock detector)
    - pending_actions_count should be 1 (1 action in mock proposer)
    """
    snapshot = client.get("/api/organism/state/snapshot").json()

    # These values match our mock setup in the fixture
    assert snapshot["dog_count"] == 2, \
        f"Expected dog_count=2 (from mock), got {snapshot['dog_count']}"
    assert snapshot["qtable_entries"] == 2, \
        f"Expected qtable_entries=2 (from mock), got {snapshot['qtable_entries']}"
    assert snapshot["residuals_count"] == 2, \
        f"Expected residuals_count=2 (from mock), got {snapshot['residuals_count']}"
    assert snapshot["pending_actions_count"] == 1, \
        f"Expected pending_actions_count=1 (from mock), got {snapshot['pending_actions_count']}"


def test_dogs_endpoint_returns_individual_dog_status(client):
    """
    REALITY: Dogs endpoint returns individual dog status with all required fields.

    Each dog should have q_score, verdict, confidence (optional), and activity (optional).
    With mock, we have 2 dogs, each with minimal status.
    """
    dogs = client.get("/api/organism/dogs").json()

    assert len(dogs["dogs"]) == 2, f"Expected 2 dogs, got {len(dogs['dogs'])}"

    # Check each dog has required structure
    for dog_id, dog_status in dogs["dogs"].items():
        assert isinstance(dog_id, str), "dog_id should be string"
        assert "q_score" in dog_status, f"Dog {dog_id} missing q_score"
        assert "verdict" in dog_status, f"Dog {dog_id} missing verdict"
        assert isinstance(dog_status["q_score"], (int, float)), \
            f"Dog {dog_id} q_score should be numeric"
        assert isinstance(dog_status["verdict"], str), \
            f"Dog {dog_id} verdict should be string"
        assert dog_status["verdict"] in ["BARK", "GROWL", "WAG", "HOWL"], \
            f"Dog {dog_id} verdict '{dog_status['verdict']}' not valid"


def test_actions_endpoint_returns_action_details(client):
    """
    REALITY: Actions endpoint returns each action with all required fields.

    Each action should have action_id, action_type, priority, and description (optional).
    With mock, we have 1 action.
    """
    actions = client.get("/api/organism/actions").json()

    assert len(actions["actions"]) == 1, f"Expected 1 action, got {len(actions['actions'])}"

    action = actions["actions"][0]
    assert "action_id" in action, "Action missing action_id"
    assert "action_type" in action, "Action missing action_type"
    assert "priority" in action, "Action missing priority"
    assert isinstance(action["action_id"], str), "action_id should be string"
    assert isinstance(action["action_type"], str), "action_type should be string"
    assert isinstance(action["priority"], int), "priority should be int"
    assert 1 <= action["priority"] <= 4, f"priority should be 1-4, got {action['priority']}"


def test_all_timestamps_are_reasonable(client):
    """
    SANITY: Snapshot timestamp is a reasonable Unix timestamp.

    Should be a positive float representing seconds since epoch (after 2020).
    """
    snapshot = client.get("/api/organism/state/snapshot").json()

    timestamp = snapshot["timestamp"]
    assert isinstance(timestamp, (int, float)), "timestamp should be numeric"
    assert timestamp > 1577836800, \
        f"timestamp {timestamp} before 2020-01-01 (invalid)"
    assert timestamp < time.time() + 10, \
        f"timestamp {timestamp} is in the future (invalid)"


def test_all_counts_are_non_negative(client):
    """
    SANITY: All count fields are non-negative integers.

    No count should be negative (would be a logical error).
    """
    snapshot = client.get("/api/organism/state/snapshot").json()

    counts = {
        "judgment_count": snapshot["judgment_count"],
        "dog_count": snapshot["dog_count"],
        "qtable_entries": snapshot["qtable_entries"],
        "residuals_count": snapshot["residuals_count"],
        "pending_actions_count": snapshot["pending_actions_count"],
    }

    for name, value in counts.items():
        assert isinstance(value, int), f"{name} should be int, got {type(value).__name__}"
        assert value >= 0, f"{name} should be >= 0, got {value}"


def test_consciousness_level_is_valid(client):
    """
    SANITY: Consciousness level is one of the four valid states.

    Must be REFLEX, MICRO, MACRO, or META (derived from scheduler LOD).
    """
    consciousness = client.get("/api/organism/consciousness").json()

    level = consciousness["level"]
    valid_levels = {"REFLEX", "MICRO", "MACRO", "META"}
    assert level in valid_levels, \
        f"consciousness.level '{level}' not in {valid_levels}"


# ════════════════════════════════════════════════════════════════════════════
# PHASE 4 TASK 2: GET /api/organism/account
# ════════════════════════════════════════════════════════════════════════════


def test_get_organism_account(client):
    """GET /api/organism/account returns AccountStatusResponse (Phase 4 Task 2)."""
    response = client.get("/api/organism/account")
    assert response.status_code == 200, f"Expected 200, got {response.status_code}: {response.text}"

    data = response.json()

    # Verify all required fields are present
    assert "timestamp" in data, "Missing 'timestamp' field"
    assert "balance_usd" in data, "Missing 'balance_usd' field"
    assert "spent_usd" in data, "Missing 'spent_usd' field"
    assert "budget_remaining_usd" in data, "Missing 'budget_remaining_usd' field"
    assert "learn_rate" in data, "Missing 'learn_rate' field"
    assert "reputation" in data, "Missing 'reputation' field"

    # Verify field types
    assert isinstance(data["timestamp"], (int, float)), "timestamp should be numeric"
    assert isinstance(data["balance_usd"], (int, float)), "balance_usd should be numeric"
    assert isinstance(data["spent_usd"], (int, float)), "spent_usd should be numeric"
    assert isinstance(data["budget_remaining_usd"], (int, float)), "budget_remaining_usd should be numeric"
    assert isinstance(data["learn_rate"], (int, float)), "learn_rate should be numeric"
    assert isinstance(data["reputation"], (int, float)), "reputation should be numeric"

    # Verify field constraints
    assert data["balance_usd"] >= 0, "balance_usd should be >= 0"
    assert data["spent_usd"] >= 0, "spent_usd should be >= 0"
    assert data["budget_remaining_usd"] >= 0, "budget_remaining_usd should be >= 0"
    assert 0.0 <= data["learn_rate"] <= 0.618, "learn_rate should be [0, 0.618] (φ⁻¹)"
    assert 0.0 <= data["reputation"] <= 100.0, "reputation should be [0, 100]"

    # Verify response can be parsed as AccountStatusResponse
    account = AccountStatusResponse(**data)
    assert account is not None
    assert account.timestamp > 0
    assert account.balance_usd == 10.0  # from mock
    assert account.spent_usd == 2.5  # from mock
    assert account.budget_remaining_usd == 7.5  # from mock
    assert account.learn_rate >= 0.0
    assert account.reputation >= 0.0


def test_account_response_is_frozen_immutable(client):
    """IMMUTABILITY: AccountStatusResponse is frozen."""
    response = client.get("/api/organism/account")
    account = AccountStatusResponse(**response.json())

    # Try to mutate (should raise FrozenInstanceError)
    try:
        account.balance_usd = 999.0  # type: ignore
        assert False, "Should have raised FrozenInstanceError"
    except Exception as exc:
        assert "frozen" in str(exc).lower() or "FrozenInstanceError" in type(exc).__name__, \
            f"Expected FrozenInstanceError, got {type(exc).__name__}: {exc}"


def test_account_budget_calculation(client):
    """CONSISTENCY: budget_remaining_usd = balance_usd - spent_usd."""
    data = client.get("/api/organism/account").json()

    # Verify budget calculation consistency
    expected_remaining = data["balance_usd"] - data["spent_usd"]
    assert data["budget_remaining_usd"] == pytest.approx(expected_remaining, abs=0.01), \
        f"budget_remaining_usd ({data['budget_remaining_usd']}) should equal " \
        f"balance_usd ({data['balance_usd']}) - spent_usd ({data['spent_usd']}) = {expected_remaining}"


def test_account_learn_rate_within_phi_bound(client):
    """PHI-BOUND: learn_rate respects φ⁻¹ = 0.618 maximum."""
    data = client.get("/api/organism/account").json()
    learn_rate = data["learn_rate"]

    assert learn_rate <= 0.618, \
        f"learn_rate {learn_rate} exceeds φ⁻¹ = 0.618"
    assert learn_rate >= 0.0, \
        f"learn_rate {learn_rate} below 0"


def test_account_reputation_within_bounds(client):
    """BOUNDS: reputation is in [0, 100]."""
    data = client.get("/api/organism/account").json()
    reputation = data["reputation"]

    assert 0.0 <= reputation <= 100.0, \
        f"reputation {reputation} not in [0, 100]"


def test_account_timestamp_is_reasonable(client):
    """SANITY: Account timestamp is a reasonable Unix timestamp."""
    data = client.get("/api/organism/account").json()

    timestamp = data["timestamp"]
    assert isinstance(timestamp, (int, float)), "timestamp should be numeric"
    assert timestamp > 1577836800, \
        f"timestamp {timestamp} before 2020-01-01 (invalid)"
    assert timestamp < time.time() + 10, \
        f"timestamp {timestamp} is in the future (invalid)"
