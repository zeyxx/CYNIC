"""Tests for behavior_log.jsonl source — parsing and tailing."""
import json
from pathlib import Path

from organs.mirror.sources.base import Event
from organs.mirror.sources.behavior import BehaviorSource


def _write_events(path: Path, events: list[dict]) -> None:
    with open(path, "w") as f:
        for e in events:
            f.write(json.dumps(e) + "\n")


def test_parse_click_event(tmp_path: Path) -> None:
    path = tmp_path / "behavior.jsonl"
    _write_events(path, [{"type": "click", "x": 500, "y": 300, "button": "left",
        "ts": "2026-05-18T21:00:00+00:00", "window_id": "0x1", "window_name": "Chrome", "url": "https://x.com"}])
    source = BehaviorSource(path)
    events = list(source.read_from(0))
    assert len(events) == 1
    assert events[0].source == "behavior"
    assert events[0].event_type == "click"
    assert events[0].data["url"] == "https://x.com"


def test_parse_key_event(tmp_path: Path) -> None:
    path = tmp_path / "behavior.jsonl"
    _write_events(path, [{"type": "key", "key": "a", "ts": "2026-05-18T21:00:00+00:00",
        "window_id": "0x1", "window_name": "Terminal", "url": ""}])
    source = BehaviorSource(path)
    events = list(source.read_from(0))
    assert events[0].event_type == "key"
    assert events[0].data["window_name"] == "Terminal"


def test_skip_malformed_lines(tmp_path: Path) -> None:
    path = tmp_path / "behavior.jsonl"
    with open(path, "w") as f:
        f.write("not json\n")
        f.write(json.dumps({"type": "scroll", "x": 0, "y": 0, "dx": 0, "dy": -3,
            "ts": "2026-05-18T21:00:00+00:00", "window_id": "0x1", "window_name": "Chrome", "url": ""}) + "\n")
    source = BehaviorSource(path)
    events = list(source.read_from(0))
    assert len(events) == 1
    assert source.error_count == 1


def test_skip_health_checkpoints(tmp_path: Path) -> None:
    path = tmp_path / "behavior.jsonl"
    _write_events(path, [{"type": "health_checkpoint", "events_captured": 100,
        "listeners_alive": True, "ts": "2026-05-18T21:00:00+00:00", "window_id": "", "window_name": "", "url": ""}])
    source = BehaviorSource(path)
    events = list(source.read_from(0))
    assert len(events) == 0


def test_read_from_offset(tmp_path: Path) -> None:
    path = tmp_path / "behavior.jsonl"
    lines = [{"type": "click", "x": i, "y": 0, "button": "left",
              "ts": f"2026-05-18T21:0{i}:00+00:00", "window_id": "0x1", "window_name": "Chrome", "url": ""}
             for i in range(5)]
    _write_events(path, lines)
    source = BehaviorSource(path)
    events = list(source.read_from(3))
    assert len(events) == 2


def test_current_offset(tmp_path: Path) -> None:
    path = tmp_path / "behavior.jsonl"
    _write_events(path, [{"type": "click", "x": 0, "y": 0, "button": "left",
        "ts": "2026-05-18T21:00:00+00:00", "window_id": "0x1", "window_name": "Chrome", "url": ""}] * 3)
    source = BehaviorSource(path)
    list(source.read_from(0))
    assert source.current_offset == 3
