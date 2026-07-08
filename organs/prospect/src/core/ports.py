"""Ports for organ-prospect."""

from __future__ import annotations

from typing import Protocol

from .entities import SearchHit, WebsiteAudit


class WebSearchPort(Protocol):
    def search(self, query: str, limit: int) -> list[SearchHit]:
        """Return public web search hits for a local prospecting query."""


class WebsiteAuditPort(Protocol):
    def audit(self, url: str | None, city: str) -> WebsiteAudit:
        """Audit a candidate business website or directory URL."""

