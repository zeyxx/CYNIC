"""
Tier 1 EXPERIMENTAL: Askesis Insight Generator — derive human-readable behavioral insights.

Research question: Can a profile delta produce actionable, calibrated insights?
Success condition: Insights are non-trivial (confidence > 0), falsifiable, and human-readable.
Timeline: 7-14 days. If not promoted to Tier 2 by 2026-06-01, delete.
Owned by: @T
Status: ACTIVE (started 2026-05-18)

Input contract: BehavioralProfile with populated activity_hours / narrative_affinity /
                app_time_distribution, and a ProfileDelta describing what changed.
Output guarantee: Insight or None. Confidence is bounded by phi^-1 (0.618).
Failure modes: Empty profile fields → no insight for that category.
Valid domains: Personal behavioral analysis (mirror organ).
"""
from __future__ import annotations

import math
from dataclasses import dataclass
from enum import Enum
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    pass

PHI_INV = 0.618034
_CONFIDENCE_HALF_LIFE = 31.77

# ---------------------------------------------------------------------------
# ProfileDelta — minimal local definition; will be replaced once learner.py
# is available. Import guard below keeps both paths clean.
# ---------------------------------------------------------------------------
try:
    from organs.mirror.learner import ProfileDelta  # type: ignore[import]
except ImportError:

    @dataclass
    class ProfileDelta:  # type: ignore[no-redef]
        """Minimal description of what changed between two profile snapshots.

        Input contract: changed_fields is a set of BehavioralProfile field names.
        Output guarantee: none — pure data container.
        Failure modes: empty changed_fields → generate_insight returns None.
        """

        changed_fields: set[str]
        max_confidence_change: float


from organs.mirror.profile import BehavioralProfile, feature_confidence


# ---------------------------------------------------------------------------
# Public types
# ---------------------------------------------------------------------------


class InsightType(Enum):
    """Categories of behavioral insight the mirror organ can generate."""

    TEMPORAL = "temporal"
    CONTENT_PREFERENCE = "content-preference"
    APP_USAGE = "app-usage"
    BLIND_SPOT = "blind-spot"


@dataclass
class Insight:
    """A calibrated, human-readable observation about a behavioral pattern.

    Input contract: produced only by generate_insight(); not instantiated directly.
    Output guarantee: confidence in (0, PHI_INV]; observation_count >= 0.
    Failure modes: N/A — dataclass, no logic.
    """

    insight_type: InsightType
    message: str
    confidence: float
    observation_count: int


# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------

_SIGNIFICANCE_THRESHOLD = 0.01  # min max_confidence_change to produce an insight
_SKEW_RATIO_MIN = 2.0           # top/second narrative ratio to be "skewed"
_PEAK_DOMINANCE_MIN = 0.20      # min fraction for a temporal peak to be "clear"


def _temporal_insight(profile: BehavioralProfile) -> Insight | None:
    """Build a TEMPORAL insight from activity_hours.

    Returns None if no clear peak exists (dominance < _PEAK_DOMINANCE_MIN).
    """
    hours = profile.activity_hours
    if not hours:
        return None

    sorted_hours = sorted(hours.items(), key=lambda kv: kv[1], reverse=True)
    top_hour, top_pct = sorted_hours[0]

    if top_pct < _PEAK_DOMINANCE_MIN:
        return None

    pct_int = round(top_pct * 100)

    if len(sorted_hours) >= 2:
        second_hour, _ = sorted_hours[1]
        msg = (
            f"Your peak activity is at {top_hour}h ({pct_int}% of events). "
            f"Second peak: {second_hour}h."
        )
    else:
        msg = f"Your peak activity is at {top_hour}h ({pct_int}% of events)."

    confidence = _bounded_confidence(profile.observation_count, stability=top_pct)
    return Insight(
        insight_type=InsightType.TEMPORAL,
        message=msg,
        confidence=confidence,
        observation_count=profile.observation_count,
    )


def _content_preference_insight(profile: BehavioralProfile) -> Insight | None:
    """Build a CONTENT_PREFERENCE insight from narrative_affinity.

    Returns None if the top narrative doesn't dominate by at least _SKEW_RATIO_MIN.
    """
    affinity = profile.narrative_affinity
    if len(affinity) < 2:
        return None

    sorted_narr = sorted(affinity.items(), key=lambda kv: kv[1], reverse=True)
    top_name, top_val = sorted_narr[0]
    second_name, second_val = sorted_narr[1]

    if second_val == 0.0:
        return None

    ratio = top_val / second_val
    if ratio < _SKEW_RATIO_MIN:
        return None

    ratio_fmt = f"{ratio:.1f}"
    msg = f"You engage {ratio_fmt}x more with '{top_name}' than '{second_name}'"

    confidence = _bounded_confidence(profile.observation_count, stability=min(ratio / 10.0, 1.0))
    return Insight(
        insight_type=InsightType.CONTENT_PREFERENCE,
        message=msg,
        confidence=confidence,
        observation_count=profile.observation_count,
    )


def _app_usage_insight(profile: BehavioralProfile) -> Insight | None:
    """Build an APP_USAGE insight from app_time_distribution."""
    dist = profile.app_time_distribution
    if not dist:
        return None

    sorted_apps = sorted(dist.items(), key=lambda kv: kv[1], reverse=True)
    top_app, top_fraction = sorted_apps[0]

    pct_int = round(top_fraction * 100)
    msg = f"You spend {pct_int}% of time in {top_app}"

    confidence = _bounded_confidence(profile.observation_count, stability=top_fraction)
    return Insight(
        insight_type=InsightType.APP_USAGE,
        message=msg,
        confidence=confidence,
        observation_count=profile.observation_count,
    )


def _bounded_confidence(n_observations: int, stability: float) -> float:
    """Exponential saturation toward PHI_INV, clamped to (0, PHI_INV].

    Delegates to profile.feature_confidence so the formula is a single source of truth.
    """
    raw = feature_confidence(n_observations, stability)
    # Ensure we never return 0.0 when there is at least one observation
    if raw == 0.0 and n_observations > 0:
        # floor: epsilon above zero when any data exists
        return min(PHI_INV, 0.001)
    return raw


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

_FIELD_TO_BUILDER = {
    "activity_hours": _temporal_insight,
    "narrative_affinity": _content_preference_insight,
    "app_time_distribution": _app_usage_insight,
}


def generate_insight(profile: BehavioralProfile, delta: ProfileDelta) -> Insight | None:
    """Derive the strongest actionable insight from a profile delta.

    Logic:
      1. Reject insignificant deltas (no changed_fields or tiny confidence shift).
      2. For each changed field with a registered builder, attempt to build an insight.
      3. Pick the insight with the highest confidence (strongest signal).
      4. Return None if no insight passes its internal threshold.

    Input contract:
      - profile: BehavioralProfile (may be partially populated).
      - delta: ProfileDelta with changed_fields and max_confidence_change.
    Output guarantee:
      - Returns Insight or None. Confidence bounded by phi^-1 (0.618).
    Failure modes:
      - delta insignificant → None.
      - All profile fields empty → None.
    """
    # Gate 1: insignificance
    if not delta.changed_fields or delta.max_confidence_change < _SIGNIFICANCE_THRESHOLD:
        return None

    candidates: list[Insight] = []

    for field_name in delta.changed_fields:
        builder = _FIELD_TO_BUILDER.get(field_name)
        if builder is None:
            continue
        insight = builder(profile)
        if insight is not None:
            candidates.append(insight)

    if not candidates:
        return None

    # Return the highest-confidence insight (strongest signal wins)
    return max(candidates, key=lambda ins: ins.confidence)
