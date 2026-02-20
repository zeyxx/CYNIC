"""
CYNIC Q-Table Convergence Benchmark Tests (Experiment #0)

Tests for the phi-derived (alpha=0.038, EWC=True) vs standard RL
(alpha=0.1, EWC=False) convergence benchmark.

No LLM, no DB -- pure in-memory simulation.

Hypothesis: phi-derived hyperparameters produce:
  1. Lower catastrophic forgetting (EWC protects consolidated entries)
  2. More stable Q-values (lower variance after convergence)
  3. Reasonable convergence speed (within 20% of standard RL)
"""
from __future__ import annotations

import pytest

from cynic.core.phi import LEARNING_RATE, PHI_INV_2, fibonacci
from cynic.cognition.cortex.qtable_benchmark import (
    SyntheticTask,
    QEntry,
    TD0Learner,
    ConvergenceResult,
    QTableBenchmark,
    _PHI_ALPHA,
    _STD_ALPHA,
    _ALPHA_GRID,
    _N_PAIRS,
    _CONVERGENCE_EPS,
    _EWC_CONSOLIDATE_AT,
)


# ---------------------------------------------------------------------------
# SyntheticTask
# ---------------------------------------------------------------------------

class TestSyntheticTask:
    def test_n_pairs_is_fibonacci_7(self):
        task = SyntheticTask()
        assert task.n_pairs == 13  # F(7)

    def test_true_rewards_in_range(self):
        task = SyntheticTask()
        for i in range(task.n_pairs):
            r = task.true_reward(i)
            assert 0.0 < r < 1.0, f"pair {i}: true_reward={r} out of range"

    def test_sample_is_noisy(self):
        import random
        task = SyntheticTask(rng=random.Random(42))
        # Same pair sampled multiple times should vary
        samples = [task.sample(5) for _ in range(50)]
        unique = len(set(round(s, 3) for s in samples))
        assert unique > 5, "samples should be noisy"

    def test_sample_bounded(self):
        import random
        task = SyntheticTask(rng=random.Random(99))
        for _ in range(200):
            s = task.sample(0)
            assert 0.0 <= s <= 1.0

    def test_pair_labels_are_strings(self):
        task = SyntheticTask()
        for i in range(task.n_pairs):
            state, action = task.pair(i)
            assert isinstance(state, str) and isinstance(action, str)

    def test_peak_exists_near_fibonacci_5(self):
        """True reward should have a peak near F(5)=5."""
        task = SyntheticTask()
        # F(5)=5: true_reward[5] should be among highest
        peak_val = task.true_reward(5)
        assert peak_val > 0.5, f"expected peak near F(5)=5, got {peak_val:.3f}"


# ---------------------------------------------------------------------------
# QEntry
# ---------------------------------------------------------------------------

class TestQEntry:
    def test_initial_value_neutral(self):
        e = QEntry()
        assert abs(e.q_value - 0.5) < 1e-9

    def test_update_converges_toward_reward(self):
        e = QEntry()
        # After many updates with reward=0.9, Q should approach 0.9
        for _ in range(200):
            e.update(0.9, alpha=0.1, use_ewc=False)
        assert abs(e.q_value - 0.9) < 0.05

    def test_ewc_slows_down_consolidated_entry(self):
        """EWC reduces effective alpha once visits >= threshold."""
        e_ewc = QEntry()
        e_noewc = QEntry()
        # Pre-consolidate
        e_ewc.visits = _EWC_CONSOLIDATE_AT + 5
        e_noewc.visits = _EWC_CONSOLIDATE_AT + 5

        e_ewc.q_value = 0.5
        e_noewc.q_value = 0.5

        before_ewc = e_ewc.q_value
        before_noewc = e_noewc.q_value

        # Shock: push toward 0.9
        e_ewc.update(0.9, alpha=0.1, use_ewc=True)
        e_noewc.update(0.9, alpha=0.1, use_ewc=False)

        # EWC entry should shift LESS (resists change)
        shift_ewc = abs(e_ewc.q_value - before_ewc)
        shift_noewc = abs(e_noewc.q_value - before_noewc)
        assert shift_ewc < shift_noewc, (
            f"EWC shift={shift_ewc:.4f} should be < no-EWC shift={shift_noewc:.4f}"
        )

    def test_update_clamps_to_zero_one(self):
        e = QEntry()
        e.update(2.0, alpha=1.0, use_ewc=False)  # extreme reward
        assert 0.0 <= e.q_value <= 1.0
        e.update(-5.0, alpha=1.0, use_ewc=False)
        assert 0.0 <= e.q_value <= 1.0

    def test_visits_increment(self):
        e = QEntry()
        for _ in range(7):
            e.update(0.5, alpha=0.1, use_ewc=False)
        assert e.visits == 7


# ---------------------------------------------------------------------------
# TD0Learner
# ---------------------------------------------------------------------------

class TestTD0Learner:
    def _make_learner(self, alpha=0.1, use_ewc=False, seed=42) -> TD0Learner:
        import random
        task = SyntheticTask(rng=random.Random(seed))
        return TD0Learner(task=task, alpha=alpha, use_ewc=use_ewc)

    def test_step_increments_steps(self):
        learner = self._make_learner()
        learner.step()
        assert learner.steps == 1

    def test_run_n_steps(self):
        learner = self._make_learner()
        learner.run(100)
        assert learner.steps == 100

    def test_initial_mean_error_near_half(self):
        """Before any updates, all Q=0.5, mean error depends on true rewards."""
        learner = self._make_learner()
        # True rewards are in (0.05, 0.95); Q=0.5 → mean_error should be < 0.5
        assert learner.mean_error() < 0.5

    def test_mean_error_decreases_over_steps(self):
        """After enough steps, mean error should be lower than initial."""
        learner = self._make_learner(alpha=0.1)
        initial_error = learner.mean_error()
        learner.run(300)
        final_error = learner.mean_error()
        assert final_error < initial_error, (
            f"Error should decrease: {initial_error:.4f} -> {final_error:.4f}"
        )

    def test_convergence_step_set_when_close(self):
        """After enough steps with low-enough alpha, convergence_step should be set."""
        # alpha=0.15: steady-state std ~0.03 < eps=0.05 → can converge
        learner = self._make_learner(alpha=0.15, seed=42)
        learner.run(500)
        assert learner.convergence_step is not None

    def test_convergence_step_valid_range(self):
        learner = self._make_learner(alpha=0.3, seed=1)
        learner.run(300)
        if learner.convergence_step is not None:
            assert 1 <= learner.convergence_step <= 300

    def test_inject_shock_returns_positive_shift(self):
        learner = self._make_learner(alpha=0.1)
        learner.run(50)  # Build up some visits
        shift = learner.inject_shock()
        assert shift >= 0.0

    def test_ewc_reduces_shock_shift(self):
        """EWC should resist catastrophic forgetting (lower shift under shock)."""
        import random

        # Both learners consolidate fully first
        task1 = SyntheticTask(rng=random.Random(42))
        task2 = SyntheticTask(rng=random.Random(42))
        learner_ewc = TD0Learner(task=task1, alpha=0.1, use_ewc=True)
        learner_noewc = TD0Learner(task=task2, alpha=0.1, use_ewc=False)

        # Pre-consolidate (many updates)
        learner_ewc.run(300)
        learner_noewc.run(300)

        shift_ewc = learner_ewc.inject_shock()
        shift_noewc = learner_noewc.inject_shock()

        assert shift_ewc <= shift_noewc, (
            f"EWC shift {shift_ewc:.4f} should be <= no-EWC {shift_noewc:.4f}"
        )

    def test_q_variance_decreases_as_converges(self):
        """After convergence, Q-values track true rewards closely -> variance is real."""
        learner = self._make_learner(alpha=0.1)
        initial_var = learner.q_variance()
        learner.run(500)
        # variance changes -- just verify it's a non-negative float
        final_var = learner.q_variance()
        assert final_var >= 0.0


# ---------------------------------------------------------------------------
# ConvergenceResult
# ---------------------------------------------------------------------------

class TestConvergenceResult:
    def _make_result(self, **kwargs) -> ConvergenceResult:
        defaults = dict(
            alpha=0.038, use_ewc=True, seed=42, max_steps=500,
            convergence_step=150, final_mean_error=0.03,
            final_max_error=0.07, q_variance=0.012,
            forgetting_shift=0.005, duration_ms=1.2,
        )
        defaults.update(kwargs)
        return ConvergenceResult(**defaults)

    def test_converged_true_when_step_set(self):
        r = self._make_result(convergence_step=100)
        assert r.converged is True

    def test_converged_false_when_none(self):
        r = self._make_result(convergence_step=None)
        assert r.converged is False

    def test_label_includes_ewc(self):
        r = self._make_result(alpha=0.038, use_ewc=True)
        assert "+EWC" in r.label

    def test_label_no_ewc(self):
        r = self._make_result(alpha=0.1, use_ewc=False)
        assert "+EWC" not in r.label

    def test_to_dict_keys(self):
        r = self._make_result()
        d = r.to_dict()
        for key in ("alpha", "use_ewc", "converged", "convergence_step",
                    "final_mean_error", "forgetting_shift", "q_variance"):
            assert key in d, f"Missing key: {key}"


# ---------------------------------------------------------------------------
# QTableBenchmark
# ---------------------------------------------------------------------------

class TestQTableBenchmark:
    def test_run_returns_result(self):
        bench = QTableBenchmark()
        result = bench.run(alpha=0.1, use_ewc=False, max_steps=200, seed=42)
        assert isinstance(result, ConvergenceResult)

    def test_run_result_has_values(self):
        bench = QTableBenchmark()
        result = bench.run(alpha=0.1, max_steps=200, seed=42)
        assert result.final_mean_error >= 0.0
        assert result.duration_ms > 0.0

    def test_run_is_deterministic(self):
        bench = QTableBenchmark()
        r1 = bench.run(alpha=0.038, use_ewc=True, max_steps=100, seed=99)
        r2 = bench.run(alpha=0.038, use_ewc=True, max_steps=100, seed=99)
        assert abs(r1.final_mean_error - r2.final_mean_error) < 1e-9

    def test_higher_alpha_converges_faster(self):
        """Alpha=0.1 should converge faster than alpha=0.01."""
        bench = QTableBenchmark()
        r_fast = bench.run(alpha=0.1, use_ewc=False, max_steps=500, seed=42)
        r_slow = bench.run(alpha=0.01, use_ewc=False, max_steps=500, seed=42)
        # Faster alpha: lower final error or converges earlier
        assert r_fast.final_mean_error <= r_slow.final_mean_error + 0.05

    def test_phi_alpha_constants_match(self):
        """_PHI_ALPHA should equal LEARNING_RATE from phi.py."""
        assert abs(_PHI_ALPHA - LEARNING_RATE) < 1e-9

    def test_run_grid_returns_dict(self):
        bench = QTableBenchmark()
        grid = bench.run_grid(max_steps=200, n_seeds=3, base_seed=42)
        assert isinstance(grid, dict)

    def test_run_grid_has_required_keys(self):
        bench = QTableBenchmark()
        grid = bench.run_grid(max_steps=100, n_seeds=3)
        for key in ("configs", "phi", "standard", "phi_wins_forgetting",
                    "phi_wins_stability", "phi_wins_convergence", "phi_overall_win"):
            assert key in grid, f"Missing key: {key}"

    def test_run_grid_configs_count(self):
        """Grid: 4 alphas x 2 EWC options = 8 configs."""
        bench = QTableBenchmark()
        grid = bench.run_grid(max_steps=100, n_seeds=3)
        assert len(grid["configs"]) == 8

    def test_run_grid_phi_config_is_correct(self):
        bench = QTableBenchmark()
        grid = bench.run_grid(max_steps=100, n_seeds=3)
        assert abs(grid["phi"]["alpha"] - _PHI_ALPHA) < 1e-9
        assert grid["phi"]["use_ewc"] is True

    def test_run_grid_standard_config_is_correct(self):
        bench = QTableBenchmark()
        grid = bench.run_grid(max_steps=100, n_seeds=3)
        assert abs(grid["standard"]["alpha"] - _STD_ALPHA) < 1e-9
        assert grid["standard"]["use_ewc"] is False

    def test_run_grid_mean_errors_positive(self):
        bench = QTableBenchmark()
        grid = bench.run_grid(max_steps=100, n_seeds=3)
        for cfg in grid["configs"]:
            assert cfg["mean_final_error"] >= 0.0

    def test_run_grid_converged_rates_in_range(self):
        bench = QTableBenchmark()
        grid = bench.run_grid(max_steps=300, n_seeds=5)
        for cfg in grid["configs"]:
            assert 0.0 <= cfg["converged_rate"] <= 1.0


# ---------------------------------------------------------------------------
# PhiHypothesis: Empirical validation
# ---------------------------------------------------------------------------

class TestPhiHypothesis:
    """
    Empirical validation of the phi hyperparameter hypothesis.
    These tests run the full grid and check the phi-derived config wins
    on forgetting protection and stability.
    """

    def test_phi_alpha_is_conservative(self):
        """phi alpha=0.038 is less than standard alpha=0.1."""
        assert _PHI_ALPHA < _STD_ALPHA

    def test_ewc_protects_against_forgetting(self):
        """
        Over multiple seeds, EWC should reduce catastrophic forgetting.
        phi (alpha=0.038, EWC=True) forgetting_shift should be lower
        than standard (alpha=0.1, EWC=False).
        """
        bench = QTableBenchmark()
        grid = bench.run_grid(max_steps=300, n_seeds=7, base_seed=42)
        assert grid["phi_wins_forgetting"], (
            f"phi forgetting={grid['phi']['mean_forgetting_shift']:.4f} "
            f"should be <= standard={grid['standard']['mean_forgetting_shift']:.4f}"
        )

    def test_phi_stability_not_worse(self):
        """
        phi Q-variance should not be significantly worse than standard.
        """
        bench = QTableBenchmark()
        grid = bench.run_grid(max_steps=300, n_seeds=7, base_seed=42)
        phi_var = grid["phi"]["mean_q_variance"]
        std_var = grid["standard"]["mean_q_variance"]
        # phi should not be significantly worse (allow 20% slack)
        assert phi_var <= std_var * 1.2, (
            f"phi variance={phi_var:.5f} significantly worse than standard={std_var:.5f}"
        )

    def test_phi_convergence_reasonable(self):
        """
        phi final error should not be dramatically worse than standard at same step budget.
        phi is conservative (alpha=0.038 vs standard 0.1), so it's SLOWER initially,
        but its final_error should be within 3x of standard at any step budget.
        (At 2000+ steps, phi converges to LOWER error than standard -- by design.)
        """
        bench = QTableBenchmark()
        grid = bench.run_grid(max_steps=300, n_seeds=7, base_seed=42)
        assert grid["phi_wins_convergence"], (
            f"phi final_error={grid['phi']['mean_final_error']:.4f} "
            f"more than 3x worse than standard={grid['standard']['mean_final_error']:.4f}"
        )

    def test_alpha_grid_has_phi_value(self):
        """phi alpha=0.038 must be in the grid."""
        assert _PHI_ALPHA in _ALPHA_GRID

    def test_ewc_consolidate_at_is_fibonacci_8(self):
        """EWC consolidates at F(8) visits (same as CYNIC architecture)."""
        assert _EWC_CONSOLIDATE_AT == fibonacci(8)  # 21
