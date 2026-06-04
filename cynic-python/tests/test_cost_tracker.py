"""Tests for metabolism.cost_tracker emitter.

Tier 1 EXPERIMENTAL: unit tests for cost_tracker module.
"""
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


def test_spike_detector_emits_on_fetch(monkeypatch, tmp_path):
    """Verify spike_detector.fetch_trending_pools emits a cost event."""
    import sys
    import importlib
    import urllib.request
    from unittest.mock import MagicMock

    ledger = str(tmp_path / "cost_ledger.jsonl")
    os.environ["CYNIC_COST_LEDGER"] = ledger
    os.environ.setdefault("CYNIC_SESSION_ID", "test-spike")

    # Mock urllib to avoid real network call
    mock_resp = MagicMock()
    mock_resp.read.return_value = json.dumps({"data": []}).encode()
    mock_resp.__enter__ = lambda s: s
    mock_resp.__exit__ = MagicMock(return_value=False)
    monkeypatch.setattr(urllib.request, "urlopen", lambda *a, **kw: mock_resp)

    # Load spike_detector from file directly
    spike_path = str(Path(__file__).parent.parent / "sensors" / "spike_detector.py")
    spec = importlib.util.spec_from_file_location("spike_detector_test", spike_path)
    sd = importlib.util.module_from_spec(spec)
    # Inject the mocked urllib into the module's namespace before exec
    import urllib as _urllib
    sd.urllib = _urllib
    spec.loader.exec_module(sd)

    sd.fetch_trending_pools()

    lines = Path(ledger).read_text().strip().splitlines()
    assert len(lines) == 1, f"Expected 1 event, got {len(lines)}: {lines}"
    event = json.loads(lines[0])
    assert event["feature_id"] == "spike_detector"
    assert event["compute_class"] == "external_api"
    assert event["provider"] == "geckoterminal"
    assert event["operation"] == "fetch_trending_pools"
