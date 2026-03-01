"""
Context-aware timeout strategy for MCP tool calls.

Implements timeout categories optimized for different operation types:
- FAST: 2s (health checks, status queries)
- NORMAL: 30s (ask, observe, learn operations)
- BATCH: 300s (empirical tests - 5 minute jobs)
- STREAM: None (watch_telemetry, watch_source - indefinite)

This prevents spurious timeouts on long-running batch operations while
maintaining responsiveness for quick operations.

See `.claude/KERNEL_GUIDANCE.md` for timeout philosophy.
"""
from __future__ import annotations

from enum import Enum


class TimeoutCategory(Enum):
    """Categories of MCP operations with different timeout requirements."""

    FAST = 2.0  # Health checks, status queries - must be quick
    NORMAL = 30.0  # Standard operations - ask, observe, learn
    BATCH = 300.0  # Long-running tests - 5 minute jobs
    STREAM = None  # Streaming - indefinite (watch_telemetry, watch_source)

    def __float__(self) -> float:
        """Allow direct conversion to float (for None, returns inf)."""
        if self.value is None:
            return float("inf")
        return float(self.value)


class TimeoutConfig:
    """
    Configuration mapping tools to timeout categories.

    Maintains a centralized registry of tool timeouts, making it easy to
    adjust timeout policies without changing adapter code.
    """

    # Tool name → TimeoutCategory
    TOOL_TIMEOUTS: dict[str, TimeoutCategory] = {
        # ════════════════════════════════════════════════════════════════
        # FAST TOOLS (2s timeout) — Health checks, status, monitoring
        # ════════════════════════════════════════════════════════════════
        "cynic_health": TimeoutCategory.FAST,
        "cynic_status": TimeoutCategory.FAST,
        "cynic_get_job_status": TimeoutCategory.FAST,
        "cynic_get_kernel_status": TimeoutCategory.FAST,
        "cynic_ping": TimeoutCategory.FAST,

        # ════════════════════════════════════════════════════════════════
        # NORMAL TOOLS (30s timeout) — Standard cognitive operations
        # ════════════════════════════════════════════════════════════════
        "ask_cynic": TimeoutCategory.NORMAL,
        "observe_cynic": TimeoutCategory.NORMAL,
        "learn_cynic": TimeoutCategory.NORMAL,
        "discuss_cynic": TimeoutCategory.NORMAL,
        "cynic_query_telemetry": TimeoutCategory.NORMAL,
        "cynic_get_axioms": TimeoutCategory.NORMAL,
        "cynic_get_dogs": TimeoutCategory.NORMAL,
        "cynic_get_q_table": TimeoutCategory.NORMAL,

        # ════════════════════════════════════════════════════════════════
        # BATCH TOOLS (300s timeout) — Long-running empirical tests
        # ════════════════════════════════════════════════════════════════
        "cynic_run_empirical_test": TimeoutCategory.BATCH,
        "cynic_test_axiom_irreducibility": TimeoutCategory.BATCH,
        "cynic_benchmark_learning_efficiency": TimeoutCategory.BATCH,
        "cynic_run_load_test": TimeoutCategory.BATCH,

        # ════════════════════════════════════════════════════════════════
        # STREAM TOOLS (∞ timeout) — Indefinite observation
        # ════════════════════════════════════════════════════════════════
        "cynic_watch_telemetry": TimeoutCategory.STREAM,
        "cynic_watch_source": TimeoutCategory.STREAM,
        "cynic_stream_judgments": TimeoutCategory.STREAM,
    }

    @classmethod
    def get_timeout(cls, tool_name: str) -> float | None:
        """
        Get timeout in seconds for a tool.

        Args:
            tool_name: Name of the tool (e.g., "ask_cynic")

        Returns:
            Timeout in seconds, or None for indefinite (stream tools)
            Defaults to NORMAL (30s) if tool not in registry
        """
        category = cls.get_category(tool_name)
        timeout_value = category.value
        return timeout_value

    @classmethod
    def get_category(cls, tool_name: str) -> TimeoutCategory:
        """
        Get timeout category for a tool.

        Args:
            tool_name: Name of the tool

        Returns:
            TimeoutCategory enum value
            Defaults to NORMAL if tool not registered
        """
        return cls.TOOL_TIMEOUTS.get(tool_name, TimeoutCategory.NORMAL)

    @classmethod
    def summary(cls) -> dict[str, dict[str, list[str]]]:
        """
        Get summary of timeout policy by category.

        Returns:
            Dict with structure:
            {
                "FAST": {"timeout_s": 2.0, "tools": [...]},
                "NORMAL": {"timeout_s": 30.0, "tools": [...]},
                "BATCH": {"timeout_s": 300.0, "tools": [...]},
                "STREAM": {"timeout_s": None, "tools": [...]},
            }
        """
        summary: dict[str, dict] = {}

        for category in TimeoutCategory:
            summary[category.name] = {
                "timeout_s": category.value,
                "tools": [],
            }

        for tool_name, category in cls.TOOL_TIMEOUTS.items():
            summary[category.name]["tools"].append(tool_name)

        # Sort tools alphabetically within each category
        for category_data in summary.values():
            category_data["tools"].sort()

        return summary
