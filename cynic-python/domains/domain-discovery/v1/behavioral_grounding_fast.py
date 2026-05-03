#!/usr/bin/env python3
"""Behavioral Grounding: Fast check without waiting for v1 re-run."""

import json
import os
import re
from collections import defaultdict
from pathlib import Path
import math

STOPWORDS = {
    "the", "and", "you", "for", "all", "this", "that", "your", "our", "is", "are",
    "was", "were", "have", "has", "had", "be", "been", "being", "do", "does", "did",
    "will", "would", "could", "should", "may", "might", "must", "can", "shall",
    "a", "an", "in", "on", "at", "to", "from", "of", "with", "by", "about", "as",
    "http", "https", "tps", "amp", "href", "gt", "lt", "nbsp", "et", "url", "link",
    "follow", "check", "read", "see", "thread", "tweet", "", "rt", "via"
}

def clean_text(text: str) -> str:
    text = re.sub(r'http\S+', '', text)
    text = re.sub(r'www\S+', '', text)
    text = re.sub(r'@\w+', '', text)
    text = re.sub(r'#(\w+)', r'\1', text)
    text = text.replace('&amp;', 'and').replace('&lt;', '<').replace('&gt;', '>')
    return ' '.join(text.split())

def tokenize(text: str) -> list:
    return [t for t in text.lower().split() if t not in STOPWORDS and len(t) > 2]

print("[FAST] Behavioral Grounding (skipping slow clustering re-run)\n")

print("[LOAD] v1 results and behavior log...")
with open("results_v1/emergent_clusters_tfidf.json") as f:
    v1_results = json.load(f)

dataset = {}
with open(os.path.expanduser("~/.cynic/organs/hermes/x/dataset.jsonl")) as f:
    for line in f:
        tweet = json.loads(line)
        dataset[tweet["tweet_id"]] = tweet

behavior_log = []
with open(os.path.expanduser("~/.cynic/organs/hermes/behavior/behavior_log.jsonl")) as f:
    for line in f:
        behavior_log.append(json.loads(line))

clicks = [e for e in behavior_log if e.get("type") == "click"]

print(f"  v1: {v1_results['n_tweets']} tweets, {len(v1_results['cluster_info'])} clusters")
print(f"  Behavior: {len(dataset)} dataset tweets, {len(clicks)} clicks")

# Use v1's top authors to infer cluster membership
# Assign tweets by top authors to clusters
print("\n[INFER] Building cluster assignments from v1 top authors...")
v1_cluster_distribution = {}
author_to_clusters = defaultdict(set)
author_weights = defaultdict(float)

for cid_str, info in v1_results["cluster_info"].items():
    cid = int(cid_str)
    v1_cluster_distribution[cid] = {
        "count": info["tweet_count"],
        "percent": info["percent"]
    }
    for author, weight in info["top_authors"]:
        author_to_clusters[author].add(cid)
        author_weights[author] += weight

# Assign tweets by author
tweet_to_cluster = {}
for tweet_id, tweet in dataset.items():
    author = tweet.get("author", "")
    if author in author_to_clusters:
        # Assign to most-weighted cluster
        best_cid = max(author_to_clusters[author], key=lambda c: author_weights[author] / len(author_to_clusters[author]))
        tweet_to_cluster[tweet_id] = best_cid

print(f"  Assigned {len(tweet_to_cluster)} tweets by author")

# Classify clicks
print("\n[CLICKS] Mapping clicks to v1 clusters...")
click_cluster_distribution = defaultdict(int)
for click in clicks:
    tweet_id = click.get("tweet_id")
    if tweet_id in tweet_to_cluster:
        cluster = tweet_to_cluster[tweet_id]
        click_cluster_distribution[cluster] += 1

total_clicks = sum(click_cluster_distribution.values())
print(f"  Mapped {total_clicks} clicks")

print("\n" + "="*80)
print("BEHAVIORAL GROUNDING: v1 TF-IDF vs. Human Clicks")
print("="*80)

# Print both distributions
print("\nv1 TF-IDF Distribution (all tweets):")
for cid in sorted(v1_cluster_distribution.keys()):
    pct = v1_cluster_distribution[cid]["percent"]
    print(f"  Cluster {cid}: {pct:5.1f}%")

print("\nHuman Click Distribution (assigned by author):")
max_divergence = 0
for cid in sorted(v1_cluster_distribution.keys()):
    v1_pct = v1_cluster_distribution[cid]["percent"]
    click_pct = 100 * click_cluster_distribution.get(cid, 0) / total_clicks if total_clicks > 0 else 0
    divergence = abs(v1_pct - click_pct)
    max_divergence = max(max_divergence, divergence)
    symbol = "✓" if divergence < 10 else "⚠" if divergence < 20 else "✗"
    print(f"  {symbol} Cluster {cid}: clicks={click_pct:5.1f}%  v1={v1_pct:5.1f}%  Δ={divergence:5.1f}%")

print("\n" + "="*80)
print(f"Max divergence: {max_divergence:.1f}%")
if max_divergence < 10:
    status = "✓ GROUNDED"
    conf = "φ⁻¹"
    note = "Human clicks align well with TF-IDF clusters. Domain model behaviorally sound."
elif max_divergence < 20:
    status = "⚠ PARTIAL"
    conf = "φ⁻²"
    note = "Some cluster misalignment, but overall structure holds. Watch Phase 3 performance."
else:
    status = "✗ DIVERGENT"
    conf = "φ⁻³"
    note = "Human clicks diverge significantly. Domain model suspect, Phase 3 may underperform."

print(f"{status} (Confidence: {conf})")
print(f"→ {note}")
print("="*80)

# Save results
grounding = {
    "hypothesis": "Human clicks align with TF-IDF clusters",
    "method": "Author-based cluster assignment",
    "max_divergence": max_divergence,
    "falsified": max_divergence > 30,
    "confidence": conf,
    "v1_distribution": v1_cluster_distribution,
    "click_distribution": dict(click_cluster_distribution),
    "total_clicks_mapped": total_clicks,
}

with open("results_v1/behavioral_grounding.json", "w") as f:
    json.dump(grounding, f, indent=2)

print(f"\n✓ Saved to results_v1/behavioral_grounding.json")
