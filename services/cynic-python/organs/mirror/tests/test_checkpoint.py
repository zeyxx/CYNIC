"""Tests for ProfileCheckpoint persistence and crash recovery."""
import json
from pathlib import Path
from organs.mirror.checkpoint import ProfileCheckpoint
from organs.mirror.profile import BehavioralProfile


def test_checkpoint_save_and_load(tmp_path: Path) -> None:
    profile = BehavioralProfile.empty()
    profile.observation_count = 100
    profile.activity_hours = {21: 0.4}
    cp = ProfileCheckpoint(
        profile=profile,
        cursors={"behavior": 5000, "x_signals": 1200},
        askesis_sent_today=2,
        askesis_date="2026-05-18",
    )
    path = tmp_path / "profile.json"
    cp.save(path)
    loaded = ProfileCheckpoint.load(path)
    assert loaded.profile.observation_count == 100
    assert loaded.cursors["behavior"] == 5000
    assert loaded.askesis_sent_today == 2


def test_checkpoint_load_missing_file(tmp_path: Path) -> None:
    path = tmp_path / "nonexistent.json"
    loaded = ProfileCheckpoint.load(path)
    assert loaded.profile.observation_count == 0
    assert loaded.cursors == {}


def test_checkpoint_askesis_throttle() -> None:
    cp = ProfileCheckpoint(
        profile=BehavioralProfile.empty(),
        cursors={},
        askesis_sent_today=3,
        askesis_date="2026-05-18",
    )
    assert not cp.can_send_askesis("2026-05-18")
    assert cp.can_send_askesis("2026-05-19")


def test_checkpoint_increment_askesis() -> None:
    cp = ProfileCheckpoint(
        profile=BehavioralProfile.empty(),
        cursors={},
        askesis_sent_today=0,
        askesis_date="2026-05-17",
    )
    cp.record_askesis_sent("2026-05-18")
    assert cp.askesis_sent_today == 1
    assert cp.askesis_date == "2026-05-18"
