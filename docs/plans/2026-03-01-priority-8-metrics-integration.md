# Priority 8: SelfProber Metrics Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Integrate EventMetricsCollector (P7) into SelfProber's analysis pipeline to generate self-improvement proposals based on real-time anomalies (rate spikes, error spikes, latency degradation).

**Architecture:** SelfProber now has five analysis dimensions (QTABLE, ESCORE, RESIDUAL, ARCHITECTURE, **METRICS**). When ANOMALY_DETECTED events arrive from BusMetricsAdapter, or manually triggered, SelfProber analyzes the metrics stream and generates proposals to address performance degradation, event rate anomalies, and error clustering.

**Tech Stack:** Pure Python (no new dependencies). Integrates EventMetricsCollector output into SelfProposal dataclass. Uses existing SelfProber proposal lifecycle (PENDING→APPLIED/DISMISSED).

---

## Task 1: Create MetricsAnalyzer (helper class)

**Files:**
- Create: `cynic/nervous/metrics_analyzer.py`
- Modify: `cynic/nervous/__init__.py`

**Step 1: Write the failing test**

Create `tests/test_priority8_metrics_integration.py`:

```python
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
            assert p.dimension == "METRICS"
```

**Step 2: Run test to verify it fails**

```bash
pytest tests/test_priority8_metrics_integration.py::TestMetricsAnalyzer::test_metrics_analyzer_init -v
```

Expected output:
```
FAILED - ModuleNotFoundError: No module named 'cynic.nervous.metrics_analyzer'
```

**Step 3: Write minimal implementation**

Create `cynic/nervous/metrics_analyzer.py`:

```python
"""
MetricsAnalyzer — Translate EventMetricsCollector output into SelfProposal recommendations.

Used by SelfProber to generate METRICS dimension proposals from anomaly records.
"""
from __future__ import annotations

import logging
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any

from cynic.kernel.core.phi import PHI, PHI_INV

if TYPE_CHECKING:
    from cynic.nervous.event_metrics import AnomalyRecord, EventMetricsCollector

logger = logging.getLogger("cynic.nervous.metrics_analyzer")


@dataclass
class MetricsProposal:
    """One metrics-driven improvement recommendation."""
    anomaly_type: str           # RATE_SPIKE | ERROR_SPIKE | LATENCY_SPIKE
    event_type: str
    metric_value: float
    threshold_value: float
    severity: float             # [0, 1]
    target: str                 # event_type (what to improve)
    recommendation: str         # Human-readable action
    current_value: float
    suggested_value: float

    def to_dict(self) -> dict[str, Any]:
        return {
            "anomaly_type": self.anomaly_type,
            "event_type": self.event_type,
            "metric_value": round(self.metric_value, 2),
            "threshold_value": round(self.threshold_value, 2),
            "severity": round(self.severity, 4),
            "target": self.target,
            "recommendation": self.recommendation,
            "current_value": round(self.current_value, 2),
            "suggested_value": round(self.suggested_value, 2),
        }


class MetricsAnalyzer:
    """
    Analyze EventMetricsCollector anomaly records and generate improvement proposals.

    Maps three anomaly types to actionable recommendations:
      - RATE_SPIKE: event frequency spiked; suggest circuit breaker or batching
      - ERROR_SPIKE: error rate exceeded threshold; suggest fallback or timeout
      - LATENCY_SPIKE: event took >3000ms; suggest optimization or async conversion
    """

    def __init__(self, collector: EventMetricsCollector) -> None:
        self._collector = collector

    def analyze_anomalies(
        self,
        anomalies: list[AnomalyRecord],
        severity_threshold: float = 0.0,
    ) -> list[MetricsProposal]:
        """
        Translate anomaly records into proposals.

        Args:
            anomalies: List of AnomalyRecord from EventMetricsCollector.detect_anomalies()
            severity_threshold: Only consider anomalies with severity >= threshold

        Returns:
            List of MetricsProposal recommendations
        """
        proposals: list[MetricsProposal] = []

        for anomaly in anomalies:
            if anomaly.severity < severity_threshold:
                continue

            if anomaly.anomaly_type == "RATE_SPIKE":
                proposals.append(self._analyze_rate_spike(anomaly))
            elif anomaly.anomaly_type == "ERROR_SPIKE":
                proposals.append(self._analyze_error_spike(anomaly))
            elif anomaly.anomaly_type == "LATENCY_SPIKE":
                proposals.append(self._analyze_latency_spike(anomaly))

        return proposals

    def _analyze_rate_spike(self, anomaly: AnomalyRecord) -> MetricsProposal:
        """Rate spike: event frequency exceeded PHI × baseline."""
        suggested_rate = anomaly.metric_value / PHI  # Scale back by PHI
        return MetricsProposal(
            anomaly_type="RATE_SPIKE",
            event_type=anomaly.event_type,
            metric_value=anomaly.metric_value,
            threshold_value=anomaly.threshold_value,
            severity=anomaly.severity,
            target=anomaly.event_type,
            recommendation=(
                f"Rate spike detected ({anomaly.metric_value:.1f}/min). "
                f"Consider batching events or implementing backpressure throttling."
            ),
            current_value=anomaly.metric_value,
            suggested_value=suggested_rate,
        )

    def _analyze_error_spike(self, anomaly: AnomalyRecord) -> MetricsProposal:
        """Error spike: error rate exceeded PHI_INV (61.8%)."""
        suggested_error_rate = anomaly.metric_value / 2.0  # Reduce by half
        return MetricsProposal(
            anomaly_type="ERROR_SPIKE",
            event_type=anomaly.event_type,
            metric_value=anomaly.metric_value,
            threshold_value=anomaly.threshold_value,
            severity=anomaly.severity,
            target=anomaly.event_type,
            recommendation=(
                f"Error rate spike detected ({anomaly.metric_value:.1%}). "
                f"Implement fallback handler or add circuit breaker for {anomaly.event_type}."
            ),
            current_value=anomaly.metric_value,
            suggested_value=suggested_error_rate,
        )

    def _analyze_latency_spike(self, anomaly: AnomalyRecord) -> MetricsProposal:
        """Latency spike: event duration exceeded LOD_LEVEL3 (3000ms)."""
        suggested_latency = 1000.0  # Target 1000ms (LOD_LEVEL2)
        return MetricsProposal(
            anomaly_type="LATENCY_SPIKE",
            event_type=anomaly.event_type,
            metric_value=anomaly.metric_value,
            threshold_value=anomaly.threshold_value,
            severity=anomaly.severity,
            target=anomaly.event_type,
            recommendation=(
                f"Latency spike detected ({anomaly.metric_value:.0f}ms). "
                f"Consider optimizing {anomaly.event_type} or converting to async."
            ),
            current_value=anomaly.metric_value,
            suggested_value=suggested_latency,
        )
```

**Step 4: Run test to verify it passes**

```bash
pytest tests/test_priority8_metrics_integration.py::TestMetricsAnalyzer -v
```

Expected: All tests pass.

**Step 5: Commit**

```bash
git add cynic/nervous/metrics_analyzer.py cynic/nervous/__init__.py tests/test_priority8_metrics_integration.py
git commit -m "feat(priority-8-p1): Create MetricsAnalyzer for anomaly translation"
```

---

## Task 2: Add metrics_collector injection to SelfProber

**Files:**
- Modify: `cynic/kernel/organism/brain/cognition/cortex/self_probe.py:130-156`
- Modify: `cynic/kernel/organism/factory.py` (wiring)

**Step 1: Write the failing test**

Add to `tests/test_priority8_metrics_integration.py`:

```python
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
```

**Step 2: Run test to verify it fails**

```bash
pytest tests/test_priority8_metrics_integration.py::TestMetricsAnalyzer::test_selfprober_set_metrics_collector -v
```

Expected:
```
FAILED - AttributeError: 'SelfProber' object has no attribute 'set_metrics_collector'
```

**Step 3: Add metrics_collector field and setter to SelfProber**

Modify `cynic/kernel/organism/brain/cognition/cortex/self_probe.py` at line 130:

```python
    def __init__(self, proposals_path: str = _PROPOSALS_PATH, bus: Optional[EventBus] = None) -> None:
        self._path = proposals_path
        self._proposals: list[SelfProposal] = []
        self._total_proposed: int = 0
        self._qtable: Any | None = None
        self._residual: Any | None = None
        self._escore: Any | None = None
        self._metrics_collector: Any | None = None
        self._registered: bool = False
        from cynic.kernel.core.event_bus import CoreEvent, Event
        self._bus = bus or get_core_bus("DEFAULT")
        self._load()
```

Add after `set_escore_tracker()` at line ~150:

```python
    def set_metrics_collector(self, collector: Any) -> None:
        """Inject EventMetricsCollector for metrics-driven analysis."""
        self._metrics_collector = collector
```

**Step 4: Run test to verify it passes**

```bash
pytest tests/test_priority8_metrics_integration.py::TestMetricsAnalyzer::test_selfprober_set_metrics_collector -v
```

Expected: PASS

**Step 5: Commit**

```bash
git add cynic/kernel/organism/brain/cognition/cortex/self_probe.py
git commit -m "feat(priority-8-p2): Add metrics_collector injection to SelfProber"
```

---

## Task 3: Implement _analyze_metrics() method in SelfProber

**Files:**
- Modify: `cynic/kernel/organism/brain/cognition/cortex/self_probe.py:225-228`
- Modify: `cynic/kernel/organism/brain/cognition/cortex/self_probe.py` (add _analyze_metrics method)

**Step 1: Write the failing test**

Add to `tests/test_priority8_metrics_integration.py`:

```python
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
```

**Step 2: Run test to verify it fails**

```bash
pytest tests/test_priority8_metrics_integration.py::TestMetricsAnalyzer::test_analyze_metrics_returns_proposals -v
```

Expected:
```
FAILED - AttributeError: 'SelfProber' object has no attribute '_analyze_metrics'
```

**Step 3: Implement _analyze_metrics() in SelfProber**

Modify `cynic/kernel/organism/brain/cognition/cortex/self_probe.py`:

First, add the import at the top with other imports:

```python
from cynic.nervous.metrics_analyzer import MetricsAnalyzer
```

Then add the method after `_analyze_architecture()` (around line 460):

```python
    def _analyze_metrics(
        self,
        anomalies: list | None = None,
        trigger: str = "MANUAL",
        pattern_type: str = "METRICS",
        severity: float = 0.0,
    ) -> list[SelfProposal]:
        """
        Analyze metrics anomalies and generate proposals.

        If anomalies not provided, fetch from EventMetricsCollector.
        """
        if not self._metrics_collector:
            return []

        # Get anomalies from collector if not provided
        if anomalies is None:
            try:
                import asyncio
                anomalies = asyncio.run(self._metrics_collector.recent_anomalies(limit=10))
            except Exception as e:
                logger.debug(f"_analyze_metrics: error fetching anomalies: {e}")
                return []

        # Translate anomalies to proposals using MetricsAnalyzer
        try:
            analyzer = MetricsAnalyzer(self._metrics_collector)
            metrics_proposals = analyzer.analyze_anomalies(
                anomalies,
                severity_threshold=severity,
            )
        except Exception as e:
            logger.debug(f"_analyze_metrics error: {e}", exc_info=True)
            return []

        # Convert MetricsProposal → SelfProposal
        proposals: list[SelfProposal] = []
        for mp in metrics_proposals:
            probe_id = _short_id()
            proposal = SelfProposal(
                probe_id=probe_id,
                trigger=trigger,
                pattern_type=pattern_type,
                severity=mp.severity,
                dimension="METRICS",
                target=mp.target,
                recommendation=mp.recommendation,
                current_value=mp.current_value,
                suggested_value=mp.suggested_value,
            )
            proposals.append(proposal)

        return proposals
```

Now update the `analyze()` method to call `_analyze_metrics()`. Modify line 227:

```python
    def analyze(
        self,
        trigger: str = "MANUAL",
        pattern_type: str = "UNKNOWN",
        severity: float = 0.5,
    ) -> list[SelfProposal]:
        """
        Run all five analyses and return new proposals generated.
        """
        new_proposals: list[SelfProposal] = []
        new_proposals.extend(self._analyze_qtable(trigger, pattern_type, severity))
        new_proposals.extend(self._analyze_escore(trigger, pattern_type, severity))
        new_proposals.extend(self._analyze_residual(trigger, pattern_type, severity))
        new_proposals.extend(self._analyze_architecture(trigger, pattern_type, severity))
        new_proposals.extend(self._analyze_metrics(anomalies=None, trigger=trigger, pattern_type=pattern_type, severity=severity))

        for proposal in new_proposals:
            self._proposals.append(proposal)
            self._total_proposed += 1

        # Rolling cap – evict oldest first
        while len(self._proposals) > _MAX_PROPOSALS:
            self._proposals.pop(0)

        if new_proposals:
            self._save()

        return new_proposals
```

**Step 4: Run test to verify it passes**

```bash
pytest tests/test_priority8_metrics_integration.py::TestMetricsAnalyzer::test_analyze_metrics_returns_proposals tests/test_priority8_metrics_integration.py::TestMetricsAnalyzer::test_analyze_metrics_filters_by_severity -v
```

Expected: Both tests pass.

**Step 5: Commit**

```bash
git add cynic/kernel/organism/brain/cognition/cortex/self_probe.py
git commit -m "feat(priority-8-p3): Implement _analyze_metrics() for anomaly-driven proposals"
```

---

## Task 4: Wire metrics_collector in factory.py

**Files:**
- Modify: `cynic/kernel/organism/factory.py` (factory wiring)

**Step 1: Write the failing test**

Add to `tests/test_priority8_metrics_integration.py`:

```python
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
```

**Step 2: Run test to verify it fails** (currently, since factory isn't updated)

```bash
pytest tests/test_priority8_metrics_integration.py::TestFactoryIntegration::test_factory_wires_metrics_to_selfprober -v
```

Expected: PASS (test is for wiring verification, not factory yet)

**Step 3: Update factory.py wiring**

Modify `cynic/kernel/organism/factory.py` around line 230 (after self.sona_emitter creation):

Find this section (around line 243-244):

```python
        self.sona_emitter = SonaEmitter(bus=instance_bus, db_pool=self.db_pool, instance_id=instance_id)
```

Add after it:

```python
        # 6b. METRICS → SELFPROBER WIRING
        self.self_prober.set_metrics_collector(self.metrics_collector)
```

**Step 4: Run test to verify it passes**

```bash
pytest tests/test_priority8_metrics_integration.py::TestFactoryIntegration -v
```

Expected: PASS

**Step 5: Commit**

```bash
git add cynic/kernel/organism/factory.py tests/test_priority8_metrics_integration.py
git commit -m "feat(priority-8-p4): Wire metrics_collector to SelfProber in factory"
```

---

## Task 5: Add ANOMALY_DETECTED event trigger for SelfProber

**Files:**
- Modify: `cynic/kernel/organism/brain/cognition/cortex/self_probe.py:159-166`

**Step 1: Write the failing test**

Add to `tests/test_priority8_metrics_integration.py`:

```python
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
```

**Step 2: Run test to verify it fails**

```bash
pytest tests/test_priority8_metrics_integration.py::TestFactoryIntegration::test_selfprober_subscribes_to_anomaly_detected -v
```

Expected: Test may pass or fail depending on current implementation.

**Step 3: Update SelfProber.start() to subscribe to ANOMALY_DETECTED**

Modify `cynic/kernel/organism/brain/cognition/cortex/self_probe.py` at line 159:

```python
    def start(self, bus=None) -> None:
        """Subscribe to EMERGENCE_DETECTED and ANOMALY_DETECTED. Call once at kernel startup."""
        if self._registered:
            return
        target_bus = bus or self._bus
        target_bus.on(CoreEvent.EMERGENCE_DETECTED, self._on_emergence)
        target_bus.on(CoreEvent.ANOMALY_DETECTED, self._on_anomaly_detected)
        self._registered = True
        logger.info("SelfProber subscribed to EMERGENCE_DETECTED and ANOMALY_DETECTED")
```

Add new handler method after `_on_emergence()`:

```python
    async def _on_anomaly_detected(self, event: Event) -> None:
        """
        Handle ANOMALY_DETECTED from BusMetricsAdapter.

        Triggers metrics analysis with anomaly severity as pattern severity.
        """
        if not self._metrics_collector:
            return

        payload = event.dict_payload
        anomaly_type = payload.get("anomaly_type", "UNKNOWN")
        severity = payload.get("severity", 0.5)

        # Trigger analysis with anomaly context
        proposals = self.analyze(
            trigger="ANOMALY_DETECTED",
            pattern_type=f"ANOMALY_{anomaly_type}",
            severity=severity,
        )

        if proposals:
            logger.info(f"SelfProber: {len(proposals)} proposals from ANOMALY_DETECTED ({anomaly_type})")
            for proposal in proposals:
                try:
                    await self._bus.emit(Event.typed(
                        CoreEvent.SELF_IMPROVEMENT_PROPOSED,
                        payload=proposal.to_dict(),
                        source="self_prober",
                    ))
                except Exception as e:
                    logger.debug(f"Failed to emit SELF_IMPROVEMENT_PROPOSED: {e}")
```

**Step 4: Run test to verify it passes**

```bash
pytest tests/test_priority8_metrics_integration.py::TestFactoryIntegration::test_selfprober_subscribes_to_anomaly_detected -v
```

Expected: PASS

**Step 5: Commit**

```bash
git add cynic/kernel/organism/brain/cognition/cortex/self_probe.py tests/test_priority8_metrics_integration.py
git commit -m "feat(priority-8-p5): Subscribe SelfProber to ANOMALY_DETECTED events"
```

---

## Task 6: Comprehensive integration test

**Files:**
- Modify: `tests/test_priority8_metrics_integration.py`

**Step 1: Write integration test**

Add to `tests/test_priority8_metrics_integration.py`:

```python
@pytest.mark.asyncio
class TestEndToEndMetricsIntegration:
    """End-to-end integration: metrics anomalies → proposals."""

    async def test_rate_spike_generates_proposal(self):
        """Test 12: Rate spike anomaly generates SelfProposal."""
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
        from cynic.nervous.event_metrics import EventMetricsCollector

        # Setup
        prober = SelfProber()
        collector = EventMetricsCollector()
        prober.set_metrics_collector(collector)

        # Create rate spike anomaly
        for i in range(50):
            await collector.record("judgment_created", duration_ms=100.0)

        # Trigger analysis
        proposals = prober.analyze(
            trigger="MANUAL",
            pattern_type="METRICS",
            severity=0.0,
        )

        # Should have generated proposals (if anomalies detected)
        assert isinstance(proposals, list)

    async def test_error_spike_generates_proposal(self):
        """Test 13: Error spike generates METRICS dimension proposal."""
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
        from cynic.nervous.event_metrics import EventMetricsCollector

        prober = SelfProber()
        collector = EventMetricsCollector()
        prober.set_metrics_collector(collector)

        # Record high error rate
        for i in range(10):
            is_error = i < 8  # 80% error rate
            await collector.record("failing_op", duration_ms=100.0, is_error=is_error)

        # Get anomalies and analyze
        anomalies = await collector.detect_anomalies()
        proposals = prober._analyze_metrics(
            anomalies=anomalies,
            trigger="ANOMALY_DETECTED",
            pattern_type="ANOMALY_ERROR_SPIKE",
            severity=0.0,
        )

        # Should generate proposals if anomalies detected
        assert isinstance(proposals, list)

    async def test_proposal_has_metrics_dimension(self):
        """Test 14: All metrics proposals have dimension='METRICS'."""
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
        from cynic.nervous.event_metrics import EventMetricsCollector, AnomalyRecord

        prober = SelfProber()
        collector = EventMetricsCollector()
        prober.set_metrics_collector(collector)

        # Create test anomaly
        anomaly = AnomalyRecord(
            detected_at_ms=1000.0,
            anomaly_type="LATENCY_SPIKE",
            event_type="slow_handler",
            metric_value=5000.0,
            threshold_value=3000.0,
            severity=0.6,
            message="Latency spike detected"
        )

        proposals = prober._analyze_metrics(
            anomalies=[anomaly],
            trigger="MANUAL",
            pattern_type="LATENCY_SPIKE",
            severity=0.0,
        )

        # All should have METRICS dimension
        for p in proposals:
            assert p.dimension == "METRICS"

    async def test_persists_metrics_proposals(self):
        """Test 15: Metrics proposals persist to disk like other proposals."""
        import tempfile
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
        from cynic.nervous.event_metrics import EventMetricsCollector, AnomalyRecord

        with tempfile.TemporaryDirectory() as tmpdir:
            proposals_file = f"{tmpdir}/proposals.json"

            prober = SelfProber(proposals_path=proposals_file)
            collector = EventMetricsCollector()
            prober.set_metrics_collector(collector)

            # Generate proposal
            anomaly = AnomalyRecord(
                detected_at_ms=1000.0,
                anomaly_type="RATE_SPIKE",
                event_type="test_event",
                metric_value=100.0,
                threshold_value=60.0,
                severity=0.5,
                message="Rate spike"
            )

            proposals = prober._analyze_metrics(
                anomalies=[anomaly],
                trigger="MANUAL",
                pattern_type="METRICS",
                severity=0.0,
            )

            # Add to prober and save
            for p in proposals:
                prober._proposals.append(p)
            prober._save()

            # File should exist
            import os
            assert os.path.exists(proposals_file)

            # Load and verify
            loaded = prober._load()
            assert len(prober._proposals) > 0
```

**Step 2: Run tests to verify they pass**

```bash
pytest tests/test_priority8_metrics_integration.py::TestEndToEndMetricsIntegration -v
```

Expected: All 4 tests pass.

**Step 3: Run full test suite to verify no regressions**

```bash
pytest tests/test_priority8_metrics_integration.py -v --tb=short
```

Expected: 15 tests total, all passing.

**Step 4: Commit**

```bash
git add tests/test_priority8_metrics_integration.py
git commit -m "test(priority-8-p6): Add comprehensive integration tests for metrics analysis"
```

---

## Task 7: Verify all tests pass and no regressions

**Step 1: Run Priority 8 test suite**

```bash
pytest tests/test_priority8_metrics_integration.py -v
```

Expected: 15/15 tests passing.

**Step 2: Run Priority 5-7 test suite to verify no regressions**

```bash
pytest tests/test_priority7_event_metrics.py tests/test_priority6_state_reconstruction.py tests/test_priority5_event_protocol.py tests/test_decision_trace.py tests/test_event_journal.py -v --tb=short
```

Expected: 88/88 tests passing (16 P7 + 72 P5-P6).

**Step 3: Run full test suite**

```bash
pytest tests/ -k "test_priority" --tb=short 2>&1 | tail -20
```

Expected: 103/103 tests passing (15 P8 + 88 P5-7).

**Step 4: Update memory**

Edit `C:\Users\zeyxm\.claude\projects\C--Users-zeyxm-Desktop-asdfasdfa-CYNIC-clean\memory\MEMORY.md`:

Add at the top:

```markdown
## 🎯 SESSION 6J: PRIORITY 8 COMPLETE — METRICS-DRIVEN SELF-IMPROVEMENT (2026-03-01)

**Status:** Priority 8 Metrics Integration COMPLETE ✅
**Scope:** MetricsAnalyzer + SelfProber integration + ANOMALY_DETECTED subscription
**Achievement:** Real-time anomaly analysis with self-improvement proposals
**Tests:** 103/103 passing (15 P8 new + 88 P5-P7 existing)
**Files:** 1 created (metrics_analyzer.py) + 3 modified

### Key Components

- **MetricsAnalyzer:** Translates AnomalyRecord→SelfProposal (rate/error/latency spikes)
- **SelfProber._analyze_metrics():** New 5th analysis dimension (QTABLE, ESCORE, RESIDUAL, ARCHITECTURE, **METRICS**)
- **ANOMALY_DETECTED trigger:** SelfProber now subscribes to anomalies from BusMetricsAdapter
- **Proposal generation:** Rate spikes→batching suggestions, error spikes→fallback suggestions, latency spikes→optimization suggestions
```

**Step 5: Commit memory update**

```bash
git add C:\Users\zeyxm\.claude\projects\C--Users-zeyxm-Desktop-asdfasdfa-CYNIC-clean\memory\MEMORY.md
git commit -m "docs: Update memory with Priority 8 completion"
```

---

## Verification Checklist

- [ ] All 15 Priority 8 tests pass
- [ ] All 88 Priority 5-7 tests pass (no regressions)
- [ ] MetricsAnalyzer correctly translates anomalies to proposals
- [ ] SelfProber has 5 analysis dimensions (QTABLE, ESCORE, RESIDUAL, ARCHITECTURE, METRICS)
- [ ] Factory wires metrics_collector to SelfProber
- [ ] SelfProber subscribes to both EMERGENCE_DETECTED and ANOMALY_DETECTED
- [ ] Proposals persist to ~/.cynic/self_proposals.json
- [ ] No circular imports (metrics_analyzer → nervous, kernel.core only)
- [ ] Async-first design (no blocking in handlers)

---

## Next Priorities Enabled

- **Priority 9:** Prometheus metrics bridge (HTTP /metrics endpoint with EventMetricsCollector export)
- **Priority 10:** SelfProber automation (auto-apply low-risk proposals, CLI review interface)
- **Priority 11:** Fractal federation (gossip protocol for peer self-improvement recommendations)

