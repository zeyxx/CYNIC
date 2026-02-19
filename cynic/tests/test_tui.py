"""
Tests for CYNIC TUI (cynic.tui.app).

No real Textual App.run() — tests cover helpers and widget logic
that can be exercised without a terminal.
"""
from __future__ import annotations

import json
import os
import time
from pathlib import Path
from typing import Dict
from unittest.mock import patch, MagicMock

import pytest

from cynic.tui.app import (
    _bar,
    _read_json,
    _fmt_uptime,
    _local_set_status,
    ActionsPanel,
    DogsPanel,
    VERDICT_EMOJI,
    VERDICT_COLOR,
    DOGS_ORDER,
)


# ── _bar ─────────────────────────────────────────────────────────────────────

class TestBar:
    def test_zero(self):
        assert _bar(0) == "░░░░░░░░░░"

    def test_full(self):
        assert _bar(100) == "██████████"

    def test_half(self):
        assert _bar(50) == "█████░░░░░"

    def test_phi_inv(self):
        # 61.8 → 6 filled chars
        b = _bar(61.8)
        assert b.count("█") == 6
        assert b.count("░") == 4

    def test_phi_inv_2(self):
        # 38.2 → 4 filled chars
        b = _bar(38.2)
        assert b.count("█") == 4

    def test_clamp_over(self):
        assert _bar(150) == "██████████"

    def test_clamp_under(self):
        assert _bar(-10) == "░░░░░░░░░░"

    def test_width(self):
        b = _bar(50, width=6)
        assert len(b) == 6
        assert b.count("█") == 3


# ── _fmt_uptime ───────────────────────────────────────────────────────────────

class TestFmtUptime:
    def test_seconds(self):
        assert _fmt_uptime(45) == "45s"

    def test_minutes(self):
        assert _fmt_uptime(90) == "2m"

    def test_hours(self):
        result = _fmt_uptime(7200)
        assert "2.0h" in result

    def test_zero(self):
        assert _fmt_uptime(0) == "0s"


# ── _read_json ────────────────────────────────────────────────────────────────

class TestReadJson:
    def test_valid_file(self, tmp_path: Path):
        f = tmp_path / "test.json"
        f.write_text('{"verdict": "HOWL", "q_score": 83.2}')
        result = _read_json(f)
        assert result == {"verdict": "HOWL", "q_score": 83.2}

    def test_missing_file(self, tmp_path: Path):
        assert _read_json(tmp_path / "nope.json") is None

    def test_invalid_json(self, tmp_path: Path):
        f = tmp_path / "bad.json"
        f.write_text("{not valid json")
        assert _read_json(f) is None

    def test_list_json(self, tmp_path: Path):
        f = tmp_path / "list.json"
        f.write_text('[{"id": 1}, {"id": 2}]')
        result = _read_json(f)
        assert isinstance(result, list)
        assert len(result) == 2


# ── _local_set_status ─────────────────────────────────────────────────────────

class TestLocalSetStatus:
    def test_updates_matching_action(self, tmp_path: Path):
        actions = [
            {"action_id": "abc123", "status": "PENDING"},
            {"action_id": "def456", "status": "PENDING"},
        ]
        f = tmp_path / "actions.json"
        f.write_text(json.dumps(actions))

        with patch("cynic.tui.app.ACTIONS_FILE", f):
            _local_set_status("abc123", "ACCEPTED")

        result = json.loads(f.read_text())
        assert result[0]["status"] == "ACCEPTED"
        assert result[1]["status"] == "PENDING"  # unchanged

    def test_no_match_is_noop(self, tmp_path: Path):
        actions = [{"action_id": "abc123", "status": "PENDING"}]
        f = tmp_path / "actions.json"
        f.write_text(json.dumps(actions))

        with patch("cynic.tui.app.ACTIONS_FILE", f):
            _local_set_status("zzz999", "ACCEPTED")

        result = json.loads(f.read_text())
        assert result[0]["status"] == "PENDING"  # unchanged

    def test_missing_file_is_noop(self, tmp_path: Path):
        """Should not raise even if file doesn't exist."""
        with patch("cynic.tui.app.ACTIONS_FILE", tmp_path / "missing.json"):
            _local_set_status("abc", "ACCEPTED")  # no exception


# ── ActionsPanel ──────────────────────────────────────────────────────────────

class TestActionsPanel:
    def _make_action(self, action_id: str, status: str = "PENDING", verdict: str = "BARK") -> Dict:
        return {
            "action_id": action_id,
            "status": status,
            "verdict": verdict,
            "action_type": "INVESTIGATE",
            "q_score": 5.0,
            "reality": "CODE",
            "description": f"Test action {action_id}",
        }

    def test_only_pending_shown(self):
        panel = ActionsPanel()
        panel.refresh_actions([
            self._make_action("a1", "PENDING"),
            self._make_action("a2", "ACCEPTED"),
            self._make_action("a3", "REJECTED"),
            self._make_action("a4", "PENDING"),
        ])
        assert len(panel._actions) == 2
        assert panel._actions[0]["action_id"] == "a1"
        assert panel._actions[1]["action_id"] == "a4"

    def test_selected_starts_at_zero(self):
        panel = ActionsPanel()
        panel.refresh_actions([self._make_action("a1"), self._make_action("a2")])
        assert panel._selected == 0
        assert panel.selected_action["action_id"] == "a1"  # type: ignore[index]

    def test_move_down(self):
        panel = ActionsPanel()
        panel.refresh_actions([self._make_action("a1"), self._make_action("a2")])
        panel.move(1)
        assert panel.selected_action["action_id"] == "a2"  # type: ignore[index]

    def test_move_wraps_around(self):
        panel = ActionsPanel()
        panel.refresh_actions([self._make_action("a1"), self._make_action("a2")])
        panel.move(2)  # 0 + 2 = 2 → wraps to 0
        assert panel.selected_action["action_id"] == "a1"  # type: ignore[index]

    def test_move_up_wraps(self):
        panel = ActionsPanel()
        panel.refresh_actions([self._make_action("a1"), self._make_action("a2")])
        panel.move(-1)  # 0 - 1 = -1 → wraps to 1
        assert panel.selected_action["action_id"] == "a2"  # type: ignore[index]

    def test_no_actions_selected_is_none(self):
        panel = ActionsPanel()
        panel.refresh_actions([])
        assert panel.selected_action is None

    def test_selection_clamped_after_refresh(self):
        panel = ActionsPanel()
        panel.refresh_actions([
            self._make_action("a1"),
            self._make_action("a2"),
            self._make_action("a3"),
        ])
        panel._selected = 2
        # Now refresh with fewer actions
        panel.refresh_actions([self._make_action("b1")])
        assert panel._selected == 0
        assert panel.selected_action["action_id"] == "b1"  # type: ignore[index]

    def test_render_no_crash_empty(self):
        panel = ActionsPanel()
        panel.refresh_actions([])
        # _render() called internally, just ensure no exception

    def test_render_no_crash_with_actions(self):
        panel = ActionsPanel()
        panel.refresh_actions([
            self._make_action("abc123", "PENDING", "HOWL"),
            self._make_action("def456", "PENDING", "BARK"),
        ])
        # Should not raise

    def test_verdict_colors_complete(self):
        for verdict in ("HOWL", "WAG", "GROWL", "BARK"):
            assert verdict in VERDICT_COLOR
            assert verdict in VERDICT_EMOJI

    def test_all_dogs_in_order(self):
        assert "GUARDIAN" in DOGS_ORDER
        assert "SAGE" in DOGS_ORDER
        assert "CYNIC" in DOGS_ORDER
        assert len(DOGS_ORDER) >= 10


# ── DogsPanel ─────────────────────────────────────────────────────────────────

class TestDogsPanel:
    def test_render_with_votes(self):
        panel = DogsPanel()
        # Should not raise
        panel.render_dogs(
            {"GUARDIAN": 82.3, "ANALYST": 61.8, "SAGE": 38.2},
            {},
        )

    def test_render_empty_votes(self):
        panel = DogsPanel()
        panel.render_dogs({}, {})

    def test_render_uses_escore_fallback(self):
        panel = DogsPanel()
        # dog_votes empty → falls back to escore
        panel.render_dogs({}, {"GUARDIAN": 75.0})
        # No exception means fallback worked

    def test_render_all_unknown(self):
        panel = DogsPanel()
        panel.render_dogs({}, {})
        # All dogs should render with '?' bar


# ── Constants ─────────────────────────────────────────────────────────────────

class TestConstants:
    def test_verdict_emoji_all_verdicts(self):
        for v in ("HOWL", "WAG", "GROWL", "BARK"):
            assert v in VERDICT_EMOJI, f"Missing emoji for {v}"

    def test_verdict_color_all_verdicts(self):
        for v in ("HOWL", "WAG", "GROWL", "BARK"):
            assert v in VERDICT_COLOR, f"Missing color for {v}"

    def test_dogs_order_no_duplicates(self):
        assert len(DOGS_ORDER) == len(set(DOGS_ORDER))
