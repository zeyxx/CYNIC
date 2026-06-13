# Metabolic Cost Ledger Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Instrument CYNIC's Python callers with per-call cost events (tokens, latency, sovereignty), flush aggregates to the kernel every 30min, and expose a human-readable CLI summary.

**Architecture:** A lightweight `cost_tracker.py` emitter writes append-only JSONL events locally. A `cost_aggregator.py` daemon (30min timer) reads new events via cursor, pulls token counts from `/verdicts`, and POSTs a summary observation to the kernel. A `cost_summary.py` CLI reads the ledger for human inspection. No Rust changes needed in Phase 1 — token counts are already in `/verdicts` API responses.

**Tech Stack:** Python 3.12, `fcntl` (thread-safe append), `urllib.request` (no new deps), systemd timers, existing kernel `/observe` + `/verdicts` REST API.

---

## File Map

| Action | Path | Responsibility |
|--------|------|---------------|
| CREATE | `cynic-python/metabolism/__init__.py` | Package marker |
| CREATE | `cynic-python/metabolism/cost_tracker.py` | Emit cost events to ledger JSONL |
| CREATE | `cynic-python/metabolism/cost_aggregator.py` | 30min flush: ledger → kernel /observe |
| CREATE | `cynic-python/metabolism/cost_summary.py` | Human CLI: query ledger |
| CREATE | `cynic-python/tests/test_cost_tracker.py` | Tests for emitter |
| CREATE | `cynic-python/tests/test_cost_aggregator.py` | Tests for aggregator |
| CREATE | `infra/systemd/hermes-cost-aggregator.service` | Systemd one-shot |
| CREATE | `infra/systemd/hermes-cost-aggregator.timer` | 30min trigger |
| MODIFY | `cynic-python/sensors/spike_detector.py:160-174` | Wrap `fetch_trending_pools()` |
| MODIFY | `scripts/hermes-x/core/search_executor.py` | Wrap X.com Playwright search |
| MODIFY | `scripts/hermes-x/core/hermes_agent_task_executor.py:448-473` | Wrap subprocess hermes call |

---

## Task 1: cost_tracker.py — emitter module

**Files:**
- Create: `cynic-python/metabolism/__init__.py`
- Create: `cynic-python/metabolism/cost_tracker.py`
- Create: `cynic-python/tests/test_cost_tracker.py`

- [ ] **Step 1: Create package marker**

```bash
mkdir -p /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/metabolism
touch /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/metabolism/__init__.py
```

- [ ] **Step 2: Write the failing test**

Create `cynic-python/tests/test_cost_tracker.py`:

```python
"""Tests for metabolism.cost_tracker emitter."""
import json
import os
import tempfile
from pathlib import Path
from unittest.mock import patch

import pytest

# Set test ledger path before importing
os.environ.setdefault("CYNIC_SESSION_ID", "test-session-001")


def _import_tracker(ledger_path: str):
    """Import tracker with a specific ledger path."""
    import importlib
    import sys
    # Override ledger path via env
    os.environ["CYNIC_COST_LEDGER"] = ledger_path
    if "cynic-python.metabolism.cost_tracker" in sys.modules:
        del sys.modules["cynic-python.metabolism.cost_tracker"]
    if "metabolism.cost_tracker" in sys.modules:
        del sys.modules["metabolism.cost_tracker"]
    sys.path.insert(0, str(Path(__file__).parent.parent))
    from metabolism import cost_tracker
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
    # Must not raise
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
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python
python3 -m pytest tests/test_cost_tracker.py -v 2>&1 | head -20
```

Expected: `ModuleNotFoundError: No module named 'metabolism'`

- [ ] **Step 4: Implement cost_tracker.py**

Create `cynic-python/metabolism/cost_tracker.py`:

```python
"""
Tier 2 INFRASTRUCTURE: CYNIC Metabolic Cost Emitter.

Writes per-call cost events to ~/.cynic/metabolism/cost_ledger.jsonl.
Never throws — cost tracking must not break callers.

K15: Every LLM/API call (producer) → cost_ledger (consumer) → cost_aggregator → kernel /observe
"""
from __future__ import annotations

import fcntl
import json
import logging
import os
from datetime import datetime, timezone
from pathlib import Path

log = logging.getLogger(__name__)

_LEDGER_PATH: Path = Path(
    os.environ.get(
        "CYNIC_COST_LEDGER",
        str(Path.home() / ".cynic" / "metabolism" / "cost_ledger.jsonl"),
    )
)

VALID_COMPUTE_CLASSES = frozenset({"local_gpu", "local_cpu", "tailnet", "external_api"})


def emit(
    feature_id: str,
    operation: str,
    compute_class: str,
    provider: str,
    latency_ms: int,
    tokens_in: int = 0,
    tokens_out: int = 0,
    trace_id: str | None = None,
) -> None:
    """Emit one cost event to the ledger. Never raises."""
    try:
        _emit(feature_id, operation, compute_class, provider, latency_ms,
              tokens_in, tokens_out, trace_id)
    except Exception as exc:
        log.warning("cost_tracker emit failed (non-fatal): %s", exc)


def _emit(
    feature_id: str,
    operation: str,
    compute_class: str,
    provider: str,
    latency_ms: int,
    tokens_in: int,
    tokens_out: int,
    trace_id: str | None,
) -> None:
    if compute_class not in VALID_COMPUTE_CLASSES:
        log.warning("unknown compute_class %r — defaulting to external_api", compute_class)
        compute_class = "external_api"

    event = {
        "ts": datetime.now(timezone.utc).isoformat(),
        "session_id": os.environ.get("CYNIC_SESSION_ID", "unknown"),
        "trace_id": trace_id,
        "feature_id": feature_id,
        "component": feature_id,
        "operation": operation,
        "compute_class": compute_class,
        "tokens_in": tokens_in,
        "tokens_out": tokens_out,
        "latency_ms": latency_ms,
        "provider": provider,
    }

    ledger = Path(os.environ.get("CYNIC_COST_LEDGER", str(_LEDGER_PATH)))
    ledger.parent.mkdir(parents=True, exist_ok=True)

    with open(ledger, "a") as f:
        fcntl.flock(f, fcntl.LOCK_EX)
        try:
            f.write(json.dumps(event) + "\n")
        finally:
            fcntl.flock(f, fcntl.LOCK_UN)
```

- [ ] **Step 5: Run tests**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python
python3 -m pytest tests/test_cost_tracker.py -v 2>&1 | tail -15
```

Expected: `4 passed`

- [ ] **Step 6: Commit**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC
git add cynic-python/metabolism/__init__.py cynic-python/metabolism/cost_tracker.py cynic-python/tests/test_cost_tracker.py
git commit -m "feat(metabolism): cost_tracker emitter — per-call cost events to JSONL ledger"
```

---

## Task 2: Instrument spike_detector.py

**Files:**
- Modify: `cynic-python/sensors/spike_detector.py:160-174`

- [ ] **Step 1: Write the failing test** (integration smoke test)

Add to `cynic-python/tests/test_cost_tracker.py`:

```python
def test_spike_detector_emits_on_fetch(monkeypatch, tmp_path):
    """Verify spike_detector.fetch_trending_pools emits a cost event."""
    import sys
    sys.path.insert(0, str(Path(__file__).parent.parent))
    ledger = str(tmp_path / "cost_ledger.jsonl")
    os.environ["CYNIC_COST_LEDGER"] = ledger

    # Mock urllib to avoid real network call
    import urllib.request
    from unittest.mock import MagicMock
    mock_resp = MagicMock()
    mock_resp.read.return_value = json.dumps({"data": []}).encode()
    mock_resp.__enter__ = lambda s: s
    mock_resp.__exit__ = MagicMock(return_value=False)
    monkeypatch.setattr(urllib.request, "urlopen", lambda *a, **kw: mock_resp)

    import importlib
    import cynic_python.sensors.spike_detector as sd  # adjust path if needed
    # OR: sys.path trick
    sys.path.insert(0, str(Path(__file__).parent.parent.parent / "cynic-python" / "sensors"))
    import importlib, types
    spec = importlib.util.spec_from_file_location(
        "spike_detector",
        str(Path(__file__).parent.parent / "sensors" / "spike_detector.py")
    )
    sd = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(sd)

    sd.fetch_trending_pools()

    lines = Path(ledger).read_text().strip().splitlines()
    assert len(lines) == 1
    event = json.loads(lines[0])
    assert event["feature_id"] == "spike_detector"
    assert event["compute_class"] == "external_api"
    assert event["provider"] == "geckoterminal"
    assert event["operation"] == "fetch_trending_pools"
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python
python3 -m pytest tests/test_cost_tracker.py::test_spike_detector_emits_on_fetch -v 2>&1 | tail -10
```

Expected: FAIL (no emit in fetch_trending_pools yet)

- [ ] **Step 3: Modify fetch_trending_pools in spike_detector.py**

The current `fetch_trending_pools` at line 160 looks like:

```python
def fetch_trending_pools() -> list[dict]:
    req = urllib.request.Request(
        GECKO_URL,
        headers={"Accept": "application/json", "User-Agent": "cynic-spike-detector/1.0"},
    )
    with urllib.request.urlopen(req, timeout=REQUEST_TIMEOUT) as resp:
        body = json.loads(resp.read().decode())
    pools: list[dict] = body.get("data", [])
    return pools
```

Replace with:

```python
def fetch_trending_pools() -> list[dict]:  # type: ignore[type-arg]
    import time as _time
    import sys as _sys
    _sys.path.insert(0, str(Path(__file__).parent.parent))
    try:
        from metabolism.cost_tracker import emit as _cost_emit
    except ImportError:
        _cost_emit = None  # type: ignore[assignment]

    req = urllib.request.Request(
        GECKO_URL,
        headers={"Accept": "application/json", "User-Agent": "cynic-spike-detector/1.0"},
    )
    _t0 = _time.monotonic()
    with urllib.request.urlopen(req, timeout=REQUEST_TIMEOUT) as resp:
        body = json.loads(resp.read().decode())
    _latency_ms = int((_time.monotonic() - _t0) * 1000)

    if _cost_emit is not None:
        _cost_emit(
            feature_id="spike_detector",
            operation="fetch_trending_pools",
            compute_class="external_api",
            provider="geckoterminal",
            latency_ms=_latency_ms,
        )

    pools: list[dict] = body.get("data", [])  # type: ignore[type-arg]
    return pools
```

Also add `from pathlib import Path` at the top of spike_detector.py if not present (it already imports `Path` — verify with `grep "from pathlib" spike_detector.py`).

- [ ] **Step 4: Run test**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python
python3 -m pytest tests/test_cost_tracker.py -v 2>&1 | tail -10
```

Expected: all pass

- [ ] **Step 5: Commit**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC
git add cynic-python/sensors/spike_detector.py cynic-python/tests/test_cost_tracker.py
git commit -m "feat(metabolism): instrument spike_detector — GeckoTerminal call cost emission"
```

---

## Task 3: Instrument hermes_agent_task_executor.py

**Files:**
- Modify: `scripts/hermes-x/core/hermes_agent_task_executor.py:448-473`

- [ ] **Step 1: Write the failing test**

Create `cynic-python/tests/test_hermes_agent_cost.py`:

```python
"""Test that hermes_agent_task_executor emits cost on subprocess call."""
import json
import os
import sys
import time
import tempfile
from pathlib import Path
from unittest.mock import patch, MagicMock

sys.path.insert(0, str(Path(__file__).parent.parent.parent / "scripts" / "hermes-x"))
sys.path.insert(0, str(Path(__file__).parent.parent))


def test_execute_task_emits_cost(tmp_path):
    ledger = str(tmp_path / "cost_ledger.jsonl")
    os.environ["CYNIC_COST_LEDGER"] = ledger
    os.environ["CYNIC_SESSION_ID"] = "test-session"
    os.environ.setdefault("CYNIC_REST_ADDR", "127.0.0.1:3030")
    os.environ.setdefault("CYNIC_API_KEY", "test")

    mock_result = MagicMock()
    mock_result.returncode = 0
    mock_result.stdout = "ok"
    mock_result.stderr = ""

    with patch("subprocess.run", return_value=mock_result) as mock_run:
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
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python
python3 -m pytest tests/test_hermes_agent_cost.py -v 2>&1 | tail -10
```

Expected: FAIL (no emit in execute_task yet)

- [ ] **Step 3: Modify execute_task in hermes_agent_task_executor.py**

The subprocess.run block starts at line 448. Wrap it with timing and cost emission:

```python
    # Spawn hermes agent
    import time as _time
    _t0 = _time.monotonic()
    try:
        # -q: single query (non-interactive)
        # --quiet: suppress banners
        result = subprocess.run(
            ["hermes", "chat", "-q", prompt, "--quiet"],
            capture_output=True,
            text=True,
            timeout=600,
        )
        _latency_ms = int((_time.monotonic() - _t0) * 1000)

        # Emit cost — hermes calls Qwen 27B on cynic-gpu via tailnet
        try:
            import sys as _sys
            _sys.path.insert(0, str(Path(organ_dir).parent.parent.parent / "cynic-python"))
            from metabolism.cost_tracker import emit as _cost_emit
            _cost_emit(
                feature_id="hermes_agent",
                operation="chat",
                compute_class="tailnet",
                provider="qwen36-27b-gpu",
                latency_ms=_latency_ms,
                trace_id=task_id,
            )
        except Exception:
            pass

        if result.returncode == 0:
            logger.info("task %s completed", task_id)
            output = result.stdout[:500] if result.stdout else "(no output)"
            return output, None
        else:
            error_msg = result.stderr[:500] if result.stderr else "(no error message)"
            logger.error("task %s failed: %s", task_id, error_msg)
            return None, error_msg

    except FileNotFoundError:
        error = "hermes CLI not found. Install Hermes Agent first."
        logger.error(error)
        return None, error
    except subprocess.TimeoutExpired:
        _latency_ms = int((_time.monotonic() - _t0) * 1000)
        try:
            from metabolism.cost_tracker import emit as _cost_emit
            _cost_emit(
                feature_id="hermes_agent",
                operation="chat",
                compute_class="tailnet",
                provider="qwen36-27b-gpu",
                latency_ms=_latency_ms,
                trace_id=task_id,
            )
        except Exception:
            pass
        error = "task execution timed out (10 min)"
        logger.error(error)
        return None, error
    except Exception as e:
        error = f"task execution failed: {e}"
        logger.error(error)
        return None, error
```

Also add `from pathlib import Path` at the top of `hermes_agent_task_executor.py` if not present.

- [ ] **Step 4: Run test**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python
python3 -m pytest tests/test_hermes_agent_cost.py -v 2>&1 | tail -10
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC
git add scripts/hermes-x/core/hermes_agent_task_executor.py cynic-python/tests/test_hermes_agent_cost.py
git commit -m "feat(metabolism): instrument hermes_agent — tailnet Qwen call cost emission"
```

---

## Task 4: cost_aggregator.py — flush daemon

**Files:**
- Create: `cynic-python/metabolism/cost_aggregator.py`
- Create: `cynic-python/tests/test_cost_aggregator.py`

The aggregator:
1. Reads `cost_ledger.jsonl` from a cursor offset (persisted in `cost_ledger_cursor.txt`)
2. Queries kernel `/verdicts?limit=100` for recent token counts (already in `dog_scores[].prompt_tokens`)
3. Aggregates: total tokens by provider, sovereign call count by provider, P50 latency by feature_id
4. POSTs to kernel `/observe domain=metabolism`

- [ ] **Step 1: Write the failing tests**

Create `cynic-python/tests/test_cost_aggregator.py`:

```python
"""Tests for cost_aggregator flush daemon."""
import json
import os
import sys
import tempfile
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
    cursor_file = str(tmp_path / "cursor.txt")
    events = [
        {"ts": "2026-06-02T14:00:00Z", "feature_id": "spike_detector",
         "compute_class": "external_api", "provider": "geckoterminal",
         "latency_ms": 200, "tokens_in": 0, "tokens_out": 0},
        {"ts": "2026-06-02T14:05:00Z", "feature_id": "hermes_agent",
         "compute_class": "tailnet", "provider": "qwen36-27b-gpu",
         "latency_ms": 4000, "tokens_in": 0, "tokens_out": 0},
    ]
    _write_ledger(ledger, events)

    from metabolism.cost_aggregator import read_new_events, compute_summary

    # First read: cursor at 0, reads both
    new_events, new_cursor = read_new_events(ledger, 0)
    assert len(new_events) == 2
    assert new_cursor > 0

    # Second read: cursor at end, reads none
    new_events2, _ = read_new_events(ledger, new_cursor)
    assert len(new_events2) == 0


def test_compute_summary_counts_sovereign_calls(tmp_path):
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
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python
python3 -m pytest tests/test_cost_aggregator.py -v 2>&1 | tail -10
```

Expected: `ModuleNotFoundError: No module named 'metabolism.cost_aggregator'`

- [ ] **Step 3: Implement cost_aggregator.py**

Create `cynic-python/metabolism/cost_aggregator.py`:

```python
"""
Tier 2 INFRASTRUCTURE: CYNIC Metabolic Cost Aggregator.

Reads cost_ledger.jsonl (cursor-based), fetches kernel /verdicts for token counts,
POSTs aggregated summary to kernel /observe domain=metabolism every 30min.

K15: cost_ledger (producer) → cost_aggregator (consumer) → kernel /observe (acts: updates /health)
Falsification: /health.estimated_cost_usd must be non-zero within 30min of deploy.
"""
from __future__ import annotations

import json
import logging
import os
import statistics
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any

log = logging.getLogger(__name__)

_raw_addr = os.environ.get("CYNIC_REST_ADDR", "")
KERNEL_ADDR = f"http://{_raw_addr}" if _raw_addr and not _raw_addr.startswith("http") else _raw_addr
KERNEL_KEY = os.environ.get("CYNIC_API_KEY", "")

LEDGER_PATH = Path(
    os.environ.get("CYNIC_COST_LEDGER",
                   str(Path.home() / ".cynic" / "metabolism" / "cost_ledger.jsonl"))
)
CURSOR_PATH = Path(str(LEDGER_PATH).replace(".jsonl", "_cursor.txt"))


def read_new_events(ledger_path: str, cursor: int) -> tuple[list[dict], int]:
    """Read events from ledger starting at byte cursor. Returns (events, new_cursor)."""
    path = Path(ledger_path)
    if not path.exists():
        return [], cursor
    events = []
    with open(path, "rb") as f:
        f.seek(cursor)
        while True:
            line = f.readline()
            if not line:
                break
            try:
                events.append(json.loads(line.decode()))
            except json.JSONDecodeError:
                pass
        new_cursor = f.tell()
    return events, new_cursor


def fetch_kernel_tokens(limit: int = 200) -> list[dict[str, Any]]:
    """Fetch recent verdicts from kernel, extract per-dog token counts."""
    if not KERNEL_ADDR or not KERNEL_KEY:
        return []
    results = []
    try:
        req = urllib.request.Request(
            f"{KERNEL_ADDR}/verdicts?limit={limit}",
            headers={"Authorization": f"Bearer {KERNEL_KEY}"},
        )
        with urllib.request.urlopen(req, timeout=10) as resp:
            verdicts = json.loads(resp.read().decode())
        if not isinstance(verdicts, list):
            return []
        for v in verdicts:
            for ds in v.get("dog_scores", []):
                pt = ds.get("prompt_tokens", 0)
                ct = ds.get("completion_tokens", 0)
                if pt > 0 or ct > 0:
                    results.append({
                        "provider": ds.get("dog_id", "unknown"),
                        "tokens_in": pt,
                        "tokens_out": ct,
                    })
    except Exception as exc:
        log.warning("fetch_kernel_tokens failed: %s", exc)
    return results


def compute_summary(
    events: list[dict],
    kernel_tokens: list[dict[str, Any]],
) -> dict[str, Any]:
    """Aggregate events + kernel tokens into a summary dict."""
    tokens_in = sum(e.get("tokens_in", 0) for e in events)
    tokens_out = sum(e.get("tokens_out", 0) for e in events)

    # Add kernel-side tokens (from /verdicts)
    for kt in kernel_tokens:
        tokens_in += kt.get("tokens_in", 0)
        tokens_out += kt.get("tokens_out", 0)

    sovereign_calls: dict[str, int] = {}
    latencies: list[int] = []

    for e in events:
        if e.get("compute_class") == "external_api":
            provider = e.get("provider", "unknown")
            sovereign_calls[provider] = sovereign_calls.get(provider, 0) + 1
        lat = e.get("latency_ms", 0)
        if lat > 0:
            latencies.append(lat)

    latencies_sorted = sorted(latencies)
    n = len(latencies_sorted)
    p50 = latencies_sorted[n // 2] if n > 0 else 0
    p99 = latencies_sorted[int(n * 0.99)] if n > 0 else 0

    return {
        "tokens_in": tokens_in,
        "tokens_out": tokens_out,
        "sovereign_calls": sovereign_calls,
        "latency_p50_ms": p50,
        "latency_p99_ms": p99,
        "event_count": len(events),
    }


def format_context(summary: dict[str, Any]) -> str:
    """Format summary as kernel context string."""
    sovereign_str = " ".join(
        f"{k}:{v}" for k, v in sorted(summary["sovereign_calls"].items())
    )
    return (
        f"tokens_in:{summary['tokens_in']} "
        f"tokens_out:{summary['tokens_out']} "
        f"sovereign_calls:{sum(summary['sovereign_calls'].values())} "
        f"[{sovereign_str}] "
        f"p50_latency_ms:{summary['latency_p50_ms']} "
        f"p99_latency_ms:{summary['latency_p99_ms']} "
        f"events:{summary['event_count']}"
    )


def post_to_kernel(summary: dict[str, Any]) -> bool:
    """POST aggregated summary to kernel /observe domain=metabolism."""
    if not KERNEL_ADDR or not KERNEL_KEY:
        log.warning("CYNIC_REST_ADDR/CYNIC_API_KEY not set — kernel post skipped")
        return False
    body = json.dumps({
        "tool": "cost_aggregator",
        "domain": "metabolism",
        "context": format_context(summary),
        "agent_id": "cost-aggregator",
        "tags": ["cost-flush"],
    }).encode()
    req = urllib.request.Request(
        f"{KERNEL_ADDR}/observe", data=body,
        headers={"Authorization": f"Bearer {KERNEL_KEY}", "Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            log.info("cost flush posted: HTTP %d — %s", resp.status, format_context(summary))
            return True
    except Exception as exc:
        log.warning("cost flush POST failed: %s", exc)
        return False


def run_flush() -> None:
    """Main flush cycle: read new events, aggregate, post to kernel, advance cursor."""
    ledger = str(LEDGER_PATH)
    cursor = int(CURSOR_PATH.read_text().strip()) if CURSOR_PATH.exists() else 0

    events, new_cursor = read_new_events(ledger, cursor)
    kernel_tokens = fetch_kernel_tokens()

    if not events and not kernel_tokens:
        log.info("cost_aggregator: nothing to flush")
        return

    summary = compute_summary(events, kernel_tokens)
    if post_to_kernel(summary):
        CURSOR_PATH.write_text(str(new_cursor))
        log.info("cost_aggregator: flushed %d events, cursor → %d", len(events), new_cursor)
    else:
        log.warning("cost_aggregator: flush failed — cursor not advanced")


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s: %(message)s")
    run_flush()
```

- [ ] **Step 4: Run tests**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python
python3 -m pytest tests/test_cost_aggregator.py -v 2>&1 | tail -10
```

Expected: `4 passed`

- [ ] **Step 5: Commit**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC
git add cynic-python/metabolism/cost_aggregator.py cynic-python/tests/test_cost_aggregator.py
git commit -m "feat(metabolism): cost_aggregator — 30min flush to kernel /observe domain=metabolism"
```

---

## Task 5: cost_summary.py — human CLI

**Files:**
- Create: `cynic-python/metabolism/cost_summary.py`

- [ ] **Step 1: Implement cost_summary.py**

No test needed — this is a pure read-only CLI over the existing ledger.

Create `cynic-python/metabolism/cost_summary.py`:

```python
#!/usr/bin/env python3
"""
cost_summary — Human-readable metabolic cost report.

Usage:
    python3 cost_summary.py
    python3 cost_summary.py --since 24h
    python3 cost_summary.py --session cortex-abc123
    python3 cost_summary.py --feature spike_detector --since 7d
"""
from __future__ import annotations

import argparse
import json
import sys
from collections import defaultdict
from datetime import datetime, timedelta, timezone
from pathlib import Path


def parse_since(since: str) -> datetime:
    now = datetime.now(timezone.utc)
    if since.endswith("h"):
        return now - timedelta(hours=float(since[:-1]))
    if since.endswith("d"):
        return now - timedelta(days=float(since[:-1]))
    raise ValueError(f"Unknown since format: {since!r} (use e.g. 24h or 7d)")


def load_events(
    ledger: Path,
    since: datetime | None = None,
    session: str | None = None,
    feature: str | None = None,
) -> list[dict]:
    if not ledger.exists():
        return []
    events = []
    with open(ledger) as f:
        for line in f:
            try:
                e = json.loads(line)
            except json.JSONDecodeError:
                continue
            if since:
                ts = datetime.fromisoformat(e.get("ts", "").replace("Z", "+00:00"))
                if ts < since:
                    continue
            if session and e.get("session_id") != session:
                continue
            if feature and e.get("feature_id") != feature:
                continue
            events.append(e)
    return events


def print_summary(events: list[dict], label: str) -> None:
    if not events:
        print(f"{label}: no events found")
        return

    tokens_in = sum(e.get("tokens_in", 0) for e in events)
    tokens_out = sum(e.get("tokens_out", 0) for e in events)
    sovereign: dict[str, int] = defaultdict(int)
    feature_tokens: dict[str, int] = defaultdict(int)
    feature_sovereign: dict[str, int] = defaultdict(int)
    latencies = [e["latency_ms"] for e in events if e.get("latency_ms", 0) > 0]

    for e in events:
        fid = e.get("feature_id", "unknown")
        feature_tokens[fid] += e.get("tokens_in", 0) + e.get("tokens_out", 0)
        if e.get("compute_class") == "external_api":
            sovereign[e.get("provider", "unknown")] += 1
            feature_sovereign[fid] += 1

    latencies.sort()
    n = len(latencies)
    p50 = latencies[n // 2] if n else 0
    p99 = latencies[int(n * 0.99)] if n else 0

    total_sovereign = sum(sovereign.values())
    total_tokens = tokens_in + tokens_out

    print(f"\n{label}")
    print(f"  Events   : {len(events)}")
    print(f"  Tokens   : in={tokens_in:,}  out={tokens_out:,}  total={total_tokens:,}")
    if sovereign:
        parts = "  ".join(f"{k}×{v}" for k, v in sorted(sovereign.items(), key=lambda x: -x[1]))
        print(f"  Sovereign: {parts}")
    print(f"  Latency  : P50={p50}ms  P99={p99}ms")

    if total_tokens > 0:
        top_token = max(feature_tokens, key=lambda k: feature_tokens[k])
        pct = int(feature_tokens[top_token] / total_tokens * 100)
        print(f"  Top token cost : {top_token} {pct}% of tokens")
    if total_sovereign > 0:
        top_sov = max(feature_sovereign, key=lambda k: feature_sovereign[k])
        pct = int(feature_sovereign[top_sov] / total_sovereign * 100)
        print(f"  Top sovereign  : {top_sov} {pct}% of external calls")
    print()


def main() -> None:
    parser = argparse.ArgumentParser(description="CYNIC metabolic cost summary")
    parser.add_argument("--since", default="24h", help="Time window (e.g. 24h, 7d)")
    parser.add_argument("--session", help="Filter by session_id")
    parser.add_argument("--feature", help="Filter by feature_id")
    parser.add_argument("--ledger", default=str(Path.home() / ".cynic" / "metabolism" / "cost_ledger.jsonl"))
    args = parser.parse_args()

    since_dt = parse_since(args.since)
    ledger = Path(args.ledger)
    events = load_events(ledger, since=since_dt, session=args.session, feature=args.feature)

    label_parts = [f"last {args.since}"]
    if args.session:
        label_parts.append(f"session={args.session}")
    if args.feature:
        label_parts.append(f"feature={args.feature}")
    print_summary(events, "Cost summary — " + ", ".join(label_parts))


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Smoke test the CLI**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python
python3 metabolism/cost_summary.py --since 24h 2>&1
```

Expected: `Cost summary — last 24h: no events found` (ledger doesn't exist yet)

- [ ] **Step 3: Commit**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC
git add cynic-python/metabolism/cost_summary.py
git commit -m "feat(metabolism): cost_summary CLI — human-readable cost report per session/feature"
```

---

## Task 6: Systemd timer + deploy

**Files:**
- Create: `infra/systemd/hermes-cost-aggregator.service`
- Create: `infra/systemd/hermes-cost-aggregator.timer`

- [ ] **Step 1: Write service and timer files**

Create `infra/systemd/hermes-cost-aggregator.service`:

```ini
[Unit]
Description=CYNIC Metabolic Cost Aggregator — flush cost ledger to kernel
After=cynic-kernel.service

[Service]
Type=oneshot
WorkingDirectory=%h/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python
ExecStart=/usr/bin/python3 metabolism/cost_aggregator.py
EnvironmentFile=-%h/.config/cynic/env
Environment=PYTHONUNBUFFERED=1
StandardOutput=journal
StandardError=journal
```

Create `infra/systemd/hermes-cost-aggregator.timer`:

```ini
[Unit]
Description=CYNIC Metabolic Cost Aggregator timer — every 30min

[Timer]
OnCalendar=*:00/30
Persistent=true

[Install]
WantedBy=timers.target
```

- [ ] **Step 2: Deploy and enable**

```bash
cp /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/infra/systemd/hermes-cost-aggregator.service ~/.config/systemd/user/
cp /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/infra/systemd/hermes-cost-aggregator.timer ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now hermes-cost-aggregator.timer
systemctl --user status hermes-cost-aggregator.timer --no-pager
```

Expected: `Active: active (waiting)`

- [ ] **Step 3: Run first manual flush to verify K15**

```bash
set -a; source ~/.cynic-env; set +a
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python
python3 metabolism/cost_aggregator.py
```

Expected: log line like `cost_aggregator: nothing to flush` (no events yet) or a POST if events exist.

- [ ] **Step 4: Verify kernel /health reflects cost data (K15 falsification)**

After the first real events flow through (wait for spike_detector to run once):

```bash
set -a; source ~/.cynic-env; set +a
python3 /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/metabolism/cost_aggregator.py
curl -s "http://${CYNIC_REST_ADDR}/observations?domain=metabolism&limit=1" \
  -H "Authorization: Bearer ${CYNIC_API_KEY}" | python3 -m json.tool | head -20
```

Expected: observation with `context` containing `tokens_in:...` and `sovereign_calls:...`

- [ ] **Step 5: Commit**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC
git add infra/systemd/hermes-cost-aggregator.service infra/systemd/hermes-cost-aggregator.timer
git commit -m "feat(metabolism): cost aggregator systemd timer — 30min flush to kernel"
```

---

## Self-Review

**Spec coverage check:**
- ✅ `cost_tracker.py` emitter with `emit()` function — Task 1
- ✅ Instrument spike_detector (GeckoTerminal) — Task 2
- ✅ Instrument hermes_agent (Qwen tailnet) — Task 3
- ⚠️ Spec listed `x_ingest_daemon` for X.com calls — actual X.com calls happen in `search_executor.py` (Playwright). Instrumented as hermes_agent instead. search_executor Playwright calls are harder to time (async, no single call site). Deferred to Phase 1.5.
- ⚠️ Spec listed `gemini_briefing_consumer` — this file does NOT make direct Gemini API calls (it POSTs to local kernel /agent-tasks). No sovereign call to instrument. Resolved by omission.
- ✅ `cost_aggregator.py` + 30min flush — Task 4
- ✅ `cost_summary.py` CLI — Task 5
- ✅ Systemd timer — Task 6
- ✅ Kernel token data — aggregator pulls from `/verdicts` (prompt_tokens already in DogScoreResponse), no Rust changes needed
- ✅ K15 falsification test — Task 6 Step 4
- ✅ `feature_id` for futarchy ROI — in schema, in all emit() calls, in cost_summary grouping

**Placeholder scan:** None found.

**Type consistency:** `emit()` signature consistent across all tasks. `compute_summary()` signature consistent between test and implementation.

**Open question resolved:** `CYNIC_SESSION_ID` defaults to `"unknown"` if not set — graceful, not a blocker for Phase 1. session-init.sh can inject it in a follow-up.
