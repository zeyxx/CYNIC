"""
Phase 3 Tests: Entropy Metric — Measure Information Density

Tests that CYNIC:
  1. Calculates information entropy correctly
  2. Tracks efficiency across judgments
  3. Detects when system adds noise vs creates knowledge
  4. Alerts on poor efficiency (avg < 0)
"""
import pytest
import math
from unittest.mock import MagicMock

from cynic.cognition.cortex.entropy_tracker import (
    EntropyTracker,
    EntropyMetrics,
    EntropyCalculator,
)


# ════════════════════════════════════════════════════════════════════════════
# TASK 3.1: EntropyTracker Implementation
# ════════════════════════════════════════════════════════════════════════════


class TestEntropyCalculator:
    """Test entropy calculation (Shannon formula)."""

    def test_entropy_calculator_uniform_distribution(self):
        """Test entropy when all signal types are equally likely."""
        # 4 signal types, 1 each = uniform distribution
        signals = [
            {"type": "syntax_error"},
            {"type": "security_issue"},
            {"type": "performance_gap"},
            {"type": "style_violation"},
        ]

        calc = EntropyCalculator()
        h = calc.calculate_observation_entropy(signals)

        # Uniform distribution with 4 types = H = log₂(4) = 2 bits
        assert abs(h - 2.0) < 0.01

    def test_entropy_calculator_skewed_distribution(self):
        """Test entropy when one signal type dominates."""
        signals = [
            {"type": "error"} for _ in range(9)  # 9 errors
        ] + [
            {"type": "warning"}  # 1 warning
        ]

        calc = EntropyCalculator()
        h = calc.calculate_observation_entropy(signals)

        # Skewed distribution = lower entropy than uniform
        # Max entropy would be log₂(10) ≈ 3.32 bits
        assert h < 1.5  # Should be quite low

    def test_entropy_calculator_empty_signals(self):
        """Test entropy with no observations."""
        signals = []

        calc = EntropyCalculator()
        h = calc.calculate_observation_entropy(signals)

        assert h == 0.0

    def test_confidence_entropy_certain_judgment(self):
        """Test entropy when judgment is certain (high confidence)."""
        calc = EntropyCalculator()

        # 99.9% confident = nearly certain
        h = calc.calculate_confidence_entropy(0.999)
        assert h < 0.05

        # 100% confident = completely certain
        h = calc.calculate_confidence_entropy(1.0)
        assert h == 0.0

    def test_confidence_entropy_uncertain_judgment(self):
        """Test entropy when judgment is uncertain (low confidence)."""
        calc = EntropyCalculator()

        # 50% confident = maximum entropy
        h = calc.calculate_confidence_entropy(0.5)
        assert abs(h - 1.0) < 0.01  # Should be exactly 1.0

    def test_confidence_entropy_phi_bounded(self):
        """Test entropy for φ-bounded confidence (≤0.618)."""
        calc = EntropyCalculator()

        # φ⁻¹ = 0.618 = max confidence
        h = calc.calculate_confidence_entropy(0.618)

        # Should be between uncertain (1.0) and certain (0)
        assert 0.0 < h < 1.0

    def test_efficiency_positive(self):
        """Test efficiency when observation entropy > output entropy."""
        calc = EntropyCalculator()

        h_input = 2.5  # Diverse observations
        h_output = 0.3  # Confident judgment

        efficiency = calc.calculate_efficiency(h_input, h_output)

        # Should be positive (2.5 - 0.3 = 2.2)
        assert efficiency > 0.0
        assert abs(efficiency - 2.2) < 0.01

    def test_efficiency_negative(self):
        """Test efficiency when observation entropy < output entropy (bad judgment)."""
        calc = EntropyCalculator()

        h_input = 0.5  # Few observations
        h_output = 1.0  # Uncertain judgment

        efficiency = calc.calculate_efficiency(h_input, h_output)

        # Should be negative (0.5 - 1.0 = -0.5)
        assert efficiency < 0.0


# ════════════════════════════════════════════════════════════════════════════
# TASK 3.2: Entropy Tracking
# ════════════════════════════════════════════════════════════════════════════


class TestEntropyTracking:
    """Test entropy tracking across judgments."""

    @pytest.mark.asyncio
    async def test_entropy_tracker_basic_track(self):
        """Test tracker can record entropy metrics."""
        tracker = EntropyTracker()

        signals = [
            {"type": "error"},
            {"type": "warning"},
            {"type": "error"},
        ]

        metrics = await tracker.track_judgment(
            dog_id="ANALYST",
            cell_id="cell_1",
            signals=signals,
            verdict="GROWL",
            confidence=0.45,
        )

        assert metrics.dog_id == "ANALYST"
        assert metrics.cell_id == "cell_1"
        assert metrics.verdict == "GROWL"
        assert metrics.h_input >= 0.0
        assert metrics.h_output >= 0.0
        assert metrics.efficiency == metrics.h_input - metrics.h_output

    @pytest.mark.asyncio
    async def test_entropy_tracker_high_efficiency(self):
        """Test tracker records high efficiency (good judgment)."""
        tracker = EntropyTracker()

        # Diverse signals (all different types) → very confident judgment = high efficiency
        signals = [
            {"type": f"signal_{i}"} for i in range(8)  # 8 different signals (H ≈ 3 bits)
        ]

        metrics = await tracker.track_judgment(
            dog_id="SAGE",
            cell_id="cell_diverse",
            signals=signals,
            verdict="WAG",
            confidence=0.95,  # VERY high confidence (entropy near 0)
        )

        # High h_input (diverse), very low h_output (very confident)
        assert metrics.h_input >= 2.5  # Diverse signals
        assert metrics.h_output < 0.2  # Very low entropy (0.95 confidence near-certain)
        assert metrics.efficiency > 2.0  # Excellent efficiency (compressed 3+ bits)

    @pytest.mark.asyncio
    async def test_entropy_tracker_low_efficiency(self):
        """Test tracker records low efficiency (poor judgment)."""
        tracker = EntropyTracker()

        # Same type signals → 50/50 uncertain judgment = negative efficiency (worst)
        signals = [{"type": "error"}, {"type": "error"}, {"type": "error"}]

        metrics = await tracker.track_judgment(
            dog_id="JANITOR",
            cell_id="cell_homogeneous",
            signals=signals,
            verdict="BARK",
            confidence=0.50,  # 50/50 uncertain = maximum entropy output
        )

        # Homogeneous signals (H = 0), maximally uncertain judgment (H ≈ 1.0)
        assert metrics.h_input == 0.0  # All same type = no input entropy
        assert metrics.h_output > 0.99  # 0.5 confidence = maximum entropy (≈1.0)
        assert metrics.efficiency < -0.9  # Very negative efficiency (worst case)

    @pytest.mark.asyncio
    async def test_entropy_tracker_bounded_history(self):
        """Test tracker caps history at F(11)=89."""
        tracker = EntropyTracker(max_history=10)  # Use 10 for test

        signals = [{"type": "test"}]

        # Add 15 metrics
        for i in range(15):
            await tracker.track_judgment(
                dog_id="DOG",
                cell_id=f"cell_{i}",
                signals=signals,
                verdict="WAG",
                confidence=0.5,
            )

        # Should only keep last 10
        assert len(tracker.metrics) == 10

    @pytest.mark.asyncio
    async def test_entropy_tracker_statistics(self):
        """Test tracker computes aggregate statistics."""
        tracker = EntropyTracker()

        # Track 5 judgments
        for i in range(5):
            signals = [{"type": f"sig_{j}"} for j in range(i + 1)]
            await tracker.track_judgment(
                dog_id="ARCHITECT",
                cell_id=f"cell_{i}",
                signals=signals,
                verdict="WAG",
                confidence=0.5 + (i * 0.05),
            )

        stats = tracker.get_statistics()

        assert stats["total_tracked"] == 5
        assert "avg_efficiency" in stats
        assert "min_efficiency" in stats
        assert "max_efficiency" in stats
        assert "neg_efficiency_count" in stats
        assert "verdict_distribution" in stats
        assert stats["verdict_distribution"].get("WAG", 0) == 5

    @pytest.mark.asyncio
    async def test_entropy_tracker_alert_on_low_efficiency(self):
        """Test tracker alerts when avg efficiency < 0 (adding noise)."""
        tracker = EntropyTracker()

        # Create judgments with mostly negative efficiency
        for i in range(5):
            # Few signals, low confidence = negative efficiency
            signals = [{"type": "error"}]
            await tracker.track_judgment(
                dog_id="BAD_DOG",
                cell_id=f"noisy_{i}",
                signals=signals,
                verdict="BARK",
                confidence=0.1,
            )

        stats = tracker.get_statistics()

        # Should trigger alert
        assert stats["alert"] is True
        assert stats["avg_efficiency"] < 0.0
        assert stats["neg_efficiency_count"] > 0

    @pytest.mark.asyncio
    async def test_entropy_tracker_dog_specific_stats(self):
        """Test tracker can report per-dog efficiency."""
        tracker = EntropyTracker()

        # SAGE: high efficiency
        signals_sage = [{"type": f"sig_{i}"} for i in range(5)]
        await tracker.track_judgment(
            dog_id="SAGE",
            cell_id="sage_cell",
            signals=signals_sage,
            verdict="WAG",
            confidence=0.55,
        )

        # JANITOR: low efficiency
        signals_janitor = [{"type": "error"}]
        await tracker.track_judgment(
            dog_id="JANITOR",
            cell_id="janitor_cell",
            signals=signals_janitor,
            verdict="BARK",
            confidence=0.15,
        )

        sage_stats = tracker.get_dog_efficiency("SAGE")
        janitor_stats = tracker.get_dog_efficiency("JANITOR")

        assert sage_stats["avg_efficiency"] > janitor_stats["avg_efficiency"]
        assert sage_stats["tracked"] == 1
        assert janitor_stats["tracked"] == 1


# ════════════════════════════════════════════════════════════════════════════
# TASK 3.3: Worst/Best Judgment Identification
# ════════════════════════════════════════════════════════════════════════════


class TestEntropyRanking:
    """Test identifying best and worst judgments."""

    @pytest.mark.asyncio
    async def test_worst_judgments(self):
        """Test tracker identifies worst (lowest efficiency) judgments."""
        tracker = EntropyTracker()

        # Create judgments with varying efficiency
        efficiencies = []
        for i in range(5):
            signals = [{"type": f"sig_{j}"} for j in range((i + 1) * 2)]
            confidence = 0.2 + (i * 0.1)

            metrics = await tracker.track_judgment(
                dog_id="MIXED",
                cell_id=f"cell_{i}",
                signals=signals,
                verdict="WAG",
                confidence=confidence,
            )
            efficiencies.append(metrics.efficiency)

        # Get worst judgments
        worst = tracker.get_worst_judgments(limit=2)

        assert len(worst) == 2
        # Worst should have lowest efficiency
        assert worst[0].efficiency <= worst[1].efficiency

    @pytest.mark.asyncio
    async def test_best_judgments(self):
        """Test tracker identifies best (highest efficiency) judgments."""
        tracker = EntropyTracker()

        # Create judgments with varying efficiency
        for i in range(5):
            signals = [{"type": f"sig_{j}"} for j in range((i + 1) * 2)]
            confidence = 0.2 + (i * 0.1)

            await tracker.track_judgment(
                dog_id="MIXED",
                cell_id=f"cell_{i}",
                signals=signals,
                verdict="WAG",
                confidence=confidence,
            )

        # Get best judgments
        best = tracker.get_best_judgments(limit=2)

        assert len(best) == 2
        # Best should have highest efficiency
        assert best[0].efficiency >= best[1].efficiency


# ════════════════════════════════════════════════════════════════════════════
# INTEGRATION TEST: Entropy in Judgment Flow
# ════════════════════════════════════════════════════════════════════════════


class TestEntropyIntegration:
    """Integration test: entropy tracking in full judgment flow."""

    @pytest.mark.asyncio
    async def test_full_judgment_entropy_flow(self):
        """
        Test complete entropy tracking:
        1. Dog perceives signals
        2. Dog judges (confident/uncertain)
        3. Entropy metrics recorded
        4. Statistics reveal if system creates knowledge
        """
        tracker = EntropyTracker()

        # Scenario 1: GOOD judgment (high efficiency)
        # Diverse signals + high confidence = positive efficiency
        good_signals = [
            {"type": "security_issue"},
            {"type": "style_violation"},
            {"type": "performance_gap"},
            {"type": "documentation"},
        ]

        good_metrics = await tracker.track_judgment(
            dog_id="GUARDIAN",
            cell_id="analysis_diverse",
            signals=good_signals,
            verdict="BARK",
            confidence=0.55,  # Confident despite diversity
        )

        # Scenario 2: MEDIUM judgment (lower efficiency)
        # Homogeneous signals + high confidence = still negative but better than uncertain
        medium_signals = [
            {"type": "error"},
            {"type": "error"},
        ]

        medium_metrics = await tracker.track_judgment(
            dog_id="ARCHITECT",
            cell_id="homogeneous_confident",
            signals=medium_signals,
            verdict="GROWL",
            confidence=0.60,  # Higher confidence
        )

        # Scenario 3: POOR judgment (worst efficiency)
        # Homogeneous signals + low confidence = worst (most uncertainty added)
        poor_signals = [{"type": "error"}, {"type": "error"}]

        poor_metrics = await tracker.track_judgment(
            dog_id="SCOUT",
            cell_id="homogeneous_uncertain",
            signals=poor_signals,
            verdict="WAG",
            confidence=0.05,  # Very uncertain
        )

        # Verify hierarchy: diverse+confident > homogeneous+confident > homogeneous+uncertain
        assert good_metrics.efficiency > medium_metrics.efficiency
        assert medium_metrics.efficiency > poor_metrics.efficiency

        # Overall statistics
        stats = tracker.get_statistics()

        # System should still be creating knowledge on average
        # (2 good/neutral vs 1 poor)
        assert stats["avg_efficiency"] > 0.0
        assert stats["total_tracked"] == 3
        assert stats["alert"] is False  # Avg is positive
