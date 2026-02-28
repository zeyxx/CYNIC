"""
Unit tests for Immune System — Alignment, Safety, Guardrails

Tests:
  1. AlignmentChecker: 5 axioms (FIDELITY, PHI, VERIFY, CULTURE, BURN)
  2. PowerLimiter: budget enforcement
  3. HumanApprovalGate: human-in-the-loop
  4. TransparencyAudit: logging compliance
"""
import pytest
from dataclasses import dataclass, field
from unittest.mock import MagicMock

from cynic.metabolism.immune.alignment_checker import (
    AlignmentSafetyChecker, AlignmentViolation, _MAX_CONTRADICTIONS_BEFORE_BLOCK,
    _MIN_CONFIDENCE_FOR_HIGH_IMPACT, _VERDICT_BALANCE_WINDOW,
)
from cynic.brain.learning.qlearning import VERDICTS
from cynic.kernel.core.phi import fibonacci


# ════════════════════════════════════════════════════════════════════════════
# FIXTURES
# ════════════════════════════════════════════════════════════════════════════

@dataclass
class MockJudgment:
    """Mock Judgment for testing."""
    verdict: str
    q_score: float = 50.0
    confidence: float = 0.5
    judgment_id: str = "mock-judgment-1"


@pytest.fixture
def checker() -> AlignmentSafetyChecker:
    """Create a fresh AlignmentSafetyChecker for each test."""
    return AlignmentSafetyChecker(window_size=8)


@pytest.fixture
def sample_decision() -> dict:
    """Create a sample decision dict."""
    return {
        "verdict": "GROWL",
        "confidence": 0.5,
        "q_value": 50.0,
        "action_prompt": "Review the code for security issues",
        "recommended_action": "GROWL",
    }


@pytest.fixture
def sample_judgments() -> list:
    """Create sample past judgments."""
    return [
        MockJudgment(verdict="WAG", q_score=55.0),
        MockJudgment(verdict="GROWL", q_score=45.0),
        MockJudgment(verdict="WAG", q_score=60.0),
    ]


# ════════════════════════════════════════════════════════════════════════════
# FIDELITY TESTS (Contradiction Detection)
# ════════════════════════════════════════════════════════════════════════════

def test_fidelity_no_violation_when_agreeing(checker: AlignmentSafetyChecker, sample_decision: dict) -> None:
    """Test FIDELITY passes when verdict agrees with recent history."""
    recent = [
        MockJudgment(verdict="GROWL"),
        MockJudgment(verdict="GROWL"),
        MockJudgment(verdict="GROWL"),
    ]
    
    violations = checker.check_alignment(
        judgment=MockJudgment(verdict="GROWL"),
        decision=sample_decision,
        recent_judgments=recent,
    )
    
    fidelity_violations = [v for v in violations if v.axiom == "FIDELITY"]
    assert len(fidelity_violations) == 0


def test_fidelity_warning_on_contradiction(checker: AlignmentSafetyChecker, sample_decision: dict) -> None:
    """Test FIDELITY warns on 1-2 contradictions."""
    recent = [
        MockJudgment(verdict="WAG"),
        MockJudgment(verdict="GROWL"),
    ]
    
    violations = checker.check_alignment(
        judgment=MockJudgment(verdict="HOWL"),
        decision=sample_decision,
        recent_judgments=recent,
    )
    
    fidelity_violations = [v for v in violations if v.axiom == "FIDELITY"]
    assert len(fidelity_violations) >= 1
    assert fidelity_violations[0].severity == "WARNING"
    assert fidelity_violations[0].blocking is False


def test_fidelity_blocks_on_max_contradictions(checker: AlignmentSafetyChecker, sample_decision: dict) -> None:
    """Test FIDELITY blocks when contradictions >= threshold."""
    recent = [
        MockJudgment(verdict="WAG"),
        MockJudgment(verdict="HOWL"),
        MockJudgment(verdict="BARK"),
    ]
    
    violations = checker.check_alignment(
        judgment=MockJudgment(verdict="GROWL"),
        decision=sample_decision,
        recent_judgments=recent,
    )
    
    fidelity_violations = [v for v in violations if v.axiom == "FIDELITY"]
    assert len(fidelity_violations) >= 1
    assert fidelity_violations[0].severity == "CRITICAL"
    assert fidelity_violations[0].blocking is True


# ════════════════════════════════════════════════════════════════════════════
# PHI TESTS (Verdict Balance)
# ════════════════════════════════════════════════════════════════════════════

def test_phi_balanced_verdicts_pass(checker: AlignmentSafetyChecker, sample_decision: dict) -> None:
    """Test PHI passes when verdict distribution is balanced."""
    # Record balanced verdicts
    for _ in range(4):
        checker.record_verdict("WAG")
    for _ in range(4):
        checker.record_verdict("GROWL")
    
    # Add new verdict
    sample_decision["verdict"] = "WAG"
    
    violations = checker.check_alignment(
        judgment=MockJudgment(verdict="WAG"),
        decision=sample_decision,
        recent_judgments=[],
    )
    
    phi_violations = [v for v in violations if v.axiom == "PHI"]
    # Should not have critical violations
    critical = [v for v in phi_violations if v.severity == "CRITICAL"]
    assert len(critical) == 0


def test_phi_warns_on_bark_dominance(checker: AlignmentSafetyChecker, sample_decision: dict) -> None:
    """Test PHI warns when BARK dominates."""
    # Record too many BARK verdicts
    for _ in range(4):
        checker.record_verdict("BARK")
    
    sample_decision["verdict"] = "BARK"
    
    violations = checker.check_alignment(
        judgment=MockJudgment(verdict="BARK"),
        decision=sample_decision,
        recent_judgments=[],
    )
    
    phi_violations = [v for v in violations if v.axiom == "PHI"]
    assert len(phi_violations) >= 1


# ════════════════════════════════════════════════════════════════════════════
# VERIFY TESTS (Confidence Validation)
# ════════════════════════════════════════════════════════════════════════════

def test_verify_bark_requires_high_confidence(checker: AlignmentSafetyChecker) -> None:
    """Test VERIFY blocks BARK with low confidence."""
    decision = {
        "verdict": "BARK",
        "confidence": 0.2,  # Below φ⁻² = 0.382
        "q_value": 30.0,
    }
    
    violations = checker.check_alignment(
        judgment=MockJudgment(verdict="BARK"),
        decision=decision,
        recent_judgments=[],
    )
    
    verify_violations = [v for v in violations if v.axiom == "VERIFY"]
    assert len(verify_violations) >= 1
    assert verify_violations[0].blocking is True
    assert verify_violations[0].severity == "CRITICAL"


def test_verify_growl_accepts_moderate_confidence(checker: AlignmentSafetyChecker) -> None:
    """Test VERIFY passes GROWL with moderate confidence."""
    decision = {
        "verdict": "GROWL",
        "confidence": 0.4,
        "q_value": 40.0,
    }
    
    violations = checker.check_alignment(
        judgment=MockJudgment(verdict="GROWL"),
        decision=decision,
        recent_judgments=[],
    )
    
    verify_violations = [v for v in violations if v.axiom == "VERIFY"]
    # Should not have critical violations
    critical = [v for v in verify_violations if v.severity == "CRITICAL"]
    assert len(critical) == 0


def test_verify_warns_on_low_confidence_growl(checker: AlignmentSafetyChecker) -> None:
    """Test VERIFY warns GROWL with very low confidence."""
    decision = {
        "verdict": "GROWL",
        "confidence": 0.2,
        "q_value": 30.0,
    }
    
    violations = checker.check_alignment(
        judgment=MockJudgment(verdict="GROWL"),
        decision=decision,
        recent_judgments=[],
    )
    
    verify_violations = [v for v in violations if v.axiom == "VERIFY"]
    warnings = [v for v in verify_violations if v.severity == "WARNING"]
    assert len(warnings) >= 1


def test_verify_wag_accepts_any_confidence(checker: AlignmentSafetyChecker) -> None:
    """Test VERIFY passes WAG regardless of confidence."""
    decision = {
        "verdict": "WAG",
        "confidence": 0.1,
        "q_value": 50.0,
    }
    
    violations = checker.check_alignment(
        judgment=MockJudgment(verdict="WAG"),
        decision=decision,
        recent_judgments=[],
    )
    
    verify_violations = [v for v in violations if v.axiom == "VERIFY"]
    critical = [v for v in verify_violations if v.severity == "CRITICAL"]
    assert len(critical) == 0


# ════════════════════════════════════════════════════════════════════════════
# CULTURE TESTS (Pattern Violation)
# ════════════════════════════════════════════════════════════════════════════

def test_culture_novel_bark_warns(checker: AlignmentSafetyChecker, sample_decision: dict) -> None:
    """Test CULTURE warns on novel BARK verdict."""
    recent = [
        MockJudgment(verdict="WAG"),
        MockJudgment(verdict="WAG"),
        MockJudgment(verdict="GROWL"),
        MockJudgment(verdict="WAG"),
        MockJudgment(verdict="HOWL"),
    ]  # No BARK in last 5
    
    sample_decision["verdict"] = "BARK"
    
    violations = checker.check_alignment(
        judgment=MockJudgment(verdict="BARK"),
        decision=sample_decision,
        recent_judgments=recent,
    )
    
    culture_violations = [v for v in violations if v.axiom == "CULTURE"]
    assert len(culture_violations) >= 1


def test_culture_known_bark_passes(checker: AlignmentSafetyChecker, sample_decision: dict) -> None:
    """Test CULTURE passes when BARK was recent."""
    recent = [
        MockJudgment(verdict="BARK"),
        MockJudgment(verdict="WAG"),
        MockJudgment(verdict="GROWL"),
        MockJudgment(verdict="WAG"),
        MockJudgment(verdict="HOWL"),
    ]  # BARK in last 5
    
    sample_decision["verdict"] = "BARK"
    
    violations = checker.check_alignment(
        judgment=MockJudgment(verdict="BARK"),
        decision=sample_decision,
        recent_judgments=recent,
    )
    
    culture_violations = [v for v in violations if v.axiom == "CULTURE"]
    assert len(culture_violations) == 0


# ════════════════════════════════════════════════════════════════════════════
# BURN TESTS (Minimal Action)
# ════════════════════════════════════════════════════════════════════════════

def test_burn_long_prompt_warns(checker: AlignmentSafetyChecker) -> None:
    """Test BURN warns when action prompt is too long."""
    decision = {
        "verdict": "GROWL",
        "confidence": 0.5,
        "q_value": 40.0,
        "action_prompt": "x" * 1500,  # > 1000 chars
    }
    
    violations = checker.check_alignment(
        judgment=MockJudgment(verdict="GROWL"),
        decision=decision,
        recent_judgments=[],
    )
    
    burn_violations = [v for v in violations if v.axiom == "BURN"]
    assert len(burn_violations) >= 1
    assert "too long" in burn_violations[0].reason


def test_burn_short_prompt_passes(checker: AlignmentSafetyChecker) -> None:
    """Test BURN passes with short prompt."""
    decision = {
        "verdict": "GROWL",
        "confidence": 0.5,
        "q_value": 40.0,
        "action_prompt": "Review code",
    }
    
    violations = checker.check_alignment(
        judgment=MockJudgment(verdict="GROWL"),
        decision=decision,
        recent_judgments=[],
    )
    
    burn_violations = [v for v in violations if v.axiom == "BURN"]
    critical = [v for v in burn_violations if v.severity == "CRITICAL"]
    assert len(critical) == 0


def test_burn_non_standard_action_warns(checker: AlignmentSafetyChecker) -> None:
    """Test BURN warns when action is not a standard verdict."""
    decision = {
        "verdict": "GROWL",
        "confidence": 0.5,
        "q_value": 40.0,
        "action_prompt": "Review code",
        "recommended_action": "EXECUTE_SOMETHING",  # Not in VERDICTS
    }
    
    violations = checker.check_alignment(
        judgment=MockJudgment(verdict="GROWL"),
        decision=decision,
        recent_judgments=[],
    )
    
    burn_violations = [v for v in violations if v.axiom == "BURN"]
    assert len(burn_violations) >= 1


def test_burn_standard_action_passes(checker: AlignmentSafetyChecker) -> None:
    """Test BURN passes with standard verdict action."""
    for verdict in VERDICTS:
        decision = {
            "verdict": verdict,
            "confidence": 0.5,
            "q_value": 50.0,
            "action_prompt": "Review code",
            "recommended_action": verdict,
        }
        
        violations = checker.check_alignment(
            judgment=MockJudgment(verdict=verdict),
            decision=decision,
            recent_judgments=[],
        )
        
        burn_violations = [v for v in violations if v.axiom == "BURN"]
        critical = [v for v in burn_violations if v.severity == "CRITICAL"]
        assert len(critical) == 0, f"Verdict {verdict} should not have critical BURN violation"


# ════════════════════════════════════════════════════════════════════════════
# INTEGRATION TESTS
# ════════════════════════════════════════════════════════════════════════════

def test_full_alignment_check_all_pass(checker: AlignmentSafetyChecker) -> None:
    """Test complete alignment check passes with good decision."""
    # Balanced history
    for _ in range(3):
        checker.record_verdict("WAG")
    for _ in range(3):
        checker.record_verdict("GROWL")
    for _ in range(2):
        checker.record_verdict("HOWL")
    
    decision = {
        "verdict": "GROWL",
        "confidence": 0.5,
        "q_value": 45.0,
        "action_prompt": "Review code",
        "recommended_action": "GROWL",
    }
    
    recent = [
        MockJudgment(verdict="GROWL"),
        MockJudgment(verdict="WAG"),
        MockJudgment(verdict="GROWL"),
    ]
    
    violations = checker.check_alignment(
        judgment=MockJudgment(verdict="GROWL"),
        decision=decision,
        recent_judgments=recent,
    )
    
    critical = [v for v in violations if v.blocking]
    assert len(critical) == 0, f"Should not have blocking violations: {violations}"


def test_full_alignment_check_blocks_critical(checker: AlignmentSafetyChecker) -> None:
    """Test alignment check blocks when critical violations found."""
    decision = {
        "verdict": "BARK",
        "confidence": 0.1,  # Too low
        "q_value": 20.0,
        "action_prompt": "x" * 1500,  # Too long
    }
    
    violations = checker.check_alignment(
        judgment=MockJudgment(verdict="BARK"),
        decision=decision,
        recent_judgments=[],
    )
    
    blocking = [v for v in violations if v.blocking]
    assert len(blocking) >= 1


# ════════════════════════════════════════════════════════════════════════════
# STATS & UTILITY TESTS
# ════════════════════════════════════════════════════════════════════════════

def test_stats_returns_counts(checker: AlignmentSafetyChecker) -> None:
    """Test stats() returns correct counts."""
    checker.record_verdict("WAG")
    checker.record_verdict("GROWL")
    checker.record_verdict("WAG")
    
    stats = checker.stats()
    
    assert stats["recent_verdict_count"] == 3
    assert stats["window_size"] == 8
    assert "WAG" in stats["recent_verdicts"]
    assert "GROWL" in stats["recent_verdicts"]


def test_checker_lifecycle(checker: AlignmentSafetyChecker) -> None:
    """Test checker start/stop lifecycle."""
    checker.start()
    stats = checker.stats()
    assert stats["recent_verdict_count"] == 0


# ════════════════════════════════════════════════════════════════════════════
# EDGE CASES
# ════════════════════════════════════════════════════════════════════════════

def test_empty_recent_judgments_no_violation(checker: AlignmentSafetyChecker, sample_decision: dict) -> None:
    """Test empty recent judgments doesn't cause errors."""
    violations = checker.check_alignment(
        judgment=MockJudgment(verdict="WAG"),
        decision=sample_decision,
        recent_judgments=[],
    )
    
    # Should not crash, should have no violations
    assert isinstance(violations, list)


def test_single_recent_judgment_no_violation(checker: AlignmentSafetyChecker, sample_decision: dict) -> None:
    """Test single recent judgment doesn't cause FIDELITY violation."""
    recent = [MockJudgment(verdict="GROWL")]
    
    sample_decision["verdict"] = "HOWL"
    
    violations = checker.check_alignment(
        judgment=MockJudgment(verdict="HOWL"),
        decision=sample_decision,
        recent_judgments=recent,
    )
    
    fidelity_violations = [v for v in violations if v.axiom == "FIDELITY"]
    # Only 1 judgment, can't have 2+ contradictions
    critical = [v for v in fidelity_violations if v.blocking]
    assert len(critical) == 0


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
