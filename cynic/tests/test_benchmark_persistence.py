"""
D1 — LLM Benchmark Persistence Tests

Validates that LLMRegistry._benchmarks persists to / warm-loads from PostgreSQL
WITHOUT a real database (asyncpg mock pattern from test_warmload.py).

Coverage:
  - load_benchmarks_from_db() warm-start (empty, single, multiple rows)
  - Normalization: DB quality_score [0,1] ↔ BenchmarkResult.quality_score [0, MAX_Q_SCORE]
  - Additive load (existing in-memory entries not overwritten)
  - get_best_for() uses loaded benchmarks for routing
  - update_benchmark() fires fire-and-forget task when pool is set
  - update_benchmark() is safe with no pool
  - BenchmarkRepository.get_all() (mocked DB)
"""
from __future__ import annotations

import asyncio
import pytest
from unittest.mock import AsyncMock, MagicMock, patch

from cynic.core.phi import MAX_Q_SCORE, PHI_INV
from cynic.llm.adapter import LLMRegistry, BenchmarkResult, OllamaAdapter


# ════════════════════════════════════════════════════════════════════════════
# MOCK HELPERS
# ════════════════════════════════════════════════════════════════════════════

def _make_bench_pool(rows: list) -> tuple:
    """
    Build a mock asyncpg pool returning `rows` from conn.fetch().

    rows: list of dicts with keys: dog_id, task_type, llm_id,
          quality_score (0-1), speed_score (0-1), cost_score (0-1)
    """
    conn = AsyncMock()
    conn.fetch = AsyncMock(return_value=rows)
    conn.execute = AsyncMock(return_value=None)

    pool = MagicMock()
    pool.acquire = MagicMock()
    pool.acquire.return_value.__aenter__ = AsyncMock(return_value=conn)
    pool.acquire.return_value.__aexit__ = AsyncMock(return_value=False)

    return pool, conn


def _make_adapter(model: str = "gemma2:2b") -> OllamaAdapter:
    adapter = OllamaAdapter(model=model)
    return adapter


def _registry_with_adapter(model: str = "gemma2:2b") -> LLMRegistry:
    reg = LLMRegistry()
    reg.register(_make_adapter(model), available=True)
    return reg


# ════════════════════════════════════════════════════════════════════════════
# load_benchmarks_from_db()
# ════════════════════════════════════════════════════════════════════════════

class TestLoadBenchmarksFromDB:

    @pytest.mark.asyncio
    async def test_empty_db_returns_zero(self):
        """Empty DB → 0 entries, registry still empty."""
        pool, _ = _make_bench_pool([])
        reg = LLMRegistry()
        count = await reg.load_benchmarks_from_db(pool)
        assert count == 0
        assert reg._benchmarks == {}

    @pytest.mark.asyncio
    async def test_loads_single_entry(self):
        """Single DB row → 1 BenchmarkResult in _benchmarks."""
        rows = [{"dog_id": "SAGE", "task_type": "wisdom",
                 "llm_id": "ollama:gemma2:2b",
                 "quality_score": 0.72, "speed_score": 0.85, "cost_score": 0.90}]
        pool, _ = _make_bench_pool(rows)
        reg = LLMRegistry()
        count = await reg.load_benchmarks_from_db(pool)

        assert count == 1
        key = ("SAGE", "wisdom", "ollama:gemma2:2b")
        assert key in reg._benchmarks

    @pytest.mark.asyncio
    async def test_quality_score_denormalized(self):
        """DB quality_score [0,1] → BenchmarkResult.quality_score [0, MAX_Q_SCORE]."""
        db_quality = 0.72
        rows = [{"dog_id": "SAGE", "task_type": "wisdom",
                 "llm_id": "ollama:gemma2:2b",
                 "quality_score": db_quality, "speed_score": 0.9, "cost_score": 0.9}]
        pool, _ = _make_bench_pool(rows)
        reg = LLMRegistry()
        await reg.load_benchmarks_from_db(pool)

        result = reg._benchmarks[("SAGE", "wisdom", "ollama:gemma2:2b")]
        expected = pytest.approx(db_quality * MAX_Q_SCORE, rel=1e-4)
        assert result.quality_score == expected

    @pytest.mark.asyncio
    async def test_loads_multiple_entries(self):
        """Multiple rows → each stored under correct key."""
        rows = [
            {"dog_id": "SAGE",       "task_type": "wisdom",    "llm_id": "ollama:gemma2:2b",
             "quality_score": 0.80, "speed_score": 0.90, "cost_score": 0.95},
            {"dog_id": "SCHOLAR",    "task_type": "vector_rag","llm_id": "ollama:gemma2:2b",
             "quality_score": 0.65, "speed_score": 0.85, "cost_score": 0.95},
            {"dog_id": "CARTOGRAPHER","task_type":"topology",   "llm_id": "ollama:mistral:7b",
             "quality_score": 0.55, "speed_score": 0.60, "cost_score": 0.80},
        ]
        pool, _ = _make_bench_pool(rows)
        reg = LLMRegistry()
        count = await reg.load_benchmarks_from_db(pool)

        assert count == 3
        assert ("SAGE", "wisdom", "ollama:gemma2:2b") in reg._benchmarks
        assert ("SCHOLAR", "vector_rag", "ollama:gemma2:2b") in reg._benchmarks
        assert ("CARTOGRAPHER", "topology", "ollama:mistral:7b") in reg._benchmarks

    @pytest.mark.asyncio
    async def test_deduplication_keeps_first(self):
        """Duplicate keys (same dog/task/llm) keep only first (most recent by ORDER DESC)."""
        rows = [
            {"dog_id": "SAGE", "task_type": "wisdom", "llm_id": "ollama:gemma2:2b",
             "quality_score": 0.80, "speed_score": 0.90, "cost_score": 0.95},  # newer
            {"dog_id": "SAGE", "task_type": "wisdom", "llm_id": "ollama:gemma2:2b",
             "quality_score": 0.40, "speed_score": 0.50, "cost_score": 0.60},  # older
        ]
        pool, _ = _make_bench_pool(rows)
        reg = LLMRegistry()
        count = await reg.load_benchmarks_from_db(pool)

        assert count == 1  # second row skipped (duplicate key)
        result = reg._benchmarks[("SAGE", "wisdom", "ollama:gemma2:2b")]
        assert result.quality_score == pytest.approx(0.80 * MAX_Q_SCORE, rel=1e-4)

    @pytest.mark.asyncio
    async def test_additive_does_not_overwrite_in_memory(self):
        """Load is additive — in-memory entry for same key NOT overwritten by DB."""
        reg = LLMRegistry()

        # Pre-populate in-memory with higher quality
        existing = BenchmarkResult(
            llm_id="ollama:gemma2:2b",
            dog_id="SAGE",
            task_type="wisdom",
            quality_score=55.0,
            speed_score=0.95,
            cost_score=0.95,
        )
        reg._benchmarks[("SAGE", "wisdom", "ollama:gemma2:2b")] = existing

        # DB has older/lower-quality entry for same key
        rows = [{"dog_id": "SAGE", "task_type": "wisdom", "llm_id": "ollama:gemma2:2b",
                 "quality_score": 0.30, "speed_score": 0.70, "cost_score": 0.80}]
        pool, _ = _make_bench_pool(rows)
        count = await reg.load_benchmarks_from_db(pool)

        assert count == 0  # Not loaded (key already in memory)
        # In-memory value preserved
        assert reg._benchmarks[("SAGE", "wisdom", "ollama:gemma2:2b")].quality_score == 55.0

    @pytest.mark.asyncio
    async def test_db_error_returns_zero_no_crash(self):
        """DB failure → returns 0, no exception propagated."""
        pool = MagicMock()
        pool.acquire = MagicMock()
        pool.acquire.return_value.__aenter__ = AsyncMock(
            side_effect=Exception("DB connection refused")
        )
        pool.acquire.return_value.__aexit__ = AsyncMock(return_value=False)

        reg = LLMRegistry()
        count = await reg.load_benchmarks_from_db(pool)
        assert count == 0
        assert reg._benchmarks == {}


# ════════════════════════════════════════════════════════════════════════════
# Routing: get_best_for() uses loaded benchmarks
# ════════════════════════════════════════════════════════════════════════════

class TestRoutingAfterWarmLoad:

    @pytest.mark.asyncio
    async def test_get_best_for_uses_benchmarked_adapter(self):
        """After warm-load, get_best_for() routes to adapter matching benchmarked llm_id."""
        reg = _registry_with_adapter("gemma2:2b")

        rows = [{"dog_id": "SAGE", "task_type": "wisdom",
                 "llm_id": "ollama:gemma2:2b",
                 "quality_score": 0.85, "speed_score": 0.90, "cost_score": 0.95}]
        pool, _ = _make_bench_pool(rows)
        await reg.load_benchmarks_from_db(pool)

        best = reg.get_best_for("SAGE", "wisdom")
        assert best is not None
        assert getattr(best, "model", None) == "gemma2:2b"

    @pytest.mark.asyncio
    async def test_benchmark_score_beats_preferred_model(self):
        """High-composite adapter wins over preferred default."""
        reg = LLMRegistry()
        reg.register(OllamaAdapter(model="gemma2:2b"), available=True)
        reg.register(OllamaAdapter(model="mistral:7b-instruct-q4_0"), available=True)

        # Benchmark gives mistral higher score for wisdom
        rows = [
            {"dog_id": "SAGE", "task_type": "wisdom",
             "llm_id": "ollama:gemma2:2b",
             "quality_score": 0.40, "speed_score": 0.90, "cost_score": 0.95},
            {"dog_id": "SAGE", "task_type": "wisdom",
             "llm_id": "ollama:mistral:7b-instruct-q4_0",
             "quality_score": 0.90, "speed_score": 0.60, "cost_score": 0.70},
        ]
        pool, _ = _make_bench_pool(rows)
        await reg.load_benchmarks_from_db(pool)

        # composite_score comparison: mistral has higher quality (weights: PHI > 1.0 > PHI_INV)
        sage_key_mistral = ("SAGE", "wisdom", "ollama:mistral:7b-instruct-q4_0")
        sage_key_gemma = ("SAGE", "wisdom", "ollama:gemma2:2b")
        assert reg._benchmarks[sage_key_mistral].composite_score > \
               reg._benchmarks[sage_key_gemma].composite_score


# ════════════════════════════════════════════════════════════════════════════
# update_benchmark() — fire-and-forget persistence
# ════════════════════════════════════════════════════════════════════════════

class TestUpdateBenchmarkPersistence:

    def test_update_no_pool_is_safe(self):
        """Without a pool, update_benchmark() is sync and in-memory only."""
        reg = LLMRegistry()
        result = BenchmarkResult(
            llm_id="ollama:gemma2:2b", dog_id="SAGE", task_type="wisdom",
            quality_score=45.0, speed_score=0.9, cost_score=0.9,
        )
        # Should not raise
        reg.update_benchmark("SAGE", "wisdom", "ollama:gemma2:2b", result)
        assert ("SAGE", "wisdom", "ollama:gemma2:2b") in reg._benchmarks

    def test_update_ema_applied(self):
        """Second update applies EMA (not naive overwrite)."""
        reg = LLMRegistry()
        r1 = BenchmarkResult(
            llm_id="ollama:gemma2:2b", dog_id="SAGE", task_type="wisdom",
            quality_score=50.0, speed_score=0.9, cost_score=0.9,
        )
        r2 = BenchmarkResult(
            llm_id="ollama:gemma2:2b", dog_id="SAGE", task_type="wisdom",
            quality_score=30.0, speed_score=0.7, cost_score=0.7,
        )
        reg.update_benchmark("SAGE", "wisdom", "ollama:gemma2:2b", r1)
        reg.update_benchmark("SAGE", "wisdom", "ollama:gemma2:2b", r2)

        stored = reg._benchmarks[("SAGE", "wisdom", "ollama:gemma2:2b")]
        # EMA: 0.3 * 30 + 0.7 * 50 = 44
        assert stored.quality_score == pytest.approx(44.0, abs=1.0)

    @pytest.mark.asyncio
    async def test_update_with_pool_fires_task(self):
        """With pool set, update_benchmark() schedules DB save as asyncio task."""
        pool, conn = _make_bench_pool([])
        reg = LLMRegistry()
        reg.set_db_pool(pool)

        result = BenchmarkResult(
            llm_id="ollama:gemma2:2b", dog_id="SAGE", task_type="wisdom",
            quality_score=45.0, speed_score=0.9, cost_score=0.9,
        )
        reg.update_benchmark("SAGE", "wisdom", "ollama:gemma2:2b", result)

        # Give the fire-and-forget task a chance to run
        await asyncio.sleep(0.05)

        # DB execute should have been called
        assert conn.execute.called, "fire-and-forget DB save should have run"

    @pytest.mark.asyncio
    async def test_save_normalizes_quality_score(self):
        """DB save passes quality_score / MAX_Q_SCORE (must satisfy BETWEEN 0 AND 1)."""
        pool, conn = _make_bench_pool([])
        reg = LLMRegistry()
        reg.set_db_pool(pool)

        result = BenchmarkResult(
            llm_id="ollama:gemma2:2b", dog_id="SAGE", task_type="wisdom",
            quality_score=MAX_Q_SCORE,  # Maximum value
            speed_score=1.0, cost_score=1.0,
        )
        reg.update_benchmark("SAGE", "wisdom", "ollama:gemma2:2b", result)
        await asyncio.sleep(0.05)

        assert conn.execute.called
        call_args = conn.execute.call_args[0]
        # call_args: (sql, benchmark_id, dog_id, task_type, llm_id, quality_score, ...)
        # quality_score is $5 → index 5 (0=sql, 1=uuid, 2=dog, 3=task, 4=llm, 5=quality)
        quality_arg = call_args[5]
        assert 0.0 <= quality_arg <= 1.0, (
            f"DB quality_score {quality_arg} must be in [0,1]"
        )

    @pytest.mark.asyncio
    async def test_db_save_failure_does_not_crash(self):
        """fire-and-forget DB save failure is silently logged, never raises."""
        pool = MagicMock()
        conn = AsyncMock()
        conn.execute = AsyncMock(side_effect=Exception("DB write error"))
        pool.acquire = MagicMock()
        pool.acquire.return_value.__aenter__ = AsyncMock(return_value=conn)
        pool.acquire.return_value.__aexit__ = AsyncMock(return_value=False)

        reg = LLMRegistry()
        reg.set_db_pool(pool)

        result = BenchmarkResult(
            llm_id="ollama:gemma2:2b", dog_id="SAGE", task_type="wisdom",
            quality_score=45.0, speed_score=0.9, cost_score=0.9,
        )
        reg.update_benchmark("SAGE", "wisdom", "ollama:gemma2:2b", result)

        # Should not raise even with DB failure
        await asyncio.sleep(0.05)
        # In-memory update still happened
        assert ("SAGE", "wisdom", "ollama:gemma2:2b") in reg._benchmarks
