import tempfile
from pathlib import Path

import pytest

from cynic.kernel.organism.brain.learning.memory_store import MemoryStore
from cynic.kernel.organism.brain.learning.relationship_memory import RelationshipMemory


@pytest.fixture
async def memory_store():
    """Create temporary memory store."""
    with tempfile.TemporaryDirectory() as tmpdir:
        store = MemoryStore(Path(tmpdir))
        await store.initialize()
        yield store


@pytest.mark.asyncio
async def test_store_initialization(memory_store):
    """Store initializes default memory file."""
    assert memory_store.memory_path.exists()


@pytest.mark.asyncio
async def test_save_and_load_memory(memory_store):
    """Save relationship memory and reload it."""
    memory = RelationshipMemory(
        user_values={"PHI": 0.9, "BURN": 0.6},
        user_preferences={"financial": "GROWL"},
        user_style="analytical",
        communication_style={"verbosity": "concise"},
        learning_rate=0.01
    )

    await memory_store.save_memory(memory)
    loaded = await memory_store.load_memory()

    assert loaded.user_values["PHI"] == 0.9
    assert loaded.user_preferences["financial"] == "GROWL"


@pytest.mark.asyncio
async def test_load_default_memory():
    """Load default memory when none exists."""
    with tempfile.TemporaryDirectory() as tmpdir:
        store = MemoryStore(Path(tmpdir))
        await store.initialize()

        memory = await store.load_memory()
        assert memory is not None
        assert memory.learning_rate > 0
        assert isinstance(memory.user_values, dict)


@pytest.mark.asyncio
async def test_update_and_persist(memory_store):
    """Update memory and persist changes."""
    initial = await memory_store.load_memory()
    updated = initial.update_preference("governance", "WAG")

    await memory_store.save_memory(updated)
    reloaded = await memory_store.load_memory()

    assert reloaded.user_preferences.get("governance") == "WAG"
