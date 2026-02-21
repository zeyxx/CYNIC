"""Tests for MCPBridge metrics and health checks.

TDD Step 1: Write tests FIRST, then implement.

MCPMetrics responsibilities:
- Track total, successful, and failed tool calls
- Record latency (min/avg/max)
- Expose uptime
- Health status based on bridge running state and error rate
"""

from __future__ import annotations

import time

import pytest

from cynic.core.event_bus import reset_all_buses


# ════════════════════════════════════════════════════════════════════════════
# FIXTURES
# ════════════════════════════════════════════════════════════════════════════

@pytest.fixture(autouse=True)
def _clean_buses():
    """Reset global bus singletons between tests."""
    reset_all_buses()
    yield
    reset_all_buses()


# ════════════════════════════════════════════════════════════════════════════
# MCPMetrics DATACLASS
# ════════════════════════════════════════════════════════════════════════════

class TestMCPMetricsInitialization:
    """MCPMetrics should initialize with zero counters."""

    def test_total_calls_starts_at_zero(self) -> None:
        from cynic.mcp.metrics import MCPMetrics

        metrics = MCPMetrics()
        assert metrics.total_calls == 0

    def test_successful_calls_starts_at_zero(self) -> None:
        from cynic.mcp.metrics import MCPMetrics

        metrics = MCPMetrics()
        assert metrics.successful_calls == 0

    def test_failed_calls_starts_at_zero(self) -> None:
        from cynic.mcp.metrics import MCPMetrics

        metrics = MCPMetrics()
        assert metrics.failed_calls == 0

    def test_avg_latency_starts_at_zero(self) -> None:
        from cynic.mcp.metrics import MCPMetrics

        metrics = MCPMetrics()
        assert metrics.avg_latency_ms == 0.0

    def test_max_latency_starts_at_zero(self) -> None:
        from cynic.mcp.metrics import MCPMetrics

        metrics = MCPMetrics()
        assert metrics.max_latency_ms == 0.0

    def test_started_at_is_recent(self) -> None:
        from cynic.mcp.metrics import MCPMetrics

        before = time.time()
        metrics = MCPMetrics()
        after = time.time()
        assert before <= metrics.started_at <= after


# ════════════════════════════════════════════════════════════════════════════
# MCPMetrics.record_call
# ════════════════════════════════════════════════════════════════════════════

class TestMCPMetricsRecordCall:
    """record_call must update counters and latency tracking."""

    def test_record_successful_call(self) -> None:
        from cynic.mcp.metrics import MCPMetrics

        metrics = MCPMetrics()
        metrics.record_call(latency_ms=42.5, success=True)

        assert metrics.total_calls == 1
        assert metrics.successful_calls == 1
        assert metrics.failed_calls == 0

    def test_record_failed_call(self) -> None:
        from cynic.mcp.metrics import MCPMetrics

        metrics = MCPMetrics()
        metrics.record_call(latency_ms=100.0, success=False)

        assert metrics.total_calls == 1
        assert metrics.successful_calls == 0
        assert metrics.failed_calls == 1

    def test_latency_min_max_avg(self) -> None:
        from cynic.mcp.metrics import MCPMetrics

        metrics = MCPMetrics()
        metrics.record_call(latency_ms=10.0, success=True)
        metrics.record_call(latency_ms=30.0, success=True)
        metrics.record_call(latency_ms=20.0, success=True)

        assert metrics.min_latency_ms == 10.0
        assert metrics.max_latency_ms == 30.0
        assert metrics.avg_latency_ms == 20.0

    def test_multiple_calls_accumulate(self) -> None:
        from cynic.mcp.metrics import MCPMetrics

        metrics = MCPMetrics()
        for i in range(5):
            metrics.record_call(latency_ms=float(i * 10), success=True)
        metrics.record_call(latency_ms=99.0, success=False)

        assert metrics.total_calls == 6
        assert metrics.successful_calls == 5
        assert metrics.failed_calls == 1


# ════════════════════════════════════════════════════════════════════════════
# MCPMetrics.to_dict
# ════════════════════════════════════════════════════════════════════════════

class TestMCPMetricsToDict:
    """to_dict must return a serializable summary."""

    def test_to_dict_keys(self) -> None:
        from cynic.mcp.metrics import MCPMetrics

        metrics = MCPMetrics()
        d = metrics.to_dict()

        expected_keys = {
            "total_calls",
            "successful_calls",
            "failed_calls",
            "avg_latency_ms",
            "min_latency_ms",
            "max_latency_ms",
            "uptime_s",
        }
        assert set(d.keys()) == expected_keys

    def test_to_dict_min_latency_zero_when_no_calls(self) -> None:
        from cynic.mcp.metrics import MCPMetrics

        metrics = MCPMetrics()
        d = metrics.to_dict()

        # min_latency should be 0 (not inf) when no calls recorded
        assert d["min_latency_ms"] == 0.0

    def test_to_dict_values_after_calls(self) -> None:
        from cynic.mcp.metrics import MCPMetrics

        metrics = MCPMetrics()
        metrics.record_call(latency_ms=50.0, success=True)
        metrics.record_call(latency_ms=150.0, success=False)
        d = metrics.to_dict()

        assert d["total_calls"] == 2
        assert d["successful_calls"] == 1
        assert d["failed_calls"] == 1
        assert d["avg_latency_ms"] == 100.0
        assert d["min_latency_ms"] == 50.0
        assert d["max_latency_ms"] == 150.0
        assert d["uptime_s"] >= 0.0


# ════════════════════════════════════════════════════════════════════════════
# MCPMetrics.uptime_s
# ════════════════════════════════════════════════════════════════════════════

class TestMCPMetricsUptime:
    """uptime_s must increase over time."""

    def test_uptime_is_non_negative(self) -> None:
        from cynic.mcp.metrics import MCPMetrics

        metrics = MCPMetrics()
        assert metrics.uptime_s >= 0.0

    def test_uptime_increases(self) -> None:
        from cynic.mcp.metrics import MCPMetrics

        metrics = MCPMetrics()
        t1 = metrics.uptime_s
        time.sleep(0.05)
        t2 = metrics.uptime_s
        assert t2 > t1


# ════════════════════════════════════════════════════════════════════════════
# MCPBridge.get_metrics
# ════════════════════════════════════════════════════════════════════════════

class TestMCPBridgeGetMetrics:
    """MCPBridge.get_metrics must delegate to MCPMetrics.to_dict."""

    def test_get_metrics_returns_dict(self) -> None:
        from cynic.mcp.service import MCPBridge

        bridge = MCPBridge()
        metrics = bridge.get_metrics()

        assert isinstance(metrics, dict)
        assert "total_calls" in metrics
        assert "uptime_s" in metrics

    @pytest.mark.asyncio
    async def test_get_metrics_tracks_successful_call(self) -> None:
        from cynic.mcp.service import MCPBridge, MCPTool

        bridge = MCPBridge()
        await bridge.startup()
        bridge.register_tool(MCPTool(name="test_tool", description="test", input_schema={}))

        await bridge.handle_call("test_tool", {})

        metrics = bridge.get_metrics()
        assert metrics["total_calls"] == 1
        assert metrics["successful_calls"] == 1

    @pytest.mark.asyncio
    async def test_get_metrics_tracks_latency(self) -> None:
        from cynic.mcp.service import MCPBridge, MCPTool

        bridge = MCPBridge()
        await bridge.startup()
        bridge.register_tool(MCPTool(name="test_tool", description="test", input_schema={}))

        await bridge.handle_call("test_tool", {"key": "value"})

        metrics = bridge.get_metrics()
        assert metrics["avg_latency_ms"] >= 0.0
        assert metrics["max_latency_ms"] >= 0.0


# ════════════════════════════════════════════════════════════════════════════
# MCPBridge.get_health
# ════════════════════════════════════════════════════════════════════════════

class TestMCPBridgeGetHealth:
    """MCPBridge.get_health must report operational status."""

    def test_health_has_required_keys(self) -> None:
        from cynic.mcp.service import MCPBridge

        bridge = MCPBridge()
        health = bridge.get_health()

        assert "status" in health
        assert "uptime_s" in health
        assert "tools_registered" in health
        assert "total_calls" in health
        assert "error_rate" in health

    def test_health_status_degraded_when_not_running(self) -> None:
        from cynic.mcp.service import MCPBridge

        bridge = MCPBridge()
        health = bridge.get_health()

        assert health["status"] == "degraded"

    @pytest.mark.asyncio
    async def test_health_status_healthy_when_running(self) -> None:
        from cynic.mcp.service import MCPBridge

        bridge = MCPBridge()
        await bridge.startup()
        health = bridge.get_health()

        assert health["status"] == "healthy"

    def test_health_error_rate_zero_with_no_calls(self) -> None:
        from cynic.mcp.service import MCPBridge

        bridge = MCPBridge()
        health = bridge.get_health()

        assert health["error_rate"] == 0.0

    def test_health_tools_registered_count(self) -> None:
        from cynic.mcp.service import MCPBridge, MCPTool

        bridge = MCPBridge()
        bridge.register_tool(MCPTool(name="a", description="a", input_schema={}))
        bridge.register_tool(MCPTool(name="b", description="b", input_schema={}))

        health = bridge.get_health()
        assert health["tools_registered"] == 2


# ════════════════════════════════════════════════════════════════════════════
# MCPBridge metrics integration with handle_call failures
# ════════════════════════════════════════════════════════════════════════════

class TestMCPBridgeMetricsOnFailure:
    """Metrics must track failed calls (exceptions from handle_call)."""

    @pytest.mark.asyncio
    async def test_failed_call_tracked_on_unknown_tool(self) -> None:
        from cynic.mcp.service import MCPBridge

        bridge = MCPBridge()
        await bridge.startup()

        with pytest.raises(KeyError):
            await bridge.handle_call("nonexistent", {})

        metrics = bridge.get_metrics()
        assert metrics["total_calls"] == 1
        assert metrics["failed_calls"] == 1
        assert metrics["successful_calls"] == 0

    @pytest.mark.asyncio
    async def test_failed_call_tracked_when_bridge_stopped(self) -> None:
        from cynic.mcp.service import MCPBridge, MCPTool

        bridge = MCPBridge()
        bridge.register_tool(MCPTool(name="test", description="test", input_schema={}))

        with pytest.raises(RuntimeError):
            await bridge.handle_call("test", {})

        metrics = bridge.get_metrics()
        assert metrics["total_calls"] == 1
        assert metrics["failed_calls"] == 1

    @pytest.mark.asyncio
    async def test_error_rate_reflects_failures(self) -> None:
        from cynic.mcp.service import MCPBridge, MCPTool

        bridge = MCPBridge()
        await bridge.startup()
        bridge.register_tool(MCPTool(name="test", description="test", input_schema={}))

        # 1 success
        await bridge.handle_call("test", {})

        # 1 failure
        with pytest.raises(KeyError):
            await bridge.handle_call("bad_tool", {})

        health = bridge.get_health()
        assert health["error_rate"] == 0.5
