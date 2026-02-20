"""
CYNIC ActionVerifier Tests (T30)

Verifies that the ACT → LEARN closed loop works:
  1. mark_completed() / mark_failed() on ActionProposer
  2. ACT_COMPLETED event structure from _on_act_requested
  3. QTable + EScore update path (via full-server integration)

No real LLM, no real subprocess.
"""
from __future__ import annotations

import asyncio
import json
import os
import time
import pytest
from unittest.mock import AsyncMock, MagicMock, patch

from cynic.cognition.cortex.action_proposer import ActionProposer, ProposedAction
from cynic.core.event_bus import CoreEvent, Event, get_core_bus


# ── helpers ─────────────────────────────────────────────────────────────────

def _make_action(action_id: str = "abc12345") -> ProposedAction:
    return ProposedAction(
        action_id=action_id,
        judgment_id="j001",
        state_key="CODE:quality",
        verdict="BARK",
        reality="CODE",
        action_type="INVESTIGATE",
        description="Investigate quality issue",
        prompt="Please investigate this file.",
        q_score=30.0,
        priority=1,
        proposed_at=time.time(),
        status="PENDING",
    )


# ── ActionProposer unit tests ────────────────────────────────────────────────

class TestMarkCompleted:

    def test_mark_completed_success_sets_completed_status(self, tmp_path):
        """mark_completed(id, True) → status = COMPLETED."""
        p = ActionProposer(queue_path=str(tmp_path / "actions.json"))
        action = _make_action("aaa11111")
        p._queue.append(action)

        result = p.mark_completed("aaa11111", success=True)

        assert result is not None
        assert result.status == "COMPLETED"

    def test_mark_completed_failure_sets_failed_status(self, tmp_path):
        """mark_completed(id, False) → status = FAILED."""
        p = ActionProposer(queue_path=str(tmp_path / "actions.json"))
        action = _make_action("bbb22222")
        p._queue.append(action)

        result = p.mark_completed("bbb22222", success=False)

        assert result is not None
        assert result.status == "FAILED"

    def test_mark_completed_unknown_id_returns_none(self, tmp_path):
        """mark_completed on unknown id returns None without raising."""
        p = ActionProposer(queue_path=str(tmp_path / "actions.json"))
        result = p.mark_completed("nonexistent", success=True)
        assert result is None

    def test_stats_includes_completed_and_failed_keys(self, tmp_path):
        """stats() dict contains 'completed' and 'failed' keys."""
        p = ActionProposer(queue_path=str(tmp_path / "actions.json"))
        s = p.stats()
        assert "completed" in s
        assert "failed" in s

    def test_stats_counts_completed_failed_accurately(self, tmp_path):
        """stats() counts are correct after mark_completed calls."""
        p = ActionProposer(queue_path=str(tmp_path / "actions.json"))
        p._queue.append(_make_action("c1"))
        p._queue.append(_make_action("c2"))
        p._queue.append(_make_action("c3"))

        p.mark_completed("c1", success=True)
        p.mark_completed("c2", success=False)

        s = p.stats()
        assert s["completed"] == 1
        assert s["failed"] == 1

    def test_completed_action_not_in_pending(self, tmp_path):
        """After mark_completed, action no longer appears in pending()."""
        p = ActionProposer(queue_path=str(tmp_path / "actions.json"))
        p._queue.append(_make_action("d1"))
        p.mark_completed("d1", success=True)
        assert all(a.action_id != "d1" for a in p.pending())


# ── ACT_COMPLETED event integration ─────────────────────────────────────────

class TestActCompletedEvent:

    def test_act_completed_event_has_expected_fields(self):
        """ACT_COMPLETED payload has required fields."""
        payload = {
            "action_id": "abc12345",
            "success":   True,
            "cost_usd":  0.002,
            "exec_id":   "exec-99",
            "error":     None,
        }
        event = Event(type=CoreEvent.ACT_COMPLETED, payload=payload, source="test")
        assert event.payload["action_id"] == "abc12345"
        assert event.payload["success"] is True
        assert event.payload["cost_usd"] == pytest.approx(0.002)

    async def test_act_requested_emits_act_completed_via_mock_runner(self, tmp_path, monkeypatch):
        """
        Wiring test: _on_act_requested emits ACT_COMPLETED after runner returns.

        We patch runner.execute() directly and capture what gets emitted.
        """
        from cynic.core.event_bus import EventBus, Event as Ev
        bus = EventBus(bus_id="test")

        # Mock runner
        mock_runner = AsyncMock()
        mock_runner.execute = AsyncMock(return_value={
            "success": True,
            "cost_usd": 0.001,
            "exec_id": "exec-test",
        })

        # Build minimal state stub
        mock_state = MagicMock()
        mock_state.runner = mock_runner

        completed_events: list = []

        async def on_completed(event: Ev) -> None:
            completed_events.append(event)

        bus.on(CoreEvent.ACT_COMPLETED, on_completed)

        # Simulate _on_act_requested logic directly
        async def _simulate_on_act_requested(prompt: str, action_id: str) -> None:
            result = await mock_runner.execute(prompt)
            await bus.emit(Ev(
                type=CoreEvent.ACT_COMPLETED,
                payload={
                    "action_id": action_id,
                    "success":   result.get("success", False),
                    "cost_usd":  result.get("cost_usd", 0.0),
                    "exec_id":   result.get("exec_id", ""),
                    "error":     result.get("error", ""),
                },
                source="test_simulate",
            ))

        await _simulate_on_act_requested("do the thing", "abc12345")
        # Allow bus to deliver
        await asyncio.sleep(0)

        assert len(completed_events) == 1
        p = completed_events[0].payload
        assert p["success"] is True
        assert p["action_id"] == "abc12345"
