"""OnlineLearner — incremental EMA updates to a BehavioralProfile.

Tier 2 INFRASTRUCTURE: Online profile learning for the mirror organ.

K15 Consumer: mirror agent uses updated profile to generate askesis prompts.
Promotion date: 2026-05-18 (new module, wired into mirror pipeline).
Stability: initial implementation.

Input contract: Event instances from behavior or x_signals sources.
Output guarantee: ProfileDelta reports what changed; profile mutated in place.
Failure modes: malformed timestamps → hour defaults to -1 (ignored).
Valid domains: behavioral click/key/mouse/scroll events, tweet_seen/tweet_bookmarked.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timezone

from organs.mirror.profile import BehavioralProfile, PHI_INV_SQ, feature_confidence
from organs.mirror.sources.base import Event

# Slow adaptation: profile changes gradually over many observations.
_EMA_ALPHA = 0.01

# Tweet bookmark signal is 5× stronger than a passive seen event.
_BOOKMARK_WEIGHT = 5.0
_SEEN_WEIGHT = 1.0

# Minimum confidence change to consider an askesis trigger meaningful.
_ASKESIS_MIN_CHANGE = 0.01

# Behavior event types that contribute to temporal / app patterns.
_BEHAVIOR_EVENT_TYPES = {"click", "key", "mouse_move", "scroll"}


@dataclass
class ProfileDelta:
    """What changed during one call to OnlineLearner.ingest().

    changed_fields: names of BehavioralProfile fields that were mutated.
    max_confidence_change: largest absolute confidence change across all features.
    """

    changed_fields: set[str] = field(default_factory=set)
    max_confidence_change: float = 0.0


def _parse_hour(timestamp: str) -> int:
    """Extract UTC hour from ISO 8601 timestamp; returns -1 on failure.

    Input contract: timestamp is a string (may be malformed).
    Output guarantee: integer in [-1, 23].
    """
    try:
        dt = datetime.fromisoformat(timestamp)
        return dt.astimezone(timezone.utc).hour
    except (ValueError, TypeError):
        return -1


def _ema_update(current: float, weight: float, alpha: float = _EMA_ALPHA) -> float:
    """EMA formula: new = alpha * weight + (1 - alpha) * current."""
    return alpha * weight + (1.0 - alpha) * current


def _normalize(d: dict[str | int, float]) -> None:
    """Normalize dict values to sum to 1.0 in-place. No-op if sum is zero."""
    total = sum(d.values())
    if total > 0:
        for k in d:
            d[k] /= total


class OnlineLearner:
    """Incrementally updates a BehavioralProfile from a stream of Events.

    Internal state: raw counters (frequency) and raw EMA accumulators (affinity).
    Profile fields: normalized relative frequencies and proportional affinities.
    Separation prevents EMA saturation on high-volume data (all values → 1.0).

    Thread-safety: NOT thread-safe. Single-threaded use only.
    """

    def __init__(self, profile: BehavioralProfile) -> None:
        self._profile = profile
        # Raw counters for frequency distributions
        self._hour_counts: dict[int, int] = {}
        self._app_counts: dict[str, int] = {}
        # Raw EMA accumulators for affinity (not normalized per-event)
        self._narrative_raw: dict[str, float] = dict(profile.narrative_affinity)
        self._author_raw: dict[str, float] = dict(profile.author_affinity)
        # Snapshot of feature confidences at the last askesis trigger.
        self._last_askesis_confidences: dict[str, float] = {}

    @property
    def profile(self) -> BehavioralProfile:
        """Return the current (mutable) BehavioralProfile."""
        return self._profile

    def ingest(self, event: Event) -> ProfileDelta:
        """Process one event and update the profile.

        Input contract: event is a valid Event instance.
        Output guarantee: ProfileDelta.changed_fields is non-empty on mutation.
        Failure modes: unknown event types are silently ignored (no mutation).
        """
        delta = ProfileDelta()

        event_type = event.event_type

        if event_type in _BEHAVIOR_EVENT_TYPES:
            self._handle_behavior(event, delta)
        elif event_type in {"tweet_seen", "tweet_bookmarked"}:
            self._handle_tweet(event, delta)
        # Unknown event types: no mutation, empty delta returned.

        return delta

    # ------------------------------------------------------------------
    # Private handlers
    # ------------------------------------------------------------------

    def _handle_behavior(self, event: Event, delta: ProfileDelta) -> None:
        """Update activity_hours and app_time_distribution from a behavior event."""
        hour = _parse_hour(event.timestamp)
        if hour >= 0:
            self._hour_counts[hour] = self._hour_counts.get(hour, 0) + 1
            total = sum(self._hour_counts.values())
            self._profile.activity_hours = {
                h: c / total for h, c in self._hour_counts.items()
            }
            delta.changed_fields.add("activity_hours")

        window = event.data.get("window_name", "")
        if window:
            self._app_counts[window] = self._app_counts.get(window, 0) + 1
            total = sum(self._app_counts.values())
            self._profile.app_time_distribution = {
                w: c / total for w, c in self._app_counts.items()
            }
            delta.changed_fields.add("app_time_distribution")

        self._profile.observation_count += 1
        delta.changed_fields.add("observation_count")

        self._update_max_confidence_change(delta)

    def _handle_tweet(self, event: Event, delta: ProfileDelta) -> None:
        """Update narrative_affinity and author_affinity from a tweet event."""
        bookmarked = event.event_type == "tweet_bookmarked"
        weight = _BOOKMARK_WEIGHT if bookmarked else _SEEN_WEIGHT

        narratives: list[str] = event.data.get("narratives", [])
        for narrative in narratives:
            if narrative:
                old = self._narrative_raw.get(narrative, 0.0)
                self._narrative_raw[narrative] = _ema_update(old, weight)
                delta.changed_fields.add("narrative_affinity")

        author: str = event.data.get("author_screen_name", "")
        if author:
            old = self._author_raw.get(author, 0.0)
            self._author_raw[author] = _ema_update(old, weight)
            delta.changed_fields.add("author_affinity")

        # Normalize raw EMA accumulators → profile relative affinities
        self._profile.narrative_affinity = dict(self._narrative_raw)
        _normalize(self._profile.narrative_affinity)
        self._profile.author_affinity = dict(self._author_raw)
        _normalize(self._profile.author_affinity)

        self._profile.observation_count += 1
        delta.changed_fields.add("observation_count")

        self._update_max_confidence_change(delta)

    def _update_max_confidence_change(self, delta: ProfileDelta) -> None:
        """Compute the maximum absolute confidence change since last askesis trigger.

        Confidence is derived from observation_count and a stability proxy of 1.0
        (all features treated as perfectly stable for EMA updates — stability
        calibration is deferred until 30 days of real data are available).
        """
        current_confidence = feature_confidence(self._profile.observation_count, stability=1.0)
        # last_confidence: what confidence was when askesis was last triggered.
        last_confidence = self._last_askesis_confidences.get(
            "__global__", feature_confidence(0, stability=1.0)
        )
        delta.max_confidence_change = abs(current_confidence - last_confidence)

    def should_emit_askesis(self, delta: ProfileDelta) -> bool:
        """True when a feature's confidence has crossed phi^-2 since last trigger.

        Trigger conditions (all must hold):
        1. max_confidence_change exceeds _ASKESIS_MIN_CHANGE (non-trivial update).
        2. Current global confidence has crossed PHI_INV_SQ upward since last trigger.

        Side effect: on True, records current confidence so the gate doesn't
        re-trigger on the same crossing.

        Input contract: delta came from the most recent ingest() call.
        Output guarantee: True at most once per phi^-2 crossing.
        """
        if delta.max_confidence_change < _ASKESIS_MIN_CHANGE:
            return False

        current_confidence = feature_confidence(
            self._profile.observation_count, stability=1.0
        )
        last_confidence = self._last_askesis_confidences.get("__global__", 0.0)

        # Crossing: was below threshold before, now at or above.
        crossed = last_confidence < PHI_INV_SQ <= current_confidence

        if crossed:
            self._last_askesis_confidences["__global__"] = current_confidence
            return True

        return False
