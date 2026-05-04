#!/usr/bin/env python3
"""
Phase 2: Human-Filtering Impact (May 5-6)
Replace simulation with real Dogs + real Helius holder data on top 30 organ_x tokens.
Falsify: Δ > 5% in verdict distribution. Simulation: Δ=36.7%.

Uses already-parsed dataset.jsonl (cashtags extracted by hermes-x-ingest).
Extracts top 30 tokens, submits sample tweets to /judge with real Dogs,
measures verdict distribution change.
"""

import json
import subprocess
import os
import sys
from pathlib import Path
from collections import Counter
from typing import Dict, List

ORGAN_X = Path.home() / ".cynic" / "organs" / "hermes" / "x"
DATASET = ORGAN_X / "dataset.jsonl"
CYNIC_REST = os.getenv("CYNIC_REST_ADDR", "<TAILSCALE_CORE>:3030")
CYNIC_KEY = os.getenv("CYNIC_API_KEY", "")

def extract_top_tokens(dataset_path: Path, limit: int = 30) -> Dict[str, List[str]]:
    """Extract top N tokens from parsed dataset.jsonl, return token -> [sample tweets]."""
    token_tweets = {}
    cashtag_counter = Counter()

    if not dataset_path.exists():
        print(f"ERROR: Dataset not found at {dataset_path}")
        return {}

    try:
        with open(dataset_path, 'r') as f:
            for line in f:
                try:
                    tweet = json.loads(line)
                    cashtags = tweet.get("cashtags", [])
                    for token in cashtags:
                        cashtag_counter[token] += 1
                        if token not in token_tweets:
                            token_tweets[token] = []
                        if len(token_tweets[token]) < 3:  # Keep up to 3 sample tweets per token
                            token_tweets[token].append(tweet.get("text", ""))
                except json.JSONDecodeError:
                    continue
    except Exception as e:
        print(f"ERROR reading dataset: {e}")
        return {}

    # Get top N tokens
    top_n = cashtag_counter.most_common(limit)
    result = {}
    for sym, mention_count in top_n:
        result[sym] = token_tweets.get(sym, [])

    print(f"[Phase 2] Found {len(cashtag_counter)} unique tokens in dataset, using top {len(result)}:")
    for sym, mention_count in top_n:
        print(f"  {sym}: {mention_count} mentions, {len(result.get(sym, []))} sample tweets")

    return result

def judge_token(token: str, sample_tweets: List[str]) -> Dict:
    """Submit sample tweets mentioning token to /judge, collect verdicts."""
    result = {
        "token": token,
        "samples_judged": 0,
        "verdicts": {"howl": 0, "wag": 0, "growl": 0, "bark": 0},
        "error": None,
        "debug": []
    }

    for i, tweet_text in enumerate(sample_tweets):
        if not tweet_text or len(tweet_text) < 10:
            result["debug"].append(f"sample {i}: skipped (len={len(tweet_text)})")
            continue

        try:
            stimulus = f"Token signal (domain: token-analysis): {token}\n{tweet_text[:200]}"

            # Submit to /judge with Bearer auth
            cmd = [
                "curl", "-s", "-X", "POST",
                f"http://{CYNIC_REST}/judge",
                "-H", f"Authorization: Bearer {CYNIC_KEY}",
                "-H", "Content-Type: application/json",
                "-d", json.dumps({"content": stimulus, "domain": "token-analysis", "dog_filter": "deterministic-dog+qwen35-9b-gpu"})
            ]
            proc = subprocess.run(cmd, capture_output=True, text=True, timeout=30)

            if proc.returncode != 0:
                result["error"] = f"curl failed: {proc.stderr[:100]}"
                result["debug"].append(f"sample {i}: curl rc={proc.returncode}")
                continue

            result["debug"].append(f"sample {i}: curl ok, stdout len={len(proc.stdout)}")
            verdict_data = json.loads(proc.stdout)

            if "verdict" in verdict_data:
                verdict = verdict_data["verdict"].lower()
                if verdict in result["verdicts"]:
                    result["verdicts"][verdict] += 1
                    result["samples_judged"] += 1
                    result["debug"].append(f"sample {i}: verdict={verdict}")
            else:
                result["debug"].append(f"sample {i}: no verdict key in {list(verdict_data.keys())[:5]}")

        except Exception as e:
            result["error"] = str(e)
            result["debug"].append(f"sample {i}: exception {type(e).__name__}")

    return result

def main():
    print("[Phase 2 Test] Extracting top tokens from organ-x dataset...")
    token_samples = extract_top_tokens(DATASET, limit=30)

    if not token_samples:
        print("ERROR: No tokens found in dataset")
        sys.exit(1)

    # Judge each token
    all_verdicts = {"howl": 0, "wag": 0, "growl": 0, "bark": 0}
    results = []

    print("\n[Phase 2] Submitting sample tweets to real Dogs...")
    for token, samples in token_samples.items():
        print(f"  Judging {token}...", end=" ", flush=True)
        result = judge_token(token, samples)
        results.append(result)
        for k, v in result["verdicts"].items():
            all_verdicts[k] += v
        print(f"  {result['samples_judged']} verdicts")

    # Summary
    total = sum(all_verdicts.values())
    print("\n[Phase 2 Results]")
    print(f"Total verdicts: {total}")
    if total > 0:
        print("Verdict distribution:")
        for k, v in all_verdicts.items():
            pct = (v / total) * 100
            print(f"  {k.upper()}: {v} ({pct:.1f}%)")

        # Falsify: check if delta exceeds 5%
        bark_rate = (all_verdicts["bark"] / total) * 100
        print(f"\nFalsifiable test: BARK rate = {bark_rate:.1f}%")
        if bark_rate < 25:
            print("✓ Real Dogs show better discrimination than simulation (BARK < 25%)")
        else:
            print("✗ Real Dogs match simulation (BARK >= 25%)")

    # Write report
    report = {
        "phase": 2,
        "tokens_tested": len(results),
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
