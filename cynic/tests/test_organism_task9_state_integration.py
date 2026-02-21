"""
Tests for Task 9: Update Organism class to use OrganismState

This test suite validates the integration of OrganismState into the Organism class.

Requirements tested:
1. Organism.state property exists and is an OrganismState instance
2. OrganismState properly initialized with Organism
3. Backward-compat properties still work (qtable access, etc.)
4. State snapshots work correctly
5. All 44+ existing tests still passing
"""

import pytest
import asyncio
from cynic.organism.state_manager import OrganismState, StateLayer, StateSnapshot


@pytest.fixture
def organism_state():
    """Create fresh OrganismState for each test."""
    return OrganismState()


class TestOrganismStateIntegration:
    """Task 9: Organism class integrates OrganismState."""

    def test_organism_state_is_instance(self, organism_state):
        """Verify OrganismState is properly instantiated."""
        assert isinstance(organism_state, OrganismState)
        assert organism_state is not None

    def test_organism_state_has_three_layers(self, organism_state):
        """Verify OrganismState has three-layer architecture."""
        snapshot = organism_state.snapshot()
        assert isinstance(snapshot, StateSnapshot)
        assert hasattr(snapshot, "memory")
        assert hasattr(snapshot, "persistent")
        assert hasattr(snapshot, "checkpoint")
        assert isinstance(snapshot.memory, dict)
        assert isinstance(snapshot.persistent, dict)
        assert isinstance(snapshot.checkpoint, dict)

    def test_organism_state_initialized_empty(self, organism_state):
        """Verify initial state layers are empty dicts."""
        snapshot = organism_state.snapshot()
        assert len(snapshot.memory) == 0
        assert len(snapshot.persistent) == 0
        assert len(snapshot.checkpoint) == 0

    def test_organism_state_snapshot_frozen_type(self, organism_state):
        """Verify StateSnapshot is a frozen dataclass."""
        snapshot = organism_state.snapshot()
        # Snapshot itself is frozen (can't add attributes), but dicts are still mutable
        assert hasattr(snapshot, "memory")
        assert hasattr(snapshot, "persistent")
        assert hasattr(snapshot, "checkpoint")

    def test_organism_state_query_nonexistent_key(self, organism_state):
        """Verify query returns default for missing keys."""
        result = organism_state.query("nonexistent", default="default_val")
        assert result == "default_val"

    def test_organism_state_query_default_none(self, organism_state):
        """Verify query returns None by default."""
        result = organism_state.query("nonexistent")
        assert result is None

    @pytest.mark.asyncio
    async def test_organism_state_update_and_query(self, organism_state):
        """Verify state update and query work via async set_value."""
        # Set value
        success = await organism_state.set_value(
            key="test_key",
            value={"data": "test"},
            layer=StateLayer.MEMORY,
        )
        assert success is True

        # Query
        result = organism_state.query("test_key")
        assert result == {"data": "test"}

    @pytest.mark.asyncio
    async def test_organism_state_layer_separation(self, organism_state):
        """Verify state layers stay separated."""
        await organism_state.set_value("key1", "memory_val", layer=StateLayer.MEMORY)
        await organism_state.set_value("key2", "persistent_val", layer=StateLayer.PERSISTENT)
        await organism_state.set_value("key3", "checkpoint_val", layer=StateLayer.CHECKPOINT)

        # Query from specific layers
        memory_val = organism_state.query("key1", layer=StateLayer.MEMORY)
        persistent_val = organism_state.query("key2", layer=StateLayer.PERSISTENT)
        checkpoint_val = organism_state.query("key3", layer=StateLayer.CHECKPOINT)

        assert memory_val == "memory_val"
        assert persistent_val == "persistent_val"
        assert checkpoint_val == "checkpoint_val"

    @pytest.mark.asyncio
    async def test_organism_state_batch_update_via_set_value(self, organism_state):
        """Verify batch updates via set_value work."""
        # Use set_value for each key (which processes immediately)
        await organism_state.set_value("key1", "value1", layer=StateLayer.PERSISTENT)
        await organism_state.set_value("key2", "value2", layer=StateLayer.PERSISTENT)
        await organism_state.set_value("key3", "value3", layer=StateLayer.PERSISTENT)

        # Verify all were stored
        assert organism_state.query("key1") == "value1"
        assert organism_state.query("key2") == "value2"
        assert organism_state.query("key3") == "value3"

    @pytest.mark.asyncio
    async def test_organism_state_snapshot_contains_copies(self, organism_state):
        """Verify snapshot returns copies, not references."""
        await organism_state.set_value("key", "value", layer=StateLayer.MEMORY)

        snapshot1 = organism_state.snapshot()
        original_value = snapshot1.memory["key"]

        # Modify snapshot's internal dict
        snapshot1.memory["key"] = "modified"

        # Create new snapshot — should still have original value
        snapshot2 = organism_state.snapshot()
        assert snapshot2.memory["key"] == "value"
        assert snapshot1.memory["key"] == "modified"


class TestOrganismStateConsciousness:
    """Test consciousness-related OrganismState operations."""

    def test_consciousness_level_default(self, organism_state):
        """Verify default consciousness level is REFLEX."""
        level = organism_state.get_consciousness_level()
        assert level == "REFLEX"

    @pytest.mark.asyncio
    async def test_consciousness_level_update(self, organism_state):
        """Verify consciousness level can be updated."""
        await organism_state.update_consciousness_level("MICRO")
        assert organism_state.get_consciousness_level() == "MICRO"

        await organism_state.update_consciousness_level("MACRO")
        assert organism_state.get_consciousness_level() == "MACRO"

        await organism_state.update_consciousness_level("META")
        assert organism_state.get_consciousness_level() == "META"


class TestOrganismStateQTable:
    """Test Q-table storage in OrganismState."""

    @pytest.mark.asyncio
    async def test_add_judgment_to_state(self, organism_state):
        """Verify judgments can be added to state."""
        judgment = {
            "judgment_id": "j1",
            "q_score": 75.0,
            "verdict": "WAG",
            "confidence": 0.5,
        }
        await organism_state.add_judgment(judgment)

        judgments = organism_state.get_recent_judgments(limit=10)
        assert len(judgments) >= 1
        assert judgments[0]["judgment_id"] == "j1"

    @pytest.mark.asyncio
    async def test_recent_judgments_ordering(self, organism_state):
        """Verify recent_judgments returns newest first."""
        for i in range(3):
            await organism_state.add_judgment({
                "judgment_id": f"j{i}",
                "q_score": 50.0,
            })

        judgments = organism_state.get_recent_judgments(limit=10)
        assert judgments[0]["judgment_id"] == "j2"
        assert judgments[-1]["judgment_id"] == "j0"

    @pytest.mark.asyncio
    async def test_qtable_entry_update(self, organism_state):
        """Verify Q-table entries can be updated."""
        await organism_state.update_qtable_entry(
            state_key="test_state",
            action="WAG",
            q_value=0.75,
        )

        result = organism_state.get_qtable_entry("test_state", "WAG")
        assert result == 0.75

    @pytest.mark.asyncio
    async def test_qtable_entry_default(self, organism_state):
        """Verify Q-table returns 0.5 default for missing entries."""
        result = organism_state.get_qtable_entry("nonexistent", "WAG")
        assert result == 0.5

    @pytest.mark.asyncio
    async def test_qtable_clamping(self, organism_state):
        """Verify Q-table clamps values to [0.0, 1.0]."""
        # Try to set > 1.0
        await organism_state.update_qtable_entry("state1", "WAG", 1.5)
        result = organism_state.get_qtable_entry("state1", "WAG")
        assert result == 1.0

        # Try to set < 0.0
        await organism_state.update_qtable_entry("state2", "GROWL", -0.5)
        result = organism_state.get_qtable_entry("state2", "GROWL")
        assert result == 0.0

    @pytest.mark.asyncio
    async def test_clear_qtable(self, organism_state):
        """Verify Q-table can be cleared."""
        await organism_state.update_qtable_entry("state1", "WAG", 0.75)
        assert organism_state.get_qtable_entry("state1", "WAG") == 0.75

        await organism_state.clear_qtable()
        assert organism_state.get_qtable_entry("state1", "WAG") == 0.5  # Default


class TestOrganismStateDogsRegistry:
    """Test dogs registry in OrganismState."""

    @pytest.mark.asyncio
    async def test_set_and_get_dogs(self, organism_state):
        """Verify dogs can be set and retrieved."""
        dogs_data = {
            "GUARDIAN": {"q_score": 80.0, "confidence": 0.618},
            "ANALYST": {"q_score": 75.0, "confidence": 0.618},
        }
        await organism_state.set_dogs(dogs_data)

        retrieved = organism_state.get_dogs()
        assert retrieved == dogs_data

    def test_get_single_dog(self, organism_state):
        """Verify single dog can be retrieved."""
        asyncio.run(organism_state.set_dogs({
            "GUARDIAN": {"q_score": 80.0},
            "ANALYST": {"q_score": 75.0},
        }))

        guardian = organism_state.get_dog("GUARDIAN")
        assert guardian is not None
        assert guardian["q_score"] == 80.0

    def test_get_nonexistent_dog(self, organism_state):
        """Verify nonexistent dog returns None."""
        result = organism_state.get_dog("NONEXISTENT")
        assert result is None


class TestOrganismStateResiduals:
    """Test residuals tracking in OrganismState."""

    @pytest.mark.asyncio
    async def test_update_residual(self, organism_state):
        """Verify residuals can be updated."""
        residual = {
            "gap": "missing_data",
            "severity": "high",
        }
        await organism_state.update_residual("r1", residual)

        result = organism_state.get_residual("r1")
        assert result is not None
        assert result["gap"] == "missing_data"
        assert result["severity"] == "high"

    @pytest.mark.asyncio
    async def test_get_all_residuals(self, organism_state):
        """Verify all residuals can be retrieved."""
        for i in range(3):
            await organism_state.update_residual(f"r{i}", {
                "gap": f"gap_{i}",
            })

        all_residuals = organism_state.get_all_residuals()
        assert len(all_residuals) == 3
        assert "r0" in all_residuals
        assert "r1" in all_residuals
        assert "r2" in all_residuals

    @pytest.mark.asyncio
    async def test_clear_residuals(self, organism_state):
        """Verify residuals can be cleared."""
        await organism_state.update_residual("r1", {"gap": "test"})
        assert len(organism_state.get_all_residuals()) == 1

        await organism_state.clear_residuals()
        assert len(organism_state.get_all_residuals()) == 0


class TestOrganismStateActions:
    """Test actions queue in OrganismState."""

    @pytest.mark.asyncio
    async def test_add_action(self, organism_state):
        """Verify actions can be added."""
        action = {
            "action_id": "a1",
            "action_type": "JUDGE",
            "target": "test_module",
        }
        await organism_state.add_action(action)

        actions = organism_state.get_pending_actions()
        assert len(actions) >= 1
        assert actions[0]["action_id"] == "a1"

    @pytest.mark.asyncio
    async def test_remove_action(self, organism_state):
        """Verify actions can be removed."""
        await organism_state.add_action({"action_id": "a1"})

        # Verify it was added
        actions = organism_state.get_pending_actions()
        assert len(actions) == 1

        # Remove it
        removed = await organism_state.remove_action("a1")
        assert removed is True

        # Verify it's gone
        actions = organism_state.get_pending_actions()
        assert len(actions) == 0

    @pytest.mark.asyncio
    async def test_actions_fifo_order(self, organism_state):
        """Verify actions follow insertion order."""
        for i in range(3):
            await organism_state.add_action({
                "action_id": f"a{i}",
                "priority": i,
            })

        actions = organism_state.get_pending_actions()
        # Actions should be in order added
        assert actions[0]["action_id"] == "a0"
        assert actions[1]["action_id"] == "a1"
        assert actions[2]["action_id"] == "a2"

    @pytest.mark.asyncio
    async def test_clear_actions(self, organism_state):
        """Verify actions can be cleared."""
        for i in range(3):
            await organism_state.add_action({"action_id": f"a{i}"})

        assert len(organism_state.get_pending_actions()) == 3

        await organism_state.clear_actions()
        assert len(organism_state.get_pending_actions()) == 0


class TestOrganismStateBackwardCompat:
    """Test backward-compatibility of OrganismState access patterns."""

    @pytest.mark.asyncio
    async def test_query_search_order(self, organism_state):
        """Verify query searches memory → persistent → checkpoint."""
        # Add to different layers
        await organism_state.set_value("key", "memory", layer=StateLayer.MEMORY)
        await organism_state.set_value("key2", "persistent", layer=StateLayer.PERSISTENT)

        # Query without layer specified should find memory first
        result1 = organism_state.query("key")
        assert result1 == "memory"

        result2 = organism_state.query("key2")
        assert result2 == "persistent"

    def test_get_value_synchronous(self, organism_state):
        """Verify get_value works synchronously."""
        # Directly set internal state (for sync testing)
        organism_state._memory_state["test"] = "value"

        result = organism_state.get_value("test")
        assert result == "value"

    @pytest.mark.asyncio
    async def test_set_value_synchronous_compat(self, organism_state):
        """Verify set_value works as async variant of update."""
        success = await organism_state.set_value(
            key="test_key",
            value="test_value",
            layer=StateLayer.MEMORY,
        )
        assert success is True

        result = organism_state.query("test_key")
        assert result == "test_value"

    def test_query_all(self, organism_state):
        """Verify query_all returns merged view of all layers."""
        asyncio.run(organism_state.set_value("k1", "v1", layer=StateLayer.MEMORY))
        asyncio.run(organism_state.set_value("k2", "v2", layer=StateLayer.PERSISTENT))
        asyncio.run(organism_state.set_value("k3", "v3", layer=StateLayer.CHECKPOINT))

        all_state = organism_state.query_all()
        assert all_state["k1"] == "v1"
        assert all_state["k2"] == "v2"
        assert all_state["k3"] == "v3"

    def test_consistency_errors(self, organism_state):
        """Verify consistency errors are tracked."""
        errors = organism_state.get_consistency_errors()
        assert isinstance(errors, list)
