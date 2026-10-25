"""Normalize a harness source into the canonical store.

Application layer: imports domain + ports, never adapters directly.
"""

from __future__ import annotations

import json
import time
import uuid
from dataclasses import dataclass, field

from ..domain.session import session_content_hash
from ..ports import CanonicalStorePort, NormalizerPort


def _emit(kind: str, **fields) -> None:
    """One structured JSON line per event. Stdlib-only, machine-readable, so a
    run leaves an auditable trace (progress, drops, timing) instead of nothing.
    Emitted on stdout with a stable ``cynic.canonical.event.v1`` schema."""
    print(json.dumps({"schema": "cynic.canonical.event.v1", "event": kind, **fields},
                     sort_keys=True), flush=True)


@dataclass
class NormalizeReport:
    harness: str
    sessions_ok: int = 0
    sessions_failed: int = 0
    sessions_skipped: int = 0
    totals: dict[str, int] = field(default_factory=dict)
    failures: list[tuple[str, str]] = field(default_factory=list)

    def add(self, stats: dict[str, int]) -> None:
        for k, v in stats.items():
            self.totals[k] = self.totals.get(k, 0) + v


def normalize_source(
    normalizer: NormalizerPort,
    store: CanonicalStorePort,
    progress_every: int = 250,
    manifest_uid: str | None = None,
) -> NormalizeReport:
    """Normalize every session the adapter's bound source contains.

    The adapter enumerates its own sessions (``session_refs``); the application
    layer never passes ids it could not have known how to build.

    Resume is free: a session whose stored content hash matches the fresh
    normalization is skipped, never rewritten. A crash restarts cheap and
    the run itself lands in the store ledger (``runs`` table).
    """
    report = NormalizeReport(harness=normalizer.harness)
    t0 = time.monotonic()
    started_iso = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    session_refs = normalizer.session_refs()
    _emit("start", harness=normalizer.harness, sessions=len(session_refs))
    if not session_refs:
        _emit("empty", harness=normalizer.harness,
              reason="no sessions found in bound source")
        return report

    for i, session_ref in enumerate(session_refs, 1):
        try:
            session = normalizer.normalize(session_ref)
            session.check()
            if store.session_content_hash(session.session_uid) == session_content_hash(session):
                report.sessions_skipped += 1
                continue
            store.write(session, manifest_uid)
            report.sessions_ok += 1
            report.add(session.stats())
        except Exception as exc:  # noqa: BLE001 — a bad session must not abort the run
            report.sessions_failed += 1
            report.failures.append((session_ref, f"{type(exc).__name__}: {exc}"))
            _emit("session_failed", session_ref=session_ref,
                  error=f"{type(exc).__name__}: {exc}")
        if progress_every and i % progress_every == 0:
            _emit("progress", done=i, total=len(session_refs),
                  ok=report.sessions_ok, failed=report.sessions_failed,
                  skipped=report.sessions_skipped,
                  elapsed_s=round(time.monotonic() - t0, 1))
    _emit("done", harness=normalizer.harness, ok=report.sessions_ok,
          failed=report.sessions_failed, skipped=report.sessions_skipped,
          elapsed_s=round(time.monotonic() - t0, 1),
          totals=report.totals)
    store.record_run({
        "run_uid": f"run:{started_iso}:{normalizer.harness}:{uuid.uuid4().hex[:8]}",
        "harness": normalizer.harness,
        "manifest_uid": manifest_uid,
        "started_iso": started_iso,
        "duration_s": round(time.monotonic() - t0, 1),
        "sessions_ok": report.sessions_ok,
        "sessions_failed": report.sessions_failed,
        "sessions_skipped": report.sessions_skipped,
    })
    return report


def normalize_session(
    normalizer: NormalizerPort,
    store: CanonicalStorePort,
    session_ref: str,
) -> NormalizeReport:
    """Normalize a single session. Used by tests and one-off inspections."""
    report = NormalizeReport(harness=normalizer.harness)
    try:
        session = normalizer.normalize(session_ref)
        session.check()
        store.write(session)
        report.sessions_ok = 1
        report.add(session.stats())
    except Exception as exc:  # noqa: BLE001
        report.sessions_failed = 1
        report.failures.append((session_ref, f"{type(exc).__name__}: {exc}"))
    return report
