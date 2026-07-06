"""Test that hermes_agent_task_executor emits cost on subprocess call.

Tier 1 EXPERIMENTAL: integration test for agent cost emission.
"""
import json
import os
import sys
import tempfile
from pathlib import Path
import subprocess
from unittest.mock import patch, MagicMock

sys.path.insert(0, str(Path(__file__).parent.parent.parent / "scripts" / "hermes-x"))
sys.path.insert(0, str(Path(__file__).parent.parent.parent / "services" / "reliability-runtime" / "src"))
sys.path.insert(0, str(Path(__file__).parent.parent.parent / "services" / "observability-runtime" / "src"))
sys.path.insert(0, str(Path(__file__).parent.parent.parent / "services" / "coordination-runtime" / "src"))
sys.path.insert(0, str(Path(__file__).parent.parent.parent / "services" / "ouroboros-runtime" / "src"))
sys.path.insert(0, str(Path(__file__).parent.parent))

os.environ.setdefault("CYNIC_REST_ADDR", "127.0.0.1:3030")
os.environ.setdefault("CYNIC_API_KEY", "test")


def test_execute_task_emits_cost(tmp_path):
    ledger = str(tmp_path / "cost_ledger.jsonl")
    os.environ["CYNIC_COST_LEDGER"] = ledger
    os.environ["CYNIC_SESSION_ID"] = "test-session"

    def mock_run_subprocess(cmd, timeout=None):
        if cmd[:3] == ["hermes", "kanban", "create"]:
            return subprocess.CompletedProcess(cmd, 0, json.dumps({"id": "t_mock123"}), "")
        if cmd[:3] == ["hermes", "kanban", "decompose"]:
            return subprocess.CompletedProcess(cmd, 0, "", "")
        if cmd[:3] == ["hermes", "kanban", "show"]:
            return subprocess.CompletedProcess(cmd, 0, json.dumps({"task": {"status": "done"}}), "")
        if cmd[:3] == ["hermes", "kanban", "context"]:
            return subprocess.CompletedProcess(cmd, 0, "ok", "")
        raise AssertionError(f"unexpected command: {cmd}")

    with patch(
        "ouroboros_runtime.core.adapters.hermes_kanban_executor.HermesKanbanAdapter._run_subprocess",
        side_effect=mock_run_subprocess,
    ):
        from ouroboros_runtime.core.adapters.hermes_kanban_executor import HermesKanbanAdapter
        task = {
            "id": "task-001",
            "objective": "test task",
            "actions": ["do something"],
            "domain": "D3",
            "_source": "kernel",
        }
        adapter = HermesKanbanAdapter()
        result, err = adapter.execute(task, str(tmp_path))

    assert result == "ok"
    assert err is None
    lines = Path(ledger).read_text().strip().splitlines()
    assert len(lines) == 1
    event = json.loads(lines[0])
    assert event["feature_id"] == "hermes_kanban"
    assert event["compute_class"] == "tailnet"
    assert event["provider"] == "qwen36-27b-gpu"
    assert event["latency_ms"] >= 0
    assert event["trace_id"] == "task-001"
