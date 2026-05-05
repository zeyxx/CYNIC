#!/usr/bin/env python3
"""
Tier 1 EXPERIMENTAL: Data audit for organic agent learning.

Research question: Can we correlate user clicks to tweets they clicked on?
Success condition: >60% of behavior_log clicks link to dataset tweets
Timeline: 3-4 days local testing
Owned by: @T
Status: ACTIVE (2026-05-05)

Will promote to Tier 2 if: Correlation works + agent feedback loop feasible
Death date: 2026-05-22 (unless promoted)

FALSIFIABLE:
- If <50% success: Audit fails, pixel→tweet infeasible, pivot needed
- If 50-70% success: Partial success, need enrichment (browser logger + positions)
- If >70% success: Success, proceed to Phase 1
"""

import json
import os
from pathlib import Path
from datetime import datetime, timedelta, timezone
from typing import Any
import logging

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

HERMES_DIR = Path.home() / ".cynic/organs/hermes"
HERMES_X_DIR = HERMES_DIR / "x"
BEHAVIOR_LOG = HERMES_DIR / "behavior" / "behavior_log.jsonl"
DATASET = HERMES_X_DIR / "dataset.jsonl"
FARMING_LOG = HERMES_X_DIR / "farming_log.jsonl"
SEARCH_EXECUTION_LOG = HERMES_X_DIR / "search_execution_log.jsonl"
OBSERVATIONS = HERMES_X_DIR / "observations"


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


def test_1_pixel_to_tweet() -> dict:
    """Test 1: Can we match clicks to visible tweets?"""
    logger.info("TEST 1: behavior_log clicks → dataset tweets (spatial + temporal match)")

    behavior = load_jsonl(BEHAVIOR_LOG)
    dataset = load_jsonl(DATASET)

    if not behavior or not dataset:
        logger.error("Missing behavior_log or dataset.jsonl")
        return {
            "test": "pixel_to_tweet",
            "sample_size": 0,
            "success": 0,
            "success_rate": 0.0,
            "status": "BLOCKED_NO_DATA"
        }

    # Sample 100 clicks from last 7 days
    cutoff = datetime.now(timezone.utc) - timedelta(days=7)
    recent_clicks = []
    for c in behavior:
        if c.get('type') == 'click':
            try:
                ts_str = c.get('ts') or c.get('timestamp')
                if ts_str:
                    ts = datetime.fromisoformat(ts_str.replace('Z', '+00:00'))
                    if ts > cutoff:
                        recent_clicks.append(c)
            except (ValueError, AttributeError):
                continue
    recent_clicks = recent_clicks[:100]

    if not recent_clicks:
        logger.error("No recent clicks found in behavior_log")
        return {
            "test": "pixel_to_tweet",
            "sample_size": 0,
            "success": 0,
            "success_rate": 0.0,
            "status": "NO_RECENT_CLICKS"
        }

    success_count = 0

    for click in recent_clicks:
        click_x = click.get('x')
        click_y = click.get('y')
        click_ts = click.get('timestamp')

        if not all([click_x, click_y, click_ts]):
            continue

        # Find tweets visible at (x±50, y±50) within t±500ms
        matching_tweets = []
        try:
            click_time = datetime.fromisoformat(click_ts.replace('Z', '+00:00'))

            for tweet in dataset:
                # Check if tweet has position info (would come from enrichment)
                tweet_x = tweet.get('screen_position', {}).get('x')
                tweet_y = tweet.get('screen_position', {}).get('y')
                tweet_ts = tweet.get('timestamp')

                if not all([tweet_x, tweet_y, tweet_ts]):
                    # Try fallback: if no position, can't match
                    continue

                tweet_time = datetime.fromisoformat(tweet_ts.replace('Z', '+00:00'))

                # Spatial match: x±50, y±50
                if abs(click_x - tweet_x) <= 50 and abs(click_y - tweet_y) <= 50:
                    # Temporal match: t±500ms
                    time_diff = abs((click_time - tweet_time).total_seconds() * 1000)
                    if time_diff <= 500:
                        matching_tweets.append(tweet)

            if matching_tweets:
                success_count += 1

        except Exception as e:
            logger.debug(f"Error matching click: {e}")
            continue

    success_rate = success_count / len(recent_clicks) if recent_clicks else 0.0

    return {
        "test": "pixel_to_tweet",
        "sample_size": len(recent_clicks),
        "success": success_count,
        "success_rate": success_rate,
        "status": "SUCCESS" if success_rate > 0.6 else "PARTIAL" if success_rate > 0.5 else "FAILURE"
    }


def test_2_search_to_results() -> dict:
    """Test 2: Can we identify tweets returned from searches?"""
    logger.info("TEST 2: search_execution_log → returned tweet_ids extraction")

    searches = load_jsonl(SEARCH_EXECUTION_LOG)

    if not searches:
        logger.error("No search_execution_log found")
        return {
            "test": "search_to_results",
            "sample_size": 0,
            "success": 0,
            "success_rate": 0.0,
            "status": "NO_DATA"
        }

    # Sample 10 searches
    sample = searches[:10]

    success_count = 0
    for search in sample:
        # Check if results_displayed or returned_tweet_ids exists
        if search.get('results_displayed') or search.get('returned_tweet_ids'):
            success_count += 1

    success_rate = success_count / len(sample) if sample else 0.0

    return {
        "test": "search_to_results",
        "sample_size": len(sample),
        "success": success_count,
        "success_rate": success_rate,
        "status": "SUCCESS" if success_rate > 0.6 else "PARTIAL" if success_rate > 0.5 else "FAILURE"
    }


def test_3_observation_to_tweets() -> dict:
    """Test 3: Can we trace observations back to source tweets?"""
    logger.info("TEST 3: observations → verdict source tweets traceability")

    if not OBSERVATIONS.exists():
        logger.error("No observations directory found")
        return {
            "test": "observation_to_tweets",
            "sample_size": 0,
            "success": 0,
            "success_rate": 0.0,
            "status": "NO_OBSERVATIONS"
        }

    # Sample 5 observation files
    obs_files = list(OBSERVATIONS.glob("*.jsonl"))[:5]

    success_count = 0
    for obs_file in obs_files:
        try:
            with open(obs_file) as f:
                for line in f:
                    obs = json.loads(line)
                    # Check if observation has linked_events or source_tweet_id
                    if obs.get('linked_events') or obs.get('source_tweet_id'):
                        success_count += 1
                        break  # Count file as success if ANY observation has traceability
        except:
            continue

    success_rate = success_count / len(obs_files) if obs_files else 0.0

    return {
        "test": "observation_to_tweets",
        "sample_size": len(obs_files),
        "success": success_count,
        "success_rate": success_rate,
        "status": "SUCCESS" if success_rate > 0.6 else "PARTIAL" if success_rate > 0.5 else "FAILURE"
    }


def main():
    """Run all audit tests and generate report."""
    logger.info("=" * 60)
    logger.info("PHASE 0 DATA ARCHITECTURE AUDIT")
    logger.info("=" * 60)

    results = [
        test_1_pixel_to_tweet(),
        test_2_search_to_results(),
        test_3_observation_to_tweets(),
    ]

    # Generate report
    report = f"""# Data Architecture Phase 0 Audit Report
Generated: {datetime.now().isoformat()}

## Summary
- **Audit Date**: {datetime.now().strftime('%Y-%m-%d')}
- **Falsifiable Threshold**: >60% success = PROCEED to Phase 1
- **Death Date if Failed**: 2026-05-22 (30 days from 2026-05-05)

## Test Results

"""

    overall_success = 0
    for result in results:
        report += f"""### Test: {result['test']}
- **Sample Size**: {result['sample_size']}
- **Successful Matches**: {result['success']}
- **Success Rate**: {result['success_rate']:.1%}
- **Status**: {result['status']}

"""
        if result['status'] == 'SUCCESS':
            overall_success += 1

    report += f"""## Overall Assessment

**Passed Tests**: {overall_success}/3

### Recommendation:

"""

    if overall_success >= 2:  # At least 2/3 tests passing
        avg_rate = sum(r['success_rate'] for r in results) / len(results)
        if avg_rate > 0.6:
            report += """**PROMOTE TO PHASE 1 — Data correlation works!**

The audit succeeded. Agent can learn signal yield per domain.

Next Steps:
1. Promote to Tier 2 INFRASTRUCTURE
2. Wire hermes-data-enrichment.timer (daily)
3. Start Phase 1: Enrichment (behavior_log, dataset, search_execution)
"""
        else:
            report += """**PARTIAL SUCCESS — Need enrichment layer**

Correlation partially works but needs improvement:
1. Browser logger must capture content + domain on clicks
2. Dataset needs enrichment: tweet_id, screen_position, visible_duration
3. Farming log needs: returned_tweet_ids, user_clicked_subset

Rerun audit after enrichment is complete.
"""
    else:
        report += """**AUDIT FAILED — Correlation doesn't work**

<50% success rate. Pixel→tweet matching is not feasible.

**Pivot needed**: Instead of spatial correlation, use:
- Temporal + author + content similarity matching
- Or: Upgrade browser logger to capture tweet_id directly

Recommend: Do NOT proceed to Phase 1 until pivot succeeds.
"""

    report += f"""

## Raw Results (JSON)

{json.dumps(results, indent=2)}

---
*Phase 0 audit: Tier 1 EXPERIMENTAL. Death date: 2026-05-22 if not promoted.*
"""

    # Write report
    report_path = HERMES_X_DIR / "data_audit_report.md"
    with open(report_path, 'w') as f:
        f.write(report)

    logger.info(f"Report written to {report_path}")
    logger.info("=" * 60)
    logger.info(f"AUDIT COMPLETE: {overall_success}/3 tests passed")
    logger.info("=" * 60)

    return results


if __name__ == '__main__':
    main()
