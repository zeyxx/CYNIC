"""
Tests for AlignmentSafetyChecker guardrail.
"""
import pytest
from unittest.mock import MagicMock
from cynic.immune.alignment_checker import AlignmentSafetyChecker, AlignmentViolation
from cynic.core.phi import fibonacci


class TestAlignmentCheckerInit:
    def test_init_default_window_size(self):
        checker = AlignmentSafetyChecker()
        assert checker._window_size == fibonacci(6)  # 8
        assert checker._recent_verdicts == []

    def test_init_custom_window_size(self):
        checker = AlignmentSafetyChecker(window_size=5)
        assert checker._window_size == 5

    def test_start_logging(self, caplog):
        import logging
        checker = AlignmentSafetyChecker()
        with caplog.at_level(logging.INFO):
            checker.start()
        assert "AlignmentSafetyChecker started" in caplog.text


class TestFidelityCheck:
    """FIDELITY: Detect contradictions with past judgments."""

    def test_no_violation_consistent_verdict(self):
        checker = AlignmentSafetyChecker()
        recent = [
            MagicMock(verdict="BARK"),
            MagicMock(verdict="BARK"),
            MagicMock(verdict="BARK"),
        ]
        decision = {"verdict": "BARK", "confidence": 0.5}

        violations = checker._check_fidelity("BARK", recent)
        assert len(violations) == 0

    def test_warning_one_contradiction(self):
        checker = AlignmentSafetyChecker()
        recent = [
            MagicMock(verdict="BARK"),
            MagicMock(verdict="WAG"),
            MagicMock(verdict="BARK"),
        ]

        violations = checker._check_fidelity("BARK", recent)
        assert len(violations) == 1
        assert violations[0].severity == "WARNING"
        assert violations[0].blocking is False

    def test_critical_multiple_contradictions(self):
        checker = AlignmentSafetyChecker()
        recent = [
            MagicMock(verdict="WAG"),
            MagicMock(verdict="GROWL"),
            MagicMock(verdict="HOWL"),
        ]

        violations = checker._check_fidelity("BARK", recent)
        assert len(violations) == 1
        assert violations[0].severity == "CRITICAL"
        assert violations[0].blocking is True

    def test_no_recent_judgments(self):
        checker = AlignmentSafetyChecker()
        violations = checker._check_fidelity("BARK", [])
        assert len(violations) == 0


class TestPhiBalanceCheck:
    """PHI: Detect unbalanced verdict distributions."""

    def test_balanced_verdicts(self):
        checker = AlignmentSafetyChecker(window_size=8)
        checker._recent_verdicts = ["BARK", "BARK", "GROWL", "GROWL", "WAG", "WAG", "HOWL", "HOWL"]

        violations = checker._check_phi_balance("GROWL")
        assert len(violations) == 0  # Balanced distribution

    def test_warning_too_many_barks(self):
        checker = AlignmentSafetyChecker(window_size=8)
        # F(6)=8 window, BARK limit = 8 * 0.382 = ~3
        checker._recent_verdicts = ["BARK", "BARK", "BARK", "BARK", "WAG", "WAG", "GROWL", "HOWL"]

        violations = checker._check_phi_balance("BARK")
        assert len(violations) == 1
        assert violations[0].axiom == "PHI"
        assert violations[0].severity == "WARNING"

    def test_empty_recent_verdicts(self):
        checker = AlignmentSafetyChecker()
        violations = checker._check_phi_balance("BARK")
        assert len(violations) == 0


class TestVerificationCheck:
    """VERIFY: Confidence must support decision impact."""

    def test_bark_with_high_confidence(self):
        checker = AlignmentSafetyChecker()
        violations = checker._check_verification("BARK", 0.5)
        assert len(violations) == 0  # 0.5 > 0.382

    def test_critical_bark_with_low_confidence(self):
        checker = AlignmentSafetyChecker()
        violations = checker._check_verification("BARK", 0.2)
        assert len(violations) == 1
        assert violations[0].severity == "CRITICAL"
        assert violations[0].blocking is True

    def test_growl_with_moderate_confidence(self):
        checker = AlignmentSafetyChecker()
        violations = checker._check_verification("GROWL", 0.4)
        assert len(violations) == 0

    def test_growl_with_low_confidence(self):
        checker = AlignmentSafetyChecker()
        violations = checker._check_verification("GROWL", 0.2)
        assert len(violations) == 1
        assert violations[0].severity == "WARNING"
        assert violations[0].blocking is False

    def test_wag_and_howl_no_confidence_requirement(self):
        checker = AlignmentSafetyChecker()
        assert len(checker._check_verification("WAG", 0.1)) == 0
        assert len(checker._check_verification("HOWL", 0.05)) == 0


class TestCultureCheck:
    """CULTURE: Detect novel violations of established patterns."""

    def test_novel_bark_after_consistent_wags(self):
        checker = AlignmentSafetyChecker()
        recent = [
            MagicMock(verdict="WAG"),
            MagicMock(verdict="WAG"),
            MagicMock(verdict="WAG"),
            MagicMock(verdict="WAG"),
            MagicMock(verdict="WAG"),
        ]

        violations = checker._check_culture("BARK", recent)
        assert len(violations) == 1
        assert violations[0].axiom == "CULTURE"
        assert violations[0].severity == "WARNING"

    def test_consistent_pattern_no_warning(self):
        checker = AlignmentSafetyChecker()
        recent = [
            MagicMock(verdict="BARK"),
            MagicMock(verdict="GROWL"),
            MagicMock(verdict="BARK"),
            MagicMock(verdict="GROWL"),
            MagicMock(verdict="BARK"),
        ]

        violations = checker._check_culture("BARK", recent)
        assert len(violations) == 0  # BARK seen before in pattern

    def test_novel_wag_no_warning(self):
        checker = AlignmentSafetyChecker()
        recent = [MagicMock(verdict="BARK")] * 5

        violations = checker._check_culture("WAG", recent)
        assert len(violations) == 0  # No warning for non-critical verdicts

    def test_empty_recent_judgments(self):
        checker = AlignmentSafetyChecker()
        violations = checker._check_culture("BARK", [])
        assert len(violations) == 0


class TestBurnCheck:
    """BURN: Action should be minimal and focused."""

    def test_prompt_too_long(self):
        checker = AlignmentSafetyChecker()
        decision = {"action_prompt": "x" * 1500, "recommended_action": "BARK"}

        violations = checker._check_burn(decision)
        assert len(violations) == 1
        assert violations[0].axiom == "BURN"
        assert "too long" in violations[0].reason

    def test_prompt_acceptable_length(self):
        checker = AlignmentSafetyChecker()
        decision = {"action_prompt": "x" * 500, "recommended_action": "BARK"}

        violations = checker._check_burn(decision)
        # Might have other violations, but not prompt length
        assert not any("too long" in v.reason for v in violations)

    def test_invalid_recommended_action(self):
        checker = AlignmentSafetyChecker()
        decision = {"action_prompt": "short", "recommended_action": "INVALID_ACTION"}

        violations = checker._check_burn(decision)
        assert any("not a standard verdict" in v.reason for v in violations)

    def test_valid_verdict_action(self):
        checker = AlignmentSafetyChecker()
        for verdict in ["BARK", "GROWL", "WAG", "HOWL"]:
            decision = {"action_prompt": "short", "recommended_action": verdict}
            violations = [v for v in checker._check_burn(decision) if "standard verdict" in v.reason]
            assert len(violations) == 0


class TestFullAlignmentCheck:
    """Integration tests for full alignment check."""

    def test_all_checks_pass(self):
        checker = AlignmentSafetyChecker()
        recent = [MagicMock(verdict="BARK")] * 3
        decision = {
            "verdict": "BARK",
            "confidence": 0.5,
            "q_value": 75.0,
            "action_prompt": "Fix critical issue: X",
            "recommended_action": "BARK",
        }

        violations = checker.check_alignment(None, decision, recent)
        blocking = [v for v in violations if v.blocking]
        assert len(blocking) == 0

    def test_multiple_violations(self):
        checker = AlignmentSafetyChecker()
        recent = [MagicMock(verdict="WAG")] * 5
        decision = {
            "verdict": "BARK",
            "confidence": 0.1,  # Too low
            "q_value": 75.0,
            "action_prompt": "x" * 1500,  # Too long
            "recommended_action": "INVALID",  # Invalid
        }

        violations = checker.check_alignment(None, decision, recent)
        assert any(v.blocking for v in violations)  # VERIFY violation is blocking
        assert len(violations) >= 3  # Multiple violations


class TestRecordVerdict:
    def test_record_single_verdict(self):
        checker = AlignmentSafetyChecker()
        checker.record_verdict("BARK")
        assert checker._recent_verdicts == ["BARK"]

    def test_window_size_limit(self):
        checker = AlignmentSafetyChecker(window_size=3)
        checker.record_verdict("BARK")
        checker.record_verdict("GROWL")
        checker.record_verdict("WAG")
        checker.record_verdict("HOWL")

        # Should only keep last 3
        assert len(checker._recent_verdicts) == 3
        assert checker._recent_verdicts == ["GROWL", "WAG", "HOWL"]


class TestStats:
    def test_stats_format(self):
        checker = AlignmentSafetyChecker()
        checker.record_verdict("BARK")
        checker.record_verdict("GROWL")

        stats = checker.stats()
        assert "recent_verdict_count" in stats
        assert "recent_verdicts" in stats
        assert "window_size" in stats
        assert stats["recent_verdict_count"] == 2
        assert stats["window_size"] == fibonacci(6)
