#!/usr/bin/env python3
"""
CYNIC Data Lab — Orchestrate analyses, produce briefings.

Minimal Phase Lab-1 implementation:
1. Standardize dataset (2,287 tweets)
2. Run 3 core analyses (distribution, disagreement, false_negatives)
3. Write briefing for Hermes/Claude

Usage:
  python lab.py --dataset ~/.cynic/organs/hermes/x/dataset.jsonl \
                --output ~/.cynic/organs/hermes/x/lab_briefing_latest.json
"""

import json
import os
from collections import Counter
from datetime import datetime
from pathlib import Path
from statistics import mean, stdev, median
from typing import Optional

import yaml


# ── CONFIG ──

def load_config(config_dir: str = "config") -> dict:
    """Load domains, axioms, Dogs from YAML."""
    config = {}
    for fname in ["domains.yaml", "axioms.yaml"]:
        fpath = Path(config_dir) / fname
        if fpath.exists():
            with open(fpath) as f:
                config[fname.replace(".yaml", "")] = yaml.safe_load(f)
    return config


def load_observations_from_organ(organ_dir: str = None) -> list:
    """Load observations from organ directory (JSON files in observations/ subdir)."""
    if organ_dir is None:
        organ_dir = str(Path.home() / ".cynic" / "organs" / "hermes" / "x")

    obs_dir = Path(organ_dir) / "observations"
    if not obs_dir.exists():
        return []

    observations = []
    for json_file in sorted(obs_dir.glob("*.json")):
        try:
            with open(json_file) as f:
                obs = json.load(f)
                observations.append(obs)
        except (json.JSONDecodeError, IOError):
            continue
    return observations


# ── STANDARDIZE ──

def standardize_tweet(raw: dict) -> dict:
    """Normalize raw tweet to canonical format."""
    return {
        "id": raw.get("tweet_id", raw.get("id")),
        "text": raw.get("text", ""),
        "signal": {
            "version": "2",  # Current heuristic version
            "score": raw.get("signal_score", 0),
        },
        "author": {
            "screen_name": raw.get("author_screen_name"),
            "tier": raw.get("author_tier", "unknown"),
            "verified": raw.get("author_verified", False),
            "followers": raw.get("author_followers_count", 0),
        },
        "engagement": {
            "views": raw.get("views", 0),
            "likes": raw.get("likes", 0),
            "retweets": raw.get("retweets", 0),
            "replies": raw.get("replies", 0),
            "bookmarks": raw.get("bookmarks", 0),
        },
        "metadata": {
            "source": raw.get("sampling_bias", "unknown"),
            "domain": "twitter",
            "cashtags": raw.get("cashtags", []),
            "narratives": raw.get("narratives", []),
            "captured_at": raw.get("capture_ts"),
        },
    }


# ── ANALYSES ──

def analyze_distribution(tweets: list) -> dict:
    """Signal score distribution per domain (inferred from keywords)."""
    config = load_config()
    domains = config.get("domains", {})

    # Filter out non-domain keys (e.g., "version")
    domain_signals = {d: [] for d in domains.keys() if isinstance(domains[d], dict)}

    for tweet in tweets:
        text = tweet.get("text", "").lower()
        score = tweet.get("signal", {}).get("score", 0)

        # Simple domain mapping (keyword-based)
        found = False
        for domain_id, domain_info in domains.items():
            if not isinstance(domain_info, dict):
                continue
            keywords = domain_info.get("keywords", [])
            if any(kw.lower() in text for kw in keywords):
                domain_signals[domain_id].append(score)
                found = True
                break

        if not found:
            # Default to D1 (tokens)
            domain_signals["D1"].append(score)

    result = {}
    for domain_id, scores in domain_signals.items():
        if scores:
            result[domain_id] = {
                "count": len(scores),
                "mean": round(mean(scores), 2),
                "median": round(median(scores), 2),
                "stdev": round(stdev(scores), 2) if len(scores) > 1 else 0,
                "min": min(scores),
                "max": max(scores),
            }
        else:
            result[domain_id] = {"count": 0}

    return result


def analyze_false_negatives(tweets: list) -> dict:
    """High-quality tweets that got low signal_score."""
    false_negatives = []

    for tweet in tweets:
        text = tweet.get("text", "")
        signal = tweet.get("signal", {}).get("score", 0)

        # Heuristic: high-quality indicators despite low signal
        quality_indicators = [
            "analysis" in text.lower(),
            "thread" in text.lower(),
            "research" in text.lower(),
            "data" in text.lower(),
            "on-chain" in text.lower() or "onchain" in text.lower(),
        ]

        high_engagement = (
            tweet.get("engagement", {}).get("likes", 0) > 500
            or tweet.get("engagement", {}).get("replies", 0) > 100
        )

        if (sum(quality_indicators) >= 2 or high_engagement) and signal < 2:
            false_negatives.append({
                "id": tweet.get("id"),
                "text": text[:80],
                "signal_score": signal,
                "author_tier": tweet.get("author", {}).get("tier"),
                "engagement": tweet.get("engagement", {}),
                "quality_indicators": quality_indicators,
            })

    return {
        "count": len(false_negatives),
        "rate": round(len(false_negatives) / len(tweets) * 100, 1) if tweets else 0,
        "examples": false_negatives[:5],
    }


def analyze_disagreement_zones(tweets: list) -> dict:
    """Where Dogs would likely disagree (high uncertainty indicators)."""
    disagreement_tweets = []

    for tweet in tweets:
        text = tweet.get("text", "").lower()
        signal = tweet.get("signal", {}).get("score", 0)
        author_tier = tweet.get("author", {}).get("tier")

        # Disagreement happens when:
        # - Hype language but low signal (Dogs read text, heuristic disagrees)
        # - Unverified but high-signal (authority mismatch)
        # - Mixed signals (both quality + spam keywords)

        has_hype = any(w in text for w in ["moon", "100x", "pump", "gem"])
        has_warning = any(w in text for w in ["rug", "scam", "honeypot", "fraud"])
        has_analysis = any(w in text for w in ["analysis", "research", "data"])

        uncertainty = 0
        if has_hype and signal < 3:
            uncertainty += 1  # Dogs might see hype differently
        if has_warning and signal < 4:
            uncertainty += 1  # Dogs may weigh warnings higher
        if author_tier == "unknown" and signal > 3:
            uncertainty += 1  # Dogs might penalize unverified
        if has_analysis and has_hype:
            uncertainty += 1  # Mixed signals confuse heuristics

        if uncertainty >= 2:
            disagreement_tweets.append({
                "id": tweet.get("id"),
                "text": text[:80],
                "signal_score": signal,
                "author_tier": author_tier,
                "uncertainty_factors": [
                    f"has_hype={has_hype}",
                    f"has_warning={has_warning}",
                    f"has_analysis={has_analysis}",
                ],
            })

    return {
        "count": len(disagreement_tweets),
        "rate": round(len(disagreement_tweets) / len(tweets) * 100, 1) if tweets else 0,
        "examples": disagreement_tweets[:5],
    }


# ── BRIEFING ──

def generate_briefing(dataset_path: str, organ_dir: str = None) -> dict:
    """Full analysis → briefing for Hermes/Claude.

    Loads initial dataset + observations from organ.
    K15: observations from /observe flow back into analysis.
    """
    print(f"Loading dataset: {dataset_path}")
    tweets_raw = []
    with open(dataset_path) as f:
        for line in f:
            try:
                tweets_raw.append(json.loads(line.strip()))
            except json.JSONDecodeError:
                continue

    # K15: Load observations from organ (stored via /observe → HermesXReader)
    organ_obs = load_observations_from_organ(organ_dir)
    print(f"Loading {len(organ_obs)} observations from organ...")
    tweets_raw.extend(organ_obs)

    print(f"Standardizing {len(tweets_raw)} tweets...")
    tweets = [standardize_tweet(t) for t in tweets_raw]

    print("Running analyses...")
    distribution = analyze_distribution(tweets)
    false_negatives = analyze_false_negatives(tweets)
    disagreement = analyze_disagreement_zones(tweets)

    # Generate recommendation
    config = load_config()
    domains = config.get("domains", {})

    domain_coverage = {}
    for domain_id, domain_info in domains.items():
        if not isinstance(domain_info, dict):
            continue
        curated = distribution.get(domain_id, {}).get("count", 0)
        target = domain_info.get("target", 100)
        domain_coverage[domain_id] = {
            "curated": curated,
            "target": target,
            "deficit": max(0, target - curated),
        }

    # Pick domain with highest deficit + HIGH priority
    ranked = sorted(
        domain_coverage.items(),
        key=lambda x: (
            x[1]["deficit"],
            -1 if domains[x[0]].get("priority") == "HIGH" else 1,
        ),
        reverse=True,
    )

    suggested_domain = ranked[0][0] if ranked else "D1"
    suggested_info = domains.get(suggested_domain, {})

    briefing = {
        "timestamp": datetime.now().isoformat(),
        "dataset_id": Path(dataset_path).stem,
        "tweet_count": len(tweets),
        "analyses": {
            "distribution": distribution,
            "false_negatives": false_negatives,
            "disagreement": disagreement,
        },
        "recommendation": {
            "domain": suggested_domain,
            "name": suggested_info.get("name"),
            "reason": f"Deficit: {ranked[0][1]['deficit']} tweets, Priority: {suggested_info.get('priority')}",
        },
        "briefing": f"""
Dataset Analysis: {len(tweets)} tweets

KEY FINDINGS:
- Signal distribution by domain: D1={distribution.get('D1', {}).get('mean')} avg, D4={distribution.get('D4', {}).get('mean')} avg
- False negatives: {false_negatives['count']} tweets ({false_negatives['rate']}%) have quality but low signal
- Disagreement zones: {disagreement['count']} tweets ({disagreement['rate']}%) likely to confuse Dogs

RECOMMENDATION:
Explore {suggested_domain} ({suggested_info.get('name')}) next.
Reason: {ranked[0][1]['deficit']} tweets deficit, {suggested_info.get('priority')} priority.

NEXT ACTIONS:
1. Review false_negatives examples → improve signal_score heuristic
2. Browse recommended domain (@gcrtrd for D4, etc.)
3. Post ≥3 observations to cynic_observe
4. Re-run lab after new observations
""",
        "uncertainties": [
            "Dogs not yet benchmarked on these tweets (need /judge responses)",
            "Signal_score heuristic quality unknown without verdicts",
            "Domain assignment is keyword-based; may misclassify borderline tweets",
        ],
        "_meta": {
            "lab_version": "0.1.0",
            "signal_version": "2",
            "reproducibility_token": f"lab-0.1.0-signal-2-tweets-{len(tweets)}",
        },
    }

    return briefing


def main():
    dataset_path = os.environ.get(
        "X_DATASET_PATH",
        Path.home() / ".cynic" / "organs" / "hermes" / "x" / "dataset.jsonl",
    )

    if not Path(dataset_path).exists():
        print(f"ERROR: Dataset not found: {dataset_path}")
        return

    briefing = generate_briefing(str(dataset_path))

    # Save
    output_path = (
        Path.home()
        / ".cynic"
        / "organs"
        / "hermes"
        / "x"
        / "lab_briefing_latest.json"
    )
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with open(output_path, "w") as f:
        json.dump(briefing, f, indent=2)

    print(f"\n✓ Lab briefing saved to {output_path}")
    print(f"\nRECOMMENDATION: {briefing['recommendation']['domain']} ({briefing['recommendation']['name']})")
    print(f"Reason: {briefing['recommendation']['reason']}")


if __name__ == "__main__":
    main()
