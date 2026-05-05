#!/usr/bin/env python3
"""
Phase 0.5 EXPERIMENTAL: Enrich click events with tweet_ids via CDP DOM queries.

Research question: Can we reliably extract tweet_id from click coordinates?
Success condition: >70% of recent X.com clicks matched to tweet_ids
Timeline: 1-2 days
Owned by: @T
Status: ACTIVE (2026-05-05)

Approach:
  1. Read recent clicks from behavior_log.jsonl
  2. For each click on X.com, query Chrome CDP for DOM element at (x, y)
  3. Extract tweet_id from element data attributes or ancestor tweet container
  4. Append enriched click to enriched_clicks.jsonl
  5. Compare enriched_clicks to dataset.jsonl to validate coverage

If >70% success: Use this for killchain reconstruction (click→tweet→observation)
If <70% success: Fall back to Phase 1 (capture tweet_id directly in browser logger)
"""

import json
import logging
import os
import requests
from collections import defaultdict
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Optional

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

HERMES_DIR = Path.home() / ".cynic/organs/hermes"
HERMES_X_DIR = HERMES_DIR / "x"
BEHAVIOR_LOG = HERMES_DIR / "behavior" / "behavior_log.jsonl"
DATASET = HERMES_X_DIR / "dataset.jsonl"
ENRICHED_CLICKS = HERMES_X_DIR / "enriched_clicks.jsonl"

# Chrome CDP endpoint (set by hermes-browser.service)
CDP_PORT = int(os.environ.get("HERMES_CDP_PORT", 40769))
CDP_URL = f"http://127.0.0.1:{CDP_PORT}"


def load_jsonl(path: Path) -> list[dict]:
    """Load JSONL file into list of dicts."""
    if not path.exists():
        logger.warning(f"File not found: {path}")
        return []

    data = []
    try:
        with open(path) as f:
            for line in f:
                if line.strip():
                    data.append(json.loads(line))
    except Exception as e:
        logger.error(f"Error loading {path}: {e}")

    return data


def load_dataset_index() -> dict[str, str]:
    """Load dataset and build tweet_id → raw record index."""
    dataset = load_jsonl(DATASET)
    index = {}
    for record in dataset:
        tweet_id = record.get('tweet_id')
        if tweet_id:
            index[tweet_id] = record
    logger.info(f"Dataset index built: {len(index)} tweets")
    return index


def query_dom_at_position(x: int, y: int) -> Optional[dict]:
    """Query Chrome CDP to get element at (x, y) and extract tweet_id.

    Returns: {"tweet_id": "...", "element_type": "tweet/text/other", "confidence": 0.0-1.0}
    """
    try:
        # Get list of open tabs
        resp = requests.get(f"{CDP_URL}/json", timeout=2)
        if resp.status_code != 200:
            return None

        tabs = resp.json()
        # Find the active page tab
        target = None
        for tab in tabs:
            if tab.get("type") == "page" and "x.com" in tab.get("url", "").lower():
                target = tab
                break

        if not target:
            return None

        ws_url = target.get("webSocketDebuggerUrl")
        if not ws_url:
            return None

        # Use Chrome devtools protocol to query DOM at position
        # This is a simplified approach: look for data-testid or aria-label containing tweet ID
        # In reality, we'd need WebSocket CDP to properly interact
        # For now, return None to indicate we need full CDP integration
        logger.debug(f"CDP target found but WebSocket integration needed for position {x},{y}")
        return None

    except Exception as e:
        logger.debug(f"CDP query failed: {e}")
        return None


def enrich_clicks_from_behavior_log():
    """Main enrichment loop: read recent clicks, attempt tweet_id extraction."""
    logger.info("PHASE 0.5: Click-to-Tweet Enrichment")

    behavior = load_jsonl(BEHAVIOR_LOG)
    dataset_index = load_dataset_index()

    if not behavior:
        logger.error("Missing behavior_log.jsonl")
        return {
            "test": "enrich_clicks",
            "sample_size": 0,
            "enriched": 0,
            "enrichment_rate": 0.0,
            "status": "NO_DATA"
        }

    # Sample recent clicks from last 7 days
    cutoff = datetime.now(timezone.utc) - timedelta(days=7)
    recent_clicks = []

    for record in behavior:
        if record.get('type') == 'click':
            try:
                ts_str = record.get('ts')
                if ts_str:
                    ts = datetime.fromisoformat(ts_str.replace('Z', '+00:00'))
                    if ts > cutoff and 'x.com' in record.get('url', '').lower():
                        recent_clicks.append(record)
            except (ValueError, AttributeError):
                continue

    recent_clicks = recent_clicks[:100]  # Sample 100

    if not recent_clicks:
        logger.error("No recent X.com clicks found in behavior_log")
        return {
            "test": "enrich_clicks",
            "sample_size": 0,
            "enriched": 0,
            "enrichment_rate": 0.0,
            "status": "NO_RECENT_CLICKS"
        }

    logger.info(f"Sampled {len(recent_clicks)} recent X.com clicks")

    enriched_count = 0
    enriched_records = []

    for click in recent_clicks:
        x = click.get('x')
        y = click.get('y')
        url = click.get('url', '')

        # Strategy 1: Extract tweet_id from URL if present
        tweet_id = None
        if '/status/' in url:
            # Format: https://x.com/username/status/1234567890
            parts = url.split('/status/')
            if len(parts) > 1:
                tweet_id = parts[1].split('?')[0].split('#')[0].strip()

        # Strategy 2: If not in URL, try CDP DOM query (requires WebSocket integration)
        if not tweet_id and x and y:
            dom_info = query_dom_at_position(x, y)
            if dom_info:
                tweet_id = dom_info.get('tweet_id')

        # Validate tweet_id against dataset
        if tweet_id and tweet_id in dataset_index:
            enriched_record = dict(click)
            enriched_record['tweet_id'] = tweet_id
            enriched_record['enrichment_source'] = 'url_extract' if '/status/' in url else 'dom_query'
            enriched_record['enriched_at'] = datetime.now(timezone.utc).isoformat()
            enriched_records.append(enriched_record)
            enriched_count += 1

    # Write enriched clicks
    with open(ENRICHED_CLICKS, 'w') as f:
        for record in enriched_records:
            f.write(json.dumps(record) + '\n')

    enrichment_rate = enriched_count / len(recent_clicks) if recent_clicks else 0.0

    result = {
        "test": "enrich_clicks",
        "sample_size": len(recent_clicks),
        "enriched": enriched_count,
        "enrichment_rate": enrichment_rate,
        "status": "SUCCESS" if enrichment_rate > 0.7 else "PARTIAL" if enrichment_rate > 0.5 else "FAILURE"
    }

    logger.info(f"Enrichment complete: {enriched_count}/{len(recent_clicks)} = {enrichment_rate:.1%}")

    return result


def main():
    logger.info("=" * 60)
    logger.info("PHASE 0.5 CLICK ENRICHMENT")
    logger.info("=" * 60)

    result = enrich_clicks_from_behavior_log()

    report = f"""# Click Enrichment Report (Phase 0.5)
Generated: {datetime.now().isoformat()}

## Summary
- **Test Date**: {datetime.now().strftime('%Y-%m-%d')}
- **Success Threshold**: >70% enrichment rate
- **Next Step if Failed**: Full CDP DOM integration or Phase 1 direct capture

## Results

- **Sample Size**: {result['sample_size']}
- **Enriched Clicks**: {result['enriched']}
- **Enrichment Rate**: {result['enrichment_rate']:.1%}
- **Status**: {result['status']}

## Recommendation

"""

    if result['enrichment_rate'] > 0.7:
        report += """**SUCCESS — URL extraction works!**

>70% of X.com clicks have tweet_ids in their URLs. Killchain is viable:
  click(x,y,ts) → dataset(tweet_id) → observation(verdict)

Next: Implement full click→observation reconstruction in Phase 1.
"""
    elif result['enrichment_rate'] > 0.5:
        report += """**PARTIAL SUCCESS — URL extraction partial**

50-70% of clicks have tweet_ids extractable from URLs. Need improvement:
  1. Implement WebSocket CDP DOM query for clicks without URL tweet_ids
  2. Or modify browser_logger.py to capture tweet_id directly on click

Either approach unblocks Phase 1.
"""
    else:
        report += """**ENRICHMENT FAILED — No tweet_ids found**

<50% enrichment rate. URL parsing isn't sufficient.

Pivot required: Modify behavior_logger.py to capture tweet_id directly when clicking
on tweets (requires DOM query at click time, not post-hoc).

This is Phase 1 optimization: enhance browser logger with DOM element context.
"""

    report += f"""

## Raw Result (JSON)

{json.dumps(result, indent=2)}

---
*Phase 0.5: Click enrichment optimization. If successful, unblocks killchain without Phase 1.*
"""

    report_path = HERMES_X_DIR / "click_enrichment_report.md"
    with open(report_path, 'w') as f:
        f.write(report)

    logger.info(f"Report written to {report_path}")
    logger.info("=" * 60)

    return result


if __name__ == '__main__':
    main()
