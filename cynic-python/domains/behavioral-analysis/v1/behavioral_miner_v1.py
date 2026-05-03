#!/usr/bin/env python3
"""
Behavioral Miner — Extract engagement patterns from your interaction data.

Core insight: Your clicks ARE the domain signal. Mine what content triggers them.

Pipeline:
1. Reconstruct clicks → tweets (killchain matching)
2. Extract features from clicked tweets (content, author, narrative, structure)
3. Compare against visible tweets → derive preference signal
4. Aggregate into behavioral profile (content types you engage with)
"""

import json
import os
from pathlib import Path
from datetime import datetime, timedelta
from typing import Dict, List, Set, Tuple, Optional
from collections import defaultdict
import numpy as np


def load_data():
    """Load all data sources."""
    print("[LOAD] Reading datasets...")

    # Behavior log
    behavior_log = []
    with open(os.path.expanduser("~/.cynic/organs/hermes/behavior/behavior_log.jsonl")) as f:
        for line in f:
            behavior_log.append(json.loads(line))

    # Captures indexed by time
    captures_by_time = {}
    capture_dir = Path(os.path.expanduser("~/.cynic/organs/hermes/x/captures"))
    for capture_file in sorted(capture_dir.glob("*.json")):
        ts_str = f"{capture_file.name[:8]} {capture_file.name[9:15]}"
        try:
            ts = datetime.strptime(ts_str, "%Y%m%d %H%M%S")
            with open(capture_file) as f:
                data = json.load(f)
                captures_by_time[ts] = {
                    "file": capture_file.name,
                    "type": capture_file.name.split("_")[2].replace(".json", ""),
                    "data": data,
                }
        except:
            pass

    # Dataset
    dataset = {}
    with open(os.path.expanduser("~/.cynic/organs/hermes/x/dataset.jsonl")) as f:
        for line in f:
            tweet = json.loads(line)
            dataset[tweet["tweet_id"]] = tweet

    clicks = [e for e in behavior_log if e.get("type") == "click"]

    print(f"  Clicks: {len(clicks)}")
    print(f"  Captures: {len(captures_by_time)}")
    print(f"  Tweets: {len(dataset)}")

    return clicks, captures_by_time, dataset


def extract_tweets_from_capture(capture_data: Dict) -> List[Tuple[str, int]]:
    """Extract (tweet_id, position) from capture."""
    tweets = []
    try:
        resp = capture_data.get("response", {})
        data = resp.get("data", {})

        for key in ["home", "search", "user", "tweet"]:
            if key not in data:
                continue
            urt_key = f"{key}_timeline_urt" if key != "tweet" else f"{key}_detail_urt"
            urt = data[key].get(urt_key, {})
            instructions = urt.get("instructions", [])

            for instr in instructions:
                if instr.get("type") == "TimelineAddEntries":
                    for pos, entry in enumerate(instr.get("entries", [])):
                        entry_id = entry.get("entryId", "")
                        if entry_id.startswith("tweet-"):
                            tweet_id = entry_id[6:]
                            tweets.append((tweet_id, pos))
    except:
        pass

    return tweets


def extract_content_features(tweet: Dict) -> Dict:
    """Extract engagement-relevant features from a tweet."""
    return {
        # Token signals
        "has_cashtags": len(tweet.get("cashtags", [])) > 0,
        "cashtag_count": len(tweet.get("cashtags", [])),
        "cashtags": tweet.get("cashtags", []),

        # Narrative signals
        "narratives": tweet.get("narratives", []),
        "has_narrative": len(tweet.get("narratives", [])) > 0,

        # Author signals
        "author": tweet.get("author_screen_name", ""),
        "author_tier": tweet.get("author_tier", "unknown"),
        "followers": tweet.get("author_followers_count", 0),
        "verified": tweet.get("author_verified", False),
        "blue_verified": tweet.get("author_is_blue_verified", False),

        # Structure
        "has_media": tweet.get("has_media", False),
        "word_count": len(tweet.get("text", "").split()),
        "is_quote": bool(tweet.get("quoted_tweet", {})),
        "is_reply": bool(tweet.get("in_reply_to_tweet_id", "")),

        # Engagement
        "likes": tweet.get("likes", 0),
        "retweets": tweet.get("retweets", 0),
        "replies": tweet.get("replies", 0),
        "views": tweet.get("views", 0),

        # Engagement rate
        "engagement_rate": (tweet.get("likes", 0) + tweet.get("retweets", 0)) / max(tweet.get("views", 1), 1),
    }


def reconstruct_clicked_tweets(clicks: List[Dict], captures_by_time: Dict, dataset: Dict, window_seconds: int = 120) -> List[Tuple[datetime, Dict, int, Dict]]:
    """Reconstruct which tweets were clicked.

    Returns: List of (click_time, click_event, tweet_id, tweet_content)
    """
    clicked_tweets = []

    for click in clicks:
        click_ts = datetime.fromisoformat(click["ts"].replace("+00:00", ""))

        # Find nearest capture
        nearest_ts = min(
            captures_by_time.keys(),
            key=lambda ts: abs((ts - click_ts).total_seconds()),
            default=None
        )

        if nearest_ts is None:
            continue

        gap = abs((nearest_ts - click_ts).total_seconds())
        if gap > window_seconds:
            continue

        capture = captures_by_time[nearest_ts]
        tweets = extract_tweets_from_capture(capture["data"])

        if tweets:
            # For now, assume click matches first visible tweet
            # (full coordinate mapping would be needed for precision)
            tweet_id, pos = tweets[0]

            if tweet_id in dataset:
                clicked_tweets.append((click_ts, click, tweet_id, dataset[tweet_id]))

    return clicked_tweets


def aggregate_preferences(clicked_tweets: List[Tuple[datetime, Dict, int, Dict]]) -> Dict:
    """Aggregate what content features correlate with clicks."""
    features_in_clicks = defaultdict(list)

    for click_ts, click_event, tweet_id, tweet in clicked_tweets:
        features = extract_content_features(tweet)

        for key, value in features.items():
            features_in_clicks[key].append(value)

    # Compute summary statistics
    profile = {
        "total_clicks": len(clicked_tweets),
        "token_content_clicks": sum(1 for _, _, _, t in clicked_tweets if t.get("cashtags")),
        "narrative_content_clicks": sum(1 for _, _, _, t in clicked_tweets if t.get("narratives")),
        "media_clicks": sum(1 for _, _, _, t in clicked_tweets if t.get("has_media")),
        "verified_author_clicks": sum(1 for _, _, _, t in clicked_tweets if t.get("author_verified")),
        "blue_verified_author_clicks": sum(1 for _, _, _, t in clicked_tweets if t.get("author_is_blue_verified")),
    }

    # Add ratios (must collect keys first to avoid dict size change)
    keys_to_add = {f"{k}_ratio": v / profile["total_clicks"] if profile["total_clicks"] > 0 else 0
                   for k, v in profile.items() if "clicks" in k and k != "total_clicks"}
    profile.update(keys_to_add)

    # Average metrics
    profile["avg_word_count"] = np.mean([len(t.get("text", "").split()) for _, _, _, t in clicked_tweets])
    profile["avg_likes"] = np.mean([t.get("likes", 0) for _, _, _, t in clicked_tweets])
    profile["avg_views"] = np.mean([t.get("views", 0) for _, _, _, t in clicked_tweets])
    profile["avg_engagement_rate"] = np.mean([
        (t.get("likes", 0) + t.get("retweets", 0)) / max(t.get("views", 1), 1)
        for _, _, _, t in clicked_tweets
    ])

    # Top authors you engage with
    top_authors = defaultdict(int)
    for _, _, _, t in clicked_tweets:
        top_authors[t.get("author_screen_name", "")] += 1

    profile["top_authors"] = sorted(top_authors.items(), key=lambda x: -x[1])[:10]

    # Top narratives you engage with
    top_narratives = defaultdict(int)
    for _, _, _, t in clicked_tweets:
        for narrative in t.get("narratives", []):
            top_narratives[narrative] += 1

    profile["top_narratives"] = sorted(top_narratives.items(), key=lambda x: -x[1])[:10]

    # Top cashtags you engage with
    top_cashtags = defaultdict(int)
    for _, _, _, t in clicked_tweets:
        for tag in t.get("cashtags", []):
            top_cashtags[tag] += 1

    profile["top_cashtags"] = sorted(top_cashtags.items(), key=lambda x: -x[1])[:15]

    return profile


def mine_behavior():
    """Execute behavioral mining."""
    print("\n=== BEHAVIORAL MINER ===\n")

    # Load
    clicks, captures_by_time, dataset = load_data()

    # Reconstruct
    print("\n[RECONSTRUCT] Linking clicks to tweets...")
    clicked_tweets = reconstruct_clicked_tweets(clicks, captures_by_time, dataset)
    print(f"  Clicked tweets reconstructed: {len(clicked_tweets)}")

    if not clicked_tweets:
        print("  ✗ No clicked tweets found")
        return

    # Aggregate
    print("\n[AGGREGATE] Computing engagement profile...")
    profile = aggregate_preferences(clicked_tweets)

    # Display
    print(f"\n=== YOUR ENGAGEMENT PROFILE ===")
    print(f"\nTotal clicks analyzed: {profile['total_clicks']}")

    print(f"\nContent type engagement:")
    print(f"  Tokens: {profile.get('token_content_clicks_ratio', 0):.1%}")
    print(f"  Narratives: {profile.get('narrative_content_clicks_ratio', 0):.1%}")
    print(f"  Media: {profile.get('media_clicks_ratio', 0):.1%}")
    print(f"  From verified authors: {profile.get('verified_author_clicks_ratio', 0):.1%}")
    print(f"  From blue-verified authors: {profile.get('blue_verified_author_clicks_ratio', 0):.1%}")

    print(f"\nContent metrics (what you click):")
    print(f"  Avg word count: {profile.get('avg_word_count', 0):.0f}")
    print(f"  Avg likes: {profile.get('avg_likes', 0):.0f}")
    print(f"  Avg views: {profile.get('avg_views', 0):.0f}")
    print(f"  Avg engagement rate: {profile.get('avg_engagement_rate', 0):.3f}")

    print(f"\nTop authors you engage with:")
    for author, count in profile.get('top_authors', [])[:5]:
        print(f"  @{author}: {count} clicks")

    print(f"\nTop narratives you engage with:")
    for narrative, count in profile.get('top_narratives', [])[:5]:
        print(f"  {narrative}: {count} clicks")

    print(f"\nTop tokens/cashtags you engage with:")
    for tag, count in profile.get('top_cashtags', [])[:8]:
        print(f"  ${tag}: {count} clicks")

    # Save
    output_file = Path("cynic-python/behavioral_profile.json")
    with open(output_file, "w") as f:
        # Convert non-serializable types
        profile_serialized = {
            k: v for k, v in profile.items()
            if not isinstance(v, (list, tuple)) or (isinstance(v, list) and all(isinstance(x, (str, int, float, bool)) for x in v))
        }
        json.dump(profile_serialized, f, indent=2)

    print(f"\n✓ Profile saved to {output_file}")


if __name__ == "__main__":
    mine_behavior()
