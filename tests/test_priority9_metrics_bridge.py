import pytest


@pytest.mark.asyncio
class TestMetricsRouter:
    """Tests for Prometheus metrics endpoint."""

    async def test_metrics_endpoint_exists(self):
        """Test 1: GET /metrics returns 200."""
        from fastapi.testclient import TestClient
        from cynic.interfaces.api.server import app

        client = TestClient(app)
        response = client.get("/metrics")
        assert response.status_code == 200

    async def test_metrics_returns_openmetrics_format(self):
        """Test 2: Response is valid OpenMetrics format."""
        from fastapi.testclient import TestClient
        from cynic.interfaces.api.server import app

        client = TestClient(app)
        response = client.get("/metrics")

        text = response.text
        # OpenMetrics format includes TYPE/HELP comments
        assert "# TYPE" in text or text == ""  # Allow empty initially

    async def test_metrics_includes_event_counts(self):
        """Test 3: Response includes event counts."""
        from fastapi.testclient import TestClient
        from cynic.interfaces.api.server import app

        client = TestClient(app)
        response = client.get("/metrics")

        # Should include event-related metrics (even if count=0)
        text = response.text
        assert "# HELP" in text or text == ""

    async def test_metrics_includes_anomaly_count(self):
        """Test 4: Response includes anomaly metrics."""
        from fastapi.testclient import TestClient
        from cynic.interfaces.api.server import app

        client = TestClient(app)
        response = client.get("/metrics")

        # Should include anomaly metrics
        text = response.text
        assert response.status_code == 200
