"""ChangeAnalyzer — Semantic analysis of code changes.

Subscribes to SOURCE_CHANGED events and enriches them with impact analysis:
  - subsystem classification (kernel, api, cognition, senses, learning, tests)
  - impact_level assessment (LOW, MEDIUM, HIGH, CRITICAL)
  - risk_estimate φ-bounded [0, 1]
  - suggested_action (MONITOR, REVIEW, ALERT)

Maintains in-memory ring buffer of F(8)=21 recent analyses (no DB).
Emits CHANGE_ANALYZED event for each analysis.
"""

from __future__ import annotations

import asyncio
import logging
import time
from collections import deque
from typing import Any

from cynic.core.event_bus import Event, CoreEvent, get_core_bus
from cynic.core.events_schema import ChangeAnalyzedPayload
from cynic.core.topology.payloads import SourceChangedPayload
from cynic.core.phi import fibonacci

logger = logging.getLogger("cynic.core.topology.change_analyzer")


class ChangeAnalyzer:
    """
    Semantic analysis of code changes.

    Subscribes to SOURCE_CHANGED. Enriches each change with:
    - subsystem: "kernel", "api", "cognition", "senses", "learning", "tests"
    - impact_level: "LOW", "MEDIUM", "HIGH", "CRITICAL"
    - risk_estimate: 0.0-1.0 (φ-bounded for display)
    - suggested_action: "MONITOR", "REVIEW", "ALERT"

    Keeps in-memory ring buffer of F(8)=21 analyses (no DB).
    Emits CHANGE_ANALYZED event.
    """

    # File path prefix → (subsystem, impact_level, risk)
    _CLASSIFICATION: dict[str, tuple[str, str, float]] = {
        "cynic/core/": ("kernel", "CRITICAL", 0.9),
        "cynic/cognition/cortex/": ("cognition", "HIGH", 0.7),
        "cynic/cognition/neurons/": ("cognition", "MEDIUM", 0.5),
        "cynic/learning/": ("learning", "HIGH", 0.7),
        "cynic/api/": ("api", "HIGH", 0.6),
        "cynic/api/routers/": ("api", "HIGH", 0.6),
        "cynic/senses/": ("senses", "MEDIUM", 0.4),
        "cynic/metabolism/": ("metabolism", "MEDIUM", 0.4),
        "cynic/nervous/": ("nervous", "HIGH", 0.65),
        "cynic/cognition/": ("cognition", "MEDIUM", 0.5),  # fallback for other cognition dirs
        "cynic/cli/": ("interface", "LOW", 0.2),
        "cynic/tui/": ("interface", "LOW", 0.2),
        "tests/": ("tests", "LOW", 0.1),
    }
    _BUFFER_CAP = fibonacci(8)  # 21 analyses in memory

    def __init__(self) -> None:
        self._buffer: deque[dict[str, Any]] = deque(maxlen=self._BUFFER_CAP)
        self._analysis_count = 0

    async def on_source_changed(self, event: Event) -> None:
        """
        Analyze source change with impact classification.

        Fired when SOURCE_CHANGED event detected.
        Emits CHANGE_ANALYZED event with semantic analysis.
        """
        try:
            payload = event.as_typed(SourceChangedPayload)
        except EventBusError as e:
            logger.warning("Invalid SOURCE_CHANGED payload: %s", e)
            return

        # Analyze each file and collect results
        subsystems_set: set[str] = set()
        impact_max = "LOW"
        risk_max = 0.0
        total_lines = 0

        for filepath in payload.files:
            subsys, impact, risk = self._classify_file(filepath)
            subsystems_set.add(subsys)
            total_lines += 1  # rough: 1 line per file (not actual lines)

            # Take the highest risk/impact
            if self._impact_rank(impact) > self._impact_rank(impact_max):
                impact_max = impact
            if risk > risk_max:
                risk_max = risk

        subsystems_list = sorted(subsystems_set)

        # Determine suggested action based on impact
        if impact_max == "CRITICAL":
            suggested = "ALERT"
        elif impact_max == "HIGH":
            suggested = "REVIEW"
        else:
            suggested = "MONITOR"

        # Create analysis record
        analysis = {
            "timestamp": time.time(),
            "files": payload.files,
            "subsystems": subsystems_list,
            "impact_level": impact_max,
            "risk_estimate": min(risk_max, 1.0),  # φ-bound for display
            "suggested_action": suggested,
            "file_count": len(payload.files),
            "total_lines": total_lines,
        }

        # Store in ring buffer
        self._buffer.append(analysis)
        self._analysis_count += 1

        # Emit CHANGE_ANALYZED event
        await get_core_bus().emit(Event.typed(
            CoreEvent.CHANGE_ANALYZED,
            ChangeAnalyzedPayload(
                files=payload.files,
                subsystems=subsystems_list,
                impact_level=impact_max,
                risk_estimate=min(risk_max, 1.0),
                suggested_action=suggested,
                timestamp=time.time(),
                file_count=len(payload.files),
                total_lines=total_lines,
            ),
            source="ChangeAnalyzer",
        ))

        logger.info(
            "CHANGE ANALYZED: %s files → %s subsystem(s), impact=%s, risk=%.2f, action=%s",
            len(payload.files),
            ", ".join(subsystems_list),
            impact_max,
            risk_max,
            suggested,
        )

    def _classify_file(self, filepath: str) -> tuple[str, str, float]:
        """
        Classify a file by path prefix.

        Returns (subsystem, impact_level, risk_estimate).
        """
        for prefix, (subsys, impact, risk) in self._CLASSIFICATION.items():
            if filepath.startswith(prefix):
                return (subsys, impact, risk)

        # Default classification
        return ("unknown", "MEDIUM", 0.3)

    def _impact_rank(self, impact: str) -> int:
        """Rank impact levels for comparison."""
        ranks = {"LOW": 0, "MEDIUM": 1, "HIGH": 2, "CRITICAL": 3}
        return ranks.get(impact, 0)

    def recent_analyses(self, limit: int = 10) -> list[dict[str, Any]]:
        """Get recent analyses from in-memory buffer (no file I/O)."""
        return list(self._buffer)[-limit:]

    def stats(self) -> dict[str, Any]:
        """Get analyzer statistics."""
        return {
            "total_analyzed": self._analysis_count,
            "buffer_size": len(self._buffer),
            "buffer_cap": self._BUFFER_CAP,
        }
