"""
CYNIC Anomaly Journal - The Memory of Pain.
Respects ML Platform & SRE Lenses.

Captures every failure, syntax error, and misconfiguration to feed
the MCTS Scientist. Ensures that 'Gold Mines' of learning are not lost.
"""

from __future__ import annotations

import json
import logging
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional

logger = logging.getLogger("cynic.observability.anomaly_journal")


@dataclass
class AnomalyRecord:
    timestamp: float
    error_type: str
    message: str
    traceback: str
    file_context: str
    metadata: Dict[str, Any] = field(default_factory=dict)


class AnomalyJournal:
    """
    Industrial audit of system failures.
    Stores anomalies in a format ready for ML 'retrospective' training.
    """

    def __init__(self, storage_path: str = "audit/anomalies.jsonl"):
        self.path = Path(storage_path)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self._cache: List[AnomalyRecord] = []

    def log_heresy(
        self, error: Exception, context: str, metadata: Optional[Dict] = None
    ):
        """Record a failure for future analysis."""
        record = AnomalyRecord(
            timestamp=time.time(),
            error_type=type(error).__name__,
            message=str(error),
            traceback="Captured via AnomalyJournal",  # Simplified for now
            file_context=context,
            metadata=metadata or {},
        )

        self._cache.append(record)

        # Append to disk (JSONL format for high-speed append)
        try:
            with open(self.path, "a", encoding="utf-8") as f:
                f.write(json.dumps(vars(record)) + "\n")
        except Exception as e:
            logger.error(f"Critical: Failed to record anomaly: {e}")

    def get_learning_patterns(self) -> Dict[str, int]:
        """Summarize recurring error types for the MCTS Scientist."""
        summary: Dict[str, int] = {}
        for rec in self._cache:
            summary[rec.error_type] = summary.get(rec.error_type, 0) + 1
        return summary
