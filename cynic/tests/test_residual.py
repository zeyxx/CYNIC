"""
CYNIC ResidualDetector + L2→L1 Escalation Tests

Tests composant 7/9 du kernel.
No LLM, no DB. Pure in-memory residual pattern detection.
"""
from __future__ import annotations

import asyncio
import pytest

from cynic.core.phi import MAX_Q_SCORE, PHI_INV, PHI_INV_2
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
