"""MCPBridge metrics collection.

Tracks tool call activity: counts, latency, uptime.
Designed for observability — consumed by get_metrics() and get_health()
on the MCPBridge.

All latency values are in milliseconds.
"""

from __future__ import annotations

import logging
import time
from dataclasses import dataclass, field
from typing import Any

logger = logging.getLogger("cynic.mcp.metrics")


@dataclass
class MCPMetrics:
    """Track MCPBridge activity metrics.

    Attributes:
        total_calls: Number of tool calls handled (success + failure).
        successful_calls: Number of calls that completed without error.
        failed_calls: Number of calls that raised an exception.
        total_latency_ms: Sum of all call latencies (for computing average).
        min_latency_ms: Fastest call latency. Defaults to inf (no calls yet).
        max_latency_ms: Slowest call latency.
        started_at: Unix timestamp when metrics collection began.
    """

    total_calls: int = 0
    successful_calls: int = 0
    failed_calls: int = 0
    total_latency_ms: float = 0.0
    min_latency_ms: float = float("inf")
    max_latency_ms: float = 0.0
    started_at: float = field(default_factory=time.time)

    @property
    def avg_latency_ms(self) -> float:
        """Average latency across all calls. Returns 0.0 if no calls."""
        if self.total_calls == 0:
            return 0.0
        return self.total_latency_ms / self.total_calls

    @property
    def uptime_s(self) -> float:
        """Seconds since metrics collection started."""
        return time.time() - self.started_at

    def record_call(self, latency_ms: float, *, success: bool = True) -> None:
        """Record a single tool call.

        Args:
            latency_ms: How long the call took in milliseconds.
            success: Whether the call completed without error.
        """
        self.total_calls += 1
        if success:
            self.successful_calls += 1
        else:
            self.failed_calls += 1

        self.total_latency_ms += latency_ms
        self.min_latency_ms = min(self.min_latency_ms, latency_ms)
        self.max_latency_ms = max(self.max_latency_ms, latency_ms)

    def to_dict(self) -> dict[str, Any]:
        """Serialize metrics to a plain dict for JSON responses."""
        min_latency = self.min_latency_ms if self.min_latency_ms != float("inf") else 0.0
        return {
            "total_calls": self.total_calls,
            "successful_calls": self.successful_calls,
            "failed_calls": self.failed_calls,
            "avg_latency_ms": round(self.avg_latency_ms, 2),
            "min_latency_ms": round(min_latency, 2),
            "max_latency_ms": round(self.max_latency_ms, 2),
            "uptime_s": round(self.uptime_s, 2),
        }
