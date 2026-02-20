"""
Tests for TrustModel: bidirectional confidence tracking.
"""

import pytest
from datetime import datetime
from unittest.mock import AsyncMock

from cynic.core.trust_model import TrustModel, TrustMetrics
from cynic.core.phi import PHI_INV_2, MAX_CONFIDENCE


class TestTrustModelBasics:
    """Basic trust metric calculations."""

    @pytest.mark.asyncio
    async def test_accept_increases_human_confidence(self):
        """Accepting actions increases human confidence."""
        model = TrustModel()

        # User accepts with 70% confidence
        metrics = await model.update_from_action_feedback(
            action_id="a1",
            accepted=True,
            user_confidence=0.7,
            q_score=80.0,
        )

        assert metrics.human_accept_count == 1
        assert metrics.human_reject_count == 0
        assert metrics.human_accept_rate > 0
        assert metrics.human_confidence > 0

    @pytest.mark.asyncio
    async def test_reject_decreases_human_confidence(self):
        """Rejecting actions decreases human confidence."""
        model = TrustModel()

        # Accept once
        await model.update_from_action_feedback(
            action_id="a1",
            accepted=True,
            user_confidence=0.7,
            q_score=80.0,
        )

        # Reject once
        await model.update_from_action_feedback(
            action_id="a2",
            accepted=False,
            user_confidence=0.3,
            q_score=30.0,
        )

        assert model.metrics.human_accept_count == 1
        assert model.metrics.human_reject_count == 1
        # Rate should be (1-1)/(2) = 0, but with EMA it's higher
        assert model.metrics.human_confidence < 0.5

    @pytest.mark.asyncio
    async def test_human_confidence_is_phi_bounded(self):
        """Human confidence never exceeds MAX_CONFIDENCE (61.8%)."""
        model = TrustModel()

        # Try to boost confidence very high
        for _ in range(100):
            await model.update_from_action_feedback(
                action_id="a1",
                accepted=True,
                user_confidence=1.0,  # Try 100%
                q_score=100.0,
            )

        assert model.metrics.human_confidence <= MAX_CONFIDENCE
        assert model.metrics.human_confidence < 0.62


class TestMachineUtility:
    """Machine utility metrics."""

    @pytest.mark.asyncio
    async def test_q_score_tracked(self):
        """Q-scores are tracked for utility calculation."""
        model = TrustModel()

        await model.update_from_action_feedback(
            action_id="a1",
            accepted=True,
            user_confidence=0.5,
            q_score=75.0,
            outcome_success=True,
        )

        assert len(model.q_score_history) == 1
        assert model.metrics.machine_utility > 0

    @pytest.mark.asyncio
    async def test_machine_utility_reflects_completion(self):
        """High Q-scores with low completion = low utility."""
        model = TrustModel()

        # High Q-score but user rejects
        await model.update_from_action_feedback(
            action_id="a1",
            accepted=False,  # User rejects despite high Q
            user_confidence=0.2,
            q_score=90.0,
            outcome_success=False,
        )

        # Utility should be low because not completed
        assert model.metrics.machine_utility < 0.5


class TestSymbiosisReadiness:
    """Symbiosis axiom activation conditions."""

    @pytest.mark.asyncio
    async def test_symbiosis_requires_both_thresholds(self):
        """Both human AND machine must be ≥ 38.2%."""
        model = TrustModel()

        # Start: neither ready
        assert not model.metrics.symbiosis_ready

        # Boost human confidence only
        model.metrics.human_confidence = 0.5  # > 38.2%
        model.metrics.machine_utility = 0.1   # < 38.2%
        model._check_symbiosis_ready()

        # Still not ready
        assert not model.metrics.symbiosis_ready

        # Boost machine too
        model.metrics.machine_utility = 0.5  # > 38.2%
        model._check_symbiosis_ready()

        # Now ready!
        assert model.metrics.symbiosis_ready

    @pytest.mark.asyncio
    async def test_symbiosis_threshold_is_phi_inv_2(self):
        """Symbiosis activates when both ≥ φ⁻² (38.2%)."""
        model = TrustModel()

        # Just below threshold
        model.metrics.human_confidence = PHI_INV_2 - 0.01
        model.metrics.machine_utility = PHI_INV_2 - 0.01
        model._check_symbiosis_ready()
        assert not model.metrics.symbiosis_ready

        # Just above threshold
        model.metrics.human_confidence = PHI_INV_2 + 0.01
        model.metrics.machine_utility = PHI_INV_2 + 0.01
        model._check_symbiosis_ready()
        assert model.metrics.symbiosis_ready


class TestBlendedConfidence:
    """Co-decision confidence blending."""

    @pytest.mark.asyncio
    async def test_blended_confidence_is_product(self):
        """Blended = human × cynic (both must be confident)."""
        model = TrustModel()

        blended = await model.blend_confidence(
            human_confidence=0.8,
            cynic_q_score=80.0,  # Normalized to 0.8
        )

        # 0.8 × 0.8 = 0.64, but clamped to MAX_CONFIDENCE = 0.618
        # So blended exactly MAX_CONFIDENCE (clamped from 0.64)
        assert blended == MAX_CONFIDENCE

    @pytest.mark.asyncio
    async def test_blended_low_when_either_unsure(self):
        """If either party unsure, blended is very low."""
        model = TrustModel()

        blended_both_high = await model.blend_confidence(
            human_confidence=0.8,
            cynic_q_score=80.0,
        )

        blended_human_low = await model.blend_confidence(
            human_confidence=0.2,
            cynic_q_score=80.0,
        )

        blended_cynic_low = await model.blend_confidence(
            human_confidence=0.8,
            cynic_q_score=20.0,
        )

        # Ordering should be: both_high > human_low, cynic_low
        assert blended_both_high > blended_human_low
        assert blended_both_high > blended_cynic_low
        assert blended_human_low < 0.2  # Very low
        assert blended_cynic_low < 0.2  # Very low


class TestEscalation:
    """When should CYNIC escalate instead of co-deciding?"""

    @pytest.mark.asyncio
    async def test_escalate_high_risk_low_confidence(self):
        """High-risk actions need high confidence."""
        model = TrustModel()

        should_escalate, reason = await model.should_cynic_escalate(
            human_confidence=0.4,
            cynic_q_score=50.0,
            action_risk_level="high"
        )

        assert should_escalate
        assert "Blended confidence" in reason

    @pytest.mark.asyncio
    async def test_dont_escalate_low_risk_medium_confidence(self):
        """Low-risk actions only need medium confidence."""
        model = TrustModel()

        should_escalate, reason = await model.should_cynic_escalate(
            human_confidence=0.5,
            cynic_q_score=60.0,
            action_risk_level="low"
        )

        assert not should_escalate

    @pytest.mark.asyncio
    async def test_escalation_thresholds_by_risk(self):
        """Different risk levels have different thresholds."""
        model = TrustModel()

        # Medium confidence (0.5)
        human_conf = 0.5
        cynic_q = 50.0

        # Low risk: should not escalate (threshold 30%)
        esc_low, _ = await model.should_cynic_escalate(human_conf, cynic_q, "low")

        # High risk: should escalate (threshold 70%)
        esc_high, _ = await model.should_cynic_escalate(human_conf, cynic_q, "high")

        assert not esc_low or esc_high  # Low risk less strict


class TestHistoryTracking:
    """Trust model maintains history for analysis."""

    @pytest.mark.asyncio
    async def test_accept_history_tracked(self):
        """All accept/reject events are logged."""
        model = TrustModel()

        for i in range(5):
            await model.update_from_action_feedback(
                action_id=f"a{i}",
                accepted=(i % 2 == 0),  # Alternate accept/reject
                user_confidence=0.5,
                q_score=50.0,
            )

        assert len(model.accept_history) == 5
        assert model.accept_history[0][1] == True  # First accepted
        assert model.accept_history[1][1] == False  # Second rejected

    @pytest.mark.asyncio
    async def test_q_score_history_tracked(self):
        """Q-scores with outcomes are logged."""
        model = TrustModel()

        for i in range(3):
            await model.update_from_action_feedback(
                action_id=f"a{i}",
                accepted=True,
                user_confidence=0.5,
                q_score=float(50 + i * 10),
                outcome_success=True,
            )

        assert len(model.q_score_history) == 3
        assert model.q_score_history[0][1] == 50.0
        assert model.q_score_history[2][1] == 70.0


class TestSummary:
    """Human-readable trust state."""

    @pytest.mark.asyncio
    async def test_summary_readable(self):
        """Summary can be printed for debugging."""
        model = TrustModel()

        await model.update_from_action_feedback(
            action_id="a1",
            accepted=True,
            user_confidence=0.6,
            q_score=75.0,
        )

        summary = model.get_summary()
        assert "Human trust:" in summary
        assert "Machine utility:" in summary
        assert "Symbiosis ready:" in summary
        assert "accepts=" in summary


class TestReset:
    """Testing utilities."""

    @pytest.mark.asyncio
    async def test_reset_clears_state(self):
        """Reset for test isolation."""
        model = TrustModel()

        await model.update_from_action_feedback(
            action_id="a1",
            accepted=True,
            user_confidence=0.6,
            q_score=75.0,
        )

        assert len(model.accept_history) == 1

        await model.reset()

        assert len(model.accept_history) == 0
        assert model.metrics.human_accept_count == 0
        assert model.metrics.human_confidence == 0.0
