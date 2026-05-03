#!/usr/bin/env python3
"""Behavioral Topic Analysis: Do clicked tweets match v1 cluster topics?"""

import json
import os
import re
from collections import Counter, defaultdict

STOPWORDS = {
    "the", "and", "you", "for", "all", "this", "that", "your", "our", "is", "are",
    "was", "were", "have", "has", "had", "be", "been", "being", "do", "does", "did",
    "will", "would", "could", "should", "may", "might", "must", "can", "shall",
    "a", "an", "in", "on", "at", "to", "from", "of", "with", "by", "about", "as",
    "http", "https", "tps", "amp", "href", "gt", "lt", "nbsp", "et", "url", "link",
    "follow", "check", "read", "see", "thread", "tweet", "", "rt", "via", "i", "me", "it"
}

def clean_text(text: str) -> str:
    text = re.sub(r'http\S+', '', text)
    text = re.sub(r'www\S+', '', text)
    text = re.sub(r'@\w+', '', text)
    text = re.sub(r'#(\w+)', r'\1', text)
    return ' '.join(text.split())

def tokenize(text: str) -> list:
    return [t for t in text.lower().split() if t not in STOPWORDS and len(t) > 2]

print("[TOPIC] Behavioral Clustering Analysis\n")

print("[LOAD] v1 results and datasets...")
with open("results_v1/emergent_clusters_tfidf.json") as f:
    v1_results = json.load(f)

# Extract top words per cluster from v1
v1_cluster_topics = {}
for cid_str, info in v1_results["cluster_info"].items():
    cid = int(cid_str)
    v1_cluster_topics[cid] = {
        "words": [w[0] for w in info["top_words"]],
        "percent": info["percent"]
    }

dataset = {}
with open(os.path.expanduser("~/.cynic/organs/hermes/x/dataset.jsonl")) as f:
    for line in f:
        tweet = json.loads(line)
        tweet["text"] = clean_text(tweet.get("text", ""))
        dataset[tweet["tweet_id"]] = tweet

behavior_log = []
with open(os.path.expanduser("~/.cynic/organs/hermes/behavior/behavior_log.jsonl")) as f:
    for line in f:
        behavior_log.append(json.loads(line))

clicks = [e for e in behavior_log if e.get("type") == "click"]
print(f"  v1: {len(v1_cluster_topics)} clusters with topic keywords")
print(f"  Clicks: {len(clicks)}")

# Extract topics from clicked tweets
print("\n[ANALYZE] Extracting topics from clicked tweets...")
clicked_word_freq = Counter()
clicked_tweets = set()

for click in clicks:
    tweet_id = click.get("tweet_id")
    if tweet_id in dataset:
        clicked_tweets.add(tweet_id)
        text = dataset[tweet_id]["text"]
        words = tokenize(text)
        clicked_word_freq.update(words)

print(f"  Found {len(clicked_tweets)} clicked tweets")
print(f"  Vocabulary in clicks: {len(clicked_word_freq)} unique words")

# Top 20 words in clicked tweets
top_clicked_words = [w[0] for w in clicked_word_freq.most_common(20)]
print(f"\n  Top words in clicked tweets: {', '.join(top_clicked_words)}")

# Measure topic overlap: for each cluster, count how many top words appear in clicked tweets
print("\n" + "="*80)
print("TOPIC OVERLAP: v1 Cluster Keywords vs. Clicked Tweet Topics")
print("="*80 + "\n")

overlap_scores = {}
for cid in sorted(v1_cluster_topics.keys()):
    cluster_words = set(v1_cluster_topics[cid]["words"])
    clicked_words = set(top_clicked_words)
    overlap = cluster_words & clicked_words
    overlap_pct = 100 * len(overlap) / len(cluster_words) if cluster_words else 0
    overlap_scores[cid] = overlap_pct
    symbol = "✓" if overlap_pct > 50 else "⚠" if overlap_pct > 30 else "✗"
    print(f"  {symbol} Cluster {cid} ({v1_cluster_topics[cid]['percent']:.1f}% of v1)")
    print(f"     Top words: {', '.join(v1_cluster_topics[cid]['words'][:5])}")
    print(f"     Overlap with clicked: {len(overlap)}/{len(cluster_words)} words ({overlap_pct:.0f}%)")
    if overlap:
        print(f"     → {', '.join(overlap)}")
    print()

avg_overlap = sum(overlap_scores.values()) / len(overlap_scores) if overlap_scores else 0
print("="*80)
print(f"Average topic overlap: {avg_overlap:.0f}%")

if avg_overlap > 50:
    status = "✓ ALIGNED"
    conf = "φ⁻¹"
    note = "Clicked tweets contain v1 cluster keywords. Topics structurally sound."
elif avg_overlap > 30:
    status = "⚠ PARTIAL"
    conf = "φ⁻²"
    note = "Some keyword matches, but weak. Domain may not capture user interests."
else:
    status = "✗ MISMATCH"
    conf = "φ⁻³"
    note = "Few matching keywords. Human and v1 clustering address different topics."

print(f"{status} (Confidence: {conf})")
print(f"→ {note}")
print("="*80)

# Breakdown: which clusters have best overlap?
sorted_overlap = sorted(overlap_scores.items(), key=lambda x: -x[1])
print(f"\nCluster ranking by topic overlap:")
for cid, overlap in sorted_overlap[:3]:
    print(f"  {cid}: {overlap:.0f}% overlap (v1 weight: {v1_cluster_topics[cid]['percent']:.1f}%)")

# Save analysis
analysis = {
    "hypothesis": "Clicked tweets share topics with v1 clusters",
    "method": "Keyword overlap (top 10 words per cluster vs. top 20 words in clicks)",
    "average_overlap": avg_overlap,
    "confidence": conf,
    "result": status,
    "cluster_overlaps": overlap_scores,
    "top_clicked_words": top_clicked_words,
    "total_clicked_tweets_found": len(clicked_tweets),
}

with open("results_v1/behavioral_topic_analysis.json", "w") as f:
    json.dump(analysis, f, indent=2)

print(f"\n✓ Saved analysis to results_v1/behavioral_topic_analysis.json")
