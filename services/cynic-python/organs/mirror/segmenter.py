"""Segmenter — transforms L0 raw events into L1 behavioral segments.

Tier 2 INFRASTRUCTURE: segmentation layer for the mirror organ.

K15 Consumer: OnlineLearner ingests completed Segments for profile updates.
Systemd: mirror-agent.service (coordinator feeds events through segmenter).
Promotion date: 2026-05-19 (new module, wired into mirror pipeline).
Stability: initial implementation.

A Segment is a continuous focus block on one wm_class. It ends when:
- focus_change event fires (user switches app)
- idle_start event fires (user leaves)
- segment exceeds MAX_SEGMENT_SECONDS (safety cap)

Segments carry L1 features derived from L0 primitives:
- duration, input counts by type, production ratio, input velocity.

Domains (X, code, terminal) are NOT hardcoded — they emerge from wm_class
values in the segments. The segmenter is domain-agnostic.

Input contract: Event instances from BehaviorSource (L0 events).
Output guarantee: completed Segments have duration > 0 and at least 1 event.
Failure modes: malformed timestamps → segment duration defaults to 0.
Valid domains: all machine activity captured by behavior_logger.py.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timezone

from organs.mirror.sources.base import Event

# Segments longer than this are force-closed (prevents unbounded accumulation).
MAX_SEGMENT_SECONDS = 3600.0  # 1 hour

# Events that represent user input (not synthetic logger events).
_INPUT_TYPES = {"click", "key", "mouse_move", "scroll"}


@dataclass
class Segment:
    """A continuous focus block on one application.

    L1 primitive: the atomic unit of behavioral analysis. Everything at L2+
    is derived from aggregating Segments.
    """

    wm_class: str
    sub_context: str  # segmentation sub-key: window_name (terminal) or url_domain (browser)
    window_name: str  # last observed (informational)
    workspace: int
    start_ts: str  # ISO 8601
    end_ts: str  # ISO 8601
    duration_seconds: float

    # Input counts by type
    key_count: int = 0
    click_count: int = 0
    scroll_count: int = 0
    mouse_move_count: int = 0

    # Derived L1 features
    total_inputs: int = 0
    production_ratio: float = 0.0  # keys / total_inputs (0 if no inputs)
    input_velocity: float = 0.0  # inputs / minute (0 if duration == 0)

    # URL context (for browser apps — last observed URL domain)
    url_domain: str = ""

    # Events that contributed to this segment (not stored, just counted)
    event_count: int = 0

    def compute_derived(self) -> None:
        """Recompute derived features from raw counts. Call after accumulation."""
        self.total_inputs = (
            self.key_count + self.click_count + self.scroll_count + self.mouse_move_count
        )
        if self.total_inputs > 0:
            self.production_ratio = self.key_count / self.total_inputs
        else:
            self.production_ratio = 0.0

        if self.duration_seconds > 0:
            self.input_velocity = (self.total_inputs / self.duration_seconds) * 60.0
        else:
            self.input_velocity = 0.0


@dataclass
class Session:
    """A continuous presence period between idle boundaries.

    L1 primitive: groups Segments into sessions. A session starts when
    activity begins (or idle_end) and ends at idle_start.
    """

    start_ts: str
    end_ts: str
    duration_seconds: float
    segment_count: int
    total_inputs: int
    dominant_app: str  # wm_class with most inputs in this session
    dominant_context: str = ""


def _parse_ts(ts_str: str) -> datetime | None:
    """Parse ISO 8601 timestamp, return None on failure."""
    try:
        return datetime.fromisoformat(ts_str)
    except (ValueError, TypeError):
        return None


def _seconds_between(start: str, end: str) -> float:
    """Compute seconds between two ISO timestamps. Returns 0.0 on failure."""
    s = _parse_ts(start)
    e = _parse_ts(end)
    if s and e:
        return max(0.0, (e - s).total_seconds())
    return 0.0


def _extract_domain(url: str) -> str:
    """Extract domain from URL. Returns '' on failure."""
    if not url:
        return ""
    try:
        from urllib.parse import urlparse
        return urlparse(url).netloc
    except Exception:
        return ""


_BROWSER_CLASSES = {"Google-chrome", "Chromium-browser", "firefox", "Firefox"}


def _derive_sub_context(wm_class: str, window_name: str, url: str) -> str:
    """Derive segmentation sub-context: url_domain for browsers, window_name for terminal."""
    if wm_class in _BROWSER_CLASSES:
        domain = _extract_domain(url)
        return domain if domain else window_name
    return window_name


class Segmenter:
    """Accumulates L0 events into L1 Segments.

    Stateful: tracks the current open segment. When a segment-ending event
    arrives (focus_change, idle_start, or max duration exceeded), the current
    segment is closed and returned.

    Thread-safety: NOT thread-safe. Single-threaded use only (same as Learner).
    """

    def __init__(self) -> None:
        self._current: _OpenSegment | None = None
        self._session_start: str = ""
        self._session_segments: list[Segment] = []
        self._session_total_inputs: int = 0
        self._completed_segments: list[Segment] = []
        self._completed_sessions: list[Session] = []

    @property
    def completed_segments(self) -> list[Segment]:
        """Drain completed segments (returns and clears the buffer)."""
        result = self._completed_segments
        self._completed_segments = []
        return result

    @property
    def completed_sessions(self) -> list[Session]:
        """Drain completed sessions (returns and clears the buffer)."""
        result = self._completed_sessions
        self._completed_sessions = []
        return result

    @property
    def current_segment(self) -> Segment | None:
        """Peek at the currently open segment (not yet completed)."""
        if self._current is not None:
            return self._current.to_segment()
        return None

    def ingest(self, event: Event) -> None:
        """Process one L0 event. May close the current segment and open a new one."""
        event_type = event.event_type
        ts = event.timestamp

        if event_type == "idle_start":
            self._close_current(ts, reason="idle")
            self._close_session(ts)
            return

        if event_type == "idle_end":
            # New session begins
            self._session_start = ts
            self._session_segments = []
            self._session_total_inputs = 0
            return

        if event_type == "focus_change":
            self._close_current(ts, reason="focus_change")
            wm_class = event.data.get("to_wm_class", "")
            window_name = event.data.get("to_window_name", "")
            workspace = event.data.get("workspace", -1)
            url = event.data.get("url", "")
            sub_ctx = _derive_sub_context(wm_class, window_name, url)
            self._open_new(wm_class, window_name, workspace, ts, sub_context=sub_ctx)
            return

        # Regular input event
        if event_type in _INPUT_TYPES:
            wm_class = event.data.get("wm_class", "")
            window_name = event.data.get("window_name", "")
            workspace = event.data.get("workspace", -1)
            url = event.data.get("url", "")

            sub_ctx = _derive_sub_context(wm_class, window_name, url)

            if self._current is None:
                self._open_new(wm_class, window_name, workspace, ts, sub_context=sub_ctx)
            elif (wm_class and wm_class != self._current.wm_class) or                  (sub_ctx and sub_ctx != self._current.sub_context):
                self._close_current(ts, reason="context_change")
                self._open_new(wm_class, window_name, workspace, ts, sub_context=sub_ctx)

            # Check max duration
            if self._current is not None:
                elapsed = _seconds_between(self._current.start_ts, ts)
                if elapsed > MAX_SEGMENT_SECONDS:
                    self._close_current(ts, reason="max_duration")
                    self._open_new(wm_class, window_name, workspace, ts)

            if self._current is not None:
                self._current.accumulate(event_type, ts, url)
                self._current.window_name = window_name

            # Track session start
            if not self._session_start:
                self._session_start = ts

    def flush(self) -> None:
        """Force-close the current segment (e.g. at shutdown)."""
        if self._current is not None:
            self._close_current(self._current.last_ts, reason="flush")

    # ------------------------------------------------------------------
    # Private
    # ------------------------------------------------------------------

    def _open_new(self, wm_class: str, window_name: str, workspace: int, ts: str,
                  sub_context: str = "") -> None:
        self._current = _OpenSegment(
            wm_class=wm_class,
            sub_context=sub_context or window_name,
            window_name=window_name,
            workspace=workspace,
            start_ts=ts,
        )

    def _close_current(self, end_ts: str, reason: str) -> None:
        if self._current is None:
            return

        seg = self._current.to_segment(end_ts)
        if seg.event_count > 0:
            self._completed_segments.append(seg)
            self._session_segments.append(seg)
            self._session_total_inputs += seg.total_inputs

        self._current = None

    def _close_session(self, end_ts: str) -> None:
        if not self._session_start or not self._session_segments:
            self._session_start = ""
            self._session_segments = []
            self._session_total_inputs = 0
            return

        duration = _seconds_between(self._session_start, end_ts)

        app_inputs: dict[str, int] = {}
        ctx_inputs: dict[str, int] = {}
        for seg in self._session_segments:
            app_inputs[seg.wm_class] = app_inputs.get(seg.wm_class, 0) + seg.total_inputs
            ctx_inputs[seg.sub_context] = ctx_inputs.get(seg.sub_context, 0) + seg.total_inputs
        dominant = max(app_inputs, key=app_inputs.get) if app_inputs else ""  # type: ignore[arg-type]
        dominant_ctx = max(ctx_inputs, key=ctx_inputs.get) if ctx_inputs else ""  # type: ignore[arg-type]

        session = Session(
            start_ts=self._session_start,
            end_ts=end_ts,
            duration_seconds=duration,
            segment_count=len(self._session_segments),
            total_inputs=self._session_total_inputs,
            dominant_app=dominant,
            dominant_context=dominant_ctx,
        )
        self._completed_sessions.append(session)

        self._session_start = ""
        self._session_segments = []
        self._session_total_inputs = 0


@dataclass
class _OpenSegment:
    """Mutable accumulator for a segment being built."""

    wm_class: str
    sub_context: str
    window_name: str
    workspace: int
    start_ts: str
    last_ts: str = ""

    key_count: int = 0
    click_count: int = 0
    scroll_count: int = 0
    mouse_move_count: int = 0
    event_count: int = 0
    url_domain: str = ""

    def accumulate(self, event_type: str, ts: str, url: str = "") -> None:
        self.last_ts = ts
        self.event_count += 1

        if event_type == "key":
            self.key_count += 1
        elif event_type == "click":
            self.click_count += 1
        elif event_type == "scroll":
            self.scroll_count += 1
        elif event_type == "mouse_move":
            self.mouse_move_count += 1

        if url:
            self.url_domain = _extract_domain(url)

    def to_segment(self, end_ts: str = "") -> Segment:
        actual_end = end_ts or self.last_ts or self.start_ts
        duration = _seconds_between(self.start_ts, actual_end)

        seg = Segment(
            wm_class=self.wm_class,
            sub_context=self.sub_context,
            window_name=self.window_name,
            workspace=self.workspace,
            start_ts=self.start_ts,
            end_ts=actual_end,
            duration_seconds=duration,
            key_count=self.key_count,
            click_count=self.click_count,
            scroll_count=self.scroll_count,
            mouse_move_count=self.mouse_move_count,
            url_domain=self.url_domain,
            event_count=self.event_count,
        )
        seg.compute_derived()
        return seg
