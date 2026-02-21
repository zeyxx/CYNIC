"""
Tests for HumanApprovalGate guardrail.
"""
import pytest
import json
import tempfile
from pathlib import Path
from cynic.immune.human_approval_gate import (
    HumanApprovalGate,
    ApprovalRequest,
    ApprovalStatus,
)
from cynic.core.phi import fibonacci


class TestApprovalRequestInit:
    def test_request_creation(self):
        request = ApprovalRequest(
            verdict="BARK",
            confidence=0.5,
            q_score=75.0,
            action_prompt="Fix critical issue",
        )
        assert request.request_id is not None
        assert request.verdict == "BARK"
        assert request.status == ApprovalStatus.PENDING

    def test_request_to_dict(self):
        request = ApprovalRequest(verdict="BARK")
        d = request.to_dict()
        assert isinstance(d, dict)
        assert d["verdict"] == "BARK"
        assert d["status"] == "pending"

    def test_request_to_json(self):
        request = ApprovalRequest(verdict="GROWL")
        json_str = request.to_json()
        parsed = json.loads(json_str)
        assert parsed["verdict"] == "GROWL"


class TestApprovalGateInit:
    def test_init_creates_directory(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "subdir" / "approvals.jsonl"
            gate = HumanApprovalGate(str(path))
            assert path.parent.exists()

    def test_init_empty_gate(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            gate = HumanApprovalGate(str(Path(tmpdir) / "approvals.jsonl"))
            assert len(gate._requests) == 0


class TestApprovalRequiresApproval:
    def test_bark_verdict_requires_approval(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            gate = HumanApprovalGate(str(Path(tmpdir) / "approvals.jsonl"))

            decision = {
                "verdict": "BARK",
                "confidence": 0.5,
                "action_prompt": "Fix issue",
            }

            assert gate.requires_approval(decision, []) is True

    def test_non_bark_verdict_no_approval(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            gate = HumanApprovalGate(str(Path(tmpdir) / "approvals.jsonl"))

            decision = {
                "verdict": "WAG",
                "confidence": 0.3,
                "action_prompt": "Short",
            }

            assert gate.requires_approval(decision, []) is False

    def test_large_action_prompt_requires_approval(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            gate = HumanApprovalGate(str(Path(tmpdir) / "approvals.jsonl"))

            decision = {
                "verdict": "WAG",
                "confidence": 0.3,
                "action_prompt": "x" * 1000,  # > 800 chars
            }

            assert gate.requires_approval(decision, []) is True

    def test_low_alignment_approval_rate(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            gate = HumanApprovalGate(str(Path(tmpdir) / "approvals.jsonl"))

            decision = {
                "verdict": "GROWL",
                "confidence": 0.4,
                "action_prompt": "Review",
            }

            violations = [
                {"blocking": True},
                {"blocking": True},
                {"blocking": True},  # 0/3 pass, < 50%
            ]

            assert gate.requires_approval(decision, violations) is True

    def test_consecutive_failures_trigger_approval(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            gate = HumanApprovalGate(str(Path(tmpdir) / "approvals.jsonl"))

            # Simulate failures
            gate._consecutive_failures = 2  # >= MAX_CONSECUTIVE_FAILURES

            decision = {
                "verdict": "WAG",
                "confidence": 0.3,
                "action_prompt": "Short",
            }

            assert gate.requires_approval(decision, []) is True


class TestApprovalCreation:
    def test_create_approval_request(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            gate = HumanApprovalGate(str(Path(tmpdir) / "approvals.jsonl"))

            request = gate.create_approval_request(
                record_id="rec123",
                verdict="BARK",
                confidence=0.5,
                q_score=75.0,
                action_prompt="Fix issue",
                reason="BARK verdict detected",
                risk_level="CRITICAL",
            )

            assert request.record_id == "rec123"
            assert request.verdict == "BARK"
            assert request.status == ApprovalStatus.PENDING
            assert request.risk_level == "CRITICAL"

    def test_create_request_persists(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "approvals.jsonl"
            gate = HumanApprovalGate(str(path))

            request = gate.create_approval_request(
                record_id="rec123",
                verdict="BARK",
                confidence=0.5,
                q_score=75.0,
                action_prompt="Fix",
                reason="Test",
            )

            # Verify saved to disk
            assert path.exists()
            with open(path) as f:
                data = json.loads(f.readline())
                assert data["record_id"] == "rec123"


class TestApprovalWorkflow:
    def test_approve_request(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            gate = HumanApprovalGate(str(Path(tmpdir) / "approvals.jsonl"))

            request = gate.create_approval_request(
                record_id="rec123",
                verdict="BARK",
                confidence=0.5,
                q_score=75.0,
                action_prompt="Fix",
                reason="Test",
            )

            updated = gate.approve_request(
                request.request_id,
                reviewer="alice",
                notes="Verified and safe",
            )

            assert updated.status == ApprovalStatus.APPROVED
            assert updated.human_reviewer == "alice"
            assert updated.approval_notes == "Verified and safe"

    def test_reject_request(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            gate = HumanApprovalGate(str(Path(tmpdir) / "approvals.jsonl"))

            request = gate.create_approval_request(
                record_id="rec123",
                verdict="BARK",
                confidence=0.1,
                q_score=20.0,
                action_prompt="Fix",
                reason="Low confidence",
            )

            updated = gate.reject_request(
                request.request_id,
                reviewer="bob",
                notes="Insufficient confidence",
            )

            assert updated.status == ApprovalStatus.REJECTED
            assert gate._consecutive_failures == 1

    def test_escalate_request(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            gate = HumanApprovalGate(str(Path(tmpdir) / "approvals.jsonl"))

            request = gate.create_approval_request(
                record_id="rec123",
                verdict="BARK",
                confidence=0.5,
                q_score=75.0,
                action_prompt="Fix",
                reason="Test",
            )

            updated = gate.escalate_request(
                request.request_id,
                reason="Needs security review",
            )

            assert updated.status == ApprovalStatus.ESCALATED
            assert "ESCALATED" in updated.approval_notes

    def test_get_approval_status(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            gate = HumanApprovalGate(str(Path(tmpdir) / "approvals.jsonl"))

            request = gate.create_approval_request(
                record_id="rec123",
                verdict="BARK",
                confidence=0.5,
                q_score=75.0,
                action_prompt="Fix",
                reason="Test",
            )

            status = gate.get_approval_status(request.request_id)
            assert status == ApprovalStatus.PENDING

            gate.approve_request(request.request_id, "alice")
            status = gate.get_approval_status(request.request_id)
            assert status == ApprovalStatus.APPROVED


class TestApprovalHistory:
    def test_get_pending_approvals(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            gate = HumanApprovalGate(str(Path(tmpdir) / "approvals.jsonl"))

            req1 = gate.create_approval_request(
                record_id="rec1",
                verdict="BARK",
                confidence=0.5,
                q_score=75.0,
                action_prompt="Fix",
                reason="Test",
            )
            req2 = gate.create_approval_request(
                record_id="rec2",
                verdict="BARK",
                confidence=0.5,
                q_score=75.0,
                action_prompt="Fix",
                reason="Test",
            )

            gate.approve_request(req1.request_id, "alice")

            pending = gate.get_pending_approvals()
            assert len(pending) == 1
            assert pending[0].record_id == "rec2"


class TestApprovalStats:
    def test_stats_format(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            gate = HumanApprovalGate(str(Path(tmpdir) / "approvals.jsonl"))

            req1 = gate.create_approval_request(
                record_id="rec1",
                verdict="BARK",
                confidence=0.5,
                q_score=75.0,
                action_prompt="Fix",
                reason="Test",
            )
            req2 = gate.create_approval_request(
                record_id="rec2",
                verdict="BARK",
                confidence=0.5,
                q_score=75.0,
                action_prompt="Fix",
                reason="Test",
            )

            gate.approve_request(req1.request_id, "alice")

            stats = gate.stats()
            assert stats["total_requests"] == 2
            assert stats["pending"] == 1
            assert stats["approved"] == 1
            assert stats["rejected"] == 0

    def test_approval_rate_calculation(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            gate = HumanApprovalGate(str(Path(tmpdir) / "approvals.jsonl"))

            for i in range(3):
                req = gate.create_approval_request(
                    record_id=f"rec{i}",
                    verdict="BARK",
                    confidence=0.5,
                    q_score=75.0,
                    action_prompt="Fix",
                    reason="Test",
                )
                if i < 2:
                    gate.approve_request(req.request_id, "alice")
                else:
                    gate.reject_request(req.request_id, "bob")

            stats = gate.stats()
            assert stats["approval_rate"] == pytest.approx(2 / 3, rel=0.01)


class TestApprovalPersistence:
    def test_save_and_load(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "approvals.jsonl"

            gate1 = HumanApprovalGate(str(path))
            req1 = gate1.create_approval_request(
                record_id="rec1",
                verdict="BARK",
                confidence=0.5,
                q_score=75.0,
                action_prompt="Fix",
                reason="Test",
            )

            gate2 = HumanApprovalGate(str(path))
            assert len(gate2._requests) == 1
            assert gate2._requests[0].record_id == "rec1"

    def test_rolling_cap(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            gate = HumanApprovalGate(str(Path(tmpdir) / "approvals.jsonl"))

            # Add more requests than rolling cap
            for i in range(fibonacci(9) + 10):
                gate.create_approval_request(
                    record_id=f"rec{i}",
                    verdict="BARK",
                    confidence=0.5,
                    q_score=75.0,
                    action_prompt="Fix",
                    reason="Test",
                )

            assert len(gate._requests) <= fibonacci(9)


class TestConsecutiveFailures:
    def test_record_execution_result_success(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            gate = HumanApprovalGate(str(Path(tmpdir) / "approvals.jsonl"))
            gate._consecutive_failures = 2

            req = gate.create_approval_request(
                record_id="rec1",
                verdict="BARK",
                confidence=0.5,
                q_score=75.0,
                action_prompt="Fix",
                reason="Test",
            )

            gate.record_execution_result(req.request_id, success=True)
            assert gate._consecutive_failures == 0

    def test_record_execution_result_failure(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            gate = HumanApprovalGate(str(Path(tmpdir) / "approvals.jsonl"))
            gate._consecutive_failures = 0

            req = gate.create_approval_request(
                record_id="rec1",
                verdict="BARK",
                confidence=0.5,
                q_score=75.0,
                action_prompt="Fix",
                reason="Test",
            )

            gate.record_execution_result(req.request_id, success=False)
            assert gate._consecutive_failures == 1
