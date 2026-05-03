#!/usr/bin/env python3
"""
Emergent Domain Discovery v2 — TF-IDF + K-Means with Stopword Filtering.

Upgrade from v1: Remove common stopwords (the, and, you, etc) + clean URL fragments.

Domain: domain-discovery
Version: v2
Purpose: Improved semantic clustering with cleaner vocabulary.

Entry point: python3 emergent_clustering_tfidf_v2.py
Output: emergent_clusters_tfidf_v2.json + results_v2/metrics_v2.json
"""

__version__ = "2.0.0"

import json
import os
import math
import re
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Set
from collections import defaultdict


# Common English stopwords + observed junk
STOPWORDS = {
    "the", "and", "you", "for", "all", "this", "that", "your", "our", "is", "are",
    "was", "were", "have", "has", "had", "be", "been", "being", "do", "does", "did",
    "will", "would", "could", "should", "may", "might", "must", "can", "shall",
    "a", "an", "in", "on", "at", "to", "from", "of", "with", "by", "about",
    "as", "into", "through", "during", "before", "after", "above", "below",
    "up", "down", "out", "off", "over", "under", "again", "further", "then", "once",
    "here", "there", "when", "where", "why", "how", "which", "who", "what",
    "it", "its", "i", "me", "him", "her", "us", "them", "myself", "yourself",
    "himself", "herself", "itself", "ourselves", "yourselves", "themselves",
    # Observed junk in dataset
    "http", "https", "tps", "amp", "href", "gt", "lt", "nbsp", "et",
    "url", "link", "follow", "check", "read", "see", "thread", "tweet",
    # Empty/meaningless
    "", "a", "rt", "via"
}


def clean_text(text: str) -> str:
    """Clean tweet text: remove URLs, mentions, special chars."""
    # Remove URLs
    text = re.sub(r'http\S+', '', text)
    text = re.sub(r'www\S+', '', text)
    # Remove mentions
    text = re.sub(r'@\w+', '', text)
    # Remove hashtags (keep content)
    text = re.sub(r'#(\w+)', r'\1', text)
    # Remove HTML entities
    text = text.replace('&amp;', 'and').replace('&lt;', '<').replace('&gt;', '>')
    # Remove excess whitespace
    text = ' '.join(text.split())
    return text


def tokenize_and_filter(text: str, stopwords: Set[str]) -> List[str]:
    """Tokenize text and remove stopwords."""
    tokens = text.lower().split()
    return [t for t in tokens if t not in stopwords and len(t) > 2]


def load_data():
    """Load dataset and behavioral data."""
    print("[LOAD] Reading datasets...")

    dataset = {}
    with open(os.path.expanduser("~/.cynic/organs/hermes/x/dataset.jsonl")) as f:
        for line in f:
            tweet = json.loads(line)
            # Clean tweet text upfront
            tweet["text"] = clean_text(tweet.get("text", ""))
            dataset[tweet["tweet_id"]] = tweet

    behavior_log = []
    with open(os.path.expanduser("~/.cynic/organs/hermes/behavior/behavior_log.jsonl")) as f:
        for line in f:
            behavior_log.append(json.loads(line))

    clicks = [e for e in behavior_log if e.get("type") == "click"]

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


def compute_click_weights(clicks: List[Dict], captures_by_time: Dict, dataset: Dict) -> Dict[str, float]:
    """Compute click weight for each tweet."""
    from collections import defaultdict
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

        capture = captures_by_time[nearest_ts]
        tweets = extract_tweets_from_capture(capture["data"])

        for tweet_id, pos in tweets:
            if tweet_id in dataset:
                # Weight by inverse position (top tweets = higher weight)
                position_weight = 1.0 / (1.0 + 0.1 * pos)
                click_weights[tweet_id] += position_weight

    return dict(click_weights)


def extract_tweets_from_capture(capture_data: Dict) -> List[tuple]:
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


def build_vocabulary_tfidf(dataset: Dict) -> tuple:
    """Build vocabulary and compute IDF weights, filtering stopwords."""
    print("[VOCAB] Building filtered vocabulary...")
    doc_freq = defaultdict(int)
    vocab = {}
    word_idx = 0

    # Count document frequency
    for tweet in dataset.values():
        text = tweet.get("text", "")
        words_seen = set(tokenize_and_filter(text, STOPWORDS))
        for word in words_seen:
            doc_freq[word] += 1

    # Assign indices and compute IDF
    idf = {}
    n_docs = len(dataset)
    for word, freq in sorted(doc_freq.items(), key=lambda x: -x[1]):
        if freq >= 2:  # Require word in at least 2 tweets
            vocab[word] = word_idx
            idf[word_idx] = math.log(n_docs / freq)
            word_idx += 1

    print(f"  Vocabulary size: {len(vocab)} words (stopwords filtered)")
    print(f"  IDF range: {min(idf.values()):.2f} to {max(idf.values()):.2f}\n")
    return vocab, idf


def vectorize_tweets_tfidf(dataset: Dict, vocab: Dict, idf: Dict) -> Dict:
    """Vectorize tweets using TF-IDF (filtered)."""
    print("[VECTORIZE] Computing TF-IDF vectors (filtered)...")
    vectors = {}

    for idx, (tweet_id, tweet) in enumerate(dataset.items()):
        if idx % 1000 == 0:
            print(f"  Vectorized {idx} tweets...", flush=True)

        text = tweet.get("text", "")
        tokens = tokenize_and_filter(text, STOPWORDS)
        token_counts = defaultdict(int)
        for token in tokens:
            if token in vocab:
                token_counts[token] += 1

        # TF-IDF: (count / doc_length) * IDF
        vector = {}
        for word, count in token_counts.items():
            word_idx = vocab[word]
            tf = count / len(tokens) if tokens else 0
            vector[word_idx] = tf * idf[word_idx]

        vectors[tweet_id] = vector

    print(f"  TF-IDF vectors: {len(vectors)} tweets\n")
    return vectors


def cosine_similarity(vec_a: Dict[int, float], vec_b: Dict[int, float]) -> float:
    """Compute cosine similarity between two sparse vectors."""
    dot_product = sum(vec_a.get(idx, 0) * vec_b.get(idx, 0) for idx in set(vec_a) | set(vec_b))
    norm_a = sum(v**2 for v in vec_a.values()) ** 0.5
    norm_b = sum(v**2 for v in vec_b.values()) ** 0.5
    if norm_a == 0 or norm_b == 0:
        return 0
    return dot_product / (norm_a * norm_b)


def simple_kmeans(tweet_ids: List[str], vectors: Dict, weights: Dict, n_clusters: int, max_iter: int = 15) -> tuple:
    """Simple K-means clustering on TF-IDF vectors."""
    print(f"[CLUSTERING] Running k-means ({n_clusters} clusters) on TF-IDF vectors...")

    import random
    random.seed(42)

    # Initialize centroids randomly
    initial_ids = random.sample(tweet_ids, min(n_clusters, len(tweet_ids)))
    centroids = {i: vectors[tid] for i, tid in enumerate(initial_ids)}
    print(f"  Initialized {n_clusters} centroids\n")

    for iteration in range(max_iter):
        print(f"  Iteration {iteration+1}/{max_iter}: assigning tweets...", end="", flush=True)

        # Assign tweets to nearest centroid
        assignments = {}
        cluster_tweets = {i: [] for i in range(n_clusters)}

        for idx, tweet_id in enumerate(tweet_ids):
            if idx % 1000 == 0 and idx > 0:
                print(f" ({idx}/{len(tweet_ids)})", end="", flush=True)
            vec = vectors[tweet_id]
            nearest_cluster = min(
                range(n_clusters),
                key=lambda c: 1 - cosine_similarity(vec, centroids[c])
            )
            assignments[tweet_id] = nearest_cluster
            cluster_tweets[nearest_cluster].append(tweet_id)

        print(" updating centroids...", flush=True)

        # Update centroids
        for cluster_id in range(n_clusters):
            if cluster_tweets[cluster_id]:
                new_centroid = {}
                for tweet_id in cluster_tweets[cluster_id]:
                    for word_idx, val in vectors[tweet_id].items():
                        new_centroid[word_idx] = new_centroid.get(word_idx, 0) + val
                for word_idx in new_centroid:
                    new_centroid[word_idx] /= len(cluster_tweets[cluster_id])
                centroids[cluster_id] = new_centroid

    clusters = [assignments.get(tid, 0) for tid in tweet_ids]
    return clusters, assignments


def measure_silhouette(tweet_ids: List[str], vectors: Dict[str, Dict[int, float]], assignments: Dict[str, int]) -> float:
    """Measure silhouette coefficient (cluster quality metric)."""
    print("[QUALITY] Measuring silhouette coefficient...", flush=True)
    silhouette_scores = []
    sample_size = min(50, len(tweet_ids))

    for idx, tweet_id in enumerate(tweet_ids[:sample_size]):
        if idx % 10 == 0:
            print(f"  Silhouette: {idx}/{sample_size}", flush=True)
        vec = vectors[tweet_id]
        cluster_id = assignments[tweet_id]

        cluster_tweets = [t for t in tweet_ids if assignments[t] == cluster_id]
        if len(cluster_tweets) > 1:
            a_i = sum(1 - cosine_similarity(vec, vectors[t]) for t in cluster_tweets if t != tweet_id) / (len(cluster_tweets) - 1)
        else:
            a_i = 0

        b_i_values = []
        for other_cluster in set(assignments.values()):
            if other_cluster != cluster_id:
                other_tweets = [t for t in tweet_ids if assignments[t] == other_cluster]
                if other_tweets:
                    avg_dist = sum(1 - cosine_similarity(vec, vectors[t]) for t in other_tweets) / len(other_tweets)
                    b_i_values.append(avg_dist)

        b_i = min(b_i_values) if b_i_values else 0
        if max(a_i, b_i) > 0:
            s_i = (b_i - a_i) / max(a_i, b_i)
        else:
            s_i = 0

        silhouette_scores.append(s_i)

    avg_silhouette = sum(silhouette_scores) / len(silhouette_scores) if silhouette_scores else 0
    return avg_silhouette


def analyze_clusters(dataset: Dict, tweet_ids: List[str], clusters: List[int], click_weights: Dict, vocab: Dict, silhouette: float):
    """Analyze cluster structure and save results."""
    print("=" * 100)
    print("EMERGENT CLUSTERS (TF-IDF v2 — STOPWORD FILTERED)")
    print("=" * 100)

    cluster_info = defaultdict(lambda: {
        "tweets": [],
        "weight": 0.0,
        "top_authors": defaultdict(float),
        "top_words": defaultdict(int),
    })

    for tweet_id, cluster_id in zip(tweet_ids, clusters):
        tweet = dataset[tweet_id]
        weight = click_weights.get(tweet_id, 1.0)

        cluster_info[cluster_id]["tweets"].append(tweet_id)
        cluster_info[cluster_id]["weight"] += weight

        author = tweet.get("author_screen_name", "unknown")
        cluster_info[cluster_id]["top_authors"][author] += weight

        text = tweet.get("text", "")
        tokens = tokenize_and_filter(text, STOPWORDS)
        for token in tokens:
            if token in vocab:
                cluster_info[cluster_id]["top_words"][token] += 1

    total_weight = sum(ci["weight"] for ci in cluster_info.values())

    for cluster_id in sorted(cluster_info.keys(), key=lambda k: -cluster_info[k]["weight"]):
        ci = cluster_info[cluster_id]
        percent = 100 * ci["weight"] / total_weight if total_weight > 0 else 0
        top_words = sorted(ci["top_words"].items(), key=lambda x: -x[1])[:5]
        print(f"\nCluster {cluster_id} ({percent:.1f}% engagement, {len(ci['tweets'])} tweets)")
        print(f"  Top words: {[w[0] for w in top_words]}")

    results = {
        "method": "TF-IDF + K-Means (stopword-filtered)",
        "n_clusters": len(cluster_info),
        "n_tweets": len(dataset),
        "silhouette": silhouette,
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
    }

    Path("results_v2").mkdir(exist_ok=True)
    with open("results_v2/emergent_clusters_tfidf_v2.json", "w") as f:
        json.dump(results, f, indent=2)

    metrics = {
        "silhouette": silhouette,
        "n_clusters": len(cluster_info),
        "n_tweets": len(dataset),
        "vocabulary_size": len(vocab),
        "stopwords_removed": len(STOPWORDS),
        "timestamp": str(datetime.now().isoformat()),
    }
    with open("results_v2/metrics_v2.json", "w") as f:
        json.dump(metrics, f, indent=2)

    print(f"\n✓ Saved to results_v2/emergent_clusters_tfidf_v2.json")
    print(f"✓ Saved metrics to results_v2/metrics_v2.json\n")


def main():
    dataset, clicks, captures_by_time = load_data()
    click_weights = compute_click_weights(clicks, captures_by_time, dataset)

    vocab, idf = build_vocabulary_tfidf(dataset)
    vectors = vectorize_tweets_tfidf(dataset, vocab, idf)

    tweet_ids = list(dataset.keys())
    clusters, assignments = simple_kmeans(tweet_ids, vectors, click_weights, n_clusters=7, max_iter=15)

    print("[QUALITY] Measuring silhouette coefficient...")
    silhouette = measure_silhouette(tweet_ids, vectors, assignments)
    print(f"  Silhouette score: {silhouette:.3f} (target > 0.5)\n")

    analyze_clusters(dataset, tweet_ids, clusters, click_weights, vocab, silhouette)


if __name__ == "__main__":
    main()
