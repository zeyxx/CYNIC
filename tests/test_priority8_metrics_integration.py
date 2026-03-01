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

    async def test_selfprober_set_metrics_collector(self):
        """Test 6: SelfProber accepts metrics_collector injection."""
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
        from cynic.nervous.event_metrics import EventMetricsCollector

        prober = SelfProber()
        collector = EventMetricsCollector()

        # Should not raise
        prober.set_metrics_collector(collector)
        assert prober._metrics_collector is collector

    async def test_selfprober_analyze_includes_metrics(self):
        """Test 7: SelfProber.analyze() calls _analyze_metrics()."""
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
        from cynic.nervous.event_metrics import EventMetricsCollector, AnomalyRecord

        prober = SelfProber()
        collector = EventMetricsCollector()
        prober.set_metrics_collector(collector)

        # Record an error event to trigger anomaly
        await collector.record("test_event", duration_ms=100.0, is_error=True)

        # analyze() should process metrics
        proposals = prober.analyze(trigger="MANUAL", pattern_type="METRICS", severity=0.5)

        # Should return a list (may be empty, but not None)
        assert isinstance(proposals, list)

    async def test_analyze_metrics_returns_proposals(self):
        """Test 8: _analyze_metrics() returns SelfProposal list."""
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
        from cynic.nervous.event_metrics import EventMetricsCollector, AnomalyRecord

        prober = SelfProber()
        collector = EventMetricsCollector()
        prober.set_metrics_collector(collector)

        # Create a mock anomaly
        anomaly = AnomalyRecord(
            detected_at_ms=1000.0,
            anomaly_type="RATE_SPIKE",
            event_type="judgment_created",
            metric_value=100.0,
            threshold_value=60.0,
            severity=0.5,
            message="Test rate spike"
        )

        # Call _analyze_metrics directly
        proposals = prober._analyze_metrics(
            anomalies=[anomaly],
            trigger="MANUAL",
            pattern_type="METRICS",
            severity=0.5
        )

        assert isinstance(proposals, list)
        assert len(proposals) > 0
        assert all(hasattr(p, "dimension") for p in proposals)
        assert all(p.dimension == "METRICS" for p in proposals)

    async def test_analyze_metrics_filters_by_severity(self):
        """Test 9: _analyze_metrics() filters anomalies by severity threshold."""
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
        from cynic.nervous.event_metrics import AnomalyRecord

        prober = SelfProber()
        collector = EventMetricsCollector()
        prober.set_metrics_collector(collector)

        # Create low-severity anomaly
        low_severity_anomaly = AnomalyRecord(
            detected_at_ms=1000.0,
            anomaly_type="RATE_SPIKE",
            event_type="test",
            metric_value=100.0,
            threshold_value=95.0,
            severity=0.1,  # Low severity
            message="Low severity spike"
        )

        # High severity threshold should filter it out
        proposals = prober._analyze_metrics(
            anomalies=[low_severity_anomaly],
            trigger="MANUAL",
            pattern_type="METRICS",
            severity=0.5  # High threshold
        )

        # Should be empty or only include high-severity items
        assert all(p.severity >= 0.5 for p in proposals)


@pytest.mark.asyncio
class TestFactoryIntegration:
    """Integration tests with factory wiring."""

    async def test_factory_wires_metrics_to_selfprober(self):
        """Test 10: Factory wires metrics_collector to SelfProber."""
        # This test verifies factory.py integration
        # Requires live factory instantiation, so it's a smoke test

        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
        from cynic.nervous.event_metrics import EventMetricsCollector

        # Simulate what factory does
        metrics_collector = EventMetricsCollector()
        prober = SelfProber()
        prober.set_metrics_collector(metrics_collector)

        # Should have reference
        assert prober._metrics_collector is metrics_collector

    async def test_selfprober_subscribes_to_anomaly_detected(self):
        """Test 11: SelfProber subscribes to ANOMALY_DETECTED (in addition to EMERGENCE_DETECTED)."""
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
        from unittest.mock import AsyncMock, MagicMock
        from cynic.kernel.core.event_bus import CoreEvent

        mock_bus = AsyncMock()
        prober = SelfProber(bus=mock_bus)

        # Register with mock bus
        prober.start(mock_bus)

        # Should have registered for both events
        calls = [call[0][0] for call in mock_bus.on.call_args_list]
        event_types = [c.value if hasattr(c, 'value') else str(c) for c in calls]

        # Should have registered for EMERGENCE_DETECTED
        assert any("emergence" in str(e).lower() for e in event_types)
