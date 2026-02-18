"""
CYNIC CircuitBreaker Tests (topology M1)

Tests state transitions, φ-thresholds, cooldown logic, and orchestrator
integration. No LLM, no DB — pure in-memory state machine.
"""
from __future__ import annotations

import time
import pytest
from unittest.mock import MagicMock, AsyncMock, patch

from cynic.judge.circuit_breaker import CircuitBreaker, CircuitState
from cynic.core.phi import fibonacci, PHI_INV_2


# ── State machine basics ───────────────────────────────────────────────────

class TestCircuitBreakerInitial:
    def test_starts_closed(self):
        cb = CircuitBreaker()
        assert cb.state == CircuitState.CLOSED

    def test_starts_with_zero_failures(self):
        cb = CircuitBreaker()
        assert cb.failure_count == 0

    def test_allow_when_closed(self):
        cb = CircuitBreaker()
        assert cb.allow() is True

    def test_stats_keys(self):
        cb = CircuitBreaker()
        s = cb.stats()
        assert "state" in s
        assert "failure_count" in s
        assert "failure_threshold" in s
        assert "cooldown_s" in s
        assert "elapsed_since_open_s" in s

    def test_failure_threshold_is_fibonacci_5(self):
        cb = CircuitBreaker()
        assert cb._failure_threshold == fibonacci(5)  # 5


# ── CLOSED → OPEN transition ───────────────────────────────────────────────

class TestClosedToOpen:
    def test_below_threshold_stays_closed(self):
        cb = CircuitBreaker(failure_threshold=5)
        for _ in range(4):
            cb.record_failure()
        assert cb.state == CircuitState.CLOSED

    def test_at_threshold_opens(self):
        cb = CircuitBreaker(failure_threshold=5)
        for _ in range(5):
            cb.record_failure()
        assert cb.state == CircuitState.OPEN

    def test_open_blocks_allow(self):
        cb = CircuitBreaker(failure_threshold=3)
        for _ in range(3):
            cb.record_failure()
        assert cb.allow() is False

    def test_success_resets_failure_count(self):
        cb = CircuitBreaker(failure_threshold=5)
        for _ in range(4):
            cb.record_failure()
        cb.record_success()
        assert cb.failure_count == 0
        assert cb.state == CircuitState.CLOSED

    def test_success_requires_full_threshold_again(self):
        """After reset, must reach threshold again to open."""
        cb = CircuitBreaker(failure_threshold=5)
        for _ in range(4):
            cb.record_failure()
        cb.record_success()
        for _ in range(4):  # 4 more — should still be CLOSED
            cb.record_failure()
        assert cb.state == CircuitState.CLOSED


# ── OPEN → HALF_OPEN transition ────────────────────────────────────────────

class TestOpenToHalfOpen:
    def test_open_with_no_cooldown_allows_probe(self):
        """Cooldown=0 → transition to HALF_OPEN immediately."""
        cb = CircuitBreaker(failure_threshold=1, cooldown_s=0.0)
        cb.record_failure()
        assert cb.state == CircuitState.OPEN
        assert cb.allow() is True  # Transitions to HALF_OPEN
        assert cb.state == CircuitState.HALF_OPEN

    def test_open_before_cooldown_blocks(self):
        """Cooldown not elapsed → remain OPEN."""
        cb = CircuitBreaker(failure_threshold=1, cooldown_s=9999.0)
        cb.record_failure()
        assert cb.allow() is False
        assert cb.state == CircuitState.OPEN

    def test_only_one_probe_in_half_open(self):
        """HALF_OPEN allows exactly one probe, then blocks."""
        cb = CircuitBreaker(failure_threshold=1, cooldown_s=0.0)
        cb.record_failure()
        assert cb.allow() is True   # First probe — allowed (transitions HALF_OPEN)
        assert cb.allow() is False  # Second probe — blocked (probe in-flight)


# ── HALF_OPEN → CLOSED / OPEN ─────────────────────────────────────────────

class TestHalfOpenTransitions:
    def _open_circuit(self, threshold=1, cooldown=0.0):
        cb = CircuitBreaker(failure_threshold=threshold, cooldown_s=cooldown)
        cb.record_failure()
        cb.allow()  # OPEN → HALF_OPEN
        return cb

    def test_probe_success_closes(self):
        cb = self._open_circuit()
        cb.record_success()
        assert cb.state == CircuitState.CLOSED
        assert cb.failure_count == 0

    def test_probe_failure_reopens(self):
        cb = self._open_circuit()
        cb.record_failure()
        assert cb.state == CircuitState.OPEN

    def test_closed_after_probe_success_allows_work(self):
        cb = self._open_circuit()
        cb.record_success()
        assert cb.allow() is True

    def test_reopened_after_probe_failure_blocks(self):
        cb = self._open_circuit(cooldown=9999.0)
        # Force HALF_OPEN manually
        cb._state = CircuitState.HALF_OPEN
        cb._probe_allowed = True
        cb.allow()  # consume probe
        cb.record_failure()  # probe failed → OPEN
        assert cb.allow() is False


# ── reset() ────────────────────────────────────────────────────────────────

class TestReset:
    def test_reset_closes_open_circuit(self):
        cb = CircuitBreaker(failure_threshold=1)
        cb.record_failure()
        assert cb.state == CircuitState.OPEN
        cb.reset()
        assert cb.state == CircuitState.CLOSED

    def test_reset_clears_failure_count(self):
        cb = CircuitBreaker(failure_threshold=3)
        for _ in range(2):
            cb.record_failure()
        cb.reset()
        assert cb.failure_count == 0

    def test_reset_allows_immediately(self):
        cb = CircuitBreaker(failure_threshold=1)
        cb.record_failure()
        cb.reset()
        assert cb.allow() is True


# ── φ-derived constants ────────────────────────────────────────────────────

class TestPhiConstants:
    def test_default_threshold_is_5(self):
        cb = CircuitBreaker()
        assert cb._failure_threshold == 5  # F(5)

    def test_default_cooldown_is_phi_inv2_60(self):
        cb = CircuitBreaker()
        expected = PHI_INV_2 * 60
        assert abs(cb._cooldown_s - expected) < 0.01

    def test_stats_state_matches_property(self):
        cb = CircuitBreaker(failure_threshold=1)
        cb.record_failure()
        s = cb.stats()
        assert s["state"] == CircuitState.OPEN.value
        assert s["failure_count"] == 1


# ── Orchestrator integration ───────────────────────────────────────────────

class TestOrchestratorCircuitBreaker:
    """
    Smoke-test that JudgeOrchestrator has a CircuitBreaker and exposes it
    in stats(). Full cascade test would require async fixtures.
    """

    def test_orchestrator_has_circuit_breaker(self):
        from cynic.core.axioms import AxiomArchitecture
        from cynic.core.heuristic_scorer import HeuristicFacetScorer
        from cynic.dogs.cynic_dog import CynicDog
        from cynic.judge.orchestrator import JudgeOrchestrator
        from cynic.judge.circuit_breaker import CircuitBreaker

        orchestrator = JudgeOrchestrator(
            dogs={},
            axiom_arch=AxiomArchitecture(facet_scorer=HeuristicFacetScorer()),
            cynic_dog=CynicDog(),
        )
        assert hasattr(orchestrator, "_circuit_breaker")
        assert isinstance(orchestrator._circuit_breaker, CircuitBreaker)

    def test_orchestrator_stats_has_circuit_key(self):
        from cynic.core.axioms import AxiomArchitecture
        from cynic.core.heuristic_scorer import HeuristicFacetScorer
        from cynic.dogs.cynic_dog import CynicDog
        from cynic.judge.orchestrator import JudgeOrchestrator

        orchestrator = JudgeOrchestrator(
            dogs={},
            axiom_arch=AxiomArchitecture(facet_scorer=HeuristicFacetScorer()),
            cynic_dog=CynicDog(),
        )
        s = orchestrator.stats()
        assert "circuit_breaker" in s
        assert s["circuit_breaker"]["state"] == "CLOSED"

    @pytest.mark.asyncio
    async def test_orchestrator_trips_after_consecutive_failures(self):
        """
        When all Dogs raise exceptions, circuit opens after F(5)=5 failures
        and subsequent calls are fast-failed with RuntimeError.
        """
        from cynic.core.axioms import AxiomArchitecture
        from cynic.core.heuristic_scorer import HeuristicFacetScorer
        from cynic.dogs.cynic_dog import CynicDog
        from cynic.dogs.base import DogId
        from cynic.judge.orchestrator import JudgeOrchestrator
        from cynic.judge.circuit_breaker import CircuitState
        from cynic.core.judgment import Cell
        from cynic.core.event_bus import reset_all_buses

        reset_all_buses()

        # Dog that always raises
        bad_dog = AsyncMock(side_effect=RuntimeError("dog always fails"))
        bad_dog.dog_id = DogId.GUARDIAN
        bad_dog.veto = False

        cynic_dog = CynicDog()

        orchestrator = JudgeOrchestrator(
            dogs={DogId.GUARDIAN: bad_dog},
            axiom_arch=AxiomArchitecture(facet_scorer=HeuristicFacetScorer()),
            cynic_dog=cynic_dog,
        )
        # Override threshold to 2 for fast test
        orchestrator._circuit_breaker._failure_threshold = 2

        cell = Cell(reality="CODE", analysis="JUDGE", time_dim="PRESENT", content="x=1")

        # First 2 failures — circuit opens
        for _ in range(2):
            with pytest.raises(Exception):
                await orchestrator.run(cell)

        assert orchestrator._circuit_breaker.state == CircuitState.OPEN

        # Next call — circuit is OPEN → fast-fail
        with pytest.raises(RuntimeError, match="CircuitBreaker OPEN"):
            await orchestrator.run(cell)

        reset_all_buses()
