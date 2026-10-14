"""Windows-Gemini capture proof — frozen layout is the only ingestion source.

The real archive is hundreds of megabytes, so this module proves the SHAPE:
a synthetic tree in the frozen layout normalizes end-to-end (chats merged by
sessionId, tool-outputs inventoried), live mutation does not move the capture,
and absent sources fail closed. The real-corpus freeze is exercised by
capture_windows_gemini.py, not by this suite.
"""

from __future__ import annotations

import json
import os

import pytest

from cynic_data.canonical.adapters.normalizers.windows_gemini import WindowsGeminiNormalizer
from cynic_data.capture.adapters.capturer_directory import DirectorySourceCapturer
from cynic_data.capture.usecases.capture_source import capture_sources


def _make_tree(root: str) -> None:
    chat_a = {
        "sessionId": "sid-1",
        "projectHash": "ph",
        "startTime": "2026-03-01T00:00:00Z",
        "lastUpdated": "2026-03-01T01:00:00Z",
        "kind": "main",
        "summary": "probe session",
        "messages": [
            {"id": "m1", "timestamp": "2026-03-01T00:00:01Z", "type": "user",
             "content": [{"text": "hello"}]},
            {"id": "m2", "timestamp": "2026-03-01T00:00:02Z", "type": "gemini",
             "content": "world", "model": "gemini-x",
             "thoughts": [{"subject": "s", "description": "d", "timestamp": "2026-03-01T00:00:02Z"}],
             "tokens": {"input": 10, "output": 5, "cached": 1, "thoughts": 2, "tool": 0, "total": 15},
             "toolCalls": [{"id": "c1", "name": "read", "displayName": "Read",
                             "description": "r", "args": {"p": "x"},
                             "result": [{"functionResponse": {"id": "c1", "name": "read",
                                                              "response": {"output": "out"}}}],
                             "resultDisplay": "out", "status": "success",
                             "timestamp": "2026-03-01T00:00:03Z"}]},
        ],
    }
    chat_b = {
        "sessionId": "sid-1",
        "projectHash": "ph",
        "startTime": "2026-03-01T02:00:00Z",
        "lastUpdated": "2026-03-01T02:00:00Z",
        "messages": [
            {"id": "m3", "timestamp": "2026-03-01T02:00:01Z", "type": "info", "content": "done"},
        ],
    }
    os.makedirs(os.path.join(root, "tmp", "proj", "chats"))
    os.makedirs(os.path.join(root, "tmp", "proj", "tool-outputs", "session-sid-1"))
    with open(os.path.join(root, "tmp", "proj", "chats", "session-a.json"), "w") as fh:
        json.dump(chat_a, fh)
    with open(os.path.join(root, "tmp", "proj", "chats", "session-b.json"), "w") as fh:
        json.dump(chat_b, fh)
    with open(os.path.join(root, "tmp", "proj", "tool-outputs", "session-sid-1", "read_1.txt"), "w") as fh:
        fh.write("out")


def test_frozen_layout_normalizes_end_to_end(tmp_path):
    src = str(tmp_path / "live")
    _make_tree(src)
    manifest = capture_sources(
        sources={"windows_gemini": src},
        dest_dir=str(tmp_path / "captures"),
        capturers=[DirectorySourceCapturer()],
    )
    rec = manifest.by_harness("windows_gemini")
    assert rec is not None and rec.content_hash.startswith("sha256:")

    norm = WindowsGeminiNormalizer(rec.captured_path)
    assert norm.session_refs() == ["sid-1"]
    assert norm.can_read("sid-1")
    s = norm.normalize("sid-1")
    s.check()
    assert len(s.turns) == 3, "two chat files merge into one session"
    assert len(s.tools) == 1
    assert len(s.reasoning) == 1
    assert len(s.states) == 1
    assert len(s.human) == 1
    assert s.native_meta["summary"] == "probe session"
    assert s.native_meta["tool_outputs"]["files"] == 1


def test_capture_immutable_under_live_mutation(tmp_path):
    src = str(tmp_path / "live")
    _make_tree(src)
    manifest = capture_sources(
        sources={"windows_gemini": src},
        dest_dir=str(tmp_path / "captures"),
        capturers=[DirectorySourceCapturer()],
    )
    rec = manifest.by_harness("windows_gemini")
    assert rec is not None
    before = WindowsGeminiNormalizer(rec.captured_path).normalize("sid-1").stats()

    with open(os.path.join(src, "tmp", "proj", "chats", "session-c.json"), "w") as fh:
        json.dump({"sessionId": "sid-2", "messages": []}, fh)
    after = WindowsGeminiNormalizer(rec.captured_path).normalize("sid-1").stats()
    assert after == before
    assert WindowsGeminiNormalizer(rec.captured_path).session_refs() == ["sid-1"]


def test_absent_source_fails_closed(tmp_path):
    with pytest.raises(Exception):
        capture_sources(
            sources={"windows_gemini": "/nonexistent/wg-root"},
            dest_dir=str(tmp_path / "captures"),
            capturers=[DirectorySourceCapturer()],
        )
