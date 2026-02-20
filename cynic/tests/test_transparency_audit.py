"""
Tests for TransparencyAuditTrail guardrail.
"""
import pytest
import json
import tempfile
from pathlib import Path
from cynic.immune.transparency_audit import TransparencyAuditTrail, AuditRecord
from cynic.core.phi import fibonacci


class TestAuditRecordInit:
    def test_record_creation(self):
        record = AuditRecord(
            judgment_id="j123",
            verdict="BARK",
            confidence=0.5,
            q_score=75.0,
        )
        assert record.record_id is not None
        assert record.judgment_id == "j123"
        assert record.verdict == "BARK"
        assert record.confidence == 0.5
        assert record.q_score == 75.0
        assert record.alignment_approved is True
        assert record.action_executed is False
        assert record.success is False

    def test_record_to_dict(self):
        record = AuditRecord(verdict="GROWL", confidence=0.4, q_score=50.0)
        d = record.to_dict()
        assert isinstance(d, dict)
        assert d["verdict"] == "GROWL"
        assert d["confidence"] == 0.4

    def test_record_to_json(self):
        record = AuditRecord(verdict="WAG", confidence=0.3)
        json_str = record.to_json()
        parsed = json.loads(json_str)
        assert parsed["verdict"] == "WAG"

    def test_record_from_dict(self):
        data = {
            "record_id": "rec123",
            "judgment_id": "j123",
            "verdict": "HOWL",
            "confidence": 0.9,
            "q_score": 95.0,
            "timestamp": 1234567890.0,
            "alignment_violations": [],
            "alignment_approved": True,
            "recommended_action": "HOWL",
            "action_prompt": "Deploy",
            "action_executed": False,
            "execution_result": {},
            "execution_error": None,
            "success": False,
            "human_reviewed": False,
            "human_review_notes": "",
        }
        record = AuditRecord.from_dict(data)
        assert record.record_id == "rec123"
        assert record.verdict == "HOWL"


class TestAuditTrailInit:
    def test_init_creates_directory(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "subdir" / "audit.jsonl"
            trail = TransparencyAuditTrail(str(path))
            assert path.parent.exists()

    def test_init_empty_trail(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "audit.jsonl"
            trail = TransparencyAuditTrail(str(path))
            assert len(trail._records) == 0


class TestAuditTrailDecisionRecording:
    def test_record_decision(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            trail = TransparencyAuditTrail(str(Path(tmpdir) / "audit.jsonl"))

            record = trail.record_decision(
                judgment_id="j123",
                verdict="BARK",
                confidence=0.5,
                q_score=75.0,
            )

            assert record.judgment_id == "j123"
            assert record.verdict == "BARK"
            assert len(trail._records) == 1

    def test_record_alignment_check(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            trail = TransparencyAuditTrail(str(Path(tmpdir) / "audit.jsonl"))

            record = trail.record_decision("j123", "BARK", 0.5, 75.0)
            violations = [
                {"axiom": "VERIFY", "severity": "WARNING", "blocking": False}
            ]

            trail.record_alignment_check(record.record_id, violations)
            updated = trail._find_record(record.record_id)
            assert updated.alignment_violations == violations
            assert updated.alignment_approved is True

    def test_record_alignment_check_blocking(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            trail = TransparencyAuditTrail(str(Path(tmpdir) / "audit.jsonl"))

            record = trail.record_decision("j123", "BARK", 0.1, 20.0)
            violations = [
                {"axiom": "VERIFY", "severity": "CRITICAL", "blocking": True}
            ]

            trail.record_alignment_check(record.record_id, violations)
            updated = trail._find_record(record.record_id)
            assert updated.alignment_approved is False

    def test_record_decision_recommendation(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            trail = TransparencyAuditTrail(str(Path(tmpdir) / "audit.jsonl"))

            record = trail.record_decision("j123", "BARK", 0.5, 75.0)
            trail.record_decision_recommendation(
                record.record_id,
                recommended_action="BARK",
                action_prompt="Fix critical issue",
            )

            updated = trail._find_record(record.record_id)
            assert updated.recommended_action == "BARK"
            assert updated.action_prompt == "Fix critical issue"

    def test_record_execution_success(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            trail = TransparencyAuditTrail(str(Path(tmpdir) / "audit.jsonl"))

            record = trail.record_decision("j123", "BARK", 0.5, 75.0)
            result = {"execution_id": "e123", "output": "Issue fixed"}

            trail.record_execution(record.record_id, success=True, result=result)

            updated = trail._find_record(record.record_id)
            assert updated.action_executed is True
            assert updated.success is True
            assert updated.execution_result == result

    def test_record_execution_failure(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            trail = TransparencyAuditTrail(str(Path(tmpdir) / "audit.jsonl"))

            record = trail.record_decision("j123", "BARK", 0.5, 75.0)
            trail.record_execution(
                record.record_id,
                success=False,
                error="Connection timeout",
            )

            updated = trail._find_record(record.record_id)
            assert updated.action_executed is True
            assert updated.success is False
            assert updated.execution_error == "Connection timeout"

    def test_record_human_review_approved(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            trail = TransparencyAuditTrail(str(Path(tmpdir) / "audit.jsonl"))

            record = trail.record_decision("j123", "BARK", 0.5, 75.0)
            trail.record_human_review(
                record.record_id,
                approved=True,
                notes="Checked and verified",
            )

            updated = trail._find_record(record.record_id)
            assert updated.human_reviewed is True
            assert updated.success is True
            assert updated.human_review_notes == "Checked and verified"

    def test_record_human_review_rejected(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            trail = TransparencyAuditTrail(str(Path(tmpdir) / "audit.jsonl"))

            record = trail.record_decision("j123", "BARK", 0.5, 75.0)
            trail.record_human_review(
                record.record_id,
                approved=False,
                notes="Insufficient evidence",
            )

            updated = trail._find_record(record.record_id)
            assert updated.human_reviewed is True
            assert updated.success is False


class TestAuditTrailHistory:
    def test_get_decision_history(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            trail = TransparencyAuditTrail(str(Path(tmpdir) / "audit.jsonl"))

            trail.record_decision("j1", "BARK", 0.5, 75.0)
            trail.record_decision("j2", "GROWL", 0.4, 50.0)
            trail.record_decision("j3", "WAG", 0.3, 65.0)

            history = trail.get_decision_history()
            assert len(history) == 3
            # Most recent first
            assert history[0].judgment_id == "j3"

    def test_get_decision_history_filtered(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            trail = TransparencyAuditTrail(str(Path(tmpdir) / "audit.jsonl"))

            trail.record_decision("j1", "BARK", 0.5, 75.0)
            trail.record_decision("j2", "GROWL", 0.4, 50.0)
            trail.record_decision("j3", "BARK", 0.6, 80.0)

            barks = trail.get_decision_history(verdict="BARK")
            assert len(barks) == 2
            assert all(r.verdict == "BARK" for r in barks)

    def test_get_decision_history_limit(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            trail = TransparencyAuditTrail(str(Path(tmpdir) / "audit.jsonl"))

            for i in range(10):
                trail.record_decision(f"j{i}", "WAG", 0.3, 65.0)

            history = trail.get_decision_history(limit=5)
            assert len(history) == 5

    def test_get_blocked_decisions(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            trail = TransparencyAuditTrail(str(Path(tmpdir) / "audit.jsonl"))

            rec1 = trail.record_decision("j1", "BARK", 0.5, 75.0)
            rec2 = trail.record_decision("j2", "GROWL", 0.4, 50.0)

            # Block rec1
            trail.record_alignment_check(rec1.record_id, [{"blocking": True}])

            blocked = trail.get_blocked_decisions()
            assert len(blocked) == 1
            assert blocked[0].judgment_id == "j1"

    def test_get_failed_executions(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            trail = TransparencyAuditTrail(str(Path(tmpdir) / "audit.jsonl"))

            rec1 = trail.record_decision("j1", "BARK", 0.5, 75.0)
            rec2 = trail.record_decision("j2", "GROWL", 0.4, 50.0)

            trail.record_execution(rec1.record_id, success=False, error="Timeout")
            trail.record_execution(rec2.record_id, success=True)

            failed = trail.get_failed_executions()
            assert len(failed) == 1
            assert failed[0].judgment_id == "j1"


class TestAuditTrailStats:
    def test_stats_format(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            trail = TransparencyAuditTrail(str(Path(tmpdir) / "audit.jsonl"))

            trail.record_decision("j1", "BARK", 0.5, 75.0)
            trail.record_decision("j2", "GROWL", 0.4, 50.0)

            stats = trail.stats()
            assert "total_records" in stats
            assert "alignment_approved" in stats
            assert "executed" in stats
            assert "successful" in stats
            assert "human_reviewed" in stats
            assert "verdict_distribution" in stats
            assert stats["total_records"] == 2
            assert stats["verdict_distribution"]["BARK"] == 1
            assert stats["verdict_distribution"]["GROWL"] == 1


class TestAuditTrailPersistence:
    def test_save_and_load_from_disk(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "audit.jsonl"

            # Write records
            trail1 = TransparencyAuditTrail(str(path))
            trail1.record_decision("j1", "BARK", 0.5, 75.0)
            trail1.record_decision("j2", "GROWL", 0.4, 50.0)

            # Load records
            trail2 = TransparencyAuditTrail(str(path))
            assert len(trail2._records) == 2
            assert trail2._records[0].judgment_id == "j1"
            assert trail2._records[1].judgment_id == "j2"

    def test_rolling_cap_on_disk(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "audit.jsonl"
            trail = TransparencyAuditTrail(str(path))

            # Add more records than rolling cap
            for i in range(fibonacci(10) + 10):
                trail.record_decision(f"j{i}", "WAG", 0.3, 65.0)

            # Should be capped
            assert len(trail._records) <= fibonacci(10)

    def test_jsonl_format(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "audit.jsonl"
            trail = TransparencyAuditTrail(str(path))

            trail.record_decision("j1", "BARK", 0.5, 75.0)
            trail.record_decision("j2", "GROWL", 0.4, 50.0)

            # Verify JSONL format
            with open(path) as f:
                lines = f.readlines()
                assert len(lines) == 2
                for line in lines:
                    data = json.loads(line)
                    assert "verdict" in data
                    assert "confidence" in data
