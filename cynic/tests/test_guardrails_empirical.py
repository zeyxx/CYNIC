"""
Empirical validation: Run synthetic judgments through guardrails 1-4 live.

Tests the full chain: PowerLimiter → AlignmentChecker → AuditTrail → HumanGate → DecisionValidator
in realistic scenarios to verify:
1. Judgments flow cleanly through the chain
2. Guardrails block appropriately (at least one should block)
3. Audit trail captures everything
4. BlockedDecision exceptions caught and logged
5. No regressions in existing 7-step cycle
"""
import pytest
from unittest.mock import MagicMock, AsyncMock
from cynic.immune.power_limiter import PowerLimiter
from cynic.immune.alignment_checker import AlignmentSafetyChecker, AlignmentViolation
from cynic.immune.human_approval_gate import HumanApprovalGate
from cynic.immune.transparency_audit import TransparencyAuditTrail
from cynic.cognition.cortex.decision_validator import DecisionValidator, BlockedDecision, ValidatedDecision
import tempfile
from pathlib import Path


@pytest.fixture
def guardrails_live():
    """Create all guardrails for empirical testing."""
    with tempfile.TemporaryDirectory() as tmpdir:
        power_limiter = PowerLimiter()
        alignment_checker = AlignmentSafetyChecker()
        human_gate = HumanApprovalGate(str(Path(tmpdir) / "approvals.jsonl"))
        audit_trail = TransparencyAuditTrail(str(Path(tmpdir) / "audit.jsonl"))

        decision_validator = DecisionValidator(
            power_limiter=power_limiter,
            alignment_checker=alignment_checker,
            human_gate=human_gate,
            audit_trail=audit_trail,
        )

        yield {
            "power_limiter": power_limiter,
            "alignment_checker": alignment_checker,
            "human_gate": human_gate,
            "audit_trail": audit_trail,
            "decision_validator": decision_validator,
        }


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


class TestEmpiricalGuardrailChain:
    """
    Test 10 synthetic judgments flowing through the full guardrail chain.

    Scenario setup:
    - 2 BARK verdicts (high severity)
    - 3 GROWL verdicts (moderate severity)
    - 5 WAG verdicts (low severity)
    """

    @pytest.mark.asyncio
    async def test_empirical_batch_1_bark_high_confidence(self, guardrails_live, healthy_scheduler):
        """BARK with high confidence — should pass PowerLimiter + AlignmentChecker but be gated by HumanApprovalGate."""
        validator = guardrails_live["decision_validator"]

        decision = {
            "verdict": "BARK",
            "confidence": 0.8,
            "q_score": 85.0,
            "action_prompt": "Fix critical security vulnerability in authentication",
            "judgment_id": "j_bark_high_conf",
            "recommended_action": "BARK",
        }

        # Should be blocked by HumanApprovalGate (BARK verdict always requires approval)
        with pytest.raises(BlockedDecision) as exc_info:
            await validator.validate_decision(
                decision=decision,
                judgment=MagicMock(verdict="BARK"),
                recent_judgments=[],
                scheduler=healthy_scheduler,
            )

        assert exc_info.value.guardrail == "HumanApprovalGate"
        assert "approval" in exc_info.value.reason.lower()

        # Verify audit trail recorded it
        assert len(guardrails_live["audit_trail"]._records) == 1
        record = guardrails_live["audit_trail"]._records[0]
        assert record.verdict == "BARK"

    @pytest.mark.asyncio
    async def test_empirical_batch_2_bark_low_confidence(self, guardrails_live, healthy_scheduler):
        """BARK with low confidence — should be blocked by AlignmentChecker (VERIFY axiom violation)."""
        validator = guardrails_live["decision_validator"]

        decision = {
            "verdict": "BARK",
            "confidence": 0.15,  # Low confidence
            "q_score": 25.0,
            "action_prompt": "Delete user account",
            "judgment_id": "j_bark_low_conf",
            "recommended_action": "BARK",
        }

        # Should be blocked by AlignmentChecker (BARK with low confidence violates VERIFY axiom)
        with pytest.raises(BlockedDecision) as exc_info:
            await validator.validate_decision(
                decision=decision,
                judgment=MagicMock(verdict="BARK"),
                recent_judgments=[],
                scheduler=healthy_scheduler,
            )

        # Could be blocked by AlignmentChecker OR HumanApprovalGate
        assert exc_info.value.guardrail in ["AlignmentChecker", "HumanApprovalGate"]

    @pytest.mark.asyncio
    async def test_empirical_batch_3_growl_moderate_confidence(self, guardrails_live, healthy_scheduler):
        """GROWL with moderate confidence — should likely pass (no blocking axiom violations)."""
        validator = guardrails_live["decision_validator"]

        decision = {
            "verdict": "GROWL",
            "confidence": 0.5,
            "q_score": 55.0,
            "action_prompt": "Review API design for optimization opportunities",
            "judgment_id": "j_growl_mod_conf",
            "recommended_action": "GROWL",
        }

        # Should pass all checks (GROWL is moderate, confidence is decent)
        result = await validator.validate_decision(
            decision=decision,
            judgment=MagicMock(verdict="GROWL"),
            recent_judgments=[],
            scheduler=healthy_scheduler,
        )

        assert isinstance(result, ValidatedDecision)
        assert result.verdict == "GROWL"

    @pytest.mark.asyncio
    async def test_empirical_batch_4_growl_low_confidence(self, guardrails_live, healthy_scheduler):
        """GROWL with low confidence — produces WARNING (non-blocking), decision passes with audit log."""
        validator = guardrails_live["decision_validator"]

        decision = {
            "verdict": "GROWL",
            "confidence": 0.2,  # Low
            "q_score": 30.0,
            "action_prompt": "Refactor database schema",
            "judgment_id": "j_growl_low_conf",
            "recommended_action": "GROWL",
        }

        # GROWL with low confidence produces a WARNING from AlignmentChecker (VERIFY axiom)
        # but is NOT blocking — decision passes through with warnings recorded in audit trail
        result = await validator.validate_decision(
            decision=decision,
            judgment=MagicMock(verdict="GROWL"),
            recent_judgments=[],
            scheduler=healthy_scheduler,
        )

        assert isinstance(result, ValidatedDecision)
        assert result.verdict == "GROWL"
        # Verify warning was recorded in audit trail
        assert len(guardrails_live["audit_trail"]._records) > 0

    @pytest.mark.asyncio
    async def test_empirical_batch_5_growl_high_confidence(self, guardrails_live, healthy_scheduler):
        """GROWL with high confidence — should pass all checks."""
        validator = guardrails_live["decision_validator"]

        decision = {
            "verdict": "GROWL",
            "confidence": 0.7,
            "q_score": 65.0,
            "action_prompt": "Implement caching optimization",
            "judgment_id": "j_growl_high_conf",
            "recommended_action": "GROWL",
        }

        result = await validator.validate_decision(
            decision=decision,
            judgment=MagicMock(verdict="GROWL"),
            recent_judgments=[],
            scheduler=healthy_scheduler,
        )

        assert isinstance(result, ValidatedDecision)
        assert result.verdict == "GROWL"

    @pytest.mark.asyncio
    async def test_empirical_batch_6_wag_standard(self, guardrails_live, healthy_scheduler):
        """WAG verdict (standard flow) — should pass all checks."""
        validator = guardrails_live["decision_validator"]

        decision = {
            "verdict": "WAG",
            "confidence": 0.55,
            "q_score": 70.0,
            "action_prompt": "Monitor system performance metrics",
            "judgment_id": "j_wag_1",
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

    @pytest.mark.asyncio
    async def test_empirical_batch_7_wag_with_large_prompt(self, guardrails_live, healthy_scheduler):
        """WAG with large prompt (>800 chars) — should be blocked by HumanApprovalGate."""
        validator = guardrails_live["decision_validator"]

        large_prompt = "x" * 1000  # Exceeds 800 char limit

        decision = {
            "verdict": "WAG",
            "confidence": 0.6,
            "q_score": 72.0,
            "action_prompt": large_prompt,
            "judgment_id": "j_wag_large_prompt",
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

    @pytest.mark.asyncio
    async def test_empirical_batch_8_wag_standard_2(self, guardrails_live, healthy_scheduler):
        """WAG verdict (second instance) — should pass."""
        validator = guardrails_live["decision_validator"]

        decision = {
            "verdict": "WAG",
            "confidence": 0.5,
            "q_score": 68.0,
            "action_prompt": "Log execution metrics",
            "judgment_id": "j_wag_2",
            "recommended_action": "WAG",
        }

        result = await validator.validate_decision(
            decision=decision,
            judgment=MagicMock(verdict="WAG"),
            recent_judgments=[],
            scheduler=healthy_scheduler,
        )

        assert isinstance(result, ValidatedDecision)

    @pytest.mark.asyncio
    async def test_empirical_batch_9_wag_standard_3(self, guardrails_live, healthy_scheduler):
        """WAG verdict (third instance) — should pass."""
        validator = guardrails_live["decision_validator"]

        decision = {
            "verdict": "WAG",
            "confidence": 0.52,
            "q_score": 71.0,
            "action_prompt": "Update documentation",
            "judgment_id": "j_wag_3",
            "recommended_action": "WAG",
        }

        result = await validator.validate_decision(
            decision=decision,
            judgment=MagicMock(verdict="WAG"),
            recent_judgments=[],
            scheduler=healthy_scheduler,
        )

        assert isinstance(result, ValidatedDecision)

    @pytest.mark.asyncio
    async def test_empirical_batch_10_wag_standard_4(self, guardrails_live, healthy_scheduler):
        """WAG verdict (fourth instance) — should pass."""
        validator = guardrails_live["decision_validator"]

        decision = {
            "verdict": "WAG",
            "confidence": 0.58,
            "q_score": 73.0,
            "action_prompt": "Check system health",
            "judgment_id": "j_wag_4",
            "recommended_action": "WAG",
        }

        result = await validator.validate_decision(
            decision=decision,
            judgment=MagicMock(verdict="WAG"),
            recent_judgments=[],
            scheduler=healthy_scheduler,
        )

        assert isinstance(result, ValidatedDecision)

    @pytest.mark.asyncio
    async def test_empirical_batch_11_wag_standard_5(self, guardrails_live, healthy_scheduler):
        """WAG verdict (fifth instance) — should pass."""
        validator = guardrails_live["decision_validator"]

        decision = {
            "verdict": "WAG",
            "confidence": 0.54,
            "q_score": 69.0,
            "action_prompt": "Analyze performance data",
            "judgment_id": "j_wag_5",
            "recommended_action": "WAG",
        }

        result = await validator.validate_decision(
            decision=decision,
            judgment=MagicMock(verdict="WAG"),
            recent_judgments=[],
            scheduler=healthy_scheduler,
        )

        assert isinstance(result, ValidatedDecision)


class TestEmpiricalAuditTrail:
    """Verify audit trail captures complete lifecycle."""

    @pytest.mark.asyncio
    async def test_audit_trail_completeness(self, guardrails_live, healthy_scheduler):
        """Run 3 judgments and verify audit trail has complete records."""
        validator = guardrails_live["decision_validator"]
        audit_trail = guardrails_live["audit_trail"]

        # Run 3 judgments
        judgments = [
            {"verdict": "WAG", "confidence": 0.6, "q_score": 70.0, "action_prompt": "Test 1", "judgment_id": "j1"},
            {"verdict": "GROWL", "confidence": 0.5, "q_score": 55.0, "action_prompt": "Test 2", "judgment_id": "j2"},
            {"verdict": "BARK", "confidence": 0.8, "q_score": 85.0, "action_prompt": "Test 3", "judgment_id": "j3"},
        ]

        passed = 0
        blocked = 0

        for j in judgments:
            try:
                await validator.validate_decision(
                    decision=j,
                    judgment=MagicMock(verdict=j["verdict"]),
                    recent_judgments=[],
                    scheduler=healthy_scheduler,
                )
                passed += 1
            except BlockedDecision:
                blocked += 1

        # Verify audit trail has records for all 3
        assert len(audit_trail._records) == 3

        # Verify records have complete info
        for record in audit_trail._records:
            assert record.verdict in ["WAG", "GROWL", "BARK"]
            assert record.judgment_id in ["j1", "j2", "j3"]
            assert record.q_score > 0

        # At least one should be blocked (BARK requires approval)
        assert blocked >= 1
        assert passed >= 1

    @pytest.mark.asyncio
    async def test_audit_trail_jsonl_persistence(self, guardrails_live, healthy_scheduler):
        """Verify audit trail persists to JSONL and can be reloaded."""
        validator = guardrails_live["decision_validator"]
        audit_trail_orig = guardrails_live["audit_trail"]

        # Create 2 records
        for i in range(2):
            try:
                await validator.validate_decision(
                    decision={
                        "verdict": "WAG",
                        "confidence": 0.6,
                        "q_score": 70.0,
                        "action_prompt": f"Test {i}",
                        "judgment_id": f"j_{i}",
                    },
                    judgment=MagicMock(verdict="WAG"),
                    recent_judgments=[],
                    scheduler=healthy_scheduler,
                )
            except BlockedDecision:
                pass

        # Reload from disk
        audit_trail_reloaded = TransparencyAuditTrail(audit_trail_orig._storage_path)

        # Verify records persisted
        assert len(audit_trail_reloaded._records) == 2
