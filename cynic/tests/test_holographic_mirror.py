"""
Tests: R2 — Holographic Mirror
Dogs receive organism health context (lod_level, active_axioms) and adapt judgment.

Coverage:
  SAGE:       lod_level=1 → fast_temporal (3p); lod_level=0 → full temporal (7p)
  GUARDIAN:   lod_level=1 → lower veto threshold (WARN vs VETO)
  CynicDog:   active_axioms kwarg → 4th coherence factor included/excluded
  Orchestrator: _cycle_macro builds organism_kwargs with lod_level + active_axioms
"""
from __future__ import annotations

import asyncio
import pytest
from unittest.mock import AsyncMock, MagicMock, patch

from cynic.core.judgment import Cell, ConsensusResult
from cynic.core.phi import PHI_2, PHI_INV, MAX_Q_SCORE
from cynic.dogs.guardian import GuardianDog, VETO_THRESHOLD, WARN_THRESHOLD
from cynic.dogs.cynic_dog import CynicDog
from cynic.judge.orchestrator import JudgeOrchestrator


# ── Helpers ──────────────────────────────────────────────────────────────────

def _make_cell(risk: float = 0.5, reality: str = "CODE") -> Cell:
    return Cell(
        reality=reality,
        analysis="JUDGE",
        content="def foo(): pass",
        context="unit test cell",
        novelty=0.5,
        complexity=0.5,
        risk=risk,
    )


def _make_orchestrator() -> JudgeOrchestrator:
    return JudgeOrchestrator(
        dogs={},
        axiom_arch=MagicMock(),
        cynic_dog=MagicMock(),
    )


def _run(coro):
    """Run a coroutine using the event loop (matches existing test pattern)."""
    return asyncio.get_event_loop().run_until_complete(coro)


# ═══════════════════════════════════════════════════════════════════════════
# GUARDIAN — veto threshold adapts to LOD
# ═══════════════════════════════════════════════════════════════════════════

class TestGuardianLodAware:

    def test_full_lod_does_not_veto_at_warn_level(self):
        """LOD=FULL (0): danger at WARN level (1.5) → no veto (needs VETO=2.618)."""
        dog = GuardianDog()
        cell = _make_cell(risk=0.1)
        with patch.object(dog, "_score_anomaly", return_value=(0.9, True, 1.5)):
            result = _run(dog.analyze(cell, lod_level=0))
        # 1.5 < VETO_THRESHOLD (2.618) → no veto at FULL LOD
        assert result.veto is False

    def test_reduced_lod_vetos_at_warn_threshold(self):
        """LOD=REDUCED (1): danger ≥ WARN_THRESHOLD (1.0) → VETO (organism stressed)."""
        dog = GuardianDog()
        cell = _make_cell(risk=0.5)
        with patch.object(dog, "_score_anomaly", return_value=(0.9, True, 1.5)):
            result = _run(dog.analyze(cell, lod_level=1))
        # 1.5 ≥ WARN_THRESHOLD (1.0) → veto in REDUCED mode
        assert result.veto is True

    def test_no_lod_kwarg_uses_veto_threshold(self):
        """No lod_level kwarg → defaults to LOD=0 → uses normal VETO_THRESHOLD."""
        dog = GuardianDog()
        cell = _make_cell(risk=0.1)
        with patch.object(dog, "_score_anomaly", return_value=(0.9, True, 1.5)):
            result = _run(dog.analyze(cell))
        # 1.5 < VETO_THRESHOLD (2.618) → no veto (normal mode, lod_level defaults to 0)
        assert result.veto is False

    def test_extreme_danger_vetos_at_full_lod(self):
        """Danger ≥ VETO_THRESHOLD (2.618) → FULL LOD still vetos."""
        dog = GuardianDog()
        cell = _make_cell(risk=0.9)
        with patch.object(dog, "_score_anomaly", return_value=(1.0, True, 3.0)):
            result = _run(dog.analyze(cell, lod_level=0))
        assert result.veto is True

    def test_extreme_danger_vetos_at_reduced_lod(self):
        """Danger ≥ VETO_THRESHOLD (2.618) → REDUCED LOD also vetos (WARN < VETO < danger)."""
        dog = GuardianDog()
        cell = _make_cell(risk=0.9)
        with patch.object(dog, "_score_anomaly", return_value=(1.0, True, 3.0)):
            result = _run(dog.analyze(cell, lod_level=1))
        # 3.0 ≥ WARN_THRESHOLD (1.0) at REDUCED → veto
        assert result.veto is True

    def test_threshold_constants_order(self):
        """WARN < VETO — lower threshold is more permissive under stress."""
        assert WARN_THRESHOLD < VETO_THRESHOLD
        assert WARN_THRESHOLD == pytest.approx(1.0)
        assert VETO_THRESHOLD == pytest.approx(PHI_2)


# ═══════════════════════════════════════════════════════════════════════════
# CynicDog — active_axioms as 4th coherence factor
# ═══════════════════════════════════════════════════════════════════════════

class TestCynicDogAxiomFactor:

    def test_no_active_axioms_kwarg_no_axiom_in_reasoning(self):
        """Without active_axioms kwarg → 3 factors only, no axiom mention."""
        dog = CynicDog()
        cell = _make_cell()
        result = _run(dog.analyze(cell, budget_usd=0.01))
        assert "axioms" not in result.reasoning

    def test_active_axioms_4_yields_higher_score_than_0(self):
        """4 active axioms (ratio=1.0) beats 0 axioms (ratio=0.01) in geometric mean."""
        result_0 = _run(CynicDog().analyze(_make_cell(), budget_usd=0.01, active_axioms=0))
        result_4 = _run(CynicDog().analyze(_make_cell(), budget_usd=0.01, active_axioms=4))
        assert result_4.q_score >= result_0.q_score

    def test_active_axioms_appears_in_reasoning(self):
        """active_axioms=2 → reasoning mentions 'axioms=2/4'."""
        dog = CynicDog()
        cell = _make_cell()
        result = _run(dog.analyze(cell, budget_usd=0.01, active_axioms=2))
        assert "axioms=2/4" in result.reasoning

    def test_active_axioms_0_does_not_zero_score(self):
        """active_axioms=0 uses ratio=0.01 (not 0) — q_score stays positive."""
        dog = CynicDog()
        cell = _make_cell()
        result = _run(dog.analyze(cell, budget_usd=0.01, active_axioms=0))
        assert result.q_score > 0.0

    def test_active_axioms_1_in_reasoning(self):
        """active_axioms=1 → 'axioms=1/4' in reasoning."""
        dog = CynicDog()
        result = _run(dog.analyze(_make_cell(), budget_usd=0.01, active_axioms=1))
        assert "axioms=1/4" in result.reasoning


# ═══════════════════════════════════════════════════════════════════════════
# SAGE — temporal path selection based on lod_level
# ═══════════════════════════════════════════════════════════════════════════

class TestSageLodAwarePath:

    def _make_mock_tj(self, q: float = 70.0) -> MagicMock:
        """Build a minimal TemporalJudgment mock."""
        from cynic.llm.temporal import TemporalJudgment
        tj = MagicMock(spec=TemporalJudgment)
        tj.phi_aggregate = q
        tj.confidence = 0.5
        tj.scores = {"PRESENT": q, "FUTURE": q - 2.0}
        tj.llm_id = "mock-llm"
        tj.to_dict.return_value = {}
        return tj

    def test_full_lod_calls_temporal_judgment(self):
        """LOD=FULL (0) → calls temporal_judgment (7p), NOT fast_temporal_judgment."""
        from cynic.dogs.sage import SageDog
        dog = SageDog()
        cell = _make_cell()
        mock_tj = self._make_mock_tj()

        mock_full = AsyncMock(return_value=mock_tj)
        mock_fast = AsyncMock(return_value=mock_tj)

        with patch("cynic.llm.temporal.temporal_judgment", mock_full), \
             patch("cynic.llm.temporal.fast_temporal_judgment", mock_fast), \
             patch.object(dog, "get_llm", AsyncMock(return_value=MagicMock())):
            _run(dog.analyze(cell, lod_level=0))

        mock_full.assert_called_once()
        mock_fast.assert_not_called()

    def test_reduced_lod_calls_fast_temporal_judgment(self):
        """LOD=REDUCED (1) → calls fast_temporal_judgment (3p), NOT temporal_judgment."""
        from cynic.dogs.sage import SageDog
        dog = SageDog()
        cell = _make_cell()
        mock_tj = self._make_mock_tj(65.0)

        mock_full = AsyncMock(return_value=mock_tj)
        mock_fast = AsyncMock(return_value=mock_tj)

        with patch("cynic.llm.temporal.temporal_judgment", mock_full), \
             patch("cynic.llm.temporal.fast_temporal_judgment", mock_fast), \
             patch.object(dog, "get_llm", AsyncMock(return_value=MagicMock())):
            _run(dog.analyze(cell, lod_level=1))

        mock_fast.assert_called_once()
        mock_full.assert_not_called()

    def test_no_lod_kwarg_defaults_to_full_temporal(self):
        """No lod_level kwarg → defaults to 0 → uses full temporal_judgment."""
        from cynic.dogs.sage import SageDog
        dog = SageDog()
        cell = _make_cell()
        mock_tj = self._make_mock_tj()

        mock_full = AsyncMock(return_value=mock_tj)
        mock_fast = AsyncMock(return_value=mock_tj)

        with patch("cynic.llm.temporal.temporal_judgment", mock_full), \
             patch("cynic.llm.temporal.fast_temporal_judgment", mock_fast), \
             patch.object(dog, "get_llm", AsyncMock(return_value=MagicMock())):
            _run(dog.analyze(cell))  # No lod_level

        mock_full.assert_called_once()
        mock_fast.assert_not_called()

    def test_reduced_lod_reasoning_mentions_3p(self):
        """LOD=REDUCED → reasoning contains '3p' (3 perspectives)."""
        from cynic.dogs.sage import SageDog
        dog = SageDog()
        cell = _make_cell()
        mock_tj = self._make_mock_tj(65.0)

        with patch("cynic.llm.temporal.fast_temporal_judgment", AsyncMock(return_value=mock_tj)), \
             patch.object(dog, "get_llm", AsyncMock(return_value=MagicMock())):
            result = _run(dog.analyze(cell, lod_level=1))

        assert "3p" in result.reasoning

    def test_full_lod_reasoning_mentions_7p(self):
        """LOD=FULL → reasoning contains '7p' (7 perspectives)."""
        from cynic.dogs.sage import SageDog
        dog = SageDog()
        cell = _make_cell()
        mock_tj = self._make_mock_tj()

        with patch("cynic.llm.temporal.temporal_judgment", AsyncMock(return_value=mock_tj)), \
             patch.object(dog, "get_llm", AsyncMock(return_value=MagicMock())):
            result = _run(dog.analyze(cell, lod_level=0))

        assert "7p" in result.reasoning


# ═══════════════════════════════════════════════════════════════════════════
# Orchestrator — organism_kwargs built and passed to Dogs in _cycle_macro
# ═══════════════════════════════════════════════════════════════════════════

class TestOrchestratorOrganismKwargs:

    def _make_mock_pipeline(self, cell: Cell) -> MagicMock:
        pipeline = MagicMock()
        pipeline.cell = cell
        pipeline.dog_judgments = []
        pipeline.elapsed_ms.return_value = 5.0
        return pipeline

    def _make_mock_consensus(self) -> ConsensusResult:
        return ConsensusResult(
            consensus=True,
            votes=5,
            quorum=3,
            final_q_score=70.0,
            final_confidence=0.5,
        )

    def _setup_orch_mocks(self, orch: JudgeOrchestrator) -> None:
        orch.cynic_dog = MagicMock()
        orch.cynic_dog.pbft_run = AsyncMock(return_value=self._make_mock_consensus())
        mock_axiom_result = MagicMock()
        mock_axiom_result.q_score = 70.0
        mock_axiom_result.axiom_scores = {}
        mock_axiom_result.active_axioms = set()
        orch.axiom_arch = MagicMock()
        orch.axiom_arch.score_and_compute.return_value = mock_axiom_result

    def test_lod_controller_passes_lod_level_to_dogs(self):
        """When lod_controller is set, Dogs receive lod_level in kwargs."""
        from cynic.judge.lod import LODController, SurvivalLOD
        captured_kwargs: dict = {}

        async def capture_analyze(cell, **kwargs):
            captured_kwargs.update(kwargs)
            return MagicMock(q_score=70.0, veto=False, cost_usd=0.0, llm_id=None)

        mock_dog = MagicMock()
        mock_dog.analyze = capture_analyze

        orch = _make_orchestrator()
        orch.dogs = {"mock": mock_dog}
        self._setup_orch_mocks(orch)

        lod_ctrl = LODController()
        lod_ctrl.force(SurvivalLOD.REDUCED)
        orch.lod_controller = lod_ctrl

        cell = _make_cell()
        _run(orch._cycle_macro(self._make_mock_pipeline(cell)))

        assert "lod_level" in captured_kwargs
        assert captured_kwargs["lod_level"] == int(SurvivalLOD.REDUCED)

    def test_no_lod_controller_no_lod_kwarg(self):
        """No lod_controller → lod_level NOT included in organism_kwargs."""
        captured_kwargs: dict = {}

        async def capture_analyze(cell, **kwargs):
            captured_kwargs.update(kwargs)
            return MagicMock(q_score=70.0, veto=False, cost_usd=0.0, llm_id=None)

        mock_dog = MagicMock()
        mock_dog.analyze = capture_analyze

        orch = _make_orchestrator()
        orch.dogs = {"mock": mock_dog}
        self._setup_orch_mocks(orch)
        # lod_controller is None by default

        cell = _make_cell()
        _run(orch._cycle_macro(self._make_mock_pipeline(cell)))

        assert "lod_level" not in captured_kwargs

    def test_axiom_monitor_passes_active_axioms_to_dogs(self):
        """When axiom_monitor is set, Dogs receive active_axioms=3 in kwargs."""
        captured_kwargs: dict = {}

        async def capture_analyze(cell, **kwargs):
            captured_kwargs.update(kwargs)
            return MagicMock(q_score=70.0, veto=False, cost_usd=0.0, llm_id=None)

        mock_dog = MagicMock()
        mock_dog.analyze = capture_analyze

        orch = _make_orchestrator()
        orch.dogs = {"mock": mock_dog}
        self._setup_orch_mocks(orch)

        mock_axiom_mon = MagicMock()
        mock_axiom_mon.active_count.return_value = 3
        orch.axiom_monitor = mock_axiom_mon

        cell = _make_cell()
        _run(orch._cycle_macro(self._make_mock_pipeline(cell)))

        assert "active_axioms" in captured_kwargs
        assert captured_kwargs["active_axioms"] == 3

    def test_no_axiom_monitor_no_active_axioms_kwarg(self):
        """No axiom_monitor → active_axioms NOT included."""
        captured_kwargs: dict = {}

        async def capture_analyze(cell, **kwargs):
            captured_kwargs.update(kwargs)
            return MagicMock(q_score=70.0, veto=False, cost_usd=0.0, llm_id=None)

        mock_dog = MagicMock()
        mock_dog.analyze = capture_analyze

        orch = _make_orchestrator()
        orch.dogs = {"mock": mock_dog}
        self._setup_orch_mocks(orch)
        # axiom_monitor is None by default

        cell = _make_cell()
        _run(orch._cycle_macro(self._make_mock_pipeline(cell)))

        assert "active_axioms" not in captured_kwargs

    def test_active_dogs_always_included(self):
        """active_dogs kwarg always included (Dog count for CYNIC coherence)."""
        captured_kwargs: dict = {}

        async def capture_analyze(cell, **kwargs):
            captured_kwargs.update(kwargs)
            return MagicMock(q_score=70.0, veto=False, cost_usd=0.0, llm_id=None)

        mock_dog = MagicMock()
        mock_dog.analyze = capture_analyze

        orch = _make_orchestrator()
        orch.dogs = {"mock": mock_dog}
        self._setup_orch_mocks(orch)

        cell = _make_cell()
        _run(orch._cycle_macro(self._make_mock_pipeline(cell)))

        assert "active_dogs" in captured_kwargs
        assert captured_kwargs["active_dogs"] == 1  # 1 mock dog
