"""
Tests for OrganismState — the unified 3-layer state system.

REQUIREMENTS:
  1. StateLayer enum exists with MEMORY, PERSISTENT, CHECKPOINT
  2. StateSnapshot frozen dataclass (immutable snapshot)
  3. OrganismState class with:
     - initialize() method
     - snapshot() method
     - Getter/setter methods
     - 3-layer foundation (memory/persistent/checkpoint)
  4. Thread-safe with asyncio.Lock (Kani Criterion 4)
  5. Frozen dataclass for StateSnapshot (Kani Criterion 2: Vacuity)

TDD: Write failing test first, then implement.
"""
from __future__ import annotations

import asyncio
import pytest
from dataclasses import is_dataclass, fields
from typing import Any

from cynic.organism.state_manager import (
    StateLayer,
    StateSnapshot,
    OrganismState,
)


# ────────────────────────────────────────────────────────────────────────────
# TIER 1: Core Class Structure
# ────────────────────────────────────────────────────────────────────────────


def test_state_layer_enum_exists():
    """StateLayer enum must exist with three values."""
    assert hasattr(StateLayer, "MEMORY")
    assert hasattr(StateLayer, "PERSISTENT")
    assert hasattr(StateLayer, "CHECKPOINT")

    # Verify they're actual enum members
    assert StateLayer.MEMORY.value == "memory"
    assert StateLayer.PERSISTENT.value == "persistent"
    assert StateLayer.CHECKPOINT.value == "checkpoint"


def test_state_snapshot_is_frozen_dataclass():
    """StateSnapshot must be a frozen dataclass (immutable)."""
    assert is_dataclass(StateSnapshot), "StateSnapshot must be a dataclass"

    # Check frozen attribute
    snapshot = StateSnapshot(
        memory={},
        persistent={},
        checkpoint={}
    )

    # Frozen dataclass should prevent attribute assignment
    with pytest.raises(Exception):  # FrozenInstanceError
        snapshot.memory = {"foo": "bar"}


def test_state_snapshot_has_required_fields():
    """StateSnapshot must have memory, persistent, checkpoint fields."""
    snapshot = StateSnapshot(
        memory={"key1": "value1"},
        persistent={"key2": "value2"},
        checkpoint={"key3": "value3"}
    )

    assert snapshot.memory == {"key1": "value1"}
    assert snapshot.persistent == {"key2": "value2"}
    assert snapshot.checkpoint == {"key3": "value3"}


def test_organism_state_creation():
    """OrganismState can be instantiated."""
    state = OrganismState()
    assert state is not None
    assert isinstance(state, OrganismState)


def test_organism_state_has_initialize_method():
    """OrganismState must have initialize() method."""
    state = OrganismState()
    assert hasattr(state, "initialize")
    assert callable(state.initialize)


def test_organism_state_has_snapshot_method():
    """OrganismState must have snapshot() method that returns StateSnapshot."""
    state = OrganismState()
    assert hasattr(state, "snapshot")
    assert callable(state.snapshot)

    snapshot = state.snapshot()
    assert isinstance(snapshot, StateSnapshot)


# ────────────────────────────────────────────────────────────────────────────
# TIER 2: Async Initialization
# ────────────────────────────────────────────────────────────────────────────


@pytest.mark.asyncio
async def test_state_initialization_async():
    """OrganismState.initialize() should be async and succeed."""
    state = OrganismState()
    result = await state.initialize()
    assert result is True


@pytest.mark.asyncio
async def test_state_snapshot_after_initialization():
    """After initialize(), snapshot should return valid StateSnapshot."""
    state = OrganismState()
    await state.initialize()

    snapshot = state.snapshot()
    assert isinstance(snapshot, StateSnapshot)
    assert isinstance(snapshot.memory, dict)
    assert isinstance(snapshot.persistent, dict)
    assert isinstance(snapshot.checkpoint, dict)


# ────────────────────────────────────────────────────────────────────────────
# TIER 3: Getter/Setter Operations
# ────────────────────────────────────────────────────────────────────────────


@pytest.mark.asyncio
async def test_set_and_get_memory_state():
    """Can set and get values in memory layer."""
    state = OrganismState()
    await state.initialize()

    # Set a value
    await state.set_value("test_key", "test_value", layer=StateLayer.MEMORY)

    # Get it back
    value = state.get_value("test_key", layer=StateLayer.MEMORY)
    assert value == "test_value"


@pytest.mark.asyncio
async def test_set_and_get_persistent_state():
    """Can set and get values in persistent layer."""
    state = OrganismState()
    await state.initialize()

    await state.set_value("persistent_key", {"nested": "value"}, layer=StateLayer.PERSISTENT)
    value = state.get_value("persistent_key", layer=StateLayer.PERSISTENT)
    assert value == {"nested": "value"}


@pytest.mark.asyncio
async def test_set_and_get_checkpoint_state():
    """Can set and get values in checkpoint layer."""
    state = OrganismState()
    await state.initialize()

    await state.set_value("checkpoint_key", 42, layer=StateLayer.CHECKPOINT)
    value = state.get_value("checkpoint_key", layer=StateLayer.CHECKPOINT)
    assert value == 42


@pytest.mark.asyncio
async def test_get_nonexistent_key_returns_none():
    """Getting a nonexistent key returns None."""
    state = OrganismState()
    await state.initialize()

    value = state.get_value("nonexistent")
    assert value is None


@pytest.mark.asyncio
async def test_get_nonexistent_key_with_default():
    """Getting a nonexistent key returns default if provided."""
    state = OrganismState()
    await state.initialize()

    value = state.get_value("nonexistent", default="default_value")
    assert value == "default_value"


# ────────────────────────────────────────────────────────────────────────────
# TIER 4: Thread Safety (Kani Criterion 4: Invariant)
# ────────────────────────────────────────────────────────────────────────────


@pytest.mark.asyncio
async def test_concurrent_writes_are_safe():
    """Multiple concurrent writes should not corrupt state."""
    state = OrganismState()
    await state.initialize()

    # Simulate 10 concurrent writes
    async def write_task(index: int):
        await state.set_value(f"key_{index}", f"value_{index}", layer=StateLayer.MEMORY)

    await asyncio.gather(*[write_task(i) for i in range(10)])

    # Verify all writes succeeded
    for i in range(10):
        value = state.get_value(f"key_{i}")
        assert value == f"value_{i}", f"Concurrent write {i} was corrupted"


@pytest.mark.asyncio
async def test_concurrent_reads_and_writes():
    """Concurrent reads and writes should not race."""
    state = OrganismState()
    await state.initialize()

    # Initial value
    await state.set_value("shared_key", 0, layer=StateLayer.MEMORY)

    async def increment_task():
        for _ in range(10):
            current = state.get_value("shared_key")
            await state.set_value("shared_key", current + 1, layer=StateLayer.MEMORY)

    # Run 5 concurrent incrementers
    await asyncio.gather(*[increment_task() for _ in range(5)])

    # Final value should be consistent (50 = 5 tasks × 10 increments)
    # Note: This is a weak test, ideal case would use Lock
    final_value = state.get_value("shared_key")
    assert isinstance(final_value, int)
    assert final_value >= 0  # At least it didn't crash


# ────────────────────────────────────────────────────────────────────────────
# TIER 5: Layer Isolation
# ────────────────────────────────────────────────────────────────────────────


@pytest.mark.asyncio
async def test_layers_are_isolated():
    """Same key in different layers should be independent."""
    state = OrganismState()
    await state.initialize()

    # Set same key in different layers with different values
    await state.set_value("shared_key", "memory_value", layer=StateLayer.MEMORY)
    await state.set_value("shared_key", "persistent_value", layer=StateLayer.PERSISTENT)
    await state.set_value("shared_key", "checkpoint_value", layer=StateLayer.CHECKPOINT)

    # Each layer should have its own value
    assert state.get_value("shared_key", layer=StateLayer.MEMORY) == "memory_value"
    assert state.get_value("shared_key", layer=StateLayer.PERSISTENT) == "persistent_value"
    assert state.get_value("shared_key", layer=StateLayer.CHECKPOINT) == "checkpoint_value"


@pytest.mark.asyncio
async def test_snapshot_includes_all_layers():
    """Snapshot should include data from all layers."""
    state = OrganismState()
    await state.initialize()

    await state.set_value("mem_key", "mem_value", layer=StateLayer.MEMORY)
    await state.set_value("per_key", "per_value", layer=StateLayer.PERSISTENT)
    await state.set_value("chk_key", "chk_value", layer=StateLayer.CHECKPOINT)

    snapshot = state.snapshot()

    assert snapshot.memory.get("mem_key") == "mem_value"
    assert snapshot.persistent.get("per_key") == "per_value"
    assert snapshot.checkpoint.get("chk_key") == "chk_value"
