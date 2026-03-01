"""
Tests for Priority 10 Task 1: Risk Classifier & Proposal Executor.

Tests cover:
- Risk classification based on severity and dimension
- Dimension-specific execution handlers
- Graceful failure when dependencies not injected
"""

import pytest
from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProposal
from cynic.kernel.organism.brain.cognition.cortex.proposal_executor import (
    ProposalExecutor,
    ExecutionResult,
    RiskLevel,
)


class TestRiskClassification:
    """Test risk classification logic."""

    def test_1_low_severity_proposal_classified_low_risk(self):
        """Test 1: Low severity proposal (< 0.2) classified as LOW_RISK."""
        executor = ProposalExecutor()
        proposal = SelfProposal(
            probe_id="p1",
            trigger="MANUAL",
            pattern_type="TEST",
            severity=0.15,
            dimension="METRICS",
            target="test_event",
            recommendation="Test recommendation",
            current_value=1.0,
            suggested_value=0.9,
        )

        risk_level = executor.classify_risk(proposal)
        assert risk_level == RiskLevel.LOW_RISK

    def test_2_high_severity_proposal_classified_review_required(self):
        """Test 2: High severity proposal (>= 0.5) classified as REVIEW_REQUIRED."""
        executor = ProposalExecutor()
        proposal = SelfProposal(
            probe_id="p2",
            trigger="MANUAL",
            pattern_type="TEST",
            severity=0.75,
            dimension="METRICS",
            target="test_event",
            recommendation="Test recommendation",
            current_value=1.0,
            suggested_value=0.5,
        )

        risk_level = executor.classify_risk(proposal)
        assert risk_level == RiskLevel.REVIEW_REQUIRED

    def test_3_escore_dimension_always_review_required(self):
        """Test 3: ESCORE dimension always REVIEW_REQUIRED regardless of severity."""
        executor = ProposalExecutor()
        proposal = SelfProposal(
            probe_id="p3",
            trigger="MANUAL",
            pattern_type="TEST",
            severity=0.1,
            dimension="ESCORE",
            target="agent:test_dog",
            recommendation="Test recommendation",
            current_value=30.0,
            suggested_value=38.2,
        )

        risk_level = executor.classify_risk(proposal)
        assert risk_level == RiskLevel.REVIEW_REQUIRED

    def test_4_metrics_rate_spike_low_severity_classified_low_risk(self):
        """Test 4: METRICS + RATE_SPIKE pattern + low severity = LOW_RISK."""
        executor = ProposalExecutor()
        proposal = SelfProposal(
            probe_id="p4",
            trigger="ANOMALY_DETECTED",
            pattern_type="RATE_SPIKE",
            severity=0.18,
            dimension="METRICS",
            target="core.judgment_created",
            recommendation="Implement batching",
            current_value=50.0,
            suggested_value=30.0,
        )

        risk_level = executor.classify_risk(proposal)
        assert risk_level == RiskLevel.LOW_RISK


class TestProposalExecution:
    """Test proposal execution handlers."""

    def test_5_execute_metrics_proposal_succeeds(self):
        """Test 5: Execute METRICS proposal succeeds (logging only)."""
        executor = ProposalExecutor()
        proposal = SelfProposal(
            probe_id="p5",
            trigger="MANUAL",
            pattern_type="RATE_SPIKE",
            severity=0.3,
            dimension="METRICS",
            target="core.judgment_created",
            recommendation="Implement batching",
            current_value=50.0,
            suggested_value=30.0,
        )

        result = executor.execute(proposal)
        assert result.success is True
        assert result.dimension == "METRICS"
        assert "logging" in result.message.lower() or "recorded" in result.message.lower()

    def test_6_execute_qtable_proposal_succeeds(self):
        """Test 6: Execute QTABLE proposal succeeds (with injected qtable)."""
        executor = ProposalExecutor()

        # Create mock qtable
        class MockQTable:
            def __init__(self):
                self._table = {
                    "state_test": {
                        "action_a": {"value": 0.3, "visits": 5}
                    }
                }

            def update(self, state, action, new_value):
                """Mock update method."""
                if state in self._table and action in self._table[state]:
                    self._table[state][action]["value"] = new_value

        executor.set_qtable(MockQTable())

        proposal = SelfProposal(
            probe_id="p6",
            trigger="MANUAL",
            pattern_type="TEST",
            severity=0.4,
            dimension="QTABLE",
            target="state_test:action_a",
            recommendation="Improve Q-value",
            current_value=0.3,
            suggested_value=0.45,
        )

        result = executor.execute(proposal)
        assert result.success is True
        assert result.dimension == "QTABLE"
        assert "updated" in result.message.lower() or "applied" in result.message.lower()

    def test_7_execute_qtable_when_not_injected_fails_gracefully(self):
        """Test 7: Execute QTABLE when QTable not injected fails gracefully."""
        executor = ProposalExecutor()
        # Don't inject qtable

        proposal = SelfProposal(
            probe_id="p7",
            trigger="MANUAL",
            pattern_type="TEST",
            severity=0.4,
            dimension="QTABLE",
            target="state_test:action_a",
            recommendation="Improve Q-value",
            current_value=0.3,
            suggested_value=0.45,
        )

        result = executor.execute(proposal)
        assert result.success is False
        assert "not available" in result.message.lower() or "not injected" in result.message.lower()


class TestExecutionResult:
    """Test ExecutionResult dataclass."""

    def test_execution_result_has_required_fields(self):
        """Test that ExecutionResult has all required fields."""
        result = ExecutionResult(
            success=True,
            dimension="METRICS",
            message="Test execution",
            new_value=0.5,
        )

        assert result.success is True
        assert result.dimension == "METRICS"
        assert result.message == "Test execution"
        assert result.new_value == 0.5

    def test_execution_result_optional_new_value(self):
        """Test that ExecutionResult.new_value is optional."""
        result = ExecutionResult(
            success=False,
            dimension="QTABLE",
            message="Failed to execute",
        )

        assert result.success is False
        assert result.new_value is None
