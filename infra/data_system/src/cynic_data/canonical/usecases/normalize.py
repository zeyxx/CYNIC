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


def normalize_many(
    normalizer: NormalizerPort,
    store: CanonicalStorePort,
    raw_session_ids: list[str],
    progress_every: int = 250,
) -> NormalizeReport:
    report = NormalizeReport(harness=normalizer.harness)
    for i, raw_id in enumerate(raw_session_ids, 1):
        try:
            session = normalizer.normalize(raw_id)
            session.check()
            store.write(session)
            report.sessions_ok += 1
            report.add(session.stats())
        except Exception as exc:  # noqa: BLE001 — a bad session must not abort the run
            report.sessions_failed += 1
            report.failures.append((raw_id, f"{type(exc).__name__}: {exc}"))
        if progress_every and i % progress_every == 0:
            print(f"  ... {i}/{len(raw_session_ids)} ({report.sessions_ok} ok, {report.sessions_failed} failed)")
    return report
