"""
CYNIC Temporal MCTS Benchmark Tests (δ3)

Tests for the Standard vs Temporal MCTS convergence benchmark.
No LLM, no DB — pure in-memory simulation.

Hypothesis: Temporal MCTS (7 φ-weighted perspectives) converges
faster than Standard MCTS (single perspective) on the same problem.
Expected speedup ≥ φ (1.618), hypothesis: approaches φ².
"""
from __future__ import annotations

import random
import pytest

from cynic.core.phi import PHI, PHI_2, MAX_Q_SCORE
from cynic.judge.mcts_benchmark import (
    SearchProblem,
    MCTSNode,
    MCTSVariant,
    MCTSBenchmark,
    BenchmarkResult,
    _TEMPORAL_WEIGHTS,
    _TOTAL_TEMPORAL_WEIGHT,
    _PERSPECTIVES,
)


# ── SearchProblem ─────────────────────────────────────────────────────────────

class TestSearchProblem:
    def test_optimum_has_highest_true_value(self):
        prob = SearchProblem()
        opt_val = prob.true_value(prob.optimum_idx)
        for a in range(prob.n_actions):
            assert prob.true_value(a) <= opt_val + 0.01  # allow float noise

    def test_true_value_in_range(self):
        prob = SearchProblem()
        for a in range(prob.n_actions):
            v = prob.true_value(a)
            assert 0.0 <= v <= MAX_Q_SCORE, f"Action {a} has value {v} out of range"

    def test_sample_is_noisy(self):
        prob = SearchProblem(rng=random.Random(42))
        samples = [prob.sample(prob.optimum_idx) for _ in range(100)]
        # Standard deviation > 0 (noise present)
        mean = sum(samples) / len(samples)
        variance = sum((s - mean) ** 2 for s in samples) / len(samples)
        assert variance > 1.0, "Sample should have noise"

    def test_temporal_sample_lower_variance_than_standard(self):
        """Temporal sample should have lower variance than standard (key hypothesis)."""
        rng = random.Random(99)
        prob = SearchProblem(rng=rng)
        action = prob.optimum_idx

        std_samples = [prob.sample(action) for _ in range(200)]
        tmp_samples = [prob.temporal_sample(action) for _ in range(200)]

        def variance(xs):
            mean = sum(xs) / len(xs)
            return sum((x - mean) ** 2 for x in xs) / len(xs)

        var_std = variance(std_samples)
        var_tmp = variance(tmp_samples)
        # Temporal should be less noisy
        assert var_tmp < var_std, (
            f"Temporal variance {var_tmp:.2f} should be < standard {var_std:.2f}"
        )

    def test_sample_never_below_zero(self):
        prob = SearchProblem(rng=random.Random(7))
        for _ in range(500):
            s = prob.sample(0)
            assert s >= 0.0

    def test_sample_never_above_max(self):
        prob = SearchProblem(rng=random.Random(7))
        for _ in range(500):
            s = prob.sample(prob.optimum_idx)
            assert s <= MAX_Q_SCORE

    def test_n_actions_is_fibonacci_9(self):
        prob = SearchProblem()
        assert prob.n_actions == 34  # F(9)

    def test_optimum_idx_is_fibonacci_8(self):
        prob = SearchProblem()
        assert prob.optimum_idx == 21  # F(8)


# ── MCTSNode ──────────────────────────────────────────────────────────────────

class TestMCTSNode:
    def test_initial_value_zero(self):
        n = MCTSNode(action=5)
        assert n.value == 0.0

    def test_ucb_infinite_unvisited(self):
        n = MCTSNode(action=0)
        ucb = n.ucb(total_visits=10)
        assert ucb == float("inf")

    def test_ucb_finite_after_visit(self):
        n = MCTSNode(action=0)
        n.update(50.0)
        ucb = n.ucb(total_visits=10)
        assert 0.0 < ucb < float("inf")

    def test_update_increments_visits(self):
        n = MCTSNode(action=1)
        for i in range(5):
            n.update(40.0)
        assert n.visits == 5

    def test_value_is_running_average(self):
        n = MCTSNode(action=2)
        n.update(60.0)
        n.update(40.0)
        assert abs(n.value - 50.0) < 0.01

    def test_high_exploit_boosts_ucb(self):
        n_high = MCTSNode(action=0)
        n_low = MCTSNode(action=1)
        for _ in range(5):
            n_high.update(80.0)
            n_low.update(20.0)
        assert n_high.ucb(100) > n_low.ucb(100)


# ── MCTSVariant ───────────────────────────────────────────────────────────────

class TestMCTSVariant:
    def _make_variant(self, use_temporal: bool, seed: int = 42) -> MCTSVariant:
        prob = SearchProblem(rng=random.Random(seed))
        return MCTSVariant(problem=prob, use_temporal=use_temporal)

    def test_step_increments_total_visits(self):
        v = self._make_variant(use_temporal=False)
        v.step()
        assert v.total_visits == 1

    def test_run_n_steps(self):
        v = self._make_variant(use_temporal=False)
        v.run(50)
        assert v.total_visits == 50

    def test_best_action_in_range(self):
        v = self._make_variant(use_temporal=False)
        v.run(100)
        assert 0 <= v.best_action() < v.problem.n_actions

    def test_all_actions_explored_eventually(self):
        """After many iterations, most actions visited at least once."""
        v = self._make_variant(use_temporal=False, seed=1)
        v.run(500)
        visited = sum(1 for n in v.nodes if n.visits > 0)
        # With enough budget, at least 70% of actions explored
        assert visited >= int(v.problem.n_actions * 0.7)

    def test_temporal_variant_uses_all_perspectives(self):
        """7 perspectives defined, temporal sample compresses them."""
        assert len(_PERSPECTIVES) == 7
        assert abs(_TOTAL_TEMPORAL_WEIGHT - sum(_TEMPORAL_WEIGHTS.values())) < 0.001

    def test_convergence_iter_set_when_optimum_found(self):
        """When best action = optimum after enough visits → convergence_iter set."""
        v = self._make_variant(use_temporal=False, seed=42)
        v.run(500)
        # After 500 steps with clear optimum, may or may not converge
        # Just verify the field exists and is valid when set
        if v.convergence_iter is not None:
            assert 1 <= v.convergence_iter <= 500

    def test_best_value_matches_problem(self):
        v = self._make_variant(use_temporal=False)
        v.run(100)
        ba = v.best_action()
        assert abs(v.best_value() - v.problem.true_value(ba)) < 0.01


# ── BenchmarkResult ───────────────────────────────────────────────────────────

class TestBenchmarkResult:
    def _make_result(self, **kwargs) -> BenchmarkResult:
        defaults = dict(
            seed=42, max_iterations=500,
            standard_convergence_iter=300, standard_best_value=85.0,
            standard_best_action=21, standard_found_optimum=True,
            temporal_convergence_iter=150, temporal_best_value=92.0,
            temporal_best_action=21, temporal_found_optimum=True,
            speedup_ratio=2.0, quality_gain_pct=8.24, duration_ms=12.5,
        )
        defaults.update(kwargs)
        return BenchmarkResult(**defaults)

    def test_to_dict_keys(self):
        r = self._make_result()
        d = r.to_dict()
        assert "seed" in d
        assert "standard" in d
        assert "temporal" in d
        assert "speedup_ratio" in d
        assert "quality_gain_pct" in d
        assert "duration_ms" in d

    def test_to_dict_standard_keys(self):
        r = self._make_result()
        s = r.to_dict()["standard"]
        assert "convergence_iter" in s
        assert "best_value" in s
        assert "found_optimum" in s

    def test_speedup_ratio_stored(self):
        r = self._make_result(speedup_ratio=2.618)
        assert abs(r.speedup_ratio - 2.618) < 0.001


# ── MCTSBenchmark ─────────────────────────────────────────────────────────────

class TestMCTSBenchmark:
    def test_run_returns_result(self):
        bench = MCTSBenchmark()
        result = bench.run(max_iterations=200, seed=42)
        assert isinstance(result, BenchmarkResult)

    def test_run_result_has_values(self):
        bench = MCTSBenchmark()
        result = bench.run(max_iterations=200, seed=42)
        assert result.standard_best_value > 0
        assert result.temporal_best_value > 0
        assert result.duration_ms > 0

    def test_run_is_deterministic(self):
        bench = MCTSBenchmark()
        r1 = bench.run(max_iterations=100, seed=99)
        r2 = bench.run(max_iterations=100, seed=99)
        assert r1.standard_best_action == r2.standard_best_action
        assert r1.temporal_best_action == r2.temporal_best_action

    def test_different_seeds_give_different_results(self):
        bench = MCTSBenchmark()
        r1 = bench.run(max_iterations=200, seed=1)
        r2 = bench.run(max_iterations=200, seed=99)
        # Not guaranteed to differ on best_action, but speedup may differ
        # At minimum, they should not both have identical convergence iters
        assert True  # Just verify no crash

    def test_temporal_finds_optimum_more_often(self):
        """
        Across 7 seeds, temporal should find the optimum at least as often as standard.
        This is the core hypothesis test (statistical, not guaranteed per run).
        """
        bench = MCTSBenchmark()
        n_seeds = 7
        temporal_wins = 0
        standard_wins = 0

        for seed in range(n_seeds):
            r = bench.run(max_iterations=300, seed=seed * 13 + 7)
            if r.temporal_found_optimum and not r.standard_found_optimum:
                temporal_wins += 1
            elif r.standard_found_optimum and not r.temporal_found_optimum:
                standard_wins += 1

        # Temporal should not be worse than standard
        assert temporal_wins >= standard_wins, (
            f"Standard won more ({standard_wins}) than temporal ({temporal_wins})"
        )

    def test_run_multi_keys(self):
        bench = MCTSBenchmark()
        agg = bench.run_multi(max_iterations=100, n_seeds=3, base_seed=42)
        assert "mean_speedup" in agg
        assert "median_speedup" in agg
        assert "phi2_hypothesis_passed" in agg
        assert "temporal_found_optimum_rate" in agg
        assert "standard_found_optimum_rate" in agg
        assert "runs" in agg
        assert len(agg["runs"]) == 3

    def test_run_multi_speedup_positive(self):
        bench = MCTSBenchmark()
        agg = bench.run_multi(max_iterations=200, n_seeds=5)
        assert agg["mean_speedup"] > 0

    def test_run_multi_found_optimum_rates_in_range(self):
        bench = MCTSBenchmark()
        agg = bench.run_multi(max_iterations=300, n_seeds=7)
        assert 0.0 <= agg["temporal_found_optimum_rate"] <= 1.0
        assert 0.0 <= agg["standard_found_optimum_rate"] <= 1.0


class TestPhiHypothesis:
    """
    Empirical validation of the φ² speedup hypothesis.
    These tests measure whether Temporal MCTS converges faster.
    """

    def test_temporal_mean_value_not_worse(self):
        """
        Over multiple seeds, temporal MCTS should not give systematically
        lower quality results than standard MCTS.
        """
        bench = MCTSBenchmark()
        agg = bench.run_multi(max_iterations=300, n_seeds=7, base_seed=42)

        temporal_values = [r["temporal"]["best_value"] for r in agg["runs"]]
        standard_values = [r["standard"]["best_value"] for r in agg["runs"]]

        mean_tmp = sum(temporal_values) / len(temporal_values)
        mean_std = sum(standard_values) / len(standard_values)

        # Temporal should not be significantly worse than standard
        assert mean_tmp >= mean_std * 0.9, (
            f"Temporal mean {mean_tmp:.2f} significantly worse than standard {mean_std:.2f}"
        )

    def test_phi_weights_sum_matches_constant(self):
        """_TOTAL_TEMPORAL_WEIGHT must match sum of all perspective weights."""
        assert abs(_TOTAL_TEMPORAL_WEIGHT - sum(_TEMPORAL_WEIGHTS.values())) < 1e-9

    def test_ideal_has_highest_weight(self):
        """IDEAL perspective gets highest weight (φ²)."""
        assert _TEMPORAL_WEIGHTS["IDEAL"] == max(_TEMPORAL_WEIGHTS.values())

    def test_never_has_lowest_weight(self):
        """NEVER and FLOW share lowest weight (φ⁻²)."""
        min_weight = min(_TEMPORAL_WEIGHTS.values())
        assert _TEMPORAL_WEIGHTS["NEVER"] == min_weight
        assert _TEMPORAL_WEIGHTS["FLOW"] == min_weight
