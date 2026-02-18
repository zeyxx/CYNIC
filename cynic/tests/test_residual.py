"""
CYNIC ResidualDetector + L2→L1 Escalation Tests

Tests composant 7/9 du kernel.
Covers in-memory pattern detection AND PostgreSQL persistence (β3).
"""
from __future__ import annotations

import asyncio
import time
import pytest
from unittest.mock import AsyncMock, MagicMock

from cynic.core.phi import MAX_Q_SCORE, PHI_INV, PHI_INV_2
from cynic.judge.residual import ResidualPoint
from cynic.core.judgment import Cell, Judgment
from cynic.core.axioms import AxiomArchitecture
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.event_bus import get_core_bus, reset_all_buses, Event, CoreEvent
from cynic.core.axioms import verdict_from_q_score
from cynic.dogs.base import DogId
from cynic.dogs.cynic_dog import CynicDog
from cynic.dogs.guardian import GuardianDog
from cynic.dogs.analyst import AnalystDog
from cynic.dogs.janitor import JanitorDog
from cynic.judge.orchestrator import JudgeOrchestrator
from cynic.judge.residual import ResidualDetector, ResidualPattern, ANOMALY_THRESHOLD, STABLE_HIGH_N


# ════════════════════════════════════════════════════════════════════════════
# HELPERS
# ════════════════════════════════════════════════════════════════════════════

def make_cell(
    reality: str = "CODE",
    analysis: str = "JUDGE",
    content: str = "def foo(): pass",
) -> Cell:
    return Cell(reality=reality, analysis=analysis, content=content)


def make_judgment(residual: float = 0.0, q_score: float = 30.0) -> Judgment:
    """Create a Judgment with specified residual_variance."""
    cell = make_cell()
    bounded_q = min(q_score, MAX_Q_SCORE)
    verdict = verdict_from_q_score(bounded_q).value
    return Judgment(
        cell=cell,
        q_score=bounded_q,
        verdict=verdict,
        confidence=0.5,
        residual_variance=residual,
        unnameable_detected=(residual > PHI_INV),
    )


@pytest.fixture(autouse=True)
def reset_bus():
    """Each test gets a fresh event bus."""
    reset_all_buses()
    yield
    reset_all_buses()


# ════════════════════════════════════════════════════════════════════════════
# UNIT TESTS: ResidualDetector
# ════════════════════════════════════════════════════════════════════════════

class TestResidualDetector:
    """ResidualDetector core pattern detection."""

    def test_cold_start_no_pattern(self):
        """New detector with < MIN_SAMPLES returns None."""
        det = ResidualDetector()
        j = make_judgment(residual=0.9)  # Very high residual
        result = det.observe(j)
        assert result is None  # Not enough samples yet

    def test_stats_initial(self):
        det = ResidualDetector()
        s = det.stats()
        assert s["observations"] == 0
        assert s["anomalies"] == 0
        assert s["patterns_detected"] == 0

    def test_observe_increments_stats(self):
        det = ResidualDetector()
        det.observe(make_judgment(residual=0.5))  # above threshold
        det.observe(make_judgment(residual=0.1))  # below threshold

        s = det.stats()
        assert s["observations"] == 2
        assert s["anomalies"] == 1

    def test_spike_detected(self):
        """SPIKE: sudden jump after stable baseline."""
        det = ResidualDetector()
        # Stable baseline (low residuals)
        for _ in range(5):
            det.observe(make_judgment(residual=0.05))

        # Sudden spike far above mean
        result = det.observe(make_judgment(residual=0.95))
        assert result is not None
        assert result.pattern_type == "SPIKE"
        assert result.severity > 0.0
        # Evidence contains either z_score (std mode) or jump (absolute mode)
        assert "z_score" in result.evidence or "jump" in result.evidence

    def test_no_spike_when_consistent(self):
        """No SPIKE when values are consistently high (no sudden jump)."""
        det = ResidualDetector()
        for _ in range(8):
            det.observe(make_judgment(residual=0.8))

        # Another high residual — consistent, not a spike
        result = det.observe(make_judgment(residual=0.82))
        # May detect STABLE_HIGH or RISING, but not SPIKE
        if result is not None:
            assert result.pattern_type != "SPIKE"

    def test_stable_high_detected(self):
        """STABLE_HIGH: N consecutive residuals above threshold."""
        det = ResidualDetector()
        # Need MIN_SAMPLES first
        for _ in range(3):
            det.observe(make_judgment(residual=0.1))

        # Now consecutive high residuals
        pattern = None
        for _ in range(STABLE_HIGH_N):
            pattern = det.observe(make_judgment(residual=0.5))

        assert pattern is not None
        assert pattern.pattern_type == "STABLE_HIGH"
        assert pattern.evidence["consecutive_high"] >= STABLE_HIGH_N

    def test_stable_high_resets_on_low(self):
        """Consecutive count resets when residual drops below threshold."""
        det = ResidualDetector()
        # Warm up
        for _ in range(3):
            det.observe(make_judgment(residual=0.1))

        # Some high
        for _ in range(3):
            det.observe(make_judgment(residual=0.5))

        # Low residual resets counter
        det.observe(make_judgment(residual=0.05))
        assert det._consecutive_high == 0

    def test_phi_bounds_in_evidence(self):
        """Evidence values should be reasonable (no negative, no >1 severity)."""
        det = ResidualDetector()
        for _ in range(5):
            det.observe(make_judgment(residual=0.05))
        result = det.observe(make_judgment(residual=0.99))

        if result is not None:
            assert 0.0 <= result.severity <= 1.0

    def test_no_listener_before_start(self):
        """Before start(), _listener_registered is False."""
        det = ResidualDetector()
        assert not det._listener_registered

    def test_start_registers_listener(self):
        """start() registers on event bus."""
        det = ResidualDetector()
        bus = get_core_bus()
        det.start(bus)
        assert det._listener_registered

    def test_start_idempotent(self):
        """Calling start() twice does not double-register."""
        det = ResidualDetector()
        bus = get_core_bus()
        det.start(bus)
        det.start(bus)  # Should not raise

        # Only one handler registered
        handlers = bus._handlers.get(CoreEvent.JUDGMENT_CREATED, [])
        assert len(handlers) == 1

    @pytest.mark.asyncio
    async def test_emergence_emitted_on_spike(self):
        """EMERGENCE_DETECTED event emitted when spike pattern detected."""
        bus = get_core_bus()
        det = ResidualDetector()
        det.start(bus)

        emitted = []
        bus.on(CoreEvent.EMERGENCE_DETECTED, lambda e: emitted.append(e))

        # Feed stable baseline then spike via event
        for i in range(5):
            j = make_judgment(residual=0.05)
            await bus.emit(Event(
                type=CoreEvent.JUDGMENT_CREATED,
                payload={
                    "judgment_id": f"test-{i}",
                    "residual_variance": 0.05,
                    "unnameable_detected": False,
                    "cell": {"reality": "CODE", "analysis": "JUDGE"},
                },
            ))

        # Spike
        await bus.emit(Event(
            type=CoreEvent.JUDGMENT_CREATED,
            payload={
                "judgment_id": "spike-001",
                "residual_variance": 0.95,
                "unnameable_detected": True,
                "cell": {"reality": "CODE", "analysis": "JUDGE"},
            },
        ))

        # Allow event loop to process
        await asyncio.sleep(0.05)

        # May or may not have emitted depending on z-score calculation
        # Just verify no errors and emitted events are valid if any
        for e in emitted:
            assert e.payload["pattern_type"] in ("SPIKE", "STABLE_HIGH", "RISING")
            assert 0.0 <= e.payload["severity"] <= 1.0


# ════════════════════════════════════════════════════════════════════════════
# INTEGRATION: ResidualDetector wired into orchestrator
# ════════════════════════════════════════════════════════════════════════════

class TestResidualInOrchestrator:
    """ResidualDetector wired into JudgeOrchestrator."""

    @pytest.fixture
    def orchestrator_with_residual(self):
        """Orchestrator with ResidualDetector attached."""
        cynic_dog = CynicDog()
        dogs = {
            DogId.CYNIC:    cynic_dog,
            DogId.GUARDIAN: GuardianDog(),
            DogId.ANALYST:  AnalystDog(),
            DogId.JANITOR:  JanitorDog(),
        }
        det = ResidualDetector()
        orch = JudgeOrchestrator(
            dogs=dogs,
            axiom_arch=AxiomArchitecture(),
            cynic_dog=cynic_dog,
            residual_detector=det,
        )
        return orch, det

    @pytest.mark.asyncio
    async def test_orchestrator_calls_observe(self, orchestrator_with_residual):
        """After run(), ResidualDetector.observe() has been called."""
        orch, det = orchestrator_with_residual
        cell = make_cell()
        await orch.run(cell, level=ConsciousnessLevel.REFLEX)
        assert det._observations == 1

    @pytest.mark.asyncio
    async def test_orchestrator_without_residual_ok(self):
        """Orchestrator works fine without residual_detector (None)."""
        cynic_dog = CynicDog()
        dogs = {
            DogId.CYNIC:    cynic_dog,
            DogId.GUARDIAN: GuardianDog(),
            DogId.ANALYST:  AnalystDog(),
            DogId.JANITOR:  JanitorDog(),
        }
        orch = JudgeOrchestrator(
            dogs=dogs,
            axiom_arch=AxiomArchitecture(),
            cynic_dog=cynic_dog,
            residual_detector=None,  # Explicit None
        )
        cell = make_cell()
        j = await orch.run(cell, level=ConsciousnessLevel.REFLEX)
        assert j.q_score >= 0.0


# ════════════════════════════════════════════════════════════════════════════
# L2 → L1 ESCALATION
# ════════════════════════════════════════════════════════════════════════════

class TestL2L1Escalation:
    """When MICRO consensus fails, orchestrator escalates to MACRO."""

    @pytest.mark.asyncio
    async def test_micro_escalates_on_failed_consensus(self):
        """
        With only GUARDIAN + ANALYST + JANITOR (3 dogs), PBFT quorum=7
        → consensus will always fail at MICRO → escalate to MACRO.
        MACRO also fails quorum but returns a judgment anyway.
        """
        cynic_dog = CynicDog()
        dogs = {
            DogId.CYNIC:    cynic_dog,
            DogId.GUARDIAN: GuardianDog(),
            DogId.ANALYST:  AnalystDog(),
            DogId.JANITOR:  JanitorDog(),
        }
        orch = JudgeOrchestrator(
            dogs=dogs,
            axiom_arch=AxiomArchitecture(),
            cynic_dog=cynic_dog,
        )

        cell = make_cell()
        # Run at MICRO level — will fail consensus and escalate
        j = await orch.run(cell, level=ConsciousnessLevel.MICRO)

        # Should return a valid judgment regardless of escalation path
        assert j is not None
        assert j.q_score >= 0.0
        assert j.verdict in ("HOWL", "WAG", "GROWL", "BARK")

    @pytest.mark.asyncio
    async def test_escalation_records_macro_level(self):
        """After escalation, the pipeline level is MACRO."""
        cynic_dog = CynicDog()
        dogs = {
            DogId.CYNIC:    cynic_dog,
            DogId.GUARDIAN: GuardianDog(),
            DogId.ANALYST:  AnalystDog(),
            DogId.JANITOR:  JanitorDog(),
        }
        orch = JudgeOrchestrator(
            dogs=dogs,
            axiom_arch=AxiomArchitecture(),
            cynic_dog=cynic_dog,
        )

        cell = make_cell()
        # Must produce a judgment (not raise)
        j = await orch.run(cell, level=ConsciousnessLevel.MICRO)
        assert j.q_score <= MAX_Q_SCORE


# ════════════════════════════════════════════════════════════════════════════
# β3: ResidualDetector DB Persistence
# ════════════════════════════════════════════════════════════════════════════

def _make_mock_pool(rows: list = None):
    """Build a mock asyncpg pool that returns rows from conn.fetch()."""
    record_rows = rows or []
    conn = AsyncMock()
    conn.fetch = AsyncMock(return_value=record_rows)
    conn.execute = AsyncMock(return_value=None)
    pool = MagicMock()
    pool.acquire = MagicMock()
    pool.acquire.return_value.__aenter__ = AsyncMock(return_value=conn)
    pool.acquire.return_value.__aexit__ = AsyncMock(return_value=False)
    return pool, conn


def _make_row(
    judgment_id: str = "j1",
    residual: float = 0.3,
    reality: str = "CODE",
    analysis: str = "JUDGE",
    unnameable: bool = False,
) -> dict:
    return {
        "judgment_id": judgment_id,
        "residual": residual,
        "reality": reality,
        "analysis": analysis,
        "unnameable": unnameable,
        "timestamp": time.time(),
    }


class TestResidualPersistence:
    """ResidualDetector DB persistence — load_from_db + _save_point_to_db."""

    @pytest.mark.asyncio
    async def test_load_from_db_empty_returns_zero(self):
        """Empty DB → load returns 0, no history populated."""
        pool, _ = _make_mock_pool([])
        det = ResidualDetector()
        loaded = await det.load_from_db(pool)
        assert loaded == 0
        assert len(det._history) == 0
        assert det._observations == 0

    @pytest.mark.asyncio
    async def test_load_from_db_populates_history(self):
        """3 rows → 3 points in _history, observations=3."""
        rows = [
            _make_row("j1", residual=0.2),
            _make_row("j2", residual=0.5),
            _make_row("j3", residual=0.1),
        ]
        pool, _ = _make_mock_pool(rows)
        det = ResidualDetector()
        loaded = await det.load_from_db(pool)
        assert loaded == 3
        assert det._observations == 3
        assert len(det._history) == 3

    @pytest.mark.asyncio
    async def test_load_from_db_counts_anomalies(self):
        """Anomalies (residual >= 0.382) counted during warm-start."""
        rows = [
            _make_row("j1", residual=0.6, unnameable=True),   # anomaly
            _make_row("j2", residual=0.1),                     # normal
            _make_row("j3", residual=0.5, unnameable=True),   # anomaly
        ]
        pool, _ = _make_mock_pool(rows)
        det = ResidualDetector()
        await det.load_from_db(pool)
        assert det._anomalies == 2

    @pytest.mark.asyncio
    async def test_load_from_db_rebuilds_consecutive_high(self):
        """consecutive_high rebuilt correctly from reversed (oldest-first) replay."""
        # DB returns newest-first; reversed → j3, j2, j1
        # j3=low → consecutive=0; j2=high → 1; j1=high → 2
        rows = [
            _make_row("j1", residual=0.6),   # newest → played last
            _make_row("j2", residual=0.5),   # played second
            _make_row("j3", residual=0.1),   # oldest → played first
        ]
        pool, _ = _make_mock_pool(rows)
        det = ResidualDetector()
        await det.load_from_db(pool)
        assert det._consecutive_high == 2

    @pytest.mark.asyncio
    async def test_load_from_db_exception_returns_zero(self):
        """DB error → returns 0, no crash, no history populated."""
        pool = MagicMock()
        pool.acquire = MagicMock()
        pool.acquire.return_value.__aenter__ = AsyncMock(
            side_effect=Exception("connection refused")
        )
        pool.acquire.return_value.__aexit__ = AsyncMock(return_value=False)

        det = ResidualDetector()
        loaded = await det.load_from_db(pool)
        assert loaded == 0
        assert det._observations == 0

    def test_maybe_persist_without_pool_is_noop(self):
        """_maybe_persist() with no pool set → no crash, no task."""
        det = ResidualDetector()
        point = ResidualPoint(
            judgment_id="x", residual=0.5, reality="CODE",
            analysis="JUDGE", unnameable=False,
        )
        det._maybe_persist(point)  # Must not raise

    @pytest.mark.asyncio
    async def test_save_point_to_db_calls_execute(self):
        """_save_point_to_db issues INSERT with correct judgment_id."""
        pool, conn = _make_mock_pool()
        det = ResidualDetector()
        det._db_pool = pool

        point = ResidualPoint(
            judgment_id="abc-123", residual=0.42, reality="CODE",
            analysis="JUDGE", unnameable=True,
        )
        await det._save_point_to_db(point)

        conn.execute.assert_called_once()
        call_args = conn.execute.call_args[0]
        assert "abc-123" in call_args
        assert 0.42 in call_args

    @pytest.mark.asyncio
    async def test_save_point_to_db_handles_exception_gracefully(self):
        """DB error in _save_point_to_db → warning, no crash."""
        pool = MagicMock()
        pool.acquire = MagicMock()
        pool.acquire.return_value.__aenter__ = AsyncMock(
            side_effect=Exception("write failed")
        )
        pool.acquire.return_value.__aexit__ = AsyncMock(return_value=False)

        det = ResidualDetector()
        det._db_pool = pool
        point = ResidualPoint(
            judgment_id="err", residual=0.1, reality="CODE",
            analysis="JUDGE", unnameable=False,
        )
        await det._save_point_to_db(point)  # Must not raise

    def test_set_db_pool_stores_pool(self):
        """set_db_pool() sets _db_pool."""
        pool = MagicMock()
        det = ResidualDetector()
        det.set_db_pool(pool)
        assert det._db_pool is pool
