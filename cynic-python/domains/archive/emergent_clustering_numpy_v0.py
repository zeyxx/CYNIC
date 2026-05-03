#!/usr/bin/env python3
"""
Emergent Domain Discovery (NumPy-only) — Cluster tweets using simple cosine similarity + k-means.

No external ML dependencies. Uses:
- TF (term frequency) vectors manually computed
- Cosine similarity from scratch
- Simple k-means clustering
"""

import json
import os
import math
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Tuple
from collections import defaultdict


def load_data():
    """Load dataset and behavioral data."""
    print("[LOAD] Reading datasets...")

    # Load tweets
    dataset = {}
    with open(os.path.expanduser("~/.cynic/organs/hermes/x/dataset.jsonl")) as f:
        for line in f:
            tweet = json.loads(line)
            dataset[tweet["tweet_id"]] = tweet

    # Load behavior log
    behavior_log = []
    with open(os.path.expanduser("~/.cynic/organs/hermes/behavior/behavior_log.jsonl")) as f:
        for line in f:
            behavior_log.append(json.loads(line))

    clicks = [e for e in behavior_log if e.get("type") == "click"]

    # Load captures for click weighting
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

    print(f"  Tweets: {len(dataset)}")
    print(f"  Clicks: {len(clicks)}")
    print(f"  Captures: {len(captures_by_time)}\n")

    return dataset, clicks, captures_by_time


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


def compute_click_weights(clicks: List[Dict], captures_by_time: Dict, dataset: Dict) -> Dict[str, float]:
    """Compute click weight for each tweet."""
    click_weights = defaultdict(float)
    capture_times = sorted(captures_by_time.keys())

    for click in clicks:
        click_ts = datetime.fromisoformat(click["ts"].replace("+00:00", ""))

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
        visible_tweets = extract_tweets_from_capture(capture["data"])

        if not visible_tweets:
            continue

        for position, (tweet_id, pos) in enumerate(visible_tweets):
            if tweet_id in dataset:
                likelihood = 1.0 / (1.0 + 0.1 * position)
                click_weights[tweet_id] += likelihood

    return dict(click_weights)


def build_vocabulary(dataset: Dict) -> Dict[str, int]:
    """Build vocabulary of key terms."""
    vocab = defaultdict(int)
    keywords = [
        "token", "mint", "dex", "swap", "rug", "pump",
        "llm", "claude", "gpt", "reasoning", "transformer", "attention",
        "validator", "node", "consensus", "decentralized", "p2p",
        "exploit", "vulnerability", "honeypot", "scam", "audit",
        "btc", "bitcoin", "macro", "leverage", "volatility",
        "confidence", "calibration", "probability", "epistemic",
        "analysis", "hype", "launch", "warning", "general"
    ]

    for tweet in dataset.values():
        text_lower = tweet.get("text", "").lower()
        for kw in keywords:
            if kw in text_lower:
                vocab[kw] += 1

    return dict(vocab)


def vectorize_tweets(dataset: Dict, vocab: Dict[str, int]) -> Dict[str, Dict[str, float]]:
    """Create simple TF vectors for tweets."""
    vectors = {}

    for tweet_id, tweet in dataset.items():
        text_lower = tweet.get("text", "").lower()
        vector = {}

        for keyword in vocab.keys():
            if keyword in text_lower:
                vector[keyword] = 1.0

        vectors[tweet_id] = vector

    return vectors


def cosine_similarity(v1: Dict[str, float], v2: Dict[str, float]) -> float:
    """Compute cosine similarity between two vectors."""
    if not v1 or not v2:
        return 0.0

    dot = sum(v1.get(k, 0) * v2.get(k, 0) for k in set(v1.keys()) | set(v2.keys()))
    mag1 = math.sqrt(sum(v**2 for v in v1.values()))
    mag2 = math.sqrt(sum(v**2 for v in v2.values()))

    if mag1 == 0 or mag2 == 0:
        return 0.0

    return dot / (mag1 * mag2)


def simple_kmeans(tweet_ids: List[str], vectors: Dict[str, Dict[str, float]], click_weights: Dict[str, float], n_clusters: int = 7, max_iter: int = 10):
    """Simple k-means clustering."""

    print(f"[CLUSTERING] Running k-means ({n_clusters} clusters)...")

    # Initialize centroids randomly
    import random
    random.seed(42)
    centroid_ids = random.sample(tweet_ids, n_clusters)
    centroids = {i: vectors[cid] for i, cid in enumerate(centroid_ids)}

    for iteration in range(max_iter):
        # Assign tweets to nearest centroid
        assignments = {}
        for tweet_id in tweet_ids:
            vec = vectors[tweet_id]
            best_cluster = 0
            best_sim = -1

            for cluster_id, centroid in centroids.items():
                sim = cosine_similarity(vec, centroid)
                if sim > best_sim:
                    best_sim = sim
                    best_cluster = cluster_id

            assignments[tweet_id] = best_cluster

        # Update centroids
        new_centroids = {i: {} for i in range(n_clusters)}
        for tweet_id, cluster_id in assignments.items():
            for keyword, value in vectors[tweet_id].items():
                new_centroids[cluster_id][keyword] = new_centroids[cluster_id].get(keyword, 0) + value

        centroids = new_centroids

        if (iteration + 1) % 3 == 0:
            print(f"  Iteration {iteration + 1}/{max_iter}")

    # Create cluster array matching tweet_ids order
    clusters = [assignments[tid] for tid in tweet_ids]

    print(f"✓ Clustering complete\n")
    return clusters, assignments


def analyze_clusters(dataset: Dict, tweet_ids: List[str], clusters: List[int], click_weights: Dict[str, float]):
    """Analyze cluster structure."""

    print("=" * 100)
    print("EMERGENT CLUSTERS FROM YOUR ENGAGEMENT DATA")
    print("=" * 100)

    cluster_info = defaultdict(lambda: {
        "tweets": [],
        "weight": 0.0,
        "top_authors": defaultdict(float),
        "top_keywords": defaultdict(int),
    })

    # Aggregate by cluster
    for tweet_id, cluster_id in zip(tweet_ids, clusters):
        tweet = dataset[tweet_id]
        weight = click_weights.get(tweet_id, 1.0)

        cluster_info[cluster_id]["tweets"].append(tweet_id)
        cluster_info[cluster_id]["weight"] += weight

        author = tweet.get("author_screen_name", "unknown")
        cluster_info[cluster_id]["top_authors"][author] += weight

        text_lower = tweet.get("text", "").lower()
        for kw in ["token", "llm", "gpt", "validator", "exploit", "btc", "confidence", "analysis", "hype", "launch", "warning"]:
            if kw in text_lower:
                cluster_info[cluster_id]["top_keywords"][kw] += 1

    # Print clusters by engagement
    total_weight = sum(ci["weight"] for ci in cluster_info.values())

    for cluster_id in sorted(cluster_info.keys(), key=lambda c: -cluster_info[c]["weight"]):
        info = cluster_info[cluster_id]
        pct = 100 * info["weight"] / total_weight

        print(f"\n{'─' * 100}")
        print(f"CLUSTER {cluster_id}: {pct:.1f}% engagement ({info['weight']:.0f} total weight)")
        print(f"{'─' * 100}")
        print(f"  Tweets: {len(info['tweets'])}")

        # Top authors
        top_authors = sorted(info["top_authors"].items(), key=lambda x: -x[1])[:5]
        author_str = ", ".join([f"@{a} ({w:.0f})" for a, w in top_authors])
        print(f"  Top authors: {author_str}")

        # Top keywords
        top_keywords = sorted(info["top_keywords"].items(), key=lambda x: -x[1])[:8]
        keyword_str = ", ".join([f"{k} ({c})" for k, c in top_keywords])
        print(f"  Keywords: {keyword_str}")

        # Example tweets
        print(f"  Example tweets:")
        for twid in info["tweets"][:3]:
            text = dataset[twid].get("text", "")[:100]
            print(f"    • {text}...")

    # Summary
    print(f"\n{'=' * 100}")
    print("SUMMARY")
    print(f"{'=' * 100}\n")
    print(f"Total clusters: {len(cluster_info)}")
    print(f"Engagement distribution (% of total clicks):")
    for cluster_id in sorted(cluster_info.keys(), key=lambda c: -cluster_info[c]["weight"]):
        pct = 100 * cluster_info[cluster_id]["weight"] / total_weight
        print(f"  Cluster {cluster_id}: {pct:>5.1f}%  ({cluster_info[cluster_id]['weight']:.0f} weight, {len(cluster_info[cluster_id]['tweets'])} tweets)")

    # Save results
    print(f"\n{'=' * 100}")
    print("SAVING RESULTS")
    print(f"{'=' * 100}\n")

    results = {
        "n_clusters": 7,
        "n_tweets": len(dataset),
        "cluster_info": {
            str(k): {
                "weight": v["weight"],
                "percent": 100 * v["weight"] / total_weight,
                "tweet_count": len(v["tweets"]),
                "top_authors": list(sorted(v["top_authors"].items(), key=lambda x: -x[1])[:5]),
                "top_keywords": list(sorted(v["top_keywords"].items(), key=lambda x: -x[1])[:8]),
            }
            for k, v in cluster_info.items()
        }
    }

    with open("cynic-python/emergent_clusters.json", "w") as f:
        json.dump(results, f, indent=2)

    print(f"✓ Saved to cynic-python/emergent_clusters.json\n")

    return cluster_info


def main():
    dataset, clicks, captures_by_time = load_data()
    click_weights = compute_click_weights(clicks, captures_by_time, dataset)

    # Build vocabulary and vectors
    print("[VECTORIZE] Building vocabulary and tweet vectors...")
    vocab = build_vocabulary(dataset)
    print(f"  Vocabulary: {len(vocab)} keywords")

    vectors = vectorize_tweets(dataset, vocab)
    print(f"  Vectors: {len(vectors)} tweets\n")

    # Cluster
    tweet_ids = list(dataset.keys())
    clusters, assignments = simple_kmeans(tweet_ids, vectors, click_weights, n_clusters=7, max_iter=15)

    # Analyze
    analyze_clusters(dataset, tweet_ids, clusters, click_weights)


if __name__ == "__main__":
    main()
