"""Tests for 7-layer consciousness ecosystem endpoints.

Test-driven development:
1. Write failing tests
2. Implement endpoints
3. All tests pass
4. Refine for modularity
"""
import pytest
from fastapi.testclient import TestClient

from cynic.api.server import app


class TestEcosystemEndpoints:
    """Test all 7 consciousness ecosystem endpoints."""

    def test_get_ecosystem_state(self):
        """GET /api/consciousness/ecosystem returns cross-bus topology."""
        with TestClient(app) as client:
            response = client.get("/api/consciousness/ecosystem")
            assert response.status_code == 200
            data = response.json()
            assert "core_events" in data
            assert "automation_events" in data
            assert "agent_events" in data
            assert "timestamp" in data
            assert isinstance(data["core_events"], list)
            assert isinstance(data["automation_events"], list)
            assert isinstance(data["agent_events"], list)

    def test_get_perception_sources(self):
        """GET /api/consciousness/perception-sources returns perceive worker activity."""
        with TestClient(app) as client:
            response = client.get("/api/consciousness/perception-sources")
            assert response.status_code == 200
            data = response.json()
            assert isinstance(data, dict)

    def test_get_decision_trace(self):
        """GET /api/consciousness/decision-trace/{id} traces decision through guardrails."""
        with TestClient(app) as client:
            response = client.get("/api/consciousness/decision-trace/test_123")
            assert response.status_code == 200
            data = response.json()
            assert "decision_id" in data
            assert "path" in data
            assert "timestamp" in data
            assert isinstance(data["path"], list)

    def test_get_topology(self):
        """GET /api/consciousness/topology returns architecture consciousness."""
        with TestClient(app) as client:
            response = client.get("/api/consciousness/topology")
            assert response.status_code == 200
            data = response.json()
            assert "source_changes_detected" in data
            assert "topology_deltas_computed" in data
            assert "convergence_validations" in data
            assert "timestamp" in data

    def test_get_nervous_system(self):
        """GET /api/consciousness/nervous-system returns audit trail."""
        with TestClient(app) as client:
            response = client.get("/api/consciousness/nervous-system")
            assert response.status_code == 200
            data = response.json()
            assert "all_events" in data
            assert "decision_reasons" in data
            assert "loop_integrity_checks" in data
            assert "event_count" in data
            assert "decision_count" in data
            assert "timestamp" in data

    def test_get_self_awareness(self):
        """GET /api/consciousness/self-awareness returns organism's meta-cognition."""
        with TestClient(app) as client:
            response = client.get("/api/consciousness/self-awareness")
            assert response.status_code == 200
            data = response.json()
            assert "kernel_observations" in data
            assert "meta_insights" in data
            assert "improvement_proposals" in data
            assert "self_assessment" in data
            assert "timestamp" in data

    def test_get_guardrails(self):
        """GET /api/consciousness/guardrails returns guardian decisions."""
        with TestClient(app) as client:
            response = client.get("/api/consciousness/guardrails")
            assert response.status_code == 200
            data = response.json()
            assert isinstance(data, list)
