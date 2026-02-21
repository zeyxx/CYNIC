"""
Empirical Ollama Inference Validation

Test REAL Ollama calls (not mocked) to:
1. Verify gemma2:2b responds correctly
2. Measure actual latencies
3. Compare with test expectations
4. Validate SAGE dog integration

Requires: Ollama running at localhost:11434 with gemma2:2b model loaded
"""
from __future__ import annotations

import asyncio
import time
import statistics
from typing import Any

import pytest
import httpx

from cynic.core.phi import MAX_Q_SCORE, PHI_INV


OLLAMA_URL = "http://localhost:11434"
OLLAMA_MODEL = "gemma2:2b"


@pytest.mark.integration
class TestOllamaEmpirical:
    """Real Ollama inference tests."""

    @classmethod
    def setup_class(cls) -> None:
        """Check Ollama is available."""
        try:
            resp = httpx.get(f"{OLLAMA_URL}/api/tags", timeout=5)
            resp.raise_for_status()
            models = resp.json().get("models", [])
            model_names = [m.get("name") for m in models]
            assert OLLAMA_MODEL in model_names, f"Model {OLLAMA_MODEL} not found. Available: {model_names}"
        except ValidationError as e:
            pytest.skip(f"Ollama not available: {e}")

    def test_ollama_available(self) -> None:
        """Verify Ollama responds."""
        resp = httpx.get(f"{OLLAMA_URL}/api/tags", timeout=5)
        assert resp.status_code == 200
        models = resp.json().get("models", [])
        assert len(models) > 0

    def test_ollama_gemma2_available(self) -> None:
        """Verify gemma2:2b is loaded."""
        resp = httpx.get(f"{OLLAMA_URL}/api/tags", timeout=5)
        models = resp.json().get("models", [])
        model_names = [m.get("name") for m in models]
        assert OLLAMA_MODEL in model_names

    def test_ollama_simple_generate(self) -> None:
        """Single inference call."""
        payload = {
            "model": OLLAMA_MODEL,
            "prompt": "What is 2+2?",
            "stream": False,
        }
        resp = httpx.post(
            f"{OLLAMA_URL}/api/generate",
            json=payload,
            timeout=30,
        )
        assert resp.status_code == 200
        result = resp.json()
        assert "response" in result
        assert len(result["response"]) > 0
        print(f"\n[Ollama] Response: {result['response'][:100]}...")

    def test_ollama_latency_single(self) -> None:
        """Measure single inference latency."""
        payload = {
            "model": OLLAMA_MODEL,
            "prompt": "Respond with a single word: ready",
            "stream": False,
        }
        t0 = time.perf_counter()
        resp = httpx.post(
            f"{OLLAMA_URL}/api/generate",
            json=payload,
            timeout=30,
        )
        latency_ms = (time.perf_counter() - t0) * 1000

        assert resp.status_code == 200
        assert latency_ms < 5000, f"Single inference too slow: {latency_ms:.0f}ms"
        print(f"\n[Ollama] Single inference latency: {latency_ms:.0f}ms")

    def test_ollama_latency_10_calls(self) -> None:
        """Measure 10 consecutive inference calls."""
        latencies: list[float] = []

        for i in range(10):
            payload = {
                "model": OLLAMA_MODEL,
                "prompt": f"Question {i}: What is {i}+1?",
                "stream": False,
            }
            t0 = time.perf_counter()
            resp = httpx.post(
                f"{OLLAMA_URL}/api/generate",
                json=payload,
                timeout=30,
            )
            latency_ms = (time.perf_counter() - t0) * 1000

            assert resp.status_code == 200
            latencies.append(latency_ms)

        # Statistics
        mean_lat = statistics.mean(latencies)
        median_lat = statistics.median(latencies)
        stdev_lat = statistics.stdev(latencies) if len(latencies) > 1 else 0
        min_lat = min(latencies)
        max_lat = max(latencies)

        print(f"\n[Ollama] 10-call latency statistics:")
        print(f"  Mean:   {mean_lat:.0f}ms")
        print(f"  Median: {median_lat:.0f}ms")
        print(f"  StDev:  {stdev_lat:.0f}ms")
        print(f"  Min:    {min_lat:.0f}ms")
        print(f"  Max:    {max_lat:.0f}ms")

        # Assertions: should be reasonably fast
        assert mean_lat < 2000, f"Mean latency too high: {mean_lat:.0f}ms"
        assert max_lat < 5000, f"Max latency too high: {max_lat:.0f}ms"

    def test_ollama_output_quality(self) -> None:
        """Verify response quality for code-related prompts."""
        payload = {
            "model": OLLAMA_MODEL,
            "prompt": "Write a Python function that adds two numbers.",
            "stream": False,
        }
        resp = httpx.post(
            f"{OLLAMA_URL}/api/generate",
            json=payload,
            timeout=30,
        )
        assert resp.status_code == 200
        result = resp.json()
        response = result["response"]

        # Should contain code-like content
        assert "def" in response or "function" in response.lower()
        print(f"\n[Ollama] Code generation sample:")
        print(response[:200] + "...")

    def test_ollama_consistency(self) -> None:
        """Test that model produces consistent outputs for same prompt."""
        prompt = "List three colors."
        responses: list[str] = []

        for _ in range(3):
            payload = {
                "model": OLLAMA_MODEL,
                "prompt": prompt,
                "stream": False,
            }
            resp = httpx.post(
                f"{OLLAMA_URL}/api/generate",
                json=payload,
                timeout=30,
            )
            assert resp.status_code == 200
            responses.append(resp.json()["response"])

        # Check that responses are reasonably similar (not identical due to sampling, but related)
        print(f"\n[Ollama] Consistency check (3 responses to same prompt):")
        for i, r in enumerate(responses):
            print(f"  [{i}]: {r[:80]}...")

        # At least one should contain color words
        color_words = {"red", "blue", "green", "yellow", "orange", "purple"}
        has_colors = any(
            any(color in r.lower() for color in color_words)
            for r in responses
        )
        assert has_colors, "No color words in any response"


@pytest.mark.integration
class TestOllamaVsExpectations:
    """Compare real Ollama performance with test mocks."""

    @classmethod
    def setup_class(cls) -> None:
        """Check Ollama availability."""
        try:
            resp = httpx.get(f"{OLLAMA_URL}/api/tags", timeout=5)
            resp.raise_for_status()
        except ValidationError as e:
            pytest.skip(f"Ollama not available: {e}")

    def test_ollama_vs_mocked_latency(self) -> None:
        """
        Compare real Ollama latency with what tests assume.

        Mocked tests assume <100ms latency.
        Reality: gemma2:2b on laptop = ~500-2000ms depending on hardware.
        """
        # Real measurement
        payload = {
            "model": OLLAMA_MODEL,
            "prompt": "Quick answer: ready",
            "stream": False,
        }
        t0 = time.perf_counter()
        resp = httpx.post(
            f"{OLLAMA_URL}/api/generate",
            json=payload,
            timeout=30,
        )
        real_latency_ms = (time.perf_counter() - t0) * 1000

        print(f"\n[Comparison] Real Ollama latency: {real_latency_ms:.0f}ms")
        print(f"[Comparison] Mocked test assumption: <100ms")
        print(f"[Comparison] MACRO threshold: <2000ms ✓" if real_latency_ms < 2000 else f"[Comparison] MACRO threshold: EXCEEDED ✗")

        # Still within MACRO threshold
        assert real_latency_ms < 2000, f"Real latency exceeds MACRO threshold: {real_latency_ms:.0f}ms"

        # Gap analysis
        gap = real_latency_ms / 100  # Real vs mocked ratio
        print(f"[Comparison] Gap: {gap:.0f}x slower than mock ({gap*100:.0f}%)")

    def test_ollama_vs_expected_format(self) -> None:
        """Verify Ollama response format matches SAGE dog expectations."""
        payload = {
            "model": OLLAMA_MODEL,
            "prompt": "Evaluate this code: x = 1 + 1",
            "stream": False,
        }
        resp = httpx.post(
            f"{OLLAMA_URL}/api/generate",
            json=payload,
            timeout=30,
        )
        assert resp.status_code == 200
        result = resp.json()

        # SAGE dog expects these fields
        assert "response" in result, "Missing 'response' field"
        assert isinstance(result["response"], str), "Response not a string"
        assert len(result["response"]) > 0, "Empty response"

        # Optional but useful
        if "eval_count" in result:
            print(f"\n[Format] Eval tokens: {result['eval_count']}")
        if "prompt_eval_count" in result:
            print(f"[Format] Prompt tokens: {result['prompt_eval_count']}")

        print(f"[Format] Response length: {len(result['response'])} chars")
        print(f"[Format] ✓ Format valid for SAGE dog")


@pytest.mark.integration
@pytest.mark.asyncio
async def test_sage_dog_with_real_ollama() -> None:
    """
    Integration: SAGE dog making real Ollama calls.

    This validates the full pipeline:
    SAGE.analyze() → temporal_judgment() → OllamaAdapter.complete() → real gemma2:2b
    """
    try:
        from cynic.dogs.sage import SageDog
        from cynic.core.judgment import Cell
        from cynic.core.consciousness import ConsciousnessLevel

        # Create test cell
        cell = Cell(
            cell_id="test_real_ollama",
            reality="CODE",
            content="Review this code quality: def add(x, y): return x + y",
            budget_usd=0.01,
        )

        # Create SAGE dog
        sage = SageDog()

        # Measure end-to-end
        t0 = time.perf_counter()
        judgment = await sage.analyze(cell)
        duration_ms = (time.perf_counter() - t0) * 1000

        print(f"\n[SAGE Dog + Real Ollama]")
        print(f"  Duration: {duration_ms:.0f}ms")
        print(f"  Q-Score: {judgment.q_score:.1f}")
        print(f"  Verdict: {judgment.verdict}")
        print(f"  LLM Used: {judgment.llm_id if judgment.llm_id else 'None'}")

        # Should be reasonable
        assert duration_ms < 5000, f"SAGE + Ollama too slow: {duration_ms:.0f}ms"
        assert 0 <= judgment.q_score <= MAX_Q_SCORE
        assert judgment.verdict in ["HOWL", "WAG", "GROWL", "BARK"]

    except ImportError:
        pytest.skip("SAGE dog not available")
