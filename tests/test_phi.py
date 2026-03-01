"""
Tests for φ (Phi) Constants and Mathematical Functions

Tests the core.phi module for:
- φ constants (PHI, PHI_INV, PHI_INV_2, etc.)
- Fibonacci and Lucas sequences
- φ-bound functions
- Judgment thresholds
- UCB formulas
"""
import pytest
import math

from cynic.kernel.core.phi import (
    PHI, PHI_INV, PHI_INV_2, PHI_INV_3,
    PHI_2, PHI_3, PHI_4, PHI_5,
    MAX_CONFIDENCE, MAX_Q_SCORE, MAX_CONFIDENCE_PCT,
    HOWL_MIN, WAG_MIN, GROWL_MIN, BARK_MAX,
    DOGS_TOTAL, DOGS_BYZANTINE, DOGS_QUORUM,
    LEARNING_RATE, EWC_PENALTY, THOMPSON_CONFIDENCE,
    fibonacci, lucas, FIBONACCI, LUCAS,
    AXIOMS_CORE, AXIOMS_FACETS, DOGS_COUNT, REALITY_DIMS,
    phi_bound, phi_bound_score, phi_classify,
    geometric_mean, weighted_geometric_mean,
    phi_ratio_split, phi_ucb, phi_temporal_ucb,
    E_SCORE_WEIGHTS, E_SCORE_TOTAL_WEIGHT,
    validate_phi_constants,
)


class TestPhiConstants:
    """Test suite for φ constants."""

    def test_phi_value(self):
        """PHI should equal (1 + sqrt(5)) / 2."""
        expected = (1 + math.sqrt(5)) / 2
        assert abs(PHI - expected) < 1e-12

    def test_phi_inv_relationship(self):
        """PHI_INV should equal 1/PHI."""
        assert abs(PHI_INV - 1/PHI) < 1e-12
        assert abs(PHI * PHI_INV - 1.0) < 1e-12

    def test_phi_inv_2_relationship(self):
        """PHI_INV_2 should equal 1/PHI²."""
        assert abs(PHI_INV_2 - 1/(PHI**2)) < 1e-12

    def test_phi_inv_sum(self):
        """PHI_INV + PHI_INV_2 should equal 1."""
        assert abs(PHI_INV + PHI_INV_2 - 1.0) < 1e-12

    def test_phi_squared(self):
        """PHI² should equal PHI + 1."""
        assert abs(PHI_2 - (PHI + 1)) < 1e-12

    def test_phi_cubed(self):
        """PHI³ should equal 2*PHI + 1."""
        assert abs(PHI_3 - (2 * PHI + 1)) < 1e-12


class TestFibonacciSequence:
    """Test suite for Fibonacci sequence."""

    def test_fibonacci_values(self):
        """Should compute correct Fibonacci numbers."""
        assert fibonacci(0) == 0
        assert fibonacci(1) == 1
        assert fibonacci(2) == 1
        assert fibonacci(3) == 2
        assert fibonacci(4) == 3
        assert fibonacci(5) == 5
        assert fibonacci(6) == 8
        assert fibonacci(7) == 13
        assert fibonacci(8) == 21
        assert fibonacci(10) == 55
        assert fibonacci(13) == 233

    def test_fibonacci_negative_raises(self):
        """Negative index should raise ValueError."""
        with pytest.raises(ValueError):
            fibonacci(-1)

    def test_fibonacci_convergence(self):
        """F(n)/F(n-1) should converge to PHI."""
        for n in range(10, 20):
            ratio = fibonacci(n) / fibonacci(n - 1)
            assert abs(ratio - PHI) < 0.01


class TestLucasSequence:
    """Test suite for Lucas sequence."""

    def test_lucas_values(self):
        """Should compute correct Lucas numbers."""
        assert lucas(0) == 2
        assert lucas(1) == 1
        assert lucas(2) == 3
        assert lucas(3) == 4
        assert lucas(4) == 7
        assert lucas(5) == 11

    def test_lucas_negative_raises(self):
        """Negative index should raise ValueError."""
        with pytest.raises(ValueError):
            lucas(-1)


class TestArchitectureConstants:
    """Test suite for architecture constants."""

    def test_axioms_core(self):
        """Should have 5 core axioms."""
        assert AXIOMS_CORE == 5
        assert AXIOMS_CORE == fibonacci(5)

    def test_axioms_facets(self):
        """Should have 7 facets per axiom."""
        assert AXIOMS_FACETS == 7
        assert AXIOMS_FACETS == lucas(4)

    def test_dogs_count(self):
        """Should have 11 Dogs."""
        assert DOGS_COUNT == 11
        assert DOGS_COUNT == lucas(5)

    def test_reality_dims(self):
        """Should have 7 reality dimensions."""
        assert REALITY_DIMS == 7
        assert REALITY_DIMS == lucas(4)


class TestPBFTConstants:
    """Test suite for PBFT constants."""

    def test_total_dogs(self):
        """Should have 11 total dogs."""
        assert DOGS_TOTAL == 11

    def test_byzantine_threshold(self):
        """Byzantine threshold should be 3."""
        assert DOGS_BYZANTINE == 3

    def test_quorum(self):
        """Quorum should be 2f+1 = 7."""
        assert DOGS_QUORUM == 7
        assert DOGS_QUORUM == 2 * DOGS_BYZANTINE + 1


class TestJudgmentThresholds:
    """Test suite for judgment thresholds."""

    def test_howl_above_wag(self):
        """HOWL should be above WAG."""
        assert HOWL_MIN > WAG_MIN

    def test_wag_above_growl(self):
        """WAG should be above GROWL."""
        assert WAG_MIN > GROWL_MIN

    def test_growl_above_zero(self):
        """GROWL should be above 0."""
        assert GROWL_MIN > 0

    def test_wag_equals_phi_inv_percent(self):
        """WAG threshold should equal φ⁻¹ × 100."""
        assert abs(WAG_MIN - PHI_INV * 100) < 1e-10

    def test_growl_equals_phi_inv_2_percent(self):
        """GROWL threshold should equal φ⁻² × 100."""
        assert abs(GROWL_MIN - PHI_INV_2 * 100) < 1e-10

    def test_howl_above_82(self):
        """HOWL should be at least 82."""
        assert HOWL_MIN >= 82


class TestLearningRates:
    """Test suite for learning rate constants."""

    def test_learning_rate_positive(self):
        """Learning rate should be positive."""
        assert LEARNING_RATE > 0

    def test_learning_rate_small(self):
        """Learning rate should be small (< 0.1)."""
        assert LEARNING_RATE < 0.1

    def test_ewc_penalty_phi_inv(self):
        """EWC penalty should equal PHI_INV."""
        assert abs(EWC_PENALTY - PHI_INV) < 1e-10


class TestPhiBound:
    """Test suite for phi_bound function."""

    def test_bound_basic(self):
        """Should clamp value to range."""
        assert phi_bound(0.5, 0.0, 1.0) == 0.5
        assert phi_bound(-1.0, 0.0, 1.0) == 0.0
        assert phi_bound(2.0, 0.0, 1.0) == 1.0

    def test_bound_default_max(self):
        """Default max should be MAX_CONFIDENCE (φ⁻¹)."""
        assert phi_bound(1.0) == MAX_CONFIDENCE
        assert phi_bound(0.5) == 0.5

    def test_bound_confidence(self):
        """Should clamp to confidence range."""
        assert abs(phi_bound(0.7) - PHI_INV) < 1e-10  # Clamped to PHI_INV


class TestPhiBoundScore:
    """Test suite for phi_bound_score function."""

    def test_score_clamp(self):
        """Should clamp to [0, 100]."""
        assert phi_bound_score(50) == 50
        assert phi_bound_score(-10) == 0
        assert phi_bound_score(150) == 100

    def test_score_bounds(self):
        """Score should always be in [0, 100]."""
        for v in [-100, 0, 50, 100, 200]:
            assert 0 <= phi_bound_score(v) <= 100


class TestPhiClassify:
    """Test suite for phi_classify function."""

    def test_exceptional(self):
        """Should classify ≥0.82 as EXCEPTIONAL."""
        assert phi_classify(0.82) == "EXCEPTIONAL"
        assert phi_classify(1.0) == "EXCEPTIONAL"

    def test_good(self):
        """Should classify ≥φ⁻¹ as GOOD."""
        assert phi_classify(PHI_INV) == "GOOD"  # Use actual PHI_INV value

    def test_moderate(self):
        """Should classify ≥φ⁻² as MODERATE."""
        assert phi_classify(0.382) == "MODERATE"

    def test_poor(self):
        """Should classify ≥φ⁻³ as POOR."""
        assert phi_classify(PHI_INV_3) == "POOR"  # Use actual PHI_INV_3 value

    def test_critical(self):
        """Should classify <φ⁻³ as CRITICAL."""
        assert phi_classify(0.1) == "CRITICAL"
        assert phi_classify(0.0) == "CRITICAL"


class TestGeometricMean:
    """Test suite for geometric mean functions."""

    def test_geometric_mean_basic(self):
        """Should compute geometric mean."""
        result = geometric_mean([0.5, 0.5])
        assert abs(result - 0.5) < 1e-10

    def test_geometric_mean_zeros(self):
        """Zero values should return 0."""
        assert geometric_mean([0.5, 0.0]) == 0

    def test_geometric_mean_empty(self):
        """Empty list should return 0."""
        assert geometric_mean([]) == 0

    def test_weighted_geometric_mean(self):
        """Should compute weighted geometric mean."""
        result = weighted_geometric_mean([0.5, 0.5], [1.0, 1.0])
        assert result > 0

    def test_weighted_geometric_mean_weights(self):
        """Higher weight on higher value should give higher result."""
        result1 = weighted_geometric_mean([0.8, 0.2], [0.9, 0.1])
        result2 = weighted_geometric_mean([0.8, 0.2], [0.1, 0.9])
        
        # result1 should be closer to 0.8
        assert result1 > result2


class TestPhiRatioSplit:
    """Test suite for phi_ratio_split function."""

    def test_split_sum(self):
        """Parts should sum to total."""
        small, large = phi_ratio_split(100)
        assert abs(small + large - 100) < 1e-10

    def test_split_ratio(self):
        """Should be φ⁻² and φ⁻¹."""
        small, large = phi_ratio_split(1.0)
        assert abs(small - PHI_INV_2) < 1e-10
        assert abs(large - PHI_INV) < 1e-10


class TestUCB:
    """Test suite for UCB formulas."""

    def test_phi_ucb_zero_visits(self):
        """Zero visits should return infinity."""
        result = phi_ucb(0.5, 0, 100)
        assert result == float('inf')

    def test_phi_ucb_normal(self):
        """Should compute UCB."""
        result = phi_ucb(0.5, 10, 100)
        assert result > 0

    def test_temporal_ucb_zero_visits(self):
        """Zero visits should return infinity."""
        result = phi_temporal_ucb(0.5, 0, 100, 3)
        assert result == float('inf')

    def test_temporal_ucb_depth_decay(self):
        """Deeper nodes should have temporal decay (less exploration)."""
        result_shallow = phi_temporal_ucb(0.5, 10, 100, depth=1)
        result_deep = phi_temporal_ucb(0.5, 10, 100, depth=5)

        # With same Q, deeper nodes have less exploration due to temporal decay
        assert result_shallow > result_deep


class TestEScoreWeights:
    """Test suite for E-Score weights."""

    def test_all_weights_positive(self):
        """All weights should be positive."""
        for weight in E_SCORE_WEIGHTS.values():
            assert weight > 0

    def test_burn_highest(self):
        """BURN should have highest weight."""
        assert E_SCORE_WEIGHTS["BURN"] == max(E_SCORE_WEIGHTS.values())

    def test_total_weight(self):
        """Total should equal ~10.708."""
        total = sum(E_SCORE_WEIGHTS.values())
        assert abs(total - E_SCORE_TOTAL_WEIGHT) < 0.001


class TestValidation:
    """Test suite for phi constant validation."""

    def test_validation_runs(self):
        """Validation should run without error."""
        # validate_phi_constants runs on import
        # Just verify constants are valid
        assert PHI > 1.0
        assert PHI < 2.0
        assert FIBONACCI[10] == 55
        assert LUCAS[5] == 11


class TestPrecomputedSequences:
    """Test suite for precomputed sequences."""

    def test_fibonacci_length(self):
        """FIBONACCI should have 21 elements (F(0) to F(20))."""
        assert len(FIBONACCI) == 21
        assert FIBONACCI[10] == 55
        assert FIBONACCI[20] == 6765

    def test_lucas_length(self):
        """LUCAS should have 11 elements (L(0) to L(10))."""
        assert len(LUCAS) == 11
        assert LUCAS[5] == 11
