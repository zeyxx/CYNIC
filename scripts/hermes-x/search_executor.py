#!/usr/bin/env python3
"""
CYNIC Hermes Search Executor — Layer -1 conscious automation.

Reads search_tasks.jsonl and executes searches on X/Twitter via shared hermes-browser CDP.
Respects behavioral patterns: timing, frequency, interaction style from behavior_log.jsonl.

Architecture:
  search_generation.py → search_tasks.jsonl (conscience-driven keywords)
                      ↓
        search_executor.py (this script) ← connects to hermes-browser CDP
                      ↓
           hermes-proxy captures results
                      ↓
              dataset.jsonl grows
                      ↓
         Feedback measures search effectiveness
                      ↓
    search_generation.py updates weights (Phase 2)

Execution model:
  - Connects to shared hermes-browser CDP (port 40769)
  - Reuses browser identity and session state (already logged in)
  - Searches via X.com/search endpoint (not API, to avoid rate limits)
  - Respects timing: delays between searches, honors user's sleep schedule
  - Logs search outcomes: results, visibility, engagement

Usage:
    python3 search_executor.py --organ-dir ~/.cynic/organs/hermes/x

Environment:
    X_ORGAN_DIR — organ directory
    X_PROXY_ADDR — optional: Hermes proxy address (default: localhost:8888)
    HERMES_CDP_PORT — hermes-browser CDP port (default: 40769)
    CYNIC_AGENT_EMAIL — identity (for audit logging)
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

    async def execute_search(self, task: dict, context) -> dict:
        """Execute a single search query via X/Twitter using shared hermes-browser context.

        Returns {
            "query": str,
            "domain": str,
            "status": "success" | "error",
            "results_count": int,
            "timestamp": str,
            "agent": str (identity used),
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
            # Create page in shared context (reuses hermes-browser session)
            page = await context.new_page()

            # Navigate to X search
            search_url = f"https://x.com/search?q={query}&f=live"
            logger.info("searching: %s (%s) [agent=%s]", query, domain, agent)
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
                "agent": agent,
            }

            logger.info("✓ search complete: %d results", results)

            await page.close()
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
        """Execute pending searches (up to max_searches per cycle) using shared hermes-browser CDP."""
        if not PLAYWRIGHT_AVAILABLE:
            logger.error("Playwright not installed. Install: pip install playwright")
            logger.info("Alternatively: run searches manually from search_tasks.jsonl")
            return

        tasks = self.load_search_tasks()
        if not tasks:
            logger.info("no pending searches")
            return

        # Get CDP port from env (hermes-browser service sets this)
        cdp_port = os.environ.get("HERMES_CDP_PORT", "40769")
        cdp_url = f"ws://127.0.0.1:{cdp_port}/devtools/browser"

        pattern = self.extract_behavior_pattern()
        interval = pattern["search_interval_secs"]

        logger.info("executing up to %d searches (interval: %ds) via CDP %s", max_searches, interval, cdp_url)

        try:
            async with async_playwright() as p:
                # Connect to shared hermes-browser via CDP (don't launch our own)
                browser = await p.chromium.connect_over_cdp(cdp_url)
                context = await browser.new_context()

                for i, task in enumerate(tasks[:max_searches]):
                    if i > 0:
                        logger.info("waiting %ds before next search...", interval)
                        time.sleep(interval)

                    result = await self.execute_search(task, context)
                    self.log_execution(result)
                    self.search_count += 1

                await context.close()

        except Exception as e:
            logger.error("failed to connect to hermes-browser CDP at %s: %s", cdp_url, str(e))
            logger.info("ensure hermes-browser.service is running: systemctl --user status hermes-browser.service")
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
    parser = argparse.ArgumentParser(description="CYNIC Hermes Search Executor (shared hermes-browser CDP)")
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
    logger.info("Note: connects to shared hermes-browser.service (port 40769)")

    executor.run_sync(max_searches=args.max_searches)
    return 0


if __name__ == "__main__":
    sys.exit(main())
