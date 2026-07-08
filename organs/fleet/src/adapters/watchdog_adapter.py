from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from ..core.entities import ObservedResource


class WatchdogObservationAdapter:
    def __init__(self, path: Path | None) -> None:
        self.path = path

    def resources(self) -> list[ObservedResource]:
        if self.path is None or not self.path.exists():
            return []
        payload = json.loads(self.path.read_text(encoding="utf-8"))
        items = payload if isinstance(payload, list) else [payload]
        return [
            resource
            for item in items
            if isinstance(item, dict)
            for resource in [self._resource(item)]
            if resource is not None
        ]

    def _resource(self, payload: dict[str, Any]) -> ObservedResource | None:
        value = payload.get("value") if payload.get("tool") == "cynic-node-watchdog" else payload
        if not isinstance(value, dict):
            return None
        if value.get("schema") != "cynic.node_watchdog_observation.v1":
            return None
        node = value.get("node")
        if not isinstance(node, dict):
            return None
        name = node.get("name")
        if not isinstance(name, str) or not name:
            return None
        status = value.get("status")
        return ObservedResource(
            provider="cynic-node-watchdog",
            name=name,
            status="running" if status == "ok" else "degraded",
            kind="node_watchdog",
            constellation=node.get("constellation") if isinstance(node.get("constellation"), str) else None,
        )
