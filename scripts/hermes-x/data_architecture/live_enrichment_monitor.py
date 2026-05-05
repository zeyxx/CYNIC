#!/usr/bin/env python3
"""
Live enrichment monitor: Track click→tweet_id resolution rate in real-time.

Monitors behavior_log.jsonl for new clicks and measures:
- Enrichment rate (clicks with tweet_id / total clicks)
- Enrichment source distribution (CDP vs index)
- Temporal profile (when enrichment happens relative to click)

Runs continuously, reporting every N seconds.
"""

import json
import logging
import time
from datetime import datetime, timezone
from pathlib import Path
from collections import defaultdict

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

BEHAVIOR_LOG = Path.home() / ".cynic/organs/hermes/behavior/behavior_log.jsonl"
REPORT_INTERVAL = 30  # Report every N seconds


class EnrichmentMonitor:
    def __init__(self):
        self.last_line = 0
        self.stats = {
            "total_clicks": 0,
            "enriched_clicks": 0,
            "cdp_enriched": 0,
            "index_enriched": 0,
            "unenriched": 0,
        }

    def read_new_lines(self):
        """Read new click events from behavior_log."""
        if not BEHAVIOR_LOG.exists():
            return []

        new_clicks = []
        try:
            with open(BEHAVIOR_LOG) as f:
                # Skip to last position
                lines = f.readlines()
                for line in lines[self.last_line:]:
                    try:
                        event = json.loads(line)
                        if event.get("type") == "click":
                            new_clicks.append(event)
                    except json.JSONDecodeError:
                        continue

                self.last_line = len(lines)
        except Exception as e:
            logger.error(f"Error reading behavior_log: {e}")

        return new_clicks

    def process_clicks(self, clicks):
        """Analyze click enrichment."""
        for click in clicks:
            self.stats["total_clicks"] += 1

            if "tweet_id" in click:
                self.stats["enriched_clicks"] += 1
                source = click.get("tweet_id_source", "unknown")
                if source == "cdp":
                    self.stats["cdp_enriched"] += 1
                elif source == "index":
                    self.stats["index_enriched"] += 1
            else:
                self.stats["unenriched"] += 1

    def report(self):
        """Print enrichment report."""
        if self.stats["total_clicks"] == 0:
            logger.info("No clicks captured yet")
            return

        total = self.stats["total_clicks"]
        enriched = self.stats["enriched_clicks"]
        rate = (enriched / total * 100) if total > 0 else 0

        logger.info("=" * 60)
        logger.info("LIVE ENRICHMENT REPORT")
        logger.info("=" * 60)
        logger.info(f"Total clicks:       {total}")
        logger.info(f"Enriched:           {enriched} ({rate:.1f}%)")
        logger.info(f"  ├─ CDP source:    {self.stats['cdp_enriched']}")
        logger.info(f"  └─ Index source:  {self.stats['index_enriched']}")
        logger.info(f"Unenriched:         {self.stats['unenriched']}")
        logger.info("=" * 60)

        if rate >= 60:
            logger.info("✓ PHASE 1 THRESHOLD MET (≥60%)")
        elif rate >= 20:
            logger.info("△ APPROACHING PHASE 1 TARGET (20-60%)")
        else:
            logger.info("⚠ BELOW TARGET (<20%)")

    def run(self):
        """Monitor loop."""
        logger.info("Starting live enrichment monitor...")
        logger.info(f"Watching: {BEHAVIOR_LOG}")

        last_report = time.time()

        while True:
            try:
                new_clicks = self.read_new_lines()
                if new_clicks:
                    self.process_clicks(new_clicks)
                    logger.info(f"Processed {len(new_clicks)} new clicks")

                # Report periodically
                now = time.time()
                if now - last_report >= REPORT_INTERVAL:
                    self.report()
                    last_report = now

                time.sleep(1)

            except KeyboardInterrupt:
                logger.info("\nMonitor stopped")
                self.report()
                break
            except Exception as e:
                logger.error(f"Monitor error: {e}")
                time.sleep(5)


def main():
    monitor = EnrichmentMonitor()
    monitor.run()


if __name__ == "__main__":
    main()
