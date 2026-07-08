import json
import types

import pytest

from organs.twitter.src.entrypoints import behavior_logger


def test_behavior_logger_masks_printable_keys(tmp_path, monkeypatch):
    logger = behavior_logger.BehaviorLogger(tmp_path / "behavior.jsonl")
    monkeypatch.setattr(logger, "_context", lambda: {})

    logger.on_key_press(types.SimpleNamespace(char="a"))
    logger.on_key_press(types.SimpleNamespace(char="7"))
    logger.on_key_press(types.SimpleNamespace(char="?"))

    keys = [event["key"] for event in logger.buffer]
    assert keys == ["<alpha>", "<digit>", "<symbol>"]


def test_behavior_logger_flush_writes_jsonl(tmp_path, monkeypatch):
    output = tmp_path / "behavior.jsonl"
    logger = behavior_logger.BehaviorLogger(output)
    monkeypatch.setattr(logger, "_context", lambda: {"window_id": "0x1"})

    logger.on_mouse_scroll(1, 2, 0, -1)
    logger.flush()

    rows = [json.loads(line) for line in output.read_text().splitlines()]
    assert rows[0]["type"] == "scroll"
    assert rows[0]["window_id"] == "0x1"
    assert logger.buffer == []


def test_load_pynput_reports_missing_dependency(monkeypatch):
    monkeypatch.setattr(behavior_logger, "mouse", None)
    monkeypatch.setattr(behavior_logger, "keyboard", None)

    real_import = __import__

    def fake_import(name, *args, **kwargs):
        if name == "pynput":
            raise ImportError("missing pynput")
        return real_import(name, *args, **kwargs)

    monkeypatch.setattr("builtins.__import__", fake_import)

    with pytest.raises(RuntimeError, match="pynput is required"):
        behavior_logger._load_pynput()

