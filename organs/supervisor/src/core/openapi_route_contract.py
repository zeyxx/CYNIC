"""Compatibility mirror for kernel router and OpenAPI route parity validation.

Stable deterministic authority is migrating into crates/cynic-mcp. Keep this
module as a Python compatibility mirror until remaining callers are ported.
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable


FindingFactory = Callable[[str, str, str, str], Any]


ROUTE_RE = re.compile(r"\.route\(\s*\"([^\"]+)\"", re.MULTILINE)


def declared_router_paths(router_text: str) -> set[str]:
    return set(ROUTE_RE.findall(router_text))


@dataclass(frozen=True)
class OpenApiRouteContractService:
    workspace_root: Path
    router_path: Path
    openapi_path: Path
    load_json: Callable[[Path], Any]
    relative: Callable[[Path], str]
    finding: FindingFactory

    def validate(self) -> list[Any]:
        findings: list[Any] = []
        if not self.router_path.exists():
            return [
                self.finding(
                    "critical",
                    "openapi_router_source_missing",
                    self.relative(self.router_path),
                    "kernel REST router source must exist before OpenAPI route parity can be validated.",
                )
            ]
        if not self.openapi_path.exists():
            return [
                self.finding(
                    "critical",
                    "openapi_contract_missing",
                    self.relative(self.openapi_path),
                    "docs/api/openapi.json must exist before route parity can be validated.",
                )
            ]

        router_text = self.router_path.read_text(encoding="utf-8")
        try:
            openapi = self.load_json(self.openapi_path)
        except json.JSONDecodeError as exc:
            return [
                self.finding(
                    "critical",
                    "openapi_contract_invalid_json",
                    self.relative(self.openapi_path),
                    f"{exc.msg} at line {exc.lineno} column {exc.colno}",
                )
            ]
        if not isinstance(openapi, dict):
            return [
                self.finding(
                    "critical",
                    "openapi_contract_invalid_shape",
                    self.relative(self.openapi_path),
                    "OpenAPI document must be a JSON object.",
                )
            ]

        router_paths = declared_router_paths(router_text)
        openapi_paths = set(openapi.get("paths", {}).keys())
        for route in sorted(router_paths - openapi_paths):
            findings.append(
                self.finding(
                    "critical",
                    "openapi_route_missing",
                    self.relative(self.openapi_path),
                    f"OpenAPI is missing registered kernel route {route!r}.",
                )
            )
        for route in sorted(openapi_paths - router_paths):
            findings.append(
                self.finding(
                    "critical",
                    "openapi_route_stale",
                    self.relative(self.openapi_path),
                    f"OpenAPI declares path {route!r} not registered by the kernel router.",
                )
            )
        return findings
