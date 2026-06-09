"""Tests for L3 behavioral pattern detection."""
from __future__ import annotations

from organs.mirror.patterns import (
    PatternDetector,
    SpinnerState,
    is_flow_block,
    spinner_state,
    _stability_from_values,
)
from organs.mirror.profile import BehavioralProfile
from organs.mirror.segmenter import Segment, Session


def _seg(
    wm_class: str = "Gnome-terminal",
    window_name: str = "✳ Test Task",
    duration: float = 600.0,
    keys: int = 200,
    clicks: int = 10,
    scrolls: int = 20,
    moves: int = 100,
    url_domain: str = "",
) -> Segment:
    """Build a test segment with derived features computed."""
    s = Segment(
        wm_class=wm_class,
        sub_context=window_name,
        window_name=window_name,
        workspace=0,
        start_ts="2026-05-20T14:00:00+00:00",
        end_ts="2026-05-20T14:10:00+00:00",
        duration_seconds=duration,
        key_count=keys,
        click_count=clicks,
        scroll_count=scrolls,
        mouse_move_count=moves,
        url_domain=url_domain,
        event_count=keys + clicks + scrolls + moves,
    )
    s.compute_derived()
    return s


# ── Spinner state parsing ──


def test_spinner_user_typing():
    assert spinner_state("✳ Build app") == SpinnerState.USER_TYPING


def test_spinner_agent_thinking():
    assert spinner_state("⠂ Build app") == SpinnerState.AGENT_THINKING
    assert spinner_state("⠐ Design thing") == SpinnerState.AGENT_THINKING
    assert spinner_state("⠸ Other") == SpinnerState.AGENT_THINKING


def test_spinner_unknown():
    assert spinner_state("") == SpinnerState.UNKNOWN
    assert spinner_state("Firefox") == SpinnerState.UNKNOWN
    assert spinner_state("Alacritty") == SpinnerState.UNKNOWN


# ── Flow block detection ──


def test_flow_block_qualifies():
    seg = _seg(duration=400, keys=100, moves=200)
    # velocity = 330/400*60 = 49.5/min > 10
    assert is_flow_block(seg)


def test_flow_block_too_short():
    seg = _seg(duration=60, keys=100, moves=200)
    assert not is_flow_block(seg)


def test_flow_block_too_few_keys():
    seg = _seg(duration=600, keys=10, moves=200)
    assert not is_flow_block(seg)


def test_flow_block_agent_thinking_rejected():
    seg = _seg(window_name="⠂ Build app", duration=600, keys=100, moves=200)
    assert not is_flow_block(seg)


def test_flow_block_chrome_no_spinner_check():
    """Chrome flow blocks don't check spinner state."""
    seg = _seg(
        wm_class="Google-chrome",
        window_name="Docs - Chrome",
        duration=400,
        keys=100,
        moves=200,
    )
    assert is_flow_block(seg)


# ── Stability from variance ──


def test_stability_identical_values():
    # Zero variance → stability = 1.0
    assert _stability_from_values([0.4, 0.4, 0.4, 0.4, 0.4]) == 1.0


def test_stability_high_variance():
    # High variance → low stability
    result = _stability_from_values([0.1, 0.9, 0.1, 0.9, 0.1])
    assert result < 0.5


def test_stability_insufficient_data():
    # < 3 values → default 0.5
    assert _stability_from_values([0.4]) == 0.5
    assert _stability_from_values([]) == 0.5


# ── PatternDetector integration ──


def test_detector_ingest_segment_tracks_flow():
    pd = PatternDetector()
    flow_seg = _seg(duration=400, keys=100, moves=200)
    pd.ingest_segment(flow_seg)
    summary = pd.summarize()
    assert summary.flow_blocks_today == 1
    assert summary.longest_flow_minutes > 0


def test_detector_ingest_session():
    pd = PatternDetector()
    session = Session(
        start_ts="2026-05-20T14:00:00+00:00",
        end_ts="2026-05-20T15:00:00+00:00",
        duration_seconds=3600.0,
        segment_count=10,
        total_inputs=500,
        dominant_app="Gnome-terminal",
        dominant_context="✳ Test",
    )
    pd.ingest_session(session)
    profile = BehavioralProfile.empty()
    summary = pd.summarize()
    changed = pd.update_profile(profile, summary)
    assert "session_duration_dist" in changed
    assert profile.session_duration_dist.n == 1


def test_detector_update_profile_stability():
    pd = PatternDetector()
    # Ingest several segments with varying production ratios
    for prod in [0.3, 0.35, 0.32, 0.38, 0.31]:
        seg = _seg(keys=int(prod * 100), moves=int((1 - prod) * 100))
        pd.ingest_segment(seg)

    profile = BehavioralProfile.empty()
    summary = pd.summarize()
    changed = pd.update_profile(profile, summary)

    assert "pattern_stability" in changed
    assert "prod_ratio" in profile.pattern_stability
    # Similar values → high stability
    assert profile.pattern_stability["prod_ratio"] > 0.7


def test_detector_human_ai_ratio():
    pd = PatternDetector()
    # 3 human segments, 1 AI segment
    for _ in range(3):
        pd.ingest_segment(_seg(window_name="✳ Coding", duration=60, keys=50, moves=30))
    pd.ingest_segment(_seg(window_name="⠂ Thinking", duration=60, keys=5, moves=30))

    summary = pd.summarize()
    # 3 * 60 / (3 * 60 + 60) = 0.75
    assert 0.7 < summary.human_ai_ratio < 0.8


# ── Surprise ranking ──


def test_surprise_ranking():
    from organs.mirror.askesis import Insight, InsightType

    boring = Insight(
        insight_type=InsightType.TEMPORAL,
        message="boring",
        confidence=0.5,
        observation_count=100,
        surprise=0.0,
    )
    surprising = Insight(
        insight_type=InsightType.FLOW_STATE,
        message="flow!",
        confidence=0.3,
        observation_count=50,
        surprise=0.8,
    )
    # boring.score = 0.5 * 0.1 = 0.05
    # surprising.score = 0.3 * 0.8 = 0.24
    assert surprising.score > boring.score
