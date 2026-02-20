"""
Tests for BenchmarkRegistry — probe run persistence and drift detection.

Unit tests — all DB calls use AsyncMock. No real PostgreSQL needed.
"""
from __future__ import annotations

import pytest
from unittest.mock import AsyncMock, MagicMock

from cynic.core.phi import MAX_Q_SCORE
from cynic.cognition.cortex.probes import ProbeResult
from cynic.benchmark.registry import BenchmarkRegistry


# ── Helpers ──────────────────────────────────────────────────────────────────

def _make_result(
    name: str = "P1:clean_code",
    q_score: float = 38.0,
    verdict: str = "WAG",
    passed: bool = True,
    duration_ms: float = 15.0,
    error: str = "",
) -> ProbeResult:
    return ProbeResult(
        name=name,
        q_score=q_score,
        verdict=verdict,
        expected_min=25.0,
        expected_max=MAX_Q_SCORE,
        passed=passed,
        duration_ms=duration_ms,
        error=error,
    )


def _make_pool(fetch_return=None):
    """Mock asyncpg pool with acquire() context manager."""
    conn = MagicMock()
    conn.execute = AsyncMock()
    conn.executemany = AsyncMock()
    conn.fetch = AsyncMock(return_value=fetch_return or [])
    conn.fetchrow = AsyncMock(return_value=None)

    pool = MagicMock()
    pool.acquire.return_value.__aenter__ = AsyncMock(return_value=conn)
    pool.acquire.return_value.__aexit__ = AsyncMock(return_value=False)
    return pool, conn


# ── create_tables ─────────────────────────────────────────────────────────────

class TestCreateTables:
    async def test_calls_execute_with_ddl(self):
        pool, conn = _make_pool()
        await BenchmarkRegistry.create_tables(pool)
        conn.execute.assert_called_once()
        sql = conn.execute.call_args[0][0]
        assert "probe_runs" in sql
        assert "benchmark_snapshots" in sql
        assert "CREATE TABLE IF NOT EXISTS" in sql

    async def test_creates_both_indexes(self):
        pool, conn = _make_pool()
        await BenchmarkRegistry.create_tables(pool)
        sql = conn.execute.call_args[0][0]
        assert "idx_probe_runs_ts" in sql
        assert "idx_snapshots_probe" in sql

    async def test_idempotent_double_call(self):
        """create_tables can safely be called twice (IF NOT EXISTS)."""
        pool, conn = _make_pool()
        await BenchmarkRegistry.create_tables(pool)
        await BenchmarkRegistry.create_tables(pool)
        assert conn.execute.call_count == 2


# ── record_evolve ─────────────────────────────────────────────────────────────

class TestRecordEvolve:
    async def test_inserts_one_row_per_result(self):
        pool, conn = _make_pool()
        reg = BenchmarkRegistry(pool)
        results = [
            _make_result("P1:clean_code"),
            _make_result("P2:smelly_code", q_score=10.0, verdict="BARK", passed=False),
        ]
        await reg.record_evolve(results)
        conn.executemany.assert_called_once()
        rows = conn.executemany.call_args[0][1]
        assert len(rows) == 2

    async def test_probe_id_extracted_from_name(self):
        """'P1:clean_code' → probe_id='P1'."""
        pool, conn = _make_pool()
        reg = BenchmarkRegistry(pool)
        await reg.record_evolve([_make_result("P3:dangerous_act", q_score=0.0, verdict="BARK")])
        row = conn.executemany.call_args[0][1][0]
        assert row[0] == "P3"           # probe_id
        assert row[1] == "P3:dangerous_act"  # probe_name

    async def test_row_fields_match_probe_result(self):
        pool, conn = _make_pool()
        reg = BenchmarkRegistry(pool)
        r = _make_result("P1:clean_code", q_score=38.5, verdict="WAG",
                         passed=True, duration_ms=12.0, error="")
        await reg.record_evolve([r], source="test_source")

        probe_id, probe_name, q_score, verdict, passed, duration_ms, source, error = (
            conn.executemany.call_args[0][1][0]
        )
        assert probe_id == "P1"
        assert probe_name == "P1:clean_code"
        assert q_score == 38.5
        assert verdict == "WAG"
        assert passed is True
        assert duration_ms == 12.0
        assert source == "test_source"
        assert error == ""

    async def test_skips_if_pool_is_none(self):
        reg = BenchmarkRegistry(pool=None)
        # Must not raise; just returns silently
        await reg.record_evolve([_make_result()])

    async def test_p3_vetoed_records_zero_q(self):
        pool, conn = _make_pool()
        reg = BenchmarkRegistry(pool)
        r = _make_result("P3:dangerous_act", q_score=0.0, verdict="BARK", passed=True)
        await reg.record_evolve([r])
        row = conn.executemany.call_args[0][1][0]
        assert row[2] == 0.0     # q_score
        assert row[3] == "BARK"  # verdict

    async def test_five_probes_batch(self):
        """All 5 probes in one call → executemany receives 5 rows."""
        pool, conn = _make_pool()
        reg = BenchmarkRegistry(pool)
        results = [
            _make_result(f"P{i}:probe_{i}", q_score=float(30 + i))
            for i in range(1, 6)
        ]
        await reg.record_evolve(results)
        rows = conn.executemany.call_args[0][1]
        assert len(rows) == 5


# ── snapshot ──────────────────────────────────────────────────────────────────

class TestSnapshot:
    async def test_returns_empty_if_pool_is_none(self):
        reg = BenchmarkRegistry(pool=None)
        result = await reg.snapshot()
        assert result == {}

    async def test_returns_empty_if_no_runs(self):
        pool, conn = _make_pool(fetch_return=[])
        reg = BenchmarkRegistry(pool)
        result = await reg.snapshot()
        assert result == {}
        conn.executemany.assert_not_called()

    async def test_aggregates_per_probe(self):
        pool, conn = _make_pool(fetch_return=[
            {"probe_id": "P1", "probe_name": "P1:clean_code", "q_score": 38.0, "passed": True},
            {"probe_id": "P1", "probe_name": "P1:clean_code", "q_score": 37.0, "passed": True},
            {"probe_id": "P1", "probe_name": "P1:clean_code", "q_score": 36.0, "passed": True},
        ])
        reg = BenchmarkRegistry(pool)
        result = await reg.snapshot(window_runs=10)

        assert "P1" in result
        assert result["P1"]["run_count"] == 3
        assert result["P1"]["pass_rate"] == pytest.approx(1.0)
        assert result["P1"]["mean_q"] == pytest.approx(37.0)

    async def test_partial_pass_rate(self):
        pool, conn = _make_pool(fetch_return=[
            {"probe_id": "P2", "probe_name": "P2:smelly_code", "q_score": 20.0, "passed": True},
            {"probe_id": "P2", "probe_name": "P2:smelly_code", "q_score": 10.0, "passed": False},
            {"probe_id": "P2", "probe_name": "P2:smelly_code", "q_score": 10.0, "passed": False},
            {"probe_id": "P2", "probe_name": "P2:smelly_code", "q_score": 10.0, "passed": False},
        ])
        reg = BenchmarkRegistry(pool)
        result = await reg.snapshot()

        assert result["P2"]["pass_rate"] == pytest.approx(0.25)

    async def test_saves_snapshot_row(self):
        pool, conn = _make_pool(fetch_return=[
            {"probe_id": "P1", "probe_name": "P1:clean_code", "q_score": 38.0, "passed": True},
        ])
        reg = BenchmarkRegistry(pool)
        await reg.snapshot()
        conn.executemany.assert_called_once()
        snap_rows = conn.executemany.call_args[0][1]
        assert len(snap_rows) == 1
        probe_id, probe_name, pass_rate, mean_q, std_q, run_count, source = snap_rows[0]
        assert probe_id == "P1"
        assert pass_rate == pytest.approx(1.0)
        assert mean_q == pytest.approx(38.0)
        assert std_q == pytest.approx(0.0)
        assert run_count == 1

    async def test_multiple_probes(self):
        pool, conn = _make_pool(fetch_return=[
            {"probe_id": "P1", "probe_name": "P1:clean_code", "q_score": 38.0, "passed": True},
            {"probe_id": "P3", "probe_name": "P3:dangerous_act", "q_score": 0.0, "passed": True},
        ])
        reg = BenchmarkRegistry(pool)
        result = await reg.snapshot()
        assert len(result) == 2
        assert "P1" in result
        assert "P3" in result


# ── drift_alerts ──────────────────────────────────────────────────────────────

class TestDriftAlerts:
    async def test_returns_empty_if_pool_is_none(self):
        reg = BenchmarkRegistry(pool=None)
        alerts = await reg.drift_alerts()
        assert alerts == []

    async def test_returns_empty_if_no_snapshots(self):
        pool, conn = _make_pool(fetch_return=[])
        reg = BenchmarkRegistry(pool)
        alerts = await reg.drift_alerts()
        assert alerts == []

    async def test_detects_critical_drop(self):
        """Drop of 30% >= threshold 15% → CRITICAL alert."""
        pool, conn = _make_pool(fetch_return=[
            {"probe_id": "P1", "probe_name": "P1:clean_code",
             "pass_rate": 0.60, "mean_q": 35.0, "snapshot_ts": "2026-02-18", "rn": 1},
            {"probe_id": "P1", "probe_name": "P1:clean_code",
             "pass_rate": 0.90, "mean_q": 38.0, "snapshot_ts": "2026-02-17", "rn": 2},
        ])
        reg = BenchmarkRegistry(pool)
        alerts = await reg.drift_alerts(threshold=0.15)

        assert len(alerts) == 1
        assert alerts[0]["probe_id"] == "P1"
        assert alerts[0]["delta"] == pytest.approx(0.30)
        assert alerts[0]["severity"] == "CRITICAL"

    async def test_detects_warning_drop(self):
        """Drop of 20% (< 30%) → WARNING severity."""
        pool, conn = _make_pool(fetch_return=[
            {"probe_id": "P2", "probe_name": "P2:smelly_code",
             "pass_rate": 0.70, "mean_q": 30.0, "snapshot_ts": "2026-02-18", "rn": 1},
            {"probe_id": "P2", "probe_name": "P2:smelly_code",
             "pass_rate": 0.90, "mean_q": 35.0, "snapshot_ts": "2026-02-17", "rn": 2},
        ])
        reg = BenchmarkRegistry(pool)
        alerts = await reg.drift_alerts(threshold=0.15)

        assert len(alerts) == 1
        assert alerts[0]["severity"] == "WARNING"
        assert alerts[0]["delta"] == pytest.approx(0.20)

    async def test_no_alert_if_stable(self):
        """Drop of 5% < 15% threshold → no alert."""
        pool, conn = _make_pool(fetch_return=[
            {"probe_id": "P1", "probe_name": "P1:clean_code",
             "pass_rate": 0.95, "mean_q": 38.0, "snapshot_ts": "2026-02-18", "rn": 1},
            {"probe_id": "P1", "probe_name": "P1:clean_code",
             "pass_rate": 1.00, "mean_q": 39.0, "snapshot_ts": "2026-02-17", "rn": 2},
        ])
        reg = BenchmarkRegistry(pool)
        alerts = await reg.drift_alerts(threshold=0.15)
        assert len(alerts) == 0

    async def test_no_alert_with_single_snapshot(self):
        """Single snapshot — no previous to compare against."""
        pool, conn = _make_pool(fetch_return=[
            {"probe_id": "P1", "probe_name": "P1:clean_code",
             "pass_rate": 0.80, "mean_q": 38.0, "snapshot_ts": "2026-02-18", "rn": 1},
        ])
        reg = BenchmarkRegistry(pool)
        alerts = await reg.drift_alerts()
        assert len(alerts) == 0

    async def test_improvement_not_an_alert(self):
        """Pass_rate went UP → delta is negative → no alert."""
        pool, conn = _make_pool(fetch_return=[
            {"probe_id": "P4", "probe_name": "P4:cynic_self_state",
             "pass_rate": 1.00, "mean_q": 40.0, "snapshot_ts": "2026-02-18", "rn": 1},
            {"probe_id": "P4", "probe_name": "P4:cynic_self_state",
             "pass_rate": 0.80, "mean_q": 37.0, "snapshot_ts": "2026-02-17", "rn": 2},
        ])
        reg = BenchmarkRegistry(pool)
        alerts = await reg.drift_alerts(threshold=0.15)
        assert len(alerts) == 0

    async def test_multiple_probes_mixed_alerts(self):
        """P1 stable, P2 drops → only P2 alert."""
        pool, conn = _make_pool(fetch_return=[
            # P1: stable
            {"probe_id": "P1", "probe_name": "P1:clean_code",
             "pass_rate": 0.95, "mean_q": 38.0, "snapshot_ts": "2026-02-18", "rn": 1},
            {"probe_id": "P1", "probe_name": "P1:clean_code",
             "pass_rate": 1.00, "mean_q": 39.0, "snapshot_ts": "2026-02-17", "rn": 2},
            # P2: drops 30%
            {"probe_id": "P2", "probe_name": "P2:smelly_code",
             "pass_rate": 0.60, "mean_q": 20.0, "snapshot_ts": "2026-02-18", "rn": 1},
            {"probe_id": "P2", "probe_name": "P2:smelly_code",
             "pass_rate": 0.90, "mean_q": 30.0, "snapshot_ts": "2026-02-17", "rn": 2},
        ])
        reg = BenchmarkRegistry(pool)
        alerts = await reg.drift_alerts(threshold=0.15)

        assert len(alerts) == 1
        assert alerts[0]["probe_id"] == "P2"


# ── Orchestrator integration ──────────────────────────────────────────────────

class TestOrchestratorIntegration:
    """evolve() calls benchmark_registry.record_evolve() when wired."""

    async def test_evolve_calls_record_when_wired(self):
        from unittest.mock import patch, AsyncMock as AM
        from cynic.cognition.cortex.orchestrator import JudgeOrchestrator
        from cynic.core.judgment import Judgment
        from cynic.core.phi import PHI_INV_2

        orch = JudgeOrchestrator(
            dogs=MagicMock(),
            axiom_arch=MagicMock(),
            cynic_dog=MagicMock(),
        )

        # Wire a mock BenchmarkRegistry
        mock_reg = MagicMock()
        mock_reg.record_evolve = AsyncMock()
        orch.benchmark_registry = mock_reg

        # Stub orchestrator.run()
        from cynic.core.judgment import Cell
        from cynic.core.axioms import verdict_from_q_score
        cell = Cell(reality="CODE", analysis="JUDGE", content="x", time_dim="PRESENT")
        verdict = verdict_from_q_score(42.0).value
        judgment = Judgment(
            cell=cell, q_score=42.0, verdict=verdict,
            confidence=PHI_INV_2, axiom_scores={}, active_axioms=[],
            dog_votes={}, consensus_votes=3, consensus_quorum=3,
            consensus_reached=True, cost_usd=0.0, duration_ms=10.0,
        )
        orch.run = AsyncMock(return_value=judgment)

        await orch.evolve()

        mock_reg.record_evolve.assert_called_once()
        # Called with 5 ProbeResult objects
        call_results = mock_reg.record_evolve.call_args[0][0]
        assert len(call_results) == 5

    async def test_evolve_no_crash_if_registry_fails(self):
        """If record_evolve() raises, evolve() must still return summary."""
        from cynic.cognition.cortex.orchestrator import JudgeOrchestrator
        from cynic.core.judgment import Judgment, Cell
        from cynic.core.phi import PHI_INV_2
        from cynic.core.axioms import verdict_from_q_score

        orch = JudgeOrchestrator(
            dogs=MagicMock(),
            axiom_arch=MagicMock(),
            cynic_dog=MagicMock(),
        )
        mock_reg = MagicMock()
        mock_reg.record_evolve = AsyncMock(side_effect=RuntimeError("DB exploded"))
        orch.benchmark_registry = mock_reg

        cell = Cell(reality="CODE", analysis="JUDGE", content="x", time_dim="PRESENT")
        verdict = verdict_from_q_score(42.0).value
        judgment = Judgment(
            cell=cell, q_score=42.0, verdict=verdict,
            confidence=PHI_INV_2, axiom_scores={}, active_axioms=[],
            dog_votes={}, consensus_votes=3, consensus_quorum=3,
            consensus_reached=True, cost_usd=0.0, duration_ms=10.0,
        )
        orch.run = AsyncMock(return_value=judgment)

        summary = await orch.evolve()
        assert summary["total"] == 5  # evolve() completed despite DB error

    async def test_evolve_skips_record_if_not_wired(self):
        """benchmark_registry=None (default) → no record call, no error."""
        from cynic.cognition.cortex.orchestrator import JudgeOrchestrator
        from cynic.core.judgment import Judgment, Cell
        from cynic.core.phi import PHI_INV_2
        from cynic.core.axioms import verdict_from_q_score

        orch = JudgeOrchestrator(
            dogs=MagicMock(),
            axiom_arch=MagicMock(),
            cynic_dog=MagicMock(),
        )
        assert orch.benchmark_registry is None  # default

        cell = Cell(reality="CODE", analysis="JUDGE", content="x", time_dim="PRESENT")
        verdict = verdict_from_q_score(42.0).value
        judgment = Judgment(
            cell=cell, q_score=42.0, verdict=verdict,
            confidence=PHI_INV_2, axiom_scores={}, active_axioms=[],
            dog_votes={}, consensus_votes=3, consensus_quorum=3,
            consensus_reached=True, cost_usd=0.0, duration_ms=10.0,
        )
        orch.run = AsyncMock(return_value=judgment)

        # Must not raise
        summary = await orch.evolve()
        assert summary["total"] == 5
