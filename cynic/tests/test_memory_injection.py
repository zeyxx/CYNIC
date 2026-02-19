"""
γ5: Memory Injection Tests — Compressed CYNIC context flows into LLM calls.

Validates that temporal_judgment, SAGE, and the orchestrator all properly
thread the compressed_context through to each LLM system prompt.

Architecture tested:
  orchestrator._cycle_macro()
    → organism_kwargs["compressed_context"] = compressor.get_compressed_context(200)
      → sage.analyze(**organism_kwargs)
        → _temporal_path(..., compressed_context=...)
          → temporal_judgment(adapter, content, context=compressed_context)
            → _judge_perspective(..., context=...) → system += CYNIC Memory Context
"""
import asyncio
import math
from typing import Any
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from cynic.core.judgment import Cell
from cynic.core.phi import MAX_Q_SCORE, PHI_INV
from cynic.llm.temporal import (
    TemporalJudgment,
    _judge_perspective,
    fast_temporal_judgment,
    temporal_judgment,
)


# ──────────────────────────────────────────────────────────────────────────────
# Helpers
# ──────────────────────────────────────────────────────────────────────────────

def _mock_adapter(score_text: str = "SCORE: 75") -> Any:
    adapter = MagicMock()
    resp = MagicMock()
    resp.content = score_text
    adapter.complete_safe = AsyncMock(return_value=resp)
    adapter.adapter_id = "mock-adapter"
    return adapter


def _make_cell(**kwargs) -> Cell:
    defaults = dict(
        reality="CODE", analysis="JUDGE", time_dim="PRESENT",
        content="def foo(): pass",
        context="",
        risk=0.1, complexity=0.2, budget_usd=0.01,
    )
    defaults.update(kwargs)
    return Cell(**defaults)


def run(coro):
    return asyncio.get_event_loop().run_until_complete(coro)


# ──────────────────────────────────────────────────────────────────────────────
# 1. _judge_perspective: context appended to system prompt
# ──────────────────────────────────────────────────────────────────────────────

class TestJudgePerspectiveContext:
    def test_no_context_system_unchanged(self):
        """Without context, system prompt is TEMPORAL_SYSTEM_BASE + perspective only."""
        adapter = _mock_adapter()
        run(_judge_perspective(adapter, "code here", "PRESENT"))
        captured = adapter.complete_safe.call_args[0][0]
        assert "CYNIC Memory Context" not in captured.system

    def test_context_appended_to_system(self):
        """With context, 'CYNIC Memory Context:' is appended to system prompt."""
        adapter = _mock_adapter()
        run(_judge_perspective(adapter, "code here", "PRESENT", context="Q=72 WAG last session"))
        captured = adapter.complete_safe.call_args[0][0]
        assert "CYNIC Memory Context" in captured.system
        assert "Q=72 WAG last session" in captured.system

    def test_context_capped_at_500_chars(self):
        """Long context is capped at 500 chars to prevent token bloat."""
        adapter = _mock_adapter()
        long_ctx = "x" * 1000
        run(_judge_perspective(adapter, "code", "IDEAL", context=long_ctx))
        captured = adapter.complete_safe.call_args[0][0]
        # The portion in system should be ≤500 chars of context
        marker = "CYNIC Memory Context:\n"
        idx = captured.system.find(marker)
        assert idx != -1
        injected = captured.system[idx + len(marker):]
        assert len(injected) <= 500

    def test_empty_context_not_injected(self):
        """Empty string context does not add the memory section."""
        adapter = _mock_adapter()
        run(_judge_perspective(adapter, "code", "FUTURE", context=""))
        captured = adapter.complete_safe.call_args[0][0]
        assert "CYNIC Memory Context" not in captured.system


# ──────────────────────────────────────────────────────────────────────────────
# 2. temporal_judgment: context threaded to all perspectives
# ──────────────────────────────────────────────────────────────────────────────

class TestTemporalJudgmentContext:
    def test_context_reaches_all_7_perspectives(self):
        """Context injected into temporal_judgment reaches every perspective call."""
        call_systems = []

        async def mock_complete_safe(req):
            call_systems.append(req.system)
            r = MagicMock()
            r.content = "SCORE: 60"
            return r

        adapter = MagicMock()
        adapter.complete_safe = mock_complete_safe
        adapter.adapter_id = "mock"

        tj = run(temporal_judgment(adapter, "content", context="past=BARK residual=HIGH"))

        assert len(call_systems) == 7
        for sys in call_systems:
            assert "past=BARK residual=HIGH" in sys

    def test_no_context_no_memory_section(self):
        """Without context, no perspective gets memory injection."""
        call_systems = []

        async def mock_complete_safe(req):
            call_systems.append(req.system)
            r = MagicMock()
            r.content = "SCORE: 55"
            return r

        adapter = MagicMock()
        adapter.complete_safe = mock_complete_safe
        adapter.adapter_id = "mock"

        run(temporal_judgment(adapter, "content"))

        for sys in call_systems:
            assert "CYNIC Memory Context" not in sys

    def test_returns_temporal_judgment(self):
        """temporal_judgment with context still returns TemporalJudgment."""
        adapter = _mock_adapter("SCORE: 80")
        result = run(temporal_judgment(adapter, "test", context="some ctx"))
        assert isinstance(result, TemporalJudgment)
        assert result.phi_aggregate > 0


# ──────────────────────────────────────────────────────────────────────────────
# 3. fast_temporal_judgment: context threaded to 3 perspectives
# ──────────────────────────────────────────────────────────────────────────────

class TestFastTemporalContext:
    def test_context_reaches_3_perspectives(self):
        """Context injected into fast_temporal_judgment reaches all 3 perspectives."""
        call_systems = []

        async def mock_complete_safe(req):
            call_systems.append(req.system)
            r = MagicMock()
            r.content = "SCORE: 65"
            return r

        adapter = MagicMock()
        adapter.complete_safe = mock_complete_safe
        adapter.adapter_id = "mock"

        run(fast_temporal_judgment(adapter, "content", context="QTable=WAG 3x"))

        assert len(call_systems) == 3
        for sys in call_systems:
            assert "QTable=WAG 3x" in sys


# ──────────────────────────────────────────────────────────────────────────────
# 4. SageDog: extracts compressed_context from kwargs
# ──────────────────────────────────────────────────────────────────────────────

class TestSageCompressedContext:
    def _make_sage_with_mock_adapter(self):
        from cynic.dogs.sage import SageDog
        sage = SageDog()
        mock_adapter = MagicMock()
        mock_adapter.adapter_id = "mock-haiku"

        async def mock_get_llm():
            return mock_adapter

        sage.get_llm = mock_get_llm
        return sage, mock_adapter

    def test_sage_passes_context_to_temporal(self):
        """SAGE extracts compressed_context kwarg and passes it to temporal_judgment."""
        sage, mock_adapter = self._make_sage_with_mock_adapter()
        cell = _make_cell()

        received_context = []

        async def fake_temporal(adapter, content, perspectives=None, context=""):
            received_context.append(context)
            return TemporalJudgment(
                past=70.0, present=70.0, future=70.0, ideal=70.0,
                never=70.0, cycles=70.0, flow=70.0, llm_id="mock",
            )

        # sage._temporal_path does a local import, so patch the source module
        with patch("cynic.llm.temporal.temporal_judgment", side_effect=fake_temporal):
            run(sage.analyze(cell, compressed_context="test CYNIC context"))

        assert len(received_context) == 1
        assert received_context[0] == "test CYNIC context"

    def test_sage_empty_context_default(self):
        """When compressed_context not in kwargs, SAGE passes empty string."""
        sage, mock_adapter = self._make_sage_with_mock_adapter()
        cell = _make_cell()

        received_context = []

        async def fake_temporal(adapter, content, perspectives=None, context=""):
            received_context.append(context)
            return TemporalJudgment(
                past=50.0, present=50.0, future=50.0, ideal=50.0,
                never=50.0, cycles=50.0, flow=50.0, llm_id="mock",
            )

        with patch("cynic.llm.temporal.temporal_judgment", side_effect=fake_temporal):
            run(sage.analyze(cell))  # No compressed_context kwarg

        assert received_context[0] == ""

    def test_sage_fast_path_passes_context(self):
        """SAGE at lod_level=1 uses fast_temporal and still passes context."""
        sage, mock_adapter = self._make_sage_with_mock_adapter()
        cell = _make_cell()

        received_context = []

        async def fake_fast_temporal(adapter, content, context=""):
            received_context.append(context)
            return TemporalJudgment(
                past=60.0, present=60.0, future=60.0, ideal=60.0,
                never=60.0, cycles=60.0, flow=60.0, llm_id="mock",
            )

        with patch("cynic.llm.temporal.fast_temporal_judgment", side_effect=fake_fast_temporal):
            run(sage.analyze(cell, lod_level=1, compressed_context="fast path ctx"))

        assert received_context[0] == "fast path ctx"


# ──────────────────────────────────────────────────────────────────────────────
# 5. Orchestrator: builds compressed_context in organism_kwargs
# ──────────────────────────────────────────────────────────────────────────────

def _make_orchestrator():
    from cynic.core.axioms import AxiomArchitecture
    from cynic.core.heuristic_scorer import HeuristicFacetScorer
    from cynic.dogs.cynic_dog import CynicDog
    from cynic.judge.orchestrator import JudgeOrchestrator
    cynic_dog = CynicDog()
    arch = AxiomArchitecture(facet_scorer=HeuristicFacetScorer())
    return JudgeOrchestrator(dogs={}, axiom_arch=arch, cynic_dog=cynic_dog)


def _make_mock_pipeline(cell):
    pipeline = MagicMock()
    pipeline.cell = cell
    pipeline.dog_judgments = []
    pipeline.elapsed_ms.return_value = 5.0
    return pipeline


def _make_mock_consensus():
    from cynic.core.judgment import ConsensusResult
    return ConsensusResult(
        consensus=True, votes=5, quorum=3,
        final_q_score=70.0, final_confidence=0.5,
    )


def _setup_orch_mocks(orch):
    orch.cynic_dog = MagicMock()
    orch.cynic_dog.pbft_run = AsyncMock(return_value=_make_mock_consensus())
    mock_axiom_result = MagicMock()
    mock_axiom_result.q_score = 70.0
    mock_axiom_result.axiom_scores = {}
    mock_axiom_result.active_axioms = set()
    orch.axiom_arch = MagicMock()
    orch.axiom_arch.score_and_compute.return_value = mock_axiom_result


class TestOrchestratorMemoryInjection:

    def test_compressor_attribute_defaults_to_none(self):
        """context_compressor is None by default — no injection without wiring."""
        orch = _make_orchestrator()
        assert orch.context_compressor is None

    def test_compressor_injection_sets_attribute(self):
        """orchestrator.context_compressor = compressor sets the attribute."""
        orch = _make_orchestrator()
        compressor = MagicMock()
        orch.context_compressor = compressor
        assert orch.context_compressor is compressor

    def test_no_compressor_no_compressed_context_kwarg(self):
        """Without compressor, organism_kwargs has no compressed_context key."""
        captured_kwargs: dict = {}

        async def capture_analyze(cell, **kwargs):
            captured_kwargs.update(kwargs)
            return MagicMock(q_score=70.0, veto=False, cost_usd=0.0, llm_id=None)

        mock_dog = MagicMock()
        mock_dog.analyze = capture_analyze

        orch = _make_orchestrator()
        orch.dogs = {"mock": mock_dog}
        _setup_orch_mocks(orch)
        # context_compressor is None by default

        run(orch._cycle_macro(_make_mock_pipeline(_make_cell())))

        assert "compressed_context" not in captured_kwargs

    def test_with_compressor_context_injected(self):
        """With compressor returning content, organism_kwargs gets compressed_context."""
        captured_kwargs: dict = {}

        async def capture_analyze(cell, **kwargs):
            captured_kwargs.update(kwargs)
            return MagicMock(q_score=70.0, veto=False, cost_usd=0.0, llm_id=None)

        mock_dog = MagicMock()
        mock_dog.analyze = capture_analyze

        orch = _make_orchestrator()
        orch.dogs = {"mock": mock_dog}
        _setup_orch_mocks(orch)

        compressor = MagicMock()
        compressor.get_compressed_context.return_value = "BARK 3x last hour, QTable=GROWL"
        orch.context_compressor = compressor

        run(orch._cycle_macro(_make_mock_pipeline(_make_cell())))

        assert "compressed_context" in captured_kwargs
        assert captured_kwargs["compressed_context"] == "BARK 3x last hour, QTable=GROWL"
        compressor.get_compressed_context.assert_called_once_with(budget=200)

    def test_empty_context_not_injected(self):
        """Compressor returning empty string → compressed_context not in kwargs."""
        captured_kwargs: dict = {}

        async def capture_analyze(cell, **kwargs):
            captured_kwargs.update(kwargs)
            return MagicMock(q_score=70.0, veto=False, cost_usd=0.0, llm_id=None)

        mock_dog = MagicMock()
        mock_dog.analyze = capture_analyze

        orch = _make_orchestrator()
        orch.dogs = {"mock": mock_dog}
        _setup_orch_mocks(orch)

        compressor = MagicMock()
        compressor.get_compressed_context.return_value = ""
        orch.context_compressor = compressor

        run(orch._cycle_macro(_make_mock_pipeline(_make_cell())))

        assert "compressed_context" not in captured_kwargs
