"""
Tests for DecisionValidator integration layer.
"""
import pytest
from unittest.mock import MagicMock, AsyncMock
from cynic.cognition.cortex.decision_validator import (
    DecisionValidator,
    ValidatedDecision,
    BlockedDecision,
)
from cynic.immune.power_limiter import PowerLimiter
from cynic.immune.alignment_checker import AlignmentSafetyChecker
from cynic.immune.human_approval_gate import HumanApprovalGate
from cynic.immune.transparency_audit import TransparencyAuditTrail
import tempfile
from pathlib import Path


@pytest.fixture
def guardrails():
    """Create all guardrails for testing."""
    with tempfile.TemporaryDirectory() as tmpdir:
        power_limiter = PowerLimiter()
        alignment_checker = AlignmentSafetyChecker()
        human_gate = HumanApprovalGate(str(Path(tmpdir) / "approvals.jsonl"))
        audit_trail = TransparencyAuditTrail(str(Path(tmpdir) / "audit.jsonl"))

        yield {
            "power_limiter": power_limiter,
            "alignment_checker": alignment_checker,
            "human_gate": human_gate,
            "audit_trail": audit_trail,
        }


@pytest.fixture
def validator(guardrails):
    """Create validator with all guardrails."""
    return DecisionValidator(
        power_limiter=guardrails["power_limiter"],
        alignment_checker=guardrails["alignment_checker"],
        human_gate=guardrails["human_gate"],
        audit_trail=guardrails["audit_trail"],
    )


@pytest.fixture
def healthy_scheduler():
    """Mock scheduler with healthy metrics."""
    scheduler = MagicMock()
    scheduler._tasks = []
    queue1 = MagicMock()
    queue1.qsize.return_value = 2
    queue2 = MagicMock()
    queue2.qsize.return_value = 2
    scheduler._queues = {"REFLEX": queue1, "MICRO": queue2}
    return scheduler


@pytest.fixture
def overloaded_scheduler():
    """Mock scheduler with high load."""
    scheduler = MagicMock()
    scheduler._tasks = [MagicMock()] * 50
    queue1 = MagicMock()
    queue1.qsize.return_value = 100
    queue2 = MagicMock()
    queue2.qsize.return_value = 100
    scheduler._queues = {"REFLEX": queue1, "MICRO": queue2}
    return scheduler


class TestValidateDecisionPass:
    @pytest.mark.asyncio
    async def test_wag_decision_passes_all_checks(self, validator, healthy_scheduler):
        """WAG verdict with good confidence should pass."""
        decision = {
            "verdict": "WAG",
            "confidence": 0.5,
            "q_score": 70.0,
            "action_prompt": "Monitor progress",
            "judgment_id": "j123",
            "recommended_action": "WAG",
        }

        result = await validator.validate_decision(
            decision=decision,
            judgment=MagicMock(verdict="WAG"),
            recent_judgments=[],
            scheduler=healthy_scheduler,
        )

        assert isinstance(result, ValidatedDecision)
        assert result.verdict == "WAG"
        assert result.confidence == 0.5
        assert result.approved_by_human is False


class TestValidatePowerLimiterBlock:
    @pytest.mark.asyncio
    async def test_overloaded_system_blocks(self, validator, overloaded_scheduler):
        """Overloaded system should block decision."""
        # Patch PowerLimiter to return False (system overloaded)
        validator._power_limiter.check_available = MagicMock(return_value=False)
        validator._power_limiter.recommended_level = MagicMock()

        decision = {
            "verdict": "GROWL",
            "confidence": 0.4,
            "q_score": 50.0,
            "action_prompt": "Review issue",
            "judgment_id": "j123",
        }

        with pytest.raises(BlockedDecision) as exc_info:
            await validator.validate_decision(
                decision=decision,
                judgment=MagicMock(verdict="GROWL"),
                recent_judgments=[],
                scheduler=overloaded_scheduler,
            )

        assert exc_info.value.guardrail == "PowerLimiter"
        assert "overloaded" in exc_info.value.reason.lower()


class TestValidateAlignmentBlock:
    @pytest.mark.asyncio
    async def test_blocking_alignment_violations_block(self, validator, healthy_scheduler):
        """Blocking alignment violations should block decision."""
        from cynic.immune.alignment_checker import AlignmentViolation

        # Create blocking violation
        violation = AlignmentViolation(
            axiom="VERIFY",
            severity="CRITICAL",
            reason="BARK with low confidence",
            blocking=True,
            recommendation="Increase confidence or lower verdict",
        )

        validator._alignment_checker.check_alignment = MagicMock(
            return_value=[violation]
        )

        decision = {
            "verdict": "BARK",
            "confidence": 0.1,
            "q_score": 20.0,
            "action_prompt": "Fix critical issue",
            "judgment_id": "j123",
        }

        with pytest.raises(BlockedDecision) as exc_info:
            await validator.validate_decision(
                decision=decision,
                judgment=MagicMock(verdict="BARK"),
                recent_judgments=[],
                scheduler=healthy_scheduler,
            )

        assert exc_info.value.guardrail == "AlignmentChecker"
        assert "VERIFY" in exc_info.value.reason


class TestValidateHumanApprovalGate:
    @pytest.mark.asyncio
    async def test_bark_verdict_requires_human_approval(self, validator, healthy_scheduler):
        """BARK verdict should trigger human approval gate."""
        decision = {
            "verdict": "BARK",
            "confidence": 0.6,
            "q_score": 80.0,
            "action_prompt": "Fix critical issue",
            "judgment_id": "j123",
            "recommended_action": "BARK",
        }

        with pytest.raises(BlockedDecision) as exc_info:
            await validator.validate_decision(
                decision=decision,
                judgment=MagicMock(verdict="BARK"),
                recent_judgments=[],
                scheduler=healthy_scheduler,
            )

        assert exc_info.value.guardrail == "HumanApprovalGate"
        assert "approval" in exc_info.value.reason.lower()

    @pytest.mark.asyncio
    async def test_large_prompt_requires_approval(self, validator, healthy_scheduler):
        """Large action prompts should trigger approval gate."""
        decision = {
            "verdict": "WAG",
            "confidence": 0.5,
            "q_score": 70.0,
            "action_prompt": "x" * 1000,  # Large prompt
            "judgment_id": "j123",
            "recommended_action": "WAG",
        }

        with pytest.raises(BlockedDecision) as exc_info:
            await validator.validate_decision(
                decision=decision,
                judgment=MagicMock(verdict="WAG"),
                recent_judgments=[],
                scheduler=healthy_scheduler,
            )

        assert exc_info.value.guardrail == "HumanApprovalGate"


class TestValidationSequence:
    @pytest.mark.asyncio
    async def test_power_limiter_checked_first(self, validator, overloaded_scheduler):
        """PowerLimiter should be checked before alignment."""
        validator._power_limiter.check_available = MagicMock(return_value=False)
        validator._alignment_checker.check_alignment = MagicMock(
            side_effect=Exception("Should not be called")
        )

        decision = {
            "verdict": "WAG",
            "confidence": 0.5,
            "q_score": 70.0,
            "action_prompt": "Test",
            "judgment_id": "j123",
        }

        with pytest.raises(BlockedDecision):
            await validator.validate_decision(
                decision=decision,
                judgment=MagicMock(verdict="WAG"),
                recent_judgments=[],
                scheduler=overloaded_scheduler,
            )

        # Alignment checker should not be called
        validator._alignment_checker.check_alignment.assert_not_called()

    @pytest.mark.asyncio
    async def test_audit_trail_records_decision(self, validator, healthy_scheduler):
        """Audit trail should record the decision."""
        decision = {
            "verdict": "WAG",
            "confidence": 0.5,
            "q_score": 70.0,
            "action_prompt": "Monitor",
            "judgment_id": "j123",
            "recommended_action": "WAG",
        }

        await validator.validate_decision(
            decision=decision,
            judgment=MagicMock(verdict="WAG"),
            recent_judgments=[],
            scheduler=healthy_scheduler,
        )

        # Check audit trail has the record
        assert len(validator._audit_trail._records) == 1
        record = validator._audit_trail._records[0]
        assert record.verdict == "WAG"
        assert record.judgment_id == "j123"


class TestBlockedDecisionException:
    def test_blocked_decision_attributes(self):
        exc = BlockedDecision(
            reason="Test reason",
            guardrail="TestGuardrail",
            blocking_rule="Test rule",
            recommendation="Test recommendation",
        )

        assert exc.reason == "Test reason"
        assert exc.guardrail == "TestGuardrail"
        assert exc.blocking_rule == "Test rule"
        assert exc.recommendation == "Test recommendation"


class TestValidatorStats:
    @pytest.mark.asyncio
    async def test_stats_aggregates_all_guardrails(self, validator, healthy_scheduler):
        """Stats should aggregate all guardrail stats."""
        decision = {
            "verdict": "WAG",
            "confidence": 0.5,
            "q_score": 70.0,
            "action_prompt": "Monitor",
            "judgment_id": "j123",
            "recommended_action": "WAG",
        }

        await validator.validate_decision(
            decision=decision,
            judgment=MagicMock(verdict="WAG"),
            recent_judgments=[],
            scheduler=healthy_scheduler,
        )

        stats = validator.stats()

        assert "power_limiter" in stats
        assert "alignment_checker" in stats
        assert "human_gate" in stats
        assert "audit_trail" in stats

        assert stats["audit_trail"]["total_records"] == 1


class TestValidatedDecision:
    def test_validated_decision_creation(self):
        decision = ValidatedDecision(
            decision_id="d123",
            verdict="WAG",
            confidence=0.5,
            action_prompt="Test",
            approved_by_human=False,
            audit_record_id="a123",
        )

        assert decision.decision_id == "d123"
        assert decision.verdict == "WAG"
        assert decision.approved_by_human is False
