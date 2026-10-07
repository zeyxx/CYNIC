"""Normalize a harness source into the canonical store.

Application layer: imports domain + ports, never adapters directly.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from ..ports import CanonicalStorePort, NormalizerPort


@dataclass
class NormalizeReport:
    harness: str
    sessions_ok: int = 0
    sessions_failed: int = 0
    totals: dict[str, int] = field(default_factory=dict)
    failures: list[tuple[str, str]] = field(default_factory=list)

    def add(self, stats: dict[str, int]) -> None:
        for k, v in stats.items():
            self.totals[k] = self.totals.get(k, 0) + v


def normalize_source(
    normalizer: NormalizerPort,
    store: CanonicalStorePort,
    progress_every: int = 250,
) -> NormalizeReport:
    """Normalize every session the adapter's bound source contains.

    The adapter enumerates its own sessions (``session_refs``); the application
    layer never passes ids it could not have known how to build.
    """
    report = NormalizeReport(harness=normalizer.harness)
    session_refs = normalizer.session_refs()
    if not session_refs:
        print(f"  [normalize] {normalizer.harness}: no sessions found in bound source")
        return report
    print(f"  [normalize] {normalizer.harness}: {len(session_refs)} sessions to normalize")

    for i, session_ref in enumerate(session_refs, 1):
        try:
            session = normalizer.normalize(session_ref)
            session.check()
            store.write(session)
            report.sessions_ok += 1
            report.add(session.stats())
        except Exception as exc:  # noqa: BLE001 — a bad session must not abort the run
            report.sessions_failed += 1
            report.failures.append((session_ref, f"{type(exc).__name__}: {exc}"))
        if progress_every and i % progress_every == 0:
            print(
                f"  ... {i}/{len(session_refs)} "
                f"({report.sessions_ok} ok, {report.sessions_failed} failed)"
            )
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
