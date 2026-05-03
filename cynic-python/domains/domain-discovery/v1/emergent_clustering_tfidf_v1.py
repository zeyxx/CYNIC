#!/usr/bin/env python3
"""
Emergent Domain Discovery v1 (TF-IDF + K-Means) — PHASE 1 LOCKED

🔒 IMMUTABLE UNTIL PHASE 3 MEASUREMENT
   Consumer: K15 router (Phase 2 kernel integration)
   Falsification: If Phase 3 signal yield < 2% improvement, consider alternative

Upgrade from binary keywords to TF-IDF vectorization.
TF-IDF captures term frequency + inverse document frequency.
Solves: "gpt mentioned once" vs "gpt discussed 50×" are different semantic signals.
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


def build_vocabulary_tfidf(dataset: Dict) -> Tuple[Dict[str, int], Dict[str, float]]:
    """
    Build vocabulary and compute IDF (inverse document frequency).

    Returns:
        vocab: {word: word_index}
        idf: {word: idf_weight}
    """
    vocab = {}
    word_doc_count = defaultdict(int)
    total_docs = len(dataset)

    # Build vocabulary and count documents containing each word
    for tweet in dataset.values():
        text_lower = tweet.get("text", "").lower()
        words_seen = set()

        # Simple tokenization: split on whitespace and punctuation
        for word in text_lower.split():
            word = word.strip('.,!?;:()[]{}"\'-')
            if len(word) > 2:  # Skip very short words
                if word not in vocab:
                    vocab[word] = len(vocab)
                words_seen.add(word)

        # Count document frequency
        for word in words_seen:
            word_doc_count[word] += 1

    # Compute IDF for each word
    idf = {}
    for word in vocab:
        doc_count = word_doc_count.get(word, 1)
        idf[word] = math.log(total_docs / doc_count)

    print(f"  Vocabulary size: {len(vocab)} words")
    print(f"  IDF weights range: {min(idf.values()):.2f} to {max(idf.values()):.2f}\n")

    return vocab, idf


def vectorize_tweets_tfidf(dataset: Dict, vocab: Dict[str, int], idf: Dict[str, float]) -> Dict[str, Dict[int, float]]:
    """
    Create TF-IDF vectors for tweets.

    Returns:
        vectors: {tweet_id: {word_index: tfidf_weight}}
    """
    vectors = {}

    # Create reverse mapping once (much faster than repeated list comprehension)
    idx_to_word = {idx: word for word, idx in vocab.items()}
    processed = 0

    for tweet_id, tweet in dataset.items():
        text_lower = tweet.get("text", "").lower()

        # Count term frequency
        tf = {}
        word_count = 0
        for word in text_lower.split():
            word = word.strip('.,!?;:()[]{}"\'-')
            if len(word) > 2 and word in vocab:
                word_idx = vocab[word]
                tf[word_idx] = tf.get(word_idx, 0) + 1
                word_count += 1

        # Normalize TF and apply IDF
        vector = {}
        if word_count > 0:
            for word_idx, tf_val in tf.items():
                norm_tf = tf_val / word_count  # Normalize TF
                word = idx_to_word[word_idx]
                vector[word_idx] = norm_tf * idf.get(word, 0)

        vectors[tweet_id] = vector
        processed += 1
        if processed % 1000 == 0:
            print(f"  Vectorized {processed} tweets...")

    return vectors


def cosine_similarity(v1: Dict[int, float], v2: Dict[int, float]) -> float:
    """Compute cosine similarity between two TF-IDF vectors."""
    if not v1 or not v2:
        return 0.0

    # Get all indices present in either vector
    all_indices = set(v1.keys()) | set(v2.keys())

    # Compute dot product
    dot = sum(v1.get(idx, 0) * v2.get(idx, 0) for idx in all_indices)

    # Compute magnitudes
    mag1 = math.sqrt(sum(v**2 for v in v1.values()))
    mag2 = math.sqrt(sum(v**2 for v in v2.values()))

    if mag1 == 0 or mag2 == 0:
        return 0.0

    return dot / (mag1 * mag2)


def simple_kmeans(tweet_ids: List[str], vectors: Dict[str, Dict[int, float]], click_weights: Dict[str, float],
                  n_clusters: int = 7, max_iter: int = 15):
    """Simple k-means clustering on TF-IDF vectors."""

    print(f"[CLUSTERING] Running k-means ({n_clusters} clusters) on TF-IDF vectors...")

    # Initialize centroids randomly
    import random
    random.seed(42)
    centroid_ids = random.sample(tweet_ids, n_clusters)
    centroids = {i: vectors[cid] for i, cid in enumerate(centroid_ids)}
    print(f"  Initialized {n_clusters} centroids\n")

    for iteration in range(max_iter):
        print(f"  Iteration {iteration + 1}/{max_iter}: assigning tweets...", end=" ", flush=True)

        # Assign tweets to nearest centroid
        assignments = {}
        for idx, tweet_id in enumerate(tweet_ids):
            if idx % 1000 == 0:
                print(f"({idx}/{len(tweet_ids)})", end=" ", flush=True)
            vec = vectors[tweet_id]
            best_cluster = 0
            best_sim = -1

            for cluster_id, centroid in centroids.items():
                sim = cosine_similarity(vec, centroid)
                if sim > best_sim:
                    best_sim = sim
                    best_cluster = cluster_id

            assignments[tweet_id] = best_cluster

        print("updating centroids...", flush=True)

        # Update centroids (simple averaging of TF-IDF vectors)
        new_centroids = {i: {} for i in range(n_clusters)}
        for tweet_id, cluster_id in assignments.items():
            vec = vectors[tweet_id]
            for word_idx, val in vec.items():
                if word_idx not in new_centroids[cluster_id]:
                    new_centroids[cluster_id][word_idx] = 0
                new_centroids[cluster_id][word_idx] += val

        centroids = new_centroids

    # Create cluster array matching tweet_ids order
    clusters = [assignments[tid] for tid in tweet_ids]

    print(f"✓ Clustering complete\n")
    return clusters, assignments


def measure_silhouette(tweet_ids: List[str], vectors: Dict[str, Dict[int, float]], assignments: Dict[str, int]) -> float:
    """
    Measure silhouette coefficient (cluster quality metric).
    Range: -1 to +1 (higher is better, >0.5 is good)
    """
    print("[QUALITY] Measuring silhouette coefficient...", flush=True)
    silhouette_scores = []
    sample_size = min(50, len(tweet_ids))  # Sample only 50 tweets for speed

    for idx, tweet_id in enumerate(tweet_ids[:sample_size]):  # Sample tweets for speed
        if idx % 10 == 0:
            print(f"  Silhouette: {idx}/{sample_size}", flush=True)
        vec = vectors[tweet_id]
        cluster_id = assignments[tweet_id]

        # a(i) = average distance to other points in same cluster
        cluster_tweets = [t for t in tweet_ids if assignments[t] == cluster_id]
        if len(cluster_tweets) > 1:
            a_i = sum(1 - cosine_similarity(vec, vectors[t]) for t in cluster_tweets if t != tweet_id) / (len(cluster_tweets) - 1)
        else:
            a_i = 0

        # b(i) = min average distance to points in other clusters
        b_i_values = []
        for other_cluster in set(assignments.values()):
            if other_cluster != cluster_id:
                other_tweets = [t for t in tweet_ids if assignments[t] == other_cluster]
                if other_tweets:
                    avg_dist = sum(1 - cosine_similarity(vec, vectors[t]) for t in other_tweets) / len(other_tweets)
                    b_i_values.append(avg_dist)

        b_i = min(b_i_values) if b_i_values else 0

        # s(i) = (b(i) - a(i)) / max(a(i), b(i))
        if max(a_i, b_i) > 0:
            s_i = (b_i - a_i) / max(a_i, b_i)
        else:
            s_i = 0

        silhouette_scores.append(s_i)

    avg_silhouette = sum(silhouette_scores) / len(silhouette_scores) if silhouette_scores else 0
    return avg_silhouette


def analyze_clusters(dataset: Dict, tweet_ids: List[str], clusters: List[int], click_weights: Dict[str, float], vocab: Dict[str, int], silhouette: float = 0.0):
    """Analyze cluster structure and quality."""

    print("=" * 100)
    print("EMERGENT CLUSTERS (TF-IDF) FROM YOUR ENGAGEMENT DATA")
    print("=" * 100)

    cluster_info = defaultdict(lambda: {
        "tweets": [],
        "weight": 0.0,
        "top_authors": defaultdict(float),
        "top_words": defaultdict(int),
    })

    # Aggregate by cluster
    for tweet_id, cluster_id in zip(tweet_ids, clusters):
        tweet = dataset[tweet_id]
        weight = click_weights.get(tweet_id, 1.0)

        cluster_info[cluster_id]["tweets"].append(tweet_id)
        cluster_info[cluster_id]["weight"] += weight

        author = tweet.get("author_screen_name", "unknown")
        cluster_info[cluster_id]["top_authors"][author] += weight

        # Extract words from tweet
        text_lower = tweet.get("text", "").lower()
        for word_idx, word in [(idx, w) for w, idx in vocab.items()]:
            if word in text_lower:
                cluster_info[cluster_id]["top_words"][word] += 1

    # Print clusters by engagement
    total_weight = sum(ci["weight"] for ci in cluster_info.values())

    for cluster_id in sorted(cluster_info.keys(), key=lambda c: -cluster_info[c]["weight"]):
        info = cluster_info[cluster_id]
        pct = 100 * info["weight"] / total_weight if total_weight > 0 else 0

        print(f"\n{'─' * 100}")
        print(f"CLUSTER {cluster_id}: {pct:.1f}% engagement ({info['weight']:.0f} total weight)")
        print(f"{'─' * 100}")
        print(f"  Tweets: {len(info['tweets'])}")

        # Top authors
        top_authors = sorted(info["top_authors"].items(), key=lambda x: -x[1])[:5]
        author_str = ", ".join([f"@{a} ({w:.0f})" for a, w in top_authors])
        print(f"  Top authors: {author_str}")

        # Top words
        top_words = sorted(info["top_words"].items(), key=lambda x: -x[1])[:10]
        word_str = ", ".join([f"{w} ({c})" for w, c in top_words])
        print(f"  Top words: {word_str}")

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
        pct = 100 * cluster_info[cluster_id]["weight"] / total_weight if total_weight > 0 else 0
        print(f"  Cluster {cluster_id}: {pct:>5.1f}%  ({cluster_info[cluster_id]['weight']:.0f} weight, {len(cluster_info[cluster_id]['tweets'])} tweets)")

    # Save results
    print(f"\n{'=' * 100}")
    print("SAVING RESULTS")
    print(f"{'=' * 100}\n")

    results = {
        "method": "TF-IDF + K-Means",
        "n_clusters": len(cluster_info),
        "n_tweets": len(dataset),
        "silhouette": silhouette,  # Quality metric (target > 0.5)
        "cluster_info": {
            str(k): {
                "weight": v["weight"],
                "percent": 100 * v["weight"] / total_weight if total_weight > 0 else 0,
                "tweet_count": len(v["tweets"]),
                "top_authors": list(sorted(v["top_authors"].items(), key=lambda x: -x[1])[:5]),
                "top_words": list(sorted(v["top_words"].items(), key=lambda x: -x[1])[:10]),
            }
            for k, v in cluster_info.items()
        },
        "assignments": assignments  # Save cluster assignments for bridge detection
    }

    with open("emergent_clusters_tfidf.json", "w") as f:
        json.dump(results, f, indent=2)

    # Save assignments separately (for behavioral grounding)
    with open("results_v1/assignments_v1.json", "w") as f:
        json.dump(assignments, f, indent=2)

    # Save metrics separately
    metrics = {
        "silhouette": silhouette,
        "n_clusters": len(cluster_info),
        "n_tweets": len(dataset),
        "vocabulary_size": len(set(w for tweet in dataset.values() for w in tweet.get("text", "").lower().split())),
        "timestamp": str(__import__("datetime").datetime.now().isoformat()),
    }
    with open("results_v1/metrics_v1.json", "w") as f:
        json.dump(metrics, f, indent=2)

    print(f"✓ Saved to emergent_clusters_tfidf.json")
    print(f"✓ Saved metrics to results_v1/metrics_v1.json\n")

    return cluster_info


def main():
    dataset, clicks, captures_by_time = load_data()
    click_weights = compute_click_weights(clicks, captures_by_time, dataset)

    # Build vocabulary and IDF
    print("[VECTORIZE] Building vocabulary and computing IDF...")
    vocab, idf = build_vocabulary_tfidf(dataset)

    # Vectorize tweets using TF-IDF
    print("[VECTORIZE] Computing TF-IDF vectors...")
    vectors = vectorize_tweets_tfidf(dataset, vocab, idf)
    print(f"  TF-IDF vectors: {len(vectors)} tweets\n")

    # Cluster
    tweet_ids = list(dataset.keys())
    clusters, assignments = simple_kmeans(tweet_ids, vectors, click_weights, n_clusters=7, max_iter=15)

    # Measure silhouette
    print("[QUALITY] Measuring silhouette coefficient...")
    silhouette = measure_silhouette(tweet_ids, vectors, assignments)
    print(f"  Silhouette score: {silhouette:.3f} (target > 0.5)\n")

    # Analyze (pass silhouette for saving)
    analyze_clusters(dataset, tweet_ids, clusters, click_weights, vocab, silhouette)


if __name__ == "__main__":
    main()
