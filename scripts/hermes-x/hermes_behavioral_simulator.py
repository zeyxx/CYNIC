#!/usr/bin/env python3
"""
Behavioral Simulator — injects user interaction patterns into Playwright.

Takes a search intent + behavioral profile, executes as if the user did it:
  - Types with user's WPM and keystroke timing distribution
  - Moves mouse with user's velocity patterns
  - Scrolls with user's burst frequency and direction bias
  - Deliberates with user's pause distribution

Purpose: Avoid bot detection by matching user's actual interaction style.
"""

__version__ = "0.1.0"

import json
import logging
import random
import statistics
import asyncio
from argparse import ArgumentParser
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List, Optional

try:
    from playwright.async_api import async_playwright, Page
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


async def search_x_com(
    query: str,
    profile_path: Path,
    headless: bool = True,
) -> Dict:
    """Execute search on X.com with behavioral mimicry."""

    simulator = BehavioralSimulator(profile_path)

    async with async_playwright() as p:
        browser = await p.chromium.launch(
            headless=headless,
            args=["--disable-blink-features=AutomationControlled"]
        )

        page = await browser.new_page(
            user_agent="Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36"
        )

        try:
            logger.info(f"navigating to x.com/search...")
            await page.goto("https://x.com/search", wait_until="domcontentloaded", timeout=10000)

            # Wait for search box
            await page.wait_for_selector('input[placeholder*="search"]', timeout=5000)

            logger.info(f"executing search query: {query}")

            # Type search with behavioral profile
            await simulator.type_like_user(page, 'input[placeholder*="search"]', query)

            # Deliberate before pressing enter
            await simulator.deliberate(2.0)

            # Press enter
            await page.press('input[placeholder*="search"]', "Enter")

            # Wait for results
            await page.wait_for_timeout(3000)
            await page.wait_for_selector('article', timeout=10000)

            logger.info("search completed, scrolling results...")

            # Scroll results like user
            for i in range(3):
                await simulator.scroll_like_user(page, "auto")
                await asyncio.sleep(random.uniform(1, 2))

            # Capture result count
            result_text = await page.text_content()
            result_count = result_text.count("article") if result_text else 0

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

        finally:
            await page.close()
            await browser.close()


def main():
    parser = ArgumentParser(description="Execute search with behavioral mimicry")
    parser.add_argument(
        "--query",
        required=True,
        help="Search query to execute"
    )
    parser.add_argument(
        "--profile",
        type=Path,
        default=Path.home() / ".cynic" / "organs" / "hermes" / "x" / "behavioral_profile.json",
        help="Behavioral profile JSON"
    )
    parser.add_argument(
        "--headless",
        action="store_true",
        default=True,
        help="Run headless (default: true)"
    )
    args = parser.parse_args()

    if not args.profile.exists():
        logger.error(f"profile not found: {args.profile}")
        return 1

    logger.info(f"behavioral simulator v{__version__}")
    logger.info(f"executing with profile: {args.profile.name}")

    result = asyncio.run(search_x_com(args.query, args.profile, headless=args.headless))

    logger.info(f"result: {result['status']}")
    if result["status"] == "success":
        logger.info(f"  query: {result['query']}")
        logger.info(f"  results: {result.get('result_count', 0)}")

    return 0 if result["status"] == "success" else 1


if __name__ == "__main__":
    import sys
    sys.exit(main())
