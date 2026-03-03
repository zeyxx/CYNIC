"""
CYNIC Somatic Conduits  The IO Drivers.

Implements specific protocol drivers for the Somatic Gateway.
Includes:
  - StreamConduit: Raw WebSockets via VascularSystem.
  - BrowserConduit: Headless browser monitoring via Playwright.
  - PolledConduit: HTTP polling via VascularSystem.

Lentilles : Backend (High-performance), SRE (Resilience).
"""

from __future__ import annotations

import asyncio
import logging
from typing import Any, Callable, Optional

from playwright.async_api import async_playwright, Browser, BrowserContext, Page

from cynic.kernel.core.vascular import VascularSystem
from cynic.kernel.organism.perception.somatic_gateway import Conduit

logger = logging.getLogger("cynic.perception.conduits")


class StreamConduit(Conduit):
    """Direct WebSocket driver."""

    def __init__(self, conduit_id: str, url: str, vascular: VascularSystem):
        super().__init__(conduit_id)
        self.url = url
        self.vascular = vascular
        self._running = False
        self._task: Optional[asyncio.Task] = None

    async def start(self, ingest_cb: Callable[[str, Any], None]):
        await super().start(ingest_cb)
        self._running = True
        self._task = asyncio.create_task(self._read_loop())
        logger.info(f"StreamConduit {self.conduit_id} started on {self.url}")

    async def stop(self):
        self._running = False
        if self._task:
            self._task.cancel()
        logger.info(f"StreamConduit {self.conduit_id} stopped.")

    async def _read_loop(self):
        while self._running:
            try:
                ws = await self.vascular.open_stream(self.url)
                async for message in ws:
                    if not self._running:
                        break
                    if self._ingest_cb:
                        await self._ingest_cb(self.conduit_id, message)
            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error(f"StreamConduit {self.conduit_id} error: {e}")
                await asyncio.sleep(5)  # Backoff


class BrowserConduit(Conduit):
    """Playwright-based driver for complex sites."""

    def __init__(self, conduit_id: str, url: str):
        super().__init__(conduit_id)
        self.url = url
        self._playwright = None
        self._browser: Optional[Browser] = None
        self._context: Optional[BrowserContext] = None
        self.page: Optional[Page] = None
        self._running = False
        self._task: Optional[asyncio.Task] = None

    async def start(self, ingest_cb: Callable[[str, Any], None]):
        await super().start(ingest_cb)
        try:
            self._playwright = await async_playwright().start()
            self._browser = await self._playwright.chromium.launch(headless=True)
            self._context = await self._browser.new_context(
                user_agent="Mozilla/5.0 (Windows NT 10.0; Win64; x64) CYNIC/V2.0"
            )
            self.page = await self._context.new_page()

            # Intercept WebSockets if possible
            self.page.on("websocket", self._on_websocket)

            await self.page.goto(self.url, wait_until="networkidle")

            self._running = True
            logger.info(f"BrowserConduit {self.conduit_id} active on {self.url}")
        except Exception as e:
            logger.error(f"BrowserConduit {self.conduit_id} failed to start: {e}")
            await self.stop()

    def _on_websocket(self, ws):
        logger.debug(f"BrowserConduit {self.conduit_id}: WebSocket detected: {ws.url}")
        ws.on("framereceived", lambda frame: self._on_frame(ws.url, frame))

    def _on_frame(self, url: str, frame: Any):
        if self._ingest_cb:
            # Inject frame into gateway
            asyncio.create_task(
                self._ingest_cb(
                    self.conduit_id,
                    {"type": "websocket_frame", "url": url, "payload": frame},
                )
            )

    async def stop(self):
        self._running = False
        if self.page:
            await self.page.close()
        if self._context:
            await self._context.close()
        if self._browser:
            await self._browser.close()
        if self._playwright:
            await self._playwright.stop()
        logger.info(f"BrowserConduit {self.conduit_id} stopped.")
