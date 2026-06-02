"""Test that hermes_agent_task_executor emits cost on subprocess call."""
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

    mock_result = MagicMock()
    mock_result.returncode = 0
    mock_result.stdout = "ok"
    mock_result.stderr = ""

    with patch("subprocess.run", return_value=mock_result):
        with patch("core.hermes_agent_task_executor.consult_soma_gate",
                   return_value={"decision": "allocate", "data": {}}):
            from core import hermes_agent_task_executor as hae
            task = {
                "id": "task-001",
                "objective": "test task",
                "actions": ["do something"],
                "domain": "D3",
                "_source": "kernel",
            }
            result, err = hae.execute_task(task, str(tmp_path))

    assert result == "ok"
    assert err is None
    lines = Path(ledger).read_text().strip().splitlines()
    assert len(lines) == 1
    event = json.loads(lines[0])
    assert event["feature_id"] == "hermes_agent"
    assert event["compute_class"] == "tailnet"
    assert event["provider"] == "qwen36-27b-gpu"
    assert event["latency_ms"] >= 0
    assert event["trace_id"] == "task-001"
