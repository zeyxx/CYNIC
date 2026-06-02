"""Tests for metabolism.cost_tracker emitter."""
import json
import os
import tempfile
from pathlib import Path

import pytest

os.environ.setdefault("CYNIC_SESSION_ID", "test-session-001")


def _import_tracker(ledger_path: str):
    import importlib
    import sys
    os.environ["CYNIC_COST_LEDGER"] = ledger_path
    for key in ["cynic-python.metabolism.cost_tracker", "metabolism.cost_tracker"]:
        if key in sys.modules:
            del sys.modules[key]
    sys.path.insert(0, str(Path(__file__).parent.parent))
    from metabolism import cost_tracker
    # Re-register in sys.modules so importlib.reload() can find it
    sys.modules["metabolism.cost_tracker"] = cost_tracker
    importlib.reload(cost_tracker)
    return cost_tracker


def test_emit_writes_valid_jsonl():
    with tempfile.NamedTemporaryFile(suffix=".jsonl", delete=False) as f:
        ledger = f.name
    try:
        ct = _import_tracker(ledger)
        ct.emit(
            feature_id="spike_detector",
            operation="fetch_ohlcv",
            compute_class="external_api",
            provider="geckoterminal",
            latency_ms=230,
        )
        lines = Path(ledger).read_text().strip().splitlines()
        assert len(lines) == 1
        event = json.loads(lines[0])
        assert event["feature_id"] == "spike_detector"
        assert event["compute_class"] == "external_api"
        assert event["provider"] == "geckoterminal"
        assert event["latency_ms"] == 230
        assert event["tokens_in"] == 0
        assert event["tokens_out"] == 0
        assert "ts" in event
        assert event["session_id"] == "test-session-001"
    finally:
        Path(ledger).unlink(missing_ok=True)


def test_emit_with_tokens():
    with tempfile.NamedTemporaryFile(suffix=".jsonl", delete=False) as f:
        ledger = f.name
    try:
        ct = _import_tracker(ledger)
        ct.emit(
            feature_id="kernel_infer",
            operation="infer",
            compute_class="local_gpu",
            provider="qwen36-27b-gpu",
            latency_ms=4200,
            tokens_in=412,
            tokens_out=89,
            trace_id="verdict-abc123",
        )
        event = json.loads(Path(ledger).read_text().strip())
        assert event["tokens_in"] == 412
        assert event["tokens_out"] == 89
        assert event["trace_id"] == "verdict-abc123"
    finally:
        Path(ledger).unlink(missing_ok=True)


def test_emit_never_throws_on_bad_path():
    ct = _import_tracker("/nonexistent/path/cost_ledger.jsonl")
    ct.emit(
        feature_id="test",
        operation="test",
        compute_class="local_gpu",
        provider="test",
        latency_ms=1,
    )


def test_emit_appends_multiple_events():
    with tempfile.NamedTemporaryFile(suffix=".jsonl", delete=False) as f:
        ledger = f.name
    try:
        ct = _import_tracker(ledger)
        for i in range(3):
            ct.emit(
                feature_id="spike_detector",
                operation="fetch",
                compute_class="external_api",
                provider="geckoterminal",
                latency_ms=100 + i,
            )
        lines = Path(ledger).read_text().strip().splitlines()
        assert len(lines) == 3
    finally:
        Path(ledger).unlink(missing_ok=True)
