"""JSONL read-back adapter for oracle observations."""

from __future__ import annotations

import json
from pathlib import Path

from ..core.observation import OracleObservation


class JsonlObservationReadback:
    def __init__(self, path: Path) -> None:
        self.path = path

    def visible(self, observation: OracleObservation) -> bool:
        if not self.path.exists():
            return False
        expected = observation.to_dict()
        with self.path.open("r", encoding="utf-8") as stream:
            for raw_line in stream:
                try:
                    payload = json.loads(raw_line)
                except json.JSONDecodeError:
                    continue
                if not isinstance(payload, dict):
                    continue
                candidate = payload.get("observation", payload)
                if isinstance(candidate, dict) and all(candidate.get(key) == value for key, value in expected.items()):
                    return True
        return False

