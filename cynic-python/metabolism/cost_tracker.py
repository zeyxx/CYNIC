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

_created_dirs: set[Path] = set()

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

    ledger = Path(
        os.environ.get("CYNIC_COST_LEDGER")
        or str(Path.home() / ".cynic" / "metabolism" / "cost_ledger.jsonl")
    )
    # flock is advisory; correct for local filesystems (not NFS/overlayfs)
    if ledger.parent not in _created_dirs:
        ledger.parent.mkdir(parents=True, exist_ok=True)
        _created_dirs.add(ledger.parent)

    latency_ms = max(0, latency_ms)
    tokens_in = max(0, tokens_in)
    tokens_out = max(0, tokens_out)

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

    with open(ledger, "a") as f:
        fcntl.flock(f, fcntl.LOCK_EX)
        try:
            f.write(json.dumps(event) + "\n")
        finally:
            fcntl.flock(f, fcntl.LOCK_UN)
