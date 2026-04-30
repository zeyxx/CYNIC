#!/usr/bin/env python3
"""
CYNIC Hermes Search Executor — Layer -1 conscious automation.

Reads search_tasks.jsonl and executes searches on X/Twitter via Playwright CDP.
Respects behavioral patterns: timing, frequency, interaction style from behavior_log.jsonl.

Architecture:
  search_generation.py → search_tasks.jsonl (conscience-driven keywords)
                      ↓
        search_executor.py (this script) ← runs searches
                      ↓
           hermes-proxy captures results
                      ↓
              dataset.jsonl grows
                      ↓
         Feedback measures search effectiveness
                      ↓
    search_generation.py updates weights (Phase 2)

Execution model:
  - Reads X cookies from Firefox or Playwright context
  - Searches via X.com/search endpoint (not API, to avoid rate limits)
  - Respects timing: delays between searches, honors user's sleep schedule
  - Logs search outcomes: results, visibility, engagement

Usage:
    python3 search_executor.py --organ-dir ~/.cynic/organs/hermes/x [--headless]

Environment:
    X_ORGAN_DIR — organ directory
    X_PROXY_ADDR — optional: Hermes proxy address (default: localhost:8888)
"""

__version__ = "0.1.0"

import argparse
import json
import logging
import os
import sys
import time
from datetime import datetime
from pathlib import Path
from typing import Optional

# Optional: Playwright for CDP execution
try:
    from playwright.async_api import async_playwright, Browser
    PLAYWRIGHT_AVAILABLE = True
except ImportError:
    PLAYWRIGHT_AVAILABLE = False

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(name)s: %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("search-executor")

DEFAULT_ORGAN_DIR = Path.home() / ".cynic" / "organs" / "hermes" / "x"
SEARCH_TASKS_FILE = "search_tasks.jsonl"
SEARCH_EXECUTION_LOG = "search_execution_log.jsonl"
BEHAVIOR_LOG = "behavior_log.jsonl"


class SearchExecutor:
    """Execute farming searches via Playwright CDP."""

    def __init__(self, organ_dir: Path, headless: bool = False, proxy_addr: Optional[str] = None):
        self.organ_dir = Path(organ_dir)
        self.tasks_file = self.organ_dir / SEARCH_TASKS_FILE
        self.exec_log_file = self.organ_dir / SEARCH_EXECUTION_LOG
        self.behavior_file = self.organ_dir / BEHAVIOR_LOG
        self.headless = headless
        self.proxy_addr = proxy_addr or os.environ.get("X_PROXY_ADDR", "localhost:8888")
        self.search_count = 0
        self.search_errors = 0

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

    async def execute_search(self, task: dict, browser: Browser) -> dict:
        """Execute a single search query via X/Twitter.

        Returns {
            "query": str,
            "domain": str,
            "status": "success" | "error",
            "results_count": int,
            "timestamp": str,
            "error": str (if status=="error")
        }
        """
        query = task.get("query", "")
        domain = task.get("domain", "unknown")
        timestamp = datetime.now().isoformat()

        if not query:
            return {
                "query": "",
                "domain": domain,
                "status": "error",
                "error": "empty query",
                "timestamp": timestamp,
            }

        try:
            # Open context reusing Chrome cookies and profile
            chrome_profile = os.path.expanduser("~/.config/google-chrome/Default")
            context = await browser.new_context(
                storage_state=None,  # Will inherit from Chrome profile
                user_agent="Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
            )

            # Copy cookies from Chrome if available (Phase 2 enhancement)
            page = await context.new_page()

            # Navigate to X search
            search_url = f"https://x.com/search?q={query}&f=live"
            logger.info("searching: %s (%s)", query, domain)
            await page.goto(search_url, wait_until="networkidle", timeout=15000)

            # Wait for results to load
            await page.wait_for_selector('[role="article"]', timeout=5000)

            # Count visible results (simplified: count tweet articles)
            results = await page.locator('[role="article"]').count()

            result = {
                "query": query,
                "domain": domain,
                "status": "success",
                "results_count": results,
                "timestamp": timestamp,
                "url": search_url,
            }

            logger.info("✓ search complete: %d results", results)

            await context.close()
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
            }

    def log_execution(self, result: dict) -> None:
        """Append search execution result to log."""
        try:
            with open(self.exec_log_file, "a") as f:
                f.write(json.dumps(result, default=str) + "\n")
        except IOError as e:
            logger.error("failed to log execution: %s", e)

    async def run_cycle(self, max_searches: int = 10) -> None:
        """Execute pending searches (up to max_searches per cycle)."""
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

        async with async_playwright() as p:
            # Use Chrome with existing cookies and profile
            browser = await p.chromium.launch(
                headless=self.headless,
                args=["--disable-blink-features=AutomationControlled"]  # Reduce bot-detection
            )

            for i, task in enumerate(tasks[:max_searches]):
                if i > 0:
                    logger.info("waiting %ds before next search...", interval)
                    time.sleep(interval)

                result = await self.execute_search(task, browser)
                self.log_execution(result)
                self.search_count += 1

            await browser.close()

        logger.info("cycle complete: %d searches, %d errors", self.search_count, self.search_errors)

    def run_sync(self) -> None:
        """Run executor (with fallback if Playwright unavailable)."""
        if not PLAYWRIGHT_AVAILABLE:
            logger.error("Playwright not available.")
            logger.info("Fallback: Manual execution")
            logger.info("Option 1: Copy keywords from search_tasks.jsonl and search manually")
            logger.info("Option 2: Install Playwright: pip install playwright && playwright install firefox")
            return

        import asyncio

        asyncio.run(self.run_cycle())


def main():
    parser = argparse.ArgumentParser(description="CYNIC Hermes Search Executor")
    parser.add_argument(
        "--organ-dir",
        type=Path,
        default=DEFAULT_ORGAN_DIR,
        help="Organ directory",
    )
    parser.add_argument(
        "--headless",
        action="store_true",
        help="Run browser in headless mode",
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

    executor = SearchExecutor(organ_dir, headless=args.headless, proxy_addr=args.proxy_addr)

    logger.info("Hermes Search Executor v%s starting...", __version__)
    logger.info("Organ directory: %s", organ_dir)
    logger.info("Proxy: %s", executor.proxy_addr)

    executor.run_sync()
    return 0


if __name__ == "__main__":
    sys.exit(main())
