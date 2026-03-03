"""
CYNIC Web Eye  Advanced perception via Playwright.

Connects the organism to the web world. 
Specifically tuned for https://cannon.pumpparty.com/
Lentilles : Data Engineer (Real-time extraction), SRE (Lifecycle management).
"""

from __future__ import annotations

import asyncio
import logging
import time
from typing import Optional, Any

from playwright.async_api import async_playwright, Browser, BrowserContext, Page

from cynic.kernel.core.event_bus import EventBus, CoreEvent, Event
from cynic.kernel.core.judgment import Cell

logger = logging.getLogger("cynic.senses.web_eye")

class WebEye:
    """
    CYNIC's visual interface to the web.
    Maintains a headless browser session and emits perceptions.
    """

    def __init__(self, bus: EventBus, url: str = "https://cannon.pumpparty.com/"):
        self.bus = bus
        self.url = url
        self._playwright = None
        self._browser: Optional[Browser] = None
        self._context: Optional[BrowserContext] = None
        self.page: Optional[Page] = None
        
        self._running = False
        self._task: Optional[asyncio.Task] = None
        
        # State tracking
        self._last_multiplier = 1.0
        self._game_active = False

    async def start(self):
        """Awaken the eye  launch the browser."""
        if self._running:
            return
            
        try:
            self._playwright = await async_playwright().start()
            self._browser = await self._playwright.chromium.launch(headless=True)
            self._context = await self._browser.new_context(
                user_agent="Mozilla/5.0 (Windows NT 10.0; Win64; x64) CYNIC/V2.0"
            )
            self.page = await self._context.new_page()
            
            logger.info(f"[{self.bus.instance_id}] WebEye: Navigating to {self.url}...")
            await self.page.goto(self.url, wait_until="networkidle")
            
            self._running = True
            self._task = asyncio.create_task(self._watch_loop())
            logger.info(f"[{self.bus.instance_id}] WebEye: Perception loop active.")
            
        except Exception as e:
            logger.error(f"WebEye: Failed to awaken: {e}")
            await self.stop()

    async def stop(self):
        """Detach the eye  close browser."""
        self._running = False
        if self._task:
            self._task.cancel()
            
        if self._context:
            await self._context.close()
        if self._browser:
            await self._browser.close()
        if self._playwright:
            await self._playwright.stop()
            
        logger.info(f"[{self.bus.instance_id}] WebEye: Dormant.")

    async def _watch_loop(self):
        """Real-time observation of the game state."""
        while self._running:
            try:
                # 1. Extract game state from DOM
                # Note: These selectors are speculative and need adjustment after real scrape
                state = await self._perceive_dom()
                
                # 2. Emit only on significant changes (Debouncing)
                if self._should_emit(state):
                    await self._emit_perception(state)
                    
                await asyncio.sleep(0.1) # 10Hz sampling
            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.debug(f"WebEye observation error: {e}")
                await asyncio.sleep(1)

    async def _perceive_dom(self) -> dict[str, Any]:
        """Scrape the current page state."""
        if not self.page:
            return {}
            
        # Example logic for cannon.pumpparty.com
        # We look for multiplier text and game status
        try:
            # Speculative extraction
            multiplier_text = await self.page.inner_text(".multiplier-class") # TODO: Update selector
            multiplier = float(multiplier_text.replace("x", ""))
            
            return {
                "multiplier": multiplier,
                "timestamp": time.time(),
                "game_id": "cannon-session"
            }
        except asyncio.CancelledError:
            raise
        except Exception as e:
            logger.debug("web_eye: error fetching multiplier: %s", e)
            return {}

    def _should_emit(self, state: dict) -> bool:
        """Heuristic to avoid flooding the bus."""
        mult = state.get("multiplier", 1.0)
        # Emit on 0.1 increments or game end
        if abs(mult - self._last_multiplier) >= 0.1 or mult < self._last_multiplier:
            self._last_multiplier = mult
            return True
        return False

    async def _emit_perception(self, data: dict):
        """Translate DOM state to CYNIC Perception."""
        cell = Cell(
            reality="WEB_GAMBLING",
            analysis="PERCEIVE",
            content=data,
            budget_usd=0.0 # Perception is free
        )
        await self.bus.emit(Event.typed(
            CoreEvent.PERCEPTION_RECEIVED,
            cell.model_dump(),
            source="web_eye"
        ))
