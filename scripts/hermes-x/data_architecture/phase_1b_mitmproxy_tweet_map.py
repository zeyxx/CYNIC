#!/usr/bin/env python3
"""
Phase 1B: mitmproxy response → tweet_id mapping for behavior logger lookup.

x_proxy captures GraphQL responses and extracts tweet_ids.
This layer creates a lookup table: operation_type + query → [tweet_ids]

behavior_logger can then query: "I clicked on home timeline at T, what tweets were returned?"

Implementation:
1. Read search_results.jsonl (from x_proxy)
2. Build in-memory index: operation_type + timestamp → [tweet_ids]
3. Expose HTTP endpoint for behavior_logger to query
4. Or write to SQLite for persistent lookup

Simple first: write tweet_id coordinate hints to a JSONL file that behavior_logger can binary-search.
"""

import json
import logging
import sqlite3
from datetime import datetime, timezone, timedelta
from pathlib import Path
from typing import Optional, List

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

HERMES_X_DIR = Path.home() / ".cynic/organs/hermes/x"
SEARCH_RESULTS_LOG = HERMES_X_DIR / "search_results.jsonl"
TWEET_ID_INDEX_DB = HERMES_X_DIR / "tweet_id_index.db"


def build_tweet_id_index():
    """Build SQLite index of operation_type + timestamp → tweet_ids for fast lookup."""
    if not SEARCH_RESULTS_LOG.exists():
        logger.error(f"search_results.jsonl not found at {SEARCH_RESULTS_LOG}")
        return

    # Create database
    conn = sqlite3.connect(TWEET_ID_INDEX_DB)
    cursor = conn.cursor()

    # Drop existing table to rebuild
    cursor.execute("DROP TABLE IF EXISTS tweet_operations")
    cursor.execute("""
        CREATE TABLE tweet_operations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            operation TEXT,
            query TEXT,
            timestamp TEXT,
            tweet_ids TEXT,
            tweet_count INTEGER,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
    """)
    cursor.execute("CREATE INDEX idx_timestamp ON tweet_operations(timestamp)")
    cursor.execute("CREATE INDEX idx_operation ON tweet_operations(operation)")

    # Load search results
    inserted = 0
    try:
        with open(SEARCH_RESULTS_LOG) as f:
            for line in f:
                try:
                    result = json.loads(line)
                    tweet_ids_json = json.dumps(result.get("returned_tweet_ids", []))

                    cursor.execute("""
                        INSERT INTO tweet_operations
                        (operation, query, timestamp, tweet_ids, tweet_count)
                        VALUES (?, ?, ?, ?, ?)
                    """, (
                        result.get("operation"),
                        result.get("query", ""),
                        result.get("timestamp"),
                        tweet_ids_json,
                        result.get("tweet_count", 0),
                    ))
                    inserted += 1
                except json.JSONDecodeError:
                    continue
    except Exception as e:
        logger.error(f"Error loading search results: {e}")
        return

    conn.commit()
    logger.info(f"Indexed {inserted} tweet operations into {TWEET_ID_INDEX_DB}")

    # Show index stats
    cursor.execute("SELECT COUNT(*) FROM tweet_operations")
    total = cursor.fetchone()[0]
    cursor.execute("SELECT COUNT(DISTINCT operation) FROM tweet_operations")
    ops = cursor.fetchone()[0]
    cursor.execute("SELECT SUM(tweet_count) FROM tweet_operations")
    tweets = cursor.fetchone()[0] or 0

    logger.info(f"Index stats: {total} operations, {ops} operation types, {tweets} total tweets")

    conn.close()


def query_tweets_near_time(timestamp_str: str, window_sec: int = 30) -> List[dict]:
    """Query tweets returned within ±window_sec of given timestamp."""
    if not TWEET_ID_INDEX_DB.exists():
        logger.warning("Index database not found. Run build_tweet_id_index() first.")
        return []

    try:
        dt = datetime.fromisoformat(timestamp_str.replace('Z', '+00:00'))
    except ValueError:
        logger.error(f"Invalid timestamp: {timestamp_str}")
        return []

    start_time = (dt - timedelta(seconds=window_sec)).isoformat()
    end_time = (dt + timedelta(seconds=window_sec)).isoformat()

    conn = sqlite3.connect(TWEET_ID_INDEX_DB)
    cursor = conn.cursor()

    cursor.execute("""
        SELECT operation, query, timestamp, tweet_ids, tweet_count
        FROM tweet_operations
        WHERE timestamp BETWEEN ? AND ?
        ORDER BY ABS(julianday(timestamp) - julianday(?)) ASC
    """, (start_time, end_time, timestamp_str))

    results = []
    for row in cursor.fetchall():
        op, query, ts, tweet_ids_json, count = row
        try:
            tweet_ids = json.loads(tweet_ids_json)
        except json.JSONDecodeError:
            tweet_ids = []

        results.append({
            "operation": op,
            "query": query,
            "timestamp": ts,
            "tweet_ids": tweet_ids,
            "tweet_count": count,
        })

    conn.close()
    return results


def lookup_tweet_id(click_timestamp: str, click_url: Optional[str] = None) -> Optional[str]:
    """
    Smart tweet_id lookup for a click event.

    Strategy:
    1. If URL contains /status/{id}, extract directly
    2. Otherwise, find nearest search/timeline operation and try to match
    """

    # If URL is a direct tweet link, extract tweet_id
    if click_url:
        import re
        match = re.search(r'/status/(\d+)', click_url)
        if match:
            return match.group(1)

    # Otherwise, find tweets from nearest operation
    nearby = query_tweets_near_time(click_timestamp, window_sec=30)
    if nearby and nearby[0]["tweet_ids"]:
        # Return most recent/nearest tweet_id
        return nearby[0]["tweet_ids"][0]

    return None


def main():
    logger.info("=" * 60)
    logger.info("PHASE 1B: mitmproxy TWEET_ID INDEX")
    logger.info("=" * 60)

    build_tweet_id_index()

    # Test lookup
    if TWEET_ID_INDEX_DB.exists():
        logger.info("\nSample queries:")
        recent = query_tweets_near_time(datetime.now(timezone.utc).isoformat(), window_sec=300)
        for i, op in enumerate(recent[:3]):
            logger.info(f"  {i+1}. {op['operation']} at {op['timestamp'][:19]}: {op['tweet_count']} tweets")

    logger.info("=" * 60)


if __name__ == "__main__":
    main()
