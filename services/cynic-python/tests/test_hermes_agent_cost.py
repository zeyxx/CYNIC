"""Test that hermes_agent_task_executor emits cost on subprocess call.

Tier 1 EXPERIMENTAL: integration test for agent cost emission.
"""
import json
import os
import sys
import tempfile
from pathlib import Path
from unittest.mock import patch, MagicMock

sys.path.insert(0, str(Path(__file__).parent.parent.parent / "scripts" / "hermes-x"))
sys.path.insert(0, str(Path(__file__).parent.parent))

os.environ.setdefault("CYNIC_REST_ADDR", "127.0.0.1:3030")
os.environ.setdefault("CYNIC_API_KEY", "test")


def test_execute_task_emits_cost(tmp_path):
    ledger = str(tmp_path / "cost_ledger.jsonl")
    os.environ["CYNIC_COST_LEDGER"] = ledger
    os.environ["CYNIC_SESSION_ID"] = "test-session"

    def mock_subprocess_run(cmd, *args, **kwargs):
        mock_result = MagicMock()
        mock_result.returncode = 0
        if "create" in cmd:
            mock_result.stdout = json.dumps({"id": "t_mock123"})
        elif "decompose" in cmd:
            mock_result.stdout = ""
        elif "show" in cmd:
            mock_result.stdout = json.dumps({"task": {"status": "done"}})
        elif "context" in cmd:
            mock_result.stdout = "ok"
        else:
            mock_result.stdout = ""
        return mock_result

    with patch("subprocess.run", side_effect=mock_subprocess_run):
        from organs.core.adapters.hermes_kanban_executor import HermesKanbanAdapter
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
