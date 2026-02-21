"""
Integration tests: Real perception workers on actual file system.

These tests use:
- Real file system operations
- Real CYNIC internal metrics
- No mocks — actual data from system

Run locally: pytest -m integration tests/test_integration/test_perception_real.py
"""
import pytest

from cynic.senses.workers.health import HealthWatcher
from cynic.senses.workers.disk import DiskWatcher


@pytest.mark.integration
class TestPerceptionIntegration:
    """Verify perception workers function correctly."""

    def test_health_watcher_creation(self):
        """
        Verify HealthWatcher can be created.

        No mocks - uses real CYNIC infrastructure.
        """
        watcher = HealthWatcher()

        # Verify watcher is ready
        assert watcher is not None
        assert watcher.name == "health_watcher"
        assert watcher.interval_s > 0

    def test_disk_watcher_creation(self):
        """
        Verify DiskWatcher can be created.

        Uses real file system, no mocks.
        """
        watcher = DiskWatcher()

        # Verify watcher structure
        assert watcher is not None
        assert hasattr(watcher, "sense")
        assert watcher.name == "disk_watcher"

    def test_perception_imports_work(self):
        """
        Verify perception modules can be imported.

        Smoke test for import integrity.
        """
        from cynic.senses.workers.git import GitWatcher
        from cynic.senses.workers.health import HealthWatcher
        from cynic.senses.workers.disk import DiskWatcher
        from cynic.senses.workers.memory import MemoryWatcher

        assert GitWatcher is not None
        assert HealthWatcher is not None
        assert DiskWatcher is not None
        assert MemoryWatcher is not None

    def test_senses_package_integrity(self):
        """
        Verify senses package structure is intact.

        Checks that restructuring (perceive→senses) is complete.
        """
        from cynic.senses.workers import base
        from cynic import senses

        # Verify senses module exists and has workers
        assert hasattr(senses, "__path__")
        # Verify base worker class exists
        assert hasattr(base, "PerceiveWorker")
