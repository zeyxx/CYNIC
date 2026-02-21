"""
Tests for OrganismState Tasks 4-7: Consciousness, Dogs, Residuals, Actions

Tasks:
  4. Migrate conscious_state to OrganismState
  5. Migrate dogs registry to OrganismState
  6. Migrate residuals to OrganismState
  7. Migrate actions queue to OrganismState

Each task includes multiple test methods following TDD pattern.
"""

import pytest
import asyncio
from cynic.organism.state_manager import OrganismState, StateLayer


@pytest.fixture
def organism_state():
    """Create fresh OrganismState for each test."""
    return OrganismState()


class TestTask4ConsciousnessSubsystem:
    """Task 4: Migrate conscious_state to OrganismState."""

    @pytest.mark.asyncio
    async def test_add_and_get_judgments(self, organism_state):
        """Should add judgment and retrieve it."""
        judgment = {
            "judgment_id": "j1",
            "q_score": 75.0,
            "verdict": "WAG",
            "confidence": 0.5,
        }

        await organism_state.add_judgment(judgment)

        judgments = organism_state.get_recent_judgments(limit=10)
        assert len(judgments) == 1
        assert judgments[0]["judgment_id"] == "j1"

    @pytest.mark.asyncio
    async def test_recent_judgments_limited_to_100(self, organism_state):
        """Should cap recent_judgments at 100."""
        # Add 110 judgments
        for i in range(110):
            await organism_state.add_judgment({
                "judgment_id": f"j{i}",
                "q_score": 50.0 + (i % 30),
                "verdict": "WAG",
            })

        # Should only have last 100
        all_judgments = organism_state.get_recent_judgments(limit=1000)
        assert len(all_judgments) == 100

        # Most recent should be j109
        assert all_judgments[0]["judgment_id"] == "j109"

    @pytest.mark.asyncio
    async def test_recent_judgments_newest_first(self, organism_state):
        """get_recent_judgments should return newest first."""
        for i in range(5):
            await organism_state.add_judgment({
                "judgment_id": f"j{i}",
                "q_score": 50.0,
            })

        judgments = organism_state.get_recent_judgments(limit=10)
        assert judgments[0]["judgment_id"] == "j4"
        assert judgments[-1]["judgment_id"] == "j0"

    @pytest.mark.asyncio
    async def test_recent_judgments_respects_limit(self, organism_state):
        """get_recent_judgments(limit=n) should return at most n items."""
        for i in range(20):
            await organism_state.add_judgment({
                "judgment_id": f"j{i}",
                "q_score": 50.0,
            })

        judgments = organism_state.get_recent_judgments(limit=5)
        assert len(judgments) == 5
        assert judgments[0]["judgment_id"] == "j19"

    def test_consciousness_level_get_set(self, organism_state):
        """Should get/set consciousness level."""
        # Initial default
        assert organism_state.get_consciousness_level() == "REFLEX"

        # Set to MICRO
        asyncio.run(organism_state.update_consciousness_level("MICRO"))
        assert organism_state.get_consciousness_level() == "MICRO"

        # Set to MACRO
        asyncio.run(organism_state.update_consciousness_level("MACRO"))
        assert organism_state.get_consciousness_level() == "MACRO"

        # Set to META
        asyncio.run(organism_state.update_consciousness_level("META"))
        assert organism_state.get_consciousness_level() == "META"

    @pytest.mark.asyncio
    async def test_consciousness_level_persists(self, organism_state):
        """Consciousness level should be stored in persistent layer."""
        await organism_state.update_consciousness_level("MACRO")
        level = organism_state._persistent_state.get("consciousness_level")
        assert level == "MACRO"

    @pytest.mark.asyncio
    async def test_invalid_consciousness_level_rejected(self, organism_state):
        """Should reject invalid consciousness levels."""
        with pytest.raises(ValueError, match="Invalid consciousness level"):
            await organism_state.update_consciousness_level("INVALID")

        # Level should remain unchanged
        assert organism_state.get_consciousness_level() == "REFLEX"

    @pytest.mark.asyncio
    async def test_invalid_consciousness_levels_all_rejected(self, organism_state):
        """Should reject all invalid consciousness levels."""
        invalid_levels = ["ASLEEP", "AWAKE", "DREAMING", "dead", "123"]

        for invalid in invalid_levels:
            with pytest.raises(ValueError):
                await organism_state.update_consciousness_level(invalid)


class TestTask5DogsRegistry:
    """Task 5: Migrate dogs registry to OrganismState."""

    @pytest.mark.asyncio
    async def test_set_and_get_dogs(self, organism_state):
        """Should set and retrieve dogs registry."""
        dogs_config = {
            "dog1": {"breed": "SAGE", "model": "ollama"},
            "dog2": {"breed": "GUARDIAN", "model": "claude"},
        }

        await organism_state.set_dogs(dogs_config)

        retrieved = organism_state.get_dogs()
        assert retrieved == dogs_config

    @pytest.mark.asyncio
    async def test_get_dogs_returns_copy(self, organism_state):
        """get_dogs() should return a copy, not reference."""
        dogs = {"dog1": {"name": "Buddy"}}
        await organism_state.set_dogs(dogs)

        retrieved = organism_state.get_dogs()
        retrieved["dog1"]["name"] = "Changed"

        # Original should be unchanged
        assert organism_state.get_dogs()["dog1"]["name"] == "Buddy"

    @pytest.mark.asyncio
    async def test_get_single_dog(self, organism_state):
        """Should retrieve single dog by ID."""
        dogs = {
            "sage": {"breed": "SAGE", "model": "ollama"},
            "guardian": {"breed": "GUARDIAN", "model": "claude"},
        }
        await organism_state.set_dogs(dogs)

        sage_dog = organism_state.get_dog("sage")
        assert sage_dog == {"breed": "SAGE", "model": "ollama"}

    def test_get_nonexistent_dog_returns_none(self, organism_state):
        """get_dog() should return None for nonexistent dog."""
        asyncio.run(organism_state.set_dogs({"dog1": {}}))

        result = organism_state.get_dog("nonexistent")
        assert result is None

    @pytest.mark.asyncio
    async def test_get_dog_before_set_returns_none(self, organism_state):
        """get_dog() should return None if dogs never set."""
        result = organism_state.get_dog("any_dog")
        assert result is None

    @pytest.mark.asyncio
    async def test_dogs_is_stored_in_memory_layer(self, organism_state):
        """Dogs should be stored in memory layer."""
        dogs = {"dog1": {"name": "Buddy"}}
        await organism_state.set_dogs(dogs)

        assert "dogs" in organism_state._memory_state
        assert organism_state._memory_state["dogs"] == dogs


class TestTask6Residuals:
    """Task 6: Migrate residuals to OrganismState."""

    @pytest.mark.asyncio
    async def test_update_and_get_residual(self, organism_state):
        """Should update and retrieve residual."""
        residual_state = {
            "id": "res1",
            "type": "learning_gap",
            "confidence": 0.45,
        }

        await organism_state.update_residual("res1", residual_state)

        retrieved = organism_state.get_residual("res1")
        assert retrieved == residual_state

    @pytest.mark.asyncio
    async def test_get_nonexistent_residual_returns_empty(self, organism_state):
        """get_residual() should return {} if not found."""
        result = organism_state.get_residual("nonexistent")
        assert result == {}

    @pytest.mark.asyncio
    async def test_get_all_residuals(self, organism_state):
        """Should retrieve all residuals."""
        residuals = {
            "res1": {"type": "gap", "confidence": 0.3},
            "res2": {"type": "anomaly", "confidence": 0.7},
        }

        for res_id, state in residuals.items():
            await organism_state.update_residual(res_id, state)

        all_residuals = organism_state.get_all_residuals()
        assert all_residuals == residuals

    @pytest.mark.asyncio
    async def test_update_residual_overwrites(self, organism_state):
        """Updating residual should overwrite previous value."""
        await organism_state.update_residual("res1", {"v": 1})
        await organism_state.update_residual("res1", {"v": 2})

        residual = organism_state.get_residual("res1")
        assert residual == {"v": 2}

    @pytest.mark.asyncio
    async def test_clear_residuals(self, organism_state):
        """Should clear all residuals."""
        # Add some residuals
        for i in range(5):
            await organism_state.update_residual(
                f"res{i}",
                {"id": f"res{i}"}
            )

        # Verify they exist
        assert len(organism_state.get_all_residuals()) == 5

        # Clear them
        await organism_state.clear_residuals()

        # Should be empty
        assert len(organism_state.get_all_residuals()) == 0

    @pytest.mark.asyncio
    async def test_get_all_residuals_returns_copy(self, organism_state):
        """get_all_residuals() should return a copy."""
        await organism_state.update_residual("res1", {"value": 1})

        residuals = organism_state.get_all_residuals()
        residuals["res1"]["value"] = 999

        # Original should be unchanged
        assert organism_state.get_residual("res1")["value"] == 1

    @pytest.mark.asyncio
    async def test_get_all_residuals_empty(self, organism_state):
        """get_all_residuals() should return {} if none exist."""
        result = organism_state.get_all_residuals()
        assert result == {}

    @pytest.mark.asyncio
    async def test_residuals_stored_in_memory_layer(self, organism_state):
        """Residuals should be stored in memory layer."""
        await organism_state.update_residual("res1", {"data": "test"})

        assert "residuals" in organism_state._memory_state
        assert "res1" in organism_state._memory_state["residuals"]


class TestTask7ActionsQueue:
    """Task 7: Migrate actions queue to OrganismState."""

    @pytest.mark.asyncio
    async def test_add_and_get_actions(self, organism_state):
        """Should add action and retrieve it."""
        action = {
            "action_id": "a1",
            "type": "edit",
            "target": "file.py",
        }

        await organism_state.add_action(action)

        actions = organism_state.get_pending_actions()
        assert len(actions) == 1
        assert actions[0]["action_id"] == "a1"

    @pytest.mark.asyncio
    async def test_actions_capped_at_89(self, organism_state):
        """Should cap actions queue at 89 (BURN axiom)."""
        # Add 100 actions
        for i in range(100):
            await organism_state.add_action({
                "action_id": f"a{i}",
                "type": "edit",
            })

        # Should only have last 89
        actions = organism_state.get_pending_actions()
        assert len(actions) == 89

        # Most recent should be a99
        assert actions[-1]["action_id"] == "a99"

        # Oldest should be a11 (100 - 89 = 11)
        assert actions[0]["action_id"] == "a11"

    @pytest.mark.asyncio
    async def test_actions_fifo_order(self, organism_state):
        """Actions should be retrieved in FIFO order."""
        for i in range(5):
            await organism_state.add_action({"action_id": f"a{i}"})

        actions = organism_state.get_pending_actions()
        assert actions[0]["action_id"] == "a0"
        assert actions[4]["action_id"] == "a4"

    @pytest.mark.asyncio
    async def test_remove_action(self, organism_state):
        """Should remove action by ID."""
        for i in range(3):
            await organism_state.add_action({"action_id": f"a{i}"})

        # Remove middle action
        removed = await organism_state.remove_action("a1")
        assert removed is True

        actions = organism_state.get_pending_actions()
        assert len(actions) == 2
        assert actions[0]["action_id"] == "a0"
        assert actions[1]["action_id"] == "a2"

    @pytest.mark.asyncio
    async def test_remove_nonexistent_action_returns_false(self, organism_state):
        """remove_action() should return False if action not found."""
        await organism_state.add_action({"action_id": "a1"})

        removed = await organism_state.remove_action("nonexistent")
        assert removed is False

    @pytest.mark.asyncio
    async def test_remove_action_from_empty_queue(self, organism_state):
        """remove_action() should return False on empty queue."""
        removed = await organism_state.remove_action("any")
        assert removed is False

    @pytest.mark.asyncio
    async def test_clear_actions(self, organism_state):
        """Should clear all pending actions."""
        # Add actions
        for i in range(10):
            await organism_state.add_action({"action_id": f"a{i}"})

        # Verify they exist
        assert len(organism_state.get_pending_actions()) == 10

        # Clear them
        await organism_state.clear_actions()

        # Should be empty
        assert len(organism_state.get_pending_actions()) == 0

    @pytest.mark.asyncio
    async def test_get_pending_actions_returns_copy(self, organism_state):
        """get_pending_actions() should return a copy."""
        await organism_state.add_action({"action_id": "a1", "value": 1})

        actions = organism_state.get_pending_actions()
        actions[0]["value"] = 999

        # Original should be unchanged
        assert organism_state.get_pending_actions()[0]["value"] == 1

    @pytest.mark.asyncio
    async def test_get_pending_actions_empty(self, organism_state):
        """get_pending_actions() should return [] if none exist."""
        result = organism_state.get_pending_actions()
        assert result == []

    @pytest.mark.asyncio
    async def test_actions_stored_in_memory_layer(self, organism_state):
        """Actions should be stored in memory layer."""
        await organism_state.add_action({"action_id": "a1"})

        assert "pending_actions" in organism_state._memory_state


class TestTask4_7_ThreadSafety:
    """Test thread safety across all 4 tasks."""

    @pytest.mark.asyncio
    async def test_concurrent_judgments_thread_safe(self, organism_state):
        """Adding judgments concurrently should be thread-safe."""
        async def add_judgments(start, count):
            for i in range(start, start + count):
                await organism_state.add_judgment({
                    "judgment_id": f"j{i}",
                    "q_score": 50.0,
                })

        # Run concurrent additions
        await asyncio.gather(
            add_judgments(0, 10),
            add_judgments(10, 10),
            add_judgments(20, 10),
        )

        # Should have all 30
        judgments = organism_state.get_recent_judgments(limit=1000)
        assert len(judgments) == 30

    @pytest.mark.asyncio
    async def test_concurrent_actions_thread_safe(self, organism_state):
        """Adding actions concurrently should be thread-safe."""
        async def add_actions(start, count):
            for i in range(start, start + count):
                await organism_state.add_action({
                    "action_id": f"a{i}",
                    "type": "edit",
                })

        # Run concurrent additions
        await asyncio.gather(
            add_actions(0, 10),
            add_actions(10, 10),
            add_actions(20, 10),
        )

        # Should have all 30 (under 89 cap)
        actions = organism_state.get_pending_actions()
        assert len(actions) == 30

    @pytest.mark.asyncio
    async def test_concurrent_residuals_thread_safe(self, organism_state):
        """Updating residuals concurrently should be thread-safe."""
        async def update_residuals(start, count):
            for i in range(start, start + count):
                await organism_state.update_residual(
                    f"res{i}",
                    {"id": f"res{i}"}
                )

        # Run concurrent updates
        await asyncio.gather(
            update_residuals(0, 10),
            update_residuals(10, 10),
            update_residuals(20, 10),
        )

        # Should have all 30
        residuals = organism_state.get_all_residuals()
        assert len(residuals) == 30


class TestTask4_7_Integration:
    """Integration tests combining multiple tasks."""

    @pytest.mark.asyncio
    async def test_full_subsystem_workflow(self, organism_state):
        """Test realistic workflow with all 4 subsystems."""
        # Set consciousness level
        await organism_state.update_consciousness_level("MACRO")
        assert organism_state.get_consciousness_level() == "MACRO"

        # Set dogs
        dogs = {
            "sage": {"breed": "SAGE"},
            "guardian": {"breed": "GUARDIAN"},
        }
        await organism_state.set_dogs(dogs)
        assert len(organism_state.get_dogs()) == 2

        # Add judgments
        for i in range(5):
            await organism_state.add_judgment({
                "judgment_id": f"j{i}",
                "q_score": 50.0 + i,
            })
        assert len(organism_state.get_recent_judgments(limit=100)) == 5

        # Add residuals
        for i in range(3):
            await organism_state.update_residual(f"res{i}", {"id": f"res{i}"})
        assert len(organism_state.get_all_residuals()) == 3

        # Add actions
        for i in range(4):
            await organism_state.add_action({"action_id": f"a{i}"})
        assert len(organism_state.get_pending_actions()) == 4

    @pytest.mark.asyncio
    async def test_separate_memory_layers_independent(self, organism_state):
        """Tasks should use independent memory structures."""
        # Add data to all subsystems
        await organism_state.add_judgment({"judgment_id": "j1"})
        await organism_state.set_dogs({"dog1": {}})
        await organism_state.update_residual("res1", {})
        await organism_state.add_action({"action_id": "a1"})

        # Update consciousness (persistent)
        await organism_state.update_consciousness_level("MICRO")

        # Verify all exist independently
        assert len(organism_state.get_recent_judgments(limit=100)) == 1
        assert len(organism_state.get_dogs()) == 1
        assert len(organism_state.get_all_residuals()) == 1
        assert len(organism_state.get_pending_actions()) == 1
        assert organism_state.get_consciousness_level() == "MICRO"
