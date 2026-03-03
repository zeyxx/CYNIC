"""Tests for 7-layer consciousness ecosystem endpoints.

Test-driven development:
1. Write failing tests
2. Implement endpoints
3. All tests pass
4. Refine for modularity
"""

import pytest

pytestmark = pytest.mark.skip(
    reason="Old architecture: module imports not available in V5"
)

# Block all imports that would fail
pytest.skip("Skipping old architecture test module", allow_module_level=True)

import pytest

from fastapi.testclient import TestClient

from cynic.interfaces.api.server import app


@pytest.fixture(scope="class")
def ecosystemendpoints_client():
    """Class-scoped HTTP client - reuses single organism."""
    with TestClient(app) as c:
        yield c


class TestEcosystemEndpoints:
    """Test all 7 consciousness ecosystem endpoints."""

    def test_get_ecosystem_state(self, ecosystemendpoints_client):
        """GET /api/consciousness/ecosystem returns cross-bus topology."""
        response = ecosystemendpoints_client.get("/api/consciousness/ecosystem")
        assert response.status_code == 200
        data = response.json()
        assert "core_events" in data
        assert "automation_events" in data
        assert "agent_events" in data
        assert "timestamp" in data
        assert isinstance(data["core_events"], list)
        assert isinstance(data["automation_events"], list)
        assert isinstance(data["agent_events"], list)

    def test_get_perception_sources(self, ecosystemendpoints_client):
        """GET /api/consciousness/perception-sources returns perceive worker activity."""
        response = ecosystemendpoints_client.get(
            "/api/consciousness/perception-sources"
        )
        assert response.status_code == 200
        data = response.json()
        assert isinstance(data, dict)

    def test_get_decision_trace(self, ecosystemendpoints_client):
        """GET /api/consciousness/decision-trace/{id} traces decision through guardrails."""
        response = ecosystemendpoints_client.get(
            "/api/consciousness/decision-trace/test_123"
        )
        assert response.status_code == 200
        data = response.json()
        assert "decision_id" in data
        assert "path" in data
        assert "timestamp" in data
        assert isinstance(data["path"], list)

    def test_get_topology(self, ecosystemendpoints_client):
        """GET /api/consciousness/topology returns architecture consciousness."""
        response = ecosystemendpoints_client.get("/api/consciousness/topology")
        assert response.status_code == 200
        data = response.json()
        assert "source_changes_detected" in data
        assert "topology_deltas_computed" in data
        assert "convergence_validations" in data
        assert "timestamp" in data

    def test_get_nervous_system(self, ecosystemendpoints_client):
        """GET /api/consciousness/nervous-system returns audit trail."""
        response = ecosystemendpoints_client.get("/api/consciousness/nervous-system")
        assert response.status_code == 200
        data = response.json()
        assert "all_events" in data
        assert "decision_reasons" in data
        assert "loop_integrity_checks" in data
        assert "event_count" in data
        assert "decision_count" in data
        assert "timestamp" in data

    def test_get_self_awareness(self, ecosystemendpoints_client):
        """GET /api/consciousness/self-awareness returns organism's meta-cognition."""
        response = ecosystemendpoints_client.get("/api/consciousness/self-awareness")
        assert response.status_code == 200
        data = response.json()
        assert "kernel_observations" in data
        assert "meta_insights" in data
        assert "improvement_proposals" in data
        assert "self_assessment" in data
        assert "timestamp" in data

    def test_get_guardrails(self, ecosystemendpoints_client):
        """GET /api/consciousness/guardrails returns guardian decisions."""
        response = ecosystemendpoints_client.get("/api/consciousness/guardrails")
        assert response.status_code == 200
        data = response.json()
        assert isinstance(data, list)
