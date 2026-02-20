"""
Tests for LLM adapter layer — LLMRegistry, BenchmarkResult, feedback loop.

Coverage:
  1. BenchmarkResult: composite_score, EMA update
  2. LLMRegistry: register/get_available, get_best_for (routing logic)
  3. Benchmark feedback loop: LLMDog.record_judgment() → registry.update_benchmark()
  4. LLMDog._record_benchmark(): speed/cost normalization
  5. Routing improves after repeated feedback (EMA convergence)
"""
from __future__ import annotations

from unittest.mock import MagicMock

import pytest

from cynic.core.phi import PHI, PHI_INV, PHI_INV_2, MAX_Q_SCORE
from cynic.cognition.neurons.base import DogId, DogJudgment, LLMDog, _SPEED_TARGET_MS, _COST_BUDGET_USD


# ════════════════════════════════════════════════════════════════════════════
# HELPERS
# ════════════════════════════════════════════════════════════════════════════

def make_judgment(
    dog_id: str = DogId.SAGE,
    q_score: float = 45.0,
    latency_ms: float = 500.0,
    cost_usd: float = 0.0,
    llm_id: str = "ollama:llama3.2",
) -> DogJudgment:
    return DogJudgment(
        dog_id=dog_id,
        cell_id="test-cell",
        q_score=q_score,
        confidence=PHI_INV_2,
        reasoning="test",
        latency_ms=latency_ms,
        cost_usd=cost_usd,
        llm_id=llm_id,
    )


def make_adapter(adapter_id: str = "ollama:llama3.2") -> MagicMock:
    m = MagicMock()
    m.adapter_id = adapter_id
    return m


def make_benchmark(
    llm_id: str = "ollama:llama3.2",
    quality: float = 40.0,
    speed: float = 0.8,
    cost: float = 1.0,
) -> "BenchmarkResult":
    from cynic.llm.adapter import BenchmarkResult
    return BenchmarkResult(
        llm_id=llm_id,
        dog_id=DogId.SAGE,
        task_type="temporal_mcts",
        quality_score=quality,
        speed_score=speed,
        cost_score=cost,
    )


# ════════════════════════════════════════════════════════════════════════════
# BenchmarkResult
# ════════════════════════════════════════════════════════════════════════════

class TestBenchmarkResult:

    def test_composite_score_positive(self):
        b = make_benchmark(quality=40.0, speed=0.8, cost=1.0)
        assert b.composite_score > 0.0

    def test_composite_score_zero_quality(self):
        """Zero quality → composite = 0 (geometric mean property)."""
        b = make_benchmark(quality=0.0, speed=1.0, cost=1.0)
        assert b.composite_score == 0.0

    def test_composite_quality_dominates(self):
        """High quality wins when speed/cost are equal between candidates."""
        # Geometric mean punishes low values hard — hold speed/cost equal
        # so only quality difference determines winner
        high_q = make_benchmark(quality=55.0, speed=0.7, cost=0.7)
        low_q  = make_benchmark(quality=20.0, speed=0.7, cost=0.7)
        assert high_q.composite_score > low_q.composite_score

    def test_ema_update_moves_toward_new(self):
        """EMA update: new value pulls old value toward it."""
        old = make_benchmark(quality=30.0, speed=0.5, cost=0.5)
        new_result = make_benchmark(quality=50.0, speed=0.9, cost=0.9)
        updated = old.ema_update(new_result, alpha=0.3)
        # Should be between old and new (weighted)
        assert old.quality_score < updated.quality_score < new_result.quality_score

    def test_ema_update_increments_sample_count(self):
        old = make_benchmark()
        new_result = make_benchmark()
        updated = old.ema_update(new_result)
        assert updated.sample_count == old.sample_count + 1

    def test_ema_preserves_identifiers(self):
        old = make_benchmark(llm_id="ollama:llama3.2")
        new_result = make_benchmark(llm_id="ollama:llama3.2")
        updated = old.ema_update(new_result)
        assert updated.llm_id == "ollama:llama3.2"
        assert updated.dog_id == old.dog_id
        assert updated.task_type == old.task_type

    def test_ema_alpha_zero_no_change(self):
        """Alpha=0 → no movement from old values."""
        old = make_benchmark(quality=30.0)
        new_result = make_benchmark(quality=60.0)
        updated = old.ema_update(new_result, alpha=0.0)
        assert abs(updated.quality_score - old.quality_score) < 1e-9

    def test_ema_alpha_one_full_update(self):
        """Alpha=1 → new value replaces old completely."""
        old = make_benchmark(quality=30.0)
        new_result = make_benchmark(quality=55.0)
        updated = old.ema_update(new_result, alpha=1.0)
        assert abs(updated.quality_score - new_result.quality_score) < 1e-9


# ════════════════════════════════════════════════════════════════════════════
# LLMRegistry — register + routing
# ════════════════════════════════════════════════════════════════════════════

class TestLLMRegistry:

    def _make_registry(self):
        from cynic.llm.adapter import LLMRegistry
        return LLMRegistry()

    def test_register_makes_available(self):
        reg = self._make_registry()
        adapter = make_adapter("ollama:llama3.2")
        reg.register(adapter, available=True)
        assert adapter in reg.get_available()

    def test_register_unavailable_not_in_get_available(self):
        reg = self._make_registry()
        adapter = make_adapter("ollama:llama3.2")
        reg.register(adapter, available=False)
        assert adapter not in reg.get_available()

    def test_register_multiple(self):
        reg = self._make_registry()
        a1 = make_adapter("ollama:llama3.2")
        a2 = make_adapter("claude:sonnet")
        reg.register(a1, available=True)
        reg.register(a2, available=True)
        available = reg.get_available()
        assert len(available) == 2

    def test_get_best_for_no_benchmarks_returns_first(self):
        """No benchmark data → return first available adapter."""
        reg = self._make_registry()
        a1 = make_adapter("ollama:llama3.2")
        reg.register(a1, available=True)
        result = reg.get_best_for(DogId.SAGE, "temporal_mcts")
        assert result is a1

    def test_get_best_for_no_adapters_returns_none(self):
        reg = self._make_registry()
        result = reg.get_best_for(DogId.SAGE, "temporal_mcts")
        assert result is None

    def test_get_best_for_picks_highest_composite(self):
        """Registry routes to LLM with best composite score."""
        from cynic.llm.adapter import LLMRegistry, BenchmarkResult
        reg = LLMRegistry()

        fast = make_adapter("ollama:fast")
        smart = make_adapter("claude:smart")
        reg.register(fast, available=True)
        reg.register(smart, available=True)

        # Fast model: low quality
        reg.update_benchmark(DogId.SAGE, "temporal_mcts", "ollama:fast",
            BenchmarkResult(llm_id="ollama:fast", dog_id=DogId.SAGE,
                           task_type="temporal_mcts",
                           quality_score=25.0, speed_score=0.95, cost_score=1.0))

        # Smart model: much higher quality, slightly slower, modest cost
        # (geometric mean: quality weight=φ wins when speed/cost aren't terrible)
        reg.update_benchmark(DogId.SAGE, "temporal_mcts", "claude:smart",
            BenchmarkResult(llm_id="claude:smart", dog_id=DogId.SAGE,
                           task_type="temporal_mcts",
                           quality_score=55.0, speed_score=0.85, cost_score=0.9))

        # Quality advantage outweighs slight speed/cost penalty
        best = reg.get_best_for(DogId.SAGE, "temporal_mcts")
        assert best.adapter_id == "claude:smart"

    def test_update_benchmark_stores_result(self):
        from cynic.llm.adapter import LLMRegistry, BenchmarkResult
        reg = LLMRegistry()
        b = BenchmarkResult(llm_id="ollama:llama3.2", dog_id=DogId.SAGE,
                           task_type="temporal_mcts",
                           quality_score=40.0, speed_score=0.8, cost_score=1.0)
        reg.update_benchmark(DogId.SAGE, "temporal_mcts", "ollama:llama3.2", b)
        matrix = reg.benchmark_matrix(DogId.SAGE, "temporal_mcts")
        assert "ollama:llama3.2" in matrix

    def test_update_benchmark_ema_on_second_call(self):
        """Second call to update_benchmark uses EMA (not replace)."""
        from cynic.llm.adapter import LLMRegistry, BenchmarkResult
        reg = LLMRegistry()

        b1 = BenchmarkResult(llm_id="ollama:llama3.2", dog_id=DogId.SAGE,
                            task_type="temporal_mcts",
                            quality_score=30.0, speed_score=0.8, cost_score=1.0)
        b2 = BenchmarkResult(llm_id="ollama:llama3.2", dog_id=DogId.SAGE,
                            task_type="temporal_mcts",
                            quality_score=60.0, speed_score=0.8, cost_score=1.0)

        reg.update_benchmark(DogId.SAGE, "temporal_mcts", "ollama:llama3.2", b1)
        reg.update_benchmark(DogId.SAGE, "temporal_mcts", "ollama:llama3.2", b2)

        matrix = reg.benchmark_matrix(DogId.SAGE, "temporal_mcts")
        stored = matrix["ollama:llama3.2"]
        # EMA: quality should be between 30 and 60
        assert 30.0 < stored.quality_score < 60.0
        assert stored.sample_count == 2

    def test_benchmark_matrix_scoped_by_dog_and_task(self):
        """benchmark_matrix() only returns results for the queried dog+task."""
        from cynic.llm.adapter import LLMRegistry, BenchmarkResult
        reg = LLMRegistry()

        for dog, task in [(DogId.SAGE, "temporal_mcts"), (DogId.SCHOLAR, "vector_rag")]:
            reg.update_benchmark(dog, task, "ollama:llama3.2",
                BenchmarkResult(llm_id="ollama:llama3.2", dog_id=dog,
                               task_type=task,
                               quality_score=40.0, speed_score=0.8, cost_score=1.0))

        sage_matrix = reg.benchmark_matrix(DogId.SAGE, "temporal_mcts")
        scholar_matrix = reg.benchmark_matrix(DogId.SCHOLAR, "vector_rag")
        assert len(sage_matrix) == 1
        assert len(scholar_matrix) == 1

    def test_unavailable_adapter_excluded_from_routing(self):
        """Unavailable adapters never selected even with great benchmarks."""
        from cynic.llm.adapter import LLMRegistry, BenchmarkResult
        reg = LLMRegistry()

        gone = make_adapter("ollama:gone")
        active = make_adapter("ollama:active")
        reg.register(gone, available=False)   # Down
        reg.register(active, available=True)  # Up

        # Give gone adapter a perfect score
        reg.update_benchmark(DogId.SAGE, "temporal_mcts", "ollama:gone",
            BenchmarkResult(llm_id="ollama:gone", dog_id=DogId.SAGE,
                           task_type="temporal_mcts",
                           quality_score=61.8, speed_score=1.0, cost_score=1.0))

        best = reg.get_best_for(DogId.SAGE, "temporal_mcts")
        assert best.adapter_id == "ollama:active"


# ════════════════════════════════════════════════════════════════════════════
# Benchmark Feedback Loop — LLMDog._record_benchmark()
# ════════════════════════════════════════════════════════════════════════════

class TestBenchmarkFeedbackLoop:

    def _make_llm_dog_with_registry(self):
        """SageDog with a real LLMRegistry injected."""
        from cynic.cognition.neurons.sage import SageDog
        from cynic.llm.adapter import LLMRegistry
        dog = SageDog()
        reg = LLMRegistry()
        adapter = make_adapter("ollama:llama3.2")
        reg.register(adapter, available=True)
        dog.set_llm_registry(reg)
        return dog, reg

    def test_record_judgment_with_llm_id_triggers_benchmark(self):
        """record_judgment(llm_id set) → registry.update_benchmark called."""
        from cynic.cognition.neurons.sage import SageDog
        dog = SageDog()
        mock_reg = MagicMock()
        dog.set_llm_registry(mock_reg)

        j = make_judgment(dog_id=DogId.SAGE, llm_id="ollama:llama3.2")
        dog.record_judgment(j)

        mock_reg.update_benchmark.assert_called_once()
        call_kwargs = mock_reg.update_benchmark.call_args
        assert call_kwargs[1]["dog_id"] == DogId.SAGE or call_kwargs[0][0] == DogId.SAGE

    def test_record_judgment_no_llm_id_no_benchmark(self):
        """Heuristic path (llm_id=None) does NOT update benchmark."""
        from cynic.cognition.neurons.sage import SageDog
        dog = SageDog()
        mock_reg = MagicMock()
        dog.set_llm_registry(mock_reg)

        j = make_judgment(dog_id=DogId.SAGE, llm_id=None)
        dog.record_judgment(j)

        mock_reg.update_benchmark.assert_not_called()

    def test_record_judgment_no_registry_no_crash(self):
        """Without registry (no LLM mode), record_judgment doesn't crash."""
        from cynic.cognition.neurons.sage import SageDog
        dog = SageDog()
        # No registry injected
        j = make_judgment(dog_id=DogId.SAGE, llm_id="ollama:llama3.2")
        dog.record_judgment(j)  # Must not raise

    def test_speed_score_fast_call(self):
        """Fast call (<< 3000ms) → high speed score."""
        from cynic.cognition.neurons.sage import SageDog
        dog = SageDog()
        mock_reg = MagicMock()
        dog.set_llm_registry(mock_reg)

        j = make_judgment(latency_ms=300.0, cost_usd=0.0, llm_id="ollama:x")
        dog.record_judgment(j)

        result = mock_reg.update_benchmark.call_args[1]["result"]
        expected_speed = max(0.0, 1.0 - 300.0 / _SPEED_TARGET_MS)
        assert abs(result.speed_score - expected_speed) < 1e-6

    def test_speed_score_slow_call_near_zero(self):
        """Call at 3000ms → speed_score ≈ 0."""
        from cynic.cognition.neurons.sage import SageDog
        dog = SageDog()
        mock_reg = MagicMock()
        dog.set_llm_registry(mock_reg)

        j = make_judgment(latency_ms=3000.0, llm_id="ollama:x")
        dog.record_judgment(j)

        result = mock_reg.update_benchmark.call_args[1]["result"]
        assert result.speed_score == pytest.approx(0.0, abs=1e-6)

    def test_speed_score_beyond_target_clamped_zero(self):
        """Call beyond 3000ms → speed_score clamped to 0."""
        from cynic.cognition.neurons.sage import SageDog
        dog = SageDog()
        mock_reg = MagicMock()
        dog.set_llm_registry(mock_reg)

        j = make_judgment(latency_ms=5000.0, llm_id="ollama:x")
        dog.record_judgment(j)

        result = mock_reg.update_benchmark.call_args[1]["result"]
        assert result.speed_score == 0.0

    def test_cost_score_free_ollama(self):
        """cost_usd=0 (Ollama) → cost_score = 1.0."""
        from cynic.cognition.neurons.sage import SageDog
        dog = SageDog()
        mock_reg = MagicMock()
        dog.set_llm_registry(mock_reg)

        j = make_judgment(cost_usd=0.0, llm_id="ollama:x")
        dog.record_judgment(j)

        result = mock_reg.update_benchmark.call_args[1]["result"]
        assert result.cost_score == 1.0

    def test_cost_score_over_budget_clamped_zero(self):
        """cost_usd > budget → cost_score = 0."""
        from cynic.cognition.neurons.sage import SageDog
        dog = SageDog()
        mock_reg = MagicMock()
        dog.set_llm_registry(mock_reg)

        j = make_judgment(cost_usd=0.02, llm_id="claude:x")  # 2× budget
        dog.record_judgment(j)

        result = mock_reg.update_benchmark.call_args[1]["result"]
        assert result.cost_score == 0.0

    def test_cost_score_half_budget(self):
        """cost_usd = $0.005 (half budget) → cost_score = 0.5."""
        from cynic.cognition.neurons.sage import SageDog
        dog = SageDog()
        mock_reg = MagicMock()
        dog.set_llm_registry(mock_reg)

        j = make_judgment(cost_usd=_COST_BUDGET_USD * 0.5, llm_id="claude:x")
        dog.record_judgment(j)

        result = mock_reg.update_benchmark.call_args[1]["result"]
        assert abs(result.cost_score - 0.5) < 1e-6

    def test_quality_score_passthrough(self):
        """quality_score in BenchmarkResult = DogJudgment.q_score."""
        from cynic.cognition.neurons.sage import SageDog
        dog = SageDog()
        mock_reg = MagicMock()
        dog.set_llm_registry(mock_reg)

        j = make_judgment(q_score=42.0, llm_id="ollama:x")
        dog.record_judgment(j)

        result = mock_reg.update_benchmark.call_args[1]["result"]
        assert abs(result.quality_score - 42.0) < 1e-6

    def test_error_rate_zero_for_successful_judgment(self):
        from cynic.cognition.neurons.sage import SageDog
        dog = SageDog()
        mock_reg = MagicMock()
        dog.set_llm_registry(mock_reg)

        j = make_judgment(llm_id="ollama:x")
        dog.record_judgment(j)

        result = mock_reg.update_benchmark.call_args[1]["result"]
        assert result.error_rate == 0.0

    def test_benchmark_update_passes_dog_id_and_task_type(self):
        """update_benchmark called with correct dog_id + task_type."""
        from cynic.cognition.neurons.scholar import ScholarDog
        dog = ScholarDog()  # task_type = "vector_rag"
        mock_reg = MagicMock()
        dog.set_llm_registry(mock_reg)

        j = make_judgment(dog_id=DogId.SCHOLAR, llm_id="ollama:x")
        dog.record_judgment(j)

        call = mock_reg.update_benchmark.call_args
        # Positional or keyword — handle both
        args = call[0] if call[0] else []
        kwargs = call[1] if call[1] else {}
        dog_arg = kwargs.get("dog_id", args[0] if args else None)
        task_arg = kwargs.get("task_type", args[1] if len(args) > 1 else None)
        llm_arg  = kwargs.get("llm_id",   args[2] if len(args) > 2 else None)
        assert dog_arg == DogId.SCHOLAR
        assert task_arg == "vector_rag"
        assert llm_arg  == "ollama:x"


# ════════════════════════════════════════════════════════════════════════════
# Routing Improves With Feedback (end-to-end)
# ════════════════════════════════════════════════════════════════════════════

class TestRoutingConvergence:

    def test_routing_converges_toward_best_model(self):
        """
        After many feedback rounds, registry routes to higher-quality model.

        Simulates 10 judgments: model A (quality=50) vs model B (quality=25).
        After feedback, get_best_for() should return model A.
        """
        from cynic.cognition.neurons.sage import SageDog
        from cynic.llm.adapter import LLMRegistry, BenchmarkResult

        reg = LLMRegistry()
        a = make_adapter("ollama:good")   # High quality
        b = make_adapter("ollama:bad")    # Low quality
        reg.register(a, available=True)
        reg.register(b, available=True)

        # Feed 5 rounds of benchmark for each
        for _ in range(5):
            reg.update_benchmark(DogId.SAGE, "temporal_mcts", "ollama:good",
                BenchmarkResult(llm_id="ollama:good", dog_id=DogId.SAGE,
                               task_type="temporal_mcts",
                               quality_score=50.0, speed_score=0.8, cost_score=1.0))
            reg.update_benchmark(DogId.SAGE, "temporal_mcts", "ollama:bad",
                BenchmarkResult(llm_id="ollama:bad", dog_id=DogId.SAGE,
                               task_type="temporal_mcts",
                               quality_score=20.0, speed_score=0.9, cost_score=1.0))

        best = reg.get_best_for(DogId.SAGE, "temporal_mcts")
        assert best.adapter_id == "ollama:good"

    def test_routing_updates_after_degradation(self):
        """
        If a previously-good model degrades, routing switches to alternative.
        """
        from cynic.llm.adapter import LLMRegistry, BenchmarkResult

        reg = LLMRegistry()
        good = make_adapter("ollama:good")
        backup = make_adapter("ollama:backup")
        reg.register(good, available=True)
        reg.register(backup, available=True)

        # Initially good model is best
        reg.update_benchmark(DogId.SAGE, "temporal_mcts", "ollama:good",
            BenchmarkResult(llm_id="ollama:good", dog_id=DogId.SAGE,
                           task_type="temporal_mcts",
                           quality_score=55.0, speed_score=0.8, cost_score=1.0))
        reg.update_benchmark(DogId.SAGE, "temporal_mcts", "ollama:backup",
            BenchmarkResult(llm_id="ollama:backup", dog_id=DogId.SAGE,
                           task_type="temporal_mcts",
                           quality_score=30.0, speed_score=0.9, cost_score=1.0))

        assert reg.get_best_for(DogId.SAGE, "temporal_mcts").adapter_id == "ollama:good"

        # Now good model degrades (many bad rounds)
        for _ in range(10):
            reg.update_benchmark(DogId.SAGE, "temporal_mcts", "ollama:good",
                BenchmarkResult(llm_id="ollama:good", dog_id=DogId.SAGE,
                               task_type="temporal_mcts",
                               quality_score=5.0, speed_score=0.1, cost_score=1.0))

        # Routing should switch
        best_after = reg.get_best_for(DogId.SAGE, "temporal_mcts")
        assert best_after.adapter_id == "ollama:backup"


# ════════════════════════════════════════════════════════════════════════════
# T05: LLM Benchmark persistence via BenchmarkRepo (SurrealDB)
# ════════════════════════════════════════════════════════════════════════════

class TestBenchmarkSurrealPersistence:
    """T05 — LLMRegistry persists benchmarks to SurrealDB via BenchmarkRepo."""

    def test_set_surreal_stores_reference(self):
        """set_surreal() stores the SurrealDB reference."""
        from cynic.llm.adapter import LLMRegistry
        reg = LLMRegistry()
        mock_surreal = MagicMock()
        reg.set_surreal(mock_surreal)
        assert reg._surreal is mock_surreal

    @pytest.mark.asyncio
    async def test_save_benchmark_to_surreal_calls_benchmarks_save(self):
        """_save_benchmark_to_surreal() calls surreal.benchmarks.save()."""
        from cynic.llm.adapter import LLMRegistry, BenchmarkResult
        from unittest.mock import AsyncMock
        reg = LLMRegistry()
        mock_surreal = MagicMock()
        mock_surreal.benchmarks.save = AsyncMock()
        reg.set_surreal(mock_surreal)

        result = BenchmarkResult(
            llm_id="ollama:gemma2", dog_id=DogId.SAGE,
            task_type="temporal_mcts",
            quality_score=50.0, speed_score=0.8, cost_score=1.0,
        )
        await reg._save_benchmark_to_surreal(result)
        mock_surreal.benchmarks.save.assert_called_once()
        saved = mock_surreal.benchmarks.save.call_args[0][0]
        assert saved["llm_id"] == "ollama:gemma2"
        assert saved["dog_id"] == DogId.SAGE

    @pytest.mark.asyncio
    async def test_save_benchmark_to_surreal_tolerates_exception(self):
        """SurrealDB failure in _save_benchmark_to_surreal → no crash."""
        from cynic.llm.adapter import LLMRegistry, BenchmarkResult
        from unittest.mock import AsyncMock
        reg = LLMRegistry()
        mock_surreal = MagicMock()
        mock_surreal.benchmarks.save = AsyncMock(side_effect=Exception("surreal down"))
        reg.set_surreal(mock_surreal)

        result = BenchmarkResult(
            llm_id="x", dog_id=DogId.SAGE,
            task_type="temporal_mcts",
            quality_score=40.0, speed_score=0.8, cost_score=1.0,
        )
        await reg._save_benchmark_to_surreal(result)  # Must not raise

    @pytest.mark.asyncio
    async def test_load_benchmarks_from_surreal_populates_dict(self):
        """load_benchmarks_from_surreal() fills _benchmarks from SurrealDB rows."""
        from cynic.llm.adapter import LLMRegistry
        from unittest.mock import AsyncMock
        reg = LLMRegistry()

        mock_surreal = MagicMock()
        mock_surreal.benchmarks.get_all = AsyncMock(return_value=[
            {"llm_id": "ollama:gemma2", "dog_id": DogId.SAGE, "task_type": "temporal_mcts",
             "quality_score": 0.8, "speed_score": 0.9, "cost_score": 1.0},
        ])

        loaded = await reg.load_benchmarks_from_surreal(mock_surreal)
        assert loaded == 1
        assert (DogId.SAGE, "temporal_mcts", "ollama:gemma2") in reg._benchmarks

    @pytest.mark.asyncio
    async def test_load_benchmarks_from_surreal_skips_existing(self):
        """load_benchmarks_from_surreal() skips keys already in _benchmarks (most-recent wins)."""
        from cynic.llm.adapter import LLMRegistry, BenchmarkResult
        from unittest.mock import AsyncMock
        reg = LLMRegistry()

        # Pre-populate
        reg.update_benchmark(DogId.SAGE, "temporal_mcts", "ollama:x",
            BenchmarkResult(llm_id="ollama:x", dog_id=DogId.SAGE,
                           task_type="temporal_mcts",
                           quality_score=55.0, speed_score=0.8, cost_score=1.0))

        mock_surreal = MagicMock()
        mock_surreal.benchmarks.get_all = AsyncMock(return_value=[
            {"llm_id": "ollama:x", "dog_id": DogId.SAGE, "task_type": "temporal_mcts",
             "quality_score": 0.1, "speed_score": 0.1, "cost_score": 1.0},
        ])

        loaded = await reg.load_benchmarks_from_surreal(mock_surreal)
        assert loaded == 0  # Skipped — key already exists
        # Original value preserved
        key = (DogId.SAGE, "temporal_mcts", "ollama:x")
        assert reg._benchmarks[key].quality_score > 30.0  # Not overwritten

    @pytest.mark.asyncio
    async def test_load_benchmarks_from_surreal_handles_exception(self):
        """SurrealDB failure → returns 0, no crash."""
        from cynic.llm.adapter import LLMRegistry
        from unittest.mock import AsyncMock
        reg = LLMRegistry()

        mock_surreal = MagicMock()
        mock_surreal.benchmarks.get_all = AsyncMock(side_effect=Exception("surreal down"))

        loaded = await reg.load_benchmarks_from_surreal(mock_surreal)
        assert loaded == 0
        assert len(reg._benchmarks) == 0
