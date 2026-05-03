#!/usr/bin/env python3
"""
Killchain Reconstructor — link human clicks to tweets viewed on Twitter.

Key finding: All 6 observation tweets appear in captures.
New strategy: For each observation tweet, find captures containing it,
then find clicks around those capture times.
"""

import json
import os
from pathlib import Path
from datetime import datetime, timedelta
from typing import Dict, List, Tuple, Optional, Set


def load_behavior_log() -> List[Dict]:
    """Load behavior_log.jsonl."""
    events = []
    with open(os.path.expanduser("~/.cynic/organs/hermes/behavior/behavior_log.jsonl")) as f:
        for i, line in enumerate(f):
            events.append(json.loads(line))
            if i >= 1000000:  # Load enough for analysis
                break
    return events


def load_captures_index() -> Dict[str, Dict]:
    """Load all captures with their timestamp as key."""
    captures = {}
    capture_dir = Path(os.path.expanduser("~/.cynic/organs/hermes/x/captures"))

    for capture_file in sorted(capture_dir.glob("*.json")):
        timestamp_str = f"{capture_file.name[:8]} {capture_file.name[9:15]}"
        try:
            ts = datetime.strptime(timestamp_str, "%Y%m%d %H%M%S")
            with open(capture_file) as f:
                data = json.load(f)
                captures[ts] = {
                    "file": capture_file.name,
                    "data": data,
                }
        except Exception as e:
            pass

    return captures


def load_observations() -> List[Dict]:
    """Load observations."""
    observations = []
    obs_dir = Path(os.path.expanduser("~/.cynic/organs/hermes/x/observations"))

    for obs_file in sorted(obs_dir.glob("*.json")):
        if obs_file.name.startswith("."):
            continue
        try:
            with open(obs_file) as f:
                data = json.load(f)
                if data.get("type") == "observation":
                    observations.append(data)
        except:
            pass

    return observations


def extract_tweets_from_capture(capture_data: Dict) -> Set[str]:
    """Extract tweet IDs from a capture snapshot."""
    tweets = set()

    try:
        resp = capture_data.get("response", {})
        data = resp.get("data", {})

        # Try all possible timeline keys
        timeline_keys = [
            ("home", "home_timeline_urt"),
            ("search", "search_timeline_urt"),
            ("user", "user_timeline_urt"),
            ("tweet", "tweet_detail_urt"),
        ]

        for data_key, urt_key in timeline_keys:
            if data_key in data:
                urt = data[data_key].get(urt_key, {})
                instructions = urt.get("instructions", [])
                for instr in instructions:
                    if instr.get("type") == "TimelineAddEntries":
                        for entry in instr.get("entries", []):
                            entry_id = entry.get("entryId", "")
                            if entry_id.startswith("tweet-"):
                                tweet_id = entry_id[6:]
                                tweets.add(tweet_id)
    except:
        pass

    return tweets


def find_captures_with_tweet(
    tweet_id: str,
    captures: Dict[datetime, Dict]
) -> List[Tuple[datetime, str]]:
    """Find all captures containing a specific tweet."""
    matches = []

    for ts, capture in captures.items():
        content_str = json.dumps(capture["data"])
        if tweet_id in content_str:
            matches.append((ts, capture["file"]))

    return matches


def find_clicks_near_time(
    target_time: datetime,
    clicks: List[Dict],
    window_seconds: int = 60
) -> List[Tuple[datetime, Dict]]:
    """Find all clicks within a time window."""
    matches = []

    for click in clicks:
        click_ts = datetime.fromisoformat(click["ts"].replace("+00:00", ""))
        gap = abs((click_ts - target_time).total_seconds())

        if gap <= window_seconds:
            matches.append((click_ts, click))

    return matches


def reconstruct_killchain():
    """Reconstruct killchains for all observations."""
    print("=== KILLCHAIN RECONSTRUCTION (COMPLETE) ===\n")

    # Load data
    print("[1/4] Loading data...")
    behavior_log = load_behavior_log()
    captures = load_captures_index()
    observations = load_observations()

    clicks = [e for e in behavior_log if e.get("type") == "click"]
    print(f"  Behavior: {len(clicks)} clicks (of {len(behavior_log)} total events)")
    print(f"  Captures: {len(captures)} snapshots")
    print(f"  Observations: {len(observations)}")

    # Build observation index
    print("\n[2/4] Indexing observations...")
    observation_by_tweet = {}
    for obs in observations:
        for tid in obs.get("tweet_ids", []):
            observation_by_tweet[tid] = obs

    print(f"  {len(observation_by_tweet)} unique tweets in observations")

    # Find killchains
    print("\n[3/4] Tracing killchains...")
    killchains = []

    for obs in observations:
        obs_finding = obs.get("finding", "")
        print(f"\n  Observation: {obs_finding[:60]}...")

        for tweet_id in obs.get("tweet_ids", []):
            # Find captures containing this tweet
            matching_captures = find_captures_with_tweet(tweet_id, captures)

            if not matching_captures:
                print(f"    ✗ Tweet {tweet_id}: not in any capture")
                continue

            print(f"    ✓ Tweet {tweet_id}: found in {len(matching_captures)} captures")

            # For each capture, find clicks around that time
            for capture_ts, capture_file in matching_captures:
                nearby_clicks = find_clicks_near_time(capture_ts, clicks, window_seconds=60)

                if nearby_clicks:
                    print(f"      Capture {capture_file}: {len(nearby_clicks)} clicks within ±60s")

                    for click_ts, click in nearby_clicks[:3]:
                        gap = (capture_ts - click_ts).total_seconds()
                        killchains.append({
                            "observation": obs_finding,
                            "tweet_id": tweet_id,
                            "click_time": click_ts,
                            "click_coords": (click["x"], click["y"]),
                            "capture_time": capture_ts,
                            "capture_file": capture_file,
                            "gap_seconds": gap,
                        })

                        print(
                            f"        → Click {click_ts.strftime('%H:%M:%S')} at "
                            f"({click['x']}, {click['y']}) [{gap:.0f}s before capture]"
                        )

    # Summary
    print(f"\n[4/4] Summary")
    print(f"  Total killchains linked: {len(killchains)}")
    print(f"  Observations with links: {len(set(k['observation'] for k in killchains))}")

    if killchains:
        print(f"\n=== PROOF-OF-CONCEPT ===")
        print(f"✓ Killchain reconstruction WORKS!")
        print(f"  Method: Click timestamp → Find nearby captures → Extract tweet IDs → Match to observations")
        print(f"  Result: {len(killchains)} click→tweet links established")
        print(f"\nNext step: Use click coordinates (X,Y) to map to specific tweet position in feed")
        print(f"  Blocker: Need Twitter feed layout (tweet height, scroll offset)")
    else:
        print("\n✗ No killchains linked")
        print("  Reason: clicks may not occur near captures")
        print("  Alternative: Analyze TimelineAddEntries order to infer feed position from offset")

    return killchains


if __name__ == "__main__":
    killchains = reconstruct_killchain()
