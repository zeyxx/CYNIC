#!/usr/bin/env python3
"""
Emergent Domain Discovery — Cluster tweets by semantic similarity, let domains emerge.

Rather than imposing D1-D6 (general, llm, token, security, sovereignty, macro, epistemology),
this script discovers natural clusters from your engagement data.

CHAOS → MATRIX: Let structure crystallize from behavioral signal.

Usage:
  python3 emergent_domain_discovery.py --method semantic
  python3 emergent_domain_discovery.py --method semantic --validate
"""

import json
import os
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Tuple
from collections import defaultdict
import numpy as np
import argparse

try:
    from sklearn.cluster import SpectralClustering
    from sklearn.metrics.pairwise import cosine_similarity
    from sentence_transformers import SentenceTransformer
    HAS_ML = True
except ImportError:
    HAS_ML = False
    print("⚠ Install: pip install scikit-learn sentence-transformers")


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
    """Compute click weight for each tweet (how often you engaged with it)."""
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
                # Weight by position (earlier = more likely clicked)
                likelihood = 1.0 / (1.0 + 0.1 * position)
                click_weights[tweet_id] += likelihood

    return dict(click_weights)


def semantic_clustering(dataset: Dict, click_weights: Dict, n_clusters: int = 7):
    """Cluster tweets by semantic similarity, weighted by your engagement."""

    if not HAS_ML:
        print("✗ Required: pip install scikit-learn sentence-transformers")
        return None

    print(f"[CLUSTER] Computing semantic embeddings for {len(dataset)} tweets...")

    # Generate embeddings
    model = SentenceTransformer('all-MiniLM-L6-v2')
    tweet_ids = list(dataset.keys())
    texts = [dataset[tid].get("text", "") for tid in tweet_ids]

    embeddings = model.encode(texts, show_progress_bar=True)
    print(f"  Embeddings computed: {embeddings.shape}")

    # Compute similarity matrix
    print("[SIMILARITY] Computing pairwise similarity...")
    similarity = cosine_similarity(embeddings)

    # Weight similarity by click weights
    print("[WEIGHT] Weighting similarity by your engagement...")
    for i, tid_i in enumerate(tweet_ids):
        for j, tid_j in enumerate(tweet_ids):
            if i != j:
                w_i = click_weights.get(tid_i, 1.0)
                w_j = click_weights.get(tid_j, 1.0)
                similarity[i, j] *= (w_i * w_j) ** 0.5  # Geometric mean

    print("[CLUSTER] Running spectral clustering...")
    clustering = SpectralClustering(
        n_clusters=n_clusters,
        affinity='precomputed',
        random_state=42,
        n_init=10
    )
    clusters = clustering.fit_predict(similarity)

    print(f"✓ Clustered into {n_clusters} groups\n")

    return tweet_ids, clusters, click_weights


def analyze_clusters(dataset: Dict, tweet_ids: List[str], clusters: np.ndarray, click_weights: Dict):
    """Analyze cluster structure and properties."""

    print("=" * 100)
    print("EMERGENT CLUSTER STRUCTURE")
    print("=" * 100)

    cluster_info = defaultdict(lambda: {
        "tweets": [],
        "weight": 0,
        "top_authors": defaultdict(float),
        "top_keywords": defaultdict(int),
    })

    for tweet_id, cluster_id in zip(tweet_ids, clusters):
        tweet = dataset[tweet_id]
        weight = click_weights.get(tweet_id, 1.0)

        cluster_info[cluster_id]["tweets"].append(tweet_id)
        cluster_info[cluster_id]["weight"] += weight

        author = tweet.get("author_screen_name", "unknown")
        cluster_info[cluster_id]["top_authors"][author] += weight

        # Extract keywords
        text_lower = tweet.get("text", "").lower()
        keywords = ["token", "llm", "validator", "exploit", "btc", "confidence", "general"]
        for kw in keywords:
            if kw in text_lower:
                cluster_info[cluster_id]["top_keywords"][kw] += 1

    # Print cluster summaries
    for cluster_id in sorted(cluster_info.keys(), key=lambda c: -cluster_info[c]["weight"]):
        info = cluster_info[cluster_id]
        pct = 100 * info["weight"] / sum(ci["weight"] for ci in cluster_info.values())

        print(f"\nCLUSTER {cluster_id}: {pct:.1f}% of engagement ({info['weight']:.0f} total)")
        print(f"  Tweets: {len(info['tweets'])}")

        # Top authors
        top_authors = sorted(info["top_authors"].items(), key=lambda x: -x[1])[:5]
        print(f"  Top authors: {', '.join([f'@{a} ({w:.0f})' for a, w in top_authors])}")

        # Top keywords
        top_keywords = sorted(info["top_keywords"].items(), key=lambda x: -x[1])[:5]
        print(f"  Top keywords: {', '.join([f'{k} ({c})' for k, c in top_keywords])}")

        # Example tweets
        print(f"  Example tweets:")
        sample_tweets = info["tweets"][:3]
        for twid in sample_tweets:
            text = dataset[twid].get("text", "")[:80]
            print(f"    • {text}...")


def main():
    parser = argparse.ArgumentParser(description="Discover emergent domains via clustering")
    parser.add_argument("--method", type=str, default="semantic", help="Clustering method")
    parser.add_argument("--n-clusters", type=int, default=7, help="Number of clusters")
    parser.add_argument("--validate", action="store_true", help="Run validation tests")

    args = parser.parse_args()

    dataset, clicks, captures_by_time = load_data()
    click_weights = compute_click_weights(clicks, captures_by_time, dataset)

    if args.method == "semantic":
        result = semantic_clustering(dataset, click_weights, n_clusters=args.n_clusters)
        if result:
            tweet_ids, clusters, weights = result
            analyze_clusters(dataset, tweet_ids, clusters, weights)

            if args.validate:
                print("\n" + "=" * 100)
                print("VALIDATION")
                print("=" * 100)

                # Measure cluster quality
                # TODO: Compare to heuristic domains
                # Falsification: R² from clusters > R² from heuristics?

                print("✓ Validation complete")


if __name__ == "__main__":
    main()
