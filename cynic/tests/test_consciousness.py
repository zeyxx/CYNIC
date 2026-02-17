"""
Tests: 4 Consciousness Levels

These 4 levels define HOW FAST CYNIC thinks.
Each has a different set of Dogs, timing targets, and budget requirements.
"""
import pytest
from cynic.core.consciousness import (
    ConsciousnessLevel, ConsciousnessGradient, ConsciousnessState,
    CycleTimer, dogs_for_level, gradient_from_budget,
    REFLEX_DOGS, MICRO_DOGS, MACRO_DOGS,
    get_consciousness, reset_consciousness,
)
from cynic.core.phi import PHI_INV, PHI_INV_2, fibonacci


class TestConsciousnessLevels:
    """The 4 consciousness levels (L3/L2/L1/L4)."""

    def test_four_levels_exist(self):
        levels = list(ConsciousnessLevel)
        assert len(levels) == 4
        names = {l.name for l in levels}
        assert names == {"REFLEX", "MICRO", "MACRO", "META"}

    def test_reflex_no_llm(self):
        assert ConsciousnessLevel.REFLEX.allows_llm == False

    def test_micro_allows_llm(self):
        assert ConsciousnessLevel.MICRO.allows_llm == True

    def test_macro_allows_llm(self):
        assert ConsciousnessLevel.MACRO.allows_llm == True

    def test_target_latencies_ordered(self):
        """L3 fastest, L4 slowest."""
        reflex_ms = ConsciousnessLevel.REFLEX.target_ms
        micro_ms  = ConsciousnessLevel.MICRO.target_ms
        macro_ms  = ConsciousnessLevel.MACRO.target_ms
        meta_ms   = ConsciousnessLevel.META.target_ms
        assert reflex_ms < micro_ms < macro_ms < meta_ms

    def test_reflex_target_under_10ms(self):
        """L3 must target sub-10ms."""
        assert ConsciousnessLevel.REFLEX.target_ms < 10.0

    def test_gradients_ordered(self):
        """Higher level = higher consciousness gradient."""
        assert (
            ConsciousnessLevel.REFLEX.gradient
            < ConsciousnessLevel.MICRO.gradient
            < ConsciousnessLevel.MACRO.gradient
            <= ConsciousnessLevel.META.gradient
        )

    def test_fibonacci_timing(self):
        """Timing constants are φ-aligned (Fibonacci-based)."""
        # L3: F(3)×3 = 2×3 = 6ms ≈ <10ms ✓
        reflex_ms = ConsciousnessLevel.REFLEX.target_ms
        assert reflex_ms == fibonacci(3) * 3.0


class TestDogSetsByLevel:
    """Each level has the right set of Dogs."""

    def test_reflex_dogs_non_llm_only(self):
        reflex = dogs_for_level(ConsciousnessLevel.REFLEX)
        assert "CYNIC" in reflex
        assert "GUARDIAN" in reflex
        assert "ANALYST" in reflex
        assert "JANITOR" in reflex
        assert "ARCHITECT" in reflex   # non-LLM AST analysis
        assert "ORACLE" in reflex      # non-LLM Thompson Sampling
        # LLM Dogs should NOT be in reflex
        assert "SAGE" not in reflex
        assert "SCHOLAR" not in reflex

    def test_micro_dogs_superset_of_reflex(self):
        reflex = dogs_for_level(ConsciousnessLevel.REFLEX)
        micro  = dogs_for_level(ConsciousnessLevel.MICRO)
        assert reflex.issubset(micro)

    def test_macro_has_all_11_dogs(self):
        macro = dogs_for_level(ConsciousnessLevel.MACRO)
        expected = {
            "CYNIC", "GUARDIAN", "ANALYST", "JANITOR",
            "SAGE", "SCHOLAR", "ORACLE", "ARCHITECT",
            "DEPLOYER", "SCOUT", "CARTOGRAPHER",
        }
        assert macro == expected

    def test_macro_count_is_11(self):
        from cynic.core.phi import lucas
        macro = dogs_for_level(ConsciousnessLevel.MACRO)
        assert len(macro) == lucas(5)  # L(5) = 11


class TestConsciousnessGradient:
    """7-level per-cell consciousness gradient."""

    def test_seven_levels(self):
        assert len(list(ConsciousnessGradient)) == 7

    def test_gradient_range(self):
        for g in ConsciousnessGradient:
            assert 0 <= g.value <= 6

    def test_min_budget_increases_with_gradient(self):
        budgets = [g.min_budget_usd for g in ConsciousnessGradient]
        # Higher gradient = higher minimum budget
        assert budgets[0] <= budgets[3] <= budgets[6]

    def test_gradient_from_budget_low(self):
        g = gradient_from_budget(0.001)  # very low budget
        assert g == 0  # REFLEX gradient

    def test_gradient_from_budget_high(self):
        g = gradient_from_budget(1.0)  # comfortable budget
        assert g > 0  # at least REACTIVE


class TestCycleTimer:
    """CycleTimer tracks latency vs φ-aligned target."""

    def test_timer_records_sample(self):
        timer = CycleTimer(level=ConsciousnessLevel.MACRO)
        timer.start()
        import time; time.sleep(0.001)
        elapsed = timer.stop()
        assert elapsed > 0
        assert len(timer._samples) == 1

    def test_health_excellent_when_fast(self):
        timer = CycleTimer(level=ConsciousnessLevel.MACRO)
        # Simulate very fast samples
        timer._samples = [1.0] * 10  # 1ms, well within budget
        assert timer.health == "EXCELLENT"

    def test_health_critical_when_slow(self):
        timer = CycleTimer(level=ConsciousnessLevel.REFLEX)
        # Simulate 1000ms for a <10ms target = 100× over budget = CRITICAL
        timer._samples = [1000.0] * 10
        assert timer.health == "CRITICAL"

    def test_p50_correct(self):
        timer = CycleTimer(level=ConsciousnessLevel.MACRO)
        timer._samples = [1.0, 2.0, 3.0, 4.0, 5.0]
        assert timer.p50_ms == 3.0


class TestConsciousnessState:
    """Global consciousness state machine."""

    def test_singleton_returns_same_instance(self):
        a = get_consciousness()
        b = get_consciousness()
        assert a is b

    def test_increment_counters(self):
        state = ConsciousnessState()
        state.increment(ConsciousnessLevel.MACRO)
        state.increment(ConsciousnessLevel.MACRO)
        state.increment(ConsciousnessLevel.REFLEX)
        assert state.macro_cycles == 2
        assert state.reflex_cycles == 1
        assert state.total_cycles == 3

    def test_downgrade_on_low_budget(self):
        state = ConsciousnessState()
        # Very low budget → suggest REFLEX
        suggested = state.should_downgrade(0.001)
        assert suggested == ConsciousnessLevel.REFLEX

    def test_no_downgrade_on_good_budget(self):
        state = ConsciousnessState()
        # Healthy budget → no downgrade
        suggested = state.should_downgrade(5.0)
        assert suggested is None

    def test_to_dict_includes_all_keys(self):
        state = ConsciousnessState()
        d = state.to_dict()
        assert "active_level" in d
        assert "gradient" in d
        assert "cycles" in d
        assert "timers" in d
        assert "REFLEX" in d["cycles"]
        assert "MACRO" in d["cycles"]
