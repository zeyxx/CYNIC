#!/usr/bin/env python3
"""
Trace Author Interactions — Drill down from aggregated profile to individual tweets/clicks.

Usage:
  python3 trace_author_interactions.py --author gcrtrd
  python3 trace_author_interactions.py --author gcrtrd --domain general
  python3 trace_author_interactions.py --author gcrtrd --domain general --narrative hype
"""

import json
import os
from pathlib import Path
from datetime import datetime
from typing import Dict, List
from collections import defaultdict
import argparse


def load_data():
    """Load all datasets."""
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
    print(f"  Tweets: {len(dataset)}\n")

    return clicks, captures_by_time, dataset


def extract_all_tweets_from_capture(capture_data: Dict) -> List[tuple]:
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


def infer_domain(tweet: Dict) -> str:
    """Infer domain from tweet content."""
    cashtags = tweet.get("cashtags", [])
    text_lower = tweet.get("text", "").lower()

    if cashtags or any(w in text_lower for w in ["token", "mint", "dex", "swap", "rug", "pump.fun"]):
        return "token"
    if any(w in text_lower for w in ["llm", "claude", "gpt", "reasoning", "inference", "model", "qwen"]):
        return "llm"
    if any(w in text_lower for w in ["validator", "node", "decentralized", "p2p", "consensus"]):
        return "sovereignty"
    if any(w in text_lower for w in ["exploit", "vulnerability", "honeypot", "audit", "scam"]):
        return "security"
    if any(w in text_lower for w in ["btc", "bitcoin", "macro", "volatility", "leverage"]):
        return "macro"
    if any(w in text_lower for w in ["confidence", "calibration", "probability", "epistemic"]):
        return "epistemology"
    return "general"


def trace_author_interactions(
    clicks: List[Dict],
    captures_by_time: Dict,
    dataset: Dict,
    author_filter: str = None,
    domain_filter: str = None,
    narrative_filter: str = None,
):
    """Trace individual author interactions."""
    interactions = []
    capture_times = sorted(captures_by_time.keys())

    print(f"[TRACE] Linking clicks to tweets for author={author_filter}, domain={domain_filter}...\n")

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
        if gap > 120:
            continue

        capture = captures_by_time[nearest_ts]
        visible_tweets = extract_all_tweets_from_capture(capture["data"])

        if not visible_tweets:
            continue

        # Infer likelihood of each visible tweet being clicked
        for position, (tweet_id, pos) in enumerate(visible_tweets):
            if tweet_id not in dataset:
                continue

            tweet = dataset[tweet_id]
            author = tweet.get("author_screen_name", "unknown")
            domain = infer_domain(tweet)
            narratives = tweet.get("narratives", [])
            likelihood = 1.0 / (1.0 + 0.1 * position)

            # Apply filters
            if author_filter and author.lower() != author_filter.lower():
                continue
            if domain_filter and domain != domain_filter:
                continue
            if narrative_filter and narrative_filter not in narratives:
                continue

            interactions.append({
                "click_time": click_ts,
                "click_coords": (click["x"], click["y"]),
                "capture_time": nearest_ts,
                "capture_type": capture["type"],
                "capture_file": capture["file"],
                "tweet_id": tweet_id,
                "author": author,
                "domain": domain,
                "narratives": narratives,
                "position_in_feed": pos,
                "position_in_capture": position,
                "likelihood": likelihood,
                "text": tweet.get("text", ""),
                "created_at": tweet.get("created_at", ""),
                "likes": tweet.get("likes", 0),
                "views": tweet.get("views", 0),
            })

    return interactions


def print_interactions(interactions: List[Dict], max_show: int = 20):
    """Pretty-print interactions."""
    if not interactions:
        print("✗ No interactions found matching filters\n")
        return

    print(f"=== {len(interactions)} INTERACTIONS FOUND ===\n")

    # Aggregate by domain
    by_domain = defaultdict(list)
    for inter in interactions:
        by_domain[inter["domain"]].append(inter)

    for domain in sorted(by_domain.keys(), key=lambda d: -len(by_domain[d])):
        domain_inters = by_domain[domain]
        print(f"\n{domain.upper()} ({len(domain_inters)} tweets):")
        print("─" * 100)

        for i, inter in enumerate(sorted(domain_inters, key=lambda x: -x["likelihood"])[:max_show]):
            print(f"\n  [{i+1}] Likelihood: {inter['likelihood']:.2f}")
            print(f"      Time: {inter['click_time'].isoformat()} (capture: {inter['capture_type']})")
            print(f"      Position in feed: #{inter['position_in_feed']} → Position in capture: #{inter['position_in_capture']}")
            print(f"      Domain: {inter['domain']} | Narratives: {inter['narratives']}")
            print(f"      Engagement: {inter['likes']} likes, {inter['views']} views")
            print(f"      Tweet: {inter['text'][:100]}...")
            print(f"      ID: {inter['tweet_id']}")


def main():
    parser = argparse.ArgumentParser(description="Trace author interactions")
    parser.add_argument("--author", type=str, help="Author handle (e.g., gcrtrd)")
    parser.add_argument("--domain", type=str, help="Domain filter (general, llm, token, security, etc.)")
    parser.add_argument("--narrative", type=str, help="Narrative filter (hype, analysis, warning, launch)")
    parser.add_argument("--max-show", type=int, default=20, help="Max interactions to show per domain")

    args = parser.parse_args()

    if not args.author:
        print("Usage: python3 trace_author_interactions.py --author <handle> [--domain <domain>] [--narrative <narrative>]\n")
        print("Examples:")
        print("  python3 trace_author_interactions.py --author gcrtrd")
        print("  python3 trace_author_interactions.py --author gcrtrd --domain general")
        print("  python3 trace_author_interactions.py --author gcrtrd --domain general --narrative hype\n")
        return

    clicks, captures_by_time, dataset = load_data()

    interactions = trace_author_interactions(
        clicks,
        captures_by_time,
        dataset,
        author_filter=args.author,
        domain_filter=args.domain,
        narrative_filter=args.narrative,
    )

    print_interactions(interactions, max_show=args.max_show)

    # Summary stats
    if interactions:
        print(f"\n=== SUMMARY ===\n")
        by_domain = defaultdict(int)
        by_narrative = defaultdict(int)
        total_likelihood = 0

        for inter in interactions:
            by_domain[inter["domain"]] += 1
            total_likelihood += inter["likelihood"]
            for narr in inter["narratives"]:
                by_narrative[narr] += 1

        print(f"Total weighted engagement: {total_likelihood:.1f}")
        print(f"\nBy domain:")
        for domain in sorted(by_domain.keys(), key=lambda d: -by_domain[d]):
            print(f"  {domain}: {by_domain[domain]} tweets")

        if by_narrative:
            print(f"\nBy narrative:")
            for narr in sorted(by_narrative.keys(), key=lambda n: -by_narrative[n]):
                print(f"  {narr}: {by_narrative[narr]} mentions")


if __name__ == "__main__":
    main()
