"""
Tier 2 INFRASTRUCTURE: Askesis Insight Generator — derive human-readable behavioral insights.

K15 Consumer: kernel /observe receives insights, routed to human via session-init.
Systemd: mirror-agent.service (coordinator calls generate_insight per cycle).
Promotion date: 2026-05-20 (promoted from Tier 1 with L3 pattern integration).
Stability: active development.

Input contract: BehavioralProfile with L2/L3 fields populated, and a ProfileDelta.
Output guarantee: Insight or None. Confidence bounded by phi^-1 (0.618).
Failure modes: Empty profile fields → no insight for that category.
Valid domains: Personal behavioral analysis (mirror organ).
"""
from __future__ import annotations

import math
from dataclasses import dataclass, field
from enum import Enum
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    pass

PHI_INV = 0.618034
_CONFIDENCE_HALF_LIFE = 31.77

# ---------------------------------------------------------------------------
# ProfileDelta — minimal local definition; replaced when learner.py available.
# ---------------------------------------------------------------------------
try:
    from organs.mirror.learner import ProfileDelta  # type: ignore[import]
except ImportError:

    @dataclass
    class ProfileDelta:  # type: ignore[no-redef]
        changed_fields: set[str] = field(default_factory=set)
        max_confidence_change: float = 0.0


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
    FLOW_STATE = "flow-state"
    SPINNER_RATIO = "spinner-ratio"
    SWITCH_ANOMALY = "switch-anomaly"
    PROD_SHIFT = "prod-shift"


@dataclass
class Insight:
    """A calibrated, human-readable observation about a behavioral pattern.

    Output guarantee: confidence in (0, PHI_INV]; surprise in [0, 1].
    Score = confidence * max(surprise, 0.1) — surprising insights rank higher.
    """

    insight_type: InsightType
    message: str
    confidence: float
    observation_count: int
    surprise: float = 0.0

    @property
    def score(self) -> float:
        """Ranking score: confidence weighted by surprise."""
        return self.confidence * max(self.surprise, 0.1)


# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------

_SIGNIFICANCE_THRESHOLD = 0.01
_SKEW_RATIO_MIN = 2.0
_PEAK_DOMINANCE_MIN = 0.20
_SWITCH_ANOMALY_FACTOR = 2.5  # current rate must exceed baseline by this factor
_SPINNER_BASELINE = 0.75      # observed 75/25 human/AI split from 24h data
_SPINNER_DEVIATION_MIN = 0.15 # min deviation from baseline to trigger


def _temporal_insight(profile: BehavioralProfile) -> Insight | None:
    """Build a TEMPORAL insight from activity_hours."""
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
    """Build a CONTENT_PREFERENCE insight from narrative_affinity."""
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


# ---------------------------------------------------------------------------
# L3 insight builders
# ---------------------------------------------------------------------------


def _flow_state_insight(profile: BehavioralProfile) -> Insight | None:
    """Report flow blocks detected — deep work sessions with sustained focus.

    Reads session_duration_dist (populated by PatternDetector) to contextualize
    flow block duration against the user's typical session length.
    """
    dwell = profile.dwell_by_content_type
    if not dwell:
        return None

    # Find the longest ✳ (user typing) context by dwell weight
    flow_contexts = []
    for key, weight in dwell.items():
        if ":✳" in key and weight > 0.01:
            flow_contexts.append((key, weight))

    if not flow_contexts:
        return None

    flow_contexts.sort(key=lambda x: -x[1])
    top_key, top_weight = flow_contexts[0]

    # Extract task name from key (format: "Gnome-terminal:✳ Task Name")
    task_name = top_key.split(":✳ ", 1)[1] if ":✳ " in top_key else top_key

    # Median session duration for context
    median_min = profile.session_duration_dist.p50 / 60.0 if profile.session_duration_dist.n > 0 else 0.0

    # Surprise: how much larger is this than median
    surprise = min(top_weight / (sum(dwell.values()) + 0.001), 1.0)

    stability = profile.pattern_stability.get("prod_ratio", 0.5)
    confidence = _bounded_confidence(profile.observation_count, stability=stability)

    msg = f"Deep work detected on '{task_name}'. "
    if median_min > 0:
        msg += f"Your median session: {median_min:.0f}min. "
    msg += f"Flow is rare — protect these blocks."

    return Insight(
        insight_type=InsightType.FLOW_STATE,
        message=msg,
        confidence=confidence,
        observation_count=profile.observation_count,
        surprise=surprise,
    )


def _spinner_ratio_insight(profile: BehavioralProfile) -> Insight | None:
    """Report human vs AI activity ratio in terminal sessions.

    Reads dwell_by_content_type, groups by spinner state (✳ vs ⠂/⠐).
    """
    dwell = profile.dwell_by_content_type
    if not dwell:
        return None

    human_total = 0.0
    ai_total = 0.0
    for key, weight in dwell.items():
        if not key.startswith("Gnome-terminal:"):
            continue
        sub = key.split(":", 1)[1] if ":" in key else ""
        if not sub:
            continue
        first_char = sub[0]
        if first_char == "✳":
            human_total += weight
        elif first_char in "⠂⠐⠠⠄⠈⠁⠃⠇⠋⠉⠙⠸⠴⠦⠖⠒⠓⠏⠟⠻⠽⠾⠷⠿":
            ai_total += weight

    total = human_total + ai_total
    if total < 0.01:
        return None

    ratio = human_total / total
    deviation = abs(ratio - _SPINNER_BASELINE)
    if deviation < _SPINNER_DEVIATION_MIN:
        return None

    surprise = min(deviation / _SPINNER_BASELINE, 1.0)
    human_pct = round(ratio * 100)
    ai_pct = 100 - human_pct

    if ratio > _SPINNER_BASELINE:
        msg = f"You typed {human_pct}% of terminal time (AI ran {ai_pct}%). Above your baseline of 75/25. Heavy prompting session."
    else:
        msg = f"AI ran {ai_pct}% of terminal time (you typed {human_pct}%). Below your baseline of 75/25. Delegation-heavy session."

    stability = profile.pattern_stability.get("human_ai_ratio", 0.5)
    confidence = _bounded_confidence(profile.observation_count, stability=stability)

    return Insight(
        insight_type=InsightType.SPINNER_RATIO,
        message=msg,
        confidence=confidence,
        observation_count=profile.observation_count,
        surprise=surprise,
    )


def _switch_anomaly_insight(profile: BehavioralProfile) -> Insight | None:
    """Report abnormal context switching rate.

    Reads context_switch_rate (per-hour, set by PatternDetector) and
    pattern_stability['switch_rate'] for baseline comparison.
    """
    rate = profile.context_switch_rate
    if rate < 1.0:
        return None

    stability = profile.pattern_stability.get("switch_rate", 0.5)
    # Baseline: infer from stability. High stability → rate is consistent → baseline ≈ rate.
    # Low stability → rate varies → current might be anomalous.
    # Without a stored baseline, use the rate itself as a signal when stability is low.
    if stability > 0.7:
        # Stable switching pattern — not anomalous
        return None

    # Low stability + high rate = scanning mode
    if rate < 20:
        return None  # 20 switches/h is not alarming

    surprise = min((1.0 - stability), 1.0)

    msg = (
        f"High context switching: {rate:.0f} switches this hour. "
        f"Switch rate stability: {stability:.2f} (unstable). "
        f"Consider a 25-min single-task block."
    )

    confidence = _bounded_confidence(profile.observation_count, stability=max(stability, 0.3))

    return Insight(
        insight_type=InsightType.SWITCH_ANOMALY,
        message=msg,
        confidence=confidence,
        observation_count=profile.observation_count,
        surprise=surprise,
    )


def _prod_shift_insight(profile: BehavioralProfile) -> Insight | None:
    """Report a shift in production ratio (typing vs consuming).

    Reads coding_vs_browsing_ratio and pattern_stability['prod_ratio'].
    """
    ratio = profile.coding_vs_browsing_ratio
    stability = profile.pattern_stability.get("prod_ratio", 0.5)

    # Only trigger when production ratio is meaningfully measurable
    if ratio == 0.0 and stability == 0.5:
        return None

    # High stability = no shift detected
    if stability > 0.6:
        return None

    surprise = min(1.0 - stability, 1.0)
    pct = round(ratio * 100)

    if ratio < 0.25:
        msg = f"Production ratio: {pct}% (mostly consuming/reading). Stability: {stability:.2f} — shifting pattern."
    elif ratio > 0.45:
        msg = f"Production ratio: {pct}% (heavy writing session). Stability: {stability:.2f} — above your norm."
    else:
        msg = f"Production ratio: {pct}% (balanced). Stability: {stability:.2f} — pattern is shifting."

    confidence = _bounded_confidence(profile.observation_count, stability=max(stability, 0.3))

    return Insight(
        insight_type=InsightType.PROD_SHIFT,
        message=msg,
        confidence=confidence,
        observation_count=profile.observation_count,
        surprise=surprise,
    )


def _bounded_confidence(n_observations: int, stability: float) -> float:
    """Exponential saturation toward PHI_INV, clamped to (0, PHI_INV]."""
    raw = feature_confidence(n_observations, stability)
    if raw == 0.0 and n_observations > 0:
        return min(PHI_INV, 0.001)
    return raw


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

_FIELD_TO_BUILDER = {
    "activity_hours": _temporal_insight,
    "narrative_affinity": _content_preference_insight,
    # L3 builders — triggered by PatternDetector.update_profile() fields
    "session_duration_dist": _flow_state_insight,
    "dwell_by_content_type": _spinner_ratio_insight,
    "context_switch_rate": _switch_anomaly_insight,
    "coding_vs_browsing_ratio": _prod_shift_insight,
}


def generate_insight(
    profile: BehavioralProfile,
    delta: ProfileDelta,
    force: bool = False,
) -> Insight | None:
    """Derive the strongest actionable insight from a profile delta.

    Args:
        force: bypass the significance threshold (for L3 pattern events).
               Still respects the 3/day cap via the coordinator.

    Ranking: confidence * max(surprise, 0.1) — surprising insights win.
    """
    if not force:
        if not delta.changed_fields or delta.max_confidence_change < _SIGNIFICANCE_THRESHOLD:
            return None
    else:
        if not delta.changed_fields:
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

    return max(candidates, key=lambda ins: ins.score)
