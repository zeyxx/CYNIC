"""
Tests for CYNIC Health Dashboard (TUI visualizer).
"""
from __future__ import annotations

import pytest
from cynic.cli.dashboard import (
    CYNICDashboard,
    BreathingCheck,
)


class TestBreathingCheck:
    """Test breathing check data structure."""

    def test_breathing_check_ok(self):
        """Test OK status check."""
        check = BreathingCheck(
            name="Process Alive",
            status="OK",
            value="alive",
            threshold="running",
        )
        assert check.status == "OK"
        assert check.name == "Process Alive"
        assert "✓" in check.render()

    def test_breathing_check_warn(self):
        """Test WARN status check."""
        check = BreathingCheck(
            name="Memory Budget",
            status="WARN",
            value="75%",
            threshold="<80%",
        )
        assert check.status == "WARN"
        assert "⚠" in check.render()

    def test_breathing_check_fail(self):
        """Test FAIL status check."""
        check = BreathingCheck(
            name="DB Connected",
            status="FAIL",
            value="disconnected",
            threshold="connected",
        )
        assert check.status == "FAIL"
        assert "✗" in check.render()


@pytest.mark.asyncio
class TestCYNICDashboard:
    """Test dashboard metrics computation."""

    async def test_dashboard_initialization(self):
        """Test dashboard can be created."""
        dashboard = CYNICDashboard()
        assert dashboard.kernel_url == "http://localhost:8000"
        await dashboard.close()

    async def test_breathing_checks_all_ok(self):
        """Test breathing check computation with all OK."""
        dashboard = CYNICDashboard()

        # Mock healthy health response
        # 360 judgments over 3600s = 0.1 events/sec minimum
        health = {
            "status": "alive",
            "uptime_s": 3600,
            "judgments_total": 360,
            "storage": {"surreal": "connected"},
            "dogs": ["ANALYST", "GUARDIAN", "JANITOR"],
            "consciousness": {"last_latency_ms": 100},
            "learning": {"states": 50, "total_updates": 200, "pending_flush": 0},
        }

        # Mock healthy introspect response
        introspect = {
            "φ_self_assessment": {
                "kernel_integrity": 0.95,
                "self_confidence": 0.6,
            }
        }

        checks = await dashboard.compute_breathing_checks(health, introspect)

        # All checks should be OK
        assert len(checks) == 8
        for check in checks:
            assert check.status == "OK", f"{check.name} failed: {check.value} vs {check.threshold}"

        await dashboard.close()

    async def test_breathing_checks_degraded(self):
        """Test breathing check computation with degraded state."""
        dashboard = CYNICDashboard()

        # Mock degraded health response
        health = {
            "status": "degraded",
            "uptime_s": 60,
            "judgments_total": 0,
            "storage": {"surreal": "disconnected"},
            "dogs": [],
            "consciousness": {"last_latency_ms": 5000},
            "learning": {"states": 0, "total_updates": 0, "pending_flush": 0},
        }

        introspect = {
            "φ_self_assessment": {
                "kernel_integrity": 0.2,
                "self_confidence": 0.1,
            }
        }

        checks = await dashboard.compute_breathing_checks(health, introspect)

        # Most checks should fail or warn
        assert len(checks) == 8
        failures = [c for c in checks if c.status in ("FAIL", "WARN")]
        assert len(failures) >= 4, "Expected multiple failures in degraded state"

        await dashboard.close()

    async def test_breathing_checks_process_alive(self):
        """Test Process Alive check."""
        dashboard = CYNICDashboard()

        health_alive = {"status": "alive"}
        introspect = {"φ_self_assessment": {"kernel_integrity": 0.5}}

        checks = await dashboard.compute_breathing_checks(health_alive, introspect)
        process_check = [c for c in checks if c.name == "Process Alive"][0]
        assert process_check.status == "OK"

        health_dead = {"status": "dead"}
        checks = await dashboard.compute_breathing_checks(health_dead, introspect)
        process_check = [c for c in checks if c.name == "Process Alive"][0]
        assert process_check.status == "FAIL"

        await dashboard.close()

    async def test_breathing_checks_db_connected(self):
        """Test DB Connected check."""
        dashboard = CYNICDashboard()

        health_connected = {"storage": {"surreal": "connected"}}
        introspect = {"φ_self_assessment": {"kernel_integrity": 0.5}}

        checks = await dashboard.compute_breathing_checks(health_connected, introspect)
        db_check = [c for c in checks if c.name == "DB Connected"][0]
        assert db_check.status == "OK"

        health_disconnected = {"storage": {"surreal": "disconnected"}}
        checks = await dashboard.compute_breathing_checks(health_disconnected, introspect)
        db_check = [c for c in checks if c.name == "DB Connected"][0]
        assert db_check.status == "FAIL"

        await dashboard.close()

    async def test_breathing_checks_dogs_active(self):
        """Test Dogs Active check."""
        dashboard = CYNICDashboard()

        # Test with 0 dogs
        health_no_dogs = {"dogs": []}
        introspect = {"φ_self_assessment": {"kernel_integrity": 0.5}}

        checks = await dashboard.compute_breathing_checks(health_no_dogs, introspect)
        dogs_check = [c for c in checks if c.name == "Dogs Active"][0]
        assert dogs_check.status == "FAIL"

        # Test with 1 dog
        health_one_dog = {"dogs": ["ANALYST"]}
        checks = await dashboard.compute_breathing_checks(health_one_dog, introspect)
        dogs_check = [c for c in checks if c.name == "Dogs Active"][0]
        assert dogs_check.status == "WARN"

        # Test with 3+ dogs
        health_three_dogs = {"dogs": ["ANALYST", "GUARDIAN", "JANITOR"]}
        checks = await dashboard.compute_breathing_checks(health_three_dogs, introspect)
        dogs_check = [c for c in checks if c.name == "Dogs Active"][0]
        assert dogs_check.status == "OK"

        await dashboard.close()

    async def test_breathing_checks_qtable_health(self):
        """Test Q-Table states check."""
        dashboard = CYNICDashboard()

        # Test with 0 states
        health_cold = {"learning": {"states": 0, "total_updates": 0, "pending_flush": 0}}
        introspect = {"φ_self_assessment": {"kernel_integrity": 0.5}}

        checks = await dashboard.compute_breathing_checks(health_cold, introspect)
        qtable_check = [c for c in checks if c.name == "Q-Table States"][0]
        assert qtable_check.status == "FAIL"

        # Test with 1-10 states
        health_warm = {"learning": {"states": 5, "total_updates": 10, "pending_flush": 0}}
        checks = await dashboard.compute_breathing_checks(health_warm, introspect)
        qtable_check = [c for c in checks if c.name == "Q-Table States"][0]
        assert qtable_check.status == "WARN"

        # Test with 10+ states
        health_hot = {"learning": {"states": 50, "total_updates": 100, "pending_flush": 0}}
        checks = await dashboard.compute_breathing_checks(health_hot, introspect)
        qtable_check = [c for c in checks if c.name == "Q-Table States"][0]
        assert qtable_check.status == "OK"

        await dashboard.close()

    async def test_breathing_checks_latency(self):
        """Test Judgment Latency check."""
        dashboard = CYNICDashboard()

        health_fast = {"consciousness": {"last_latency_ms": 100}}
        introspect = {"φ_self_assessment": {"kernel_integrity": 0.5}}

        checks = await dashboard.compute_breathing_checks(health_fast, introspect)
        latency_check = [c for c in checks if c.name == "Judgment Latency"][0]
        assert latency_check.status == "OK"

        # Test warn threshold (2-5s)
        health_slow = {"consciousness": {"last_latency_ms": 3000}}
        checks = await dashboard.compute_breathing_checks(health_slow, introspect)
        latency_check = [c for c in checks if c.name == "Judgment Latency"][0]
        assert latency_check.status == "WARN"

        # Test fail threshold (>5s)
        health_very_slow = {"consciousness": {"last_latency_ms": 6000}}
        checks = await dashboard.compute_breathing_checks(health_very_slow, introspect)
        latency_check = [c for c in checks if c.name == "Judgment Latency"][0]
        assert latency_check.status == "FAIL"

        await dashboard.close()
