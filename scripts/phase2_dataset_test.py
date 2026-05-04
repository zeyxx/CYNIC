#!/usr/bin/env python3
"""
Phase 2: Human-Filtering Impact (May 5-6)
Test Dogs on real tweet dataset from organ-x.
Falsify: Δ > 5% in verdict distribution. Simulation baseline: Δ=36.7%.

This script:
1. Loads pre-extracted tweets from dataset.jsonl
2. Extracts top 30 tokens by mention frequency
3. Gets sample tweets for each token
4. Submits tweets to /judge with real Dogs
5. Measures verdict distribution
"""

import json
import subprocess
import os
from pathlib import Path
from collections import Counter, defaultdict
from typing import Dict, List

ORGAN_X = Path.home() / ".cynic" / "organs" / "hermes" / "x"
DATASET = ORGAN_X / "dataset.jsonl"
CYNIC_REST = os.getenv("CYNIC_REST_ADDR", "http://localhost:3030")
CYNIC_KEY = os.getenv("CYNIC_API_KEY", "")

def judge(content, domain="token-analysis"):
    """Submit content to /judge, return verdict."""
    payload = {
        "content": content,
        "domain": domain
    }

    cmd = f"""
    curl -s -X POST {CYNIC_REST}/judge \
      -H "Authorization: Bearer {CYNIC_KEY}" \
      -H "Content-Type: application/json" \
      -d '{json.dumps(payload)}'
    """
    try:
        proc = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=60)
        return json.loads(proc.stdout)
    except subprocess.TimeoutExpired:
        return {"error": "TIMEOUT", "verdict": "TIMEOUT"}
    except Exception as e:
        return {"error": str(e)}

def load_dataset():
    """Load tweets from dataset.jsonl."""
    tweets = []
    with open(DATASET, 'r') as f:
        for i, line in enumerate(f):
            try:
                tweets.append(json.loads(line))
            except json.JSONDecodeError:
                if i % 100 == 0:
                    print(f"  Warning: JSON decode error at line {i}")
                continue
    return tweets

def main():
    print("[Phase 2 Dataset Test] Loading organ-x dataset...")
    tweets = load_dataset()
    print(f"Loaded {len(tweets)} tweets\n")

    # Extract tokens and their sample tweets
    ticker_tweets = defaultdict(list)
    ticker_count = Counter()

    for tweet in tweets:
        cashtags = tweet.get("cashtags", [])
        for tag in cashtags:
            ticker_count[tag] += 1
            if len(ticker_tweets[tag]) < 3:  # Keep up to 3 sample tweets per token
                ticker_tweets[tag].append(tweet.get("text", ""))

    # Get top 30 tokens
    top_30 = ticker_count.most_common(30)
    print(f"[Phase 2] Found {len(ticker_count)} unique tokens, testing top 30:")
    for ticker, count in top_30:
        print(f"  {ticker}: {count} mentions")
    print()

    # Submit sample tweets to /judge
    all_verdicts = {"howl": 0, "wag": 0, "growl": 0, "bark": 0}
    results = []

    print("[Phase 2] Submitting sample tweets to real Dogs...")
    for ticker, mention_count in top_30:
        print(f"  Testing {ticker}...", end=" ", flush=True)
        verdict_counts = {"howl": 0, "wag": 0, "growl": 0, "bark": 0}
        samples = ticker_tweets.get(ticker, [])

        for sample_text in samples[:3]:  # Submit up to 3 samples per token
            if sample_text:
                resp = judge(sample_text[:300])  # Cap at 300 chars
                if "verdict" in resp:
                    v = resp["verdict"].lower()
                    if v in verdict_counts:
                        verdict_counts[v] += 1
                        all_verdicts[v] += 1

        total_verdicts = sum(verdict_counts.values())
        print(f"  {total_verdicts} verdicts")
        results.append({
            "ticker": ticker,
            "mentions": mention_count,
            "samples_judged": total_verdicts,
            "verdicts": verdict_counts
        })

    # Summary
    total = sum(all_verdicts.values())
    print("\n[Phase 2 Results]")
    print(f"Total verdicts: {total}")
    if total > 0:
        print("Verdict distribution:")
        for k, v in all_verdicts.items():
            pct = (v / total) * 100
            print(f"  {k.upper()}: {v} ({pct:.1f}%)")

    # Write report
    report = {
        "phase": 2,
        "tokens_tested": len(top_30),
        "total_verdicts": total,
        "verdicts": all_verdicts,
        "results": results,
    }

    report_path = ORGAN_X / "reports" / "phase2_results.json"
    report_path.parent.mkdir(parents=True, exist_ok=True)
    with open(report_path, 'w') as f:
        json.dump(report, f, indent=2)

    print(f"\nReport written to {report_path}")
    print("[Phase 2 Test] Complete")

if __name__ == "__main__":
    main()
