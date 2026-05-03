#!/usr/bin/env python3
"""
Emergent Domain Discovery (Lite) — Cluster tweets using TF-IDF + Spectral Clustering.

Lighter version that doesn't require sentence-transformers.
Uses TF-IDF vectors for semantic similarity instead of transformer embeddings.

Usage:
  python3 emergent_domain_discovery_lite.py
"""

import json
import os
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Tuple
from collections import defaultdict
import numpy as np

from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.cluster import SpectralClustering
from sklearn.metrics.pairwise import cosine_similarity


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
        visible_tweets = extract_tweets_from_capture(capture["data"])

        if not visible_tweets:
            continue

        for position, (tweet_id, pos) in enumerate(visible_tweets):
            if tweet_id in dataset:
                likelihood = 1.0 / (1.0 + 0.1 * position)
                click_weights[tweet_id] += likelihood

    return dict(click_weights)


def tfidf_clustering(dataset: Dict, click_weights: Dict, n_clusters: int = 7):
    """Cluster tweets using TF-IDF + Spectral Clustering."""

    print("[TF-IDF] Computing text vectors...")

    tweet_ids = list(dataset.keys())
    texts = [dataset[tid].get("text", "") for tid in tweet_ids]

    # TF-IDF vectorization
    vectorizer = TfidfVectorizer(
        max_features=500,
        min_df=2,
        max_df=0.8,
        stop_words='english',
        ngram_range=(1, 2)
    )
    tfidf_matrix = vectorizer.fit_transform(texts)
    print(f"  TF-IDF matrix: {tfidf_matrix.shape}")

    # Compute similarity
    print("[SIMILARITY] Computing pairwise cosine similarity...")
    similarity = cosine_similarity(tfidf_matrix)

    # Weight by clicks
    print("[WEIGHT] Weighting by engagement...")
    for i, tid_i in enumerate(tweet_ids):
        for j, tid_j in enumerate(tweet_ids):
            if i != j:
                w_i = click_weights.get(tid_i, 1.0)
                w_j = click_weights.get(tid_j, 1.0)
                similarity[i, j] *= (w_i * w_j) ** 0.5

    # Ensure positive semi-definite
    similarity = (similarity + similarity.T) / 2

    # Clustering
    print(f"[CLUSTER] Running spectral clustering ({n_clusters} clusters)...")
    clustering = SpectralClustering(
        n_clusters=n_clusters,
        affinity='precomputed',
        random_state=42,
        n_init=10
    )
    clusters = clustering.fit_predict(similarity)

    print(f"✓ Clustering complete\n")

    return tweet_ids, clusters, click_weights, similarity


def analyze_clusters(dataset: Dict, tweet_ids: List[str], clusters: np.ndarray, click_weights: Dict):
    """Analyze and display cluster structure."""

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
    print(f"Engagement distribution:")
    for cluster_id in sorted(cluster_info.keys(), key=lambda c: -cluster_info[c]["weight"]):
        pct = 100 * cluster_info[cluster_id]["weight"] / total_weight
        print(f"  Cluster {cluster_id}: {pct:>5.1f}%")

    return cluster_info


def detect_bridges(tweet_ids: List[str], clusters: np.ndarray, similarity: np.ndarray, dataset: Dict, click_weights: Dict, threshold: float = 0.5):
    """Detect bridge tweets (inter-cluster connections)."""

    print(f"\n{'=' * 100}")
    print("BRIDGE DETECTION (Multi-Domain Connections)")
    print(f"{'=' * 100}\n")

    bridges = defaultdict(list)
    bridge_count = defaultdict(int)

    for i, tweet_id_i in enumerate(tweet_ids):
        cluster_i = clusters[i]

        # Find high-similarity neighbors in OTHER clusters
        for j, tweet_id_j in enumerate(tweet_ids):
            if i >= j:  # Avoid duplicates
                continue

            cluster_j = clusters[j]
            if cluster_i == cluster_j:  # Skip same cluster
                continue

            sim = similarity[i, j]
            if sim > threshold:
                pair = tuple(sorted([cluster_i, cluster_j]))
                bridges[pair].append({
                    "tweet_1": tweet_id_i,
                    "tweet_2": tweet_id_j,
                    "similarity": sim,
                    "weight": (click_weights.get(tweet_id_i, 1.0) * click_weights.get(tweet_id_j, 1.0)) ** 0.5
                })
                bridge_count[pair] += 1

    # Print bridge report
    print(f"Inter-cluster bridges (similarity > {threshold}):\n")
    for (c1, c2) in sorted(bridge_count.keys(), key=lambda x: -bridge_count[x]):
        count = bridge_count[(c1, c2)]
        print(f"  Cluster {c1} ↔ Cluster {c2}: {count} bridge tweets")

        # Show example bridges
        examples = bridges[(c1, c2)][:2]
        for ex in examples:
            t1 = dataset[ex["tweet_1"]].get("text", "")[:60]
            print(f"    • Example: {t1}...")

    print()


def main():
    dataset, clicks, captures_by_time = load_data()
    click_weights = compute_click_weights(clicks, captures_by_time, dataset)

    # Run clustering
    tweet_ids, clusters, weights, similarity = tfidf_clustering(dataset, click_weights, n_clusters=7)

    # Analyze
    cluster_info = analyze_clusters(dataset, tweet_ids, clusters, click_weights)

    # Detect bridges
    detect_bridges(tweet_ids, clusters, similarity, dataset, click_weights, threshold=0.5)

    # Save results
    print(f"{'=' * 100}")
    print("SAVING RESULTS")
    print(f"{'=' * 100}\n")

    results = {
        "n_clusters": 7,
        "n_tweets": len(dataset),
        "cluster_info": {
            str(k): {
                "weight": v["weight"],
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


if __name__ == "__main__":
    main()
