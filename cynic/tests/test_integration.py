"""
Integration tests — real Ollama (gemma2:2b) required.

Run:   pytest -m integration
Skip:  pytest (default — no integration tests run)

These tests hit real infrastructure:
    - Ollama HTTP API (localhost:11434)
    - gemma2:2b model for temporal scoring

Validates the full pipeline that unit tests mock:
    temporal_judgment() → asyncio.gather() → 7 real Ollama calls → scores
    SageDog → LLMRegistry.get_best_for() → temporal_judgment() → DogJudgment
"""
from __future__ import annotations

import asyncio
import pytest

from cynic.core.judgment import Cell
from cynic.core.phi import MAX_Q_SCORE, PHI_INV, PHI_INV_2
from cynic.llm.adapter import LLMRegistry, OllamaAdapter
from cynic.llm.temporal import (
    TemporalJudgment,
    TemporalPerspective,
    fast_temporal_judgment,
    temporal_judgment,
)

pytestmark = pytest.mark.integration


# ════════════════════════════════════════════════════════════════════════════
# TEST CONTENT
# ════════════════════════════════════════════════════════════════════════════

GOOD_CODE = """\
from cynic.core.phi import PHI, PHI_INV, phi_bound_score


def phi_aggregate(scores: dict, weights: dict) -> float:
    \"\"\"phi-weighted geometric mean. Preserves [0, MAX_Q] range.\"\"\"
    import math
    log_sum = sum(weights[k] * math.log(max(v, 0.1)) for k, v in scores.items())
    total_weight = sum(weights.values())
    return phi_bound_score(math.exp(log_sum / total_weight))


def fibonacci(n: int) -> int:
    \"\"\"F(n) exact, no recursion, no memoization needed for n < 20.\"\"\"
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    return a
"""

# Bad code: poor naming, globals, bare except, deep nesting, dead markers.
# Intentionally low quality — used as scoring baseline, not executed.
BAD_CODE = "\n".join([
    "import sys, json",
    "x = []",
    "cred = 'hunter2'",
    "def f(a,b,c,d,e,f,g,h):",
    "    global x",
    "    try:",
    "        for i in range(9999):",
    "            for j in range(9999):",
    "                for k in range(9999):",
    "                    x.append(i*j*k)",
    "                    print(x)",
    "    except:",
    "        pass",
    "    return None",
    "# TODO fix this",
    "# FIXME broken",
    "# HACK temporary",
])


# ════════════════════════════════════════════════════════════════════════════
# FIXTURES
# ════════════════════════════════════════════════════════════════════════════

@pytest.fixture(scope="module")
def ollama_available() -> bool:
    """Check once per module if Ollama + gemma2:2b is reachable."""
    async def _check() -> bool:
        return await OllamaAdapter("gemma2:2b").check_available()
    return asyncio.run(_check())


@pytest.fixture
def gemma2(ollama_available: bool) -> OllamaAdapter:
    """Real gemma2:2b adapter — skips if Ollama not running."""
    if not ollama_available:
        pytest.skip("Ollama not available at localhost:11434")
    return OllamaAdapter("gemma2:2b")


@pytest.fixture
def registry_with_gemma2(gemma2: OllamaAdapter) -> LLMRegistry:
    """LLMRegistry with gemma2:2b registered and available."""
    registry = LLMRegistry()
    registry.register(gemma2, available=True)
    return registry


def _make_cell(content: str, reality: str = "CODE") -> Cell:
    return Cell(
        reality=reality,
        analysis="JUDGE",
        content=content,
        novelty=0.3,
        complexity=0.5,
        risk=0.2,
        budget_usd=0.1,
    )


# ════════════════════════════════════════════════════════════════════════════
# temporal_judgment — 7 parallel real Ollama calls
# ════════════════════════════════════════════════════════════════════════════

class TestTemporalJudgmentReal:
    async def test_returns_temporal_judgment(self, gemma2):
        result = await temporal_judgment(gemma2, GOOD_CODE)
        assert isinstance(result, TemporalJudgment)

    async def test_7_perspectives_populated(self, gemma2):
        result = await temporal_judgment(gemma2, GOOD_CODE)
        assert len(result.scores) == 7
        assert all(v > 0 for v in result.scores.values())

    async def test_phi_aggregate_in_range(self, gemma2):
        result = await temporal_judgment(gemma2, GOOD_CODE)
        assert 0 < result.phi_aggregate <= MAX_Q_SCORE

    async def test_confidence_bounded_by_phi_inv(self, gemma2):
        result = await temporal_judgment(gemma2, GOOD_CODE)
        assert 0 < result.confidence <= PHI_INV

    async def test_latency_recorded(self, gemma2):
        result = await temporal_judgment(gemma2, GOOD_CODE)
        assert result.latency_ms > 0

    async def test_llm_id_contains_gemma2(self, gemma2):
        result = await temporal_judgment(gemma2, GOOD_CODE)
        assert "gemma2" in result.llm_id

    async def test_failed_count_acceptable(self, gemma2):
        """gemma2:2b validated — at most 2 perspective failures expected."""
        result = await temporal_judgment(gemma2, GOOD_CODE)
        assert result.failed_count <= 2, (
            f"Too many failures ({result.failed_count}/7) — "
            "check prompt or model availability"
        )

    async def test_good_code_above_neutral(self, gemma2):
        result = await temporal_judgment(gemma2, GOOD_CODE)
        assert result.phi_aggregate > MAX_Q_SCORE * 0.40

    async def test_bad_code_below_good(self, gemma2):
        """Bad code (deep nesting, globals, dead markers) must score lower."""
        good = await temporal_judgment(gemma2, GOOD_CODE)
        bad = await temporal_judgment(gemma2, BAD_CODE)
        assert good.phi_aggregate > bad.phi_aggregate, (
            f"Expected good > bad: {good.phi_aggregate:.1f} vs {bad.phi_aggregate:.1f}"
        )

    async def test_discrimination_threshold(self, gemma2):
        """Core hypothesis: temporal MCTS discriminates quality. Diff > 15 Q."""
        good = await temporal_judgment(gemma2, GOOD_CODE)
        bad = await temporal_judgment(gemma2, BAD_CODE)
        diff = good.phi_aggregate - bad.phi_aggregate
        assert diff > 15.0, (
            f"Discrimination too weak: {diff:.1f} Q points "
            f"(good={good.phi_aggregate:.1f}, bad={bad.phi_aggregate:.1f})"
        )


# ════════════════════════════════════════════════════════════════════════════
# fast_temporal_judgment — 3 perspectives (MICRO path)
# ════════════════════════════════════════════════════════════════════════════

class TestFastTemporalJudgmentReal:
    async def test_returns_temporal_judgment(self, gemma2):
        result = await fast_temporal_judgment(gemma2, GOOD_CODE)
        assert isinstance(result, TemporalJudgment)

    async def test_phi_aggregate_in_range(self, gemma2):
        result = await fast_temporal_judgment(gemma2, GOOD_CODE)
        assert 0 < result.phi_aggregate <= MAX_Q_SCORE

    async def test_at_least_2_perspectives_non_neutral(self, gemma2):
        """Fast path: PRESENT, FUTURE, NEVER scored — rest stay neutral."""
        result = await fast_temporal_judgment(gemma2, GOOD_CODE)
        neutral = MAX_Q_SCORE * 0.5
        scored = [
            p for p, v in result.scores.items()
            if abs(v - neutral) > 1.0
        ]
        assert len(scored) >= 2


# ════════════════════════════════════════════════════════════════════════════
# SageDog — end-to-end with real LLMRegistry + gemma2:2b
# ════════════════════════════════════════════════════════════════════════════

class TestSageDogReal:
    async def test_temporal_path_activated(self, registry_with_gemma2):
        """With real registry → temporal MCTS path (confidence > heuristic max)."""
        from cynic.cognition.neurons.sage import SageDog
        dog = SageDog()
        dog.set_llm_registry(registry_with_gemma2)

        result = await dog.analyze(_make_cell(GOOD_CODE))
        assert result.q_score > 0
        assert result.confidence <= PHI_INV
        assert result.confidence > PHI_INV_2, (
            f"Heuristic fallback triggered? conf={result.confidence:.3f} "
            f"(expected > {PHI_INV_2:.3f} for temporal path)"
        )

    async def test_good_code_scores_higher_than_bad(self, registry_with_gemma2):
        from cynic.cognition.neurons.sage import SageDog
        dog = SageDog()
        dog.set_llm_registry(registry_with_gemma2)

        good = await dog.analyze(_make_cell(GOOD_CODE))
        bad = await dog.analyze(_make_cell(BAD_CODE))
        assert good.q_score > bad.q_score, (
            f"Expected good ({good.q_score:.1f}) > bad ({bad.q_score:.1f})"
        )

    async def test_temporal_evidence_in_judgment(self, registry_with_gemma2):
        """Evidence dict must contain 'path': 'temporal_mcts'."""
        from cynic.cognition.neurons.sage import SageDog
        dog = SageDog()
        dog.set_llm_registry(registry_with_gemma2)

        result = await dog.analyze(_make_cell(GOOD_CODE))
        assert result.evidence is not None
        assert result.evidence.get("path") == "temporal_mcts"


# ════════════════════════════════════════════════════════════════════════════
# LLMRegistry.discover() — finds real models from Ollama
# ════════════════════════════════════════════════════════════════════════════

class TestRegistryDiscovery:
    async def test_discover_finds_at_least_one_model(self, ollama_available):
        if not ollama_available:
            pytest.skip("Ollama not available")
        registry = LLMRegistry()
        found = await registry.discover()
        assert len(found) > 0

    async def test_discover_includes_gemma2(self, ollama_available):
        if not ollama_available:
            pytest.skip("Ollama not available")
        registry = LLMRegistry()
        await registry.discover()
        gen_adapters = registry.get_available_for_generation()
        models = [a.model for a in gen_adapters]
        assert any("gemma2" in m for m in models), (
            f"gemma2:2b not found in discovered models: {models}"
        )

    async def test_nomic_excluded_from_generation(self, ollama_available):
        """nomic-embed-text must not appear in generation adapters."""
        if not ollama_available:
            pytest.skip("Ollama not available")
        registry = LLMRegistry()
        await registry.discover()
        gen_adapters = registry.get_available_for_generation()
        models = [a.model for a in gen_adapters]
        assert not any("nomic" in m for m in models), (
            f"nomic-embed-text leaked into generation adapters: {models}"
        )

    async def test_get_for_temporal_mcts_returns_gemma2(self, ollama_available):
        if not ollama_available:
            pytest.skip("Ollama not available")
        registry = LLMRegistry()
        await registry.discover()
        adapter = registry.get_for_temporal_mcts()
        assert adapter is not None
        assert "gemma2" in adapter.model
