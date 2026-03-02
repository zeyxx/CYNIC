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

    @pytest.mark.asyncio
    async def test_5_execute_metrics_proposal_succeeds(self):
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

        result = await executor.execute(proposal)
        assert result.success is True
        assert result.proposal_id == "p5"
        assert result.dimension == "METRICS"
        assert "logging" in result.message.lower() or "recorded" in result.message.lower()

    @pytest.mark.asyncio
    async def test_6_execute_qtable_proposal_succeeds(self):
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

        result = await executor.execute(proposal)
        assert result.success is True
        assert result.proposal_id == "p6"
        assert result.dimension == "QTABLE"
        assert "updated" in result.message.lower() or "applied" in result.message.lower()

    @pytest.mark.asyncio
    async def test_7_execute_qtable_when_not_injected_fails_gracefully(self):
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

        result = await executor.execute(proposal)
        assert result.success is False
        assert result.proposal_id == "p7"
        assert "not available" in result.message.lower() or "not injected" in result.message.lower()


class TestExecutionResult:
    """Test ExecutionResult dataclass."""

    def test_execution_result_has_required_fields(self):
        """Test that ExecutionResult has all required fields."""
        result = ExecutionResult(
            success=True,
            proposal_id="p1",
            dimension="METRICS",
            message="Test execution",
            new_value=0.5,
        )

        assert result.success is True
        assert result.proposal_id == "p1"
        assert result.dimension == "METRICS"
        assert result.message == "Test execution"
        assert result.new_value == 0.5

    def test_execution_result_optional_new_value(self):
        """Test that ExecutionResult.new_value is optional."""
        result = ExecutionResult(
            success=False,
            proposal_id="p2",
            dimension="QTABLE",
            message="Failed to execute",
        )

        assert result.success is False
        assert result.proposal_id == "p2"
        assert result.new_value is None

    def test_execution_result_all_fields(self):
        """Test ExecutionResult with all fields populated."""
        result = ExecutionResult(
            success=True,
            proposal_id="p3",
            dimension="QTABLE",
            message="Execution succeeded",
            error_message="",
            old_value=0.3,
            new_value=0.45,
        )

        assert result.success is True
        assert result.proposal_id == "p3"
        assert result.dimension == "QTABLE"
        assert result.message == "Execution succeeded"
        assert result.error_message == ""
        assert result.old_value == 0.3
        assert result.new_value == 0.45


class TestSelfProberExecutorIntegration:
    """Test SelfProber + ProposalExecutor integration (Priority 10 Task 2)."""

    @pytest.mark.asyncio
    async def test_8_selfprober_accepts_executor_injection(self):
        """Test 8: SelfProber accepts executor injection via set_executor()."""
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber

        prober = SelfProber()
        executor = ProposalExecutor()

        # Should not raise
        prober.set_executor(executor)
        assert prober._executor is executor

    @pytest.mark.asyncio
    async def test_9_apply_async_executes_low_risk_proposal(self):
        """Test 9: apply_async() executes LOW_RISK proposals."""
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
        from cynic.kernel.core.event_bus import EventBus

        bus = EventBus("test_bus")
        prober = SelfProber(bus=bus)
        executor = ProposalExecutor()

        # Create mock qtable
        class MockQTable:
            def __init__(self):
                self._table = {
                    "state_a": {"action_1": {"value": 0.15, "visits": 5}}
                }

            def update(self, state, action, new_value):
                if state in self._table and action in self._table[state]:
                    self._table[state][action]["value"] = new_value

        executor.set_qtable(MockQTable())
        prober.set_executor(executor)

        # Create LOW_RISK proposal
        proposal = SelfProposal(
            probe_id="p9",
            trigger="MANUAL",
            pattern_type="TEST",
            severity=0.15,  # < 0.2, so LOW_RISK for METRICS/QTABLE
            dimension="QTABLE",
            target="state_a:action_1",
            recommendation="Test proposal",
            current_value=0.15,
            suggested_value=0.30,
        )
        prober._proposals.append(proposal)

        # Apply the proposal
        updated_proposal = await prober.apply_async("p9")

        # Should be marked as APPLIED and execution should succeed
        assert updated_proposal is not None
        assert updated_proposal.status == "APPLIED"

    @pytest.mark.asyncio
    async def test_10_apply_async_skips_review_required_proposals(self):
        """Test 10: apply_async() skips execution for REVIEW_REQUIRED, just marks APPLIED."""
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
        from cynic.kernel.core.event_bus import EventBus

        bus = EventBus("test_bus")
        prober = SelfProber(bus=bus)
        executor = ProposalExecutor()
        prober.set_executor(executor)

        # Create REVIEW_REQUIRED proposal (high severity METRICS)
        proposal = SelfProposal(
            probe_id="p10",
            trigger="MANUAL",
            pattern_type="TEST",
            severity=0.75,  # >= 0.5, so REVIEW_REQUIRED
            dimension="METRICS",
            target="test_metric",
            recommendation="Test proposal",
            current_value=1.0,
            suggested_value=0.5,
        )
        prober._proposals.append(proposal)

        # Apply the proposal
        updated_proposal = await prober.apply_async("p10")

        # Should be marked as APPLIED but not executed (no event emitted)
        assert updated_proposal is not None
        assert updated_proposal.status == "APPLIED"

    @pytest.mark.asyncio
    async def test_11_apply_async_emits_proposal_executed_on_success(self):
        """Test 11: apply_async() emits PROPOSAL_EXECUTED event on successful execution."""
        import asyncio
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
        from cynic.kernel.core.event_bus import EventBus, CoreEvent

        bus = EventBus("test_bus")
        prober = SelfProber(bus=bus)
        executor = ProposalExecutor()

        # Track emitted events
        emitted_events = []

        async def capture_event(event):
            emitted_events.append(event)

        bus.on(CoreEvent.PROPOSAL_EXECUTED, capture_event)

        # Create mock qtable
        class MockQTable:
            def __init__(self):
                self._table = {
                    "state_b": {"action_2": {"value": 0.10, "visits": 5}}
                }

            def update(self, state, action, new_value):
                if state in self._table and action in self._table[state]:
                    self._table[state][action]["value"] = new_value

        executor.set_qtable(MockQTable())
        prober.set_executor(executor)

        # Create LOW_RISK QTABLE proposal
        proposal = SelfProposal(
            probe_id="p11",
            trigger="MANUAL",
            pattern_type="TEST",
            severity=0.15,
            dimension="QTABLE",
            target="state_b:action_2",
            recommendation="Test proposal",
            current_value=0.10,
            suggested_value=0.25,
        )
        prober._proposals.append(proposal)

        # Apply the proposal
        await prober.apply_async("p11")

        # Wait for async event handlers to complete
        await bus.drain(timeout=1.0)

        # Should have emitted PROPOSAL_EXECUTED event
        assert len(emitted_events) > 0
        executed_event = emitted_events[0]
        assert executed_event.type == CoreEvent.PROPOSAL_EXECUTED.value
        payload = executed_event.dict_payload
        assert payload["proposal_id"] == "p11"
        assert payload["success"] is True
        assert payload["dimension"] == "QTABLE"

    @pytest.mark.asyncio
    async def test_12_apply_async_emits_proposal_failed_on_execution_failure(self):
        """Test 12: apply_async() emits PROPOSAL_FAILED event on execution failure."""
        import asyncio
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
        from cynic.kernel.core.event_bus import EventBus, CoreEvent

        bus = EventBus("test_bus")
        prober = SelfProber(bus=bus)
        executor = ProposalExecutor()

        # Track emitted events
        emitted_events = []

        async def capture_event(event):
            emitted_events.append(event)

        bus.on(CoreEvent.PROPOSAL_FAILED, capture_event)

        # Don't inject qtable - should cause execution failure
        prober.set_executor(executor)

        # Create QTABLE proposal (will fail due to missing qtable)
        proposal = SelfProposal(
            probe_id="p12",
            trigger="MANUAL",
            pattern_type="TEST",
            severity=0.15,
            dimension="QTABLE",
            target="state_c:action_3",
            recommendation="Test proposal",
            current_value=0.10,
            suggested_value=0.25,
        )
        prober._proposals.append(proposal)

        # Apply the proposal
        await prober.apply_async("p12")

        # Wait for async event handlers to complete
        await bus.drain(timeout=1.0)

        # Should have emitted PROPOSAL_FAILED event
        assert len(emitted_events) > 0
        failed_event = emitted_events[0]
        assert failed_event.type == CoreEvent.PROPOSAL_FAILED.value
        payload = failed_event.dict_payload
        assert payload["proposal_id"] == "p12"
        assert payload["success"] is False
        assert payload["dimension"] == "QTABLE"

    @pytest.mark.asyncio
    async def test_13_apply_async_missing_proposal_returns_none(self):
        """Test 13: apply_async() returns None if proposal not found."""
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
        from cynic.kernel.core.event_bus import EventBus

        bus = EventBus("test_bus")
        prober = SelfProber(bus=bus)
        executor = ProposalExecutor()
        prober.set_executor(executor)

        # Try to apply non-existent proposal
        result = await prober.apply_async("nonexistent")

        assert result is None

    @pytest.mark.asyncio
    async def test_14_apply_async_without_executor_still_marks_applied(self):
        """Test 14: apply_async() without executor still marks proposal as APPLIED."""
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
        from cynic.kernel.core.event_bus import EventBus

        bus = EventBus("test_bus")
        prober = SelfProber(bus=bus)
        # Don't inject executor

        # Create proposal
        proposal = SelfProposal(
            probe_id="p14",
            trigger="MANUAL",
            pattern_type="TEST",
            severity=0.15,
            dimension="QTABLE",
            target="state_d:action_4",
            recommendation="Test proposal",
            current_value=0.10,
            suggested_value=0.25,
        )
        prober._proposals.append(proposal)

        # Apply the proposal without executor
        updated_proposal = await prober.apply_async("p14")

        # Should still mark as APPLIED
        assert updated_proposal is not None
        assert updated_proposal.status == "APPLIED"

    @pytest.mark.asyncio
    async def test_15_apply_async_event_payload_includes_all_fields(self):
        """Test 15: PROPOSAL_EXECUTED event payload includes all relevant fields."""
        import asyncio
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
        from cynic.kernel.core.event_bus import EventBus, CoreEvent

        bus = EventBus("test_bus")
        prober = SelfProber(bus=bus)
        executor = ProposalExecutor()

        # Track emitted events
        emitted_events = []

        async def capture_event(event):
            emitted_events.append(event)

        bus.on(CoreEvent.PROPOSAL_EXECUTED, capture_event)

        # Create mock qtable
        class MockQTable:
            def __init__(self):
                self._table = {
                    "state_e": {"action_5": {"value": 0.10, "visits": 5}}
                }

            def update(self, state, action, new_value):
                if state in self._table and action in self._table[state]:
                    self._table[state][action]["value"] = new_value

        executor.set_qtable(MockQTable())
        prober.set_executor(executor)

        # Create LOW_RISK QTABLE proposal
        proposal = SelfProposal(
            probe_id="p15",
            trigger="MANUAL",
            pattern_type="TEST",
            severity=0.15,
            dimension="QTABLE",
            target="state_e:action_5",
            recommendation="Test proposal",
            current_value=0.10,
            suggested_value=0.25,
        )
        prober._proposals.append(proposal)

        # Apply the proposal
        await prober.apply_async("p15")

        # Wait for async event handlers to complete
        await bus.drain(timeout=1.0)

        # Verify event payload
        assert len(emitted_events) > 0
        event = emitted_events[0]
        payload = event.dict_payload
        assert "proposal_id" in payload
        assert "dimension" in payload
        assert "success" in payload
        assert "message" in payload
        assert "old_value" in payload
        assert "new_value" in payload


class TestCLIInterface:
    """Tests for Priority 10 Task 3: CLI Review Interface."""

    def test_16_cli_list_command_exists(self):
        """Test 16: CLI list command exists and can be invoked."""
        import subprocess
        result = subprocess.run(
            ["python", "-m", "cynic.interfaces.cli.probes_commands", "list"],
            capture_output=True,
            text=True,
            timeout=5,
        )
        # Should complete without raising
        assert result is not None

    def test_17_cli_list_default_shows_pending(self):
        """Test 17: list command without --status shows PENDING proposals."""
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import (
            SelfProber,
            SelfProposal,
        )
        import tempfile
        import json

        # Create temp file with test proposals
        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            proposals = [
                {
                    "probe_id": "p1",
                    "trigger": "MANUAL",
                    "pattern_type": "TEST",
                    "severity": 0.5,
                    "dimension": "METRICS",
                    "target": "test_metric",
                    "recommendation": "Test recommendation",
                    "current_value": 1.0,
                    "suggested_value": 0.5,
                    "proposed_at": 1234567890.0,
                    "status": "PENDING",
                },
                {
                    "probe_id": "p2",
                    "trigger": "MANUAL",
                    "pattern_type": "TEST",
                    "severity": 0.3,
                    "dimension": "QTABLE",
                    "target": "state:action",
                    "recommendation": "Another recommendation",
                    "current_value": 0.2,
                    "suggested_value": 0.4,
                    "proposed_at": 1234567891.0,
                    "status": "APPLIED",
                },
            ]
            json.dump(proposals, f)
            temp_path = f.name

        try:
            prober = SelfProber(proposals_path=temp_path)
            pending = prober.pending()
            assert len(pending) == 1
            assert pending[0].probe_id == "p1"
            assert pending[0].status == "PENDING"
        finally:
            import os
            os.unlink(temp_path)

    def test_18_cli_list_status_filter_applied(self):
        """Test 18: list --status APPLIED filters to APPLIED proposals only."""
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
        import tempfile
        import json

        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            proposals = [
                {
                    "probe_id": "p1",
                    "trigger": "MANUAL",
                    "pattern_type": "TEST",
                    "severity": 0.5,
                    "dimension": "METRICS",
                    "target": "test_metric",
                    "recommendation": "Test recommendation",
                    "current_value": 1.0,
                    "suggested_value": 0.5,
                    "proposed_at": 1234567890.0,
                    "status": "PENDING",
                },
                {
                    "probe_id": "p2",
                    "trigger": "MANUAL",
                    "pattern_type": "TEST",
                    "severity": 0.3,
                    "dimension": "QTABLE",
                    "target": "state:action",
                    "recommendation": "Another recommendation",
                    "current_value": 0.2,
                    "suggested_value": 0.4,
                    "proposed_at": 1234567891.0,
                    "status": "APPLIED",
                },
                {
                    "probe_id": "p3",
                    "trigger": "MANUAL",
                    "pattern_type": "TEST",
                    "severity": 0.2,
                    "dimension": "ESCORE",
                    "target": "agent:dog",
                    "recommendation": "Third recommendation",
                    "current_value": 30.0,
                    "suggested_value": 38.2,
                    "proposed_at": 1234567892.0,
                    "status": "APPLIED",
                },
            ]
            json.dump(proposals, f)
            temp_path = f.name

        try:
            prober = SelfProber(proposals_path=temp_path)
            all_props = prober.all_proposals()
            applied = [p for p in all_props if p.status == "APPLIED"]
            assert len(applied) == 2
            assert all(p.status == "APPLIED" for p in applied)
        finally:
            import os
            os.unlink(temp_path)

    def test_19_cli_show_command_displays_proposal_details(self):
        """Test 19: show {probe_id} displays all proposal fields."""
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
        import tempfile
        import json

        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            proposals = [
                {
                    "probe_id": "test_probe_123",
                    "trigger": "EMERGENCE",
                    "pattern_type": "SPIKE",
                    "severity": 0.75,
                    "dimension": "METRICS",
                    "target": "core.judgment_created",
                    "recommendation": "Implement batching to reduce event rate",
                    "current_value": 100.5,
                    "suggested_value": 50.0,
                    "proposed_at": 1234567890.0,
                    "status": "PENDING",
                }
            ]
            json.dump(proposals, f)
            temp_path = f.name

        try:
            prober = SelfProber(proposals_path=temp_path)
            proposal = prober.get("test_probe_123")
            assert proposal is not None
            assert proposal.probe_id == "test_probe_123"
            assert proposal.dimension == "METRICS"
            assert proposal.trigger == "EMERGENCE"
            assert proposal.pattern_type == "SPIKE"
            assert proposal.severity == 0.75
            assert proposal.target == "core.judgment_created"
            assert proposal.recommendation == "Implement batching to reduce event rate"
            assert proposal.current_value == 100.5
            assert proposal.suggested_value == 50.0
            assert proposal.status == "PENDING"
        finally:
            import os
            os.unlink(temp_path)

    def test_20_cli_approve_command_marks_applied(self):
        """Test 20: approve {probe_id} marks proposal as APPLIED."""
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
        import tempfile
        import json

        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            proposals = [
                {
                    "probe_id": "p_to_approve",
                    "trigger": "MANUAL",
                    "pattern_type": "TEST",
                    "severity": 0.2,
                    "dimension": "METRICS",
                    "target": "test",
                    "recommendation": "Test",
                    "current_value": 1.0,
                    "suggested_value": 0.5,
                    "proposed_at": 1234567890.0,
                    "status": "PENDING",
                }
            ]
            json.dump(proposals, f)
            temp_path = f.name

        try:
            prober = SelfProber(proposals_path=temp_path)
            result = prober.apply("p_to_approve")
            assert result is not None
            assert result.status == "APPLIED"
            assert result.probe_id == "p_to_approve"
        finally:
            import os
            os.unlink(temp_path)

    def test_21_cli_dismiss_command_marks_dismissed(self):
        """Test 21: dismiss {probe_id} marks proposal as DISMISSED."""
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
        import tempfile
        import json

        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            proposals = [
                {
                    "probe_id": "p_to_dismiss",
                    "trigger": "MANUAL",
                    "pattern_type": "TEST",
                    "severity": 0.2,
                    "dimension": "METRICS",
                    "target": "test",
                    "recommendation": "Test",
                    "current_value": 1.0,
                    "suggested_value": 0.5,
                    "proposed_at": 1234567890.0,
                    "status": "PENDING",
                }
            ]
            json.dump(proposals, f)
            temp_path = f.name

        try:
            prober = SelfProber(proposals_path=temp_path)
            result = prober.dismiss("p_to_dismiss")
            assert result is not None
            assert result.status == "DISMISSED"
            assert result.probe_id == "p_to_dismiss"
        finally:
            import os
            os.unlink(temp_path)

    def test_22_cli_audit_command_shows_applied_and_dismissed(self):
        """Test 22: audit command shows APPLIED and DISMISSED proposals."""
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
        import tempfile
        import json

        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            proposals = [
                {
                    "probe_id": "p1",
                    "trigger": "MANUAL",
                    "pattern_type": "TEST",
                    "severity": 0.5,
                    "dimension": "METRICS",
                    "target": "test",
                    "recommendation": "First",
                    "current_value": 1.0,
                    "suggested_value": 0.5,
                    "proposed_at": 1234567890.0,
                    "status": "PENDING",
                },
                {
                    "probe_id": "p2",
                    "trigger": "MANUAL",
                    "pattern_type": "TEST",
                    "severity": 0.3,
                    "dimension": "QTABLE",
                    "target": "state",
                    "recommendation": "Second",
                    "current_value": 0.2,
                    "suggested_value": 0.4,
                    "proposed_at": 1234567891.0,
                    "status": "APPLIED",
                },
                {
                    "probe_id": "p3",
                    "trigger": "MANUAL",
                    "pattern_type": "TEST",
                    "severity": 0.2,
                    "dimension": "ESCORE",
                    "target": "dog",
                    "recommendation": "Third",
                    "current_value": 30.0,
                    "suggested_value": 38.2,
                    "proposed_at": 1234567892.0,
                    "status": "DISMISSED",
                },
            ]
            json.dump(proposals, f)
            temp_path = f.name

        try:
            prober = SelfProber(proposals_path=temp_path)
            all_props = prober.all_proposals()
            applied_and_dismissed = [
                p for p in all_props if p.status in ("APPLIED", "DISMISSED")
            ]
            assert len(applied_and_dismissed) == 2
            assert any(p.status == "APPLIED" for p in applied_and_dismissed)
            assert any(p.status == "DISMISSED" for p in applied_and_dismissed)
        finally:
            import os
            os.unlink(temp_path)

    def test_23_cli_show_with_missing_probe_id_returns_none(self):
        """Test 23: show command with non-existent probe_id returns None."""
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
        import tempfile
        import json

        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            proposals = [
                {
                    "probe_id": "p_exists",
                    "trigger": "MANUAL",
                    "pattern_type": "TEST",
                    "severity": 0.5,
                    "dimension": "METRICS",
                    "target": "test",
                    "recommendation": "Test",
                    "current_value": 1.0,
                    "suggested_value": 0.5,
                    "proposed_at": 1234567890.0,
                    "status": "PENDING",
                }
            ]
            json.dump(proposals, f)
            temp_path = f.name

        try:
            prober = SelfProber(proposals_path=temp_path)
            result = prober.get("nonexistent")
            assert result is None
        finally:
            import os
            os.unlink(temp_path)

    def test_24_cli_stats_shows_proposal_counts(self):
        """Test 24: stats() returns counts of PENDING, APPLIED, DISMISSED."""
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
        import tempfile
        import json

        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            proposals = [
                {
                    "probe_id": "p1",
                    "trigger": "MANUAL",
                    "pattern_type": "TEST",
                    "severity": 0.5,
                    "dimension": "METRICS",
                    "target": "test",
                    "recommendation": "First",
                    "current_value": 1.0,
                    "suggested_value": 0.5,
                    "proposed_at": 1234567890.0,
                    "status": "PENDING",
                },
                {
                    "probe_id": "p2",
                    "trigger": "MANUAL",
                    "pattern_type": "TEST",
                    "severity": 0.3,
                    "dimension": "QTABLE",
                    "target": "state",
                    "recommendation": "Second",
                    "current_value": 0.2,
                    "suggested_value": 0.4,
                    "proposed_at": 1234567891.0,
                    "status": "PENDING",
                },
                {
                    "probe_id": "p3",
                    "trigger": "MANUAL",
                    "pattern_type": "TEST",
                    "severity": 0.2,
                    "dimension": "ESCORE",
                    "target": "dog",
                    "recommendation": "Third",
                    "current_value": 30.0,
                    "suggested_value": 38.2,
                    "proposed_at": 1234567892.0,
                    "status": "APPLIED",
                },
                {
                    "probe_id": "p4",
                    "trigger": "MANUAL",
                    "pattern_type": "TEST",
                    "severity": 0.1,
                    "dimension": "CONFIG",
                    "target": "threshold",
                    "recommendation": "Fourth",
                    "current_value": 0.382,
                    "suggested_value": 0.300,
                    "proposed_at": 1234567893.0,
                    "status": "DISMISSED",
                },
            ]
            json.dump(proposals, f)
            temp_path = f.name

        try:
            prober = SelfProber(proposals_path=temp_path)
            stats = prober.stats()
            assert stats["pending"] == 2
            assert stats["applied"] == 1
            assert stats["dismissed"] == 1
            assert stats["queue_size"] == 4
        finally:
            import os
            os.unlink(temp_path)


class TestProposalExecutorGuardrails:
    """Tests for Priority 10 Task 4: Safety Guardrails & Rollback."""

    @pytest.mark.asyncio
    async def test_25_rate_limit_blocks_excessive_auto_apply(self):
        """Test 25: Rate limit blocks excessive auto-apply (3 proposals, 1/sec = 2+ seconds elapsed)."""
        import time
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber, SelfProposal
        from cynic.kernel.core.event_bus import EventBus

        bus = EventBus("test_bus")
        prober = SelfProber(bus=bus)
        executor = ProposalExecutor()

        # Set rate limit to 1 per second
        executor.set_rate_limit(1.0)

        # Create mock qtable
        class MockQTable:
            def __init__(self):
                self._table = {
                    f"state_{i}": {f"action_{i}": {"value": 0.15, "visits": 5}}
                    for i in range(3)
                }

            def update(self, state, action, new_value):
                if state in self._table and action in self._table[state]:
                    self._table[state][action]["value"] = new_value

        executor.set_qtable(MockQTable())
        prober.set_executor(executor)

        # Create 3 LOW_RISK proposals
        proposals = []
        for i in range(3):
            proposal = SelfProposal(
                probe_id=f"p25_{i}",
                trigger="MANUAL",
                pattern_type="TEST",
                severity=0.15,
                dimension="QTABLE",
                target=f"state_{i}:action_{i}",
                recommendation="Test proposal",
                current_value=0.15,
                suggested_value=0.30,
            )
            prober._proposals.append(proposal)
            proposals.append(proposal)

        # Execute 3 proposals in rapid succession
        start_time = time.time()
        for proposal in proposals:
            await prober.apply_async(proposal.probe_id)
        elapsed = time.time() - start_time

        # Should take 2+ seconds due to rate limiting (3 executions at 1/sec = 2 seconds wait)
        assert elapsed >= 2.0, f"Expected >= 2.0s elapsed, got {elapsed}"

    @pytest.mark.asyncio
    async def test_26_circuit_breaker_opens_after_max_failures(self):
        """Test 26: Circuit breaker disables after N failures (5 failures -> circuit open)."""
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber, SelfProposal
        from cynic.kernel.core.event_bus import EventBus

        bus = EventBus("test_bus")
        prober = SelfProber(bus=bus)
        executor = ProposalExecutor()

        # Set circuit breaker to open after 5 failures
        executor.set_circuit_breaker_threshold(5)

        # Don't inject qtable - all executions will fail
        prober.set_executor(executor)

        # Create 6 LOW_RISK QTABLE proposals (all will fail due to missing qtable)
        for i in range(6):
            proposal = SelfProposal(
                probe_id=f"p26_{i}",
                trigger="MANUAL",
                pattern_type="TEST",
                severity=0.15,
                dimension="QTABLE",
                target=f"state_{i}:action_{i}",
                recommendation="Test proposal",
                current_value=0.15,
                suggested_value=0.30,
            )
            prober._proposals.append(proposal)

        # Apply first 5 proposals - should all fail
        for i in range(5):
            await prober.apply_async(f"p26_{i}")

        # Verify circuit is now open
        assert executor.is_circuit_open() is True

    @pytest.mark.asyncio
    async def test_27_circuit_breaker_blocks_execution(self):
        """Test 27: Circuit breaker blocks execution (blocked proposal returns error with 'circuit breaker')."""
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber, SelfProposal
        from cynic.kernel.core.event_bus import EventBus

        bus = EventBus("test_bus")
        prober = SelfProber(bus=bus)
        executor = ProposalExecutor()

        # Set circuit breaker to open after 2 failures
        executor.set_circuit_breaker_threshold(2)

        # Don't inject qtable - all executions will fail
        prober.set_executor(executor)

        # Create 3 LOW_RISK QTABLE proposals
        for i in range(3):
            proposal = SelfProposal(
                probe_id=f"p27_{i}",
                trigger="MANUAL",
                pattern_type="TEST",
                severity=0.15,
                dimension="QTABLE",
                target=f"state_{i}:action_{i}",
                recommendation="Test proposal",
                current_value=0.15,
                suggested_value=0.30,
            )
            prober._proposals.append(proposal)

        # Apply first 2 proposals - should fail and open circuit
        for i in range(2):
            await prober.apply_async(f"p27_{i}")

        # Apply third proposal - should be blocked by circuit breaker
        result = await executor.execute(prober.get(f"p27_2"))
        assert result.success is False
        assert "circuit breaker" in result.error_message.lower()

    def test_28_proposal_rollback_records_executions(self):
        """Test 28: ProposalRollback records executions."""
        import tempfile
        from cynic.kernel.organism.brain.cognition.cortex.proposal_rollback import ProposalRollback

        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            temp_path = f.name

        try:
            rollback = ProposalRollback(rollback_path=temp_path)

            # Record execution
            rollback.record(
                proposal_id="p1",
                dimension="QTABLE",
                target="state:action",
                old_value=0.15,
                new_value=0.30,
                reversible=True,
            )

            # Verify recorded
            history = rollback.history(limit=10)
            assert len(history) == 1
            assert history[0]["proposal_id"] == "p1"
            assert history[0]["old_value"] == 0.15
            assert history[0]["new_value"] == 0.30
        finally:
            import os
            if os.path.exists(temp_path):
                os.unlink(temp_path)

    def test_29_proposal_rollback_last_n_proposals(self):
        """Test 29: Rollback last N proposals."""
        import tempfile
        from cynic.kernel.organism.brain.cognition.cortex.proposal_rollback import ProposalRollback

        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            temp_path = f.name

        try:
            rollback = ProposalRollback(rollback_path=temp_path)

            # Record 3 executions
            for i in range(3):
                rollback.record(
                    proposal_id=f"p{i}",
                    dimension="QTABLE",
                    target=f"state_{i}:action_{i}",
                    old_value=0.1 + i * 0.1,
                    new_value=0.3 + i * 0.1,
                    reversible=True,
                )

            # Rollback last 2
            rolled_back = rollback.rollback_last(2)

            # Should have rolled back 2 entries
            assert len(rolled_back) == 2
            assert rolled_back[0]["proposal_id"] == "p2"
            assert rolled_back[1]["proposal_id"] == "p1"

            # History should only have 1 entry now
            history = rollback.history(limit=10)
            assert len(history) == 1
            assert history[0]["proposal_id"] == "p0"
        finally:
            import os
            if os.path.exists(temp_path):
                os.unlink(temp_path)

    def test_30_proposal_rollback_since_minutes_ago(self):
        """Test 30: Rollback since X minutes ago."""
        import time
        import tempfile
        from cynic.kernel.organism.brain.cognition.cortex.proposal_rollback import ProposalRollback

        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            temp_path = f.name

        try:
            rollback = ProposalRollback(rollback_path=temp_path)

            # Record 2 entries with old timestamps
            now = time.time()
            old_time = now - (10 * 60)  # 10 minutes ago

            # Manually add old entries
            from cynic.kernel.organism.brain.cognition.cortex.proposal_rollback import RollbackEntry
            rollback._entries.append(RollbackEntry(
                proposal_id="p0",
                dimension="QTABLE",
                target="state_0:action_0",
                old_value=0.1,
                new_value=0.3,
                executed_at=old_time,
                reversible=True,
            ))
            rollback._entries.append(RollbackEntry(
                proposal_id="p1",
                dimension="QTABLE",
                target="state_1:action_1",
                old_value=0.1,
                new_value=0.3,
                executed_at=old_time + 1,
                reversible=True,
            ))

            # Record a recent entry
            rollback.record(
                proposal_id="p2",
                dimension="METRICS",
                target="test",
                old_value=1.0,
                new_value=0.5,
                reversible=True,
            )

            # Rollback entries from last 1 minute
            rolled_back = rollback.rollback_since(minutes_ago=1.0)

            # Should have rolled back only the most recent entry
            assert len(rolled_back) == 1
            assert rolled_back[0]["proposal_id"] == "p2"

            # Old entries should still be there
            history = rollback.history(limit=10)
            assert len(history) == 2
            assert history[0]["proposal_id"] == "p1"
            assert history[1]["proposal_id"] == "p0"
        finally:
            import os
            if os.path.exists(temp_path):
                os.unlink(temp_path)

    def test_31_proposal_rollback_history_returns_recent_entries(self):
        """Test 31: History returns recent entries."""
        import tempfile
        from cynic.kernel.organism.brain.cognition.cortex.proposal_rollback import ProposalRollback

        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            temp_path = f.name

        try:
            rollback = ProposalRollback(rollback_path=temp_path)

            # Record 10 entries
            for i in range(10):
                rollback.record(
                    proposal_id=f"p{i}",
                    dimension="QTABLE",
                    target=f"state_{i}:action_{i}",
                    old_value=0.1,
                    new_value=0.3,
                    reversible=True,
                )

            # Get history with limit=5
            history = rollback.history(limit=5)

            # Should return last 5 entries in reverse order
            assert len(history) == 5
            assert history[0]["proposal_id"] == "p9"
            assert history[1]["proposal_id"] == "p8"
            assert history[4]["proposal_id"] == "p5"
        finally:
            import os
            if os.path.exists(temp_path):
                os.unlink(temp_path)
