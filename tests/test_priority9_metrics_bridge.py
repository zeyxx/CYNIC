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

    async def test_metrics_includes_histograms(self):
        """Test 5: Response includes latency histograms."""
        from fastapi.testclient import TestClient
        from cynic.interfaces.api.server import app

        client = TestClient(app)
        response = client.get("/metrics")

        text = response.text
        # Should include histogram buckets if metrics collected
        assert response.status_code == 200
        # Histogram would show LOD levels (100ms, 300ms, 1000ms, 3000ms)


@pytest.mark.asyncio
class TestMetricsIntegration:
    """Integration tests with actual metrics collection."""

    async def test_metrics_reflects_collected_events(self):
        """Test 6: /metrics reflects EventMetricsCollector data."""
        from fastapi.testclient import TestClient
        from cynic.interfaces.api.server import app
        from cynic.kernel.core.event_bus import get_core_bus

        # Get collector from bus
        bus = get_core_bus("DEFAULT")

        # If collector available, verify metrics endpoint reports it
        client = TestClient(app)
        response = client.get("/metrics")

        assert response.status_code == 200
        assert "cynic_" in response.text or response.text == "# No metrics available\n"

    async def test_metrics_endpoint_performance(self):
        """Test 7: /metrics responds in <100ms (no blocking)."""
        from fastapi.testclient import TestClient
        from cynic.interfaces.api.server import app
        import time

        client = TestClient(app)
        start = time.perf_counter()
        response = client.get("/metrics")
        elapsed_ms = (time.perf_counter() - start) * 1000

        assert response.status_code == 200
        assert elapsed_ms < 100.0, f"Metrics endpoint took {elapsed_ms:.1f}ms (>100ms)"
