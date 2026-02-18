"""
CYNIC ActionProposer Tests (P5)

Tests ProposedAction creation, action type mapping, queue management,
accept/reject lifecycle, persistence, and rolling cap.
No LLM, no DB, no event bus — pure in-memory unit tests.
"""
from __future__ import annotations

import json
import os
import pytest
import asyncio
import tempfile
from unittest.mock import AsyncMock, MagicMock, patch

from cynic.judge.action_proposer import (
    ActionProposer,
    ProposedAction,
    _action_type_and_priority,
    _description,
    _MAX_QUEUE,
)
from cynic.core.event_bus import CoreEvent, Event


# ── Helpers ───────────────────────────────────────────────────────────────────

def _make_decision_event(
    verdict="BARK",
    reality="CODE",
    state_key="CODE:JUDGE:PRESENT:0",
    judgment_id="jid-001",
    q_value=0.3,
    content_preview="null pointer in auth.py",
    action_prompt="[CYNIC AUTO-ACT] BARK...",
) -> Event:
    return Event(
        type=CoreEvent.DECISION_MADE,
        payload={
            "recommended_action": verdict,
            "judgment_id":        judgment_id,
            "state_key":          state_key,
            "reality":            reality,
            "content_preview":    content_preview,
            "action_prompt":      action_prompt,
            "q_value":            q_value,
        },
        source="decide_agent",
    )


def _proposer_with_tmp(tmp_path) -> ActionProposer:
    """ActionProposer backed by a temp dir, bypassing ~/.cynic."""
    path = str(tmp_path / "pending_actions.json")
    return ActionProposer(queue_path=path)


# ── _action_type_and_priority ─────────────────────────────────────────────────

class TestActionTypeMapping:
    def test_bark_code_is_investigate_priority_1(self):
        t, p = _action_type_and_priority("BARK", "CODE")
        assert t == "INVESTIGATE"
        assert p == 1

    def test_bark_cynic_is_investigate_priority_1(self):
        t, p = _action_type_and_priority("BARK", "CYNIC")
        assert t == "INVESTIGATE"
        assert p == 1

    def test_bark_market_is_alert_priority_2(self):
        t, p = _action_type_and_priority("BARK", "MARKET")
        assert t == "ALERT"
        assert p == 2

    def test_growl_code_is_refactor_priority_2(self):
        t, p = _action_type_and_priority("GROWL", "CODE")
        assert t == "REFACTOR"
        assert p == 2

    def test_growl_cynic_is_refactor_priority_2(self):
        t, p = _action_type_and_priority("GROWL", "CYNIC")
        assert t == "REFACTOR"
        assert p == 2

    def test_growl_social_is_monitor_priority_3(self):
        t, p = _action_type_and_priority("GROWL", "SOCIAL")
        assert t == "MONITOR"
        assert p == 3

    def test_bark_solana_is_alert_priority_2(self):
        t, p = _action_type_and_priority("BARK", "SOLANA")
        assert t == "ALERT"
        assert p == 2


# ── _description ──────────────────────────────────────────────────────────────

class TestDescriptionBuilder:
    def test_investigate_description_contains_verdict(self):
        d = _description("BARK", "CODE", "INVESTIGATE", "null pointer")
        assert "BARK" in d
        assert "INVESTIGATE" in d.upper() or "Investigate" in d

    def test_description_max_120_chars(self):
        long_preview = "x" * 200
        d = _description("BARK", "CODE", "INVESTIGATE", long_preview)
        assert len(d) <= 120

    def test_empty_preview_still_produces_description(self):
        d = _description("GROWL", "CODE", "REFACTOR", "")
        assert len(d) > 10

    def test_monitor_description_contains_reality(self):
        d = _description("GROWL", "SOCIAL", "MONITOR", "")
        assert "SOCIAL" in d


# ── ProposedAction dataclass ──────────────────────────────────────────────────

class TestProposedAction:
    def _make(self) -> ProposedAction:
        return ProposedAction(
            action_id="abc12345",
            judgment_id="jid-001",
            state_key="CODE:JUDGE:PRESENT:0",
            verdict="BARK",
            reality="CODE",
            action_type="INVESTIGATE",
            description="[BARK] Investigate critical issue",
            prompt="Please fix the null pointer",
            q_score=0.3,
            priority=1,
            proposed_at=1234567890.0,
            status="PENDING",
        )

    def test_default_status_pending(self):
        a = self._make()
        assert a.status == "PENDING"

    def test_to_dict_roundtrip(self):
        a = self._make()
        d = a.to_dict()
        b = ProposedAction.from_dict(d)
        assert b.action_id == a.action_id
        assert b.verdict == a.verdict
        assert b.status == a.status

    def test_to_dict_has_all_fields(self):
        a = self._make()
        d = a.to_dict()
        for key in ["action_id", "judgment_id", "state_key", "verdict", "reality",
                    "action_type", "description", "prompt", "q_score", "priority",
                    "proposed_at", "status"]:
            assert key in d


# ── ActionProposer core ───────────────────────────────────────────────────────

class TestActionProposerCore:
    def test_initial_state_empty(self, tmp_path):
        ap = _proposer_with_tmp(tmp_path)
        assert ap.pending() == []
        assert ap.all_actions() == []

    def test_on_decision_made_creates_action(self, tmp_path):
        ap = _proposer_with_tmp(tmp_path)
        event = _make_decision_event(verdict="BARK", reality="CODE")

        # Patch bus emit so we don't need a real event loop for the emit call
        with patch.object(ap, "_on_decision_made", wraps=ap._on_decision_made):
            asyncio.get_event_loop().run_until_complete(ap._on_decision_made(event))

        assert len(ap.all_actions()) == 1
        action = ap.all_actions()[0]
        assert action.verdict == "BARK"
        assert action.reality == "CODE"
        assert action.action_type == "INVESTIGATE"
        assert action.priority == 1
        assert action.status == "PENDING"

    def test_action_id_is_8_chars(self, tmp_path):
        ap = _proposer_with_tmp(tmp_path)
        event = _make_decision_event()
        with patch("cynic.judge.action_proposer.get_core_bus") as mock_bus:
            mock_bus.return_value.emit = AsyncMock()
            asyncio.get_event_loop().run_until_complete(ap._on_decision_made(event))
        assert len(ap.all_actions()[0].action_id) == 8

    def test_multiple_events_stack(self, tmp_path):
        ap = _proposer_with_tmp(tmp_path)
        with patch("cynic.judge.action_proposer.get_core_bus") as mock_bus:
            mock_bus.return_value.emit = AsyncMock()
            for _ in range(5):
                asyncio.get_event_loop().run_until_complete(
                    ap._on_decision_made(_make_decision_event())
                )
        assert len(ap.all_actions()) == 5

    def test_pending_sorted_by_priority(self, tmp_path):
        ap = _proposer_with_tmp(tmp_path)
        with patch("cynic.judge.action_proposer.get_core_bus") as mock_bus:
            mock_bus.return_value.emit = AsyncMock()
            # GROWL CODE → priority 2
            asyncio.get_event_loop().run_until_complete(
                ap._on_decision_made(_make_decision_event(verdict="GROWL", reality="CODE"))
            )
            # BARK CODE → priority 1
            asyncio.get_event_loop().run_until_complete(
                ap._on_decision_made(_make_decision_event(verdict="BARK", reality="CODE"))
            )

        pending = ap.pending()
        assert pending[0].priority == 1   # BARK first
        assert pending[1].priority == 2   # GROWL second

    def test_growl_market_action_type_monitor(self, tmp_path):
        ap = _proposer_with_tmp(tmp_path)
        with patch("cynic.judge.action_proposer.get_core_bus") as mock_bus:
            mock_bus.return_value.emit = AsyncMock()
            asyncio.get_event_loop().run_until_complete(
                ap._on_decision_made(_make_decision_event(verdict="GROWL", reality="MARKET"))
            )
        assert ap.all_actions()[0].action_type == "MONITOR"


# ── Accept / Reject lifecycle ─────────────────────────────────────────────────

class TestActionLifecycle:
    def _setup(self, tmp_path) -> tuple:
        ap = _proposer_with_tmp(tmp_path)
        with patch("cynic.judge.action_proposer.get_core_bus") as mock_bus:
            mock_bus.return_value.emit = AsyncMock()
            asyncio.get_event_loop().run_until_complete(
                ap._on_decision_made(_make_decision_event())
            )
        action = ap.all_actions()[0]
        return ap, action

    def test_accept_changes_status(self, tmp_path):
        ap, action = self._setup(tmp_path)
        result = ap.accept(action.action_id)
        assert result is not None
        assert result.status == "ACCEPTED"

    def test_reject_changes_status(self, tmp_path):
        ap, action = self._setup(tmp_path)
        result = ap.reject(action.action_id)
        assert result is not None
        assert result.status == "REJECTED"

    def test_auto_executed_changes_status(self, tmp_path):
        ap, action = self._setup(tmp_path)
        result = ap.mark_auto_executed(action.action_id)
        assert result is not None
        assert result.status == "AUTO_EXECUTED"

    def test_accepted_action_not_in_pending(self, tmp_path):
        ap, action = self._setup(tmp_path)
        ap.accept(action.action_id)
        assert action.action_id not in [a.action_id for a in ap.pending()]

    def test_rejected_action_not_in_pending(self, tmp_path):
        ap, action = self._setup(tmp_path)
        ap.reject(action.action_id)
        assert action.action_id not in [a.action_id for a in ap.pending()]

    def test_unknown_action_id_returns_none(self, tmp_path):
        ap, _ = self._setup(tmp_path)
        result = ap.accept("deadbeef")
        assert result is None

    def test_get_by_id(self, tmp_path):
        ap, action = self._setup(tmp_path)
        found = ap.get(action.action_id)
        assert found is not None
        assert found.action_id == action.action_id

    def test_get_unknown_returns_none(self, tmp_path):
        ap, _ = self._setup(tmp_path)
        assert ap.get("00000000") is None


# ── Persistence ───────────────────────────────────────────────────────────────

class TestPersistence:
    def test_save_creates_json_file(self, tmp_path):
        ap = _proposer_with_tmp(tmp_path)
        with patch("cynic.judge.action_proposer.get_core_bus") as mock_bus:
            mock_bus.return_value.emit = AsyncMock()
            asyncio.get_event_loop().run_until_complete(
                ap._on_decision_made(_make_decision_event())
            )
        path = str(tmp_path / "pending_actions.json")
        assert os.path.exists(path)

    def test_saved_json_is_list(self, tmp_path):
        ap = _proposer_with_tmp(tmp_path)
        with patch("cynic.judge.action_proposer.get_core_bus") as mock_bus:
            mock_bus.return_value.emit = AsyncMock()
            asyncio.get_event_loop().run_until_complete(
                ap._on_decision_made(_make_decision_event())
            )
        path = str(tmp_path / "pending_actions.json")
        with open(path) as fh:
            data = json.load(fh)
        assert isinstance(data, list)
        assert len(data) == 1

    def test_load_restores_actions(self, tmp_path):
        # Write then load
        ap1 = _proposer_with_tmp(tmp_path)
        with patch("cynic.judge.action_proposer.get_core_bus") as mock_bus:
            mock_bus.return_value.emit = AsyncMock()
            asyncio.get_event_loop().run_until_complete(
                ap._on_decision_made(_make_decision_event())
            ) if False else None  # Skip — use ap1 directly
        with patch("cynic.judge.action_proposer.get_core_bus") as mock_bus:
            mock_bus.return_value.emit = AsyncMock()
            asyncio.get_event_loop().run_until_complete(
                ap1._on_decision_made(_make_decision_event(verdict="BARK"))
            )

        # Second instance loads from same file
        ap2 = _proposer_with_tmp(tmp_path)
        ap2._path = ap1._path
        ap2._load()
        assert len(ap2.all_actions()) == 1
        assert ap2.all_actions()[0].verdict == "BARK"

    def test_accept_persists_to_disk(self, tmp_path):
        ap = _proposer_with_tmp(tmp_path)
        with patch("cynic.judge.action_proposer.get_core_bus") as mock_bus:
            mock_bus.return_value.emit = AsyncMock()
            asyncio.get_event_loop().run_until_complete(
                ap._on_decision_made(_make_decision_event())
            )
        action = ap.all_actions()[0]
        ap.accept(action.action_id)

        path = str(tmp_path / "pending_actions.json")
        with open(path) as fh:
            data = json.load(fh)
        assert data[0]["status"] == "ACCEPTED"


# ── Rolling cap ───────────────────────────────────────────────────────────────

class TestRollingCap:
    def test_queue_capped_at_max(self, tmp_path):
        ap = _proposer_with_tmp(tmp_path)
        with patch("cynic.judge.action_proposer.get_core_bus") as mock_bus:
            mock_bus.return_value.emit = AsyncMock()
            for _ in range(_MAX_QUEUE + 10):
                asyncio.get_event_loop().run_until_complete(
                    ap._on_decision_made(_make_decision_event())
                )
        assert len(ap.all_actions()) == _MAX_QUEUE

    def test_oldest_dropped_when_cap_exceeded(self, tmp_path):
        ap = _proposer_with_tmp(tmp_path)
        first_id = None
        with patch("cynic.judge.action_proposer.get_core_bus") as mock_bus:
            mock_bus.return_value.emit = AsyncMock()
            for i in range(_MAX_QUEUE + 1):
                asyncio.get_event_loop().run_until_complete(
                    ap._on_decision_made(_make_decision_event(judgment_id=f"jid-{i:04d}"))
                )
                if i == 0:
                    first_id = ap.all_actions()[0].action_id

        # First entry should have been dropped
        ids = [a.action_id for a in ap.all_actions()]
        assert first_id not in ids


# ── Stats ─────────────────────────────────────────────────────────────────────

class TestStats:
    def test_stats_initial(self, tmp_path):
        ap = _proposer_with_tmp(tmp_path)
        s = ap.stats()
        assert s["proposed_total"] == 0
        assert s["queue_size"] == 0
        assert s["pending"] == 0

    def test_stats_after_proposal(self, tmp_path):
        ap = _proposer_with_tmp(tmp_path)
        with patch("cynic.judge.action_proposer.get_core_bus") as mock_bus:
            mock_bus.return_value.emit = AsyncMock()
            asyncio.get_event_loop().run_until_complete(
                ap._on_decision_made(_make_decision_event())
            )
        s = ap.stats()
        assert s["proposed_total"] == 1
        assert s["queue_size"] == 1
        assert s["pending"] == 1

    def test_stats_after_accept(self, tmp_path):
        ap = _proposer_with_tmp(tmp_path)
        with patch("cynic.judge.action_proposer.get_core_bus") as mock_bus:
            mock_bus.return_value.emit = AsyncMock()
            asyncio.get_event_loop().run_until_complete(
                ap._on_decision_made(_make_decision_event())
            )
        action = ap.all_actions()[0]
        ap.accept(action.action_id)
        s = ap.stats()
        assert s["pending"] == 0
        assert s["accepted"] == 1

    def test_stats_after_reject(self, tmp_path):
        ap = _proposer_with_tmp(tmp_path)
        with patch("cynic.judge.action_proposer.get_core_bus") as mock_bus:
            mock_bus.return_value.emit = AsyncMock()
            asyncio.get_event_loop().run_until_complete(
                ap._on_decision_made(_make_decision_event())
            )
        action = ap.all_actions()[0]
        ap.reject(action.action_id)
        s = ap.stats()
        assert s["pending"] == 0
        assert s["rejected"] == 1


# ── L1 closure — accept→execute, reject→QTable, auto_executed ─────────────────

class TestL1Closure:
    """
    Tests for the L1 Machine→Actions loop closure:
    - accept → ACT_REQUESTED fired (execution triggered)
    - reject → QTable negative reward (learning from rejection)
    - auto-ACT → mark_auto_executed() called (queue status linked)
    """

    def test_accept_fires_act_requested(self):
        """POST /actions/{id}/accept → executing=True when action has prompt."""
        import time as _t
        import uuid
        from starlette.testclient import TestClient
        from cynic.api.server import app
        from cynic.api.state import get_state
        from cynic.judge.action_proposer import ProposedAction

        # Populate the queue INSIDE TestClient context — lifespan runs first.
        with TestClient(app) as client:
            state = get_state()
            action = ProposedAction(
                action_id=uuid.uuid4().hex[:8],
                judgment_id="jid-test",
                state_key="CODE:JUDGE:PRESENT:0",
                verdict="BARK",
                reality="CODE",
                action_type="INVESTIGATE",
                description="[BARK] Investigate critical issue in CODE",
                prompt="[CYNIC AUTO-ACT] Please review the critical issue",
                q_score=0.25,
                priority=1,
                proposed_at=_t.time(),
                status="PENDING",
            )
            state.action_proposer._queue.append(action)
            state.action_proposer._proposed_total += 1

            resp = client.post(f"/actions/{action.action_id}/accept")

        assert resp.status_code == 200
        data = resp.json()
        assert data["accepted"] is True
        assert data.get("executing") is True      # ACT_REQUESTED was fired
        assert data["action"]["status"] == "ACCEPTED"

    def test_reject_returns_qtable_update(self):
        """POST /actions/{id}/reject → action REJECTED and QTable penalized."""
        import time as _t
        import uuid
        from starlette.testclient import TestClient
        from cynic.api.server import app
        from cynic.api.state import get_state
        from cynic.judge.action_proposer import ProposedAction

        state_key = "CODE:JUDGE:PRESENT:0"

        with TestClient(app) as client:
            state = get_state()
            action = ProposedAction(
                action_id=uuid.uuid4().hex[:8],
                judgment_id="jid-rej",
                state_key=state_key,
                verdict="BARK",
                reality="CODE",
                action_type="INVESTIGATE",
                description="[BARK] Investigate critical issue",
                prompt="[CYNIC AUTO-ACT] review critical issue",
                q_score=0.25,
                priority=1,
                proposed_at=_t.time(),
                status="PENDING",
            )
            state.action_proposer._queue.append(action)
            state.action_proposer._proposed_total += 1

            resp = client.post(f"/actions/{action.action_id}/reject")
            q_after = state.qtable.predict_q(state_key, "BARK")

        assert resp.status_code == 200
        data = resp.json()
        assert data["rejected"] is True
        assert data["action"]["status"] == "REJECTED"
        # Q-value updated with low reward (reject penalty — below neutral 0.5)
        assert q_after <= 0.5, f"Expected lower Q after rejection, got {q_after}"

    def test_mark_auto_executed_links_queue(self, tmp_path):
        """mark_auto_executed() updates status in the in-memory queue."""
        ap = _proposer_with_tmp(tmp_path)
        with patch("cynic.judge.action_proposer.get_core_bus") as mock_bus:
            mock_bus.return_value.emit = AsyncMock()
            asyncio.get_event_loop().run_until_complete(
                ap._on_decision_made(_make_decision_event(judgment_id="jid-auto"))
            )

        action = ap.all_actions()[0]
        assert action.status == "PENDING"

        result = ap.mark_auto_executed(action.action_id)
        assert result is not None
        assert result.status == "AUTO_EXECUTED"
        assert ap.stats()["auto_executed"] == 1
        assert ap.stats()["pending"] == 0

    def test_auto_executed_not_in_pending(self, tmp_path):
        """AUTO_EXECUTED actions don't appear in pending()."""
        ap = _proposer_with_tmp(tmp_path)
        with patch("cynic.judge.action_proposer.get_core_bus") as mock_bus:
            mock_bus.return_value.emit = AsyncMock()
            asyncio.get_event_loop().run_until_complete(
                ap._on_decision_made(_make_decision_event())
            )
        action = ap.all_actions()[0]
        ap.mark_auto_executed(action.action_id)
        assert len(ap.pending()) == 0

    def test_accept_without_prompt_not_executing(self):
        """Accept action without prompt → executing=False (no runner spawn)."""
        import time as _t
        import uuid
        from starlette.testclient import TestClient
        from cynic.api.server import app
        from cynic.api.state import get_state
        from cynic.judge.action_proposer import ProposedAction

        with TestClient(app) as client:
            state = get_state()
            action = ProposedAction(
                action_id=uuid.uuid4().hex[:8],
                judgment_id="jid-noprompt",
                state_key="CODE:JUDGE:PRESENT:0",
                verdict="GROWL",
                reality="MARKET",
                action_type="MONITOR",
                description="[GROWL] Monitor MARKET",
                prompt="",  # no prompt
                q_score=0.4,
                priority=3,
                proposed_at=_t.time(),
                status="PENDING",
            )
            state.action_proposer._queue.append(action)

            resp = client.post(f"/actions/{action.action_id}/accept")

        assert resp.status_code == 200
        assert resp.json().get("executing") is False


class TestL4P5Bridge:
    """L4→P5: SELF_IMPROVEMENT_PROPOSED → ActionProposer.propose_self_improvement()"""

    def _make_proposer(self, tmp_path):
        from cynic.judge.action_proposer import ActionProposer
        p = ActionProposer()
        p._path = str(tmp_path / "pending_actions.json")
        p._queue = []  # clear items pre-loaded from ~/.cynic/pending_actions.json
        return p

    def _sample_proposal(self, **overrides):
        import time
        base = {
            "probe_id":        "test-probe-1",
            "trigger":         "EMERGENCE",
            "pattern_type":    "SPIKE",
            "severity":        0.7,
            "dimension":       "QTABLE",
            "target":          "CODE:JUDGE:WAG",
            "recommendation":  "Increase learning rate for CODE×JUDGE",
            "current_value":   0.038,
            "suggested_value": 0.1,
            "proposed_at":     time.time(),
            "status":          "PENDING",
        }
        base.update(overrides)
        return base

    def test_propose_returns_proposed_action(self, tmp_path):
        proposer = self._make_proposer(tmp_path)
        result = proposer.propose_self_improvement(self._sample_proposal())
        assert result is not None
        assert result.action_type == "IMPROVE"
        assert result.reality == "CYNIC"
        assert result.status == "PENDING"

    def test_propose_description_contains_dimension(self, tmp_path):
        proposer = self._make_proposer(tmp_path)
        result = proposer.propose_self_improvement(self._sample_proposal())
        assert "QTABLE" in result.description

    def test_propose_high_severity_priority_2(self, tmp_path):
        """severity >= PHI_INV (0.618) → priority 2."""
        proposer = self._make_proposer(tmp_path)
        result = proposer.propose_self_improvement(self._sample_proposal(severity=0.7))
        assert result.priority == 2

    def test_propose_medium_severity_priority_3(self, tmp_path):
        """severity >= PHI_INV_2 (0.382) → priority 3."""
        proposer = self._make_proposer(tmp_path)
        result = proposer.propose_self_improvement(self._sample_proposal(severity=0.5))
        assert result.priority == 3

    def test_propose_low_severity_priority_4(self, tmp_path):
        """severity < PHI_INV_2 (0.382) → priority 4 (FYI)."""
        proposer = self._make_proposer(tmp_path)
        result = proposer.propose_self_improvement(self._sample_proposal(severity=0.2))
        assert result.priority == 4

    def test_propose_appears_in_pending(self, tmp_path):
        proposer = self._make_proposer(tmp_path)
        proposer.propose_self_improvement(self._sample_proposal())
        pending = proposer.pending()
        assert len(pending) == 1
        assert pending[0].action_type == "IMPROVE"

    def test_propose_empty_recommendation_returns_none(self, tmp_path):
        proposer = self._make_proposer(tmp_path)
        result = proposer.propose_self_improvement(self._sample_proposal(recommendation=""))
        assert result is None

    def test_propose_written_to_disk(self, tmp_path):
        import json
        proposer = self._make_proposer(tmp_path)
        proposer.propose_self_improvement(self._sample_proposal())
        saved = json.loads((tmp_path / "pending_actions.json").read_text())
        assert len(saved) == 1
        assert saved[0]["action_type"] == "IMPROVE"
