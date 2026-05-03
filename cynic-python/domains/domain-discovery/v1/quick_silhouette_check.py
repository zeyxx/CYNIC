#!/usr/bin/env python3
"""Quick silhouette check on existing v1 clustering results."""

import json
import math
from typing import Dict, List

# Load existing clustering results
with open("results_v1/emergent_clusters_tfidf.json") as f:
    results = json.load(f)

# Extract assignments (cluster_id per tweet)
assignments = results.get("assignments", {})
if not assignments:
    print("✗ No assignments in results; cannot compute silhouette")
    exit(1)

print(f"✓ Loaded {len(assignments)} assignments from v1 results")

# We don't have vectors saved, so estimate silhouette from cluster structure
# Proxy: use per-cluster isolation (ratio of within vs between cluster size)

cluster_sizes = {}
for tweet_id, cluster_id in assignments.items():
    cluster_id_str = str(cluster_id)
    cluster_sizes[cluster_id_str] = cluster_sizes.get(cluster_id_str, 0) + 1

n_clusters = len(cluster_sizes)
total_tweets = len(assignments)

print(f"\nCluster structure:")
for cid in sorted(cluster_sizes.keys(), key=lambda x: -cluster_sizes[x]):
    size = cluster_sizes[cid]
    percent = 100 * size / total_tweets
    print(f"  Cluster {cid}: {size} tweets ({percent:.1f}%)")

# Rough silhouette proxy: HHI of cluster distribution
# (more imbalanced = lower silhouette, typically)
hhi = sum((size / total_tweets) ** 2 for size in cluster_sizes.values())
silhouette_proxy = 1.0 - hhi  # Rough proxy (0=one cluster, ~0.86=balanced 7-cluster)

print(f"\nCluster distribution quality (HHI-based proxy):")
print(f"  HHI: {hhi:.3f} (perfect balance = 1/{n_clusters} = {1/n_clusters:.3f})")
print(f"  Silhouette proxy: {silhouette_proxy:.3f} (rough estimate)")
print(f"  Interpretation: {'well-distributed' if silhouette_proxy > 0.70 else 'imbalanced'} clusters")

# Save as metrics
metrics = {
    "silhouette_measured": False,
    "silhouette_proxy": silhouette_proxy,
    "n_clusters": n_clusters,
    "n_tweets": total_tweets,
    "cluster_distribution": cluster_sizes,
    "note": "Proxy based on cluster imbalance; full silhouette requires vectors"
}

with open("results_v1/metrics_v1.json", "w") as f:
    json.dump(metrics, f, indent=2)

print(f"\n✓ Saved proxy metrics to results_v1/metrics_v1.json")
print(f"\nNOTE: This is a proxy estimate only.")
print(f"For accurate silhouette, re-run clustering with vectors saved.")
