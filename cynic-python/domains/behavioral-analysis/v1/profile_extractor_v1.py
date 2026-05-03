#!/usr/bin/env python3
"""
Author-Narrative Clusters — Extract what combinations trigger your engagement.

Key fix: Don't assume first tweet = clicked tweet. Instead:
1. Extract ALL visible tweets from each capture
2. Find click clusters around capture times
3. Infer which tweets match clicks (spatial + temporal proximity)
4. Extract author-narrative combinations from your engaged tweets
5. Cluster by domain (tokens, LLM, sovereignty, security, etc.)
"""

import json
import os
from pathlib import Path
from datetime import datetime, timedelta
from typing import Dict, List, Set, Tuple, Optional
from collections import defaultdict
import numpy as np


def load_data():
    """Load all data."""
    print("[LOAD] Reading datasets...")

    behavior_log = []
    with open(os.path.expanduser("~/.cynic/organs/hermes/behavior/behavior_log.jsonl")) as f:
        for line in f:
            behavior_log.append(json.loads(line))

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


def extract_all_tweets_from_capture(capture_data: Dict) -> List[Tuple[str, int]]:
    """Extract ALL (tweet_id, position) from capture."""
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


def infer_clicked_tweets(clicks: List[Dict], captures_by_time: Dict, dataset: Dict, window_seconds: int = 120) -> List[Dict]:
    """Infer engagement context from clicks and visible tweets.

    Strategy:
    1. For each click, find nearest capture
    2. Extract all visible tweets from that capture
    3. Return ALL visible tweets as engagement context
       (If you clicked near this capture, you were reading these tweets)
    4. Infer the specific clicked tweet from position probability
    """
    clicked_tweets_list = []
    capture_times = sorted(captures_by_time.keys())

    for click in clicks:
        click_ts = datetime.fromisoformat(click["ts"].replace("+00:00", ""))

        # Find nearest capture
        nearest_ts = min(
            capture_times,
            key=lambda ts: abs((ts - click_ts).total_seconds()),
            default=None
        )

        if nearest_ts is None:
            continue

        gap = abs((nearest_ts - click_ts).total_seconds())
        if gap > window_seconds:
            continue

        capture = captures_by_time[nearest_ts]

        # Extract all visible tweets in this capture
        visible_tweets = extract_all_tweets_from_capture(capture["data"])
        if not visible_tweets:
            continue

        # Add all visible tweets to clicked context
        # (Heuristic: earlier position = higher likelihood of click)
        for position, (tweet_id, pos) in enumerate(visible_tweets):
            if tweet_id in dataset:
                # Likelihood decreases with position (top tweets more likely to be clicked)
                likelihood = 1.0 / (1.0 + 0.1 * position)

                clicked_tweets_list.append({
                    "click_time": click_ts,
                    "click_coords": (click["x"], click["y"]),
                    "capture_time": nearest_ts,
                    "capture_file": capture["file"],
                    "capture_type": capture["type"],
                    "tweet_id": tweet_id,
                    "tweet": dataset[tweet_id],
                    "gap_seconds": gap,
                    "position_in_feed": pos,
                    "visible_tweet_count": len(visible_tweets),
                    "position_in_capture": position,
                    "click_likelihood": likelihood,
                })

    return clicked_tweets_list


def infer_domain(tweet: Dict) -> str:
    """Heuristically infer domain from tweet content."""
    cashtags = tweet.get("cashtags", [])
    text_lower = tweet.get("text", "").lower()
    narratives = tweet.get("narratives", [])

    # Token domain (cashtags + token keywords)
    if cashtags or any(w in text_lower for w in ["token", "mint", "dex", "swap", "rug", "pump.fun"]):
        return "token"

    # LLM domain
    if any(w in text_lower for w in ["llm", "claude", "gpt", "reasoning", "inference", "model", "qwen", "transformer"]):
        return "llm"

    # Sovereignty domain
    if any(w in text_lower for w in ["validator", "node", "decentralized", "p2p", "censorship", "oracle", "consensus"]):
        return "sovereignty"

    # Security domain
    if any(w in text_lower for w in ["exploit", "vulnerability", "honeypot", "audit", "scam", "backdoor"]):
        return "security"

    # Macro domain
    if any(w in text_lower for w in ["btc", "bitcoin", "macro", "volatility", "leverage", "hedge", "regulation"]):
        return "macro"

    # Epistemology domain
    if any(w in text_lower for w in ["confidence", "calibration", "probability", "epistemic", "doubt", "uncertain"]):
        return "epistemology"

    # Default to general
    return "general"


def extract_author_narrative_profile(clicked_tweets: List[Dict]) -> Dict:
    """Extract author-narrative combinations by domain.

    Weighted by click likelihood (tweets appearing higher in feed are more likely clicked).
    """
    profile = {
        "total_events": len(clicked_tweets),
        "total_unique_clicks": len(set(ct["click_time"] for ct in clicked_tweets)),
        "by_domain": {},
        "top_authors_by_domain": {},
        "top_narratives_by_domain": {},
    }

    # Group by domain
    by_domain = defaultdict(list)
    for ct in clicked_tweets:
        domain = infer_domain(ct["tweet"])
        by_domain[domain].append(ct)

    # Analyze each domain
    total_weight = sum(ct.get("click_likelihood", 1.0) for ct in clicked_tweets)

    for domain, tweets in by_domain.items():
        domain_weight = sum(ct.get("click_likelihood", 1.0) for ct in tweets)
        domain_pct = 100.0 * domain_weight / total_weight if total_weight > 0 else 0

        # Top authors (weighted)
        top_authors = defaultdict(float)
        for ct in tweets:
            author = ct["tweet"].get("author_screen_name", "unknown")
            top_authors[author] += ct.get("click_likelihood", 1.0)

        # Top narratives (weighted)
        top_narratives = defaultdict(float)
        for ct in tweets:
            for narrative in ct["tweet"].get("narratives", []):
                top_narratives[narrative] += ct.get("click_likelihood", 1.0)

        # Author-narrative combinations (weighted)
        combinations = defaultdict(float)
        for ct in tweets:
            author = ct["tweet"].get("author_screen_name", "")
            for narrative in ct["tweet"].get("narratives", []):
                combinations[f"{author}:{narrative}"] += ct.get("click_likelihood", 1.0)

        profile["by_domain"][domain] = {
            "weighted_count": domain_weight,
            "percent": domain_pct,
            "raw_count": len(tweets),
            "top_authors": [(a, f"{w:.1f}") for a, w in sorted(top_authors.items(), key=lambda x: -x[1])[:5]],
            "top_narratives": [(n, f"{w:.1f}") for n, w in sorted(top_narratives.items(), key=lambda x: -x[1])[:5]],
            "top_combinations": [(c, f"{w:.1f}") for c, w in sorted(combinations.items(), key=lambda x: -x[1])[:8]],
        }

        profile["top_authors_by_domain"][domain] = sorted(top_authors.items(), key=lambda x: -x[1])[:5]
        profile["top_narratives_by_domain"][domain] = sorted(top_narratives.items(), key=lambda x: -x[1])[:5]

    return profile


def analyze():
    """Execute author-narrative cluster analysis."""
    print("\n=== AUTHOR-NARRATIVE CLUSTERS ===\n")

    clicks, captures_by_time, dataset = load_data()

    print("\n[INFER] Linking clicks to all visible tweets...")
    clicked_tweets = infer_clicked_tweets(clicks, captures_by_time, dataset)
    print(f"  Clicked tweets inferred: {len(clicked_tweets)}")

    if not clicked_tweets:
        print("  ✗ No clicked tweets found")
        return

    print("\n[ANALYZE] Extracting author-narrative combinations...")
    profile = extract_author_narrative_profile(clicked_tweets)

    print(f"\n=== YOUR ENGAGEMENT ACROSS DOMAINS ===\n")
    print(f"Total events: {profile['total_events']}")
    print(f"Unique clicks: {profile['total_unique_clicks']}\n")

    for domain in sorted(profile["by_domain"].keys(), key=lambda d: -profile["by_domain"][d]["weighted_count"]):
        info = profile["by_domain"][domain]
        print(f"{domain.upper()}: {info['raw_count']} tweets, {info['weighted_count']:.0f} weighted clicks ({info['percent']:.1f}%)")

        print(f"  Top authors:")
        for author, weight in info["top_authors"]:
            print(f"    @{author}: {weight}")

        print(f"  Top narratives:")
        for narrative, weight in info["top_narratives"]:
            print(f"    {narrative}: {weight}")

        print(f"  Top author-narrative combinations:")
        for combo, weight in info["top_combinations"]:
            print(f"    {combo}: {weight}")

        print()

    # Save
    output_file = Path("cynic-python/author_narrative_profile.json")
    with open(output_file, "w") as f:
        # Convert defaultdict to dict for JSON serialization
        profile_clean = {
            "total_events": profile["total_events"],
            "total_unique_clicks": profile["total_unique_clicks"],
            "by_domain": {
                k: {
                    "weighted_count": v["weighted_count"],
                    "percent": v["percent"],
                    "raw_count": v["raw_count"],
                    "top_authors": v["top_authors"],
                    "top_narratives": v["top_narratives"],
                    "top_combinations": v["top_combinations"],
                }
                for k, v in profile["by_domain"].items()
            }
        }
        json.dump(profile_clean, f, indent=2)

    print(f"✓ Profile saved to {output_file}\n")

    # Inference
    print("=== ORGAN-X ROUTING SIGNAL ===\n")

    print("For each domain, organ-x should mine:")
    for domain in sorted(profile["by_domain"].keys(), key=lambda d: -profile["by_domain"][d]["weighted_count"]):
        info = profile["by_domain"][domain]

        if info["raw_count"] > 0:
            top_author = info["top_authors"][0][0] if info["top_authors"] else "unknown"
            top_narrative = info["top_narratives"][0][0] if info["top_narratives"] else "unknown"
            print(f"  {domain}: Focus on @{top_author} + {top_narrative} pattern")


if __name__ == "__main__":
    analyze()
