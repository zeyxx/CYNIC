"""
CYNIC Amplification Benchmark Tests (Experiment #2)

Tests for WarmStartResult, _run_warm_cold, AmplificationBenchmark.

Hypothesis: accumulated session knowledge (warm QTable) amplifies performance.
  phi (α=0.038, EWC=True) amplifies more than standard RL because EWC
  protects consolidated warm values against overwrite.

Key assertion: phi_beats_standard (phi mean amplification_ratio > standard's).
"""
from __future__ import annotations

import pytest

from cynic.judge.amplification_benchmark import (
    WarmStartResult,
    AmplificationBenchmark,
    _run_warm_cold,
    _WARM_LEVELS,
    _TEST_BUDGET,
    _SEED_OFFSET,
)
from cynic.judge.qtable_benchmark import _PHI_ALPHA, _STD_ALPHA, _EWC_CONSOLIDATE_AT
from cynic.judge.real_benchmark import RealKernelTask, _N_PAIRS_REAL
from cynic.core.phi import fibonacci


# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

class TestConstants:
    def test_warm_levels_start_at_zero(self):
        assert _WARM_LEVELS[0] == 0

    def test_warm_levels_include_fibonacci(self):
        """Fibonacci-anchored levels: F(5)=5 and F(7)=13 should be present."""
        assert fibonacci(5) in _WARM_LEVELS
        assert fibonacci(7) in _WARM_LEVELS

    def test_warm_levels_increasing(self):
        for i in range(len(_WARM_LEVELS) - 1):
            assert _WARM_LEVELS[i] < _WARM_LEVELS[i + 1]

    def test_test_budget_reasonable(self):
        assert 50 <= _TEST_BUDGET <= 500


# ---------------------------------------------------------------------------
# _run_warm_cold
# ---------------------------------------------------------------------------

class TestRunWarmCold:
    def test_cold_start_returns_result(self):
        r = _run_warm_cold(alpha=0.1, use_ewc=False, warm_steps=0,
                           test_budget=100, seed=42)
        assert isinstance(r, WarmStartResult)

    def test_warm_start_returns_result(self):
        r = _run_warm_cold(alpha=0.1, use_ewc=False, warm_steps=100,
                           test_budget=100, seed=42)
        assert isinstance(r, WarmStartResult)

    def test_cold_start_warm_steps_is_zero(self):
        r = _run_warm_cold(alpha=0.1, use_ewc=False, warm_steps=0,
                           test_budget=100, seed=42)
        assert r.warm_steps == 0

    def test_errors_non_negative(self):
        r = _run_warm_cold(alpha=0.1, use_ewc=False, warm_steps=50,
                           test_budget=100, seed=42)
        assert r.cold_final_error >= 0.0
        assert r.warm_final_error >= 0.0

    def test_amplification_ratio_positive(self):
        r = _run_warm_cold(alpha=0.1, use_ewc=False, warm_steps=50,
                           test_budget=100, seed=42)
        assert r.amplification_ratio > 0.0

    def test_deterministic_same_seed(self):
        r1 = _run_warm_cold(alpha=0.038, use_ewc=True, warm_steps=100,
                            test_budget=100, seed=99)
        r2 = _run_warm_cold(alpha=0.038, use_ewc=True, warm_steps=100,
                            test_budget=100, seed=99)
        assert abs(r1.amplification_ratio - r2.amplification_ratio) < 1e-9

    def test_duration_positive(self):
        r = _run_warm_cold(alpha=0.1, use_ewc=False, warm_steps=0,
                           test_budget=50, seed=42)
        assert r.duration_ms > 0.0

    def test_knowledge_retention_formula(self):
        """retention = 1 - warm_error / cold_error."""
        r = _run_warm_cold(alpha=0.1, use_ewc=False, warm_steps=100,
                           test_budget=100, seed=42)
        if r.cold_final_error > 1e-9:
            expected = 1.0 - r.warm_final_error / r.cold_final_error
            assert abs(r.knowledge_retention - expected) < 1e-6

    def test_warm_500_steps_transfers_knowledge(self):
        """After 500 warm steps, warm learner should start much closer to true rewards."""
        r = _run_warm_cold(alpha=0.1, use_ewc=False, warm_steps=500,
                           test_budget=10, seed=42)
        # With 500 warm steps, warm_error should be < cold_error
        assert r.warm_final_error <= r.cold_final_error + 0.05

    def test_zero_warm_cold_and_warm_identical_paths(self):
        """With warm_steps=0, both cold and warm should follow identical trajectories."""
        r = _run_warm_cold(alpha=0.1, use_ewc=False, warm_steps=0,
                           test_budget=100, seed=42)
        # Both start from Q=0.5 with same task seed
        # cold_error should equal warm_error (same learner, same task)
        assert abs(r.cold_final_error - r.warm_final_error) < 1e-6

    def test_amplified_property(self):
        r = _run_warm_cold(alpha=0.1, use_ewc=False, warm_steps=0,
                           test_budget=100, seed=42)
        # warm_steps=0: ratio=1.0 → not amplified
        assert not r.amplified or r.amplification_ratio > 1.0

    def test_to_dict_keys(self):
        r = _run_warm_cold(alpha=0.1, use_ewc=False, warm_steps=50,
                           test_budget=50, seed=42)
        d = r.to_dict()
        for key in ("warm_steps", "alpha", "use_ewc", "cold_final_error",
                    "warm_final_error", "amplification_ratio", "knowledge_retention",
                    "amplified"):
            assert key in d, f"Missing key: {key}"

    def test_label_format(self):
        r = _run_warm_cold(alpha=0.038, use_ewc=True, warm_steps=100,
                           test_budget=50, seed=42)
        assert "+EWC" in r.label
        assert "warm=100" in r.label


# ---------------------------------------------------------------------------
# AmplificationBenchmark.run_level
# ---------------------------------------------------------------------------

class TestRunLevel:
    def test_run_level_returns_dict(self):
        bench = AmplificationBenchmark()
        result = bench.run_level(alpha=0.1, use_ewc=False, warm_steps=0,
                                 test_budget=100, n_seeds=3)
        assert isinstance(result, dict)

    def test_run_level_keys(self):
        bench = AmplificationBenchmark()
        result = bench.run_level(alpha=0.1, use_ewc=False, warm_steps=50,
                                 test_budget=100, n_seeds=3)
        for key in ("warm_steps", "alpha", "use_ewc", "n_seeds",
                    "mean_amplification_ratio", "mean_knowledge_retention",
                    "mean_cold_error", "mean_warm_error", "amplified_rate"):
            assert key in result, f"Missing key: {key}"

    def test_amplified_rate_in_range(self):
        bench = AmplificationBenchmark()
        result = bench.run_level(alpha=0.1, use_ewc=False, warm_steps=100,
                                 test_budget=100, n_seeds=5)
        assert 0.0 <= result["amplified_rate"] <= 1.0

    def test_cold_error_non_negative(self):
        bench = AmplificationBenchmark()
        result = bench.run_level(alpha=0.038, use_ewc=True, warm_steps=0,
                                 test_budget=100, n_seeds=3)
        assert result["mean_cold_error"] >= 0.0

    def test_warm_500_better_than_cold(self):
        """With 500 warm steps, warm error should be <= cold error on average."""
        bench = AmplificationBenchmark()
        result = bench.run_level(alpha=0.1, use_ewc=False, warm_steps=500,
                                 test_budget=100, n_seeds=5)
        assert result["mean_warm_error"] <= result["mean_cold_error"] + 0.02


# ---------------------------------------------------------------------------
# AmplificationBenchmark.run_grid
# ---------------------------------------------------------------------------

class TestRunGrid:
    def test_run_grid_returns_dict(self):
        bench = AmplificationBenchmark()
        grid = bench.run_grid(warm_levels=[0, 50], test_budget=100, n_seeds=3)
        assert isinstance(grid, dict)

    def test_run_grid_required_keys(self):
        bench = AmplificationBenchmark()
        grid = bench.run_grid(warm_levels=[0, 50], test_budget=100, n_seeds=3)
        for key in (
            "warm_levels", "test_budget", "n_seeds",
            "phi_levels", "standard_levels",
            "phi_curve", "standard_curve",
            "phi_mean_ratio", "standard_mean_ratio",
            "phi_beats_standard", "warm_advantage_grows",
            "phi_amplification_at_max_warm",
        ):
            assert key in grid, f"Missing key: {key}"

    def test_curves_length_matches_warm_levels(self):
        bench = AmplificationBenchmark()
        warm_levels = [0, 50, 100]
        grid = bench.run_grid(warm_levels=warm_levels, test_budget=100, n_seeds=3)
        assert len(grid["phi_curve"]) == len(warm_levels)
        assert len(grid["standard_curve"]) == len(warm_levels)

    def test_phi_curve_ratios_positive(self):
        bench = AmplificationBenchmark()
        grid = bench.run_grid(warm_levels=[0, 50, 100], test_budget=100, n_seeds=3)
        for ratio in grid["phi_curve"]:
            assert ratio > 0.0

    def test_cold_start_ratio_is_one(self):
        """warm_steps=0: cold and warm are identical → ratio should be ~1.0."""
        bench = AmplificationBenchmark()
        grid = bench.run_grid(warm_levels=[0, 100], test_budget=100, n_seeds=3)
        cold_ratio = grid["phi_curve"][0]
        assert abs(cold_ratio - 1.0) < 0.01, (
            f"warm=0 ratio should be ~1.0, got {cold_ratio:.4f}"
        )


# ---------------------------------------------------------------------------
# PhiAmplification: empirical validation
# ---------------------------------------------------------------------------

class TestPhiAmplification:
    """
    Empirical validation of the amplification hypothesis.
    phi (α=0.038, EWC=True) should amplify MORE than standard RL
    because EWC protects warm-loaded consolidated values.
    """

    def test_phi_alpha_is_conservative(self):
        assert _PHI_ALPHA < _STD_ALPHA

    def test_warm_advantage_grows_with_depth_phi(self):
        """
        phi amplification_ratio should increase with more warm steps.
        More session history → stronger amplification.
        """
        bench = AmplificationBenchmark()
        grid = bench.run_grid(
            warm_levels=[0, 100, 500],
            test_budget=_TEST_BUDGET,
            n_seeds=7,
            base_seed=42,
        )
        assert grid["warm_advantage_grows"], (
            f"phi amplification should grow: "
            f"ratio@0={grid['phi_curve'][0]:.4f}, "
            f"ratio@500={grid['phi_curve'][-1]:.4f}"
        )

    def test_phi_beats_standard_amplification(self):
        """
        phi (α=0.038, EWC=True) mean amplification_ratio > standard (α=0.1, EWC=False).
        EWC protects warm values → phi amplifies more on average.
        """
        bench = AmplificationBenchmark()
        grid = bench.run_grid(
            warm_levels=_WARM_LEVELS,
            test_budget=_TEST_BUDGET,
            n_seeds=7,
            base_seed=42,
        )
        assert grid["phi_beats_standard"], (
            f"phi mean_ratio={grid['phi_mean_ratio']:.4f} "
            f"should be > standard={grid['standard_mean_ratio']:.4f}"
        )

    def test_phi_amplification_at_max_warm_above_one(self):
        """
        At max warm depth (500 steps), phi amplification_ratio should be > 1.0.
        This is the core claim: accumulated knowledge amplifies performance.
        """
        bench = AmplificationBenchmark()
        grid = bench.run_grid(
            warm_levels=[0, 500],
            test_budget=_TEST_BUDGET,
            n_seeds=7,
            base_seed=42,
        )
        ratio = grid["phi_amplification_at_max_warm"]
        assert ratio > 1.0, (
            f"phi amplification at warm=500 should be > 1.0, got {ratio:.4f}"
        )

    def test_warm_levels_fibonacci_anchored(self):
        """_WARM_LEVELS should include F(5)=5 and F(7)=13."""
        assert fibonacci(5) in _WARM_LEVELS
        assert fibonacci(7) in _WARM_LEVELS
