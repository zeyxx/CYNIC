"""
Tests for LLMRegistry - Dynamic LLM Discovery and Routing

Tests the LLMRegistry class for:
- Model discovery (Ollama, Claude, Gemini)
- Benchmark-based routing
- Best model selection
- Registration and availability
"""
import pytest

pytestmark = pytest.mark.skip(reason="Old architecture removed in V5 - OllamaAdapter not exported from adapter module")

from unittest.mock import MagicMock

from cynic.kernel.organism.brain.llm.adapter import (
    PREFERRED_MODELS,
    BenchmarkResult,
    ClaudeAdapter,
    LLMRegistry,
    OllamaAdapter,
)


class TestLLMRegistry:
    """Test suite for LLMRegistry."""

    @pytest.fixture
    def registry(self):
        """Create a fresh registry for testing."""
        return LLMRegistry()

    def test_initialization(self, registry):
        """Should initialize with empty state."""
        assert len(registry._adapters) == 0
        assert len(registry._available) == 0
        assert len(registry._benchmarks) == 0
        assert registry._db_pool is None
        assert registry._surreal is None

    def test_register(self, registry):
        """Should register adapters."""
        adapter = OllamaAdapter(model="llama3.2")
        
        registry.register(adapter, available=True)
        
        assert "ollama:llama3.2" in registry._adapters
        assert registry._available["ollama:llama3.2"] is True

    def test_register_unavailable(self, registry):
        """Should register as unavailable."""
        adapter = OllamaAdapter(model="llama3.2")
        
        registry.register(adapter, available=False)
        
        assert registry._available["ollama:llama3.2"] is False

    def test_get_available(self, registry):
        """Should return available adapters."""
        adapter1 = OllamaAdapter(model="llama3.2")
        adapter2 = ClaudeAdapter(model="claude-haiku")
        
        registry.register(adapter1, available=True)
        registry.register(adapter2, available=False)
        
        available = registry.get_available()
        
        assert len(available) == 1
        assert available[0].adapter_id == "ollama:llama3.2"

    def test_get_available_for_generation(self, registry):
        """Should filter out embedding-only models."""
        gen_adapter = OllamaAdapter(model="llama3.2")
        embed_adapter = MagicMock()
        embed_adapter.model = "nomic-embed-text"
        
        registry.register(gen_adapter, available=True)
        registry.register(embed_adapter, available=True)
        
        gen_only = registry.get_available_for_generation()
        
        assert len(gen_only) == 1
        assert gen_only[0].adapter_id == "ollama:llama3.2"

    def test_get_best_for_no_benchmarks(self, registry):
        """Should fall back to preferred models when no benchmarks."""
        ollama = OllamaAdapter(model="gemma2:2b")
        
        registry.register(ollama, available=True)
        
        best = registry.get_best_for("sage", "wisdom")
        
        assert best is not None

    def test_get_best_for_with_benchmarks(self, registry):
        """Should use benchmarks when available."""
        ollama = OllamaAdapter(model="gemma2:2b")
        claude = ClaudeAdapter(model="claude-haiku")
        
        registry.register(ollama, available=True)
        registry.register(claude, available=True)
        
        # Add benchmark showing Ollama is better for this task
        benchmark = BenchmarkResult(
            llm_id="ollama:gemma2:2b",
            dog_id="sage",
            task_type="wisdom",
            quality_score=55.0,
            speed_score=0.9,
            cost_score=1.0,
        )
        registry.update_benchmark("sage", "wisdom", "ollama:gemma2:2b", benchmark)
        
        best = registry.get_best_for("sage", "wisdom")
        
        assert best is not None
        assert best.model == "gemma2:2b"

    def test_update_benchmark(self, registry):
        """Should update benchmark scores."""
        benchmark = BenchmarkResult(
            llm_id="ollama:test",
            dog_id="sage",
            task_type="wisdom",
            quality_score=50.0,
            speed_score=0.5,
            cost_score=0.8,
        )
        
        registry.update_benchmark("sage", "wisdom", "ollama:test", benchmark)
        
        key = ("sage", "wisdom", "ollama:test")
        assert key in registry._benchmarks

    def test_update_benchmark_ema(self, registry):
        """Should EMA update existing benchmarks."""
        benchmark1 = BenchmarkResult(
            llm_id="ollama:test",
            dog_id="sage",
            task_type="wisdom",
            quality_score=50.0,
            speed_score=0.5,
            cost_score=0.8,
        )
        
        benchmark2 = BenchmarkResult(
            llm_id="ollama:test",
            dog_id="sage",
            task_type="wisdom",
            quality_score=60.0,  # Higher
            speed_score=0.6,
            cost_score=0.9,
        )
        
        registry.update_benchmark("sage", "wisdom", "ollama:test", benchmark1)
        registry.update_benchmark("sage", "wisdom", "ollama:test", benchmark2)
        
        key = ("sage", "wisdom", "ollama:test")
        # Should be EMA updated
        assert registry._benchmarks[key].quality_score != 50.0

    def test_benchmark_matrix(self, registry):
        """Should return benchmarks for specific dog/task."""
        # Add benchmarks for different tasks
        registry.update_benchmark("sage", "wisdom", "ollama:gemma", BenchmarkResult(
            llm_id="ollama:gemma", dog_id="sage", task_type="wisdom",
            quality_score=55, speed_score=0.9, cost_score=1.0,
        ))
        registry.update_benchmark("guardian", "anomaly", "ollama:gemma", BenchmarkResult(
            llm_id="ollama:gemma", dog_id="guardian", task_type="anomaly",
            quality_score=45, speed_score=0.9, cost_score=1.0,
        ))
        
        matrix = registry.benchmark_matrix("sage", "wisdom")
        
        assert "ollama:gemma" in matrix

    def test_get_for_temporal_mcts(self, registry):
        """Should get best adapter for temporal MCTS."""
        adapter = OllamaAdapter(model="gemma2:2b")
        registry.register(adapter, available=True)
        
        result = registry.get_for_temporal_mcts()
        
        assert result is not None


class TestBenchmarkResult:
    """Test suite for BenchmarkResult."""

    def test_default_values(self):
        """Should have correct defaults."""
        result = BenchmarkResult(
            llm_id="test",
            dog_id="sage",
            task_type="wisdom",
            quality_score=50.0,
            speed_score=0.5,
            cost_score=0.8,
        )
        
        assert result.error_rate == 0.0
        assert result.sample_count == 1

    def test_composite_score(self):
        """Should compute composite score with φ-weighting."""
        result = BenchmarkResult(
            llm_id="test",
            dog_id="sage",
            task_type="wisdom",
            quality_score=61.8,  # Max φ-bounded
            speed_score=1.0,
            cost_score=1.0,
        )
        
        score = result.composite_score
        
        assert score > 0
        assert score <= 1.0

    def test_ema_update(self):
        """Should EMA update with new result."""
        result1 = BenchmarkResult(
            llm_id="test",
            dog_id="sage",
            task_type="wisdom",
            quality_score=50.0,
            speed_score=0.5,
            cost_score=0.8,
        )
        
        result2 = BenchmarkResult(
            llm_id="test",
            dog_id="sage",
            task_type="wisdom",
            quality_score=70.0,
            speed_score=0.7,
            cost_score=0.9,
        )
        
        updated = result1.ema_update(result2, alpha=0.3)
        
        assert updated.quality_score > 50.0
        assert updated.sample_count == 2


class TestPreferredModels:
    """Test suite for PREFERRED_MODELS."""

    def test_preferred_models_defined(self):
        """Should have preferred models for key tasks."""
        assert "temporal_mcts" in PREFERRED_MODELS
        assert "wisdom" in PREFERRED_MODELS
        assert "default" in PREFERRED_MODELS

    def test_gemma2_is_fast(self):
        """gemma2:2b should be the fast model for parallel tasks."""
        assert PREFERRED_MODELS["temporal_mcts"] == "gemma2:2b"
        assert PREFERRED_MODELS["wisdom"] == "gemma2:2b"

    def test_mistral_for_deep(self):
        """mistral should be used for deep analysis."""
        assert "mistral" in PREFERRED_MODELS["deep_analysis"]
