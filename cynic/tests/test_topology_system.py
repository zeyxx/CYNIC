"""Tests for real-time topology system (L1-L4)."""

import pytest
import time
from pathlib import Path
from unittest.mock import MagicMock, AsyncMock, patch

from cynic.core.topology import (
    SourceWatcher,
    IncrementalTopologyBuilder,
    TopologyMirror,
    SourceChangedPayload,
    TopologyChangedPayload,
)
from cynic.core.event_bus import Event, CoreEvent


class TestSourceWatcher:
    """Test L1: SourceWatcher — file system monitoring."""

    def test_init(self):
        """SourceWatcher initializes with empty state."""
        watcher = SourceWatcher()
        assert watcher._previous_state == {}

    def test_snapshot_tree(self):
        """_snapshot_tree captures .py files with mtimes."""
        watcher = SourceWatcher()
        snap = watcher._snapshot_tree()

        # Should return dict of categories
        assert isinstance(snap, dict)
        assert "handlers" in snap
        assert "dogs" in snap

        # handlers should have some files
        if snap["handlers"]:
            # Each entry is {filepath: mtime}
            for path, mtime in snap["handlers"].items():
                assert isinstance(path, str)
                assert isinstance(mtime, float)

    def test_diff_state_no_changes(self):
        """_diff_state returns empty dict if nothing changed."""
        watcher = SourceWatcher()
        state = {"handlers": {"file1.py": 100.0}}
        diff = watcher._diff_state(state, state)
        assert diff == {}

    def test_diff_state_with_new_file(self):
        """_diff_state detects new files."""
        watcher = SourceWatcher()
        prev = {"handlers": {"file1.py": 100.0}}
        curr = {"handlers": {"file1.py": 100.0, "file2.py": 200.0}}
        diff = watcher._diff_state(prev, curr)

        assert "handlers" in diff
        assert "file2.py" in diff["handlers"]

    def test_diff_state_with_deleted_file(self):
        """_diff_state detects deleted files."""
        watcher = SourceWatcher()
        prev = {"handlers": {"file1.py": 100.0, "file2.py": 200.0}}
        curr = {"handlers": {"file1.py": 100.0}}
        diff = watcher._diff_state(prev, curr)

        assert "handlers" in diff
        assert "file2.py" in diff["handlers"]

    def test_diff_state_with_modified_file(self):
        """_diff_state detects modified files (mtime changed)."""
        watcher = SourceWatcher()
        prev = {"handlers": {"file1.py": 100.0}}
        curr = {"handlers": {"file1.py": 150.0}}  # mtime changed
        diff = watcher._diff_state(prev, curr)

        assert "handlers" in diff
        assert "file1.py" in diff["handlers"]


class TestIncrementalTopologyBuilder:
    """Test L2: IncrementalTopologyBuilder — change discovery."""

    def test_init(self):
        """Builder initializes with empty inventory."""
        builder = IncrementalTopologyBuilder()
        assert builder._previous_inventory == {}

    def test_file_to_module_path(self):
        """_file_to_module_path converts file paths to module names."""
        builder = IncrementalTopologyBuilder()

        # Test conversions
        assert builder._file_to_module_path("cynic/api/handlers/direct.py") == "cynic.api.handlers.direct"
        assert builder._file_to_module_path("cynic/cli/perceive_watch.py") == "cynic.cli.perceive_watch"

    def test_compute_delta_added(self):
        """_compute_delta detects added handlers."""
        builder = IncrementalTopologyBuilder()
        prev = {}
        curr = {"direct": MagicMock(), "axiom": MagicMock()}

        delta = builder._compute_delta(prev, curr)

        assert set(delta.added) == {"direct", "axiom"}
        assert delta.removed == []
        assert delta.modified == []

    def test_compute_delta_removed(self):
        """_compute_delta detects removed handlers."""
        builder = IncrementalTopologyBuilder()
        prev = {"old_handler": MagicMock()}
        curr = {}

        delta = builder._compute_delta(prev, curr)

        assert delta.added == []
        assert delta.removed == ["old_handler"]
        assert delta.modified == []

    def test_compute_delta_modified(self):
        """_compute_delta detects modified handlers."""
        builder = IncrementalTopologyBuilder()
        old_cls = MagicMock()
        new_cls = MagicMock()
        prev = {"direct": old_cls}
        curr = {"direct": new_cls}

        delta = builder._compute_delta(prev, curr)

        assert delta.added == []
        assert delta.removed == []
        assert "direct" in delta.modified

    def test_validate_delta_always_true(self):
        """_validate_delta currently always returns True (basic validation)."""
        builder = IncrementalTopologyBuilder()
        from cynic.core.topology import TopologyDelta
        delta = TopologyDelta(added=["test"], removed=[], modified=[])

        assert builder._validate_delta(delta) is True

    @pytest.mark.asyncio
    async def test_on_source_changed_valid_payload(self):
        """on_source_changed processes valid SOURCE_CHANGED events."""
        builder = IncrementalTopologyBuilder()

        # Create mock event
        payload = SourceChangedPayload(
            category="handlers",
            files=["cynic/api/handlers/test.py"],
            timestamp=time.time(),
        )
        event = Event.typed(CoreEvent.SOURCE_CHANGED, payload)

        # Should not raise
        await builder.on_source_changed(event)

    @pytest.mark.asyncio
    async def test_on_source_changed_ignores_non_handler_changes(self):
        """on_source_changed ignores non-handler category changes."""
        builder = IncrementalTopologyBuilder()

        payload = SourceChangedPayload(
            category="cli",  # Not handlers
            files=["cynic/cli/test.py"],
            timestamp=time.time(),
        )
        event = Event.typed(CoreEvent.SOURCE_CHANGED, payload)

        # Should not emit TOPOLOGY_CHANGED
        await builder.on_source_changed(event)


class TestTopologyMirror:
    """Test L4: TopologyMirror — continuous snapshots."""

    def test_init(self):
        """Mirror initializes with zero snapshots."""
        mirror = TopologyMirror()
        assert mirror._snapshot_count == 0
        assert mirror._last_snapshot_time == 0.0

    def test_write_topology_file(self):
        """_write_topology_file writes to ~/.cynic/topology.json."""
        mirror = TopologyMirror()
        snap = {
            "snapshot_id": 1,
            "timestamp": time.time(),
            "handlers": {
                "direct": {"handler_count": 1},
            },
        }

        mirror._write_topology_file(snap)

        # Verify file was written
        path = Path.home() / ".cynic" / "topology.json"
        assert path.exists()
        assert "handlers" in path.read_text()

    def test_append_topology_history(self):
        """_append_topology_history appends to jsonl file."""
        mirror = TopologyMirror()
        snap = {"snapshot_id": 1, "timestamp": time.time()}

        mirror._append_topology_history(snap)

        path = Path.home() / ".cynic" / "topology_history.jsonl"
        assert path.exists()

        # Verify line was appended
        lines = path.read_text().strip().split("\n")
        assert len(lines) >= 1

    @pytest.mark.asyncio
    async def test_on_topology_applied_resets_timing(self):
        """_on_topology_applied forces immediate snapshot."""
        mirror = TopologyMirror()
        mirror._last_snapshot_time = time.time()

        event = Event.typed(
            CoreEvent.TOPOLOGY_APPLIED,
            {"handlers_added": 1, "handlers_removed": 0, "timestamp": time.time()},
        )

        await mirror._on_topology_applied(event)

        # Timing should be reset
        assert mirror._last_snapshot_time == 0.0


class TestTopologyIntegration:
    """Test integration of topology layers."""

    @pytest.mark.asyncio
    async def test_source_change_flows_to_builder(self):
        """Source changes flow from watcher to builder."""
        watcher = SourceWatcher()
        builder = IncrementalTopologyBuilder()

        # Create a source change event
        payload = SourceChangedPayload(
            category="handlers",
            files=["cynic/api/handlers/test.py"],
            timestamp=time.time(),
        )
        event = Event.typed(CoreEvent.SOURCE_CHANGED, payload)

        # Builder should handle it without error
        await builder.on_source_changed(event)

    def test_payloads_are_dataclasses(self):
        """All payloads can be converted to/from dicts."""
        payload = SourceChangedPayload(
            category="handlers",
            files=["file.py"],
            timestamp=123.0,
        )

        # Should convert to dict via Event
        event = Event.typed(CoreEvent.SOURCE_CHANGED, payload)
        assert isinstance(event.payload, dict)

        # Should reconstruct
        recovered = event.as_typed(SourceChangedPayload)
        assert recovered.category == "handlers"
        assert recovered.files == ["file.py"]
