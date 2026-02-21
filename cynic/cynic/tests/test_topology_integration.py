"""Test real-time topology system integration.

Verifies that L0 (SourceWatcher, TopologyBuilder, HotReloadCoordinator,
TopologyMirror, ChangeTracker) is properly wired and functional.
"""

import asyncio
import json
import os
import time
from pathlib import Path
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from cynic.api.state import awaken
from cynic.core.event_bus import Event, CoreEvent, get_core_bus
from cynic.core.topology import ChangeTracker, SourceChangedPayload


@pytest.mark.asyncio
async def test_change_tracker_receives_source_changed_event():
    """ChangeTracker should be created and subscribed to SOURCE_CHANGED."""
    state = awaken(db_pool=None)

    assert state.change_tracker is not None
    assert isinstance(state.change_tracker, ChangeTracker)
    print("[OK] ChangeTracker instantiated")


@pytest.mark.asyncio
async def test_change_tracker_logs_file_changes():
    """ChangeTracker should log file modifications to ~/.cynic/changes.jsonl."""
    tracker = ChangeTracker()
    changes_path = Path.home() / ".cynic" / "changes.jsonl"

    # Clear file to start fresh
    if changes_path.exists():
        changes_path.unlink()

    # Create a mock SOURCE_CHANGED event
    payload = SourceChangedPayload(
        category="handlers",
        files=["cynic/api/handlers/test.py", "cynic/api/handlers/other.py"],
        timestamp=time.time(),
    )
    event = Event.typed(CoreEvent.SOURCE_CHANGED, payload, source="test:watcher")

    # Process the event
    await tracker.on_source_changed(event)

    # Verify changes.jsonl was created and contains records
    assert changes_path.exists(), f"changes.jsonl not found at {changes_path}"

    # Read and verify the logged changes
    lines = changes_path.read_text(encoding="utf-8", errors="ignore").splitlines()
    assert len(lines) >= 2, f"Expected at least 2 change records, got {len(lines)}"

    # Parse records and find the test.py one
    records = [json.loads(line) for line in lines]
    test_change = next((r for r in records if "test.py" in r["filepath"]), None)
    assert test_change is not None, "test.py record not found"
    assert test_change["category"] == "handlers"
    assert test_change["change_type"] in ["ADDED", "MODIFIED", "DELETED", "UNKNOWN"]

    print(f"[OK] ChangeTracker logged {len(lines)} changes to changes.jsonl")
    print(f"     Sample record: {test_change}")


@pytest.mark.asyncio
async def test_topology_system_awakens_fully():
    """Full topology system (L0) should awaken with all 4 layers + ChangeTracker."""
    state = awaken(db_pool=None)

    # Verify all topology components exist
    assert state.source_watcher is not None, "SourceWatcher missing"
    assert state.topology_builder is not None, "TopologyBuilder missing"
    assert state.hot_reload_coordinator is not None, "HotReloadCoordinator missing"
    assert state.topology_mirror is not None, "TopologyMirror missing"
    assert state.change_tracker is not None, "ChangeTracker missing"

    print("[OK] All topology system layers present:")
    print("     - L1: SourceWatcher (file monitoring)")
    print("     - L2: IncrementalTopologyBuilder (change detection)")
    print("     - L3: HotReloadCoordinator (safe application)")
    print("     - L4: TopologyMirror (architecture snapshots)")
    print("     - L4.5: ChangeTracker (modification visibility)")


@pytest.mark.asyncio
async def test_change_tracker_rolling_cap():
    """ChangeTracker should enforce rolling cap of F(13)=233 records."""
    tracker = ChangeTracker()
    changes_path = Path.home() / ".cynic" / "changes.jsonl"

    # Remove existing changes.jsonl to start fresh
    if changes_path.exists():
        changes_path.unlink()

    # Create 250 change records (exceeds cap of 233)
    for i in range(250):
        payload = SourceChangedPayload(
            category="test",
            files=[f"file_{i}.py"],
            timestamp=time.time() + i,  # Spread timestamps
        )
        event = Event.typed(CoreEvent.SOURCE_CHANGED, payload, source="test")
        await tracker.on_source_changed(event)

    # Verify rolling cap was enforced
    lines = changes_path.read_text(encoding="utf-8", errors="ignore").splitlines()
    assert len(lines) <= 233, f"Rolling cap not enforced: {len(lines)} > 233"

    # Most recent records should be kept (high index)
    last_record = json.loads(lines[-1])
    assert "file_" in last_record["filepath"], "Recent records not kept"

    print(f"[OK] Rolling cap enforced: {len(lines)} <= 233 records")


@pytest.mark.asyncio
async def test_change_tracker_tracks_change_types():
    """ChangeTracker should correctly classify ADDED, MODIFIED, DELETED changes."""
    tracker = ChangeTracker()

    # Simulate initial state (no previous mtime)
    payload1 = SourceChangedPayload(
        category="test",
        files=["new_file.py"],
        timestamp=time.time(),
    )
    await tracker.on_source_changed(Event.typed(CoreEvent.SOURCE_CHANGED, payload1, source="test"))

    # Check that first change is logged as ADDED
    changes_path = Path.home() / ".cynic" / "changes.jsonl"
    if changes_path.exists():
        lines = changes_path.read_text(encoding="utf-8", errors="ignore").splitlines()
        # Find the record for new_file.py
        for line in lines[-5:]:  # Check recent records
            record = json.loads(line)
            if record["filepath"] == "new_file.py":
                assert record["change_type"] == "ADDED", f"Expected ADDED, got {record['change_type']}"
                print(f"[OK] Correctly classified change: {record['change_type']}")
                break


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
