#!/usr/bin/env python3
"""
Attribution system: measure organism-guided vs spontaneous searches.

K15 closure: link organism suggestions → your searches → dataset growth.

Reads:
  - search_tasks.jsonl (organism suggestions)
  - behavior_log.jsonl (your actual keyboard input, searches)
  - dataset.jsonl (captured tweets, timestamps)

Outputs:
  - attribution_log.jsonl (source: organism_guided | spontaneous)
  - attribution_summary.json (metrics: % organism-guided, results per source)
"""

__version__ = "0.1.0"

import json
import re
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List, Tuple
from collections import defaultdict

ORGAN_DIR = Path.home() / ".cynic" / "organs" / "hermes" / "x"
BEHAVIOR_DIR = Path.home() / ".cynic" / "organs" / "hermes" / "behavior"

SEARCH_TASKS_PATH = ORGAN_DIR / "search_tasks.jsonl"
BEHAVIOR_LOG_PATH = BEHAVIOR_DIR / "behavior_log.jsonl"
DATASET_PATH = ORGAN_DIR / "dataset.jsonl"
ATTRIBUTION_LOG = ORGAN_DIR / "attribution_log.jsonl"
ATTRIBUTION_SUMMARY = ORGAN_DIR / "attribution_summary.json"


def load_organism_keywords() -> set:
    """Extract keywords from search_tasks.jsonl (organism suggestions)."""
    keywords = set()
    if not SEARCH_TASKS_PATH.exists():
        return keywords

    try:
        with open(SEARCH_TASKS_PATH) as f:
            for line in f:
                try:
                    task = json.loads(line)
                    keyword = task.get("keyword", "").lower().strip()
                    if keyword:
                        keywords.add(keyword)
                except json.JSONDecodeError:
                    pass
    except IOError:
        pass

    return keywords


def extract_searches_from_behavior(window_minutes: int = 5) -> List[Dict]:
    """Extract X searches from behavior_log (keyboard input in search box).

    Look for text input events in X.com/search context.
    Cluster by timestamp (within window_minutes = one search session).
    """
    searches = []
    if not BEHAVIOR_LOG_PATH.exists():
        return searches

    try:
        with open(BEHAVIOR_LOG_PATH) as f:
            for line in f:
                try:
                    event = json.loads(line)
                    # Simple heuristic: text input events
                    if event.get("type") == "text_input":
                        text = event.get("text", "").strip()
                        ts = event.get("ts")
                        if text and ts:
                            searches.append({
                                "text": text,
                                "timestamp": ts,
                                "type": "keyboard_input"
                            })
                except json.JSONDecodeError:
                    pass
    except IOError:
        pass

    return searches


def load_dataset_with_timestamps() -> List[Dict]:
    """Load dataset.jsonl, extract tweets with timestamps."""
    tweets = []
    if not DATASET_PATH.exists():
        return tweets

    try:
        with open(DATASET_PATH) as f:
            for line in f:
                try:
                    tweet = json.loads(line)
                    tweets.append(tweet)
                except json.JSONDecodeError:
                    pass
    except IOError:
        pass

    return tweets


def match_searches_to_results(
    organism_keywords: set,
    user_searches: List[Dict],
    tweets: List[Dict],
    time_window_minutes: int = 30
) -> Tuple[List[Dict], Dict]:
    """Match user searches to dataset results.

    Heuristic:
    - If search text contains organism keyword → organism_guided
    - Tweets within time_window_minutes after search → attributed to that search
    """
    attribution = []
    metrics = {
        "total_tweets": len(tweets),
        "organism_guided_tweets": 0,
        "spontaneous_tweets": 0,
        "unattributed_tweets": 0,
        "searches_matched": 0,
        "organism_keywords": list(organism_keywords)[:10],  # First 10 for logging
    }

    # For each search, find tweets within time window
    for search in user_searches:
        search_text = search.get("text", "").lower()
        search_ts_str = search.get("timestamp", "")

        try:
            search_ts = datetime.fromisoformat(search_ts_str.replace("Z", "+00:00"))
        except (ValueError, TypeError):
            continue

        # Determine: organism-guided or spontaneous?
        is_organism_guided = any(
            keyword in search_text for keyword in organism_keywords
        )

        if is_organism_guided:
            metrics["searches_matched"] += 1

        # Find tweets within time window
        for tweet in tweets:
            tweet_ts_str = tweet.get("created_at") or tweet.get("timestamp")
            if not tweet_ts_str:
                continue

            try:
                tweet_ts = datetime.fromisoformat(tweet_ts_str.replace("Z", "+00:00"))
            except (ValueError, TypeError):
                continue

            time_diff = (tweet_ts - search_ts).total_seconds() / 60
            if 0 <= time_diff <= time_window_minutes:
                # Tweet is within window of search
                if is_organism_guided:
                    source = "organism_guided"
                    metrics["organism_guided_tweets"] += 1
                else:
                    source = "spontaneous"
                    metrics["spontaneous_tweets"] += 1

                attribution.append({
                    "tweet_id": tweet.get("id"),
                    "tweet_text": tweet.get("text", "")[:100],
                    "search_text": search_text,
                    "source": source,
                    "search_timestamp": search_ts_str,
                    "tweet_timestamp": tweet_ts_str,
                    "time_delta_minutes": round(time_diff, 2),
                    "matched_keywords": [
                        kw for kw in organism_keywords if kw in search_text
                    ],
                })

    # Unattributed tweets
    metrics["unattributed_tweets"] = (
        metrics["total_tweets"] -
        metrics["organism_guided_tweets"] -
        metrics["spontaneous_tweets"]
    )

    return attribution, metrics


def write_attribution_log(attributions: List[Dict]) -> bool:
    """Write attribution results to attribution_log.jsonl."""
    try:
        with open(ATTRIBUTION_LOG, "w") as f:
            for attr in attributions:
                f.write(json.dumps(attr) + "\n")
        print(f"✓ Wrote {len(attributions)} attributions to {ATTRIBUTION_LOG}")
        return True
    except IOError as e:
        print(f"ERROR: Failed to write attribution log: {e}")
        return False


def write_attribution_summary(metrics: Dict) -> bool:
    """Write summary metrics."""
    try:
        metrics["timestamp"] = datetime.now().isoformat()
        metrics["version"] = __version__
        with open(ATTRIBUTION_SUMMARY, "w") as f:
            json.dump(metrics, f, indent=2)
        print(f"✓ Wrote summary to {ATTRIBUTION_SUMMARY}")
        return True
    except IOError as e:
        print(f"ERROR: Failed to write summary: {e}")
        return False


def run_attribution():
    """Execute attribution cycle."""
    print(f"[Attribution v{__version__}] Starting analysis...")

    # 1. Load organism keywords
    organism_keywords = load_organism_keywords()
    print(f"  Organism keywords loaded: {len(organism_keywords)}")
    if organism_keywords:
        print(f"    Examples: {list(organism_keywords)[:5]}")

    # 2. Extract user searches from behavior log
    user_searches = extract_searches_from_behavior()
    print(f"  User searches extracted: {len(user_searches)}")

    # 3. Load dataset tweets
    tweets = load_dataset_with_timestamps()
    print(f"  Dataset tweets loaded: {len(tweets)}")

    if not tweets:
        print("⚠ No tweets in dataset. Skipping attribution.")
        return 1

    # 4. Match searches to results
    attributions, metrics = match_searches_to_results(
        organism_keywords, user_searches, tweets
    )

    print(f"  Attributions found: {len(attributions)}")
    print(f"    Organism-guided: {metrics['organism_guided_tweets']}")
    print(f"    Spontaneous: {metrics['spontaneous_tweets']}")
    print(f"    Unattributed: {metrics['unattributed_tweets']}")

    # 5. Write outputs
    if not write_attribution_log(attributions):
        return 1

    if not write_attribution_summary(metrics):
        return 1

    print(f"✓ Attribution complete.")
    return 0


if __name__ == "__main__":
    import sys
    sys.exit(run_attribution())
