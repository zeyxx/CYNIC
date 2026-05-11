"""
Browser Hub — sole CDP client, tab registry, attribution API.

The Hub is the only process that talks to Chrome via CDP. All consumers
(proxy, navigator, search-executor, behavior-logger) go through the Hub.

K15: Tab registry (producer) → proxy attribution (consumer: tags dataset.jsonl rows)
     Tab registry (producer) → behavior logger (consumer: correlates events to tabs)

Usage:
    python3 browser_hub.py                    # standalone
    systemctl --user start hermes-browser-hub # production
"""

__version__ = "0.1.0"

import asyncio
import json
import logging
import os
import time
from dataclasses import dataclass, field, asdict
from typing import Optional
from collections import deque

logger = logging.getLogger("browser-hub")


@dataclass
class TabEntry:
    tab_id: str
    cdp_target_id: str
    owner: str  # "human" | "agent:<service_name>"
    window_id: str
    created_at: float = field(default_factory=time.time)
    last_url: str = ""
    last_activity: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


class TabRegistry:
    """In-memory tab→owner mapping. SSOT for attribution."""

    def __init__(self):
        self._tabs: dict[str, TabEntry] = {}
        self._target_to_tab: dict[str, str] = {}

    def register(self, tab_id: str, cdp_target_id: str, owner: str, window_id: str, url: str = "") -> TabEntry:
        entry = TabEntry(
            tab_id=tab_id,
            cdp_target_id=cdp_target_id,
            owner=owner,
            window_id=window_id,
            last_url=url,
        )
        self._tabs[tab_id] = entry
        self._target_to_tab[cdp_target_id] = tab_id
        return entry

    def remove(self, tab_id: str) -> None:
        entry = self._tabs.pop(tab_id, None)
        if entry:
            self._target_to_tab.pop(entry.cdp_target_id, None)

    def get_owner(self, tab_id: str) -> str:
        entry = self._tabs.get(tab_id)
        return entry.owner if entry else "human"  # K14: unknown = safe default

    def resolve_target(self, cdp_target_id: str) -> str:
        tab_id = self._target_to_tab.get(cdp_target_id)
        return self.get_owner(tab_id) if tab_id else "human"

    def update_url(self, tab_id: str, url: str) -> None:
        entry = self._tabs.get(tab_id)
        if entry:
            entry.last_url = url
            entry.last_activity = time.time()

    def update_window(self, tab_id: str, window_id: str) -> None:
        """Handle tab drag between windows."""
        entry = self._tabs.get(tab_id)
        if entry:
            entry.window_id = window_id

    def list_all(self) -> list[TabEntry]:
        return list(self._tabs.values())


@dataclass
class _BufferEntry:
    url: str
    timestamp: float
    owner: str


class AttributionBuffer:
    """Ring buffer for CDP Network requests → owner mapping.

    CDP emits Network.requestWillBeSent before the proxy sees the HTTP request.
    We store (url, timestamp, owner) and match when the proxy asks.
    """

    def __init__(self, ttl_seconds: float = 5.0, max_size: int = 1000):
        self._entries: deque[_BufferEntry] = deque(maxlen=max_size)
        self._ttl = ttl_seconds

    def record(self, url: str, timestamp: float, owner: str) -> None:
        self._entries.append(_BufferEntry(url=url, timestamp=timestamp, owner=owner))

    def match(self, url: str, timestamp: float) -> str:
        """Find the owner for a URL within TTL window. Returns 'unknown' if no match."""
        self._evict(timestamp)
        for entry in reversed(self._entries):
            if entry.url == url and abs(timestamp - entry.timestamp) < self._ttl:
                return entry.owner
        return "unknown"

    def _evict(self, now: float) -> None:
        while self._entries and (now - self._entries[0].timestamp) > self._ttl:
            self._entries.popleft()


# ---------------------------------------------------------------------------
# HTTP API
# ---------------------------------------------------------------------------
from aiohttp import web


def create_app(registry: TabRegistry, attribution: AttributionBuffer) -> web.Application:
    """Create the Hub HTTP application. Testable without CDP."""

    async def handle_status(request: web.Request) -> web.Response:
        tabs = [e.to_dict() for e in registry.list_all()]
        return web.json_response({"tabs": tabs, "tab_count": len(tabs), "version": __version__})

    async def handle_get_tabs(request: web.Request) -> web.Response:
        return web.json_response([e.to_dict() for e in registry.list_all()])

    async def handle_get_tab_owner(request: web.Request) -> web.Response:
        tab_id = request.match_info["tab_id"]
        return web.json_response({"tab_id": tab_id, "owner": registry.get_owner(tab_id)})

    async def handle_post_tab(request: web.Request) -> web.Response:
        body = await request.json()
        owner = body.get("owner", "unknown")
        url = body.get("url", "")
        window = body.get("window", "agent")
        tab_id = f"hub-{int(time.time() * 1000)}"
        target_id = f"target-{tab_id}"
        window_id = request.app.get(f"window_{window}", window)
        entry = registry.register(tab_id, target_id, owner, window_id, url=url)
        return web.json_response(entry.to_dict(), status=201)

    async def handle_delete_tab(request: web.Request) -> web.Response:
        tab_id = request.match_info["tab_id"]
        registry.remove(tab_id)
        return web.json_response({"removed": tab_id})

    async def handle_attribution(request: web.Request) -> web.Response:
        url = request.query.get("url", "")
        ts = float(request.query.get("ts", "0"))
        source = attribution.match(url, ts)
        return web.json_response({"source": source, "url": url})

    async def handle_extension_event(request: web.Request) -> web.Response:
        body = await request.json()
        logger.info("Extension event: %s", body.get("type", "unknown"))
        return web.json_response({"ok": True})

    app = web.Application()
    app.router.add_get("/status", handle_status)
    app.router.add_get("/tabs", handle_get_tabs)
    app.router.add_get("/tabs/{tab_id}/owner", handle_get_tab_owner)
    app.router.add_post("/tabs", handle_post_tab)
    app.router.add_delete("/tabs/{tab_id}", handle_delete_tab)
    app.router.add_get("/attribution", handle_attribution)
    app.router.add_post("/events/extension", handle_extension_event)
    return app
