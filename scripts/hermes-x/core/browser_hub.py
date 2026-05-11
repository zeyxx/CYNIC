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
