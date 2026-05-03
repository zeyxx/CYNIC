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
logger = logging.getLogger("search-executor")

DEFAULT_ORGAN_DIR = Path.home() / ".cynic" / "organs" / "hermes" / "x"
SEARCH_TASKS_FILE = "search_tasks.jsonl"
SEARCH_EXECUTION_LOG = "search_execution_log.jsonl"
BEHAVIOR_LOG = "behavior_log.jsonl"


class SearchExecutor:
    """Execute X.com searches via headless Playwright browser."""

    def __init__(self, organ_dir: Path, proxy_addr: Optional[str] = None):
        self.organ_dir = Path(organ_dir)
        self.tasks_file = self.organ_dir / SEARCH_TASKS_FILE
        self.exec_log_file = self.organ_dir / SEARCH_EXECUTION_LOG
        self.behavior_file = self.organ_dir / BEHAVIOR_LOG
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

            # Wait for dynamic content to load
            await page.wait_for_timeout(2000)

            # Count visible tweet articles
            results = await page.locator('[role="article"]').count()

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

        Launches headless Playwright browser for X.com searches.
        Runs silent (headless=True) to avoid visible windows.
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

        try:
            async with async_playwright() as p:
                # Launch headless browser for X.com searches
                browser = await p.chromium.launch(
                    headless=True,
                    args=[
                        "--no-first-run",
                        "--no-default-browser-check",
                    ]
                )

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
                await context.close()
                await browser.close()

        except Exception as e:
            logger.error("failed to execute searches: %s", str(e)[:150])
            return 1

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
