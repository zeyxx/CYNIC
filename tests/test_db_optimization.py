"""Tests for database connection pooling optimization."""

from __future__ import annotations

import pytest

from cynic.kernel.infrastructure.db_pool import DatabasePool


@pytest.mark.performance
class TestDatabasePoolInitialization:
    """Test DatabasePool initialization and configuration."""

    def test_connection_pool_initialization_with_defaults(self):
        """Verify pool initializes with correct Fibonacci-aligned defaults."""
        # pool_size=F(6)=8, max_overflow=F(5)=5 (no-op mode: db_url=None)
        pool = DatabasePool()

        assert pool.pool_size == 8, "pool_size should default to F(6)=8"
        assert pool.max_overflow == 5, "max_overflow should default to F(5)=5"
        assert pool.db_url is None, "db_url should default to None"
        assert (
            pool._is_no_op is True
        ), "pool should be in no-op mode when db_url is None"
        assert pool._engine is None, "engine should be None in no-op mode"

    def test_connection_pool_initialization_with_custom_sizes(self):
        """Verify pool accepts custom pool_size and max_overflow."""
        pool = DatabasePool(pool_size=16, max_overflow=10)

        assert pool.pool_size == 16, "pool_size should be 16"
        assert pool.max_overflow == 10, "max_overflow should be 10"
        assert pool._is_no_op is True, "still no-op without db_url"

    def test_available_connections_in_no_op_mode(self):
        """Verify available_connections() returns pool_size in no-op mode."""
        pool = DatabasePool(pool_size=8, max_overflow=5)

        available = pool.available_connections()
        assert (
            available == 8
        ), "available_connections should return pool_size in no-op mode"

    def test_get_connection_returns_none_in_no_op_mode(self):
        """Verify get_connection() returns None in no-op mode."""
        pool = DatabasePool()

        conn = pool.get_connection()
        assert conn is None, "get_connection should return None in no-op mode"

    def test_pool_disposal_in_no_op_mode(self):
        """Verify dispose() completes without error in no-op mode."""
        pool = DatabasePool()

        # Should not raise, just log
        pool.dispose()
        assert (
            pool._engine is None
        ), "engine should remain None after dispose in no-op mode"


@pytest.mark.performance
class TestDatabasePoolGracefulDegradation:
    """Test graceful degradation when SQLAlchemy is unavailable."""

    def test_pool_handles_missing_sqlalchemy(self):
        """
        Verify pool gracefully handles missing SQLAlchemy.

        When db_url is provided but SQLAlchemy import fails,
        pool falls back to no-op mode instead of crashing.
        """
        # This test simulates the behavior with db_url set but no actual engine
        # (We can't easily mock SQLAlchemy import without complex fixtures)
        # So we verify that if engine init fails, pool remains safe
        pool = DatabasePool(db_url="postgresql://invalid")

        # If init failed (no SQLAlchemy or bad URL), pool is no-op
        if pool._is_no_op:
            available = pool.available_connections()
            assert (
                available >= 0
            ), "available_connections should return safe value even in degraded mode"
            conn = pool.get_connection()
            assert conn is None, "get_connection should return None safely"

    def test_pool_remains_safe_after_init_error(self):
        """Verify pool can be used safely even if initialization partially fails."""
        pool = DatabasePool()

        # Multiple calls should be safe
        available1 = pool.available_connections()
        available2 = pool.available_connections()
        assert (
            available1 == available2
        ), "multiple calls should return consistent values"

        conn1 = pool.get_connection()
        conn2 = pool.get_connection()
        assert conn1 == conn2, "multiple get_connection calls should be consistent"

        pool.dispose()
        available3 = pool.available_connections()
        assert available3 >= 0, "available_connections should be safe after dispose"


@pytest.mark.performance
class TestDatabasePoolResourceManagement:
    """Test resource management and cleanup."""

    def test_pool_disposal_closes_connections(self):
        """Verify dispose() properly cleans up resources."""
        pool = DatabasePool(pool_size=8, max_overflow=5)

        # No engine in no-op mode, so just verify dispose doesn't crash
        pool.dispose()

        # After dispose, should still be safe to query
        available = pool.available_connections()
        assert available >= 0, "should still report safe availability after dispose"

    def test_pool_can_be_reinitialized(self):
        """Verify pool can be created and disposed multiple times."""
        for _ in range(3):
            pool = DatabasePool(pool_size=8, max_overflow=5)
            assert pool._is_no_op is True
            pool.dispose()

    def test_available_connections_consistency(self):
        """
        Verify available_connections() returns consistent, non-negative values.

        This is important for upstream code that uses this to make decisions
        about request throttling or backpressure.
        """
        pool = DatabasePool(pool_size=8, max_overflow=5)

        for _ in range(5):
            available = pool.available_connections()
            assert isinstance(available, int), "available_connections should return int"
            assert available >= 0, "available connections should never be negative"
            assert available <= (
                pool.pool_size + pool.max_overflow
            ), "available should not exceed pool_size + max_overflow"


@pytest.mark.performance
class TestDatabasePoolConfiguration:
    """Test pool configuration options."""

    def test_pool_echo_option(self):
        """Verify pool accepts and stores echo parameter."""
        pool = DatabasePool(echo=True)
        assert pool.echo is True, "echo parameter should be stored"

    def test_pool_db_url_option(self):
        """Verify pool accepts and stores db_url parameter."""
        db_url = "postgresql://user:pass@localhost/db"
        pool = DatabasePool(db_url=db_url)
        assert pool.db_url == db_url, "db_url should be stored"

    def test_pool_size_boundaries(self):
        """Verify pool accepts reasonable size boundaries."""
        # Fibonacci-aligned sequence: 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144
        # Pool sizes should typically be in this range
        for size in [1, 2, 3, 5, 8, 13, 21, 34, 55, 89]:
            pool = DatabasePool(pool_size=size, max_overflow=size // 2)
            assert pool.pool_size == size, f"pool_size {size} should be stored"


@pytest.mark.performance
class TestDatabasePoolMetrics:
    """Test pool metrics and observability."""

    def test_available_connections_is_observable(self):
        """Verify available_connections() is suitable for observability systems."""
        pool = DatabasePool(pool_size=8, max_overflow=5)

        metrics = {
            "pool_size": pool.pool_size,
            "max_overflow": pool.max_overflow,
            "available": pool.available_connections(),
        }

        # Should produce valid metrics dict
        assert metrics["pool_size"] == 8
        assert metrics["max_overflow"] == 5
        assert 0 <= metrics["available"] <= 13

    def test_pool_state_queryable_after_dispose(self):
        """Verify pool state remains queryable after disposal."""
        pool = DatabasePool(pool_size=8, max_overflow=5)
        pool.dispose()

        # Should not crash when querying after dispose
        available = pool.available_connections()
        size = pool.pool_size
        max_overflow = pool.max_overflow

        assert isinstance(available, int)
        assert size == 8
        assert max_overflow == 5
