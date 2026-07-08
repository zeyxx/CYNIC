"""Browser runtime for Hermes organic navigation."""

from __future__ import annotations

import asyncio
import logging
import random
from datetime import datetime
from pathlib import Path

from organs.twitter.src.adapters.hermes_paths import NAVIGATION_LOG
from organs.twitter.src.adapters.organic_navigation_io import get_cdp_endpoint, log_navigation
from organs.twitter.src.core.organic_navigation import UCB1Selector

try:
    from organs.twitter.src.adapters.hub_client import HubClient

    HUB_AVAILABLE = True
except ImportError:
    HUB_AVAILABLE = False


logger = logging.getLogger("navigator")
MIN_DWELL = 3.0
MAX_DWELL = 12.0
SCROLL_PAUSE = 1.5
NAV_DELAY_MIN = 2.0
NAV_DELAY_MAX = 8.0


async def browse_page(page, url: str, domain: str) -> float:
    """Navigate to a URL, scroll organically, return a reward signal."""
    try:
        await page.goto(url, wait_until="domcontentloaded", timeout=15000)
    except Exception as exc:
        logger.warning("Navigation failed for %s: %s", url, str(exc)[:80])
        return 0.0

    dwell = random.uniform(MIN_DWELL, MAX_DWELL)
    scroll_count = random.randint(1, 4)
    for _ in range(scroll_count):
        await asyncio.sleep(SCROLL_PAUSE + random.uniform(0, 1.0))
        await page.evaluate(f"window.scrollBy(0, {random.randint(300, 800)})")

    remaining = max(0, dwell - (scroll_count * SCROLL_PAUSE))
    if remaining > 0:
        await asyncio.sleep(remaining)

    try:
        articles = await page.locator('[role="article"]').count()
        reward = min(articles / 10.0, 1.0)
        logger.info("Browsed %s: %d articles, reward=%.2f", domain, articles, reward)
        return reward
    except Exception:
        return 0.3


async def run_navigation_cycle(
    organ_dir: Path,
    budget: int,
    targets: list[dict],
    selector: UCB1Selector,
) -> int:
    """Run one navigation cycle: select targets, browse, update bandit."""
    cdp_url = get_cdp_endpoint(organ_dir)
    if not cdp_url:
        logger.error("No CDP endpoint available. Is hermes-browser running?")
        return 1

    hub_tab = None
    hub = None
    if HUB_AVAILABLE:
        hub = HubClient()
        hub_tab = hub.create_tab("agent:navigator", "https://x.com/home")
        if hub_tab:
            logger.info("Hub tab created: %s", hub_tab.get("tab_id", "?"))

    try:
        try:
            from playwright.async_api import async_playwright
        except ImportError:
            logger.error("Playwright not available. Install: pip install playwright && playwright install")
            return 1

        async with async_playwright() as playwright:
            try:
                browser = await playwright.chromium.connect_over_cdp(cdp_url.replace("ws://", "http://"))
                logger.info("Connected to hermes-browser via CDP")
            except Exception as exc:
                logger.error("Failed to connect to CDP: %s", str(exc)[:100])
                return 1

            context = browser.contexts[0] if browser.contexts else await browser.new_context()
            page = await context.new_page()

            actions = 0
            for _ in range(budget):
                target = selector.select(targets)
                url = target["url"]
                domain = target.get("domain", "unknown")

                if actions > 0:
                    await asyncio.sleep(random.uniform(NAV_DELAY_MIN, NAV_DELAY_MAX))

                reward = await browse_page(page, url, domain)
                selector.update(domain, reward)
                actions += 1
                log_navigation(
                    NAVIGATION_LOG,
                    {
                        "timestamp": datetime.now().isoformat(),
                        "url": url,
                        "domain": domain,
                        "reward": round(reward, 3),
                        "action_index": actions,
                        "budget": budget,
                    },
                )

            await page.close()
            await browser.close()
            logger.info("Navigation cycle complete: %d actions", actions)
            return 0

    except Exception as exc:
        logger.error("Navigation cycle failed: %s", str(exc)[:150])
        return 1
    finally:
        if hub_tab and hub:
            hub.release_tab(hub_tab["tab_id"])
