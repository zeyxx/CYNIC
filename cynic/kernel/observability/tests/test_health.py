"""
Test suite for Health Checker system.

Tests verify:
- Health check returns correct overall status
- All subsystems are checked (database, llm, consciousness, event_bus)
- Graceful degradation when systems fail
- ISO8601 timestamps included
- Detailed checks with remediation hints
"""

from unittest.mock import AsyncMock, MagicMock

import pytest

from cynic.kernel.observability.health import HealthChecker


class TestHealthChecker:
    """Test HealthChecker functionality."""

    @pytest.mark.asyncio
    async def test_health_check_returns_status(self):
        """Health check should return overall status and subsystem states."""
        health = HealthChecker()
        status = await health.check()

        # Required fields
        assert "timestamp" in status
        assert "overall" in status
        assert "database" in status
        assert "llm" in status
        assert "consciousness" in status
        assert "event_bus" in status
        assert "app" in status

        # overall should be one of the valid values
        assert status["overall"] in ["healthy", "degraded", "critical"]

    @pytest.mark.asyncio
    async def test_health_checks_all_systems(self):
        """Health check should check database, llm, consciousness, event_bus."""
        health = HealthChecker()
        status = await health.check()

        # All major systems should be represented
        assert "database" in status
        assert "llm" in status
        assert "consciousness" in status
        assert "event_bus" in status
        assert "app" in status

        # Status values should be reasonable
        assert isinstance(status["database"], str)
        assert isinstance(status["llm"], str)
        assert isinstance(status["consciousness"], str)
        assert isinstance(status["event_bus"], str)

    @pytest.mark.asyncio
    async def test_health_has_timestamp(self):
        """Health check should include ISO8601 timestamp."""
        health = HealthChecker()
        status = await health.check()

        assert "timestamp" in status
        # Check for ISO8601 format (has T and Z or +)
        timestamp = status["timestamp"]
        assert "T" in timestamp
        assert isinstance(timestamp, str)

    @pytest.mark.asyncio
    async def test_health_status_format(self):
        """Each subsystem should have a status string."""
        health = HealthChecker()
        status = await health.check()

        # overall should be a string
        assert isinstance(status["overall"], str)

        # All status values should be strings with known values
        for key in ["database", "llm", "consciousness", "event_bus"]:
            assert key in status
            assert isinstance(status[key], str)
            assert status[key] in ["ok", "down", "unknown"]

    @pytest.mark.asyncio
    async def test_health_graceful_degradation(self):
        """If optional systems down, should be degraded not critical."""
        # Mock LLM failure (optional)
        mock_registry = MagicMock()
        mock_registry.get_available = MagicMock(return_value=[])

        health = HealthChecker(registry=mock_registry)
        status = await health.check()

        # With LLM down but others ok, should be degraded (not critical)
        # Since no organism/db/buses provided, they default to ok
        assert status["overall"] in ["healthy", "degraded"]

    @pytest.mark.asyncio
    async def test_health_uptime_included(self):
        """Health check should include uptime_s if organism provided."""
        mock_organism = MagicMock()
        mock_organism.uptime_s = 123.456
        mock_organism.kernel_mirror = MagicMock()
        mock_organism.kernel_mirror.snapshot = MagicMock(return_value={"health": 1.0})

        health = HealthChecker(organism=mock_organism)
        status = await health.check()

        assert "uptime_s" in status
        assert isinstance(status["uptime_s"], int | float)
        assert status["uptime_s"] >= 0

    @pytest.mark.asyncio
    async def test_health_check_detailed(self):
        """Detailed health check should include remediation hints."""
        mock_db_pool = AsyncMock()
        mock_db_pool.acquire = AsyncMock(side_effect=RuntimeError("Connection failed"))

        health = HealthChecker(db_pool=mock_db_pool)
        status = await health.check_detailed()

        # Should have hints when systems fail
        if status.get("database") != "ok":
            assert "database_hint" in status
            assert isinstance(status["database_hint"], str)
            assert len(status["database_hint"]) > 0

    @pytest.mark.asyncio
    async def test_health_checker_resilience(self):
        """Health checker should not crash if organism methods fail."""
        mock_organism = MagicMock()
        mock_organism.uptime_s = -1  # Invalid
        mock_organism.kernel_mirror = MagicMock()
        mock_organism.kernel_mirror.snapshot = MagicMock(return_value=None)

        health = HealthChecker(organism=mock_organism)
        # Should not raise an exception
        status = await health.check()

        # Should return a response with overall status
        assert "overall" in status
        assert status["overall"] in ["healthy", "degraded", "critical"]

    @pytest.mark.asyncio
    async def test_health_status_values(self):
        """Status values should only be ok, down, or unknown."""
        health = HealthChecker()
        status = await health.check()

        for key in ["database", "llm", "consciousness", "event_bus"]:
            value = status.get(key)
            assert value in ["ok", "down", "unknown"], f"Invalid status for {key}: {value}"

    @pytest.mark.asyncio
    async def test_health_overall_critical_on_consciousness_fail(self):
        """Overall should be critical if consciousness check fails."""
        mock_organism = MagicMock()
        mock_organism.uptime_s = -999  # Invalid " will cause consciousness check to fail
        mock_organism.kernel_mirror = MagicMock()
        mock_organism.kernel_mirror.snapshot = MagicMock(side_effect=Exception("Snapshot failed"))

        health = HealthChecker(organism=mock_organism)
        status = await health.check()

        # Consciousness check should fail, making overall critical
        assert status["overall"] == "critical"

    @pytest.mark.asyncio
    async def test_health_overall_degraded_on_llm_fail(self):
        """Overall should be degraded if only LLM fails (optional system)."""
        mock_registry = MagicMock()
        mock_registry.get_available = MagicMock(side_effect=Exception("Registry unavailable"))

        # No organism, db, or buses provided (they'll default to ok)
        health = HealthChecker(registry=mock_registry)
        status = await health.check()

        # LLM is optional, so only degraded
        assert status["overall"] == "degraded"
