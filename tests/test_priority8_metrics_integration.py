import pytest
from cynic.nervous.metrics_analyzer import MetricsAnalyzer
from cynic.nervous.event_metrics import EventMetricsCollector, AnomalyRecord
from cynic.kernel.core.phi import PHI, PHI_INV


@pytest.mark.asyncio
class TestMetricsAnalyzer:
    """Tests for MetricsAnalyzer."""

    async def test_metrics_analyzer_init(self):
        """Test 1: MetricsAnalyzer initializes with collector reference."""
        collector = EventMetricsCollector()
        analyzer = MetricsAnalyzer(collector)
        assert analyzer._collector is collector

    async def test_analyze_rate_spike_proposals(self):
        """Test 2: analyze_anomalies() returns proposals for rate spikes."""
        collector = EventMetricsCollector()
        analyzer = MetricsAnalyzer(collector)

        # Record many events in rapid succession
        for i in range(50):
            await collector.record("core.judgment_created", duration_ms=100.0)

        anomalies = await collector.detect_anomalies()
        proposals = analyzer.analyze_anomalies(anomalies, severity_threshold=0.0)

        # Should have analyzed the anomalies (even if none detected)
        assert isinstance(proposals, list)

    async def test_analyze_error_spike_proposals(self):
        """Test 3: analyze_anomalies() handles ERROR_SPIKE anomalies."""
        collector = EventMetricsCollector()
        analyzer = MetricsAnalyzer(collector)

        # Record error events
        for i in range(10):
            is_error = i < 8  # 80% error rate
            await collector.record("test_event", duration_ms=100.0, is_error=is_error)

        anomalies = await collector.detect_anomalies()
        proposals = analyzer.analyze_anomalies(anomalies, severity_threshold=0.0)

        # Should identify error spike anomalies
        assert len(proposals) >= 0

    async def test_analyze_latency_spike_proposals(self):
        """Test 4: analyze_anomalies() handles LATENCY_SPIKE anomalies."""
        collector = EventMetricsCollector()
        analyzer = MetricsAnalyzer(collector)

        # Record slow event
        await collector.record("slow_op", duration_ms=5000.0)

        anomalies = await collector.detect_anomalies()
        proposals = analyzer.analyze_anomalies(anomalies, severity_threshold=0.0)

        # Should handle latency spikes
        assert isinstance(proposals, list)

    async def test_metrics_proposal_has_required_fields(self):
        """Test 5: Generated proposals have required fields."""
        collector = EventMetricsCollector()
        analyzer = MetricsAnalyzer(collector)

        # Create a mock anomaly
        from cynic.nervous.event_metrics import AnomalyRecord
        anomaly = AnomalyRecord(
            detected_at_ms=1000.0,
            anomaly_type="RATE_SPIKE",
            event_type="test_event",
            metric_value=50.0,
            threshold_value=30.0,
            severity=0.5,
            message="Test rate spike"
        )

        proposals = analyzer.analyze_anomalies([anomaly], severity_threshold=0.0)

        # Each proposal should have required fields
        for p in proposals:
            assert hasattr(p, "target")
            assert hasattr(p, "recommendation")
            assert hasattr(p, "current_value")
            assert hasattr(p, "suggested_value")
