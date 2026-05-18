"""BehavioralProfile — the core data model for the mirror agent."""
from __future__ import annotations

import json
import math
import statistics
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from typing import Any

PHI_INV = 0.618034
PHI_INV_SQ = 0.381966  # phi^-2, action gate threshold

# Half-life in observations: at n=HALF_LIFE, raw=1-1/e≈0.632.
# Derived from test constraints (n=5→~0.09, n=200→~0.55 at s=0.9).
# Conjecture — recalibrate after 30 days of real data.
_CONFIDENCE_HALF_LIFE = 31.77


def feature_confidence(n_observations: int, stability: float) -> float:
    """Confidence bounded by phi^-1, approached asymptotically.

    Uses exponential saturation toward PHI_INV. Stability=0.0 always
    yields 0.0 regardless of observation count.

    Input contract: n_observations >= 0, stability in [0.0, 1.0].
    Output guarantee: result in [0.0, PHI_INV].
    Failure modes: stability=0.0 → 0.0 (regardless of observations).
    Valid domains: behavioral pattern frequency counts.
    """
    # Exponential saturating term: approaches 1.0 as n → ∞
    saturation = 1.0 - math.exp(-n_observations / _CONFIDENCE_HALF_LIFE)
    return PHI_INV * saturation * stability


@dataclass
class TimeWindow:
    """Observed activity time window with relative strength."""

    start_hour: int
    end_hour: int
    strength: float


@dataclass
class Distribution:
    """Descriptive statistics for a measured quantity."""

    mean: float
    std: float
    p25: float
    p50: float
    p75: float
    n: int

    @classmethod
    def empty(cls) -> Distribution:
        return cls(mean=0.0, std=0.0, p25=0.0, p50=0.0, p75=0.0, n=0)

    @classmethod
    def from_values(cls, values: list[float]) -> Distribution:
        """Build distribution from a list of observed values.

        Input contract: values is a non-empty list of floats.
        Output guarantee: n == len(values), mean/std/percentiles computed.
        """
        if not values:
            return cls.empty()
        sorted_v = sorted(values)
        n = len(sorted_v)
        return cls(
            mean=statistics.mean(sorted_v),
            std=statistics.pstdev(sorted_v),
            p25=sorted_v[max(0, n // 4 - 1)] if n >= 4 else sorted_v[0],
            p50=statistics.median(sorted_v),
            p75=sorted_v[min(n - 1, 3 * n // 4)] if n >= 4 else sorted_v[-1],
            n=n,
        )


@dataclass
class Feature:
    """A weighted, confidence-tagged behavioral predictor."""

    name: str
    weight: float
    confidence: float


@dataclass
class Tension:
    """A recorded tension between mirror signal and dog signal."""

    description: str
    mirror_signal: str
    dog_signal: str
    frequency: int
    first_seen: str
    last_seen: str


@dataclass
class BehavioralProfile:
    """Complete behavioral model for T., serializable to/from JSON.

    All fields default to empty so partial profiles are valid.
    The profile is append-only: observation_count monotonically increases.
    """

    # Temporal patterns
    activity_hours: dict[int, float] = field(default_factory=dict)
    peak_windows: list[TimeWindow] = field(default_factory=list)
    session_duration_dist: Distribution = field(default_factory=Distribution.empty)
    context_switch_rate: float = 0.0

    # Content preferences
    narrative_affinity: dict[str, float] = field(default_factory=dict)
    author_affinity: dict[str, float] = field(default_factory=dict)
    content_length_pref: Distribution = field(default_factory=Distribution.empty)
    thread_vs_single: float = 0.0
    media_preference: dict[str, float] = field(default_factory=dict)

    # Engagement patterns
    dwell_by_content_type: dict[str, float] = field(default_factory=dict)
    app_time_distribution: dict[str, float] = field(default_factory=dict)
    coding_vs_browsing_ratio: float = 0.0

    # Decision predictors
    bookmark_predictors: list[Feature] = field(default_factory=list)
    ignore_predictors: list[Feature] = field(default_factory=list)

    # Epistemic tensions
    tension_zones: list[Tension] = field(default_factory=list)
    pattern_stability: dict[str, float] = field(default_factory=dict)
    blind_spots: list[str] = field(default_factory=list)

    # Metadata
    profile_version: str = "0.1.0"
    updated_at: str = ""
    observation_count: int = 0

    @classmethod
    def empty(cls) -> BehavioralProfile:
        """Create a fresh profile with current timestamp."""
        return cls(updated_at=datetime.now(timezone.utc).isoformat())

    def to_json(self) -> str:
        """Serialize to JSON string. Keys are stable (dataclass field order)."""
        return json.dumps(asdict(self), indent=2, default=str)

    @classmethod
    def from_json(cls, raw: str) -> BehavioralProfile:
        """Deserialize from JSON string. Reconstructs nested dataclasses."""
        data: dict[str, Any] = json.loads(raw)

        if "peak_windows" in data:
            data["peak_windows"] = [TimeWindow(**w) for w in data["peak_windows"]]

        for dist_field in ("session_duration_dist", "content_length_pref"):
            if dist_field in data and isinstance(data[dist_field], dict):
                data[dist_field] = Distribution(**data[dist_field])

        if "bookmark_predictors" in data:
            data["bookmark_predictors"] = [Feature(**f) for f in data["bookmark_predictors"]]
        if "ignore_predictors" in data:
            data["ignore_predictors"] = [Feature(**f) for f in data["ignore_predictors"]]
        if "tension_zones" in data:
            data["tension_zones"] = [Tension(**t) for t in data["tension_zones"]]

        # JSON keys are always strings; restore int keys for activity_hours
        if "activity_hours" in data:
            data["activity_hours"] = {int(k): v for k, v in data["activity_hours"].items()}

        return cls(**data)
