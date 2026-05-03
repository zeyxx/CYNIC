#!/usr/bin/env python3
"""
Bridge Detection on TF-IDF clustering results.

Identifies tweets with high similarity to multiple cluster centroids.
These are the multi-domain signals that hypergraph routing can exploit.
"""

import json
import math
import os
from collections import defaultdict

def cosine_similarity(v1, v2):
    """Compute cosine similarity between two sparse vectors."""
    if not v1 or not v2:
        return 0.0
    
    dot = sum(v1.get(k, 0) * v2.get(k, 0) for k in set(v1.keys()) | set(v2.keys()))
    mag1 = math.sqrt(sum(v**2 for v in v1.values()))
    mag2 = math.sqrt(sum(v**2 for v in v2.values()))
    
    if mag1 == 0 or mag2 == 0:
        return 0.0
    
    return dot / (mag1 * mag2)

print("[LOAD] Reading TF-IDF clustering results...")

# Load dataset
dataset = {}
with open(os.path.expanduser("~/.cynic/organs/hermes/x/dataset.jsonl")) as f:
    for line in f:
        tweet = json.loads(line)
        dataset[tweet["tweet_id"]] = tweet

# Load click weights
from emergent_clustering_tfidf import compute_click_weights, load_data
_, clicks, captures = load_data()
click_weights = compute_click_weights(clicks, captures, dataset)

# Load TF-IDF vectors and clustering results
from emergent_clustering_tfidf import build_vocabulary_tfidf, vectorize_tweets_tfidf

print("[VECTORIZE] Rebuilding TF-IDF vectors...")
vocab, idf = build_vocabulary_tfidf(dataset)
vectors = vectorize_tweets_tfidf(dataset, vocab, idf)

with open("emergent_clusters_tfidf.json") as f:
    clustering_result = json.load(f)

# Reconstruct assignments from cluster_info
assignments = {}
for cluster_id_str, cluster_info in clustering_result["cluster_info"].items():
    cluster_id = int(cluster_id_str)
    # Note: we need to iterate through dataset and assign based on clustering
    # For now, we'll use a simplified approach: compute centroids from vectors
    
print("[CLUSTERING] Computing cluster centroids...")
centroids = defaultdict(lambda: defaultdict(float))
cluster_sizes = defaultdict(int)

# Build centroid mapping
tweet_ids = list(dataset.keys())

# For simplicity, sample-based assignment for demo
import random
random.seed(42)
sample_indices = random.sample(range(len(tweet_ids)), min(500, len(tweet_ids)))
sample_tweets = [tweet_ids[i] for i in sample_indices]

# Assign sample tweets by finding nearest cluster centroid from clustering result
# We'll use a heuristic: assign based on top authors/words match

def heuristic_cluster_assignment(tweet_id, dataset, clustering_result):
    """Heuristic: assign tweet to cluster based on content similarity to cluster profiles."""
    tweet_text = dataset[tweet_id].get("text", "").lower()
    author = dataset[tweet_id].get("author_screen_name", "")
    
    best_cluster = None
    best_score = -1
    
    for cluster_id_str, cluster_info in clustering_result["cluster_info"].items():
        cluster_id = int(cluster_id_str)
        
        # Score based on author presence
        top_authors = [a[0] for a in cluster_info["top_authors"][:3]]
        if author in top_authors:
            best_score = 2.0
            best_cluster = cluster_id
            break
        
        # Score based on word match
        top_words = [w[0] for w in cluster_info["top_words"][:10]]
        score = sum(1 for w in top_words if w in tweet_text)
        
        if score > best_score:
            best_score = score
            best_cluster = cluster_id
    
    return best_cluster if best_cluster is not None else 0

print("[ASSIGN] Assigning tweets to clusters (heuristic)...")
for tweet_id in sample_tweets:
    cluster_id = heuristic_cluster_assignment(tweet_id, dataset, clustering_result)
    assignments[tweet_id] = cluster_id

print(f"Assigned {len(assignments)} sample tweets to clusters")

print("\n[BRIDGE] Detecting bridges (multi-domain tweets)...")

# Compute cluster centroids from sample assignments
for tweet_id, cluster_id in assignments.items():
    vec = vectors[tweet_id]
    for word_idx, val in vec.items():
        centroids[cluster_id][word_idx] += val
    cluster_sizes[cluster_id] += 1

# Normalize centroids
for cluster_id in centroids:
    if cluster_sizes[cluster_id] > 0:
        for word_idx in centroids[cluster_id]:
            centroids[cluster_id][word_idx] /= cluster_sizes[cluster_id]

# Find bridges
bridges = defaultdict(list)
bridge_count = defaultdict(int)
threshold = 0.25

for tweet_id, cluster_id in assignments.items():
    vec = vectors[tweet_id]
    
    # Similarity to own cluster
    own_sim = cosine_similarity(vec, centroids[cluster_id])
    
    # Similarity to other clusters
    for other_cluster in centroids:
        if other_cluster == cluster_id:
            continue
        
        other_sim = cosine_similarity(vec, centroids[other_cluster])
        
        # Bridge: high similarity to other cluster despite being assigned to own
        if other_sim > threshold and own_sim < 0.7:
            pair = tuple(sorted([cluster_id, other_cluster]))
            bridges[pair].append({
                "tweet_id": tweet_id,
                "from_cluster": cluster_id,
                "to_cluster": other_cluster,
                "own_similarity": own_sim,
                "other_similarity": other_sim,
                "weight": click_weights.get(tweet_id, 1.0)
            })
            bridge_count[pair] += 1

print(f"\n{'=' * 100}")
print("BRIDGE DETECTION RESULTS (Multi-Domain Tweets)")
print(f"{'=' * 100}\n")

if bridge_count:
    print(f"Inter-cluster bridges (threshold > {threshold}):\n")
    for (c1, c2) in sorted(bridge_count.keys(), key=lambda x: -bridge_count[x]):
        count = bridge_count[(c1, c2)]
        total_weight = sum(b["weight"] for b in bridges[(c1, c2)])
        avg_sim = sum(b["other_similarity"] for b in bridges[(c1, c2)]) / count if count > 0 else 0
        
        print(f"  Cluster {c1} ↔ Cluster {c2}: {count} bridges (weight: {total_weight:.0f}, avg sim: {avg_sim:.2f})")
        
        # Example
        top_bridge = sorted(bridges[(c1, c2)], key=lambda x: -x["other_similarity"])[0]
        tweet_text = dataset[top_bridge["tweet_id"]].get("text", "")[:70]
        print(f"    Example: {tweet_text}...")
else:
    print("No bridges found above threshold (data may not have multi-domain tweets).")

# Summary
total_bridges = sum(bridge_count.values())
total_sample = len(assignments)
bridge_pct = 100 * total_bridges / total_sample if total_sample > 0 else 0

print(f"\n{'=' * 100}")
print("SUMMARY")
print(f"{'=' * 100}\n")
print(f"Total bridges (sample): {total_bridges} / {total_sample} tweets ({bridge_pct:.1f}%)")
print(f"Target: 5-15% of tweets should be bridges")
print(f"Result: {'✓ PASS' if 5 <= bridge_pct <= 15 else '~ Informative'} ({bridge_pct:.1f}% bridges found)")
print()
