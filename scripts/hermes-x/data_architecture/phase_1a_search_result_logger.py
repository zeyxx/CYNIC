#!/usr/bin/env python3
"""
Phase 1A INFRASTRUCTURE: Log tweet_ids returned from X.com search/timeline operations.

Approach:
  1. Hook into x_proxy (mitmproxy) to capture GraphQL responses for SearchTimeline/HomeTimeline
  2. Extract tweet_id list from each response
  3. Log: {search_query, returned_tweet_ids, timestamp}
  4. Enable click→tweet linkage via temporal proximity

This unblocks killchain: click@t → search@t-1s → returned_tweets → dataset

Success: Once running, behavior_log clicks can be linked to dataset tweets using returned_tweet_ids from nearest search.
"""

import json
import logging
import os
from datetime import datetime, timezone
from pathlib import Path
from typing import List, Optional

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

HERMES_X_DIR = Path.home() / ".cynic/organs/hermes/x"
SEARCH_RESULTS_LOG = HERMES_X_DIR / "search_results.jsonl"

# Expected GraphQL operations that return tweet listings
TIMELINE_OPS = {
    "SearchTimeline",      # Search results
    "HomeTimeline",        # Home feed (authenticated)
    "HomeLatestTimeline",  # Latest home feed
    "UserTweets",          # User profile tweets
    "TweetDetail",         # Tweet detail + replies
    "ListLatestTweetsTimeline",  # List timeline
}


def extract_tweet_ids_from_graphql(data: dict) -> List[str]:
    """Extract tweet_ids from Twitter GraphQL response structure.

    Twitter encodes tweets in:
      data.data.search.search_by_raw_query.search_timeline.timeline.instructions[0].entries
      or similar paths depending on the operation.
    Each entry has tweet-related content with tweet_id in various forms (rest_id, id, tweet_id).
    """
    tweet_ids = []

    try:
        # Navigate the GraphQL response tree looking for tweet entries
        # This is fragile and depends on Twitter's response schema

        def find_tweets(obj, depth=0):
            """Recursively search for tweet-like structures."""
            if depth > 20:  # Prevent infinite recursion (Twitter response nesting is deep)
                return
            if not obj or not isinstance(obj, (dict, list)):
                return

            if isinstance(obj, list):
                for item in obj:
                    find_tweets(item, depth + 1)
            elif isinstance(obj, dict):
                # Look for rest_id (primary field in X API responses)
                if "rest_id" in obj and obj["rest_id"]:
                    tid = str(obj["rest_id"])
                    if tid.isdigit() and len(tid) >= 15:
                        tweet_ids.append(tid)

                # Look for tweet_id fields
                if "tweet_id" in obj and obj["tweet_id"]:
                    tid = str(obj["tweet_id"])
                    if tid.isdigit() and len(tid) >= 15:
                        tweet_ids.append(tid)

                # Also look for id field in typical tweet structures
                if "id" in obj and isinstance(obj.get("id"), str):
                    if obj["id"].isdigit() and len(obj["id"]) >= 15:
                        tweet_ids.append(obj["id"])

                for val in obj.values():
                    find_tweets(val, depth + 1)

        find_tweets(data)

    except Exception as e:
        logger.debug(f"Error extracting tweet_ids: {e}")

    return list(set(tweet_ids))  # Deduplicate


def process_graphql_response(operation: str, variables: dict, data: dict, timestamp: str) -> dict:
    """Process a GraphQL response and extract search/timeline metadata."""
    # Ensure timestamp is ISO format (convert from filename if needed)
    if not timestamp or timestamp.startswith("20260"):
        try:
            from datetime import datetime
            # Try to parse filename format: 20260504_063332
            dt = datetime.strptime(timestamp[:15], "%Y%m%d_%H%M%S")
            timestamp = dt.isoformat() + "Z"
        except (ValueError, TypeError):
            timestamp = datetime.now(timezone.utc).isoformat()

    tweet_ids = extract_tweet_ids_from_graphql(data)

    # Extract query from variables
    query = ""
    if operation == "SearchTimeline":
        query = variables.get("rawQuery", "")
    elif operation in ["HomeTimeline", "HomeLatestTimeline"]:
        query = "home_feed"
    elif operation == "UserTweets":
        query = f"user:{variables.get('userId', 'unknown')}"
    elif operation == "TweetDetail":
        query = f"tweet:{variables.get('focalTweetId', 'unknown')}"

    result = {
        "operation": operation,
        "query": query,
        "returned_tweet_ids": tweet_ids,
        "tweet_count": len(tweet_ids),
        "timestamp": timestamp,
        "variables": variables,  # Store for debugging
    }

    return result


def log_search_result(result: dict):
    """Append enriched search result to log."""
    try:
        with open(SEARCH_RESULTS_LOG, "a") as f:
            f.write(json.dumps(result) + "\n")
        logger.debug(f"Logged {result['tweet_count']} tweets from {result['operation']}")
    except Exception as e:
        logger.error(f"Error logging search result: {e}")


# Mitmproxy addon hook (would be called by x_proxy.py)
def request_hook(flow):
    """Hook for mitmproxy to log GraphQL responses."""
    # This would be integrated into x_proxy.py's mitmproxy addon
    pass


# Standalone loader for existing captures
def process_existing_captures():
    """Process existing capture files to extract search results retroactively."""
    capture_dir = HERMES_X_DIR / "captures"
    if not capture_dir.exists():
        logger.warning(f"No captures directory at {capture_dir}")
        return

    logger.info(f"Processing existing captures from {capture_dir}")

    for capture_file in sorted(capture_dir.glob("*.json")):
        try:
            with open(capture_file) as f:
                capture_data = json.load(f)

            # Capture format: single operation with 'operation', 'timestamp', 'variables', 'response'
            op_name = capture_data.get("operation")
            if not op_name:
                continue

            if op_name in TIMELINE_OPS:
                result = process_graphql_response(
                    op_name,
                    capture_data.get("variables", {}),
                    capture_data.get("response", {}),
                    capture_data.get("timestamp", capture_file.stem)
                )
                log_search_result(result)
        except Exception as e:
            logger.warning(f"Error processing {capture_file}: {e}")


def main():
    logger.info("=" * 60)
    logger.info("PHASE 1A: SEARCH RESULT LOGGING")
    logger.info("=" * 60)

    process_existing_captures()

    logger.info("=" * 60)
    logger.info(f"Search results logged to: {SEARCH_RESULTS_LOG}")
    logger.info("=" * 60)

    # Report
    if SEARCH_RESULTS_LOG.exists():
        with open(SEARCH_RESULTS_LOG) as f:
            count = sum(1 for _ in f)
        logger.info(f"Total search results logged: {count}")


if __name__ == "__main__":
    main()
