"""Tests for cost_aggregator flush daemon."""
import json
import os
import sys
from pathlib import Path
from unittest.mock import patch, MagicMock

sys.path.insert(0, str(Path(__file__).parent.parent))


def _write_ledger(path: str, events: list) -> None:
    with open(path, "w") as f:
        for e in events:
            f.write(json.dumps(e) + "\n")


def test_aggregate_reads_new_events_only(tmp_path):
    """Aggregator reads only events after cursor."""
    ledger = str(tmp_path / "cost_ledger.jsonl")
    events = [
        {"ts": "2026-06-02T14:00:00Z", "feature_id": "spike_detector",
         "compute_class": "external_api", "provider": "geckoterminal",
         "latency_ms": 200, "tokens_in": 0, "tokens_out": 0},
        {"ts": "2026-06-02T14:05:00Z", "feature_id": "hermes_agent",
         "compute_class": "tailnet", "provider": "qwen36-27b-gpu",
         "latency_ms": 4000, "tokens_in": 0, "tokens_out": 0},
    ]
    _write_ledger(ledger, events)

    from metabolism.cost_aggregator import read_new_events

    # First read: cursor at 0, reads both
    new_events, new_cursor = read_new_events(ledger, 0)
    assert len(new_events) == 2
    assert new_cursor > 0

    # Second read: cursor at end, reads none
    new_events2, _ = read_new_events(ledger, new_cursor)
    assert len(new_events2) == 0


def test_compute_summary_counts_sovereign_calls():
    from metabolism.cost_aggregator import compute_summary
    events = [
        {"feature_id": "spike_detector", "compute_class": "external_api",
         "provider": "geckoterminal", "latency_ms": 200, "tokens_in": 0, "tokens_out": 0},
        {"feature_id": "spike_detector", "compute_class": "external_api",
         "provider": "geckoterminal", "latency_ms": 250, "tokens_in": 0, "tokens_out": 0},
        {"feature_id": "hermes_agent", "compute_class": "tailnet",
         "provider": "qwen36-27b-gpu", "latency_ms": 4000, "tokens_in": 300, "tokens_out": 80},
    ]
    summary = compute_summary(events, kernel_tokens=[])
    assert summary["sovereign_calls"]["geckoterminal"] == 2
    assert summary["tokens_in"] == 300
    assert summary["tokens_out"] == 80
    assert summary["latency_p50_ms"] > 0


def test_compute_summary_includes_kernel_tokens():
    from metabolism.cost_aggregator import compute_summary
    kernel_tokens = [
        {"provider": "qwen36-27b-gpu", "tokens_in": 412, "tokens_out": 89},
        {"provider": "qwen25-7b-core", "tokens_in": 200, "tokens_out": 45},
    ]
    summary = compute_summary([], kernel_tokens=kernel_tokens)
    assert summary["tokens_in"] == 612
    assert summary["tokens_out"] == 134


def test_format_context_string():
    from metabolism.cost_aggregator import format_context
    summary = {
        "tokens_in": 4120,
        "tokens_out": 890,
        "sovereign_calls": {"geckoterminal": 23, "helius": 18},
        "latency_p50_ms": 380,
        "latency_p99_ms": 4800,
        "event_count": 47,
    }
    ctx = format_context(summary)
    assert "tokens_in:4120" in ctx
    assert "geckoterminal:23" in ctx
    assert "p50_latency_ms:380" in ctx

def test_cursor_not_advanced_on_post_failure(tmp_path):
    """Cursor must NOT advance when kernel POST fails -- prevents data loss."""
    from metabolism.cost_aggregator import run_flush
    import metabolism.cost_aggregator as agg

    # Write one event to ledger
    ledger = tmp_path / "cost_ledger.jsonl"
    cursor_file = tmp_path / "cost_ledger_cursor.txt"
    event = json.dumps({
        "feature_id": "spike_detector", "compute_class": "external_api",
        "provider": "geckoterminal", "latency_ms": 200,
        "tokens_in": 0, "tokens_out": 0, "ts": "2026-06-02T14:00:00Z",
    })
    ledger.write_text(event + "\n")

    # Patch aggregator to use tmp paths
    original_ledger = agg.LEDGER_PATH
    original_cursor = agg.CURSOR_PATH
    agg.LEDGER_PATH = ledger
    agg.CURSOR_PATH = cursor_file

    try:
        with patch("metabolism.cost_aggregator.post_to_kernel", return_value=False):
            with patch("metabolism.cost_aggregator.fetch_kernel_tokens", return_value=[]):
                run_flush()
    finally:
        agg.LEDGER_PATH = original_ledger
        agg.CURSOR_PATH = original_cursor

    assert not cursor_file.exists(), "Cursor must NOT be written when POST fails"
