"""
Tests for Temporal MCTS Engine — llm/temporal.py

The novel core of CYNIC: 7 parallel temporal perspectives via asyncio.gather().
Tests verify: parallel execution, φ-aggregate, confidence, fail-safe behavior.
"""
from __future__ import annotations

import asyncio
from unittest.mock import AsyncMock, MagicMock

import pytest

from cynic.core.phi import MAX_Q_SCORE, PHI_INV, PHI_INV_2
from cynic.llm.temporal import (
    TemporalJudgment,
    TemporalPerspective,
    _parse_score,
    fast_temporal_judgment,
    temporal_judgment,
)


# ════════════════════════════════════════════════════════════════════════════
# HELPERS
# ════════════════════════════════════════════════════════════════════════════

def _mock_adapter(score_value: float = 45.0) -> MagicMock:
    """Mock LLM adapter that returns a fixed score."""
    mock = MagicMock()
    mock.adapter_id = "ollama:llama3.2"
    resp = MagicMock()
    resp.content = f"SCORE: {score_value}"

    async def _complete_safe(req):
        return resp

    mock.complete_safe = _complete_safe
    return mock


def _mock_adapter_counting(base_score: float = 40.0) -> tuple[MagicMock, list]:
    """Mock adapter that counts calls and returns incremented scores."""
    mock = MagicMock()
    mock.adapter_id = "ollama:llama3.2"
    calls = []

    async def _complete_safe(req):
        calls.append(req)
        r = MagicMock()
        r.content = f"SCORE: {base_score + len(calls)}"
        return r

    mock.complete_safe = _complete_safe
    return mock, calls


# ════════════════════════════════════════════════════════════════════════════
# _parse_score tests
# ════════════════════════════════════════════════════════════════════════════

class TestParseScore:
    """
    LLM is asked for 0-100 scale. _parse_score converts to [0, MAX_Q_SCORE].
    Conversion: val_100 / 100 * MAX_Q_SCORE (61.8)
    """

    def test_standard_format(self):
        # LLM says "SCORE: 45" → 45/100 * 61.8 = 27.81
        result = _parse_score("SCORE: 45")
        assert result is not None
        assert abs(result - 45 / 100 * MAX_Q_SCORE) < 0.01

    def test_case_insensitive(self):
        result = _parse_score("score: 50")
        assert result is not None
        assert abs(result - 50 / 100 * MAX_Q_SCORE) < 0.01

    def test_integer_score(self):
        result = _parse_score("SCORE: 50")
        assert result is not None
        assert abs(result - MAX_Q_SCORE / 2) < 0.01  # 50% → half of max

    def test_score_in_sentence(self):
        result = _parse_score("Based on analysis: SCORE: 80 (above average)")
        assert result is not None
        assert abs(result - 80 / 100 * MAX_Q_SCORE) < 0.01

    def test_bare_number_fallback(self):
        # Bare number treated as 0-100 scale
        result = _parse_score("75")
        assert result is not None
        assert abs(result - 75 / 100 * MAX_Q_SCORE) < 0.01

    def test_no_score(self):
        assert _parse_score("This is good code") is None

    def test_empty_string(self):
        assert _parse_score("") is None

    def test_caps_at_max_q_score(self):
        # LLM says 100 → MAX_Q_SCORE (61.8)
        result = _parse_score("SCORE: 100")
        assert result == MAX_Q_SCORE

    def test_rejects_exact_100_bare(self):
        # Bare "100" likely a prompt echo — rejected
        result = _parse_score("100")
        assert result is None


# ════════════════════════════════════════════════════════════════════════════
# TemporalJudgment tests
# ════════════════════════════════════════════════════════════════════════════

class TestTemporalJudgment:
    def test_phi_aggregate_range(self):
        tj = TemporalJudgment(
            past=40, present=45, future=38,
            ideal=55, never=50, cycles=42, flow=44,
        )
        agg = tj.phi_aggregate
        assert 0 < agg <= MAX_Q_SCORE

    def test_phi_aggregate_high_scores(self):
        # All high → aggregate also high (geometric mean preserves range)
        tj = TemporalJudgment(
            past=55, present=58, future=56,
            ideal=60, never=57, cycles=54, flow=55,
        )
        agg = tj.phi_aggregate
        assert agg > 50.0  # High scores → high aggregate (~55-58)

    def test_phi_aggregate_low_scores(self):
        # All low → aggregate close to 0
        tj = TemporalJudgment(
            past=5, present=6, future=4,
            ideal=8, never=5, cycles=6, flow=5,
        )
        agg = tj.phi_aggregate
        assert agg < 20.0

    def test_confidence_range(self):
        tj = TemporalJudgment(
            past=40, present=45, future=38,
            ideal=55, never=50, cycles=42, flow=44,
        )
        conf = tj.confidence
        assert 0 < conf <= PHI_INV

    def test_confidence_high_when_aligned(self):
        # All scores same → maximum agreement → high confidence
        tj = TemporalJudgment(
            past=45, present=45, future=45,
            ideal=45, never=45, cycles=45, flow=45,
        )
        assert tj.confidence > 0.5

    def test_confidence_low_when_divergent(self):
        # Very spread scores → low confidence
        tj = TemporalJudgment(
            past=5, present=60, future=10,
            ideal=58, never=8, cycles=55, flow=12,
        )
        assert tj.confidence < 0.4

    def test_confidence_penalized_for_failures(self):
        tj_no_fail = TemporalJudgment(
            past=40, present=41, future=42,
            ideal=43, never=44, cycles=45, flow=46,
            failed_count=0,
        )
        tj_with_fail = TemporalJudgment(
            past=40, present=41, future=42,
            ideal=43, never=44, cycles=45, flow=46,
            failed_count=3,
        )
        assert tj_with_fail.confidence < tj_no_fail.confidence

    def test_scores_dict(self):
        tj = TemporalJudgment(past=40, present=45, future=38, ideal=55, never=50, cycles=42, flow=44)
        scores = tj.scores
        assert len(scores) == 7
        assert scores[TemporalPerspective.PAST] == 40
        assert scores[TemporalPerspective.IDEAL] == 55

    def test_to_dict(self):
        tj = TemporalJudgment(
            past=40, present=45, future=38, ideal=55,
            never=50, cycles=42, flow=44,
            llm_id="ollama:llama3.2",
            latency_ms=125.3,
            failed_count=0,
        )
        d = tj.to_dict()
        assert d["path"] == "temporal_mcts"
        assert d["llm_id"] == "ollama:llama3.2"
        assert len(d["scores"]) == 7
        assert 0 < d["phi_aggregate"] <= MAX_Q_SCORE
        assert 0 < d["confidence"] <= PHI_INV


# ════════════════════════════════════════════════════════════════════════════
# temporal_judgment (7 parallel calls) tests
# ════════════════════════════════════════════════════════════════════════════

class TestTemporalJudgmentFunction:
    @pytest.mark.asyncio
    async def test_makes_7_parallel_calls(self):
        """Core assertion: exactly 7 LLM calls are made."""
        adapter, calls = _mock_adapter_counting(40.0)
        await temporal_judgment(adapter, "def foo(): return 42")
        assert len(calls) == 7

    @pytest.mark.asyncio
    async def test_result_is_temporal_judgment(self):
        adapter = _mock_adapter(45.0)
        result = await temporal_judgment(adapter, "test content")
        assert isinstance(result, TemporalJudgment)

    @pytest.mark.asyncio
    async def test_phi_aggregate_in_range(self):
        adapter = _mock_adapter(45.0)
        result = await temporal_judgment(adapter, "test")
        assert 0 < result.phi_aggregate <= MAX_Q_SCORE

    @pytest.mark.asyncio
    async def test_llm_id_set(self):
        adapter = _mock_adapter(40.0)
        result = await temporal_judgment(adapter, "test")
        assert result.llm_id == "ollama:llama3.2"

    @pytest.mark.asyncio
    async def test_latency_recorded(self):
        adapter = _mock_adapter(40.0)
        result = await temporal_judgment(adapter, "test")
        assert result.latency_ms >= 0

    @pytest.mark.asyncio
    async def test_graceful_on_llm_failure(self):
        """Failing LLM returns neutral score — never raises."""
        mock = MagicMock()
        mock.adapter_id = "ollama:llama3.2"

        async def _failing_complete(req):
            r = MagicMock()
            r.content = ""  # No parseable score
            return r

        mock.complete_safe = _failing_complete

        result = await temporal_judgment(mock, "test content")
        # Should complete without error, with neutral scores
        assert isinstance(result, TemporalJudgment)
        assert result.phi_aggregate > 0  # Neutral scores (0.5 × MAX) → non-zero aggregate

    @pytest.mark.asyncio
    async def test_custom_perspectives(self):
        """Subset of perspectives uses fewer calls."""
        adapter, calls = _mock_adapter_counting(40.0)
        await temporal_judgment(
            adapter, "test",
            perspectives=[TemporalPerspective.PRESENT, TemporalPerspective.FUTURE],
        )
        assert len(calls) == 2

    @pytest.mark.asyncio
    async def test_all_scores_populated(self):
        adapter = _mock_adapter(45.0)
        result = await temporal_judgment(adapter, "test content with code")
        assert all(v > 0 for v in result.scores.values())


# ════════════════════════════════════════════════════════════════════════════
# fast_temporal_judgment (3-perspective MICRO path)
# ════════════════════════════════════════════════════════════════════════════

class TestFastTemporalJudgment:
    @pytest.mark.asyncio
    async def test_makes_3_calls(self):
        """Fast path: exactly 3 calls (PRESENT + FUTURE + NEVER)."""
        adapter, calls = _mock_adapter_counting(40.0)
        await fast_temporal_judgment(adapter, "test")
        assert len(calls) == 3

    @pytest.mark.asyncio
    async def test_result_valid(self):
        adapter = _mock_adapter(45.0)
        result = await fast_temporal_judgment(adapter, "test")
        assert isinstance(result, TemporalJudgment)
        assert 0 < result.phi_aggregate <= MAX_Q_SCORE
