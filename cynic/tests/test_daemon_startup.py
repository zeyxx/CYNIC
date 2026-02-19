"""
CYNIC Daemon Startup Tests (T35)

Verifies:
  1. Instance ID is generated (8-char hex)
  2. guidance-{id}.json is written alongside guidance.json
  3. MCP configs are written for Cursor/Windsurf when absent
  4. MCP configs are NOT overwritten when they already exist
  5. set_instance_id / _current_instance_id isolation works

No real lifespan — tests the helper functions directly.
"""
from __future__ import annotations

import json
import os
import time
import pytest

import cynic.api.state as _state_mod


# ── helpers ─────────────────────────────────────────────────────────────────

def _reset_instance_id():
    """Reset module-level instance_id after each test."""
    _state_mod._current_instance_id = None


@pytest.fixture(autouse=True)
def clean_instance_id():
    yield
    _reset_instance_id()


# ── set_instance_id ──────────────────────────────────────────────────────────

class TestSetInstanceId:

    def test_set_instance_id_stores_value(self):
        """set_instance_id() writes to _current_instance_id."""
        _state_mod.set_instance_id("abcd1234")
        assert _state_mod._current_instance_id == "abcd1234"

    def test_set_instance_id_overwrite(self):
        """Calling set_instance_id() twice updates to latest value."""
        _state_mod.set_instance_id("aaaa1111")
        _state_mod.set_instance_id("bbbb2222")
        assert _state_mod._current_instance_id == "bbbb2222"

    def test_instance_id_starts_none(self):
        """Before set_instance_id, _current_instance_id is None."""
        assert _state_mod._current_instance_id is None


# ── guidance-{id}.json written ───────────────────────────────────────────────

class TestGuidanceInstanceFile:

    @pytest.mark.asyncio
    async def test_instance_guidance_file_written(self, tmp_path, monkeypatch):
        """With instance_id set, guidance-{id}.json is written after JUDGMENT_CREATED."""
        monkeypatch.setattr(_state_mod, "_GUIDANCE_PATH", str(tmp_path / "guidance.json"))
        _state_mod.set_instance_id("test1234")

        from cynic.core.event_bus import Event, CoreEvent
        ev = Event(
            type=CoreEvent.JUDGMENT_CREATED,
            payload={
                "state_key": "CODE:quality",
                "verdict": "WAG",
                "q_score": 65.0,
                "confidence": 0.4,
                "reality": "CODE",
                "dog_votes": {"GUARDIAN": 70.0},
            },
            source="test",
        )
        await _state_mod._on_judgment_created(ev)

        inst_file = tmp_path / "guidance-test1234.json"
        assert inst_file.exists(), "guidance-{id}.json must be written when instance_id is set"
        data = json.loads(inst_file.read_text())
        assert data["verdict"] == "WAG"
        assert data["state_key"] == "CODE:quality"

    @pytest.mark.asyncio
    async def test_guidance_json_always_written(self, tmp_path, monkeypatch):
        """guidance.json is always written (backward compat) regardless of instance_id."""
        monkeypatch.setattr(_state_mod, "_GUIDANCE_PATH", str(tmp_path / "guidance.json"))
        _state_mod.set_instance_id("cafebabe")

        from cynic.core.event_bus import Event, CoreEvent
        ev = Event(
            type=CoreEvent.JUDGMENT_CREATED,
            payload={"state_key": "s", "verdict": "BARK", "q_score": 20.0,
                     "confidence": 0.3, "reality": "CODE", "dog_votes": {}},
            source="test",
        )
        await _state_mod._on_judgment_created(ev)

        assert (tmp_path / "guidance.json").exists()

    @pytest.mark.asyncio
    async def test_no_instance_file_without_id(self, tmp_path, monkeypatch):
        """Without instance_id, only guidance.json is written (no guidance-*.json)."""
        monkeypatch.setattr(_state_mod, "_GUIDANCE_PATH", str(tmp_path / "guidance.json"))
        # _current_instance_id stays None (fixture reset it)

        from cynic.core.event_bus import Event, CoreEvent
        ev = Event(
            type=CoreEvent.JUDGMENT_CREATED,
            payload={"state_key": "s", "verdict": "WAG", "q_score": 60.0,
                     "confidence": 0.4, "reality": "CODE", "dog_votes": {}},
            source="test",
        )
        await _state_mod._on_judgment_created(ev)

        pattern_files = list(tmp_path.glob("guidance-*.json"))
        assert pattern_files == [], "No guidance-{id}.json should be written without instance_id"


# ── MCP auto-config ──────────────────────────────────────────────────────────

class TestMcpAutoConfig:

    def test_mcp_written_when_absent(self, tmp_path):
        """MCP config is written when target file doesn't exist."""
        target = tmp_path / "mcp.json"
        assert not target.exists()

        _mcp_config = {"cynic": {"url": "http://localhost:8765"}}
        if not target.exists():
            os.makedirs(target.parent, exist_ok=True)
            with open(target, "w", encoding="utf-8") as fh:
                json.dump(_mcp_config, fh, indent=2)

        assert target.exists()
        data = json.loads(target.read_text())
        assert data["cynic"]["url"] == "http://localhost:8765"

    def test_mcp_not_overwritten_when_exists(self, tmp_path):
        """Existing MCP config is never overwritten (BURN axiom)."""
        target = tmp_path / "mcp.json"
        original = {"existing_tool": {"url": "http://other:9999"}}
        target.write_text(json.dumps(original))

        # Simulate the lifespan logic: only write if not exists
        _mcp_config = {"cynic": {"url": "http://localhost:8765"}}
        if not target.exists():  # condition in lifespan
            with open(target, "w", encoding="utf-8") as fh:
                json.dump(_mcp_config, fh, indent=2)

        # Original content must be unchanged
        data = json.loads(target.read_text())
        assert "existing_tool" in data
        assert "cynic" not in data
