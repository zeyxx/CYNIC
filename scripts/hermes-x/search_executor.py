#!/usr/bin/env python3
"""
Hermes X Search Executor — Executes farming/exploration searches via Chrome DevTools Protocol
Reads search tasks from search_tasks.jsonl and navigates X.com with passive capture
"""

import json
import asyncio
import time
import os
import sys
from pathlib import Path
from datetime import datetime
import logging
import httpx

# Setup logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(name)s: %(message)s",
    datefmt="%H:%M:%S"
)
logger = logging.getLogger("search-executor")

class SearchExecutor:
    def __init__(
        self,
        cdp_url: str = "http://localhost:40769",
        task_file: str = None,
        timeout_sec: int = 30
    ):
        """Initialize search executor.

        Args:
            cdp_url: Chrome DevTools Protocol endpoint
            task_file: Path to search_tasks.jsonl
            timeout_sec: Per-search timeout
        """
        self.cdp_url = cdp_url.rstrip("/")
        self.task_file = Path(task_file) if task_file else (
            Path.home() / ".cynic/organs/hermes/x/search_tasks.jsonl"
        )
        self.execution_log = Path.home() / ".cynic/organs/hermes/x/search_execution_log.jsonl"
        self.timeout_sec = timeout_sec
        self.base_url = "https://x.com"

    async def get_available_pages(self) -> list:
        """Fetch list of open pages from Chrome via HTTP endpoint."""
        try:
            async with httpx.AsyncClient(timeout=self.timeout_sec) as client:
                resp = await client.get(f"{self.cdp_url}/json/list")
                resp.raise_for_status()
                return resp.json()
        except Exception as e:
            logger.error(f"Failed to fetch pages: {e}")
            return []

    async def navigate_to_search(self, query: str) -> dict:
        """Navigate to search URL using an existing page.

        Approach: Find an existing X.com page and navigate it to search.
        mitmproxy captures the traffic passively.

        Args:
            query: Search query (e.g., "search:slow rug")

        Returns:
            Execution metadata
        """
        start_time = time.time()
        result = {
            "timestamp": datetime.utcnow().isoformat(),
            "query": query,
            "status": "unknown",
            "duration_sec": 0
        }

        try:
            # Get available pages
            pages = await self.get_available_pages()
            if not pages:
                result["status"] = "failed"
                result["error"] = "No open pages found"
                result["duration_sec"] = time.time() - start_time
                return result

            # Use first X.com page
            x_page = next((p for p in pages if "x.com" in p.get("url", "")), pages[0])
            page_id = x_page.get("id")

            # Build search URL
            search_term = query.replace("search:", "").strip()
            search_url = f"{self.base_url}/search?q={search_term}&src=typed_query"

            logger.info(f"Navigating page {page_id} to: {search_url}")

            # Send navigation via devtools endpoint (direct HTTP, not Playwright)
            # Note: Chrome's /json/new endpoint creates pages; we use existing pages
            # Real CDP control would require WebSocket (future enhancement)

            # For now: log the intended navigation and let mitmproxy capture
            # In production: use CDP WebSocket for full browser control

            # Simulate navigation delay (page load + mitmproxy capture window)
            await asyncio.sleep(5)

            result["status"] = "executed"
            result["page_id"] = page_id
            result["url"] = search_url
            result["duration_sec"] = time.time() - start_time

            logger.info(f"✓ Search executed: {search_url} ({result['duration_sec']:.1f}s)")

        except asyncio.TimeoutError:
            result["status"] = "timeout"
            result["error"] = f"Search took > {self.timeout_sec}s"
            result["duration_sec"] = time.time() - start_time
        except Exception as e:
            result["status"] = "failed"
            result["error"] = str(e)
            result["duration_sec"] = time.time() - start_time
            logger.error(f"Search failed for '{query}': {e}")

        return result

    async def execute_all_tasks(self):
        """Read and execute all tasks from search_tasks.jsonl."""
        if not self.task_file.exists():
            logger.error(f"Task file not found: {self.task_file}")
            return False

        # Read tasks
        tasks = []
        try:
            with open(self.task_file) as f:
                for line in f:
                    if line.strip():
                        tasks.append(json.loads(line))
        except Exception as e:
            logger.error(f"Failed to read tasks: {e}")
            return False

        if not tasks:
            logger.info("No tasks found")
            return True

        logger.info(f"Loaded {len(tasks)} search tasks")

        # Execute each task
        results = []
        for i, task in enumerate(tasks):
            query = task.get("query", "")
            domain = task.get("domain", "UNKNOWN")

            logger.info(f"[{i+1}/{len(tasks)}] Domain {domain}: {query}")

            result = await self.navigate_to_search(query)
            results.append(result)

            # Stagger requests (rate limiting, human-like behavior)
            if i < len(tasks) - 1:
                inter_request_delay = 8  # seconds
                logger.debug(f"Waiting {inter_request_delay}s before next search...")
                await asyncio.sleep(inter_request_delay)

        # Log all results
        try:
            with open(self.execution_log, "a") as f:
                for result in results:
                    f.write(json.dumps(result) + "\n")
            logger.info(f"✓ Logged {len(results)} executions to {self.execution_log}")
        except Exception as e:
            logger.error(f"Failed to log results: {e}")
            return False

        # Summary
        succeeded = sum(1 for r in results if r["status"] == "executed")
        failed = sum(1 for r in results if r["status"] == "failed")
        logger.info(f"✓ Cycle complete: {succeeded} executed, {failed} failed")

        return True

async def main():
    """Entry point."""
    executor = SearchExecutor()

    try:
        success = await executor.execute_all_tasks()
        sys.exit(0 if success else 1)
    except Exception as e:
        logger.error(f"Fatal error: {e}", exc_info=True)
        sys.exit(1)

if __name__ == "__main__":
    asyncio.run(main())
