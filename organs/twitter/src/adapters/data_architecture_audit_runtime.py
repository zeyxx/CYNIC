"""Runtime for the expired Hermes X data architecture audit."""

from __future__ import annotations

import json
import logging
from datetime import datetime, timedelta, timezone
from pathlib import Path

from organs.contracts.src.surface_contracts import iter_json_objects
from organs.twitter.src.adapters.hermes_paths import (
    BEHAVIOR_LOG,
    DATASET,
    HERMES_X_DIR,
    OBSERVATIONS_DIR as OBSERVATIONS,
    SEARCH_EXECUTION_LOG,
)


logger = logging.getLogger(__name__)


def load_jsonl(path: Path) -> list[dict]:
    """Load JSONL file into list of dicts."""
    if not path.exists():
        logger.warning("File not found: %s", path)
        return []

    data = []
    try:
        with path.open(encoding="utf-8") as handle:
            data.extend(iter_json_objects(handle))
    except Exception as exc:
        logger.error("Error loading %s: %s", path, exc)

    return data


def test_1_pixel_to_tweet() -> dict:
    """Test whether clicks can match visible tweets."""
    logger.info("TEST 1: behavior_log clicks -> dataset tweets")
    behavior = load_jsonl(BEHAVIOR_LOG)
    dataset = load_jsonl(DATASET)
    if not behavior or not dataset:
        logger.error("Missing behavior_log or dataset.jsonl")
        return {"test": "pixel_to_tweet", "sample_size": 0, "success": 0, "success_rate": 0.0, "status": "BLOCKED_NO_DATA"}

    cutoff = datetime.now(timezone.utc) - timedelta(days=7)
    recent_clicks = []
    for click in behavior:
        if click.get("type") != "click":
            continue
        try:
            ts_str = click.get("ts") or click.get("timestamp")
            if ts_str and datetime.fromisoformat(ts_str.replace("Z", "+00:00")) > cutoff:
                recent_clicks.append(click)
        except (ValueError, AttributeError):
            continue
    recent_clicks = recent_clicks[:100]

    if not recent_clicks:
        logger.error("No recent clicks found in behavior_log")
        return {"test": "pixel_to_tweet", "sample_size": 0, "success": 0, "success_rate": 0.0, "status": "NO_RECENT_CLICKS"}

    success_count = 0
    for click in recent_clicks:
        click_x = click.get("x")
        click_y = click.get("y")
        click_ts = click.get("timestamp")
        if not all([click_x, click_y, click_ts]):
            continue
        try:
            click_time = datetime.fromisoformat(click_ts.replace("Z", "+00:00"))
            for tweet in dataset:
                tweet_x = tweet.get("screen_position", {}).get("x")
                tweet_y = tweet.get("screen_position", {}).get("y")
                tweet_ts = tweet.get("timestamp")
                if not all([tweet_x, tweet_y, tweet_ts]):
                    continue
                tweet_time = datetime.fromisoformat(tweet_ts.replace("Z", "+00:00"))
                spatial_match = abs(click_x - tweet_x) <= 50 and abs(click_y - tweet_y) <= 50
                temporal_match = abs((click_time - tweet_time).total_seconds() * 1000) <= 500
                if spatial_match and temporal_match:
                    success_count += 1
                    break
        except Exception as exc:
            logger.debug("Error matching click: %s", exc)
            continue

    success_rate = success_count / len(recent_clicks) if recent_clicks else 0.0
    return {
        "test": "pixel_to_tweet",
        "sample_size": len(recent_clicks),
        "success": success_count,
        "success_rate": success_rate,
        "status": "SUCCESS" if success_rate > 0.6 else "PARTIAL" if success_rate > 0.5 else "FAILURE",
    }


def test_2_search_to_results() -> dict:
    """Test whether search logs expose returned tweet ids."""
    logger.info("TEST 2: search_execution_log -> returned tweet_ids extraction")
    searches = load_jsonl(SEARCH_EXECUTION_LOG)
    if not searches:
        logger.error("No search_execution_log found")
        return {"test": "search_to_results", "sample_size": 0, "success": 0, "success_rate": 0.0, "status": "NO_DATA"}

    sample = searches[:10]
    success_count = sum(1 for search in sample if search.get("results_displayed") or search.get("returned_tweet_ids"))
    success_rate = success_count / len(sample) if sample else 0.0
    return {
        "test": "search_to_results",
        "sample_size": len(sample),
        "success": success_count,
        "success_rate": success_rate,
        "status": "SUCCESS" if success_rate > 0.6 else "PARTIAL" if success_rate > 0.5 else "FAILURE",
    }


def test_3_observation_to_tweets() -> dict:
    """Test whether observations can trace back to source tweets."""
    logger.info("TEST 3: observations -> verdict source tweets traceability")
    if not OBSERVATIONS.exists():
        logger.error("No observations directory found")
        return {"test": "observation_to_tweets", "sample_size": 0, "success": 0, "success_rate": 0.0, "status": "NO_OBSERVATIONS"}

    obs_files = list(OBSERVATIONS.glob("*.jsonl"))[:5]
    success_count = 0
    for obs_file in obs_files:
        try:
            with obs_file.open(encoding="utf-8") as handle:
                for obs in iter_json_objects(handle):
                    if obs.get("linked_events") or obs.get("source_tweet_id"):
                        success_count += 1
                        break
        except OSError:
            continue

    success_rate = success_count / len(obs_files) if obs_files else 0.0
    return {
        "test": "observation_to_tweets",
        "sample_size": len(obs_files),
        "success": success_count,
        "success_rate": success_rate,
        "status": "SUCCESS" if success_rate > 0.6 else "PARTIAL" if success_rate > 0.5 else "FAILURE",
    }


def build_report(results: list[dict]) -> dict:
    """Build the canonical JSON report payload."""
    overall_success = sum(1 for result in results if result["status"] == "SUCCESS")
    summary_markdown = (
        "# Data Architecture Phase 0 Audit Report\n"
        f"Generated: {datetime.now().isoformat()}\n\n"
        f"**Passed Tests**: {overall_success}/3\n\n"
        "## Raw Results (JSON)\n\n"
        f"{json.dumps(results, indent=2)}\n"
    )
    return {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "overall_success": overall_success,
        "summary_markdown": summary_markdown,
        "results": results,
    }


def main():
    """Run all audit tests and generate report."""
    logger.info("=" * 60)
    logger.info("PHASE 0 DATA ARCHITECTURE AUDIT")
    logger.info("=" * 60)
    results = [test_1_pixel_to_tweet(), test_2_search_to_results(), test_3_observation_to_tweets()]
    report_payload = build_report(results)
    report_path = HERMES_X_DIR / "data_audit_report.json"
    with report_path.open("w", encoding="utf-8") as handle:
        json.dump(report_payload, handle, indent=2)
        handle.write("\n")
    logger.info("Report written to %s", report_path)
    logger.info("AUDIT COMPLETE: %d/3 tests passed", report_payload["overall_success"])
    return results
