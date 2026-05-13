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
        # Simple CORS handling
        headers = {
            "Access-Control-Allow-Origin": "*",
            "Access-Control-Allow-Methods": "POST, OPTIONS",
            "Access-Control-Allow-Headers": "Content-Type",
        }
        
        if request.method == "OPTIONS":
            return web.Response(status=200, headers=headers)
            
        try:
            body = await request.json()
        except Exception:
            return web.json_response({"error": "invalid json"}, status=400, headers=headers)
            
        event_type = body.get("type", "unknown")
        logger.info("Extension event: %s", event_type)
        
        # Persistent logging for behavioral analysis
        log_dir = Path(organ_dir) / "behavior"
        log_dir.mkdir(parents=True, exist_ok=True)
        log_file = log_dir / "engagement.jsonl"
        
        with open(log_file, "a") as f:
            f.write(json.dumps(body) + "\n")
            
        return web.json_response({"ok": True}, headers=headers)

    app = web.Application()
    app.router.add_get("/status", handle_status)
    app.router.add_get("/tabs", handle_get_tabs)
    app.router.add_get("/tabs/{tab_id}/owner", handle_get_tab_owner)
    app.router.add_post("/tabs", handle_post_tab)
    app.router.add_delete("/tabs/{tab_id}", handle_delete_tab)
    app.router.add_get("/attribution", handle_attribution)
    app.router.add_post("/events/extension", handle_extension_event)
    app.router.add_options("/events/extension", handle_extension_event)
    return app


# ---------------------------------------------------------------------------
# CDP Listener
# ---------------------------------------------------------------------------

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
        import websockets as _ws  # lazy import
        import aiohttp as _aiohttp
        async with _aiohttp.ClientSession() as session:
            async with session.get(f"http://127.0.0.1:{self._cdp_port}/json/version") as resp:
                info = await resp.json()
                ws_url = info["webSocketDebuggerUrl"]

        self._ws = await _ws.connect(ws_url, max_size=10 * 1024 * 1024)
        logger.info("Connected to Chrome CDP: %s", ws_url)

        # Flat-mode auto-attach: multiplexes all target sessions on this WebSocket.
        # Gives us Network events per-target with sessionId for attribution.
        await self._send("Target.setAutoAttach", {
            "autoAttach": True,
            "waitForDebuggerOnStart": False,
            "flatten": True,
        })
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
                session_id = msg["params"]["sessionId"]
                target_info = msg["params"]["targetInfo"]
                target_id = target_info["targetId"]
                self._session_to_target[session_id] = target_id
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
                session_id = msg.get("sessionId", "")
                self._on_network_request(msg.get("params", {}), session_id)

    def _on_target_created(self, info: dict) -> None:
        target_id = info["targetId"]
        url = info.get("url", "")
        if not any(e.cdp_target_id == target_id for e in self._registry.list_all()):
            self._registry.register(
                tab_id=target_id,
                cdp_target_id=target_id,
                owner="human",
                window_id="unknown",
                url=url,
            )
            logger.info("New tab (human): %s → %s", target_id[:12], url[:60])

    def _on_target_destroyed(self, target_id: str) -> None:
        self._registry.remove(target_id)
        # Clean up session mappings
        self._session_to_target = {
            s: t for s, t in self._session_to_target.items() if t != target_id
        }
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
        target_id = self._session_to_target.get(session_id, "")
        owner = self._registry.resolve_target(target_id) if target_id else "unknown"
        self._attribution.record(url, timestamp, owner)


# ---------------------------------------------------------------------------
# Main entrypoint
# ---------------------------------------------------------------------------

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

    # Create agent window (human window = Chrome's default/existing)
    await cdp._send("Target.createTarget", {
        "url": "about:blank",
        "newWindow": True,
    })
    logger.info("Agent window created")

    # Write state file
    state_path = os.path.join(organ_dir, "browser-state.json")
    os.makedirs(os.path.dirname(state_path), exist_ok=True)
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
    except Exception as e:
        logger.warning("CDP connection lost: %s — shutting down", e)
    finally:
        await runner.cleanup()


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s: %(message)s", datefmt="%H:%M:%S")
    asyncio.run(main())
