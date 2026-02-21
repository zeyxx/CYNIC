"""Tests for ConsciousState singleton.

Phase 1: Conscious State Extraction
- Verify state updates via events
- Verify thread safety
- Verify read-only interface
- Verify persistence
"""

import pytest
import asyncio
import json
from pathlib import Path
from unittest.mock import AsyncMock, MagicMock, patch

from cynic.organism.conscious_state import (
    ConsciousState,
    DogStatus,
    JudgmentSnapshot,
    AxiomStatus,
    get_conscious_state,
    STATE_FILE,
)
from cynic.core.event_bus import EventBus, Event


@pytest.fixture
def conscious_state():
    """Get fresh ConsciousState for each test."""
    # Reset singleton
    ConsciousState._instance = None
    state = ConsciousState()
    yield state
    # Cleanup
    ConsciousState._instance = None
    if STATE_FILE.exists():
        STATE_FILE.unlink()


@pytest.fixture
def mock_event_buses():
    """Create mock event buses."""
    return {
        "core": EventBus(bus_id="core"),
        "automation": EventBus(bus_id="automation"),
        "agent": EventBus(bus_id="agent"),
    }


def create_event(event_type: str, payload: dict) -> Event:
    """Helper to create Event objects."""
    return Event(type=event_type, payload=payload)


class TestConsciousStateInitialization:
    """Test singleton pattern and initialization."""

    def test_singleton_pattern(self):
        """ConsciousState should be a singleton."""
        ConsciousState._instance = None
        state1 = ConsciousState()
        state2 = ConsciousState()
        assert state1 is state2

    def test_initial_state(self, conscious_state):
        """Initial state should be REFLEX, empty dogs/axioms."""
        assert conscious_state._consciousness_level == "REFLEX"
        assert len(conscious_state._dogs) == 0
        assert len(conscious_state._axioms) == 0
        assert conscious_state._judgment_count == 0

    @pytest.mark.asyncio
    async def test_initialize_from_buses(self, conscious_state, mock_event_buses):
        """Should subscribe to event buses."""
        await conscious_state.initialize_from_buses(
            mock_event_buses["core"],
            mock_event_buses["automation"],
            mock_event_buses["agent"],
        )

        # Verify subscription by firing an event
        event = create_event(
            "JUDGMENT_CREATED",
            {
                "judgment_id": "j1",
                "q_score": 75.0,
                "verdict": "WAG",
                "confidence": 0.5,
                "dog_votes": {"dog1": 75.0},
                "source": "test",
            },
        )
        await mock_event_buses["core"].emit(event)
        await asyncio.sleep(0.1)
        health = await conscious_state.get_health()
        assert health["judgment_count"] == 1


class TestConsciousStateJudgments:
    """Test judgment tracking."""

    @pytest.mark.asyncio
    async def test_record_judgment(self, conscious_state, mock_event_buses):
        """Record a single judgment."""
        await conscious_state.initialize_from_buses(mock_event_buses["core"])

        event = create_event(
            "JUDGMENT_CREATED",
            {
                "judgment_id": "j1",
                "q_score": 85.0,
                "verdict": "HOWL",
                "confidence": 0.6,
                "dog_votes": {"dog_analyst": 85.0},
                "source": "perceive",
            },
        )
        await mock_event_buses["core"].emit(event)
        await asyncio.sleep(0.1)

        judgments = await conscious_state.get_recent_judgments(limit=10)
        assert len(judgments) == 1
        assert judgments[0].q_score == 85.0
        assert judgments[0].verdict == "HOWL"

    @pytest.mark.asyncio
    async def test_rolling_cap_f11(self, conscious_state, mock_event_buses):
        """Recent judgments should cap at F(11)=89."""
        await conscious_state.initialize_from_buses(mock_event_buses["core"])

        # Create 100 judgments
        for i in range(100):
            event = create_event(
                "JUDGMENT_CREATED",
                {
                    "judgment_id": f"j{i}",
                    "q_score": 50.0 + i,
                    "verdict": "WAG",
                    "confidence": 0.5,
                    "dog_votes": {},
                    "source": "test",
                },
            )
            await mock_event_buses["core"].emit(event)
            await asyncio.sleep(0.01)

        # Should only have 89 most recent
        judgments = await conscious_state.get_recent_judgments(limit=200)
        assert len(judgments) == 89

        # Should be in reverse chronological order (newest first)
        assert judgments[0].q_score == 99 + 50.0  # Last judgment
        assert judgments[-1].q_score == 11 + 50.0  # First judgment in rolling cap

    @pytest.mark.asyncio
    async def test_judgment_ordering(self, conscious_state, mock_event_buses):
        """Recent judgments should be newest-first."""
        await conscious_state.initialize_from_buses(mock_event_buses["core"])

        for i in range(5):
            event = create_event(
                "JUDGMENT_CREATED",
                {
                    "judgment_id": f"j{i}",
                    "q_score": 50.0 + i,
                    "verdict": "WAG",
                    "confidence": 0.5,
                    "dog_votes": {},
                    "source": "test",
                },
            )
            await mock_event_buses["core"].emit(event)
            await asyncio.sleep(0.01)

        judgments = await conscious_state.get_recent_judgments(limit=10)
        # Should be [j4, j3, j2, j1, j0]
        assert judgments[0].judgment_id == "j4"
        assert judgments[4].judgment_id == "j0"


class TestConsciousStateDogs:
    """Test dog status tracking."""

    @pytest.mark.asyncio
    async def test_dog_activity_update(self, conscious_state, mock_event_buses):
        """Track dog activity changes."""
        await conscious_state.initialize_from_buses(mock_event_buses["core"])

        event = create_event(
            "DOG_ACTIVITY",
            {
                "dog_id": "analyst",
                "activity": "judging",
                "q_score": 75.0,
                "verdict": "WAG",
                "confidence": 0.6,
            },
        )
        await mock_event_buses["core"].emit(event)
        await asyncio.sleep(0.05)

        dog = await conscious_state.get_dog("analyst")
        assert dog is not None
        assert dog["dog_id"] == "analyst"
        assert dog["activity"] == "judging"
        assert dog["judgment_count"] == 1

    @pytest.mark.asyncio
    async def test_get_all_dogs(self, conscious_state, mock_event_buses):
        """Get status of all dogs."""
        await conscious_state.initialize_from_buses(mock_event_buses["core"])

        # Create multiple dog activities
        for i, dog_name in enumerate(
            ["analyst", "architect", "guardian", "oracle"]
        ):
            event = create_event(
                "DOG_ACTIVITY",
                {
                    "dog_id": dog_name,
                    "activity": "idle",
                    "q_score": 50.0 + i * 10,
                    "verdict": "WAG",
                    "confidence": 0.5,
                },
            )
            await mock_event_buses["core"].emit(event)
            await asyncio.sleep(0.01)

        dogs = await conscious_state.get_dogs()
        assert len(dogs) == 4
        assert "analyst" in dogs
        assert dogs["analyst"].q_score == 50.0


class TestConsciousStateAxioms:
    """Test axiom activation tracking."""

    @pytest.mark.asyncio
    async def test_axiom_activation(self, conscious_state, mock_event_buses):
        """Track axiom activation signals."""
        await conscious_state.initialize_from_buses(mock_event_buses["core"])

        # Send axiom signals (need 3+ to activate)
        for i in range(3):
            event = create_event(
                "AXIOM_ACTIVATED",
                {
                    "axiom_id": "PHI",
                    "tier": "A6",
                },
            )
            await mock_event_buses["core"].emit(event)
            await asyncio.sleep(0.01)

        axiom = await conscious_state.get_axiom("PHI")
        assert axiom is not None
        assert axiom.axiom_id == "PHI"
        assert axiom.active is True
        assert axiom.signal_count == 3
        assert axiom.activated_at is not None

    @pytest.mark.asyncio
    async def test_multiple_axioms(self, conscious_state, mock_event_buses):
        """Track multiple axioms independently."""
        await conscious_state.initialize_from_buses(mock_event_buses["core"])

        for axiom_name in ["PHI", "VERIFY", "CULTURE"]:
            for i in range(3):
                event = create_event(
                    "AXIOM_ACTIVATED",
                    {
                        "axiom_id": axiom_name,
                        "tier": f"A{6 + i}",
                    },
                )
                await mock_event_buses["core"].emit(event)
                await asyncio.sleep(0.01)

        axioms = await conscious_state.get_all_axioms()
        assert len(axioms) == 3
        assert all(axiom.active for axiom in axioms.values())


class TestConsciousStateConsciousnessLevel:
    """Test consciousness tier changes."""

    @pytest.mark.asyncio
    async def test_consciousness_level_change(self, conscious_state, mock_event_buses):
        """Track consciousness level transitions."""
        await conscious_state.initialize_from_buses(mock_event_buses["core"])

        # Verify initial state
        assert await conscious_state.get_current_level() == "REFLEX"

        # Transition to MICRO
        event = create_event("CONSCIOUSNESS_LEVEL_CHANGED", {"level": "MICRO"})
        await mock_event_buses["core"].emit(event)
        await asyncio.sleep(0.05)

        assert await conscious_state.get_current_level() == "MICRO"

        # Transition to MACRO
        event = create_event("CONSCIOUSNESS_LEVEL_CHANGED", {"level": "MACRO"})
        await mock_event_buses["core"].emit(event)
        await asyncio.sleep(0.05)

        assert await conscious_state.get_current_level() == "MACRO"


class TestConsciousStateHealth:
    """Test health metrics."""

    @pytest.mark.asyncio
    async def test_health_metrics(self, conscious_state, mock_event_buses):
        """Health should report current state."""
        await conscious_state.initialize_from_buses(mock_event_buses["core"])

        # Add some activity
        for i in range(5):
            event = create_event(
                "JUDGMENT_CREATED",
                {
                    "judgment_id": f"j{i}",
                    "q_score": 50.0,
                    "verdict": "WAG",
                    "confidence": 0.5,
                    "dog_votes": {},
                    "source": "test",
                },
            )
            await mock_event_buses["core"].emit(event)
            await asyncio.sleep(0.01)

        health = await conscious_state.get_health()
        assert health["consciousness_level"] == "REFLEX"
        assert health["judgment_count"] == 5
        assert health["dog_count"] == 0
        assert "timestamp" in health

    @pytest.mark.asyncio
    async def test_error_tracking(self, conscious_state, mock_event_buses):
        """Errors should be tracked."""
        await conscious_state.initialize_from_buses(mock_event_buses["core"])

        event = create_event("ERROR", {"message": "Test error 1"})
        await mock_event_buses["core"].emit(event)
        await asyncio.sleep(0.05)

        event = create_event("ERROR", {"message": "Test error 2"})
        await mock_event_buses["core"].emit(event)
        await asyncio.sleep(0.05)

        health = await conscious_state.get_health()
        assert health["error_count"] == 2


class TestConsciousStatePersistence:
    """Test state persistence to disk."""

    @pytest.mark.asyncio
    async def test_save_to_disk(self, conscious_state, mock_event_buses):
        """State should be persistable to disk."""
        await conscious_state.initialize_from_buses(mock_event_buses["core"])

        # Add some state
        event = create_event(
            "JUDGMENT_CREATED",
            {
                "judgment_id": "j1",
                "q_score": 75.0,
                "verdict": "WAG",
                "confidence": 0.5,
                "dog_votes": {"dog1": 75.0},
                "source": "test",
            },
        )
        await mock_event_buses["core"].emit(event)
        await asyncio.sleep(0.05)

        # Save to disk
        await conscious_state.save_to_disk()

        # Verify file exists
        assert STATE_FILE.exists()

        # Load and check content
        data = json.loads(STATE_FILE.read_text())
        assert data["stats"]["judgment_count"] == 1

    @pytest.mark.asyncio
    async def test_load_from_disk(self, conscious_state, mock_event_buses):
        """State should be recoverable from disk."""
        # Create state file manually
        state_data = {
            "consciousness_level": "MACRO",
            "dogs": {},
            "recent_judgments": [],
            "axioms": {},
            "stats": {
                "judgment_count": 42,
                "axiom_activation_count": 3,
                "error_count": 1,
                "last_update": 1000.0,
            },
        }
        STATE_FILE.write_text(json.dumps(state_data))

        # Load from disk
        await conscious_state.load_from_disk()

        # Verify loaded state
        assert conscious_state._consciousness_level == "MACRO"
        assert conscious_state._judgment_count == 42
        assert conscious_state._axiom_activation_count == 3

        health = await conscious_state.get_health()
        assert health["consciousness_level"] == "MACRO"
        assert health["judgment_count"] == 42


class TestConsciousStateReadOnly:
    """Test that interface is read-only."""

    @pytest.mark.asyncio
    async def test_get_dogs_returns_copy(self, conscious_state, mock_event_buses):
        """get_dogs() should return copy, not internal dict."""
        await conscious_state.initialize_from_buses(mock_event_buses["core"])

        event = create_event(
            "DOG_ACTIVITY",
            {
                "dog_id": "analyst",
                "activity": "idle",
                "q_score": 75.0,
                "verdict": "WAG",
                "confidence": 0.5,
            },
        )
        await mock_event_buses["core"].emit(event)
        await asyncio.sleep(0.05)

        dogs1 = await conscious_state.get_dogs()
        dogs2 = await conscious_state.get_dogs()

        # Should be equal but different objects
        assert dogs1 == dogs2
        assert dogs1 is not dogs2

    @pytest.mark.asyncio
    async def test_get_axioms_returns_copy(self, conscious_state, mock_event_buses):
        """get_all_axioms() should return copy."""
        await conscious_state.initialize_from_buses(mock_event_buses["core"])

        for i in range(3):
            event = create_event(
                "AXIOM_ACTIVATED",
                {"axiom_id": "PHI", "tier": "A6"},
            )
            await mock_event_buses["core"].emit(event)
            await asyncio.sleep(0.01)

        axioms1 = await conscious_state.get_all_axioms()
        axioms2 = await conscious_state.get_all_axioms()

        assert axioms1 == axioms2
        assert axioms1 is not axioms2


class TestGetConsciousStateSingleton:
    """Test singleton getter function."""

    def test_get_conscious_state(self):
        """get_conscious_state() should return singleton."""
        ConsciousState._instance = None
        state1 = get_conscious_state()
        state2 = get_conscious_state()
        assert state1 is state2
