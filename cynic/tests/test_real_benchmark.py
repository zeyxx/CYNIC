"""
CYNIC Real Kernel Benchmark Tests (Experiment #1)

Tests for RealKernelTask (5 empirical probes) + RealBenchmark grid.

Hypothesis: phi-derived hyperparameters (alpha=0.038, EWC=True) converge to
real CYNIC probe Q-values AND resist catastrophic forgetting better than
standard RL (alpha=0.1, EWC=False) on real probe data.

Key difference from Experiment #0:
  - Real data: n=5 probes, sigma=0.012 (near-deterministic)
  - Bimodal landscape: {0.0, ~0.64-0.68}
  - P3 (dangerous_act) = 0.00 → strongest EWC anchor
"""
from __future__ import annotations

import pytest

from cynic.judge.real_benchmark import (
    RealKernelTask,
    RealBenchmark,
    _EMPIRICAL_TRUE_REWARDS,
    _EMPIRICAL_Q_SCORES,
    _SIGMA_REAL,
    _N_PAIRS_REAL,
    _Q_SCALE,
)
from cynic.judge.qtable_benchmark import (
    TD0Learner,
    QEntry,
    ConvergenceResult,
    _PHI_ALPHA,
    _STD_ALPHA,
    _CONVERGENCE_EPS,
    _EWC_CONSOLIDATE_AT,
)
from cynic.core.phi import fibonacci


# ---------------------------------------------------------------------------
# RealKernelTask
# ---------------------------------------------------------------------------

class TestRealKernelTask:
    def test_n_pairs_is_five(self):
        task = RealKernelTask()
        assert task.n_pairs == 5

    def test_n_pairs_is_fibonacci_5(self):
        """n=5 = F(5) — φ-architecture: 5 canonical probes."""
        assert _N_PAIRS_REAL == fibonacci(5)

    def test_true_rewards_count(self):
        task = RealKernelTask()
        assert len(_EMPIRICAL_TRUE_REWARDS) == 5

    def test_true_rewards_normalized(self):
        """All true_rewards are in [0, 1]."""
        task = RealKernelTask()
        for i in range(task.n_pairs):
            r = task.true_reward(i)
            assert 0.0 <= r <= 1.0, f"pair {i}: true_reward={r} out of [0,1]"

    def test_p3_dangerous_is_zero(self):
        """P3 (dangerous_act) must be exactly 0.0 — Guardian hard-block."""
        task = RealKernelTask()
        assert task.true_reward(task.dangerous_probe_idx) == 0.0

    def test_dangerous_probe_idx_is_two(self):
        task = RealKernelTask()
        assert task.dangerous_probe_idx == 2

    def test_p1_p2_p4_p5_above_phi(self):
        """Non-dangerous probes are above phi-threshold (0.618)."""
        task = RealKernelTask()
        phi_threshold = 0.618
        for i in range(task.n_pairs):
            if i == task.dangerous_probe_idx:
                continue
            r = task.true_reward(i)
            assert r >= phi_threshold, (
                f"pair {i}: true_reward={r:.4f} should be >= phi ({phi_threshold})"
            )

    def test_empirical_q_scores_count(self):
        assert len(_EMPIRICAL_Q_SCORES) == 5

    def test_empirical_q_scores_p3_is_zero(self):
        assert _EMPIRICAL_Q_SCORES[2] == 0.0

    def test_normalization_consistent(self):
        """true_rewards = empirical_q_scores / 100."""
        for q, r in zip(_EMPIRICAL_Q_SCORES, _EMPIRICAL_TRUE_REWARDS):
            assert abs(r - q / _Q_SCALE) < 1e-9

    def test_sigma_near_deterministic(self):
        """Real sigma is near-deterministic: sigma < 0.02."""
        assert _SIGMA_REAL < 0.02

    def test_sample_near_true_reward(self):
        """With tiny sigma, samples are very close to true_reward."""
        import random
        task = RealKernelTask(rng=random.Random(42))
        for i in range(task.n_pairs):
            if i == task.dangerous_probe_idx:
                continue  # P3 at 0 always stays 0 (clamped)
            samples = [task.sample(i) for _ in range(50)]
            mean = sum(samples) / len(samples)
            assert abs(mean - task.true_reward(i)) < 0.05, (
                f"pair {i}: sample mean={mean:.4f} far from true={task.true_reward(i):.4f}"
            )

    def test_sample_p3_always_near_zero(self):
        """P3 (dangerous_act) samples should always be near 0.0."""
        import random
        task = RealKernelTask(rng=random.Random(99))
        for _ in range(50):
            s = task.sample(task.dangerous_probe_idx)
            assert abs(s) < 0.05, f"P3 sample={s:.4f} should be near 0.0"

    def test_sample_bounded(self):
        import random
        task = RealKernelTask(rng=random.Random(7))
        for _ in range(200):
            for i in range(task.n_pairs):
                s = task.sample(i)
                assert 0.0 <= s <= 1.0

    def test_pair_labels_are_strings(self):
        task = RealKernelTask()
        for i in range(task.n_pairs):
            state, action = task.pair(i)
            assert isinstance(state, str) and isinstance(action, str)

    def test_pair_p3_is_dangerous_act(self):
        task = RealKernelTask()
        state, action = task.pair(task.dangerous_probe_idx)
        assert "dangerous" in action.lower() or "P3" in state

    def test_bimodal_landscape(self):
        """Landscape is bimodal: one at 0.0, others clustered near 0.65."""
        task = RealKernelTask()
        rewards = [task.true_reward(i) for i in range(task.n_pairs)]
        low_count = sum(1 for r in rewards if r < 0.1)
        high_count = sum(1 for r in rewards if r > 0.6)
        assert low_count == 1, f"Expected 1 near-zero probe, got {low_count}"
        assert high_count == 4, f"Expected 4 high probes, got {high_count}"


# ---------------------------------------------------------------------------
# TD0Learner with RealKernelTask (interface compatibility)
# ---------------------------------------------------------------------------

class TestTD0LearnerReal:
    def _make_learner(self, alpha=0.1, use_ewc=False, seed=42) -> TD0Learner:
        import random
        task = RealKernelTask(rng=random.Random(seed))
        return TD0Learner(task=task, alpha=alpha, use_ewc=use_ewc)

    def test_learner_accepts_real_task(self):
        learner = self._make_learner()
        learner.run(10)
        assert learner.steps == 10

    def test_mean_error_decreases_on_real_data(self):
        """With near-zero noise, error should decrease quickly."""
        learner = self._make_learner(alpha=0.1)
        initial_error = learner.mean_error()
        learner.run(200)
        assert learner.mean_error() < initial_error

    def test_convergence_fast_with_real_data(self):
        """Near-deterministic data → convergence should happen quickly."""
        learner = self._make_learner(alpha=0.1, seed=42)
        learner.run(300)
        assert learner.convergence_step is not None, (
            "Should converge on near-deterministic real data with alpha=0.1"
        )

    def test_p3_converges_to_zero(self):
        """P3 (dangerous_act, true_reward=0.0) should converge to ~0."""
        learner = self._make_learner(alpha=0.1)
        learner.run(300)
        p3_q = learner._entries[2].q_value
        assert p3_q < 0.1, f"P3 Q-value={p3_q:.4f} should converge near 0.0"

    def test_high_probes_converge_above_phi(self):
        """P1,P2,P4,P5 should converge above 0.60 (near their true ~0.64-0.68)."""
        learner = self._make_learner(alpha=0.1)
        learner.run(400)
        task = learner.task
        for i in range(task.n_pairs):
            if i == task.dangerous_probe_idx:
                continue
            q = learner._entries[i].q_value
            assert q > 0.55, f"pair {i}: Q={q:.4f} should be above 0.55"

    def test_inject_shock_p3_shift(self):
        """After consolidation, shock on P3 (0→1) should show EWC resistance."""
        import random
        task1 = RealKernelTask(rng=random.Random(42))
        task2 = RealKernelTask(rng=random.Random(42))
        ewc = TD0Learner(task=task1, alpha=0.1, use_ewc=True)
        noewc = TD0Learner(task=task2, alpha=0.1, use_ewc=False)

        ewc.run(300)
        noewc.run(300)

        # P3 is fully consolidated (visits >> EWC_CONSOLIDATE_AT)
        assert ewc._entries[2].visits >= _EWC_CONSOLIDATE_AT

        shift_ewc = ewc.inject_shock()
        shift_noewc = noewc.inject_shock()
        assert shift_ewc <= shift_noewc, (
            f"EWC shift={shift_ewc:.4f} should be <= no-EWC shift={shift_noewc:.4f}"
        )


# ---------------------------------------------------------------------------
# RealBenchmark
# ---------------------------------------------------------------------------

class TestRealBenchmark:
    def test_run_returns_result(self):
        bench = RealBenchmark()
        result = bench.run(alpha=0.1, use_ewc=False, max_steps=200, seed=42)
        assert isinstance(result, ConvergenceResult)

    def test_run_result_has_values(self):
        bench = RealBenchmark()
        result = bench.run(alpha=0.1, max_steps=200, seed=42)
        assert result.final_mean_error >= 0.0
        assert result.duration_ms > 0.0

    def test_run_is_deterministic(self):
        bench = RealBenchmark()
        r1 = bench.run(alpha=0.038, use_ewc=True, max_steps=100, seed=99)
        r2 = bench.run(alpha=0.038, use_ewc=True, max_steps=100, seed=99)
        assert abs(r1.final_mean_error - r2.final_mean_error) < 1e-9

    def test_run_grid_returns_dict(self):
        bench = RealBenchmark()
        grid = bench.run_grid(max_steps=200, n_seeds=3, base_seed=42)
        assert isinstance(grid, dict)

    def test_run_grid_required_keys(self):
        bench = RealBenchmark()
        grid = bench.run_grid(max_steps=100, n_seeds=3)
        for key in (
            "configs", "phi", "standard",
            "phi_wins_forgetting", "phi_wins_stability", "phi_wins_convergence",
            "phi_overall_win",
            "dangerous_probe_shift_phi", "dangerous_probe_shift_standard",
            "phi_protects_dangerous",
        ):
            assert key in grid, f"Missing key: {key}"

    def test_run_grid_configs_count(self):
        """Grid: 4 alphas × 2 EWC = 8 configs."""
        bench = RealBenchmark()
        grid = bench.run_grid(max_steps=100, n_seeds=3)
        assert len(grid["configs"]) == 8

    def test_run_grid_mean_errors_non_negative(self):
        bench = RealBenchmark()
        grid = bench.run_grid(max_steps=100, n_seeds=3)
        for cfg in grid["configs"]:
            assert cfg["mean_final_error"] >= 0.0

    def test_run_grid_converged_rates_in_range(self):
        bench = RealBenchmark()
        grid = bench.run_grid(max_steps=300, n_seeds=5)
        for cfg in grid["configs"]:
            assert 0.0 <= cfg["converged_rate"] <= 1.0

    def test_dangerous_probe_shifts_non_negative(self):
        bench = RealBenchmark()
        grid = bench.run_grid(max_steps=200, n_seeds=3)
        assert grid["dangerous_probe_shift_phi"] >= 0.0
        assert grid["dangerous_probe_shift_standard"] >= 0.0

    def test_measure_p3_shift_returns_float(self):
        bench = RealBenchmark()
        shift = bench._measure_p3_shift(
            alpha=0.1, use_ewc=False, max_steps=200, seed=42
        )
        assert isinstance(shift, float)
        assert shift >= 0.0

    def test_measure_p3_shift_ewc_vs_noewc(self):
        """EWC reduces P3 shift (resists full reversal 0→1)."""
        bench = RealBenchmark()
        shift_ewc = bench._measure_p3_shift(
            alpha=0.1, use_ewc=True, max_steps=300, seed=42
        )
        shift_noewc = bench._measure_p3_shift(
            alpha=0.1, use_ewc=False, max_steps=300, seed=42
        )
        assert shift_ewc <= shift_noewc, (
            f"EWC P3 shift={shift_ewc:.4f} should be <= no-EWC={shift_noewc:.4f}"
        )


# ---------------------------------------------------------------------------
# PhiHypothesis on real data — empirical validation
# ---------------------------------------------------------------------------

class TestPhiHypothesisReal:
    """
    Empirical validation of phi hyperparameters on REAL CYNIC probe data.
    """

    def test_phi_alpha_is_conservative(self):
        assert _PHI_ALPHA < _STD_ALPHA

    def test_real_task_is_near_deterministic(self):
        """Real sigma << 0.05 — near-deterministic heuristic mode."""
        assert _SIGMA_REAL < 0.05

    def test_ewc_protects_against_forgetting_on_real_data(self):
        """
        phi (alpha=0.038, EWC=True) forgetting_shift <= standard (alpha=0.1, EWC=False)
        on real CYNIC probe data.
        """
        bench = RealBenchmark()
        grid = bench.run_grid(max_steps=300, n_seeds=7, base_seed=42)
        assert grid["phi_wins_forgetting"], (
            f"phi forgetting={grid['phi']['mean_forgetting_shift']:.4f} "
            f"should be <= standard={grid['standard']['mean_forgetting_shift']:.4f}"
        )

    def test_phi_protects_dangerous_probe(self):
        """
        phi EWC specifically resists the P3 (dangerous_act) full reversal shock.
        This is the most critical test: the kernel should "know" P3 is always 0.
        """
        bench = RealBenchmark()
        grid = bench.run_grid(max_steps=300, n_seeds=7, base_seed=42)
        assert grid["phi_protects_dangerous"], (
            f"phi P3 shift={grid['dangerous_probe_shift_phi']:.4f} "
            f"should be <= standard={grid['dangerous_probe_shift_standard']:.4f}"
        )

    def test_phi_convergence_reasonable_on_real_data(self):
        """
        phi final_error on real data should be within 3× of standard.
        (phi is conservative — designed for stability, not raw speed)
        """
        bench = RealBenchmark()
        grid = bench.run_grid(max_steps=300, n_seeds=7, base_seed=42)
        assert grid["phi_wins_convergence"], (
            f"phi final_error={grid['phi']['mean_final_error']:.4f} "
            f"more than 3× worse than standard={grid['standard']['mean_final_error']:.4f}"
        )

    def test_real_vs_synthetic_n_pairs(self):
        """Real task has 5 pairs (F(5)), synthetic had 13 (F(7))."""
        from cynic.judge.qtable_benchmark import _N_PAIRS
        assert _N_PAIRS_REAL < _N_PAIRS
        assert _N_PAIRS_REAL == fibonacci(5)

    def test_real_sigma_smaller_than_synthetic(self):
        """Real sigma (0.012) is much smaller than synthetic (0.1)."""
        from cynic.judge.qtable_benchmark import _SIGMA
        assert _SIGMA_REAL < _SIGMA
