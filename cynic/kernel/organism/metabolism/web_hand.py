"""
CYNIC Web Hand  Somatic actuator for web interaction.

Allows the organism to physically interact with the DOM.
Specifically tuned for https://cannon.pumpparty.com/
Lentilles : Robotics (Precision execution), Backend (Async coordination).
"""

from __future__ import annotations

import logging
import time
from typing import Any, Optional

from playwright.async_api import Page

logger = logging.getLogger("cynic.metabolism.web_hand")


class WebHand:
    """
    The 'Hand' of CYNIC.
    Executes clicks and inputs on a Playwright Page.
    """

    def __init__(self, page: Optional[Page] = None):
        self.page = page
        self._actions_executed = 0

    def set_page(self, page: Page):
        """Bind the hand to a specific browser page."""
        self.page = page

    async def execute(self, action_type: str, **kwargs: Any) -> dict[str, Any]:
        """
        Execute a physical gesture on the web page.
        Supported actions: CLICK, TYPE, WAIT.
        """
        if not self.page:
            return {"success": False, "error": "Hand is not bound to a page (No Body)"}

        t0 = time.perf_counter()
        try:
            if action_type == "CLICK":
                selector = kwargs.get("selector")
                if not selector:
                    return {"success": False, "error": "Missing selector"}

                logger.info(f"WebHand: Clicking {selector}")
                await self.page.click(selector, timeout=5000)

            elif action_type == "BET":
                # Macro-action specifically for Cannon
                # TODO: Update selectors
                await self.page.click(".bet-button")

            elif action_type == "CASHOUT":
                # Macro-action specifically for Cannon
                await self.page.click(".cashout-button")

            else:
                return {"success": False, "error": f"Unknown action: {action_type}"}

            self._actions_executed += 1
            return {
                "success": True,
                "action": action_type,
                "duration_ms": (time.perf_counter() - t0) * 1000,
            }

        except Exception as e:
            logger.error(f"WebHand: Action {action_type} failed: {e}")
            return {"success": False, "error": str(e)}
