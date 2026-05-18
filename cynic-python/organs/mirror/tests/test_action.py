"""Tests for action gate and task dispatch."""
from organs.mirror.action import ActionGate, ActionDecision
from organs.mirror.profile import BehavioralProfile, PHI_INV_SQ


def test_below_threshold_observe_only() -> None:
    gate = ActionGate()
    profile = BehavioralProfile.empty()
    profile.narrative_affinity = {"agent": 0.3}
    profile.observation_count = 10
    decision = gate.evaluate(profile, domain="x_curation")
    assert decision == ActionDecision.OBSERVE


def test_above_threshold_act() -> None:
    gate = ActionGate()
    profile = BehavioralProfile.empty()
    profile.narrative_affinity = {"agent": 0.8, "defi": 0.6}
    profile.author_affinity = {"alice": 0.7}
    profile.observation_count = 500
    profile.pattern_stability = {"narrative_affinity": 0.9, "author_affinity": 0.85}
    decision = gate.evaluate(profile, domain="x_curation")
    assert decision == ActionDecision.ACT


def test_build_task_content() -> None:
    gate = ActionGate()
    profile = BehavioralProfile.empty()
    profile.narrative_affinity = {"agent": 0.8, "defi": 0.3}
    profile.author_affinity = {"alice": 0.9, "bob": 0.2}
    content = gate.build_task_content(profile)
    assert "agent" in content
    assert "alice" in content


def test_unknown_domain_observe() -> None:
    gate = ActionGate()
    profile = BehavioralProfile.empty()
    profile.narrative_affinity = {"agent": 0.9}
    profile.observation_count = 1000
    decision = gate.evaluate(profile, domain="unknown_domain")
    assert decision == ActionDecision.OBSERVE


def test_empty_profile_observe() -> None:
    gate = ActionGate()
    profile = BehavioralProfile.empty()
    decision = gate.evaluate(profile, domain="x_curation")
    assert decision == ActionDecision.OBSERVE


def test_build_task_content_top3_ordering() -> None:
    gate = ActionGate()
    profile = BehavioralProfile.empty()
    profile.narrative_affinity = {"low": 0.1, "mid": 0.5, "high": 0.9, "extra": 0.3}
    profile.author_affinity = {}
    content = gate.build_task_content(profile)
    # Top 3 narratives by score: high, mid, extra
    assert "high" in content
    assert "mid" in content
    # "low" is 4th — should NOT appear in top-3
    assert "low" not in content


def test_build_task_content_confidence_format() -> None:
    gate = ActionGate()
    profile = BehavioralProfile.empty()
    # Confidence with 0 observations and default stability → 0.00
    content = gate.build_task_content(profile)
    assert "Pattern confidence: 0.00" in content


def test_action_decision_values() -> None:
    assert ActionDecision.OBSERVE.value == "observe"
    assert ActionDecision.ACT.value == "act"
