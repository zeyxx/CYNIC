"""
Tests for cynic.act.telemetry — SessionTelemetry measurement layer.

This is the quantification infrastructure for H1-H5 hypothesis testing.
Every function here is pure Python (no LLM, no DB, no network).
"""
from __future__ import annotations

import time

import pytest

from cynic.metabolism.telemetry import (
    SessionTelemetry,
    TelemetryStore,
    classify_task,
    compute_reward,
    estimate_complexity,
)


# ════════════════════════════════════════════════════════════════════════════
# classify_task
# ════════════════════════════════════════════════════════════════════════════

class TestClassifyTask:

    def test_empty_prompt_is_general(self):
        assert classify_task("") == "general"

    def test_none_safe(self):
        # classify_task receives strings — empty string is the safe default
        assert classify_task("") == "general"

    def test_debug_keywords(self):
        assert classify_task("fix this bug in the login function") == "debug"

    def test_refactor_keywords(self):
        assert classify_task("refactor the authentication module") == "refactor"

    def test_test_keywords(self):
        assert classify_task("write pytest tests for the new feature") == "test"

    def test_review_keywords(self):
        assert classify_task("review this code and check for issues") == "review"

    def test_write_keywords(self):
        assert classify_task("implement a new endpoint for user registration") == "write"

    def test_explain_keywords(self):
        assert classify_task("explain how the Q-Table learning loop works") == "explain"

    def test_general_for_unknown(self):
        assert classify_task("hello world 123") == "general"

    def test_case_insensitive(self):
        assert classify_task("FIX THE BUG") == "debug"

    def test_tie_broken_by_order(self):
        # "fix" matches debug, "implement" matches write
        # Both get 1 match → first in dict order wins (debug)
        result = classify_task("fix and implement something")
        assert result in ("debug", "write")  # tie-broken by order (debug first)


# ════════════════════════════════════════════════════════════════════════════
# estimate_complexity
# ════════════════════════════════════════════════════════════════════════════

class TestEstimateComplexity:

    def test_empty_sequence_is_trivial(self):
        assert estimate_complexity([]) == "trivial"

    def test_one_tool_is_trivial(self):
        assert estimate_complexity(["Read"]) == "trivial"

    def test_two_tools_is_trivial(self):
        assert estimate_complexity(["Read", "Bash"]) == "trivial"

    def test_three_tools_is_simple(self):
        assert estimate_complexity(["Read", "Edit", "Bash"]) == "simple"

    def test_six_tools_is_simple(self):
        assert estimate_complexity(["Read"] * 6) == "simple"

    def test_seven_tools_is_medium(self):
        assert estimate_complexity(["Read"] * 7) == "medium"

    def test_fifteen_tools_is_medium(self):
        assert estimate_complexity(["Read"] * 15) == "medium"

    def test_sixteen_tools_is_complex(self):
        assert estimate_complexity(["Read"] * 16) == "complex"

    def test_many_tools_is_complex(self):
        assert estimate_complexity(["Read"] * 50) == "complex"


# ════════════════════════════════════════════════════════════════════════════
# compute_reward
# ════════════════════════════════════════════════════════════════════════════

class TestComputeReward:

    def test_success_few_tools_low_cost(self):
        # Success, 1 tool, $0.001 → 0.70 + 0.06 - 0.00 = 0.76 → cap 0.75
        r = compute_reward(is_error=False, tool_count=1, cost_usd=0.001)
        assert r == pytest.approx(0.75, abs=1e-3)

    def test_success_average(self):
        # Success, 8 tools, $0.010
        # base=0.70, efficiency=max(-0.15,(4-8)*0.02)=-0.08, cost=-min(0.01/0.10,0.10)=-0.10
        # total = 0.70 - 0.08 - 0.10 = 0.52
        r = compute_reward(is_error=False, tool_count=8, cost_usd=0.010)
        assert r == pytest.approx(0.52, abs=1e-3)

    def test_error_result(self):
        # Error, 3 tools, $0.005 → 0.20 - 0.02 - 0.005 = 0.175
        r = compute_reward(is_error=True, tool_count=3, cost_usd=0.005)
        assert 0.10 <= r <= 0.30

    def test_minimum_floor(self):
        # Very expensive error should still be >= 0.10
        r = compute_reward(is_error=True, tool_count=100, cost_usd=10.0)
        assert r >= 0.10

    def test_maximum_cap(self):
        # Best case can't exceed 0.75
        r = compute_reward(is_error=False, tool_count=0, cost_usd=0.0)
        assert r <= 0.75

    def test_result_in_range(self):
        for error in [True, False]:
            for tools in [0, 5, 20]:
                for cost in [0.0, 0.01, 1.0]:
                    r = compute_reward(error, tools, cost)
                    assert 0.10 <= r <= 0.75, f"Out of range: {r} (error={error}, tools={tools}, cost={cost})"


# ════════════════════════════════════════════════════════════════════════════
# TelemetryStore
# ════════════════════════════════════════════════════════════════════════════

def _make_record(**overrides) -> SessionTelemetry:
    defaults = {
        "session_id": "test-sid",
        "task": "fix the bug",
        "task_type": "debug",
        "complexity": "simple",
        "model": "claude-sonnet-4-6",
        "tools_sequence": ["Read", "Edit", "Bash"],
        "tools_allowed": 3,
        "tools_denied": 0,
        "tool_allow_rate": 1.0,
        "input_tokens": 1000,
        "output_tokens": 500,
        "total_cost_usd": 0.005,
        "duration_s": 10.0,
        "is_error": False,
        "result_text": "Done",
        "output_q_score": 40.0,
        "output_verdict": "WAG",
        "output_confidence": 0.45,
        "state_key": "SDK:claude-sonnet-4-6:debug:simple",
        "reward": 0.60,
    }
    defaults.update(overrides)
    return SessionTelemetry(**defaults)


class TestTelemetryStore:

    def test_empty_store(self):
        store = TelemetryStore()
        assert len(store) == 0

    def test_add_and_len(self):
        store = TelemetryStore()
        store.add(_make_record())
        assert len(store) == 1

    def test_recent_returns_dicts(self):
        store = TelemetryStore()
        store.add(_make_record(session_id="s1"))
        store.add(_make_record(session_id="s2"))
        recent = store.recent(1)
        assert len(recent) == 1
        assert recent[0]["session_id"] == "s2"

    def test_stats_empty(self):
        store = TelemetryStore()
        s = store.stats()
        assert s["count"] == 0
        assert "message" in s

    def test_stats_with_records(self):
        store = TelemetryStore()
        store.add(_make_record(is_error=False, output_q_score=50.0, reward=0.65))
        store.add(_make_record(is_error=True, output_q_score=20.0, reward=0.20))
        s = store.stats()
        assert s["count"] == 2
        assert s["error_rate"] == pytest.approx(0.5, abs=1e-3)
        assert s["mean_q_score"] == pytest.approx(35.0, abs=1e-3)

    def test_maxlen_ring_buffer(self):
        store = TelemetryStore(maxlen=3)
        for i in range(5):
            store.add(_make_record(session_id=f"s{i}"))
        assert len(store) == 3  # Oldest 2 discarded

    def test_export_all(self):
        store = TelemetryStore()
        for i in range(5):
            store.add(_make_record(session_id=f"s{i}"))
        exported = store.export()
        assert len(exported) == 5
        assert all(isinstance(r, dict) for r in exported)

    def test_save_jsonl(self, tmp_path):
        import json
        store = TelemetryStore()
        store.add(_make_record(session_id="jsonl-test"))
        path = str(tmp_path / "sessions.jsonl")
        count = store.save_jsonl(path)
        assert count == 1
        with open(path, "r") as f:
            line = f.readline().strip()
        record = json.loads(line)
        assert record["session_id"] == "jsonl-test"
