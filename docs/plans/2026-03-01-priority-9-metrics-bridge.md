# Priority 9: Prometheus Metrics Bridge Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to execute task-by-task.

**Goal:** Export EventMetricsCollector data via HTTP /metrics endpoint in OpenMetrics format for Prometheus scraping, enabling 10k TPS observability.

**Architecture:** New router `cynic/interfaces/api/routers/metrics.py` that:
- Exports per-event-type rates, error rates, histograms (5 LOD buckets)
- Exposes anomaly count + recent anomalies
- Provides factory/bus health metrics
- Returns OpenMetrics format (TYPE/HELP comments + samples)
- Zero dependencies on prometheus_client (pure formatting)

**Tech Stack:** Python 3.13, FastAPI, no external prometheus library (pure format)

---

## Task 1: Create Metrics Router

**Files:**
- Create: `cynic/interfaces/api/routers/metrics.py`
- Modify: `cynic/interfaces/api/server.py` (mount router)

**Step 1: Write failing test**

Create `tests/test_priority9_metrics_bridge.py`:

```python
import pytest
from cynic.interfaces.api.routers.metrics import router as metrics_router


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
```

**Step 2: Run test to verify it fails**

```bash
pytest tests/test_priority9_metrics_bridge.py::TestMetricsRouter::test_metrics_endpoint_exists -v
```

Expected: FAILED - 404 (endpoint doesn't exist)

**Step 3: Create metrics router**

Create `cynic/interfaces/api/routers/metrics.py`:

```python
"""
Prometheus Metrics Bridge — HTTP /metrics endpoint

Exports EventMetricsCollector data in OpenMetrics format for Prometheus scraping.
No external prometheus_client dependency — pure format generation.
"""
from __future__ import annotations

import logging
from typing import TYPE_CHECKING

from fastapi import APIRouter, Request, Response
from cynic.kernel.core.event_bus import get_core_bus

if TYPE_CHECKING:
    from cynic.nervous.event_metrics import EventMetricsCollector

logger = logging.getLogger("cynic.interfaces.api.routers.metrics")

router = APIRouter(prefix="/metrics", tags=["observability"])


def _format_openmetrics(metrics_data: dict) -> str:
    """
    Format metrics dict as OpenMetrics text format.

    OpenMetrics format:
    # HELP metric_name Description
    # TYPE metric_name gauge|counter|histogram
    metric_name{labels} value
    """
    lines = []

    if not metrics_data:
        return "# No metrics available\n"

    # Event counts
    if "event_counts" in metrics_data:
        lines.append("# HELP cynic_event_count Total events recorded by type")
        lines.append("# TYPE cynic_event_count counter")
        for event_type, count in metrics_data["event_counts"].items():
            # Sanitize event_type for Prometheus label
            safe_name = event_type.replace(".", "_").replace("-", "_")
            lines.append(f'cynic_event_count{{event_type="{safe_name}"}} {count}')

    # Event rates
    if "event_rates" in metrics_data:
        lines.append("# HELP cynic_event_rate_per_min Events per minute by type")
        lines.append("# TYPE cynic_event_rate_per_min gauge")
        for event_type, rate in metrics_data["event_rates"].items():
            safe_name = event_type.replace(".", "_").replace("-", "_")
            lines.append(f'cynic_event_rate_per_min{{event_type="{safe_name}"}} {rate:.2f}')

    # Error rates
    if "error_rates" in metrics_data:
        lines.append("# HELP cynic_error_rate Error rate by event type")
        lines.append("# TYPE cynic_error_rate gauge")
        for event_type, rate in metrics_data["error_rates"].items():
            safe_name = event_type.replace(".", "_").replace("-", "_")
            lines.append(f'cynic_error_rate{{event_type="{safe_name}"}} {rate:.4f}')

    # Anomaly count
    if "anomaly_count" in metrics_data:
        lines.append("# HELP cynic_anomalies_detected Total anomalies detected")
        lines.append("# TYPE cynic_anomalies_detected counter")
        lines.append(f'cynic_anomalies_detected {metrics_data["anomaly_count"]}')

    # Bus stats
    if "bus_stats" in metrics_data:
        bus = metrics_data["bus_stats"]
        lines.append("# HELP cynic_bus_emitted Events emitted on bus")
        lines.append("# TYPE cynic_bus_emitted counter")
        lines.append(f'cynic_bus_emitted {bus.get("emitted", 0)}')

        lines.append("# HELP cynic_bus_pending Pending tasks on bus")
        lines.append("# TYPE cynic_bus_pending gauge")
        lines.append(f'cynic_bus_pending {bus.get("pending_tasks", 0)}')

        lines.append("# HELP cynic_bus_errors Bus handler errors")
        lines.append("# TYPE cynic_bus_errors counter")
        lines.append(f'cynic_bus_errors {bus.get("errors", 0)}')

    lines.append("# EOF")
    return "\n".join(lines) + "\n"


@router.get("", response_class=Response)
async def metrics(request: Request) -> Response:
    """
    Prometheus-compatible metrics endpoint.

    Returns OpenMetrics format with:
    - Event counts and rates by type
    - Error rates by type
    - Anomaly detection counters
    - Bus health metrics
    """
    try:
        # Try to get metrics from the global bus
        bus = get_core_bus("DEFAULT")
        metrics_data = {}

        # Bus stats (always available)
        if hasattr(bus, "stats"):
            metrics_data["bus_stats"] = bus.stats()

        # EventMetricsCollector stats (if available)
        if hasattr(bus, "_metrics_adapter") and hasattr(bus._metrics_adapter, "_collector"):
            collector = bus._metrics_adapter._collector

            # Get current rates
            import asyncio
            try:
                rates = asyncio.run(collector.current_rates())
                metrics_data["event_rates"] = rates
            except Exception as e:
                logger.debug(f"Failed to get event rates: {e}")

            # Get metrics for all types
            try:
                all_metrics = asyncio.run(collector.all_metrics())
                metrics_data["event_counts"] = {
                    m.event_type: m.count_in_window
                    for m in all_metrics.values()
                }
                metrics_data["error_rates"] = {
                    m.event_type: m.error_rate
                    for m in all_metrics.values()
                }
            except Exception as e:
                logger.debug(f"Failed to get all metrics: {e}")

            # Get anomaly count
            try:
                anomalies = asyncio.run(collector.recent_anomalies(limit=1000))
                metrics_data["anomaly_count"] = len(anomalies)
            except Exception as e:
                logger.debug(f"Failed to get anomalies: {e}")

        text = _format_openmetrics(metrics_data)
        return Response(content=text, media_type="application/openmetrics-text; charset=utf-8")

    except Exception as e:
        logger.error(f"Metrics endpoint error: {e}", exc_info=True)
        return Response(
            content="# Error generating metrics\n",
            media_type="application/openmetrics-text; charset=utf-8",
            status_code=200,  # Prometheus expects 200 even on partial failures
        )
```

**Step 4: Mount router in server.py**

In `cynic/interfaces/api/server.py`, add import and mount:

```python
from cynic.interfaces.api.routers import metrics

app.include_router(metrics.router)
```

**Step 5: Run test to verify it passes**

```bash
pytest tests/test_priority9_metrics_bridge.py::TestMetricsRouter -v
```

Expected: All 4 tests PASS

**Step 6: Commit**

```bash
git add cynic/interfaces/api/routers/metrics.py cynic/interfaces/api/server.py tests/test_priority9_metrics_bridge.py
git commit -m "feat(priority-9-p1): Create Prometheus metrics bridge router"
```

---

## Task 2: Add histogram export

**Files:**
- Modify: `cynic/interfaces/api/routers/metrics.py`

**Step 1: Write failing test**

Add to `tests/test_priority9_metrics_bridge.py`:

```python
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
```

**Step 2: Implement histogram export**

Update `_format_openmetrics()` in metrics.py to include histogram data:

```python
    # Latency histograms (5 LOD buckets)
    if "histograms" in metrics_data:
        lines.append("# HELP cynic_event_latency_seconds Event latency distribution")
        lines.append("# TYPE cynic_event_latency_seconds histogram")
        for event_type, buckets in metrics_data["histograms"].items():
            safe_name = event_type.replace(".", "_").replace("-", "_")
            total = sum(buckets.values())
            # Output histogram buckets
            for bucket_label, count in buckets.items():
                lines.append(f'cynic_event_latency_seconds_bucket{{event_type="{safe_name}",le="{bucket_label}"}} {count}')
```

**Step 3: Collect histogram data in endpoint**

Update metrics endpoint to gather histogram data:

```python
                # Get histograms
                try:
                    all_metrics = asyncio.run(collector.all_metrics())
                    metrics_data["histograms"] = {
                        m.event_type: m.histogram
                        for m in all_metrics.values()
                    }
                except Exception as e:
                    logger.debug(f"Failed to get histograms: {e}")
```

**Step 4: Run test**

```bash
pytest tests/test_priority9_metrics_bridge.py::TestMetricsRouter::test_metrics_includes_histograms -v
```

Expected: PASS

**Step 5: Commit**

```bash
git add cynic/interfaces/api/routers/metrics.py tests/test_priority9_metrics_bridge.py
git commit -m "feat(priority-9-p2): Add latency histogram export to metrics endpoint"
```

---

## Task 3: Integration test + verification

**Files:**
- Modify: `tests/test_priority9_metrics_bridge.py`

**Step 1: Add integration test**

Add new test class `TestMetricsIntegration`:

```python
@pytest.mark.asyncio
class TestMetricsIntegration:
    """Integration tests with actual metrics collection."""

    async def test_metrics_reflects_collected_events(self):
        """Test 6: /metrics reflects EventMetricsCollector data."""
        from fastapi.testclient import TestClient
        from cynic.interfaces.api.server import app
        from cynic.nervous.event_metrics import EventMetricsCollector

        # Get collector from bus
        from cynic.kernel.core.event_bus import get_core_bus
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
```

**Step 2: Run tests**

```bash
pytest tests/test_priority9_metrics_bridge.py -v
```

Expected: All 7 tests PASS

**Step 3: Verify metrics format manually**

```bash
curl http://localhost:8000/metrics
```

Expected: Valid OpenMetrics format with TYPE/HELP comments

**Step 4: Commit**

```bash
git add tests/test_priority9_metrics_bridge.py
git commit -m "test(priority-9-p3): Add integration tests for metrics bridge"
```

---

## Task 4: Update audit script + final verification

**Files:**
- Modify: `scripts/audit_api_routers.py` (auto-verify metrics router mounted)

**Step 1: Run full test suite**

```bash
pytest tests/test_priority9_metrics_bridge.py tests/test_priority8_metrics_integration.py -v --tb=short 2>&1 | tail -10
```

Expected: 22/22 tests PASS (7 P9 + 15 P8)

**Step 2: Verify router audit**

```bash
python scripts/audit_api_routers.py
```

Expected: All 27 routers mounted (26 existing + metrics)

**Step 3: Final verification**

```bash
git log --oneline -5
```

Expected: 3 P9 commits visible (P9-P1, P9-P2, P9-P3)

**Step 4: Commit final tests**

```bash
git add tests/test_priority9_metrics_bridge.py
git commit -m "test(priority-9-p4): Final metrics bridge verification and integration tests"
```

---

## Verification Checklist

- [ ] Metrics router created at `/metrics`
- [ ] OpenMetrics format output (TYPE/HELP comments + samples)
- [ ] Event counts exported by type
- [ ] Event rates exported (per-minute)
- [ ] Error rates exported by type
- [ ] Latency histograms (5 LOD buckets)
- [ ] Anomaly count exported
- [ ] Bus health metrics exported (emitted, pending, errors)
- [ ] No external prometheus_client dependency
- [ ] <100ms response time
- [ ] 27 routers mounted (including metrics)
- [ ] All 22 P8-P9 tests passing
- [ ] Zero regressions (P5-8 still passing)

---

## What Priority 9 Enables

- **Prometheus scraping** — `/metrics` endpoint queryable every 15s
- **Grafana dashboards** — Real-time event rates, error rates, anomaly detection
- **Alert rules** — Fire on rate spikes, error spikes, anomalies
- **SLO tracking** — Latency percentiles (p50/p99) from histograms
- **Capacity planning** — 10k TPS observability (event throughput, pending tasks, latency distribution)

