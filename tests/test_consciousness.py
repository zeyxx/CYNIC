"""
Tests for Consciousness Module

Tests the 4-level consciousness system:
- REFLEX, MICRO, MACRO, META levels
- ConsciousnessGradient
- CycleTimer
- ConsciousnessState machine
"""

import pytest

pytestmark = pytest.mark.skip(
    reason="Old architecture: module imports not available in V5"
)

# Block all imports that would fail
pytest.skip("Skipping old architecture test module", allow_module_level=True)

import pytest

pytestmark = pytest.mark.skip(
    reason="Old architecture removed in V5 - ConsciousnessGradient not exported"
)


from cynic.kernel.core.consciousness import (
    MACRO_DOGS,
    REFLEX_DOGS,
    ConsciousnessGradient,
    ConsciousnessLevel,
    CycleTimer,
    dogs_for_level,
    get_consciousness,
    gradient_from_budget,
    reset_consciousness,
)
from cynic.kernel.core.phi import fibonacci


class TestConsciousnessLevel:
    """Test suite for ConsciousnessLevel enum."""

    def test_level_order(self):
        """Levels should have correct ordinal values."""
        assert ConsciousnessLevel.REFLEX.value == 3
        assert ConsciousnessLevel.MICRO.value == 2
        assert ConsciousnessLevel.MACRO.value == 1
        assert ConsciousnessLevel.META.value == 4

    def test_target_ms_reflex(self):
        """REFLEX should be fast (~8ms)."""
        target = ConsciousnessLevel.REFLEX.target_ms
        assert target < 10  # <10ms

    def test_target_ms_micro(self):
        """MICRO should be ~500ms."""
        target = ConsciousnessLevel.MICRO.target_ms
        assert 50 < target < 1000

    def test_target_ms_macro(self):
        """MACRO should be ~2.85s."""
        target = ConsciousnessLevel.MACRO.target_ms
        assert 300 < target < 5000

    def test_allows_llm(self):
        """Only REFLEX should not allow LLM."""
        assert ConsciousnessLevel.REFLEX.allows_llm is False
        assert ConsciousnessLevel.MICRO.allows_llm is True
        assert ConsciousnessLevel.MACRO.allows_llm is True
        assert ConsciousnessLevel.META.allows_llm is True

    def test_gradient_mapping(self):
        """Each level should map to a gradient."""
        assert ConsciousnessLevel.REFLEX.gradient == 0
        assert ConsciousnessLevel.MICRO.gradient == 2
        assert ConsciousnessLevel.MACRO.gradient == 4
        assert ConsciousnessLevel.META.gradient == 6


class TestDogsForLevel:
    """Test suite for dog selection by consciousness level."""

    def test_reflex_dogs(self):
        """REFLEX should have non-LLM dogs."""
        dogs = dogs_for_level(ConsciousnessLevel.REFLEX)

        assert "CYNIC" in dogs
        assert "GUARDIAN" in dogs
        assert "ANALYST" in dogs
        assert "JANITOR" in dogs

    def test_micro_dogs(self):
        """MICRO should include REFLEX + fast LLM dogs."""
        dogs = dogs_for_level(ConsciousnessLevel.MICRO)

        assert dogs.issuperset(REFLEX_DOGS)
        assert "SCHOLAR" in dogs

    def test_macro_dogs(self):
        """MACRO should have all 11 dogs."""
        dogs = dogs_for_level(ConsciousnessLevel.MACRO)

        assert len(dogs) == 11
        assert "SAGE" in dogs
        assert "DEPLOYER" in dogs

    def test_meta_dogs(self):
        """META should have all dogs."""
        dogs = dogs_for_level(ConsciousnessLevel.META)

        assert dogs == MACRO_DOGS


class TestConsciousnessGradient:
    """Test suite for ConsciousnessGradient enum."""

    def test_all_gradients_defined(self):
        """All 7 gradients should be defined."""
        assert len(list(ConsciousnessGradient)) == 7

    def test_gradient_values(self):
        """Gradients should have correct ordinals."""
        assert ConsciousnessGradient.REFLEX.value == 0
        assert ConsciousnessGradient.REACTIVE.value == 1
        assert ConsciousnessGradient.AWARE.value == 2
        assert ConsciousnessGradient.DELIBERATE.value == 3
        assert ConsciousnessGradient.REFLECTIVE.value == 4
        assert ConsciousnessGradient.METACOGNITIVE.value == 5
        assert ConsciousnessGradient.TRANSCENDENT.value == 6

    def test_min_budget_transcendent(self):
        """TRANSCENDENT should require highest budget."""
        budget = ConsciousnessGradient.TRANSCENDENT.min_budget_usd
        assert budget > ConsciousnessGradient.METACOGNITIVE.min_budget_usd


class TestGradientFromBudget:
    """Test suite for budget-to-gradient conversion."""

    def test_low_budget_reflex(self):
        """Very low budget should map to REFLEX."""
        assert gradient_from_budget(0.001) == 0
        assert gradient_from_budget(0.01) == 0

    def test_medium_budget_aware(self):
        """Medium budget should map to AWARE."""
        budget = gradient_from_budget(0.3)
        assert budget >= 2

    def test_high_budget_deliberate(self):
        """High budget should map to DELIBERATE or higher."""
        budget = gradient_from_budget(1.0)
        assert budget >= 3


class TestCycleTimer:
    """Test suite for CycleTimer."""

    @pytest.fixture
    def timer(self):
        """Create a fresh timer."""
        return CycleTimer(level=ConsciousnessLevel.MACRO)

    def test_initialization(self, timer):
        """Should initialize empty."""
        assert len(timer._samples) == 0
        assert timer._start is None

    def test_start_stop(self, timer):
        """Should measure elapsed time."""
        timer.start()
        # Simulate some work
        import time

        time.sleep(0.01)  # 10ms
        elapsed = timer.stop()

        assert elapsed > 0

    def test_record(self, timer):
        """Should record pre-measured elapsed time."""
        timer.record(100.0)

        assert len(timer._samples) == 1
        assert timer._samples[0] == 100.0

    def test_p50(self, timer):
        """Should compute p50 correctly."""
        timer.record(50.0)
        timer.record(100.0)
        timer.record(150.0)

        assert timer.p50_ms == 100.0

    def test_p95(self, timer):
        """Should compute p95 correctly."""
        for i in range(20):
            timer.record(float(i * 10))

        p95 = timer.p95_ms
        assert p95 > 100  # Should be high percentile

    def test_within_target(self, timer):
        """Should check if within target."""
        # Record times well under target
        timer.record(10.0)

        assert timer.within_target is True

    def test_health_excellent(self, timer):
        """Should return EXCELLENT when well under target."""
        timer.record(100.0)  # Way under MACRO target (~2850ms)

        assert timer.health == "EXCELLENT"

    def test_health_degraded(self, timer):
        """Should return DEGRADED when over target."""
        timer = CycleTimer(level=ConsciousnessLevel.REFLEX)
        timer.record(50.0)  # Way over REFLEX target (~8ms)

        assert timer.health in ["DEGRADED", "CRITICAL"]

    def test_max_samples(self, timer):
        """Should cap samples at F(10) = 55."""
        for i in range(100):
            timer.record(float(i))

        assert len(timer._samples) == 55

    def test_to_dict(self, timer):
        """Should serialize to dict."""
        timer.record(100.0)

        d = timer.to_dict()

        assert "level" in d
        assert "target_ms" in d
        assert "p50_ms" in d
        assert "health" in d


class TestConsciousnessState:
    """Test suite for ConsciousnessState."""

    @pytest.fixture
    def state(self):
        """Create a fresh state."""
        reset_consciousness()
        return get_consciousness()

    def test_initialization(self, state):
        """Should initialize with defaults."""
        assert state.active_level == ConsciousnessLevel.MACRO
        assert state.gradient == 2

    def test_increment_reflex(self, state):
        """Should increment REFLEX counter."""
        state.increment(ConsciousnessLevel.REFLEX)

        assert state.reflex_cycles == 1

    def test_increment_all_levels(self, state):
        """Should increment all level counters."""
        state.increment(ConsciousnessLevel.REFLEX)
        state.increment(ConsciousnessLevel.MICRO)
        state.increment(ConsciousnessLevel.MACRO)
        state.increment(ConsciousnessLevel.META)

        assert state.reflex_cycles == 1
        assert state.micro_cycles == 1
        assert state.macro_cycles == 1
        assert state.meta_cycles == 1

    def test_total_cycles(self, state):
        """Should sum all cycles."""
        state.increment(ConsciousnessLevel.REFLEX)
        state.increment(ConsciousnessLevel.REFLEX)
        state.increment(ConsciousnessLevel.MACRO)

        assert state.total_cycles == 3

    def test_should_downgrade_budget_exhausted(self, state):
        """Should downgrade when budget exhausted."""
        downgrade = state.should_downgrade(0.001)

        assert downgrade == ConsciousnessLevel.REFLEX

    def test_should_downgrade_budget_low(self, state):
        """Should downgrade when budget low."""
        downgrade = state.should_downgrade(0.02)

        assert downgrade == ConsciousnessLevel.MICRO

    def test_should_downgrade_timer_critical(self, state):
        """Should downgrade when timer is CRITICAL."""
        # Make timer critical
        timer = state.timers["MACRO"]
        for _ in range(10):
            timer.record(10000.0)  # Way over target

        downgrade = state.should_downgrade(100.0)

        # May or may not downgrade based on timer
        assert downgrade is None or downgrade == ConsciousnessLevel.MICRO

    def test_timers_initialized(self, state):
        """Should have timer for each level."""
        assert "REFLEX" in state.timers
        assert "MICRO" in state.timers
        assert "MACRO" in state.timers
        assert "META" in state.timers

    def test_to_dict(self, state):
        """Should serialize to dict."""
        state.increment(ConsciousnessLevel.MACRO)

        d = state.to_dict()

        assert "active_level" in d
        assert "gradient" in d
        assert "cycles" in d
        assert "timers" in d


class TestGetConsciousness:
    """Test suite for consciousness singleton."""

    def test_singleton(self):
        """Should return same instance."""
        reset_consciousness()

        c1 = get_consciousness()
        c2 = get_consciousness()

        assert c1 is c2

    def test_reset(self):
        """Reset should clear singleton."""
        c1 = get_consciousness()

        reset_consciousness()

        c2 = get_consciousness()
        assert c1 is not c2


class TestFibonacciTiming:
    """Test suite for Fibonacci-based timing."""

    def test_fibonacci_timing_reflex(self):
        """REFLEX timing should be F(3) * 3ms."""
        expected = fibonacci(3) * 3
        assert ConsciousnessLevel.REFLEX.target_ms == expected

    def test_fibonacci_timing_micro(self):
        """MICRO timing should be F(6) * 8ms."""
        expected = fibonacci(6) * 8
        assert ConsciousnessLevel.MICRO.target_ms == expected

    def test_fibonacci_timing_macro(self):
        """MACRO timing should be F(8) * 21ms."""
        expected = fibonacci(8) * 21
        assert ConsciousnessLevel.MACRO.target_ms == expected
