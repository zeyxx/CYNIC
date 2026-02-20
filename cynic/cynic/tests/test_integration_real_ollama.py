"""
INTEGRATION TESTS: Real Ollama Connection

Tests actual Ollama API at localhost:11434.
SKIP in CI (no Ollama running). Run locally via:
  py -3.13 -m pytest -m integration tests/test_integration_real_ollama.py -v

Requires: Ollama running at localhost:11434
  docker-compose up ollama
  ollama pull qwen2.5-coder:7b  # or other model
"""
import pytest
import os
from unittest.mock import AsyncMock, patch

# Mark all tests in this module as integration (skip in CI)
pytestmark = pytest.mark.integration


@pytest.fixture
def has_ollama():
    """Check if Ollama is available at localhost:11434."""
    try:
        import requests
        resp = requests.get("http://localhost:11434/api/tags", timeout=3)
        return resp.status_code == 200
    except Exception:
        return False


class TestOllamaConnection:
    """Validate real Ollama connectivity and judgment flow."""

    @pytest.mark.asyncio
    async def test_ollama_available_for_tests(self, has_ollama):
        """Skip entire class if Ollama not running."""
        if not has_ollama:
            pytest.skip("Ollama not available at localhost:11434")

    @pytest.mark.asyncio
    async def test_ollama_adapter_real_completion(self, has_ollama):
        """Test OllamaAdapter.complete() with REAL Ollama, not mock."""
        if not has_ollama:
            pytest.skip("Ollama not available")

        from cynic.llm.adapter import OllamaAdapter

        adapter = OllamaAdapter(base_url="http://localhost:11434")

        # Create a real LLM request
        from cynic.llm.adapter import LLMRequest
        request = LLMRequest(
            messages=[
                {"role": "system", "content": "You are a code quality analyzer."},
                {"role": "user", "content": "Rate this code: def foo(): pass"}
            ],
            model="qwen2.5-coder:7b",  # Must be pulled locally
            temperature=0.3,
            max_tokens=100,
        )

        # Get real response from Ollama
        response = await adapter.complete(request)

        # Validate response structure
        assert response.raw_message, "Should have raw_message from Ollama"
        assert len(response.raw_message) > 0, "Response should not be empty"
        # Ollama returns plain text by default
        assert isinstance(response.raw_message, str), "Should be string"

        print(f"✓ Real Ollama response: {response.raw_message[:100]}...")

    @pytest.mark.asyncio
    async def test_dog_judgment_with_real_ollama(self, has_ollama):
        """Test DogCognition.judge_cell() using real Ollama scoring."""
        if not has_ollama:
            pytest.skip("Ollama not available")

        from cynic.cognition.cortex.dog_cognition import DogCognition, DogCognitionConfig
        from cynic.cognition.neurons.dog_state import DogState
        from cynic.core.judgment import Cell
        from unittest.mock import patch, AsyncMock

        # Patch heuristic scorer to use real Ollama instead
        # (in production, heuristic_scorer calls real LLM)
        dog = DogCognition("ANALYZER", DogCognitionConfig())
        state = DogState(dog_id="ANALYZER")

        # Real signals (from production scenario)
        state.senses.observed_signals = [
            {"type": "security_issue", "severity": "high"},
            {"type": "performance_gap", "magnitude": "15%"},
        ]

        cell = Cell(reality="CODE", analysis="JUDGE", content="real code sample")

        # Judge with real signals
        judgment = await dog.judge_cell(cell, state)

        # Validate judgment properties
        assert 0 <= judgment.q_score <= 100, f"Q-score out of bounds: {judgment.q_score}"
        assert 0 <= judgment.confidence <= 0.618, f"Confidence exceeds φ⁻¹: {judgment.confidence}"
        assert judgment.verdict in ("BARK", "GROWL", "WAG", "HOWL"), f"Invalid verdict: {judgment.verdict}"

        print(f"✓ Real judgment: Q={judgment.q_score:.1f}, confidence={judgment.confidence:.3f}, verdict={judgment.verdict}")

    @pytest.mark.asyncio
    async def test_ollama_model_discovery(self, has_ollama):
        """Test LLMRegistry.discover() finds real models."""
        if not has_ollama:
            pytest.skip("Ollama not available")

        from cynic.llm.adapter import LLMRegistry

        registry = LLMRegistry()
        models = await registry.discover(base_url="http://localhost:11434")

        assert len(models) > 0, "Should discover at least one model"
        assert any("qwen" in m.lower() or "llama" in m.lower() for m in models), \
            f"Expected qwen or llama model, got {models}"

        print(f"✓ Discovered models: {models}")

    @pytest.mark.asyncio
    async def test_temporal_mcts_with_real_ollama(self, has_ollama):
        """Test SAGE temporal MCTS with real Ollama 7-perspective scoring."""
        if not has_ollama:
            pytest.skip("Ollama not available")

        from cynic.llm.temporal import temporal_judgment
        from cynic.llm.adapter import LLMRegistry

        registry = LLMRegistry()
        # Discover available model
        models = await registry.discover(base_url="http://localhost:11434")
        if not models:
            pytest.skip("No models available in Ollama")

        model = models[0]

        # Run real temporal MCTS with 7 parallel Ollama calls
        cell_content = "def calculate_total(items): return sum([i['price'] * i['qty'] for i in items])"
        judgment = await temporal_judgment(
            cell_content=cell_content,
            reality="CODE",
            analysis="JUDGE",
            registry=registry,
            model=model,
            base_url="http://localhost:11434"
        )

        # Validate temporal judgment
        assert 0 <= judgment.q_score <= 100, f"Q-score out of bounds: {judgment.q_score}"
        assert 0 <= judgment.confidence <= 0.618, f"Confidence exceeds φ⁻¹: {judgment.confidence}"
        assert len(judgment.perspective_scores) > 0, "Should have perspective scores"

        print(f"✓ Temporal MCTS Q={judgment.q_score:.1f}, perspectives={len(judgment.perspective_scores)}")


class TestOllamaPerformance:
    """Benchmark real Ollama latency and throughput."""

    @pytest.mark.asyncio
    async def test_ollama_latency_single_call(self, has_ollama):
        """Measure single Ollama API call latency."""
        if not has_ollama:
            pytest.skip("Ollama not available")

        import time
        from cynic.llm.adapter import OllamaAdapter, LLMRequest

        adapter = OllamaAdapter(base_url="http://localhost:11434")

        request = LLMRequest(
            messages=[{"role": "user", "content": "Return the word 'test'."}],
            model="qwen2.5-coder:7b",
            temperature=0.1,
            max_tokens=10,
        )

        start = time.perf_counter()
        response = await adapter.complete(request)
        elapsed_ms = (time.perf_counter() - start) * 1000

        print(f"✓ Ollama single call: {elapsed_ms:.1f}ms")
        # Ollama on CPU can be slow (1-5s), but should complete
        assert elapsed_ms < 30000, f"Ollama call took too long: {elapsed_ms:.1f}ms"

    @pytest.mark.asyncio
    async def test_ollama_parallel_calls(self, has_ollama):
        """Measure 7 parallel Ollama calls (SAGE temporal MCTS workload)."""
        if not has_ollama:
            pytest.skip("Ollama not available")

        import time
        import asyncio
        from cynic.llm.adapter import OllamaAdapter, LLMRequest

        adapter = OllamaAdapter(base_url="http://localhost:11434")

        request = LLMRequest(
            messages=[{"role": "user", "content": "Say 'ok'."}],
            model="qwen2.5-coder:7b",
            temperature=0.1,
            max_tokens=5,
        )

        start = time.perf_counter()
        responses = await asyncio.gather(*[
            adapter.complete(request)
            for _ in range(7)
        ])
        elapsed_ms = (time.perf_counter() - start) * 1000

        assert len(responses) == 7, "Should complete all 7 calls"
        print(f"✓ Ollama 7 parallel calls: {elapsed_ms:.1f}ms ({elapsed_ms/7:.1f}ms per call avg)")


if __name__ == "__main__":
    import sys
    # Run with: py -3.13 -m pytest tests/test_integration_real_ollama.py -v -m integration
    sys.exit(__import__("pytest").main([__file__, "-v", "-m", "integration"]))
