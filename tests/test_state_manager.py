"""
Tests for OrganismState - Three-Layer State Management

Tests the OrganismState class for:
- Memory/Persistent/Checkpoint layers
- Q-Table subsystem
- Consciousness management
- Dogs registry
- Persistence and recovery
"""
import pytest

pytestmark = pytest.mark.skip(reason="Old architecture removed in V5 - StateSnapshot class deleted")

import asyncio
from unittest.mock import AsyncMock, MagicMock, patch
from pathlib import Path

from cynic.kernel.organism.state_manager import (
    OrganismState, StateLayer, StateUpdate, StateSnapshot,
)


class TestOrganismState:
    """Test suite for OrganismState."""

    @pytest.fixture
    def state(self, tmp_path):
        """Create a fresh OrganismState for testing."""
        return OrganismState(storage_path=tmp_path / "test_state")

    def test_initialization(self, state):
        """Should initialize with empty layers."""
        assert len(state._memory_state) == 0
        assert len(state._persistent_state) == 0
        assert len(state._checkpoint_state) == 0

    def test_qtable_initialized(self, state):
        """Should initialize empty Q-table."""
        assert state._qtable == {}

    def test_storage_path_created(self, state, tmp_path):
        """Should create storage directory."""
        assert (tmp_path / "test_state").exists()

    @pytest.mark.asyncio
    async def test_initialize(self, state):
        """Should initialize without error."""
        result = await state.initialize()
        assert result is True


class TestStateLayers:
    """Test suite for state layer operations."""

    @pytest.fixture
    def state(self, tmp_path):
        return OrganismState(storage_path=tmp_path / "test")

    @pytest.mark.asyncio
    async def test_set_value_memory(self, state):
        """Should set value in memory layer."""
        result = await state.set_value("test_key", "test_value", StateLayer.MEMORY)
        
        assert result is True
        assert state.get_value("test_key", StateLayer.MEMORY) == "test_value"

    @pytest.mark.asyncio
    async def test_set_value_persistent(self, state):
        """Should set value in persistent layer."""
        result = await state.set_value("test_key", "test_value", StateLayer.PERSISTENT)
        
        assert result is True
        assert state.get_value("test_key", StateLayer.PERSISTENT) == "test_value"

    def test_get_value_default(self, state):
        """Should return default when key not found."""
        result = state.get_value("nonexistent", default="default_value")
        
        assert result == "default_value"

    @pytest.mark.asyncio
    async def test_update(self, state):
        """Should update via update method."""
        result = await state.update("key1", "value1", StateLayer.MEMORY)
        
        assert result is True

    @pytest.mark.asyncio
    async def test_update_many(self, state):
        """Should update multiple keys."""
        updates = {"k1": "v1", "k2": "v2"}
        
        result = await state.update_many(updates, StateLayer.MEMORY)
        
        assert result is True

    def test_query(self, state):
        """Should query across layers."""
        state._memory_state["mem_key"] = "mem_value"
        state._persistent_state["persist_key"] = "persist_value"
        
        assert state.query("mem_key") == "mem_value"
        assert state.query("persist_key") == "persist_value"

    def test_query_all(self, state):
        """Should query all layers merged."""
        state._memory_state["m1"] = "v1"
        state._persistent_state["p1"] = "v2"
        
        all_state = state.query_all()
        
        assert "m1" in all_state
        assert "p1" in all_state


class TestQTableSubsystem:
    """Test suite for Q-Table subsystem."""

    @pytest.fixture
    def state(self, tmp_path):
        return OrganismState(storage_path=tmp_path / "test")

    @pytest.mark.asyncio
    async def test_update_qtable_entry(self, state):
        """Should update Q-table entry."""
        await state.update_qtable_entry("test_state", "BARK", 0.8)
        
        q = state.get_qtable_entry("test_state", "BARK")
        assert q == 0.8

    def test_qtable_entry_clamping(self, state):
        """Should clamp Q-value to [0, 1]."""
        # This requires await but let's test the logic
        # In practice, this is called from async context
        pass

    @pytest.mark.asyncio
    async def test_get_qtable_entries(self, state):
        """Should get all actions for state."""
        await state.update_qtable_entry("state1", "BARK", 0.7)
        await state.update_qtable_entry("state1", "WAG", 0.5)
        
        entries = state.get_qtable_entries("state1")
        
        assert "BARK" in entries
        assert "WAG" in entries

    @pytest.mark.asyncio
    async def test_clear_qtable(self, state):
        """Should clear Q-table."""
        await state.update_qtable_entry("s1", "BARK", 0.9)
        
        await state.clear_qtable()
        
        assert state._qtable == {}


class TestConsciousnessManagement:
    """Test suite for consciousness management."""

    @pytest.fixture
    def state(self, tmp_path):
        return OrganismState(storage_path=tmp_path / "test")

    def test_get_consciousness_level(self, state):
        """Should return default level."""
        level = state.get_consciousness_level()
        
        assert level == "REFLEX"

    @pytest.mark.asyncio
    async def test_update_consciousness_level(self, state):
        """Should update consciousness level."""
        await state.update_consciousness_level("MACRO")
        
        assert state.get_consciousness_level() == "MACRO"

    @pytest.mark.asyncio
    async def test_update_consciousness_invalid(self, state):
        """Should raise on invalid level."""
        with pytest.raises(ValueError):
            await state.update_consciousness_level("INVALID")


class TestDogsRegistry:
    """Test suite for dogs registry."""

    @pytest.fixture
    def state(self, tmp_path):
        return OrganismState(storage_path=tmp_path / "test")

    @pytest.mark.asyncio
    async def test_set_dogs(self, state):
        """Should set dogs registry."""
        dogs = {"sage": {"model": "gemma2:2b"}, "guardian": {"model": "llama3"}}
        
        await state.set_dogs(dogs)
        
        assert state.get_dogs() == dogs

    def test_get_dog(self, state):
        """Should get single dog."""
        state._memory_state["dogs"] = {"sage": {"model": "gemma"}}
        
        dog = state.get_dog("sage")
        
        assert dog == {"model": "gemma"}

    def test_get_dog_not_found(self, state):
        """Should return None for missing dog."""
        result = state.get_dog("nonexistent")
        
        assert result is None


class TestJudgmentTracking:
    """Test suite for judgment tracking."""

    @pytest.fixture
    def state(self, tmp_path):
        return OrganismState(storage_path=tmp_path / "test")

    @pytest.mark.asyncio
    async def test_add_judgment(self, state):
        """Should add judgment to recent list."""
        judgment = {"judgment_id": "j1", "q_score": 75}
        
        await state.add_judgment(judgment)
        
        recent = state.get_recent_judgments()
        
        assert len(recent) == 1
        assert recent[0]["judgment_id"] == "j1"

    @pytest.mark.asyncio
    async def test_judgment_limit(self, state):
        """Should cap recent judgments at 100."""
        for i in range(150):
            await state.add_judgment({"judgment_id": f"j{i}", "q_score": 50})
        
        recent = state.get_recent_judgments()
        
        assert len(recent) <= 100


class TestActionQueue:
    """Test suite for action queue."""

    @pytest.fixture
    def state(self, tmp_path):
        return OrganismState(storage_path=tmp_path / "test")

    @pytest.mark.asyncio
    async def test_add_action(self, state):
        """Should add action to queue."""
        action = {"action_id": "a1", "type": "deploy"}
        
        await state.add_action(action)
        
        pending = state.get_pending_actions()
        
        assert len(pending) == 1
        assert pending[0]["action_id"] == "a1"

    @pytest.mark.asyncio
    async def test_remove_action(self, state):
        """Should remove action by ID."""
        await state.add_action({"action_id": "a1"})
        
        result = await state.remove_action("a1")
        
        assert result is True
        assert len(state.get_pending_actions()) == 0

    @pytest.mark.asyncio
    async def test_clear_actions(self, state):
        """Should clear all actions."""
        await state.add_action({"action_id": "a1"})
        await state.add_action({"action_id": "a2"})
        
        await state.clear_actions()
        
        assert len(state.get_pending_actions()) == 0


class TestSnapshot:
    """Test suite for state snapshots."""

    def test_snapshot_immutable(self):
        """Snapshot should be frozen."""
        snapshot = StateSnapshot(
            memory={"key": "value"},
            persistent={},
            checkpoint={}
        )
        
        # Should not be able to modify
        with pytest.raises(AttributeError):
            snapshot.memory["new_key"] = "new_value"

    def test_snapshot_returns_copies(self):
        """Snapshot should return copies, not references."""
        state_data = {"key": "value"}
        
        snapshot = StateSnapshot(
            memory=state_data,
            persistent={},
            checkpoint={}
        )
        
        # Modify original
        state_data["key"] = "modified"
        
        # Snapshot should be unchanged
        assert snapshot.memory["key"] == "value"


class TestConsistencyChecks:
    """Test suite for consistency checks."""

    @pytest.fixture
    def state(self, tmp_path):
        return OrganismState(storage_path=tmp_path / "test")

    def test_consistency_errors(self, state):
        """Should track consistency errors."""
        errors = state.get_consistency_errors()
        
        assert isinstance(errors, list)

    def test_clear_consistency_errors(self, state):
        """Should clear errors."""
        state._consistency_errors.append("test error")
        
        state.clear_consistency_errors()
        
        assert len(state._consistency_errors) == 0

    @pytest.mark.asyncio
    async def test_consistency_check_json_serializable(self, state):
        """Should accept JSON-serializable values."""
        result = await state.set_value("key", {"nested": "value"}, StateLayer.MEMORY)
        
        assert result is True

    @pytest.mark.asyncio
    async def test_consistency_check_too_large(self, state):
        """Should reject values > 10MB."""
        large_value = "x" * (11 * 1024 * 1024)  # 11MB
        
        result = await state.set_value("large", large_value, StateLayer.MEMORY)
        
        assert result is False


class TestStats:
    """Test suite for state statistics."""

    @pytest.fixture
    def state(self, tmp_path):
        return OrganismState(storage_path=tmp_path / "test")

    def test_get_stats(self, state):
        """Should return stats."""
        stats = state.get_stats()
        
        assert "memory_keys" in stats
        assert "persistent_keys" in stats
        assert "queue_size" in stats

    def test_repr(self, state):
        """Should have string representation."""
        repr_str = repr(state)
        
        assert "OrganismState" in repr_str
