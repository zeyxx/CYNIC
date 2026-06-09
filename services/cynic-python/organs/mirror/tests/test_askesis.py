"""Tests for askesis insight generation."""
from __future__ import annotations

from organs.mirror.askesis import Insight, InsightType, generate_insight
from organs.mirror.profile import BehavioralProfile

# ProfileDelta: prefer learner.py when available; fall back to local stub.
try:
    from organs.mirror.learner import ProfileDelta
except ImportError:
    from dataclasses import dataclass

    @dataclass
    class ProfileDelta:  # type: ignore[no-redef]
        changed_fields: set
        max_confidence_change: float


# ---------------------------------------------------------------------------
# Basic contract tests
# ---------------------------------------------------------------------------


def test_temporal_insight() -> None:
    """Clear dominant peak → TEMPORAL insight mentioning the peak hour."""
    profile = BehavioralProfile.empty()
    profile.activity_hours = {21: 0.35, 22: 0.28, 14: 0.05, 10: 0.03}
    profile.observation_count = 200
    delta = ProfileDelta(changed_fields={"activity_hours"}, max_confidence_change=0.1)

    insight = generate_insight(profile, delta)

    assert insight is not None
    assert insight.insight_type == InsightType.TEMPORAL
    assert "21" in insight.message or "peak" in insight.message.lower()
    assert 0.0 < insight.confidence <= 0.618
    assert insight.observation_count == 200


def test_temporal_insight_includes_second_peak() -> None:
    """Second peak should appear in the message when present."""
    profile = BehavioralProfile.empty()
    profile.activity_hours = {21: 0.40, 14: 0.25, 10: 0.10}
    profile.observation_count = 100
    delta = ProfileDelta(changed_fields={"activity_hours"}, max_confidence_change=0.15)

    insight = generate_insight(profile, delta)

    assert insight is not None
    # Both top and second hour should appear somewhere in the message
    assert "21" in insight.message
    assert "14" in insight.message


def test_narrative_insight() -> None:
    """Skewed narrative affinity → CONTENT_PREFERENCE insight."""
    profile = BehavioralProfile.empty()
    profile.narrative_affinity = {"agent": 0.7, "defi": 0.1, "meme": 0.05}
    profile.observation_count = 150
    delta = ProfileDelta(changed_fields={"narrative_affinity"}, max_confidence_change=0.05)

    insight = generate_insight(profile, delta)

    assert insight is not None
    assert insight.insight_type == InsightType.CONTENT_PREFERENCE
    assert "agent" in insight.message
    assert 0.0 < insight.confidence <= 0.618


def test_narrative_insight_ratio_in_message() -> None:
    """Ratio between top and second narrative must appear in the message."""
    profile = BehavioralProfile.empty()
    profile.narrative_affinity = {"agent": 0.6, "defi": 0.2}
    profile.observation_count = 80
    delta = ProfileDelta(changed_fields={"narrative_affinity"}, max_confidence_change=0.08)

    insight = generate_insight(profile, delta)

    assert insight is not None
    # ratio = 0.6 / 0.2 = 3.0 → "3.0x"
    assert "3.0" in insight.message


def test_app_usage_insight() -> None:
    """app_time_distribution no longer triggers APP_USAGE (non-actionable).
    L3 builders (flow, spinner, switch, prod) replaced it."""
    profile = BehavioralProfile.empty()
    profile.app_time_distribution = {"vscode": 0.55, "browser": 0.30, "slack": 0.15}
    profile.observation_count = 60
    delta = ProfileDelta(changed_fields={"app_time_distribution"}, max_confidence_change=0.07)

    insight = generate_insight(profile, delta)

    # No builder registered for app_time_distribution anymore
    assert insight is None


def test_no_insight_on_small_delta() -> None:
    """Delta below significance threshold → None."""
    profile = BehavioralProfile.empty()
    profile.observation_count = 10
    delta = ProfileDelta(changed_fields=set(), max_confidence_change=0.001)

    insight = generate_insight(profile, delta)

    assert insight is None


def test_no_insight_on_empty_changed_fields_even_with_large_confidence_change() -> None:
    """Empty changed_fields → None even if max_confidence_change is large."""
    profile = BehavioralProfile.empty()
    profile.activity_hours = {21: 0.50}
    profile.observation_count = 500
    delta = ProfileDelta(changed_fields=set(), max_confidence_change=0.5)

    insight = generate_insight(profile, delta)

    assert insight is None


def test_no_insight_when_confidence_change_below_threshold() -> None:
    """Below-threshold confidence change → None even with changed_fields."""
    profile = BehavioralProfile.empty()
    profile.activity_hours = {21: 0.50}
    profile.observation_count = 100
    delta = ProfileDelta(changed_fields={"activity_hours"}, max_confidence_change=0.005)

    insight = generate_insight(profile, delta)

    assert insight is None


def test_temporal_insight_no_clear_peak() -> None:
    """Flat distribution with no peak → no temporal insight."""
    profile = BehavioralProfile.empty()
    # All hours at ~4% — no clear peak
    profile.activity_hours = {h: 0.04 for h in range(24)}
    profile.observation_count = 300
    delta = ProfileDelta(changed_fields={"activity_hours"}, max_confidence_change=0.1)

    insight = generate_insight(profile, delta)

    # No clear peak means no temporal insight should be generated
    assert insight is None


def test_narrative_insight_no_skew() -> None:
    """Nearly equal narratives → no content preference insight."""
    profile = BehavioralProfile.empty()
    profile.narrative_affinity = {"agent": 0.35, "defi": 0.33, "meme": 0.32}
    profile.observation_count = 200
    delta = ProfileDelta(changed_fields={"narrative_affinity"}, max_confidence_change=0.05)

    insight = generate_insight(profile, delta)

    assert insight is None


def test_strongest_signal_wins_when_multiple_fields_changed() -> None:
    """When multiple fields change, the highest-confidence insight is returned."""
    profile = BehavioralProfile.empty()
    # Strong temporal signal
    profile.activity_hours = {21: 0.60, 14: 0.10}
    # Weak (non-skewed) narrative signal
    profile.narrative_affinity = {"agent": 0.4, "defi": 0.38}
    # Weak app distribution (close to even — low dominance, lower stability)
    profile.app_time_distribution = {"vscode": 0.48, "browser": 0.52}
    profile.observation_count = 300

    delta = ProfileDelta(
        changed_fields={"activity_hours", "narrative_affinity", "app_time_distribution"},
        max_confidence_change=0.2,
    )

    insight = generate_insight(profile, delta)

    assert insight is not None
    # The temporal signal is dominant (0.60), so TEMPORAL should win
    assert insight.insight_type == InsightType.TEMPORAL


def test_insight_confidence_bounded_by_phi_inv() -> None:
    """Confidence must never exceed phi^-1 = 0.618 regardless of observation count."""
    profile = BehavioralProfile.empty()
    profile.activity_hours = {21: 0.99}
    profile.observation_count = 100_000
    delta = ProfileDelta(changed_fields={"activity_hours"}, max_confidence_change=0.5)

    insight = generate_insight(profile, delta)

    assert insight is not None
    assert insight.confidence <= 0.618034 + 1e-9
