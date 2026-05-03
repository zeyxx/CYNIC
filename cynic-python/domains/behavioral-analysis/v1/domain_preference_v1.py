#!/usr/bin/env python3
"""
Human Domain Preference — Extract actual domain preferences from human browsing.

Key insight: Separate HomeTimeline (human) from SearchTimeline (organ-x).
This reveals what domains the human naturally engages with, not organ-x's random farming.

Goal: Build human behavioral model for domain routing prediction.
"""

import json
import os
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Set, Tuple
from collections import defaultdict


def load_behavior_log() -> List[Dict]:
    """Load behavior_log.jsonl."""
    events = []
    with open(os.path.expanduser("~/.cynic/organs/hermes/behavior/behavior_log.jsonl")) as f:
        for line in f:
            events.append(json.loads(line))
    return events


def load_captures_by_type() -> Dict[str, List[Tuple[datetime, Dict]]]:
    """Load captures, keyed by timeline type."""
    by_type = defaultdict(list)
    capture_dir = Path(os.path.expanduser("~/.cynic/organs/hermes/x/captures"))

    for capture_file in sorted(capture_dir.glob("*.json")):
        # Parse type from filename: 20260427_081933_TimelineType.json
        parts = capture_file.name.split("_")
        if len(parts) >= 3:
            ts_str = f"{parts[0]} {parts[1]}"
            timeline_type = parts[2].replace(".json", "")

            try:
                ts = datetime.strptime(ts_str, "%Y%m%d %H%M%S")
                with open(capture_file) as f:
                    data = json.load(f)
                    by_type[timeline_type].append((ts, {
                        "file": capture_file.name,
                        "data": data,
                    }))
            except:
                pass

    return dict(by_type)


def extract_tweets_from_capture(capture_data: Dict) -> List[Tuple[str, int]]:
    """Extract (tweet_id, position) from capture."""
    tweets = []

    try:
        resp = capture_data.get("response", {})
        data = resp.get("data", {})

        # Find URT structure
        for key in ["home", "search", "user", "tweet"]:
            if key not in data:
                continue

            urt_key = f"{key}_timeline_urt" if key != "tweet" else f"{key}_detail_urt"
            urt = data[key].get(urt_key, {})
            instructions = urt.get("instructions", [])

            for instr in instructions:
                if instr.get("type") == "TimelineAddEntries":
                    for position, entry in enumerate(instr.get("entries", [])):
                        entry_id = entry.get("entryId", "")
                        if entry_id.startswith("tweet-"):
                            tweet_id = entry_id[6:]
                            tweets.append((tweet_id, position))
    except:
        pass

    return tweets


def load_dataset() -> Dict[str, Dict]:
    """Load dataset."""
    dataset = {}
    with open(os.path.expanduser("~/.cynic/organs/hermes/x/dataset.jsonl")) as f:
        for line in f:
            tweet = json.loads(line)
            dataset[tweet["tweet_id"]] = tweet
    return dataset


def extract_domains_from_tweets(tweet_ids: List[str], dataset: Dict[str, Dict]) -> List[str]:
    """Heuristically extract domains from tweets."""
    domains = []

    for tid in tweet_ids:
        if tid not in dataset:
            continue

        tweet = dataset[tid]

        # Domain heuristics based on content
        cashtags = tweet.get("cashtags", [])
        text_lower = tweet.get("text", "").lower()

        if cashtags:
            domains.append("token-analysis")
        elif "wallet" in text_lower or "address:" in text_lower:
            domains.append("wallet-judgment")
        elif any(w in text_lower for w in ["mate", "gm", "bro", "eth", "solana", "cult"]):
            domains.append("social")
        elif any(w in text_lower for w in ["chess", "openings", "tactics"]):
            domains.append("chess")
        else:
            domains.append("general")

    return domains


def analyze_human_domain_preferences():
    """Analyze which domains the human naturally engages with."""
    print("=== HUMAN DOMAIN PREFERENCE ANALYSIS ===\n")

    # Load data
    print("[1/4] Loading data...")
    behavior_log = load_behavior_log()
    captures_by_type = load_captures_by_type()
    dataset = load_dataset()

    clicks = [e for e in behavior_log if e.get("type") == "click"]
    print(f"  Total clicks: {len(clicks)}")
    print(f"  Capture types: {list(captures_by_type.keys())}")

    # Separate human (HomeTimeline) from organ-x (SearchTimeline)
    print("\n[2/4] Analyzing HomeTimeline (human browsing)...")
    human_timeline_clicks = []

    if "HomeTimeline" in captures_by_type:
        home_captures = captures_by_type["HomeTimeline"]
        print(f"  HomeTimeline captures: {len(home_captures)}")

        # For each HomeTimeline capture, find nearby clicks
        for capture_ts, capture in home_captures:
            tweets = extract_tweets_from_capture(capture["data"])
            if not tweets:
                continue

            # Find clicks within 120s of this capture
            for click in clicks:
                click_ts = datetime.fromisoformat(click["ts"].replace("+00:00", ""))
                gap = abs((click_ts - capture_ts).total_seconds())

                if gap <= 120:
                    human_timeline_clicks.append({
                        "click_time": click_ts,
                        "click_coords": (click["x"], click["y"]),
                        "capture_file": capture["file"],
                        "tweets_visible": tweets,
                        "gap_seconds": gap,
                    })

        print(f"  HomeTimeline clicks linked: {len(human_timeline_clicks)}")

    # Analyze organ-x (SearchTimeline)
    print("\n[3/4] Analyzing SearchTimeline (organ-x searches)...")
    organ_x_timeline_clicks = []

    if "SearchTimeline" in captures_by_type:
        search_captures = captures_by_type["SearchTimeline"]
        print(f"  SearchTimeline captures: {len(search_captures)}")

        for capture_ts, capture in search_captures:
            tweets = extract_tweets_from_capture(capture["data"])
            if not tweets:
                continue

            for click in clicks:
                click_ts = datetime.fromisoformat(click["ts"].replace("+00:00", ""))
                gap = abs((click_ts - capture_ts).total_seconds())

                if gap <= 120:
                    organ_x_timeline_clicks.append({
                        "click_time": click_ts,
                        "capture_file": capture["file"],
                        "tweets_visible": tweets,
                        "gap_seconds": gap,
                    })

        print(f"  SearchTimeline clicks linked: {len(organ_x_timeline_clicks)}")

    # Extract domains from human clicks
    print("\n[4/4] Domain preference analysis...")
    human_domains = defaultdict(int)
    organ_x_domains = defaultdict(int)

    if human_timeline_clicks:
        print(f"\n  Human (HomeTimeline) domain clicks:")
        for link in human_timeline_clicks:
            tweet_ids = [tid for tid, _ in link["tweets_visible"]]
            domains = extract_domains_from_tweets(tweet_ids, dataset)
            for domain in domains:
                human_domains[domain] += 1

        total = sum(human_domains.values())
        for domain in sorted(human_domains.keys(), key=lambda d: -human_domains[d]):
            pct = 100.0 * human_domains[domain] / total
            print(f"    {domain}: {human_domains[domain]} ({pct:.1f}%)")

    if organ_x_timeline_clicks:
        print(f"\n  Organ-X (SearchTimeline) domain clicks:")
        for link in organ_x_timeline_clicks:
            tweet_ids = [tid for tid, _ in link["tweets_visible"]]
            domains = extract_domains_from_tweets(tweet_ids, dataset)
            for domain in domains:
                organ_x_domains[domain] += 1

        total = sum(organ_x_domains.values())
        for domain in sorted(organ_x_domains.keys(), key=lambda d: -organ_x_domains[d]):
            pct = 100.0 * organ_x_domains[domain] / total
            print(f"    {domain}: {organ_x_domains[domain]} ({pct:.1f}%)")

    # Summary
    print(f"\n=== CONCLUSION ===")
    if human_domains:
        human_preference = sorted(human_domains.keys(), key=lambda d: -human_domains[d])[0]
        print(f"✓ Human domain preference: {human_preference}")
        print(f"  (Train domain_lifecycle_predictor on this preference, not organ-x's random farming)")
    else:
        print(f"✗ No human domain preferences extracted")
        print(f"  Reason: HomeTimeline clicks may not align with captures")

    if organ_x_domains:
        print(f"\n✓ Organ-X farms across: {', '.join(sorted(organ_x_domains.keys()))}")
        print(f"  (Verify this matches farming_log domains D1-D6)")


if __name__ == "__main__":
    analyze_human_domain_preferences()
