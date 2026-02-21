"""
Tests for JudgeOrchestrator.evolve() and probe cells.

Unit tests -- all dogs are mocked; no Ollama, no DB, no network.
"""
from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock, patch
from typing import Dict, List

from cynic.core.judgment import Cell, Judgment
from cynic.core.phi import MAX_Q_SCORE, PHI_INV_2
from cynic.cognition.neurons.base import DogJudgment
from cynic.cognition.cortex.probes import PROBE_CELLS, ProbeResult


# ============================================================================
# Helpers
# ============================================================================

def _make_judgment(q_score: float = 42.0) -> Judgment:
    """Minimal Judgment for mocking orchestrator.run(). Verdict auto-derived."""
    from cynic.core.axioms import verdict_from_q_score
    cell = Cell(reality="CODE", analysis="JUDGE", content="x", time_dim="PRESENT")
    verdict = verdict_from_q_score(q_score).value
    return Judgment(
        cell=cell,
        q_score=q_score,
        verdict=verdict,
        confidence=PHI_INV_2,
        axiom_scores={},
        active_axioms=[],
        dog_votes={},
        consensus_votes=3,
        consensus_quorum=3,
        consensus_reached=True,
        cost_usd=0.0,
        duration_ms=10.0,
    )


def _make_orchestrator(q_score: float = 42.0):
    """
    JudgeOrchestrator with run() pre-patched to return a Judgment with q_score.

    evolve() delegates to self.run() for each probe, so patching run() is
    sufficient to control the entire evolve() behavior in unit tests.
    """
    from cynic.cognition.cortex.orchestrator import JudgeOrchestrator

    orch = JudgeOrchestrator(
        dogs=MagicMock(),
        axiom_arch=MagicMock(),
        cynic_dog=MagicMock(),
    )
    orch.run = AsyncMock(return_value=_make_judgment(q_score=q_score))
    return orch


# ============================================================================
# Probe Cell Definitions
# ============================================================================

class TestProbeCells:
    def test_five_probes_defined(self):
        assert len(PROBE_CELLS) == 5

    def test_probe_names_unique(self):
        names = [p["name"] for p in PROBE_CELLS]
        assert len(names) == len(set(names))

    def test_probe_cells_are_valid(self):
        for probe in PROBE_CELLS:
            cell = probe["cell"]
            assert isinstance(cell, Cell)
            assert cell.reality in {
                "CODE", "SOLANA", "CYNIC", "MARKET", "HUMAN", "SOCIAL", "COSMOS"
            }
            assert 0.0 <= probe["min_q"] <= probe["max_q"] <= MAX_Q_SCORE

    def test_p1_clean_code_range(self):
        p1 = next(p for p in PROBE_CELLS if p["name"] == "P1:clean_code")
        assert p1["min_q"] >= 20.0          # At least GROWL
        assert p1["max_q"] == MAX_Q_SCORE   # No upper cap

    def test_p2_smelly_range_capped(self):
        p2 = next(p for p in PROBE_CELLS if p["name"] == "P2:smelly_code")
        assert p2["max_q"] <= 70.0          # Should not reach HOWL zone (≥80)

    def test_p3_dangerous_range_capped(self):
        p3 = next(p for p in PROBE_CELLS if p["name"] == "P3:dangerous_act")
        assert p3["cell"].risk == 1.0       # risk=1.0 is the GUARDIAN trigger
        assert p3["max_q"] <= 50.0          # Dangerous op stays below WAG (61.8)

    def test_p4_cynic_self_state(self):
        p4 = next(p for p in PROBE_CELLS if p["name"] == "P4:cynic_self_state")
        assert p4["cell"].reality == "CYNIC"
        assert p4["cell"].analysis == "LEARN"

    def test_p5_solana_tx(self):
        p5 = next(p for p in PROBE_CELLS if p["name"] == "P5:solana_tx")
        assert p5["cell"].reality == "SOLANA"
        assert p5["cell"].analysis == "JUDGE"

    def test_all_probes_have_budget(self):
        for probe in PROBE_CELLS:
            assert probe["cell"].budget_usd > 0

    def test_probe_metadata_probe_flag(self):
        for probe in PROBE_CELLS:
            assert probe["cell"].metadata.get("probe") is True


# ============================================================================
# ProbeResult
# ============================================================================

class TestProbeResult:
    def test_passed_when_in_range(self):
        r = ProbeResult(
            name="P1:clean_code", q_score=42.0, verdict="WAG",
            expected_min=25.0, expected_max=MAX_Q_SCORE,
            passed=True, duration_ms=15.0,
        )
        assert r.passed is True

    def test_to_dict_keys(self):
        r = ProbeResult(
            name="P1:clean_code", q_score=42.0, verdict="WAG",
            expected_min=25.0, expected_max=MAX_Q_SCORE,
            passed=True, duration_ms=15.0,
        )
        d = r.to_dict()
        assert set(d.keys()) == {
            "name", "q_score", "verdict", "expected_min", "expected_max",
            "passed", "duration_ms", "error",
        }

    def test_error_field_default_empty(self):
        r = ProbeResult(
            name="P3:dangerous_act", q_score=0.0, verdict="BARK",
            expected_min=0.0, expected_max=40.0,
            passed=True, duration_ms=0.0,
        )
        assert r.error == ""

    def test_q_score_rounded_in_dict(self):
        r = ProbeResult(
            name="P1:clean_code", q_score=42.123456, verdict="WAG",
            expected_min=25.0, expected_max=MAX_Q_SCORE,
            passed=True, duration_ms=15.123456,
        )
        d = r.to_dict()
        assert d["q_score"] == 42.123
        assert d["duration_ms"] == 15.1


# ============================================================================
# JudgeOrchestrator.evolve()
# ============================================================================

class TestEvolve:
    async def test_evolve_runs_all_5_probes(self):
        """evolve() must return results for all 5 probe cells."""
        orch = _make_orchestrator(q_score=42.0)
        summary = await orch.evolve()

        assert summary["total"] == 5
        assert len(summary["results"]) == 5
        # run() called once per probe
        assert orch.run.call_count == 5

    async def test_evolve_pass_rate_in_range(self):
        """pass_rate is in [0.0, 1.0]."""
        orch = _make_orchestrator(q_score=42.0)
        summary = await orch.evolve()
        assert 0.0 <= summary["pass_rate"] <= 1.0

    async def test_evolve_no_regression_on_first_call(self):
        """First evolve() has no history — regression must be False."""
        orch = _make_orchestrator(q_score=42.0)
        summary = await orch.evolve()
        assert summary["regression"] is False

    async def test_evolve_records_history(self):
        """Each evolve() call appends to _evolve_history."""
        orch = _make_orchestrator(q_score=42.0)
        await orch.evolve()
        await orch.evolve()
        assert len(orch._evolve_history) == 2

    async def test_evolve_detects_regression(self):
        """If pass_rate drops >20% vs previous call, regression=True."""
        orch = _make_orchestrator()

        # First call: q_score=42.0 — P1 passes (min=25 ≤ 42 ≤ MAX_Q_SCORE)
        # P2 passes (0 ≤ 42 ≤ 50), P3 passes (0 ≤ 42 ≤ 40 → FAILS! 42 > 40)
        # P4 passes (20 ≤ 42 ≤ MAX), P5 passes (20 ≤ 42 ≤ MAX)
        # 4/5 = 80% pass rate
        orch.run = AsyncMock(return_value=_make_judgment(q_score=42.0))
        summary1 = await orch.evolve()
        first_rate = summary1["pass_rate"]

        # Second call: q_score=0.0 — fails all probes with min_q > 0
        # P1 fails (25 > 0), P2 passes (0 ≤ 0 ≤ 50), P3 passes (0 ≤ 0 ≤ 40)
        # P4 fails (20 > 0), P5 fails (20 > 0) → 2/5 = 40%
        # Drop = 80% → 40% = 40% drop > 20% threshold → regression
        orch.run = AsyncMock(return_value=_make_judgment(q_score=0.0))
        summary2 = await orch.evolve()

        assert summary2["regression"] is True

    async def test_evolve_no_regression_if_stable(self):
        """Stable pass_rate → regression=False."""
        orch = _make_orchestrator(q_score=42.0)
        await orch.evolve()
        summary2 = await orch.evolve()
        assert summary2["regression"] is False

    async def test_evolve_history_capped_at_21(self):
        """_evolve_history never exceeds F(8)=21 entries."""
        orch = _make_orchestrator(q_score=42.0)
        for _ in range(30):
            await orch.evolve()
        assert len(orch._evolve_history) <= 21

    async def test_evolve_emits_meta_cycle_event(self):
        """evolve() emits CoreEvent.META_CYCLE with evolve payload."""
        import asyncio
        from cynic.core.event_bus import get_core_bus, CoreEvent

        received: list = []

        async def _listener(e):
            received.append(e)

        bus = get_core_bus()
        bus.on(CoreEvent.META_CYCLE, _listener)

        orch = _make_orchestrator(q_score=42.0)
        await orch.evolve()
        # Handlers fire as create_task -- yield event loop so they execute
        await asyncio.sleep(0)

        bus.off(CoreEvent.META_CYCLE, _listener)
        assert len(received) >= 1
        assert "evolve" in received[0].payload

    async def test_evolve_handles_probe_exception_gracefully(self):
        """A probe exception marks that result as failed; others continue."""
        orch = _make_orchestrator()

        call_count = 0

        async def _run_fails_first(*args, **kwargs):
            nonlocal call_count
            call_count += 1
            if call_count == 1:
                raise RuntimeError("dog exploded")
            return _make_judgment(q_score=42.0)

        orch.run = _run_fails_first

        summary = await orch.evolve()

        assert summary["total"] == 5
        # First result has error set
        assert summary["results"][0]["error"] != ""
        assert summary["results"][0]["q_score"] == 0.0
        # Other 4 have no error
        no_error_count = sum(1 for r in summary["results"] if r["error"] == "")
        assert no_error_count == 4

    async def test_evolve_summary_keys(self):
        """Summary dict contains all expected keys."""
        orch = _make_orchestrator(q_score=42.0)
        summary = await orch.evolve()
        assert set(summary.keys()) == {
            "timestamp", "pass_rate", "pass_count", "total", "regression", "results"
        }

    async def test_stats_includes_evolve_info(self):
        """stats() exposes last evolve pass_rate and regression flag."""
        orch = _make_orchestrator(q_score=42.0)
        await orch.evolve()

        s = orch.stats()
        assert "evolve_cycles" in s
        assert s["evolve_cycles"] == 1
        assert s["last_evolve_pass_rate"] is not None
        assert s["last_evolve_regression"] is False

    async def test_stats_before_evolve(self):
        """stats() returns safe defaults before evolve() is called."""
        orch = _make_orchestrator()
        s = orch.stats()
        assert s["evolve_cycles"] == 0
        assert s["last_evolve_pass_rate"] is None
        assert s["last_evolve_regression"] is False
