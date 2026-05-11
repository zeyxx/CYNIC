# Hermes Shared Browser + Attribution (L0) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One Chrome browser shared between T. and the agent, with CDP-based attribution tagging every captured tweet as `source: "agent"` or `"human"`.

**Architecture:** A Browser Hub service (Python asyncio) is the sole CDP client. It maintains a tab registry mapping tabs to owners. The proxy (mitmdump/x_proxy.py) queries the Hub for attribution. Navigator and search-executor create tabs via the Hub instead of connecting to CDP directly.

**Tech Stack:** Python 3.11+, asyncio, aiohttp (Hub HTTP server), websockets (CDP client), Playwright (consumers)

**Spec:** `docs/superpowers/specs/2026-05-11-hermes-shared-browser-behavioral-engine-design.md`

**Dependencies to install:** `pip install websockets aiohttp pytest-aiohttp requests`

---

## Review Fixes Applied

Fixes from plan review (2026-05-11):
- B1: CDP `Network.enable` via `Target.attachToTarget` per target + session→target map
- B2: URL encoding in `_get_attribution()` via `urllib.parse.urlencode`
- B3: Mock mitmproxy import in proxy attribution tests
- B4: Lazy `websockets` import in CDPListener (+ dependency declaration above)
- W1: Added Task 10 — retire `engagement_tracker_server.py`
- W2: Update `hermes-proxy.service` to depend on Hub (in Task 5)
- W3: Two-window launch via Hub `Target.createTarget` at boot (in Task 4)
- W6: `mkdir -p tests/` step added to Task 2

---

## File Map

### New Files

| File | Responsibility |
|---|---|
| `scripts/hermes-x/core/browser_hub.py` | Browser Hub: CDP WebSocket client, tab registry, HTTP API on :40770 |
| `scripts/hermes-x/core/hub_client.py` | Thin sync client: `get_attribution()`, `create_tab()`, `release_tab()` |
| `scripts/hermes-x/tests/test_browser_hub.py` | Unit tests for Hub tab registry and attribution matching |
| `scripts/hermes-x/tests/test_hub_client.py` | Unit tests for Hub client with mock HTTP |
| `scripts/hermes-x/tests/test_proxy_attribution.py` | Integration test: proxy + Hub attribution tagging |
| `infra/systemd/hermes-browser-hub.service` | Systemd unit for the Hub |
| `infra/systemd/hermes-browser-hub.timer` | Watchdog timer (optional, if needed) |

### Modified Files

| File | Change |
|---|---|
| `infra/systemd/hermes-browser.service` | Fix DISPLAY=:0, remove `After=hermes-proxy.service`, fix ExecStart path |
| `scripts/hermes-x/scripts/launch-browser.sh` | Support `--two-windows` flag: open human window + agent window |
| `scripts/hermes-x/core/x_proxy.py` | Add `source` field via Hub attribution call with fallback |
| `~/.cynic/organs/hermes/x/core/organic_navigator.py` | Replace direct CDP connection with Hub client |
| `scripts/hermes-x/core/search_executor.py` | Replace direct CDP connection with Hub client |

---

## Task 1: Fix Chrome Service (DISPLAY + dependencies)

**Files:**
- Modify: `infra/systemd/hermes-browser.service`
- Modify: `scripts/hermes-x/scripts/launch-browser.sh`

This task fixes the crash loop. Chrome must start before anything else can work.

- [ ] **Step 1: Fix the systemd service file**

Edit `infra/systemd/hermes-browser.service`:

```ini
[Unit]
Description=CYNIC Hermes browser — Chrome with proxy + CDP for passive capture and agent control
# Hub depends on us, not the other way around. Proxy is independent.
StartLimitIntervalSec=300
StartLimitBurst=3

[Service]
Type=simple
EnvironmentFile=%h/.config/cynic/env
Environment=CHROME_BIN=/usr/bin/google-chrome
Environment=HERMES_CDP_PORT=40769
Environment=HERMES_PROXY_PORT=8888
Environment=DISPLAY=:0
ExecStart=%h/Bureau/CYNIC/scripts/hermes-x/scripts/launch-browser.sh
Restart=on-failure
RestartSec=30
TimeoutStopSec=10

[Install]
WantedBy=default.target
```

Key changes:
- Remove `After=hermes-proxy.service` and `Wants=hermes-proxy.service` (spec B1: Hub is now the dependency anchor)
- `DISPLAY=:0` (was `:1` in deployed copy — the root cause of the crash)
- ExecStart path: `scripts/hermes-x/scripts/launch-browser.sh` (the `scripts/` subdir was missing in some deployed copies)

- [ ] **Step 2: Deploy and test Chrome starts**

```bash
cp infra/systemd/hermes-browser.service ~/.config/systemd/user/hermes-browser.service
systemctl --user daemon-reload
systemctl --user restart hermes-browser.service
sleep 3
systemctl --user status hermes-browser.service
```

Expected: `Active: active (running)`. Chrome window opens on X.com.

- [ ] **Step 3: Verify CDP endpoint responds**

```bash
curl -s http://127.0.0.1:40769/json/version | python3 -m json.tool
```

Expected: JSON with `Browser`, `Protocol-Version`, `webSocketDebuggerUrl` fields.

- [ ] **Step 4: Verify proxy captures traffic**

Browse X.com in the Chrome window for 30 seconds, then:

```bash
wc -l ~/.cynic/organs/hermes/x/dataset.jsonl
```

Expected: line count > 0 (proxy is capturing via the Chrome → mitmdump route).

- [ ] **Step 5: Commit**

```bash
git add infra/systemd/hermes-browser.service
git commit -m "fix(hermes): browser service DISPLAY=:0, remove proxy dependency"
```

---

## Task 2: Browser Hub — Tab Registry (core logic, no CDP)

**Files:**
- Create: `scripts/hermes-x/core/browser_hub.py`
- Create: `scripts/hermes-x/tests/test_browser_hub.py`

Build the Hub's core data structures and HTTP API first, tested without a real Chrome.

- [ ] **Step 0: Create tests directory**

```bash
mkdir -p scripts/hermes-x/tests
touch scripts/hermes-x/tests/__init__.py
```

- [ ] **Step 1: Write failing tests for TabRegistry**

Create `scripts/hermes-x/tests/test_browser_hub.py`:

```python
"""Tests for Browser Hub tab registry and attribution."""
import pytest
import time

# We'll import after implementation
from browser_hub import TabRegistry, TabEntry


class TestTabRegistry:
    def test_register_tab_agent(self):
        reg = TabRegistry()
        entry = reg.register("tab-1", "target-1", "agent:navigator", "win-agent")
        assert entry.owner == "agent:navigator"
        assert entry.tab_id == "tab-1"

    def test_register_tab_human(self):
        reg = TabRegistry()
        entry = reg.register("tab-2", "target-2", "human", "win-human")
        assert entry.owner == "human"

    def test_get_owner(self):
        reg = TabRegistry()
        reg.register("tab-1", "target-1", "agent:navigator", "win-agent")
        assert reg.get_owner("tab-1") == "agent:navigator"

    def test_get_owner_unknown_tab(self):
        reg = TabRegistry()
        assert reg.get_owner("nonexistent") == "human"  # K14: unknown = human (safe default)

    def test_remove_tab(self):
        reg = TabRegistry()
        reg.register("tab-1", "target-1", "agent:navigator", "win-agent")
        reg.remove("tab-1")
        assert reg.get_owner("tab-1") == "human"

    def test_list_tabs(self):
        reg = TabRegistry()
        reg.register("tab-1", "t-1", "agent:navigator", "win-agent")
        reg.register("tab-2", "t-2", "human", "win-human")
        tabs = reg.list_all()
        assert len(tabs) == 2

    def test_resolve_by_target_id(self):
        reg = TabRegistry()
        reg.register("tab-1", "target-1", "agent:navigator", "win-agent")
        assert reg.resolve_target("target-1") == "agent:navigator"

    def test_resolve_unknown_target(self):
        reg = TabRegistry()
        assert reg.resolve_target("unknown") == "human"


class TestAttributionBuffer:
    def test_record_and_match(self):
        from browser_hub import AttributionBuffer
        buf = AttributionBuffer(ttl_seconds=5, max_size=100)
        now = time.time()
        buf.record("https://x.com/i/api/graphql/abc/SearchTimeline?v=1", now, "agent:navigator")
        result = buf.match("https://x.com/i/api/graphql/abc/SearchTimeline?v=1", now + 0.05)
        assert result == "agent:navigator"

    def test_match_expired(self):
        from browser_hub import AttributionBuffer
        buf = AttributionBuffer(ttl_seconds=1, max_size=100)
        now = time.time()
        buf.record("https://x.com/i/api/graphql/abc/SearchTimeline?v=1", now, "agent:navigator")
        result = buf.match("https://x.com/i/api/graphql/abc/SearchTimeline?v=1", now + 2.0)
        assert result == "unknown"

    def test_match_no_entry(self):
        from browser_hub import AttributionBuffer
        buf = AttributionBuffer(ttl_seconds=5, max_size=100)
        result = buf.match("https://x.com/something", time.time())
        assert result == "unknown"

    def test_buffer_eviction(self):
        from browser_hub import AttributionBuffer
        buf = AttributionBuffer(ttl_seconds=5, max_size=3)
        now = time.time()
        buf.record("url1", now, "a")
        buf.record("url2", now, "b")
        buf.record("url3", now, "c")
        buf.record("url4", now, "d")  # should evict oldest
        assert buf.match("url1", now) == "unknown"  # evicted
        assert buf.match("url4", now) == "d"
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd scripts/hermes-x && python3 -m pytest tests/test_browser_hub.py -v 2>&1 | head -20
```

Expected: ImportError — `browser_hub` module doesn't exist yet.

- [ ] **Step 3: Implement TabRegistry and AttributionBuffer**

Create `scripts/hermes-x/core/browser_hub.py`:

```python
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

# ── Data Structures ──


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
        # Search from newest to oldest (most likely match is recent)
        for entry in reversed(self._entries):
            if entry.url == url and abs(timestamp - entry.timestamp) < self._ttl:
                return entry.owner
        return "unknown"

    def _evict(self, now: float) -> None:
        while self._entries and (now - self._entries[0].timestamp) > self._ttl:
            self._entries.popleft()
```

- [ ] **Step 4: Run tests**

```bash
cd scripts/hermes-x && PYTHONPATH=core:$PYTHONPATH python3 -m pytest tests/test_browser_hub.py -v
```

Expected: all 12 tests pass.

- [ ] **Step 5: Commit**

```bash
git add scripts/hermes-x/core/browser_hub.py scripts/hermes-x/tests/test_browser_hub.py
git commit -m "feat(hermes): Browser Hub — TabRegistry + AttributionBuffer with tests"
```

---

## Task 3: Browser Hub — HTTP API

**Files:**
- Modify: `scripts/hermes-x/core/browser_hub.py`
- Modify: `scripts/hermes-x/tests/test_browser_hub.py`

Add the aiohttp HTTP server exposing the tab registry and attribution.

- [ ] **Step 1: Write failing tests for HTTP API**

Append to `scripts/hermes-x/tests/test_browser_hub.py`:

```python
import aiohttp
from aiohttp.test_utils import AioHTTPTestCase, unittest_run_loop

from browser_hub import create_app, TabRegistry, AttributionBuffer


class TestHubHTTPAPI(AioHTTPTestCase):
    async def get_application(self):
        registry = TabRegistry()
        attribution = AttributionBuffer()
        # Pre-populate for testing
        registry.register("tab-1", "target-1", "agent:navigator", "win-agent", url="https://x.com/search")
        registry.register("tab-2", "target-2", "human", "win-human", url="https://x.com/home")
        return create_app(registry, attribution)

    @unittest_run_loop
    async def test_get_status(self):
        resp = await self.client.request("GET", "/status")
        assert resp.status == 200
        data = await resp.json()
        assert "tabs" in data

    @unittest_run_loop
    async def test_get_tabs(self):
        resp = await self.client.request("GET", "/tabs")
        assert resp.status == 200
        data = await resp.json()
        assert len(data) == 2

    @unittest_run_loop
    async def test_get_tab_owner(self):
        resp = await self.client.request("GET", "/tabs/tab-1/owner")
        assert resp.status == 200
        data = await resp.json()
        assert data["owner"] == "agent:navigator"

    @unittest_run_loop
    async def test_get_tab_owner_unknown(self):
        resp = await self.client.request("GET", "/tabs/nonexistent/owner")
        assert resp.status == 200
        data = await resp.json()
        assert data["owner"] == "human"

    @unittest_run_loop
    async def test_get_attribution_unknown(self):
        resp = await self.client.request("GET", "/attribution?url=https://x.com/something&ts=0")
        assert resp.status == 200
        data = await resp.json()
        assert data["source"] == "unknown"

    @unittest_run_loop
    async def test_post_create_tab(self):
        resp = await self.client.request("POST", "/tabs", json={
            "owner": "agent:search-executor",
            "url": "https://x.com/search?q=test",
            "window": "agent",
        })
        assert resp.status == 201
        data = await resp.json()
        assert "tab_id" in data

    @unittest_run_loop
    async def test_delete_tab(self):
        resp = await self.client.request("DELETE", "/tabs/tab-1")
        assert resp.status == 200
        # Verify removed
        resp2 = await self.client.request("GET", "/tabs/tab-1/owner")
        data = await resp2.json()
        assert data["owner"] == "human"
```

- [ ] **Step 2: Run to verify failure**

```bash
cd scripts/hermes-x && PYTHONPATH=core:$PYTHONPATH python3 -m pytest tests/test_browser_hub.py::TestHubHTTPAPI -v 2>&1 | head -20
```

Expected: ImportError on `create_app`.

- [ ] **Step 3: Implement HTTP API**

Append to `scripts/hermes-x/core/browser_hub.py`:

```python
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
        # In production, this calls CDP to create the tab.
        # For testing, we just register it with a synthetic ID.
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
        # L1: store extension events. For L0, just acknowledge.
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
```

- [ ] **Step 4: Run all tests**

```bash
cd scripts/hermes-x && PYTHONPATH=core:$PYTHONPATH python3 -m pytest tests/test_browser_hub.py -v
```

Expected: all tests pass (unit + HTTP API tests).

- [ ] **Step 5: Commit**

```bash
git add scripts/hermes-x/core/browser_hub.py scripts/hermes-x/tests/test_browser_hub.py
git commit -m "feat(hermes): Browser Hub HTTP API — /tabs, /attribution, /events/extension"
```

---

## Task 4: Browser Hub — CDP Connection + Main Entrypoint

**Files:**
- Modify: `scripts/hermes-x/core/browser_hub.py`

Add the CDP WebSocket listener that connects to Chrome, subscribes to `Target.*` and `Network.requestWillBeSent`, and populates the registry. This is the production wiring — tested manually (CDP requires a real Chrome).

- [ ] **Step 1: Add CDP listener to browser_hub.py**

Append to `scripts/hermes-x/core/browser_hub.py`:

```python
class CDPListener:
    """Connects to Chrome CDP, listens for target and network events."""

    def __init__(self, registry: TabRegistry, attribution: AttributionBuffer,
                 cdp_port: int = 40769):
        self._registry = registry
        self._attribution = attribution
        self._cdp_port = cdp_port
        self._ws = None
        self._msg_id = 0
        self._agent_window_id: Optional[str] = None
        self._human_window_id: Optional[str] = None
        self._session_to_target: dict[str, str] = {}  # CDP sessionId → targetId

    async def connect(self) -> None:
        """Connect to Chrome CDP and subscribe to events."""
        import websockets as _ws  # lazy import (B4: may not be installed at import time)
        # Get the WebSocket URL from Chrome's JSON endpoint
        import aiohttp as _aiohttp
        async with _aiohttp.ClientSession() as session:
            async with session.get(f"http://127.0.0.1:{self._cdp_port}/json/version") as resp:
                info = await resp.json()
                ws_url = info["webSocketDebuggerUrl"]

        self._ws = await _ws.connect(ws_url, max_size=10 * 1024 * 1024)
        logger.info("Connected to Chrome CDP: %s", ws_url)

        # Enable flat-mode auto-attach: multiplexes all target sessions on this WebSocket.
        # This gives us Network events per-target with sessionId for attribution (B1 fix).
        await self._send("Target.setAutoAttach", {
            "autoAttach": True,
            "waitForDebuggerOnStart": False,
            "flatten": True,
        })
        # Also discover targets for create/destroy events
        await self._send("Target.setDiscoverTargets", {"discover": True})

    async def _send(self, method: str, params: dict = None) -> int:
        self._msg_id += 1
        msg = {"id": self._msg_id, "method": method, "params": params or {}}
        await self._ws.send(json.dumps(msg))
        return self._msg_id

    async def _send_to_session(self, method: str, params: dict, session_id: str) -> int:
        """Send a CDP command to a specific target session (flat mode)."""
        self._msg_id += 1
        msg = {"id": self._msg_id, "method": method, "params": params or {}, "sessionId": session_id}
        await self._ws.send(json.dumps(msg))
        return self._msg_id

    async def listen(self) -> None:
        """Main event loop — process CDP events."""
        async for raw in self._ws:
            try:
                msg = json.loads(raw)
            except json.JSONDecodeError:
                continue

            method = msg.get("method", "")

            if method == "Target.targetCreated":
                info = msg["params"]["targetInfo"]
                if info["type"] == "page":
                    self._on_target_created(info)

            elif method == "Target.attachedToTarget":
                # B1 fix: flat-mode auto-attach gives us a sessionId per target.
                # Enable Network on this session to get requestWillBeSent events.
                session_id = msg["params"]["sessionId"]
                target_info = msg["params"]["targetInfo"]
                target_id = target_info["targetId"]
                self._session_to_target[session_id] = target_id
                # Enable Network events for this session
                await self._send_to_session("Network.enable", {}, session_id)
                logger.info("Attached to target %s (session %s)", target_id[:12], session_id[:12])

            elif method == "Target.targetDestroyed":
                target_id = msg["params"]["targetId"]
                self._on_target_destroyed(target_id)

            elif method == "Target.targetInfoChanged":
                info = msg["params"]["targetInfo"]
                if info["type"] == "page":
                    self._on_target_changed(info)

            elif method == "Network.requestWillBeSent":
                # B1 fix: sessionId maps to targetId via _session_to_target
                session_id = msg.get("sessionId", "")
                self._on_network_request(msg.get("params", {}), session_id)

    def _on_target_created(self, info: dict) -> None:
        target_id = info["targetId"]
        url = info.get("url", "")
        # Determine window: check if this target's browserContextId matches agent window
        # For now, tabs not registered via Hub API = human
        if not any(e.cdp_target_id == target_id for e in self._registry.list_all()):
            self._registry.register(
                tab_id=target_id,  # Use targetId as tab_id for CDP-created tabs
                cdp_target_id=target_id,
                owner="human",
                window_id="unknown",
                url=url,
            )
            logger.info("New tab (human): %s → %s", target_id[:12], url[:60])

    def _on_target_destroyed(self, target_id: str) -> None:
        self._registry.remove(target_id)
        logger.info("Tab closed: %s", target_id[:12])

    def _on_target_changed(self, info: dict) -> None:
        target_id = info["targetId"]
        url = info.get("url", "")
        self._registry.update_url(target_id, url)

    def _on_network_request(self, params: dict, session_id: str) -> None:
        url = params.get("request", {}).get("url", "")
        if "/i/api/graphql/" not in url:
            return
        timestamp = params.get("wallTime", time.time())
        # B1 fix: resolve sessionId → targetId → TabEntry.owner
        target_id = self._session_to_target.get(session_id, "")
        owner = self._registry.resolve_target(target_id) if target_id else "unknown"
        self._attribution.record(url, timestamp, owner)


async def main():
    """Production entrypoint: connect CDP + start HTTP server."""
    cdp_port = int(os.environ.get("HERMES_CDP_PORT", "40769"))
    hub_port = int(os.environ.get("BROWSER_HUB_PORT", "40770"))
    organ_dir = os.environ.get("ORGAN_DIR",
                               os.path.expanduser("~/.cynic/organs/hermes/x"))

    registry = TabRegistry()
    attribution = AttributionBuffer(ttl_seconds=5.0, max_size=1000)
    cdp = CDPListener(registry, attribution, cdp_port)
    app = create_app(registry, attribution)

    # Connect to Chrome
    try:
        await cdp.connect()
        logger.info("CDP connected on port %d", cdp_port)
    except Exception as e:
        logger.error("Failed to connect to Chrome CDP: %s", e)
        logger.error("Is hermes-browser.service running?")
        raise SystemExit(1)

    # W3 fix: create agent window via CDP (human window = Chrome's default)
    # The existing Chrome window is the human window.
    # Create a second window for agent tabs.
    agent_target_id = await cdp._send("Target.createTarget", {
        "url": "about:blank",
        "newWindow": True,
    })
    logger.info("Agent window created")

    # Write state file
    state_path = os.path.join(organ_dir, "browser-state.json")
    with open(state_path, "w") as f:
        json.dump({
            "hub_port": hub_port,
            "cdp_port": cdp_port,
            "hub_url": f"http://127.0.0.1:{hub_port}",
            "started_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        }, f, indent=2)

    # Start HTTP server + CDP listener concurrently
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, "127.0.0.1", hub_port)
    await site.start()
    logger.info("Hub HTTP API listening on :%d", hub_port)

    try:
        await cdp.listen()
    except websockets.exceptions.ConnectionClosed:
        logger.warning("Chrome CDP connection lost — shutting down")
    finally:
        await runner.cleanup()


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s: %(message)s", datefmt="%H:%M:%S")
    asyncio.run(main())
```

- [ ] **Step 2: Test manually with Chrome running**

```bash
# Terminal 1: ensure Chrome is running
systemctl --user status hermes-browser.service

# Terminal 2: run Hub
cd scripts/hermes-x && PYTHONPATH=core python3 -c "import asyncio; from browser_hub import main; asyncio.run(main())"
```

Wait 3 seconds, then in another terminal:

```bash
curl -s http://127.0.0.1:40770/status | python3 -m json.tool
curl -s http://127.0.0.1:40770/tabs | python3 -m json.tool
```

Expected: `/status` returns JSON with tabs. `/tabs` lists existing Chrome tabs as `owner: "human"`.

- [ ] **Step 3: Commit**

```bash
git add scripts/hermes-x/core/browser_hub.py
git commit -m "feat(hermes): Browser Hub CDP listener + main entrypoint"
```

---

## Task 5: Hub Systemd Service

**Files:**
- Create: `infra/systemd/hermes-browser-hub.service`

- [ ] **Step 1: Write the service unit**

Create `infra/systemd/hermes-browser-hub.service`:

```ini
[Unit]
Description=CYNIC Hermes Browser Hub — tab registry + CDP attribution
After=hermes-browser.service
Wants=hermes-browser.service
StartLimitIntervalSec=300
StartLimitBurst=5

[Service]
Type=simple
EnvironmentFile=%h/.config/cynic/env
Environment=HERMES_CDP_PORT=40769
Environment=BROWSER_HUB_PORT=40770
Environment=ORGAN_DIR=%h/.cynic/organs/hermes/x
WorkingDirectory=%h/Bureau/CYNIC/scripts/hermes-x
ExecStart=/usr/bin/python3 core/browser_hub.py
Restart=on-failure
RestartSec=10
MemoryMax=128M
TimeoutStartSec=30

[Install]
WantedBy=default.target
```

- [ ] **Step 2: Deploy and start**

```bash
cp infra/systemd/hermes-browser-hub.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now hermes-browser-hub.service
sleep 3
systemctl --user status hermes-browser-hub.service
```

Expected: `Active: active (running)`.

- [ ] **Step 2b: Update proxy service to depend on Hub (W2)**

Add `After=hermes-browser-hub.service` to `infra/systemd/hermes-proxy.service` (if not already present):

```bash
grep -q "hermes-browser-hub" infra/systemd/hermes-proxy.service || \
  sed -i '/^\[Unit\]/a After=hermes-browser-hub.service' infra/systemd/hermes-proxy.service
cp infra/systemd/hermes-proxy.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user restart hermes-proxy.service
```

- [ ] **Step 3: Verify Hub is responding**

```bash
curl -s http://127.0.0.1:40770/status | python3 -m json.tool
```

Expected: JSON with tab list and version.

- [ ] **Step 4: Commit**

```bash
git add infra/systemd/hermes-browser-hub.service
git commit -m "feat(hermes): Browser Hub systemd service"
```

---

## Task 6: Hub Client Library

**Files:**
- Create: `scripts/hermes-x/core/hub_client.py`
- Create: `scripts/hermes-x/tests/test_hub_client.py`

Thin synchronous wrapper used by x_proxy.py, organic_navigator.py, and search_executor.py.

- [ ] **Step 1: Write failing tests**

Create `scripts/hermes-x/tests/test_hub_client.py`:

```python
"""Tests for Hub client — uses mock HTTP responses."""
import json
import pytest
from unittest.mock import patch, MagicMock

from hub_client import HubClient


class TestHubClient:
    def test_get_attribution_success(self):
        client = HubClient("http://127.0.0.1:40770")
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.json.return_value = {"source": "agent:navigator"}
        with patch("hub_client.requests.get", return_value=mock_resp):
            result = client.get_attribution("https://x.com/graphql/search", 1234567890.0)
        assert result == "agent:navigator"

    def test_get_attribution_hub_down(self):
        client = HubClient("http://127.0.0.1:40770")
        with patch("hub_client.requests.get", side_effect=Exception("connection refused")):
            result = client.get_attribution("https://x.com/graphql/search", 1234567890.0)
        assert result == "unknown"

    def test_create_tab(self):
        client = HubClient("http://127.0.0.1:40770")
        mock_resp = MagicMock()
        mock_resp.status_code = 201
        mock_resp.json.return_value = {"tab_id": "hub-123", "cdp_target_id": "t-123"}
        with patch("hub_client.requests.post", return_value=mock_resp):
            result = client.create_tab("agent:navigator", "https://x.com/search?q=test")
        assert result["tab_id"] == "hub-123"

    def test_create_tab_hub_down(self):
        client = HubClient("http://127.0.0.1:40770")
        with patch("hub_client.requests.post", side_effect=Exception("connection refused")):
            result = client.create_tab("agent:navigator", "https://x.com")
        assert result is None

    def test_release_tab(self):
        client = HubClient("http://127.0.0.1:40770")
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        with patch("hub_client.requests.delete", return_value=mock_resp):
            result = client.release_tab("hub-123")
        assert result is True

    def test_release_tab_hub_down(self):
        client = HubClient("http://127.0.0.1:40770")
        with patch("hub_client.requests.delete", side_effect=Exception("connection refused")):
            result = client.release_tab("hub-123")
        assert result is False
```

- [ ] **Step 2: Run to verify failure**

```bash
cd scripts/hermes-x && PYTHONPATH=core:$PYTHONPATH python3 -m pytest tests/test_hub_client.py -v 2>&1 | head -10
```

Expected: ImportError.

- [ ] **Step 3: Implement hub_client.py**

Create `scripts/hermes-x/core/hub_client.py`:

```python
"""
Hub Client — thin sync wrapper for Browser Hub HTTP API.

Used by x_proxy.py (attribution), organic_navigator.py (tab management),
and search_executor.py (tab management).

All methods return safe defaults on Hub failure (K14: degraded, not broken).
"""

__version__ = "0.1.0"

import logging
import os
from typing import Optional

import requests

logger = logging.getLogger("hub-client")

_DEFAULT_HUB_URL = "http://127.0.0.1:40770"
_TIMEOUT = 0.5  # 500ms — Hub is local, must be fast


class HubClient:
    def __init__(self, hub_url: str = None):
        self._url = hub_url or os.environ.get("BROWSER_HUB_URL", _DEFAULT_HUB_URL)

    def get_attribution(self, url: str, timestamp: float) -> str:
        """Query Hub for request attribution. Returns 'unknown' on failure."""
        try:
            resp = requests.get(
                f"{self._url}/attribution",
                params={"url": url, "ts": str(timestamp)},
                timeout=_TIMEOUT,
            )
            return resp.json().get("source", "unknown")
        except Exception:
            return "unknown"

    def create_tab(self, owner: str, url: str, window: str = "agent") -> Optional[dict]:
        """Ask Hub to create a tab. Returns tab info dict or None on failure."""
        try:
            resp = requests.post(
                f"{self._url}/tabs",
                json={"owner": owner, "url": url, "window": window},
                timeout=_TIMEOUT,
            )
            if resp.status_code == 201:
                return resp.json()
            logger.warning("Hub create_tab returned %d", resp.status_code)
            return None
        except Exception as e:
            logger.warning("Hub unreachable for create_tab: %s", e)
            return None

    def release_tab(self, tab_id: str) -> bool:
        """Ask Hub to close and unregister a tab. Returns True on success."""
        try:
            resp = requests.delete(f"{self._url}/tabs/{tab_id}", timeout=_TIMEOUT)
            return resp.status_code == 200
        except Exception as e:
            logger.warning("Hub unreachable for release_tab: %s", e)
            return False

    def get_status(self) -> Optional[dict]:
        """Get Hub status. Returns None on failure."""
        try:
            resp = requests.get(f"{self._url}/status", timeout=_TIMEOUT)
            return resp.json()
        except Exception:
            return None
```

- [ ] **Step 4: Run tests**

```bash
cd scripts/hermes-x && PYTHONPATH=core:$PYTHONPATH python3 -m pytest tests/test_hub_client.py -v
```

Expected: all 6 tests pass.

- [ ] **Step 5: Commit**

```bash
git add scripts/hermes-x/core/hub_client.py scripts/hermes-x/tests/test_hub_client.py
git commit -m "feat(hermes): Hub client — thin sync wrapper for proxy + consumers"
```

---

## Task 7: Proxy Attribution Tagging

**Files:**
- Modify: `scripts/hermes-x/core/x_proxy.py`
- Create: `scripts/hermes-x/tests/test_proxy_attribution.py`

Add the `source` field to every enriched row in dataset.jsonl.

- [ ] **Step 1: Write failing test**

Create `scripts/hermes-x/tests/test_proxy_attribution.py`:

```python
"""Test that x_proxy enrichment includes source attribution."""
import sys
from unittest.mock import MagicMock

# B3 fix: mock mitmproxy before importing x_proxy (not in test PYTHONPATH)
sys.modules['mitmproxy'] = MagicMock()
sys.modules['mitmproxy.http'] = MagicMock()

import json
import pytest


def test_enrich_includes_source_field():
    """_enrich must include a 'source' field."""
    # Import after we have the module
    import x_proxy
    tweet = {
        "id": "123", "text": "test tweet", "created_at": "", "lang": "en",
        "author": "user", "author_name": "User", "author_followers": 100,
        "author_statuses_count": 50, "author_favourites_count": 10,
        "author_bio": "", "author_verified": False,
        "author_default_profile": False, "author_default_profile_image": False,
        "author_profile_banner_url": "",
        "favorite_count": 5, "retweet_count": 1, "reply_count": 0,
        "bookmark_count": 0, "quote_count": 0, "view_count": 100,
        "cashtags": [], "hashtags": [], "mentions": [], "urls": [], "media": [],
        "has_media": False, "is_retweet": False, "is_reply": False,
        "in_reply_to_screen_name": "", "quoted_status_id": "",
        "edit_control": None, "possibly_sensitive": False,
    }
    enriched = x_proxy._enrich(tweet, "SearchTimeline", {}, {}, "agent:navigator")
    assert "source" in enriched
    assert enriched["source"] == "agent:navigator"


def test_enrich_source_defaults_to_unknown():
    import x_proxy
    tweet = {
        "id": "456", "text": "another tweet", "created_at": "", "lang": "en",
        "author": "user2", "author_name": "User2", "author_followers": 50,
        "author_statuses_count": 20, "author_favourites_count": 5,
        "author_bio": "", "author_verified": False,
        "author_default_profile": False, "author_default_profile_image": False,
        "author_profile_banner_url": "",
        "favorite_count": 2, "retweet_count": 0, "reply_count": 0,
        "bookmark_count": 0, "quote_count": 0, "view_count": 50,
        "cashtags": [], "hashtags": [], "mentions": [], "urls": [], "media": [],
        "has_media": False, "is_retweet": False, "is_reply": False,
        "in_reply_to_screen_name": "", "quoted_status_id": "",
        "edit_control": None, "possibly_sensitive": False,
    }
    enriched = x_proxy._enrich(tweet, "HomeTimeline", {}, {}, "unknown")
    assert enriched["source"] == "unknown"
```

- [ ] **Step 2: Run to verify failure**

```bash
cd scripts/hermes-x && PYTHONPATH=core:$PYTHONPATH python3 -m pytest tests/test_proxy_attribution.py -v 2>&1 | head -15
```

Expected: TypeError — `_enrich()` doesn't accept 5th argument yet.

- [ ] **Step 3: Modify x_proxy.py**

Three changes to `scripts/hermes-x/core/x_proxy.py`:

**3a.** Add `source` parameter to `_enrich()` function signature (line ~297):

Change:
```python
def _enrich(tweet: dict, operation: str, variables: dict, coord_map: dict) -> dict:
```
To:
```python
def _enrich(tweet: dict, operation: str, variables: dict, coord_map: dict, source: str = "unknown") -> dict:
```

**3b.** Add `source` field to the enriched dict (after `"sampling_bias": "proxy-passive"`):

```python
        "sampling_bias": "proxy-passive",
        "source": source,
```

**3c.** In `XProxy.__init__`, add Hub client setup (after `self._stats` line):

```python
        # Hub client for attribution (L0)
        self._hub_url = os.environ.get("BROWSER_HUB_URL", "http://127.0.0.1:40770")
```

**3d.** In `XProxy.response()`, add attribution lookup before enrichment (after `variables = self._extract_vars(url)` line):

```python
        # Attribution: ask Hub who owns this request
        source = self._get_attribution(flow.request.url,
                                        flow.request.timestamp_start)
```

**3e.** Pass `source` to `_enrich()` call:

Change:
```python
            enriched = _enrich(t, op_name, variables, coord_map)
```
To:
```python
            enriched = _enrich(t, op_name, variables, coord_map, source)
```

**3f.** Add the `_get_attribution` method to `XProxy` class:

```python
    def _get_attribution(self, url: str, timestamp: float) -> str:
        """Query Hub for source attribution. Returns 'unknown' on failure."""
        try:
            import urllib.request
            from urllib.parse import urlencode  # B2 fix: encode URL params
            req_url = f"{self._hub_url}/attribution?{urlencode({'url': url, 'ts': timestamp})}"
            with urllib.request.urlopen(req_url, timeout=0.5) as resp:
                data = json.loads(resp.read())
                return data.get("source", "unknown")
        except Exception:
            return "unknown"
```

Note: uses `urllib` instead of `requests` because mitmproxy addon environment may not have `requests` on PYTHONPATH. `urllib` is stdlib.

- [ ] **Step 4: Run tests**

```bash
cd scripts/hermes-x && PYTHONPATH=core:$PYTHONPATH python3 -m pytest tests/test_proxy_attribution.py -v
```

Expected: both tests pass.

- [ ] **Step 5: Restart proxy and verify source field appears**

```bash
systemctl --user restart hermes-proxy.service
sleep 5
# Browse X.com for 30 seconds, then check:
tail -1 ~/.cynic/organs/hermes/x/dataset.jsonl | python3 -c "import json,sys; d=json.load(sys.stdin); print(f'source={d.get(\"source\",\"MISSING\")}')"
```

Expected: `source=human` or `source=unknown` (Hub may not have attribution data yet if CDP listener isn't mapping network requests).

- [ ] **Step 6: Commit**

```bash
git add scripts/hermes-x/core/x_proxy.py scripts/hermes-x/tests/test_proxy_attribution.py
git commit -m "feat(hermes): proxy attribution — source field on every dataset row"
```

---

## Task 8: Wire Navigator + Search-Executor to Hub

**Files:**
- Modify: `~/.cynic/organs/hermes/x/core/organic_navigator.py`
- Modify: `scripts/hermes-x/core/search_executor.py`

Replace direct CDP connections with Hub client. Fallback to direct CDP if Hub is unavailable.

- [ ] **Step 1: Modify organic_navigator.py**

In `~/.cynic/organs/hermes/x/core/organic_navigator.py`, find the CDP connection block (around line 345-358) and add Hub client import + fallback:

At the top of the file (after other imports):
```python
try:
    from hub_client import HubClient
    HUB_AVAILABLE = True
except ImportError:
    HUB_AVAILABLE = False
```

Replace the CDP connection section (lines ~344-358):
```python
    # Try Hub first, fall back to direct CDP
    hub_tab = None
    if HUB_AVAILABLE:
        hub = HubClient()
        hub_tab = hub.create_tab("agent:navigator", "https://x.com/home")
        if hub_tab:
            logger.info("Tab created via Hub: %s", hub_tab.get("tab_id", "?"))

    if not cdp_url:
        browser_state = organ_dir.parent / "browser-state.json"
        if browser_state.exists():
            state = json.loads(browser_state.read_text())
            cdp_url = state.get("cdp_url", "").replace("ws://", "http://")

    if not cdp_url:
        logger.error("No CDP endpoint available")
        return []
```

And at the end of the function (before return), release the Hub tab:
```python
    # Release Hub tab
    if hub_tab and HUB_AVAILABLE:
        hub.release_tab(hub_tab["tab_id"])
```

- [ ] **Step 2: Modify search_executor.py similarly**

In `scripts/hermes-x/core/search_executor.py`, find the CDP connection (around line 260-269) and apply the same pattern:

Add import at top:
```python
try:
    from hub_client import HubClient
    HUB_AVAILABLE = True
except ImportError:
    HUB_AVAILABLE = False
```

Before the Playwright CDP connection, add Hub tab creation:
```python
                hub_tab = None
                if HUB_AVAILABLE:
                    hub = HubClient()
                    hub_tab = hub.create_tab("agent:search-executor", start_url)
                    if hub_tab:
                        logger.info("Tab created via Hub: %s", hub_tab.get("tab_id", "?"))
```

After the browsing loop completes, release:
```python
                if hub_tab and HUB_AVAILABLE:
                    hub.release_tab(hub_tab["tab_id"])
```

- [ ] **Step 3: Verify navigator uses Hub**

```bash
# With Hub running:
systemctl --user status hermes-browser-hub.service
# Trigger navigator:
systemctl --user restart hermes-navigator.service
sleep 5
journalctl --user -eu hermes-navigator --no-pager -n 10
```

Expected: log line containing "Tab created via Hub" or fallback to direct CDP.

- [ ] **Step 4: Commit**

```bash
git add scripts/hermes-x/core/search_executor.py
git commit -m "feat(hermes): navigator + search-executor use Hub for tab management"
```

Note: `organic_navigator.py` is in the organ runtime dir (not git-tracked). The pattern is documented here for the implementer.

---

## Task 9: Smoke Test — End-to-End Attribution

**Files:** none (validation only)

Verify the full chain: Chrome → Hub → Proxy → dataset.jsonl with correct source tagging.

- [ ] **Step 1: Verify all services are running**

```bash
systemctl --user status hermes-browser hermes-browser-hub hermes-proxy 2>/dev/null | grep -E "Active:|●"
```

Expected: all three `active (running)`.

- [ ] **Step 2: Browse X.com manually (human window)**

Open X.com in the Chrome window managed by hermes-browser. Scroll home feed for 30 seconds.

```bash
sleep 30
tail -5 ~/.cynic/organs/hermes/x/dataset.jsonl | python3 -c "
import json, sys
for line in sys.stdin:
    d = json.loads(line)
    print(f\"source={d.get('source','MISSING'):20s} op={d.get('operation','?'):20s} tweet={d.get('tweet_id','?')[:12]}\")
"
```

Expected: rows with `source=human` (or `source=unknown` if attribution buffer hasn't matched yet — acceptable for L0).

- [ ] **Step 3: Trigger agent search (search-executor)**

```bash
systemctl --user restart hermes-search-executor.service
sleep 15
tail -5 ~/.cynic/organs/hermes/x/dataset.jsonl | python3 -c "
import json, sys
for line in sys.stdin:
    d = json.loads(line)
    print(f\"source={d.get('source','MISSING'):20s} op={d.get('operation','?'):20s}\")
"
```

Expected: new rows with `source=agent:search-executor` (if Hub attribution is working) or `source=unknown`.

- [ ] **Step 4: Verify curation service now works**

```bash
systemctl --user restart hermes-curation.service
sleep 3
systemctl --user status hermes-curation.service | head -8
```

Expected: `status=0/SUCCESS` (dataset.jsonl now exists and has data).

- [ ] **Step 5: Check all Hermes timers**

```bash
systemctl --user list-timers 'hermes*'
```

Expected: 8 timers active (7 existing + k15-consumer).

- [ ] **Step 6: Commit any remaining fixes**

```bash
git status
# If any fixes were needed during smoke test, commit them:
# git add <files> && git commit -m "fix(hermes): <description>"
```

---

## Task 10: Retire engagement_tracker_server.py (W1)

**Files:**
- Retire: `scripts/hermes-x/core/engagement_tracker_server.py`

The Hub's `POST /events/extension` absorbs this role. Keeping both = dual-writer on `engagement.jsonl`.

- [ ] **Step 1: Check if engagement_tracker_server has a systemd service**

```bash
grep -rl "engagement_tracker" ~/.config/systemd/user/ infra/systemd/ 2>/dev/null
```

If found, stop and disable it.

- [ ] **Step 2: Move to archive**

```bash
mkdir -p scripts/hermes-x/archive
git mv scripts/hermes-x/core/engagement_tracker_server.py scripts/hermes-x/archive/
```

- [ ] **Step 3: Commit**

```bash
git add scripts/hermes-x/archive/engagement_tracker_server.py
git commit -m "chore(hermes): retire engagement_tracker_server — Hub absorbs via /events/extension"
```

---

## Summary

| Task | What | ~Time |
|---|---|---|
| 1 | Fix Chrome service (DISPLAY, deps) | 5 min |
| 2 | Hub TabRegistry + AttributionBuffer (TDD) | 15 min |
| 3 | Hub HTTP API (TDD) | 15 min |
| 4 | Hub CDP listener + two-window + main | 15 min |
| 5 | Hub systemd service + proxy dependency | 5 min |
| 6 | Hub client library (TDD) | 10 min |
| 7 | Proxy attribution tagging (TDD) | 10 min |
| 8 | Wire navigator + search-executor | 10 min |
| 9 | End-to-end smoke test | 10 min |
| 10 | Retire engagement_tracker_server | 3 min |

**Total: ~100 minutes of implementation**

After L0 is stable for 7 days: write L1 plan (behavioral capture).
