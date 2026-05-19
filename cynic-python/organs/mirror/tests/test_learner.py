"""Tests for OnlineLearner — profile updates from events."""
from organs.mirror.learner import OnlineLearner, ProfileDelta
from organs.mirror.profile import BehavioralProfile, PHI_INV_SQ
from organs.mirror.sources.base import Event


def _click_event(hour: int, window: str, url: str = "") -> Event:
    return Event(source="behavior", event_type="click",
        timestamp=f"2026-05-18T{hour:02d}:00:00+00:00",
        data={"type": "click", "x": 500, "y": 300, "button": "left",
              "window_name": window, "url": url})


def _tweet_event(narrative: str, author: str, bookmarked: bool = False) -> Event:
    return Event(source="x_signals",
        event_type="tweet_bookmarked" if bookmarked else "tweet_seen",
        timestamp="2026-05-18T21:00:00+00:00",
        data={"narratives": [narrative], "author_screen_name": author,
              "likes": 100, "signal_score": 0.5, "viewer_bookmarked": bookmarked})


def test_activity_hours_update() -> None:
    learner = OnlineLearner(BehavioralProfile.empty())
    for _ in range(10):
        learner.ingest(_click_event(21, "Chrome"))
    for _ in range(5):
        learner.ingest(_click_event(14, "Terminal"))
    profile = learner.profile
    assert profile.activity_hours[21] > profile.activity_hours[14]
    assert profile.observation_count == 15


def test_narrative_affinity_from_tweets() -> None:
    learner = OnlineLearner(BehavioralProfile.empty())
    for _ in range(20):
        learner.ingest(_tweet_event("agent", "alice"))
    for _ in range(5):
        learner.ingest(_tweet_event("defi", "bob"))
    profile = learner.profile
    assert profile.narrative_affinity["agent"] > profile.narrative_affinity["defi"]


def test_bookmarked_tweets_boost_affinity() -> None:
    # Two narratives: bookmark weight (5x) gives "agent" higher relative share
    learner = OnlineLearner(BehavioralProfile.empty())
    for _ in range(10):
        learner.ingest(_tweet_event("agent", "alice", bookmarked=False))
    for _ in range(10):
        learner.ingest(_tweet_event("defi", "bob", bookmarked=False))
    learner_bm = OnlineLearner(BehavioralProfile.empty())
    for _ in range(10):
        learner_bm.ingest(_tweet_event("agent", "alice", bookmarked=True))
    for _ in range(10):
        learner_bm.ingest(_tweet_event("defi", "bob", bookmarked=False))
    # Bookmarked "agent" should have higher relative affinity than seen-only
    assert learner_bm.profile.narrative_affinity["agent"] > \
           learner.profile.narrative_affinity["agent"]


def test_app_time_from_behavior() -> None:
    learner = OnlineLearner(BehavioralProfile.empty())
    for _ in range(30):
        learner.ingest(_click_event(21, "Google Chrome"))
    for _ in range(10):
        learner.ingest(_click_event(21, "Terminal"))
    profile = learner.profile
    assert "Google Chrome" in profile.app_time_distribution
    assert profile.app_time_distribution["Google Chrome"] > \
           profile.app_time_distribution["Terminal"]


def test_delta_reports_changes() -> None:
    learner = OnlineLearner(BehavioralProfile.empty())
    delta = learner.ingest(_click_event(21, "Chrome"))
    assert delta is not None
    assert "activity_hours" in delta.changed_fields


def test_askesis_trigger_on_confidence_crossing() -> None:
    learner = OnlineLearner(BehavioralProfile.empty())
    triggered = False
    for i in range(200):
        delta = learner.ingest(_click_event(21, "Chrome"))
        if learner.should_emit_askesis(delta):
            triggered = True
            break
    assert triggered, "Askesis should trigger when confidence crosses threshold"
