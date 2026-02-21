"""
Phase 0: Persistence Metrics
Track latency and success/failure of all persistence operations.
Used to detect bottlenecks and issues before they cause timeouts.
"""
import time
import logging
from dataclasses import dataclass, field
from collections import defaultdict
from datetime import datetime

logger = logging.getLogger(__name__)


@dataclass
class PersistenceMetric:
    """Single persistence operation metric."""
    operation: str  # "persist_judgment", "write_guidance", "sync_checkpoint"
    duration_ms: float
    success: bool
    timestamp: float = field(default_factory=time.time)
    error: str = ""


@dataclass
class PersistenceStatistics:
    """Aggregated statistics for a persistence operation type."""
    operation: str
    total_calls: int = 0
    successful_calls: int = 0
    failed_calls: int = 0
    total_duration_ms: float = 0.0
    min_duration_ms: float = float('inf')
    max_duration_ms: float = 0.0
    last_error: str = ""

    @property
    def mean_duration_ms(self) -> float:
        if self.total_calls == 0:
            return 0.0
        return self.total_duration_ms / self.total_calls

    @property
    def success_rate(self) -> float:
        if self.total_calls == 0:
            return 0.0
        return self.successful_calls / self.total_calls

    def to_dict(self) -> dict:
        return {
            "operation": self.operation,
            "total_calls": self.total_calls,
            "successful_calls": self.successful_calls,
            "failed_calls": self.failed_calls,
            "success_rate": round(self.success_rate, 4),
            "mean_duration_ms": round(self.mean_duration_ms, 2),
            "min_duration_ms": round(self.min_duration_ms, 2),
            "max_duration_ms": round(self.max_duration_ms, 2),
            "last_error": self.last_error,
        }


class PersistenceMetricsCollector:
    """Singleton collector for persistence metrics."""

    _instance = None

    def __new__(cls):
        if cls._instance is None:
            cls._instance = super().__new__(cls)
        return cls._instance

    def __init__(self):
        self.metrics: list[PersistenceMetric] = []
        self.stats: dict[str, PersistenceStatistics] = defaultdict(
            lambda: PersistenceStatistics(operation="")
        )
        logger.info("PersistenceMetricsCollector initialized")

    def record_metric(self, operation: str, duration_ms: float, success: bool, error: str = "") -> None:
        """Record a single persistence operation metric."""
        metric = PersistenceMetric(
            operation=operation,
            duration_ms=duration_ms,
            success=success,
            error=error,
        )
        self.metrics.append(metric)

        # Update aggregated statistics
        if operation not in self.stats:
            self.stats[operation] = PersistenceStatistics(operation=operation)

        stats = self.stats[operation]
        stats.total_calls += 1
        if success:
            stats.successful_calls += 1
            stats.total_duration_ms += duration_ms
            stats.min_duration_ms = min(stats.min_duration_ms, duration_ms)
            stats.max_duration_ms = max(stats.max_duration_ms, duration_ms)
        else:
            stats.failed_calls += 1
            stats.last_error = error

        # Log if duration exceeds threshold (potential timeout risk)
        if duration_ms > 1000:
            logger.warning(
                "Persistence operation %s took %.1fms (threshold: 1000ms)",
                operation,
                duration_ms,
            )

    def get_statistics(self) -> dict[str, dict]:
        """Get all aggregated statistics."""
        return {
            op: stats.to_dict()
            for op, stats in self.stats.items()
        }

    def get_health_check(self) -> dict:
        """Return health check data for monitoring."""
        if not self.metrics:
            return {"status": "no_metrics", "metrics_collected": 0}

        recent_errors = [m for m in self.metrics[-100:] if not m.success]
        error_rate = len(recent_errors) / len(self.metrics[-100:]) if self.metrics[-100:] else 0

        status = "healthy"
        if error_rate > 0.1:
            status = "degraded"
        elif error_rate > 0.05:
            status = "warning"

        return {
            "status": status,
            "metrics_collected": len(self.metrics),
            "error_rate_last_100": round(error_rate, 4),
            "statistics": self.get_statistics(),
        }

    def reset(self) -> None:
        """Reset all metrics (for testing)."""
        self.metrics.clear()
        self.stats.clear()


# Singleton instance
metrics_collector = PersistenceMetricsCollector()


async def measure_persistence(operation: str, coro):
    """
    Async context manager to measure persistence operation latency.

    Usage:
        async with measure_persistence("persist_judgment", _persist_judgment_async(j)):
            await _persist_judgment_async(j)
    """
    start = time.time()
    try:
        result = await coro
        duration_ms = (time.time() - start) * 1000
        metrics_collector.record_metric(operation, duration_ms, success=True)
        return result
    except Exception as e:
        duration_ms = (time.time() - start) * 1000
        metrics_collector.record_metric(operation, duration_ms, success=False, error=str(e))
        raise
