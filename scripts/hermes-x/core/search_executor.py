#!/usr/bin/env python3
"""
CYNIC Hermes Search Executor — autonomous search automation.

Reads search_tasks.jsonl and executes X.com searches via headless Playwright.
Respects behavioral patterns: timing, frequency from behavior_log.jsonl.

Architecture:
  search_generation.py → search_tasks.jsonl (conscience-driven keywords)
                      ↓
        search_executor.py (this script) ← headless browser automation
                      ↓
           hermes-proxy captures results (if configured)
                      ↓
              dataset.jsonl grows
                      ↓
         Feedback measures search effectiveness
                      ↓
    search_generation.py updates weights (Phase 2)

Execution model:
  - Launches headless Playwright browser
  - Executes searches via X.com/search endpoint
  - Respects timing: delays between searches, honors behavioral patterns
  - Logs search outcomes with agent identity
  - Produces search_execution_log.jsonl for accountability

Usage:
    python3 search_executor.py --organ-dir ~/.cynic/organs/hermes/x

Environment:
    X_ORGAN_DIR — organ directory
    X_PROXY_ADDR — optional: Hermes proxy address (default: localhost:8888)
    CYNIC_AGENT_EMAIL — organism identity (for audit logging)
"""

__version__ = "0.1.0"

import argparse
import json
import logging
import os
import sys
import time
import urllib.parse
from datetime import datetime
from pathlib import Path
from typing import Optional

try:
    from hub_client import HubClient
    HUB_AVAILABLE = True
except ImportError:
    HUB_AVAILABLE = False

# For reading browser state
import shutil

# Playwright required for search execution
try:
    from playwright.async_api import async_playwright
    PLAYWRIGHT_AVAILABLE = True
except ImportError:
    PLAYWRIGHT_AVAILABLE = False

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(name)s: %(message)s",
    datefmt="%H:%M:%S",
)
from hermes_paths import HERMES_X_DIR, PENDING_DIR, PROCESSED_DIR
from notification_poller import build_interaction_entry, is_already_seen, write_pending

logger = logging.getLogger("search-executor")

DEFAULT_ORGAN_DIR = HERMES_X_DIR
SEARCH_TASKS_FILE = "search_tasks.jsonl"
SEARCH_EXECUTION_LOG = "search_execution_log.jsonl"
BEHAVIOR_LOG = "behavior_log.jsonl"

_TALARIA_LABELS = {"talaria-mention", "talaria-token", "talaria-cashtag", "metadao-ico"}


def _route_to_pending_if_talaria(task: dict, tweet_id: str, author: str, text: str, url: str) -> None:
    """If the search task has a talaria/metadao label, also write to pending/."""
    label = task.get("label", "")
    if label not in _TALARIA_LABELS:
        return
    if is_already_seen(tweet_id, PENDING_DIR, PROCESSED_DIR):
        return
    PENDING_DIR.mkdir(parents=True, exist_ok=True)
    PROCESSED_DIR.mkdir(parents=True, exist_ok=True)
    entry = build_interaction_entry(
        tweet_id=tweet_id,
        notif_type="mention",
        author=author,
        text=text,
        source="search_sweep",
        keywords=[task.get("query", "")],
    )
    entry["url"] = url
    write_pending(entry, PENDING_DIR)


class SearchExecutor:
    """Execute X.com searches via headless Playwright browser."""

    def __init__(self, organ_dir: Path, proxy_addr: Optional[str] = None):
        self.organ_dir = Path(organ_dir)
        self.tasks_file = self.organ_dir / SEARCH_TASKS_FILE
        self.exec_log_file = self.organ_dir / SEARCH_EXECUTION_LOG
        self.behavior_file = self.organ_dir / BEHAVIOR_LOG
        # browser-state.json is written to parent hermes/ directory by launch-browser.sh
        self.browser_state_file = self.organ_dir.parent / "browser-state.json"
        self.proxy_addr = proxy_addr or os.environ.get("X_PROXY_ADDR", "localhost:8888")
        self.search_count = 0
        self.search_errors = 0

    def get_cdp_endpoint(self) -> Optional[str]:
        """Read CDP endpoint from browser-state.json written by launch-browser.sh."""
        if not self.browser_state_file.exists():
            logger.warning("browser-state.json not found at %s", self.browser_state_file)
            return None

        try:
            with open(self.browser_state_file) as f:
                state = json.load(f)
                cdp_url = state.get("cdp_url")
                if cdp_url:
                    logger.info("Found CDP endpoint: %s", cdp_url)
                    return cdp_url
        except Exception as e:
            logger.error("failed to read browser state: %s", e)

        return None

    def load_search_tasks(self) -> list[dict]:
        """Load pending search tasks from search_tasks.jsonl."""
        tasks = []
        if not self.tasks_file.exists():
            logger.warning("search_tasks.jsonl not found")
            return tasks

        try:
            with open(self.tasks_file) as f:
                for line in f:
                    try:
                        tasks.append(json.loads(line))
                    except json.JSONDecodeError:
                        pass
        except IOError as e:
            logger.error("failed to load search tasks: %s", e)
        return tasks

    def extract_behavior_pattern(self) -> dict:
        """Extract timing and frequency patterns from behavior_log.jsonl."""
        pattern = {
            "search_interval_secs": 60,  # Default: 1 search per minute
            "burst_searches": 0,  # If > 0, run N searches in quick succession
            "respect_sleep": True,  # Don't search during sleep hours (19:00-07:00 local)
        }

        if not self.behavior_file.exists():
            logger.info("behavior_log.jsonl not found, using defaults")
            return pattern

        # Parse behavior log to detect patterns
        # For now, use defaults. Phase 2: extract actual behavior patterns
        try:
            # Placeholder: read last 100 events to infer timing
            behaviors = []
            with open(self.behavior_file) as f:
                for line in f:
                    try:
                        behaviors.append(json.loads(line))
                    except json.JSONDecodeError:
                        pass

            if behaviors:
                # Extract inter-event timing
                times = [b.get("timestamp") for b in behaviors[-100:] if b.get("timestamp")]
                if len(times) > 1:
                    # Compute median interval (simplified)
                    logger.info("behavior pattern detected (%d events)", len(times))
                    # Phase 2: actual calculation

        except Exception as e:
            logger.warning("failed to extract behavior pattern: %s", e)

        return pattern

    async def execute_search(self, task: dict, page) -> dict:
        """Execute a single search query via X.com.

        Returns {
            "query": str,
            "domain": str,
            "status": "success" | "error",
            "results_count": int,
            "timestamp": str,
            "url": str (search URL used),
            "agent": str (identity from CYNIC_AGENT_EMAIL),
            "error": str (if status=="error")
        }
        """
        query = task.get("query", "")
        domain = task.get("domain", "unknown")
        timestamp = datetime.now().isoformat()
        agent = os.environ.get("CYNIC_AGENT_EMAIL", "unknown")

        if not query:
            return {
                "query": "",
                "domain": domain,
                "status": "error",
                "error": "empty query",
                "timestamp": timestamp,
                "agent": agent,
            }

        try:
            # Navigate to X search with proper URL encoding
            encoded_query = urllib.parse.quote(query)
            search_url = f"https://x.com/search?q={encoded_query}&f=live"
            logger.info("searching: %s (%s) [agent=%s]", query, domain, agent)
            await page.goto(search_url, wait_until="domcontentloaded", timeout=15000)

            # Wait for dynamic content to load — 5s needed for new tab with authenticated context
            try:
                await page.wait_for_selector('[role="article"]', timeout=8000)
            except Exception:
                pass
            await page.wait_for_timeout(1000)

            # Count visible tweet articles
            results = await page.locator('[role="article"]').count()

            # Route to pending/ for talaria-labeled tasks
            label = task.get("label", "")
            if label in _TALARIA_LABELS and results > 0:
                articles = page.locator('[role="article"]')
                count = await articles.count()
                for idx in range(count):
                    try:
                        article = articles.nth(idx)
                        # Extract tweet URL to derive tweet_id and author
                        link = article.locator('a[href*="/status/"]').first
                        href = await link.get_attribute("href") if await link.count() > 0 else ""
                        tweet_id = ""
                        author = "unknown"
                        if href and "/status/" in href:
                            parts = href.rstrip("/").split("/")
                            tweet_id = parts[-1]
                            # Author is the path segment before /status/
                            status_idx = parts.index("status") if "status" in parts else -1
                            if status_idx > 0:
                                author = parts[status_idx - 1]
                        if not tweet_id:
                            continue
                        # Extract text content
                        text_el = article.locator('[data-testid="tweetText"]').first
                        text = await text_el.inner_text() if await text_el.count() > 0 else query
                        tweet_url = f"https://x.com/{author}/status/{tweet_id}"
                        _route_to_pending_if_talaria(task, tweet_id, author, text, tweet_url)
                    except Exception as _e:
                        logger.debug("failed to extract tweet %d: %s", idx, str(_e)[:80])

            result = {
                "query": query,
                "domain": domain,
                "status": "success",
                "results_count": results,
                "timestamp": timestamp,
                "url": search_url,
                "agent": agent,
            }

            logger.info("✓ search complete: %d results", results)

            return result

        except Exception as e:
            logger.error("✗ search failed: %s: %s", query, str(e)[:100])
            self.search_errors += 1
            return {
                "query": query,
                "domain": domain,
                "status": "error",
                "error": str(e)[:100],
                "timestamp": timestamp,
                "agent": agent,
            }

    def log_execution(self, result: dict) -> None:
        """Append search execution result to log."""
        try:
            with open(self.exec_log_file, "a") as f:
                f.write(json.dumps(result, default=str) + "\n")
        except IOError as e:
            logger.error("failed to log execution: %s", e)

    async def run_cycle(self, max_searches: int = 10) -> None:
        """Execute pending searches (up to max_searches per cycle).

        Connects to hermes-browser (real Chrome with proxy + user profile).
        Falls back to headless Playwright if hermes-browser unavailable.
        """
        if not PLAYWRIGHT_AVAILABLE:
            logger.error("Playwright not installed. Install: pip install playwright")
            logger.info("Alternatively: run searches manually from search_tasks.jsonl")
            return

        tasks = self.load_search_tasks()
        if not tasks:
            logger.info("no pending searches")
            return

        pattern = self.extract_behavior_pattern()
        interval = pattern["search_interval_secs"]

        logger.info("executing up to %d searches (interval: %ds)", max_searches, interval)

        hub_tab = None
        if HUB_AVAILABLE:
            hub = HubClient()
            hub_tab = hub.create_tab("agent:search-executor", "https://x.com/search")
            if hub_tab:
                logger.info("Tab created via Hub: %s", hub_tab.get("tab_id", "?"))

        try:
            async with async_playwright() as p:
                # Try to connect to hermes-browser (real Chrome instance)
                cdp_endpoint = self.get_cdp_endpoint()
                browser = None
                use_real_browser = False

                if cdp_endpoint:
                    try:
                        logger.info("Connecting to hermes-browser via CDP...")
                        # Convert ws://127.0.0.1:40769 to http://127.0.0.1:40769 for connect_over_cdp
                        http_endpoint = cdp_endpoint.replace("ws://", "http://")
                        browser = await p.chromium.connect_over_cdp(http_endpoint)
                        use_real_browser = True
                        logger.info("✓ Connected to hermes-browser (real Chrome, not headless)")
                    except Exception as e:
                        logger.warning("Failed to connect via CDP: %s. Falling back to headless.", str(e)[:100])
                        browser = None

                # Fall back to headless if CDP failed
                if not browser:
                    logger.info("Launching headless Playwright browser (X.com may block this)")
                    browser = await p.chromium.launch(
                        headless=True,
                        args=[
                            "--no-first-run",
                            "--no-default-browser-check",
                        ]
                    )
                    use_real_browser = False

                # Create context and page (reuse existing context if via CDP)
                if use_real_browser:
                    # Use the existing context from hermes-browser
                    contexts = browser.contexts
                    if contexts:
                        context = contexts[0]
                        logger.info("Reusing existing browser context")
                    else:
                        context = await browser.new_context()
                else:
                    context = await browser.new_context()

                page = await context.new_page()

                for i, task in enumerate(tasks[:max_searches]):
                    if i > 0:
                        logger.info("waiting %ds before next search...", interval)
                        time.sleep(interval)

                    result = await self.execute_search(task, page)
                    self.log_execution(result)
                    self.search_count += 1

                await page.close()
                if not use_real_browser:
                    # Only close context if we created it (not via CDP)
                    await context.close()
                await browser.close()

        except Exception as e:
            logger.error("failed to execute searches: %s", str(e)[:150])
            return 1
        finally:
            if hub_tab and HUB_AVAILABLE:
                hub.release_tab(hub_tab["tab_id"])

        logger.info("cycle complete: %d searches, %d errors", self.search_count, self.search_errors)
        return 0

    def run_sync(self, max_searches: int = 10) -> None:
        """Run executor (with fallback if Playwright unavailable)."""
        if not PLAYWRIGHT_AVAILABLE:
            logger.error("Playwright not available.")
            logger.info("Fallback: Manual execution")
            logger.info("Option 1: Copy keywords from search_tasks.jsonl and search manually")
            logger.info("Option 2: Install Playwright: pip install playwright && playwright install")
            return

        import asyncio

        # Log identity
        agent_email = os.environ.get("CYNIC_AGENT_EMAIL", "unknown")
        logger.info("executor identity: %s", agent_email)

        asyncio.run(self.run_cycle(max_searches=max_searches))


def main():
    parser = argparse.ArgumentParser(description="CYNIC Hermes Search Executor — automated X.com searches")
    parser.add_argument(
        "--organ-dir",
        type=Path,
        default=DEFAULT_ORGAN_DIR,
        help="Organ directory",
    )
    parser.add_argument(
        "--proxy-addr",
        type=str,
        help="Hermes proxy address (default: localhost:8888)",
    )
    parser.add_argument(
        "--max-searches",
        type=int,
        default=10,
        help="Maximum searches per cycle",
    )
    args = parser.parse_args()

    organ_dir = args.organ_dir.expanduser()
    if not organ_dir.exists():
        logger.error("organ directory not found: %s", organ_dir)
        return 1

    executor = SearchExecutor(organ_dir, proxy_addr=args.proxy_addr)

    logger.info("Hermes Search Executor v%s starting...", __version__)
    logger.info("Organ directory: %s", organ_dir)
    logger.info("Proxy: %s", executor.proxy_addr)

    executor.run_sync(max_searches=args.max_searches)
    return 0


if __name__ == "__main__":
    sys.exit(main())
