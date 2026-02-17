"""
Tests: φ Constants (SINGLE SOURCE OF TRUTH)

These are the foundational laws. If ANY of these fail,
the entire CYNIC organism is built on sand.

LAW: validate_phi_constants() is called on import.
These tests verify the verification itself.
"""
import math
import pytest
from cynic.core.phi import (
    PHI, PHI_INV, PHI_INV_2, PHI_INV_3,
    PHI_2, PHI_3, PHI_4, PHI_5,
    fibonacci, lucas,
    MAX_Q_SCORE, MAX_CONFIDENCE,
    DOGS_TOTAL, DOGS_BYZANTINE, DOGS_QUORUM,
    HOWL_MIN, WAG_MIN, GROWL_MIN,
    E_SCORE_WEIGHTS,
    phi_bound, phi_bound_score, phi_classify,
    geometric_mean, weighted_geometric_mean,
    phi_ratio_split, phi_ucb, phi_temporal_ucb,
)


class TestPhiConstants:
    """φ mathematical identities."""

    def test_phi_identity(self):
        """φ × φ⁻¹ = 1."""
        assert abs(PHI * PHI_INV - 1.0) < 1e-12

    def test_phi_squared(self):
        """φ² = φ + 1 (unique property of golden ratio)."""
        assert abs(PHI_2 - (PHI + 1)) < 1e-12

    def test_phi_cubed(self):
        """φ³ = 2φ + 1."""
        assert abs(PHI_3 - (2 * PHI + 1)) < 1e-12

    def test_phi_inv_sum(self):
        """φ⁻¹ + φ⁻² = 1."""
        assert abs(PHI_INV + PHI_INV_2 - 1.0) < 1e-12

    def test_phi_precision(self):
        """φ accurate to 12 decimals."""
        expected = (1 + math.sqrt(5)) / 2
        assert abs(PHI - expected) < 1e-12

    def test_max_confidence(self):
        """Max confidence = φ⁻¹ = 0.618."""
        assert abs(MAX_CONFIDENCE - PHI_INV) < 1e-12
        assert MAX_CONFIDENCE < 1.0  # Never 100% confident

    def test_max_q_score(self):
        """Max Q-Score = 61.8."""
        assert abs(MAX_Q_SCORE - 61.8) < 0.01


class TestFibonacciLucas:
    """Fibonacci and Lucas sequences — architecture derivations."""

    def test_fibonacci_sequence(self):
        expected = [0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55]
        for n, exp in enumerate(expected):
            assert fibonacci(n) == exp, f"F({n}) should be {exp}"

    def test_lucas_sequence(self):
        expected = [2, 1, 3, 4, 7, 11, 18, 29]
        for n, exp in enumerate(expected):
            assert lucas(n) == exp, f"L({n}) should be {exp}"

    def test_architecture_derivation(self):
        """5 axioms, 7 dimensions, 11 Dogs all derived from φ."""
        assert fibonacci(5) == 5   # Core axioms
        assert lucas(4) == 7       # Reality/Analysis/Time dims
        assert lucas(5) == 11      # Dogs (Sefirot)

    def test_fibonacci_convergence(self):
        """F(n)/F(n-1) converges to φ."""
        for n in range(10, 18):
            ratio = fibonacci(n) / fibonacci(n - 1)
            assert abs(ratio - PHI) < 0.01, f"F({n})/F({n-1}) = {ratio} should ≈ φ"


class TestPBFT:
    """PBFT consensus parameters."""

    def test_quorum(self):
        """Quorum = 2f+1 where f = Byzantine faults."""
        assert DOGS_QUORUM == 2 * DOGS_BYZANTINE + 1

    def test_dogs_count(self):
        assert DOGS_TOTAL == 11
        assert DOGS_BYZANTINE == 3
        assert DOGS_QUORUM == 7


class TestVerdictThresholds:
    """Verdict thresholds are φ-aligned and ordered."""

    def test_order(self):
        assert HOWL_MIN > WAG_MIN > GROWL_MIN > 0

    def test_wag_equals_max_q(self):
        """WAG threshold = MAX_Q_SCORE = φ⁻¹ × 100."""
        assert abs(WAG_MIN - MAX_Q_SCORE) < 1e-10  # exact equality (same constant)

    def test_growl_equals_phi_inv2_100(self):
        """GROWL threshold = φ⁻² × 100 = 38.2."""
        assert abs(GROWL_MIN - PHI_INV_2 * 100) < 0.01


class TestPhiFunctions:
    """φ utility functions."""

    def test_phi_bound_clamps(self):
        assert phi_bound(0.8) == MAX_CONFIDENCE  # clamps at 0.618
        assert phi_bound(-0.1) == 0.0
        assert phi_bound(0.4) == 0.4

    def test_phi_bound_score_clamps(self):
        assert phi_bound_score(100.0) == MAX_Q_SCORE  # clamps at 61.8
        assert phi_bound_score(-5.0) == 0.0
        assert phi_bound_score(30.0) == 30.0

    def test_phi_classify(self):
        assert phi_classify(0.9) == "EXCEPTIONAL"
        assert phi_classify(0.7) == "GOOD"
        assert phi_classify(0.5) == "MODERATE"
        assert phi_classify(0.3) == "POOR"
        assert phi_classify(0.1) == "CRITICAL"

    def test_geometric_mean(self):
        assert geometric_mean([]) == 0.0
        assert geometric_mean([0, 0.5]) == 0.0  # zero collapses all
        result = geometric_mean([1.0, 1.0, 1.0])
        assert abs(result - 1.0) < 1e-10

    def test_weighted_geometric_mean(self):
        # Equal weights → same as geometric mean
        result = weighted_geometric_mean([1.0, 1.0], [1.0, 1.0])
        assert abs(result - 1.0) < 1e-10

        # One zero → collapses
        assert weighted_geometric_mean([0.0, 1.0], [1.0, 1.0]) == 0.0

    def test_phi_ratio_split(self):
        small, large = phi_ratio_split(100.0)
        assert abs(small - 38.2) < 0.1
        assert abs(large - 61.8) < 0.1
        assert abs(small + large - 100.0) < 0.01

    def test_phi_ucb_infinity_on_zero_visits(self):
        assert phi_ucb(0.0, visits=0, parent_visits=10) == float("inf")

    def test_phi_temporal_ucb_decays_with_depth(self):
        ucb_depth0 = phi_temporal_ucb(1.0, 1, 10, depth=0)
        ucb_depth3 = phi_temporal_ucb(1.0, 1, 10, depth=3)
        assert ucb_depth3 < ucb_depth0  # deeper = less certain


class TestEScoreWeights:
    """E-Score 7D weights are φ-symmetric."""

    def test_weights_ordered(self):
        w = E_SCORE_WEIGHTS
        assert w["BURN"] > w["BUILD"] > w["JUDGE"] > w["RUN"]
        assert w["RUN"] > w["SOCIAL"] > w["GRAPH"] > w["HOLD"]

    def test_burn_is_phi_cubed(self):
        assert abs(E_SCORE_WEIGHTS["BURN"] - PHI_3) < 1e-10

    def test_run_is_one(self):
        assert abs(E_SCORE_WEIGHTS["RUN"] - 1.0) < 1e-10

    def test_total_weight(self):
        total = sum(E_SCORE_WEIGHTS.values())
        assert abs(total - 10.708) < 0.001
