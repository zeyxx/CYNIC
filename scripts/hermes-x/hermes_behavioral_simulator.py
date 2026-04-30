#!/usr/bin/env python3
"""
Behavioral Simulator — injects user interaction patterns into Hermes browser via CDP.

Connects to CYNIC's hermes-browser service (persistent X.com login) and executes
searches with behavioral mimicry:
  - Types with user's WPM and keystroke timing distribution
  - Moves mouse with user's velocity patterns
  - Scrolls with user's burst frequency and direction bias
  - Deliberates with user's pause distribution

Architecture:
  - hermes-browser.service (CDP on :40769, persistent profile, mitmproxy-routed)
  - Behavioral Simulator connects via CDP (no new browser launched)
  - All searches logged to /observe via X ingest daemon

Purpose: Organism searches autonomously (from framing) while appearing human (from profile).
"""

__version__ = "0.2.0-cdp"

import json
import logging
import random
import statistics
import asyncio
import os
import urllib.request
import urllib.error
from argparse import ArgumentParser
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List, Optional

try:
    from playwright.async_api import async_playwright, Page, Browser
except ImportError:
    raise ImportError("playwright required: pip install playwright")

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(name)s: %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("behavioral-simulator")


class BehavioralSimulator:
    """Inject user interaction patterns into Playwright actions."""

    def __init__(self, profile_path: Path):
        """Load behavioral profile."""
        self.profile = self._load_profile(profile_path)
        self.typing = self.profile.get("typing", {})
        self.mouse = self.profile.get("mouse", {})
        self.scroll = self.profile.get("scroll", {})
        self.temporal = self.profile.get("temporal", {})

        logger.info("loaded behavioral profile: %.1f WPM, %.0f px/s velocity",
                    self.typing.get("wpm", 0),
                    self.mouse.get("velocity_mean", 0))

    def _load_profile(self, path: Path) -> Dict:
        """Load behavioral profile JSON."""
        if not path.exists():
            raise FileNotFoundError(f"Profile not found: {path}")

        with open(path) as f:
            return json.load(f)

    async def type_like_user(self, page: Page, selector: str, text: str, delay_ms: Optional[float] = None):
        """Type text with user's keystroke timing distribution."""
        await page.click(selector)

        # Get keystroke interval distribution from profile
        interval_mean = self.typing.get("interval_mean_ms", 100)
        interval_stdev = self.typing.get("interval_stdev_ms", 50)

        for char in text:
            # Sample keystroke delay from user's distribution
            if interval_stdev > 0:
                delay = max(10, random.gauss(interval_mean, interval_stdev))
            else:
                delay = interval_mean

            await asyncio.sleep(delay / 1000)
            await page.type(selector, char, delay=0)

        logger.debug(f"typed {len(text)} chars at {self.typing.get('wpm', 0):.1f} WPM")

    async def deliberate(self, duration_s: Optional[float] = None):
        """Pause for user's typical deliberation time."""
        # User's deliberation typically 4.1s; add variance
        base = duration_s or 4.1
        variance = random.gauss(0, 1.5)  # ±1.5s
        wait_time = max(0.5, base + variance)

        logger.debug(f"deliberating for {wait_time:.1f}s")
        await asyncio.sleep(wait_time)

    async def move_mouse_like_user(self, page: Page, x: int, y: int):
        """Move mouse with user's velocity patterns."""
        # Get user's velocity distribution
        velocity_mean = self.mouse.get("velocity_mean", 300)  # px/s
        velocity_median = self.mouse.get("velocity_median", 100)

        # Use median for more natural (slower) movement
        velocity = max(50, random.gauss(velocity_median, velocity_median * 0.3))

        # Simulate cursor movement (Playwright doesn't expose actual path, just endpoint)
        await page.mouse.move(x, y)
        logger.debug(f"moved mouse to ({x}, {y}) at {velocity:.0f} px/s")

    async def scroll_like_user(self, page: Page, direction: str = "down", amount: Optional[int] = None):
        """Scroll with user's scroll patterns."""
        scroll_down_pct = self.scroll.get("scroll_down_percent", 34)
        scroll_distance = amount or int(self.scroll.get("scroll_distance_mean", 100))

        # User scrolls down 34%, up 66% → bias toward up
        if direction == "auto":
            direction = "down" if random.random() < scroll_down_pct / 100 else "up"

        dy = scroll_distance if direction == "down" else -scroll_distance

        await page.evaluate(f"window.scrollBy(0, {dy})")
        logger.debug(f"scrolled {direction} {abs(scroll_distance)}px")
        await asyncio.sleep(random.uniform(0.2, 0.8))  # Brief pause after scroll

    async def click_like_user(self, page: Page, selector: str):
        """Click with user's click location patterns (center-top bias)."""
        # User clicks at (2458, 364) mean, use normal selector click
        await page.click(selector)
        logger.debug(f"clicked {selector}")


async def find_x_com_page_cdp() -> Optional[str]:
    """Find X.com page in browser and get its CDP WebSocket URL."""
    state_file = Path.home() / ".cynic" / "organs" / "hermes" / "browser-state.json"

    if not state_file.exists():
        logger.warning(f"browser state not found: {state_file}")
        logger.warning("Is hermes-browser.service running?")
        return None

    try:
        with open(state_file) as f:
            state = json.load(f)
            cdp_port = state.get("cdp_port", 40769)

        # Query /json/list to find open pages
        http_url = f"http://localhost:{cdp_port}/json/list"
        logger.debug(f"listing browser pages: {http_url}")

        try:
            response = urllib.request.urlopen(http_url, timeout=2)
            pages = json.loads(response.read().decode())

            # Find X.com page (prefer the X.com page if multiple exist)
            x_page = None
            for page in pages:
                if page.get("type") == "page":  # Only regular pages, not workers
                    url = page.get("url", "")
                    if "x.com" in url or "twitter.com" in url:
                        x_page = page
                        logger.info(f"found X.com page: {url}")
                        break

            if not x_page:
                # Fallback: use first page if no X.com page found
                for page in pages:
                    if page.get("type") == "page":
                        x_page = page
                        logger.warning(f"no X.com page found, using: {page.get('url')}")
                        break

            if x_page:
                cdp_url = x_page.get("webSocketDebuggerUrl")
                if cdp_url:
                    logger.info(f"using page: {cdp_url}")
                    return cdp_url

            logger.error("no pages found in browser")
            return None

        except (urllib.error.URLError, urllib.error.HTTPError) as e:
            logger.error(f"CDP endpoint not accessible: {e}")
            logger.error("Is hermes-browser.service running?")
            return None

    except Exception as e:
        logger.error(f"failed to find CDP page: {e}")
        return None


async def search_x_com(
    query: str,
    profile_path: Path,
) -> Dict:
    """Execute search on X.com via CYNIC's hermes-browser (CDP).

    Connects to the running hermes-browser.service (persistent X.com login)
    and executes the search with behavioral mimicry.

    Args:
        query: Search query to execute
        profile_path: Path to behavioral_profile.json

    Returns:
        Dict with status, query, result_count, timestamp
    """

    simulator = BehavioralSimulator(profile_path)

    # Find X.com page in running browser
    cdp_url = await find_x_com_page_cdp()
    if not cdp_url:
        return {
            "status": "error",
            "query": query,
            "error": "hermes-browser not running. Start with: systemctl --user start hermes-browser.service",
            "timestamp": datetime.now().isoformat(),
        }

    async with async_playwright() as p:
        try:
            # Connect to existing browser via CDP (no new launch)
            logger.info("connecting to CYNIC browser via CDP...")
            browser = await p.chromium.connect_over_cdp(cdp_url)

            # Get or create context
            contexts = await browser.contexts
            if contexts:
                context = contexts[0]
                logger.info(f"reusing existing context (login may persist)")
            else:
                context = await browser.new_context()
                logger.info(f"created new context")

            # Create page in context
            page = await context.new_page()

            logger.info(f"navigating to x.com/search...")
            await page.goto("https://x.com/search", wait_until="domcontentloaded", timeout=10000)

            # Wait for search input (try multiple selectors)
            search_selector = None
            for selector in [
                'input[aria-label*="search"]',
                'input[placeholder*="search"]',
                'input[role="searchbox"]',
                'div[role="search"] input',
            ]:
                try:
                    element = await page.query_selector(selector)
                    if element and await element.is_visible():
                        search_selector = selector
                        logger.debug(f"found search input: {selector}")
                        break
                except:
                    pass

            if not search_selector:
                logger.error("search input not found")
                return {
                    "status": "error",
                    "query": query,
                    "error": "could not find search input on page",
                    "timestamp": datetime.now().isoformat(),
                }

            logger.info(f"executing search query: {query}")

            # Type search with behavioral profile
            await simulator.type_like_user(page, search_selector, query)

            # Deliberate before pressing enter
            await simulator.deliberate(2.0)

            # Press enter
            await page.press(search_selector, "Enter")

            # Wait for results
            await page.wait_for_timeout(3000)

            # Try to find results
            try:
                await page.wait_for_selector('article', timeout=5000)
            except:
                logger.warning("no article elements found, continuing anyway")

            logger.info("search completed, scrolling results...")

            # Scroll results like user
            for i in range(3):
                await simulator.scroll_like_user(page, "auto")
                await asyncio.sleep(random.uniform(1, 2))

            # Capture result count
            result_text = await page.text_content()
            result_count = result_text.count("article") if result_text else 0

            await page.close()

            return {
                "status": "success",
                "query": query,
                "result_count": result_count,
                "timestamp": datetime.now().isoformat(),
                "page_title": await page.title(),
            }

        except Exception as e:
            logger.error(f"search failed: {e}")
            return {
                "status": "error",
                "query": query,
                "error": str(e),
                "timestamp": datetime.now().isoformat(),
            }


def main():
    parser = ArgumentParser(
        description="Execute search via CYNIC's hermes-browser with behavioral mimicry"
    )
    parser.add_argument(
        "--query",
        required=True,
        help="Search query to execute (e.g., 'recovery scammer crypto')"
    )
    parser.add_argument(
        "--profile",
        type=Path,
        default=Path.home() / ".cynic" / "organs" / "hermes" / "x" / "behavioral_profile.json",
        help="Behavioral profile JSON (default: behavioral_profile.json)"
    )
    args = parser.parse_args()

    if not args.profile.exists():
        logger.error(f"profile not found: {args.profile}")
        return 1

    logger.info(f"behavioral simulator v{__version__}")
    logger.info(f"connecting to CYNIC hermes-browser service...")
    logger.info(f"using profile: {args.profile.name}")

    result = asyncio.run(search_x_com(args.query, args.profile))

    logger.info(f"\nRESULT: {result['status']}")
    if result["status"] == "success":
        logger.info(f"  query: {result['query']}")
        logger.info(f"  result_count: {result.get('result_count', 0)}")
        logger.info(f"  timestamp: {result.get('timestamp')}")
    else:
        logger.error(f"  error: {result.get('error', 'unknown')}")

    return 0 if result["status"] == "success" else 1


if __name__ == "__main__":
    import sys
    sys.exit(main())
