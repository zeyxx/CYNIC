"""Sovereign search membranes for organ-prospect.

The organ does not scrape public search engines by default. It either consumes
operator-provided URLs or queries a local SearXNG instance controlled by CYNIC.
"""

from __future__ import annotations

import json
import os
from urllib.parse import urlencode
from urllib.request import Request, urlopen

from ..core.entities import SearchHit


class SovereignSearchUnavailable(RuntimeError):
    """Raised when no sovereign search membrane is available."""


class SearxngSearchAdapter:
    def __init__(self, base_url: str | None = None):
        self.base_url = (base_url or os.environ.get("CYNIC_SEARXNG_URL") or "http://127.0.0.1:8080").rstrip("/")

    def search(self, query: str, limit: int) -> list[SearchHit]:
        params = urlencode({"q": query, "format": "json", "language": "fr-FR"})
        request = Request(
            f"{self.base_url}/search?{params}",
            headers={"User-Agent": "CYNIC organ-prospect"},
        )
        with urlopen(request, timeout=12) as response:
            body = response.read().decode("utf-8", errors="replace")
        payload = json.loads(body)
        hits: list[SearchHit] = []
        for result in payload.get("results", []):
            title = str(result.get("title") or "").strip()
            url = str(result.get("url") or "").strip()
            snippet = str(result.get("content") or "").strip()
            if title and url:
                hits.append(SearchHit(title=title, url=url, snippet=snippet))
            if len(hits) >= limit:
                break
        return hits


class ManualOnlySearchAdapter:
    def search(self, query: str, limit: int) -> list[SearchHit]:
        raise SovereignSearchUnavailable(
            "No local search provider selected. Provide --input-url/--input-file, "
            "or run a CYNIC-controlled SearXNG instance and pass --search-provider searxng."
        )
