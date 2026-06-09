"""PatternDetector — L3 behavioral pattern detection from L1 Segments.

Tier 2 INFRASTRUCTURE: pattern detection layer for the mirror organ.

K15 Consumer: askesis module uses PatternSummary to generate actionable insights.
Systemd: mirror-agent.service (coordinator feeds completed segments/sessions).
Promotion date: 2026-05-20 (new module, wired into mirror pipeline).
Stability: initial implementation.

Detects: flow states, context switching anomalies, production ratio shifts,
human vs AI activity split (spinner state). Computes pattern_stability from
measured variance (CV of rolling windows).

Input contract: completed Segment/Session objects from Segmenter.
Output guarantee: PatternSummary with all floats in [0, 1] where applicable.
Failure modes: insufficient data → stability defaults to 0.5, ratios to 0.0.
Valid domains: all machine activity captured by behavior_logger.py.
"""
from __future__ import annotations

import statistics
from dataclasses import dataclass, field
from datetime import datetime, timezone
from enum import Enum
from typing import ClassVar

from organs.mirror.profile import BehavioralProfile, Distribution
from organs.mirror.segmenter import Segment, Session

# ── Spinner state detection ──

# Braille spinner characters used by Claude Code / Gemini CLI
_AGENT_SPINNERS = set("⠂⠐⠠⠄⠈⠁⠃⠇⠋⠉⠙⠸⠴⠦⠖⠒⠓⠏⠟⠻⠽⠾⠷⠿")
_USER_SPINNER = "✳"


class SpinnerState(Enum):
    """Terminal spinner state — distinguishes human typing from agent processing."""
    USER_TYPING = "user_typing"
    AGENT_THINKING = "agent_thinking"
    UNKNOWN = "unknown"


def spinner_state(window_name: str) -> SpinnerState:
    """Parse first character of window_name to detect spinner state.

    Claude Code uses ✳ when the user is typing, braille characters (⠂⠐⠸ etc.)
    when the agent is processing. This cleanly separates human vs AI activity.
    """
    if not window_name:
        return SpinnerState.UNKNOWN
    first = window_name[0]
    if first == _USER_SPINNER:
        return SpinnerState.USER_TYPING
    if first in _AGENT_SPINNERS:
        return SpinnerState.AGENT_THINKING
    return SpinnerState.UNKNOWN


# ── Flow detection ──

_FLOW_MIN_DURATION = 300.0   # 5 minutes
_FLOW_MIN_KEYS = 50          # at least 50 keystrokes
_FLOW_MIN_VELOCITY = 10.0    # at least 10 inputs/min


def is_flow_block(segment: Segment) -> bool:
    """True when a segment qualifies as a deep work flow block.

    Thresholds derived from 24h of real data: only 12/215 segments qualified.
    For terminal segments, also requires USER_TYPING spinner state.
    """
    if segment.duration_seconds < _FLOW_MIN_DURATION:
        return False
    if segment.key_count < _FLOW_MIN_KEYS:
        return False
    if segment.input_velocity < _FLOW_MIN_VELOCITY:
        return False
    # Terminal: require human typing, not agent processing
    if segment.wm_class == "Gnome-terminal":
        state = spinner_state(segment.window_name)
        if state == SpinnerState.AGENT_THINKING:
            return False
    return True


# ── Pattern data structures ──

_MAX_HISTORY = 50  # rolling window size per axis


@dataclass
class PatternSummary:
    """Output of PatternDetector.summarize() — current behavioral state."""

    flow_blocks_today: int = 0
    longest_flow_minutes: float = 0.0
    current_switch_rate_per_hour: float = 0.0
    current_prod_ratio: float = 0.0
    human_ai_ratio: float = 0.0  # fraction of terminal time from human typing
    pattern_stability: dict[str, float] = field(default_factory=dict)


class PatternDetector:
    """L3: detects behavioral patterns from completed Segments and Sessions.

    Stateful: maintains rolling windows across calls. Single-threaded only.
    """

    def __init__(self) -> None:
        # Rolling histories for stability computation
        self._prod_ratios: list[float] = []
        self._human_ai_ratios: list[float] = []
        self._session_durations: list[float] = []
        self._switch_rates: list[float] = []

        # Flow tracking
        self._flow_blocks_today: list[Segment] = []
        self._today: str = ""

        # Per-hour switch counting
        self._switch_timestamps: list[float] = []  # unix timestamps
        self._current_hour: int = -1

        # Accumulation for human/AI split
        self._human_dwell: float = 0.0
        self._ai_dwell: float = 0.0

    def ingest_segment(self, segment: Segment) -> None:
        """Process a completed L1 Segment."""
        self._rotate_day(segment.start_ts)

        # Flow detection
        if is_flow_block(segment):
            self._flow_blocks_today.append(segment)

        # Production ratio history (terminal segments only, with real input)
        if segment.wm_class == "Gnome-terminal" and segment.total_inputs > 10:
            self._prod_ratios.append(segment.production_ratio)
            if len(self._prod_ratios) > _MAX_HISTORY:
                self._prod_ratios.pop(0)

        # Human vs AI dwell time (terminal only)
        if segment.wm_class == "Gnome-terminal" and segment.duration_seconds > 0:
            state = spinner_state(segment.window_name)
            if state == SpinnerState.USER_TYPING:
                self._human_dwell += segment.duration_seconds
            elif state == SpinnerState.AGENT_THINKING:
                self._ai_dwell += segment.duration_seconds
            total = self._human_dwell + self._ai_dwell
            if total > 0:
                ratio = self._human_dwell / total
                self._human_ai_ratios.append(ratio)
                if len(self._human_ai_ratios) > _MAX_HISTORY:
                    self._human_ai_ratios.pop(0)

    def ingest_session(self, session: Session) -> None:
        """Record a completed session duration."""
        if session.duration_seconds > 0:
            self._session_durations.append(session.duration_seconds)
            if len(self._session_durations) > _MAX_HISTORY:
                self._session_durations.pop(0)

    def ingest_focus_change(self, timestamp: str) -> None:
        """Count a context switch for per-hour rate computation."""
        try:
            dt = datetime.fromisoformat(timestamp)
            ts_unix = dt.timestamp()
        except (ValueError, TypeError):
            return

        hour = dt.hour
        if hour != self._current_hour:
            # Hour rolled over — record the rate for the previous hour
            if self._current_hour >= 0 and self._switch_timestamps:
                rate = len(self._switch_timestamps)  # switches per hour
                self._switch_rates.append(float(rate))
                if len(self._switch_rates) > _MAX_HISTORY:
                    self._switch_rates.pop(0)
            self._switch_timestamps = []
            self._current_hour = hour

        self._switch_timestamps.append(ts_unix)

    def summarize(self) -> PatternSummary:
        """Compute current pattern summary from accumulated data."""
        # Flow
        flow_count = len(self._flow_blocks_today)
        longest = max((s.duration_seconds for s in self._flow_blocks_today), default=0.0) / 60.0

        # Switch rate: current hour extrapolated
        now_switches = len(self._switch_timestamps)
        current_switch_rate = float(now_switches)  # per partial hour, good enough

        # Production ratio: latest or 0
        current_prod = self._prod_ratios[-1] if self._prod_ratios else 0.0

        # Human/AI ratio
        total_dwell = self._human_dwell + self._ai_dwell
        human_ai = self._human_dwell / total_dwell if total_dwell > 0 else 0.0

        # Stability per axis
        stability = {
            "prod_ratio": _stability_from_values(self._prod_ratios),
            "human_ai_ratio": _stability_from_values(self._human_ai_ratios),
            "switch_rate": _stability_from_values(self._switch_rates),
            "session_duration": _stability_from_values(self._session_durations),
        }

        return PatternSummary(
            flow_blocks_today=flow_count,
            longest_flow_minutes=longest,
            current_switch_rate_per_hour=current_switch_rate,
            current_prod_ratio=current_prod,
            human_ai_ratio=human_ai,
            pattern_stability=stability,
        )

    def update_profile(self, profile: BehavioralProfile, summary: PatternSummary) -> set[str]:
        """Write pattern summary into the BehavioralProfile. Returns changed field names."""
        changed: set[str] = set()

        # Session duration distribution
        if self._session_durations:
            profile.session_duration_dist = Distribution.from_values(self._session_durations)
            changed.add("session_duration_dist")

        # Pattern stability (drives ActionGate confidence)
        if summary.pattern_stability:
            profile.pattern_stability = dict(summary.pattern_stability)
            changed.add("pattern_stability")

        # Context switch rate (actual per-hour, not EMA)
        profile.context_switch_rate = summary.current_switch_rate_per_hour
        changed.add("context_switch_rate")

        # Coding vs browsing ratio
        profile.coding_vs_browsing_ratio = summary.current_prod_ratio
        changed.add("coding_vs_browsing_ratio")

        return changed

    # ── Private ──

    def _rotate_day(self, ts: str) -> None:
        """Reset daily counters on date rollover."""
        try:
            day = datetime.fromisoformat(ts).date().isoformat()
        except (ValueError, TypeError):
            return
        if day != self._today:
            self._flow_blocks_today = []
            self._today = day


def _stability_from_values(values: list[float]) -> float:
    """Compute stability from coefficient of variation. Returns 0.5 on insufficient data."""
    if len(values) < 3:
        return 0.5
    mean = statistics.mean(values)
    if mean == 0:
        return 0.5
    std = statistics.pstdev(values)
    cv = std / abs(mean)
    return max(0.0, 1.0 - min(cv, 1.0))
