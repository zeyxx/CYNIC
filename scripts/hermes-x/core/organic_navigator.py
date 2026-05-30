#!/usr/bin/env python3
"""
Tier 2 INFRASTRUCTURE: Hermes Organic Navigator — UCB1-guided browsing.

Browses X.com organically via hermes-browser (real Chrome + CDP), mimicking
human navigation patterns. Uses UCB1 (Upper Confidence Bound) to balance
exploring new content vs exploiting high-signal domains.

Architecture:
  SKILL.md + curation_yield.json → navigation targets (weighted by domain yield)
      |
      v
  organic_navigator.py (this script) → Chrome via CDP (Hub-attributed)
      |
      v
  hermes-proxy captures all traffic → dataset.jsonl grows organically
      |
      v
  curation → search_generation → search_executor (feedback loop)

The navigator does NOT extract data itself — it drives the browser while
hermes-proxy passively captures all API responses. This separation keeps
the capture layer clean and the navigation layer focused on behavior.

K15 Consumer: hermes-proxy captures results; curation processes them.
Systemd: hermes-navigator.service (every 5min via timer)
Stability: Phase C (UCB1-guided autonomy)

Failure mode: if CDP unavailable, logs warning and exits cleanly.
Budget controls max actions per cycle to prevent detection.
"""

__version__ = "0.1.0"

import argparse
import asyncio
import json
import logging
import math
import os
import random
import sys
import time
from datetime import datetime
from pathlib import Path

from hermes_paths import HERMES_X_DIR, SKILL_MD, NAVIGATION_LOG

try:
    from hub_client import HubClient
    HUB_AVAILABLE = True
except ImportError:
    HUB_AVAILABLE = False

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(name)s: %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("navigator")

# Navigation targets — weighted by domain relevance
DEFAULT_TARGETS = [
    {"url": "https://x.com/home", "domain": "timeline", "weight": 1.0},
    {"url": "https://x.com/search?q=solana&f=live", "domain": "solana", "weight": 0.9},
    {"url": "https://x.com/search?q=crypto+rug+pull&f=live", "domain": "security", "weight": 0.8},
    {"url": "https://x.com/search?q=defi+exploit&f=live", "domain": "security", "weight": 0.7},
    {"url": "https://x.com/search?q=solana+meme+coin&f=live", "domain": "memecoin", "weight": 0.6},
    {"url": "https://x.com/search?q=jupiter+dex&f=live", "domain": "dex", "weight": 0.5},
    {"url": "https://x.com/explore", "domain": "trending", "weight": 0.4},
    {"url": "https://x.com/cynicoracle", "domain": "cynicoracle", "weight": 0.3},
]

# Human-like timing parameters (seconds)
MIN_DWELL = 3.0    # Minimum time on a page
MAX_DWELL = 12.0   # Maximum time on a page
SCROLL_PAUSE = 1.5  # Pause between scrolls
NAV_DELAY_MIN = 2.0  # Min delay between navigations
NAV_DELAY_MAX = 8.0  # Max delay between navigations


class UCB1Selector:
    """UCB1 bandit for balancing explore/exploit across navigation targets."""

    def __init__(self, nav_log_path: Path):
        self.nav_log = nav_log_path
        self.visit_counts: dict[str, int] = {}
        self.reward_sums: dict[str, float] = {}
        self.total_visits = 0
        self._load_history()

    def _load_history(self) -> None:
        """Load navigation history to initialize bandit state."""
        if not self.nav_log.exists():
            return
        try:
            with open(self.nav_log) as f:
                for line in f:
                    try:
                        entry = json.loads(line)
                        domain = entry.get("domain", "unknown")
                        reward = entry.get("reward", 0.0)
                        self.visit_counts[domain] = self.visit_counts.get(domain, 0) + 1
                        self.reward_sums[domain] = self.reward_sums.get(domain, 0.0) + reward
                        self.total_visits += 1
                    except json.JSONDecodeError:
                        pass
        except IOError:
            pass

    def select(self, targets: list[dict]) -> dict:
        """Select next target using UCB1."""
        if self.total_visits == 0:
            # Cold start: pick randomly
            return random.choice(targets)

        best_score = -1.0
        best_target = targets[0]

        for target in targets:
            domain = target.get("domain", "unknown")
            n = self.visit_counts.get(domain, 0)

            if n == 0:
                # Unvisited domain gets priority (infinite UCB)
                return target

            avg_reward = self.reward_sums.get(domain, 0.0) / n
            exploration = math.sqrt(2 * math.log(self.total_visits) / n)
            base_weight = target.get("weight", 1.0)

            ucb_score = (avg_reward + exploration) * base_weight

            if ucb_score > best_score:
                best_score = ucb_score
                best_target = target

        return best_target

    def update(self, domain: str, reward: float) -> None:
        """Update bandit with observed reward for a domain."""
        self.visit_counts[domain] = self.visit_counts.get(domain, 0) + 1
        self.reward_sums[domain] = self.reward_sums.get(domain, 0.0) + reward
        self.total_visits += 1


def load_targets_from_skill(skill_path: Path) -> list[dict]:
    """Extract navigation targets from SKILL.md domain wisdom."""
    targets = list(DEFAULT_TARGETS)

    if not skill_path.exists():
        return targets

    try:
        content = skill_path.read_text(encoding="utf-8")
    except IOError:
        return targets

    # Extract cashtags and terms of interest from SKILL.md
    import re
    cashtags = set(re.findall(r'\$([A-Z]{2,10})', content))
    for tag in list(cashtags)[:5]:  # Max 5 cashtag targets
        targets.append({
            "url": f"https://x.com/search?q=%24{tag}&f=live",
            "domain": f"token_{tag.lower()}",
            "weight": 0.7,
            "source": "skill",
        })

    return targets


def apply_yield_boost(targets: list[dict], organ_dir: Path) -> list[dict]:
    """Boost weights for domains that produce high-signal content."""
    yield_path = organ_dir / "curation_yield.json"
    if not yield_path.exists():
        return targets

    try:
        with open(yield_path) as f:
            yield_data = json.load(f)
    except (json.JSONDecodeError, IOError):
        return targets

    domains = yield_data.get("domains", {})
    for target in targets:
        domain = target.get("domain", "")
        info = domains.get(domain, {})
        pct = info.get("pct_of_total", 0)
        if pct > 20:
            target["weight"] = round(target.get("weight", 1.0) * 1.3, 2)
        elif pct < 5 and pct > 0:
            target["weight"] = round(target.get("weight", 1.0) * 0.7, 2)

    return targets


def get_cdp_endpoint(organ_dir: Path) -> str | None:
    """Read CDP endpoint from browser-state.json."""
    # Check both organ_dir and parent for browser-state.json
    candidates = [organ_dir / "browser-state.json", organ_dir.parent / "browser-state.json"]
    for browser_state in candidates:
        if browser_state.exists():
            try:
                with open(browser_state) as f:
                    state = json.load(f)
                    cdp_url = state.get("cdp_url")
                    if cdp_url:
                        return cdp_url
                    # Fallback: construct from cdp_port
                    cdp_port = state.get("cdp_port")
                    if cdp_port:
                        return f"http://127.0.0.1:{cdp_port}"
            except (json.JSONDecodeError, IOError) as e:
                logger.error("Failed to read %s: %s", browser_state, e)
    logger.warning("No browser-state.json found with CDP info")
    return None


def log_navigation(nav_log: Path, entry: dict) -> None:
    """Append navigation event to log."""
    try:
        with open(nav_log, "a") as f:
            f.write(json.dumps(entry, default=str) + "\n")
    except IOError as e:
        logger.error("Failed to log navigation: %s", e)


async def browse_page(page, url: str, domain: str) -> float:
    """Navigate to a URL, scroll organically, return a reward signal.

    Reward is based on content richness (number of tweet articles visible).
    """
    try:
        await page.goto(url, wait_until="domcontentloaded", timeout=15000)
    except Exception as e:
        logger.warning("Navigation failed for %s: %s", url, str(e)[:80])
        return 0.0

    # Human-like dwell time
    dwell = random.uniform(MIN_DWELL, MAX_DWELL)

    # Scroll a few times to trigger lazy loading (proxy captures API responses)
    scroll_count = random.randint(1, 4)
    for i in range(scroll_count):
        await asyncio.sleep(SCROLL_PAUSE + random.uniform(0, 1.0))
        scroll_amount = random.randint(300, 800)
        await page.evaluate(f"window.scrollBy(0, {scroll_amount})")

    # Wait remaining dwell time
    remaining = max(0, dwell - (scroll_count * SCROLL_PAUSE))
    if remaining > 0:
        await asyncio.sleep(remaining)

    # Compute reward: count tweet articles on page
    try:
        articles = await page.locator('[role="article"]').count()
        # Normalize: 0 articles = 0.0, 10+ articles = 1.0
        reward = min(articles / 10.0, 1.0)
        logger.info("Browsed %s: %d articles, reward=%.2f", domain, articles, reward)
        return reward
    except Exception:
        return 0.3  # Assume moderate reward on measurement failure


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

    # Hub integration for tab attribution
    hub_tab = None
    hub = None
    if HUB_AVAILABLE:
        hub = HubClient()
        hub_tab = hub.create_tab("agent:navigator", "https://x.com/home")
        if hub_tab:
            logger.info("Hub tab created: %s", hub_tab.get("tab_id", "?"))

    try:
        # Import here — optional dependency
        try:
            from playwright.async_api import async_playwright
        except ImportError:
            logger.error("Playwright not available. Install: pip install playwright && playwright install")
            return 1

        async with async_playwright() as p:
            http_endpoint = cdp_url.replace("ws://", "http://")
            try:
                browser = await p.chromium.connect_over_cdp(http_endpoint)
                logger.info("Connected to hermes-browser via CDP")
            except Exception as e:
                logger.error("Failed to connect to CDP: %s", str(e)[:100])
                return 1

            # Reuse existing context (logged-in Chrome session)
            contexts = browser.contexts
            if contexts:
                context = contexts[0]
            else:
                context = await browser.new_context()

            page = await context.new_page()

            actions = 0
            for _ in range(budget):
                # UCB1 selects next target
                target = selector.select(targets)
                url = target["url"]
                domain = target.get("domain", "unknown")

                # Human-like delay between navigations
                if actions > 0:
                    delay = random.uniform(NAV_DELAY_MIN, NAV_DELAY_MAX)
                    await asyncio.sleep(delay)

                reward = await browse_page(page, url, domain)
                selector.update(domain, reward)
                actions += 1

                # Log the navigation event
                log_navigation(NAVIGATION_LOG, {
                    "timestamp": datetime.now().isoformat(),
                    "url": url,
                    "domain": domain,
                    "reward": round(reward, 3),
                    "action_index": actions,
                    "budget": budget,
                })

            await page.close()
            await browser.close()

            logger.info("Navigation cycle complete: %d actions", actions)
            return 0

    except Exception as e:
        logger.error("Navigation cycle failed: %s", str(e)[:150])
        return 1
    finally:
        if hub_tab and hub:
            hub.release_tab(hub_tab["tab_id"])


def main() -> int:
    parser = argparse.ArgumentParser(
        description="CYNIC Hermes Organic Navigator -- UCB1-guided browsing"
    )
    parser.add_argument(
        "--organ-dir",
        type=Path,
        default=HERMES_X_DIR,
        help="Organ directory (default: ~/.cynic/organs/hermes/x)",
    )
    parser.add_argument(
        "--budget",
        type=int,
        default=5,
        help="Max navigation actions per cycle (default: 5)",
    )
    args = parser.parse_args()

    organ_dir = args.organ_dir.expanduser()
    logger.info("Organic Navigator v%s starting...", __version__)
    logger.info("Organ directory: %s", organ_dir)
    logger.info("Budget: %d actions", args.budget)

    if not organ_dir.exists():
        logger.error("Organ directory not found: %s", organ_dir)
        return 1

    # Load navigation targets
    targets = load_targets_from_skill(SKILL_MD)
    targets = apply_yield_boost(targets, organ_dir)
    logger.info("Loaded %d navigation targets", len(targets))

    # Initialize UCB1 selector from history
    selector = UCB1Selector(NAVIGATION_LOG)
    logger.info("UCB1 state: %d total visits across %d domains",
                selector.total_visits, len(selector.visit_counts))

    return asyncio.run(run_navigation_cycle(organ_dir, args.budget, targets, selector))


if __name__ == "__main__":
    sys.exit(main())
