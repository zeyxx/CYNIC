#!/usr/bin/env python3
"""
Phase 1A TEMPORAL LINKAGE: Link behavior_log clicks to dataset tweets via search result temporal proximity.

Strategy:
  1. Load behavior_log clicks (timestamps, URLs)
  2. Load search_results.jsonl (timestamps, returned_tweet_ids)
  3. For each click, find nearest search within N seconds
  4. If search returned a tweet, link click → tweet via tweet_id
  5. Measure enrichment rate (target: >60% for Phase 1A success)

Killchain: click@t + url@t → search@t-δ → returned_tweets → dataset.jsonl → observation

Success: ≥60% of clicks enriched with tweet_ids (Phase 1A threshold).
"""

import json
import logging
from datetime import datetime, timezone, timedelta
from pathlib import Path
from typing import Optional

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

BEHAVIOR_LOG = Path.home() / ".cynic/organs/hermes/behavior/behavior_log.jsonl"
SEARCH_RESULTS_LOG = Path.home() / ".cynic/organs/hermes/x/search_results.jsonl"
ENRICHMENT_WINDOW_SEC = 30  # Match clicks to searches within ±N seconds


def load_clicks():
    """Load click events from behavior_log."""
    clicks = []
    if not BEHAVIOR_LOG.exists():
        logger.warning(f"behavior_log not found at {BEHAVIOR_LOG}")
        return clicks

    try:
        with open(BEHAVIOR_LOG) as f:
            for line in f:
                try:
                    event = json.loads(line)
                    if event.get("type") == "click":
                        clicks.append(event)
                except json.JSONDecodeError:
                    continue
    except Exception as e:
        logger.error(f"Error loading clicks: {e}")

    logger.info(f"Loaded {len(clicks)} clicks")
    return clicks


def load_search_results():
    """Load search result operations from search_results.jsonl."""
    searches = []
    if not SEARCH_RESULTS_LOG.exists():
        logger.warning(f"search_results.jsonl not found at {SEARCH_RESULTS_LOG}")
        return searches

    try:
        with open(SEARCH_RESULTS_LOG) as f:
            for line in f:
                try:
                    result = json.loads(line)
                    searches.append(result)
                except json.JSONDecodeError:
                    continue
    except Exception as e:
        logger.error(f"Error loading search results: {e}")

    logger.info(f"Loaded {len(searches)} search operations")
    return searches


def parse_iso_timestamp(iso_str: str) -> Optional[datetime]:
    """Parse ISO 8601 timestamp to datetime."""
    try:
        return datetime.fromisoformat(iso_str.replace('Z', '+00:00'))
    except (ValueError, AttributeError):
        return None


def find_nearest_search(click_time: datetime, searches: list) -> Optional[dict]:
    """Find nearest search within ENRICHMENT_WINDOW_SEC of click_time."""
    best_search = None
    best_delta = timedelta(seconds=ENRICHMENT_WINDOW_SEC + 1)

    for search in searches:
        search_time_str = search.get("timestamp")
        if not search_time_str:
            continue

        search_time = parse_iso_timestamp(search_time_str)
        if not search_time:
            continue

        # Calculate time difference (positive = search after click)
        delta = abs(search_time - click_time)

        if delta < best_delta:
            best_delta = delta
            best_search = search

    # Return search only if within window
    if best_delta <= timedelta(seconds=ENRICHMENT_WINDOW_SEC):
        return best_search

    return None


def enrich_clicks(clicks: list, searches: list) -> dict:
    """Enrich clicks with tweet_ids from nearby searches."""
    enriched = 0
    enriched_clicks = []

    for click in clicks:
        click_time_str = click.get("ts")
        if not click_time_str:
            continue

        click_time = parse_iso_timestamp(click_time_str)
        if not click_time:
            continue

        # Find nearest search
        nearest_search = find_nearest_search(click_time, searches)

        if nearest_search:
            click_copy = dict(click)
            click_copy["search_operation"] = nearest_search.get("operation")
            click_copy["search_query"] = nearest_search.get("query")
            click_copy["candidate_tweet_ids"] = nearest_search.get("returned_tweet_ids", [])

            enriched_clicks.append(click_copy)
            enriched += 1

    enrichment_rate = (enriched / len(clicks) * 100) if clicks else 0

    return {
        "total_clicks": len(clicks),
        "enriched_clicks": enriched,
        "enrichment_rate": enrichment_rate,
        "enriched_clicks_sample": enriched_clicks[:5],
    }


def main():
    logger.info("=" * 60)
    logger.info("PHASE 1A: TEMPORAL LINKAGE")
    logger.info("=" * 60)

    clicks = load_clicks()
    searches = load_search_results()

    if not clicks:
        logger.error("No clicks found. Cannot proceed.")
        return

    if not searches:
        logger.error("No search results found. Cannot proceed.")
        logger.info("Waiting for hermes-proxy to capture searches...")
        return

    result = enrich_clicks(clicks, searches)

    logger.info("=" * 60)
    logger.info(f"Total clicks:      {result['total_clicks']}")
    logger.info(f"Enriched:          {result['enriched_clicks']}")
    logger.info(f"Enrichment rate:   {result['enrichment_rate']:.1f}%")
    logger.info("=" * 60)

    if result['enrichment_rate'] >= 60.0:
        logger.info("✓ PHASE 1A: SUCCESS (≥60% enrichment rate)")
    elif result['enrichment_rate'] > 0:
        logger.warning(f"△ PHASE 1A: PARTIAL ({result['enrichment_rate']:.1f}% < 60%)")
        logger.warning("  More searches needed or larger enrichment window")
    else:
        logger.error("✗ PHASE 1A: FAILURE (0% enrichment)")
        logger.error("  Ensure hermes-proxy is capturing searches and behavior_logger is running")

    # Save enrichment report
    report = {
        "phase": "Phase 1A",
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "test": "temporal_linkage",
        "sample_size": result['total_clicks'],
        "enriched": result['enriched_clicks'],
        "enrichment_rate": result['enrichment_rate'],
        "window_sec": ENRICHMENT_WINDOW_SEC,
        "status": "SUCCESS" if result['enrichment_rate'] >= 60.0 else "FAILURE",
        "sample_enriched_clicks": result['enriched_clicks_sample'],
    }

    report_path = Path.home() / ".cynic/organs/hermes/x/phase_1a_enrichment_report.md"

    with open(report_path, "w") as f:
        f.write("# Phase 1A Enrichment Report (Temporal Linkage)\n")
        f.write(f"Generated: {datetime.now(timezone.utc).isoformat()}\n\n")
        f.write("## Summary\n")
        f.write(f"- **Test Date**: {datetime.now().strftime('%Y-%m-%d')}\n")
        f.write(f"- **Success Threshold**: >60% enrichment rate\n")
        f.write(f"- **Enrichment Window**: {ENRICHMENT_WINDOW_SEC}s\n\n")
        f.write("## Results\n\n")
        f.write(f"- **Sample Size**: {result['total_clicks']}\n")
        f.write(f"- **Enriched Clicks**: {result['enriched_clicks']}\n")
        f.write(f"- **Enrichment Rate**: {result['enrichment_rate']:.1f}%\n")
        f.write(f"- **Status**: {report['status']}\n\n")
        f.write("## Recommendation\n\n")

        if result['enrichment_rate'] >= 60.0:
            f.write("**ENRICHMENT SUCCEEDED — Phase 1A validated.**\n\n")
            f.write(f"Temporal proximity linkage achieves {result['enrichment_rate']:.1f}% enrichment.\n\n")
            f.write("Next: Proceed to Phase 1 (full enrichment) with screen_position + engagement deltas.\n")
        elif result['enrichment_rate'] > 0:
            f.write(f"**ENRICHMENT PARTIAL — {result['enrichment_rate']:.1f}% linkage.**\n\n")
            f.write("Potential issues:\n")
            f.write("1. Insufficient search traffic captured (check hermes-proxy logs)\n")
            f.write("2. Enrichment window too small (current: ±10s, consider ±30s)\n")
            f.write("3. Timing skew between behavior_log and search_results.jsonl\n\n")
            f.write("Remediation: Run longer test, increase window size, verify timestamps sync.\n")
        else:
            f.write("**ENRICHMENT FAILED — No clicks linked to tweets.**\n\n")
            f.write("Blockers:\n")
            f.write("1. hermes-proxy not capturing searches (verify service is running)\n")
            f.write("2. behavior_logger not capturing clicks (verify DISPLAY and pynput)\n")
            f.write("3. Timestamp formats diverged (verify both use UTC ISO 8601)\n\n")
            f.write("Action: Verify both data sources are active before retrying.\n")

        f.write("\n## Raw Result (JSON)\n\n")
        f.write("```json\n")
        f.write(json.dumps(report, indent=2) + "\n")
        f.write("```\n\n")
        f.write("---\n")
        f.write("*Phase 1A: Temporal linkage test. If successful, validates click→tweet killchain.*\n")

    logger.info(f"Report saved to: {report_path}")


if __name__ == "__main__":
    main()
