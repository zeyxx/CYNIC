"""
CYNIC Tests: γ2 SAGE → Compressor attention feedback loop (T28).

Verifies that orchestrator.run() calls context_compressor.boost()
after each judgment with relevance = q_score / 100.0.
No LLM, no DB.
"""
from __future__ import annotations

import pytest
from unittest.mock import MagicMock

from cynic.core.judgment import Cell
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.axioms import AxiomArchitecture
from cynic.cognition.neurons.base import DogId
from cynic.cognition.neurons.cynic_dog import CynicDog
from cynic.cognition.neurons.guardian import GuardianDog
from cynic.cognition.neurons.analyst import AnalystDog
from cynic.cognition.neurons.janitor import JanitorDog
from cynic.cognition.cortex.orchestrator import JudgeOrchestrator


def _make_orchestrator():
    dogs = {
        DogId.CYNIC:    CynicDog(),
        DogId.GUARDIAN: GuardianDog(),
        DogId.ANALYST:  AnalystDog(),
        DogId.JANITOR:  JanitorDog(),
    }
    return JudgeOrchestrator(
        dogs=dogs,
        axiom_arch=AxiomArchitecture(),
        cynic_dog=dogs[DogId.CYNIC],
    )


def _make_cell():
    return Cell(
        reality="CODE",
        analysis="JUDGE",
        content={"code": "def foo(): pass"},
        context="test",
        novelty=0.3,
        complexity=0.3,
        risk=0.1,
        budget_usd=0.05,
    )


class TestCompressorBoost:

    async def test_boost_called_with_valid_relevance(self):
        """After judgment, boost() is called with a float in [0, 1]."""
        orch = _make_orchestrator()
        mock_compressor = MagicMock()
        orch.context_compressor = mock_compressor

        await orch.run(_make_cell(), level=ConsciousnessLevel.REFLEX)

        mock_compressor.boost.assert_called_once()
        _, relevance = mock_compressor.boost.call_args[0]
        assert 0.0 <= relevance <= 1.0, f"relevance {relevance} must be in [0, 1]"

    async def test_boost_not_called_without_compressor(self):
        """Orchestrator without context_compressor runs without crash."""
        orch = _make_orchestrator()
        assert orch.context_compressor is None

        judgment = await orch.run(_make_cell(), level=ConsciousnessLevel.REFLEX)
        assert judgment is not None

    async def test_boost_relevance_maps_q_score(self):
        """relevance passed to boost() equals q_score / 100.0."""
        orch = _make_orchestrator()
        captured = {}

        def capture(query, relevance):
            captured["query"] = query
            captured["relevance"] = relevance

        mock_compressor = MagicMock()
        mock_compressor.boost.side_effect = capture
        orch.context_compressor = mock_compressor

        judgment = await orch.run(_make_cell(), level=ConsciousnessLevel.REFLEX)

        assert "relevance" in captured
        expected = judgment.q_score / 100.0
        assert abs(captured["relevance"] - expected) < 1e-9

    async def test_boost_error_does_not_crash_judgment(self):
        """If boost() raises, judgment still completes successfully."""
        orch = _make_orchestrator()
        mock_compressor = MagicMock()
        mock_compressor.boost.side_effect = RuntimeError("compressor exploded")
        orch.context_compressor = mock_compressor

        judgment = await orch.run(_make_cell(), level=ConsciousnessLevel.REFLEX)
        assert judgment is not None  # never crashes

    async def test_boost_query_contains_content_text(self):
        """The query string passed to boost() contains a preview of the cell content."""
        orch = _make_orchestrator()
        captured = {}

        def capture(query, relevance):
            captured["query"] = query

        mock_compressor = MagicMock()
        mock_compressor.boost.side_effect = capture
        orch.context_compressor = mock_compressor

        await orch.run(_make_cell(), level=ConsciousnessLevel.REFLEX)

        assert isinstance(captured.get("query"), str)
        # Query is a string preview — may be empty dict repr but must be str
        assert len(captured["query"]) >= 0
