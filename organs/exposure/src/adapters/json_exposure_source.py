from __future__ import annotations

import json
from pathlib import Path

from core.entities import AgentExposureGraph
from core.ports import ExposureSourcePort


class JsonExposureSource(ExposureSourcePort):
    def __init__(self, path: Path):
        self.path = path

    def load(self) -> AgentExposureGraph:
        with open(self.path, encoding="utf-8") as f:
            return AgentExposureGraph.from_dict(json.load(f))
