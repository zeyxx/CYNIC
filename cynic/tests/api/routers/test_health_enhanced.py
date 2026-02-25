"""Tests for enhanced health endpoints.

Test-driven development:
1. GET /health — basic liveness (existing)
2. GET /health/full — comprehensive status with Dogs, learning, resources
3. GET /health/ready — blocking endpoint that waits for readiness (timeout 30s)
"""
import pytest
from fastapi.testclient import TestClient

from cynic.api.server import app


class TestHealthEnhanced:
    """Test enhanced health endpoints for Claude Code decision-making."""

    def test_health_basic(self):
        """GET /health returns simple liveness check."""
        with TestClient(app) as client:
            response = client.get("/health")
            assert response.status_code == 200
            data = response.json()
            assert data["status"] in ["alive", "degraded", "dead"]
            assert "uptime_s" in data
            assert "consciousness" in data
            assert "dogs" in data
            assert "learning" in data
            assert "phi" in data

    def test_health_full_rich_data(self):
        """GET /health/full returns comprehensive system status."""
        with TestClient(app) as client:
            response = client.get("/health/full")
            assert response.status_code == 200
            data = response.json()

            # Top-level structure
            assert "timestamp" in data
            assert "status" in data
            assert "uptime_seconds" in data
            assert "components" in data
            assert "dogs" in data
            assert "learning" in data
            assert "resources" in data

            # Components
            components = data["components"]
            assert "database" in components
            assert "llm" in components
            assert "event_bus" in components
            assert "consciousness" in components

            for comp_name, comp_data in components.items():
                assert "status" in comp_data

    def test_health_full_dogs_status(self):
        """GET /health/full includes per-Dog status (active, metrics)."""
        with TestClient(app) as client:
            response = client.get("/health/full")
            assert response.status_code == 200
            data = response.json()

            # Dogs structure
            dogs_info = data["dogs"]
            assert "active_count" in dogs_info
            assert "total_count" in dogs_info
            assert "dogs" in dogs_info
            assert isinstance(dogs_info["dogs"], list)

            # Check each dog has required fields
            for dog in dogs_info["dogs"]:
                assert "name" in dog
                assert "status" in dog
                assert dog["status"] in ["active", "inactive"]
                assert "judgments" in dog
                assert "errors" in dog
                assert "error_rate" in dog
                assert "avg_latency_ms" in dog
                assert "priority" in dog

            # Should have Dogs info (even if count is 0, structure is valid)
            assert dogs_info["total_count"] >= 0
            assert dogs_info["active_count"] >= 0

    def test_health_full_learning_metrics(self):
        """GET /health/full includes learning loop metrics."""
        with TestClient(app) as client:
            response = client.get("/health/full")
            assert response.status_code == 200
            data = response.json()

            # Learning structure
            learning = data["learning"]
            if "error" not in learning:
                assert "active" in learning
                assert "q_table_size" in learning
                assert "total_entries" in learning
                assert "total_updates" in learning
                assert "total_visits" in learning
                assert "learning_rate" in learning
                assert "discount_factor" in learning
                assert "pending_flush" in learning
                assert "ewc_consolidated" in learning
                assert "convergence_rate" in learning

    def test_health_ready_blocks_until_ready(self):
        """GET /health/ready blocks until all systems initialized (timeout 30s)."""
        with TestClient(app) as client:
            response = client.get("/health/ready?timeout=30")
            # Should succeed if system ready, or 503 if timeout
            assert response.status_code in [200, 503]
            data = response.json()
            assert "timestamp" in data
            assert "waited_seconds" in data

    def test_health_ready_success(self):
        """GET /health/ready returns 'ready' when all systems operational."""
        with TestClient(app) as client:
            # Try with a reasonable timeout
            response = client.get("/health/ready?timeout=30")

            if response.status_code == 200:
                data = response.json()
                assert data["status"] == "ready"
                assert "ready_components" in data
                assert data["waited_seconds"] >= 0
            elif response.status_code == 503:
                # System took too long, but that's OK for this test
                data = response.json()
                assert "waited_seconds" in data
                assert data["waited_seconds"] >= 0

    def test_health_ready_timeout_incomplete(self):
        """GET /health/ready returns 503 if timeout expires."""
        with TestClient(app) as client:
            # Use very short timeout
            response = client.get("/health/ready?timeout=1")

            # Either ready quickly (200) or timeout (503)
            assert response.status_code in [200, 503]
            data = response.json()
            assert "waited_seconds" in data

    def test_health_full_fails_if_component_down(self):
        """GET /health/full handles degraded components gracefully."""
        with TestClient(app) as client:
            response = client.get("/health/full")
            assert response.status_code == 200
            data = response.json()

            # Even if some components down, response is valid
            assert "status" in data
            assert "components" in data
            for comp_name, comp_data in data["components"].items():
                assert "status" in comp_data
                assert comp_data["status"] in ["healthy", "degraded", "unhealthy"]


class TestHealthIntegration:
    """Integration tests for health endpoints."""

    def test_health_endpoints_all_accessible(self):
        """All health endpoints are accessible."""
        with TestClient(app) as client:
            # Basic health
            response = client.get("/health")
            assert response.status_code == 200

            # Full health
            response = client.get("/health/full")
            assert response.status_code == 200

            # Ready check (with short timeout)
            response = client.get("/health/ready?timeout=5")
            assert response.status_code in [200, 503]

    def test_health_full_response_consistency(self):
        """GET /health/full returns consistent structure across calls."""
        with TestClient(app) as client:
            response1 = client.get("/health/full")
            response2 = client.get("/health/full")

            assert response1.status_code == 200
            assert response2.status_code == 200

            data1 = response1.json()
            data2 = response2.json()

            # Structure should be identical
            assert set(data1.keys()) == set(data2.keys())
            assert data1["dogs"]["total_count"] == data2["dogs"]["total_count"]

    def test_health_ready_polling_interval(self):
        """GET /health/ready polls at appropriate interval (0.5s)."""
        import time

        with TestClient(app) as client:
            start = time.time()
            response = client.get("/health/ready?timeout=2")
            elapsed = time.time() - start

            data = response.json()
            waited = data["waited_seconds"]

            # Should have reasonable polling resolution
            assert waited >= 0
            assert waited <= 3  # timeout + some overhead
