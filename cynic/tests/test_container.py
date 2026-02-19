"""
CYNIC DependencyContainer Tests — Phase 6

Tests the lightweight DI container.
"""
from __future__ import annotations

import pytest

from cynic.core.config import CynicConfig
from cynic.core.container import DependencyContainer


class TestDependencyContainer:
    """DependencyContainer should resolve, detect cycles, and report."""

    def test_register_and_get(self):
        """Basic register + get round-trip."""
        c = DependencyContainer()
        c.register(str, "hello")
        assert c.get(str) == "hello"

    def test_get_missing_raises_key_error(self):
        """Missing component should raise KeyError."""
        c = DependencyContainer()
        with pytest.raises(KeyError, match="No component registered"):
            c.get(int)

    def test_factory_lazy_creation(self):
        """Factory should be called lazily on first get()."""
        call_count = 0

        def factory(container: DependencyContainer) -> str:
            nonlocal call_count
            call_count += 1
            return "lazy-value"

        c = DependencyContainer()
        c.register_factory(str, factory)
        assert call_count == 0
        result = c.get(str)
        assert result == "lazy-value"
        assert call_count == 1
        # Second get should use cached instance
        c.get(str)
        assert call_count == 1

    def test_factory_receives_container(self):
        """Factory should receive the container for dependency resolution."""
        c = DependencyContainer()
        c.register(int, 42)
        c.register_factory(str, lambda cont: f"value-{cont.get(int)}")
        assert c.get(str) == "value-42"

    def test_circular_dependency_detection(self):
        """Circular factory deps should raise RuntimeError."""
        c = DependencyContainer()
        c.register_factory(int, lambda cont: cont.get(str))  # int needs str
        c.register_factory(str, lambda cont: str(cont.get(int)))  # str needs int
        with pytest.raises(RuntimeError, match="Circular dependency"):
            c.get(int)

    def test_has(self):
        """has() should check both instances and factories."""
        c = DependencyContainer()
        assert c.has(str) is False
        c.register(str, "x")
        assert c.has(str) is True
        c.register_factory(int, lambda c: 1)
        assert c.has(int) is True

    def test_registered_types(self):
        """registered_types should list all registered type names."""
        c = DependencyContainer()
        c.register(str, "x")
        c.register_factory(int, lambda c: 1)
        types = c.registered_types
        assert "str" in types
        assert "int" in types

    def test_config_passed_through(self):
        """Config should be accessible from container."""
        config = CynicConfig(port=9999)
        c = DependencyContainer(config)
        assert c.config.port == 9999

    def test_stats(self):
        """stats() should return container health info."""
        c = DependencyContainer()
        c.register(str, "x")
        c.register_factory(int, lambda c: 1)
        s = c.stats()
        assert s["instances"] == 1
        assert s["factories"] == 1
        assert "str" in s["types"]

    def test_instance_overrides_factory(self):
        """Direct register should override any factory."""
        c = DependencyContainer()
        c.register_factory(str, lambda c: "from-factory")
        c.register(str, "direct")
        assert c.get(str) == "direct"

    def test_real_types_register(self):
        """Should work with actual CYNIC types."""
        from cynic.learning.qlearning import QTable
        from cynic.core.escore import EScoreTracker

        c = DependencyContainer()
        qt = QTable()
        et = EScoreTracker()
        c.register(QTable, qt)
        c.register(EScoreTracker, et)
        assert c.get(QTable) is qt
        assert c.get(EScoreTracker) is et
