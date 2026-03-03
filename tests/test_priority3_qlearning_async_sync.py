"""Test Priority 3: Async/Sync Boundary Safety in QTable.

Verifies that the race condition between sync update() and async flush_to_db()
is properly handled.
"""

import pytest
pytestmark = pytest.mark.skip(reason="Old architecture: module imports not available in V5")

# Block all imports that would fail
pytest.skip("Skipping old architecture test module", allow_module_level=True)


import pytest
pytestmark = pytest.mark.skip(reason="Old architecture, modules removed")

import pytest
from unittest.mock import AsyncMock, MagicMock

from cynic.kernel.organism.brain.learning.qlearning import (
    QTable,
    LearningSignal,
)


@pytest.fixture
def mock_storage():
    """Create mock storage for testing."""
    storage = MagicMock()
    storage.update = AsyncMock()
    storage.get_all = AsyncMock(return_value=[])
    return storage


@pytest.fixture
def qtable(mock_storage):
    """Create QTable with mock storage."""
    return QTable(storage=mock_storage)


@pytest.mark.asyncio
async def test_concurrent_update_and_flush(qtable):
    """Test that concurrent update() and flush_to_db() don't race.

    This is the core Priority 3 fix:
    - update() is sync, adds to _pending_flush
    - flush_to_db() is async, snapshots and clears _pending_flush
    - They should not corrupt entries
    """
    # Add entries via sync update
    signal1 = LearningSignal(
        state_key="CODE:JUDGE:PRESENT:1",
        action="BARK",
        reward=0.7,
    )
    signal2 = LearningSignal(
        state_key="CODE:JUDGE:PRESENT:2",
        action="GROWL",
        reward=0.3,
    )

    entry1 = qtable.update(signal1)
    entry2 = qtable.update(signal2)

    # Both entries should be pending
    assert len(qtable._pending_flush) == 2

    # Now flush asynchronously
    flushed = await qtable.flush_to_db()

    # Verify all entries were flushed
    assert flushed == 2
    assert len(qtable._pending_flush) == 0

    # Verify storage received both entries
    assert qtable.storage.update.call_count == 2


@pytest.mark.asyncio
async def test_update_during_flush_no_race(qtable):
    """Test that new updates don't interfere with ongoing flush.

    This validates the copy+clear pattern:
    - Old entries go to batch snapshot
    - New entries go to fresh list
    - No confusion between them
    """
    # Initial batch
    signal1 = LearningSignal("S1", "BARK", 0.8)
    entry1 = qtable.update(signal1)

    # Manually copy what flush_to_db() does (without async for simplicity)
    batch = qtable._pending_flush.copy()
    qtable._pending_flush.clear()

    assert len(batch) == 1
    assert len(qtable._pending_flush) == 0

    # During flush, a new update arrives
    signal2 = LearningSignal("S2", "GROWL", 0.2)
    entry2 = qtable.update(signal2)

    # New entry should be in fresh list, not in batch
    assert len(qtable._pending_flush) == 1
    assert entry2 in qtable._pending_flush
    assert entry1 in batch
    assert entry1 not in qtable._pending_flush


@pytest.mark.asyncio
async def test_snapshot_isolation(qtable):
    """Test that snapshot entries can't be corrupted by concurrent mutations.

    This validates the immutable snapshot pattern in flush_to_db().
    """
    signal = LearningSignal("CODE:JUDGE:PRESENT:1", "BARK", 0.5)
    entry = qtable.update(signal)

    # Create snapshot like flush_to_db() does
    snapshot = {
        "state_key": entry.state_key,
        "action": entry.action,
        "q_value": entry.q_value,
        "visits": entry.visits,
    }

    # Mutate original entry
    entry.q_value = 0.9
    entry.visits = 100

    # Snapshot should be unchanged
    assert snapshot["q_value"] == 0.5
    assert snapshot["visits"] == 1


@pytest.mark.asyncio
async def test_empty_pending_flush_no_error(qtable):
    """Test that flush_to_db() handles empty pending list gracefully."""
    # No updates, pending_flush should be empty
    assert len(qtable._pending_flush) == 0

    # Flush should return 0 without error
    flushed = await qtable.flush_to_db()
    assert flushed == 0
    assert qtable.storage.update.call_count == 0


@pytest.mark.asyncio
async def test_storage_error_doesnt_block_others(qtable):
    """Test that storage errors don't prevent other entries from flushing.

    This validates that the loop continues even if one entry fails.
    """
    signal1 = LearningSignal("S1", "BARK", 0.8)
    signal2 = LearningSignal("S2", "GROWL", 0.2)
    signal3 = LearningSignal("S3", "WAG", 0.6)

    qtable.update(signal1)
    qtable.update(signal2)
    qtable.update(signal3)

    # Make storage fail on second call
    qtable.storage.update.side_effect = [None, Exception("DB error"), None]

    # Flush should continue despite error
    flushed = await qtable.flush_to_db()

    # Should have tried all three
    assert qtable.storage.update.call_count == 3

    # Flushed count should be 2 (excluding the failed one)
    assert flushed == 2
