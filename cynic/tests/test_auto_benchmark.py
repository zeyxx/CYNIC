"""
Tests for AutoBenchmark — periodic LLM probe scheduler (T09).

Coverage:
  1. run_once() calls update_benchmark() for each adapter × task_type probe
  2. Disabled by CYNIC_AUTOBENCH=0 env var
  3. Failed response → quality_score = 0.0
  4. Successful response → quality_score = WAG_MIN / 2 (conservative)
  5. stats() returns expected shape
  6. run_once() returns correct completed count
"""
from __future__ import annotations

import os
from unittest.mock import AsyncMock, MagicMock

import pytest

from cynic.metabolism.auto_benchmark import AutoBenchmark, _PROBES
from cynic.llm.adapter import LLMResponse
from cynic.core.phi import WAG_MIN


# ════════════════════════════════════════════════════════════════════════════
# HELPERS
# ════════════════════════════════════════════════════════════════════════════

def make_registry(adapters, update_mock=None):
    """Build a minimal mock LLMRegistry."""
    reg = MagicMock()
    reg.get_available_for_generation.return_value = adapters
    reg.update_benchmark = update_mock or MagicMock()
    return reg


def make_adapter(adapter_id="ollama:gemma2:2b", success=True, latency_ms=200.0,
                 tokens=20, cost_usd=0.0):
    """Build a mock LLMAdapter."""
    adapter = MagicMock()
    adapter.adapter_id = adapter_id
    resp = LLMResponse(
        content="ok" if success else "",
        model="gemma2:2b",
        provider="ollama",
        completion_tokens=tokens,
        latency_ms=latency_ms,
        cost_usd=cost_usd,
        error=None if success else "timeout",
    )
    adapter.complete_safe = AsyncMock(return_value=resp)
    return adapter


# ════════════════════════════════════════════════════════════════════════════
# TESTS
# ════════════════════════════════════════════════════════════════════════════

class TestAutoBenchmark:

    @pytest.mark.asyncio
    async def test_run_once_calls_update_benchmark_for_each_probe(self):
        """run_once() must call update_benchmark once per (adapter × task_type) probe."""
        adapter = make_adapter()
        update = MagicMock()
        reg = make_registry([adapter], update_mock=update)

        ab = AutoBenchmark(reg)
        count = await ab.run_once()

        assert count == len(_PROBES)
        assert update.call_count == len(_PROBES)

    @pytest.mark.asyncio
    async def test_run_once_returns_completed_count(self):
        """run_once() returns the number of entries successfully recorded."""
        adapter = make_adapter()
        reg = make_registry([adapter])

        ab = AutoBenchmark(reg)
        count = await ab.run_once()

        assert count == len(_PROBES)

    @pytest.mark.asyncio
    async def test_failed_response_sets_quality_zero(self):
        """Error response → quality_score = 0.0 (failure is explicit signal)."""
        adapter = make_adapter(success=False)
        captured = []
        reg = make_registry([adapter])
        reg.update_benchmark = lambda **kwargs: captured.append(kwargs["result"])

        ab = AutoBenchmark(reg)
        await ab.run_once()

        assert all(r.quality_score == pytest.approx(0.0) for r in captured)

    @pytest.mark.asyncio
    async def test_success_sets_conservative_quality(self):
        """Success → quality_score = WAG_MIN / 2 (conservative placeholder)."""
        adapter = make_adapter(success=True)
        captured = []
        reg = make_registry([adapter])
        reg.update_benchmark = lambda **kwargs: captured.append(kwargs["result"])

        ab = AutoBenchmark(reg)
        await ab.run_once()

        expected = WAG_MIN / 2.0
        assert all(r.quality_score == pytest.approx(expected, abs=0.1) for r in captured)

    @pytest.mark.asyncio
    async def test_no_adapters_returns_zero(self):
        """No available LLMs → run_once() returns 0, no update_benchmark calls."""
        update = MagicMock()
        reg = make_registry([], update_mock=update)

        ab = AutoBenchmark(reg)
        count = await ab.run_once()

        assert count == 0
        update.assert_not_called()

    def test_disabled_by_env_var(self):
        """CYNIC_AUTOBENCH=0 → start() does not launch a background task."""
        reg = make_registry([])

        ab = AutoBenchmark(reg)
        os.environ["CYNIC_AUTOBENCH"] = "0"
        try:
            ab.start()
            # No task should have been created
            assert ab._task is None
        finally:
            del os.environ["CYNIC_AUTOBENCH"]

    def test_stats_returns_expected_keys(self):
        """stats() returns enabled, runs, interval_s."""
        reg = make_registry([])
        ab = AutoBenchmark(reg)

        s = ab.stats()

        assert "enabled" in s
        assert "runs" in s
        assert "interval_s" in s
        assert s["interval_s"] == 3300  # F(10) × 60
        assert s["runs"] == 0

    @pytest.mark.asyncio
    async def test_runs_counter_increments(self):
        """Each run_once() call increments the internal runs counter."""
        adapter = make_adapter()
        reg = make_registry([adapter])

        ab = AutoBenchmark(reg)
        assert ab._runs == 0

        await ab.run_once()
        assert ab._runs == 1

        await ab.run_once()
        assert ab._runs == 2
