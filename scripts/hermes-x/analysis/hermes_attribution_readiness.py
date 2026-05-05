#!/usr/bin/env python3
"""
Hermes X Attribution & Kairos Readiness — Measure organism autonomy readiness.

Not just tagging data, but answering: **Is the organism ready to search autonomously?**

Measures:
1. Correlation: Do organism suggestions appear in user's actual searches?
2. Accuracy: When organism suggests something, do kernel verdicts validate it?
3. Domain readiness: Per-domain scores for autonomy
4. Kairos threshold: Is organism ready to make autonomous X searches?

When correlation > 0.618 (φ⁻¹) AND accuracy > 0.618 → KAIROS = ready.

Usage:
    python3 hermes_attribution_readiness.py --organ-dir ~/.cynic/organs/hermes/x

Output:
    - readiness.json (per-domain scores, kairos status)
    - attribution.jsonl (detailed correlations)
"""

__version__ = "0.1.0"

import json
import logging
import re
from collections import defaultdict
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List, Tuple
import statistics

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(name)s: %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("hermes-readiness")

ORGAN_DIR = Path.home() / ".cynic" / "organs" / "hermes" / "x"
BEHAVIOR_DIR = Path.home() / ".cynic" / "organs" / "hermes" / "behavior"

SEARCH_TASKS_PATH = ORGAN_DIR / "search_tasks.jsonl"
BEHAVIOR_LOG_PATH = BEHAVIOR_DIR / "behavior_log.jsonl"
DATASET_PATH = ORGAN_DIR / "dataset.jsonl"
VERDICTS_PATH = ORGAN_DIR / "observation-verdicts"
ATTRIBUTION_PATH = ORGAN_DIR / "attribution.jsonl"
READINESS_PATH = ORGAN_DIR / "readiness.json"


def load_organism_suggestions() -> Dict[str, List[str]]:
    """Load organism keywords grouped by domain."""
    keywords_by_domain = defaultdict(list)
    if not SEARCH_TASKS_PATH.exists():
        return keywords_by_domain

    try:
        with open(SEARCH_TASKS_PATH) as f:
            for line in f:
                try:
                    task = json.loads(line)
                    domain = task.get("domain", "unknown")
                    keyword = task.get("keyword", "").lower().strip()
                    if keyword:
                        keywords_by_domain[domain].append(keyword)
                except json.JSONDecodeError:
                    pass
    except IOError:
        pass

    return keywords_by_domain


def extract_user_searches_from_behavior(
    time_window_hours: int = 24,
) -> Dict[str, int]:
    """Extract keywords user actually searched (from behavior_log).

    Returns {keyword: count} of times user typed this keyword.
    Limited to recent window to avoid stale data.
    """
    searches = defaultdict(int)
    if not BEHAVIOR_LOG_PATH.exists():
        return searches

    cutoff = datetime.now(datetime.now().astimezone().tzinfo) - timedelta(
        hours=time_window_hours
    )

    try:
        with open(BEHAVIOR_LOG_PATH) as f:
            for line in f:
                try:
                    event = json.loads(line)
                    # Heuristic: look for key events (typing)
                    if event.get("type") == "key":
                        # This is a keystroke, not the full text
                        # Real extraction would need keystroke reconstruction
                        pass
                except json.JSONDecodeError:
                    pass
    except IOError:
        pass

    # For now: return empty (keystroke reconstruction is complex)
    # In practice, you'd integrate with keystroke logger output
    return searches


def load_verdicts() -> List[Dict]:
    """Load observation verdicts to measure organism accuracy."""
    verdicts = []
    if not VERDICTS_PATH.exists():
        return verdicts

    try:
        for verdict_file in VERDICTS_PATH.glob("*.json"):
            try:
                with open(verdict_file) as f:
                    verdicts.append(json.load(f))
            except json.JSONDecodeError:
                pass
    except OSError:
        pass

    return verdicts


def load_dataset_by_domain() -> Dict[str, List[Dict]]:
    """Group dataset tweets by inferred domain."""
    tweets_by_domain = defaultdict(list)
    if not DATASET_PATH.exists():
        return tweets_by_domain

    try:
        with open(DATASET_PATH) as f:
            for line in f:
                try:
                    tweet = json.loads(line)
                    # Infer domain from tweet text heuristics
                    text = (tweet.get("text") or "").lower()
                    domain = infer_domain_from_text(text)
                    tweets_by_domain[domain].append(tweet)
                except json.JSONDecodeError:
                    pass
    except IOError:
        pass

    return tweets_by_domain


def infer_domain_from_text(text: str) -> str:
    """Simple heuristic: infer domain from tweet text."""
    # This is a placeholder; real implementation would use ML or curated rules
    if any(word in text for word in ["supply", "chain", "logistics"]):
        return "D1"
    if any(word in text for word in ["honeypot", "rug pull", "scam"]):
        return "D2"
    if any(word in text for word in ["regulatory", "sec", "compliance"]):
        return "D4"
    if any(word in text for word in ["sovereignty", "decentralization"]):
        return "D5"
    if any(word in text for word in ["community", "culture", "vibes"]):
        return "D6"
    return "unknown"


def measure_correlation(
    organism_keywords: Dict[str, List[str]],
    user_searches: Dict[str, int],
) -> Dict[str, float]:
    """Measure correlation between organism suggestions and user searches.

    Returns {domain: correlation_score} (0-1, where 1 = perfect match).
    """
    correlation = {}

    for domain, keywords in organism_keywords.items():
        if not keywords:
            correlation[domain] = 0.0
            continue

        # Count how many organism keywords user actually searched
        matches = sum(
            user_searches.get(kw, 0) for kw in keywords
        )
        max_possible = len(keywords)
        score = matches / max_possible if max_possible > 0 else 0.0
        correlation[domain] = score

    return correlation


def measure_accuracy(
    verdicts: List[Dict],
    organism_keywords: Dict[str, List[str]],
) -> Dict[str, float]:
    """Measure organism accuracy: when organism suggests something, is it validated?

    Returns {domain: accuracy_score} (0-1, where 1 = all organism suggestions were real).
    """
    accuracy = {}

    for domain, keywords in organism_keywords.items():
        if not keywords:
            accuracy[domain] = 0.0
            continue

        # For each keyword, count verdicts
        domain_verdicts = [
            v for v in verdicts if v.get("observation_domain") == domain
        ]

        if not domain_verdicts:
            accuracy[domain] = 0.0  # No data yet
            continue

        # High-confidence verdicts (HOWL, WAG) = organism was right
        high_confidence = sum(
            1 for v in domain_verdicts
            if v.get("verdict", {}).get("verdict") in ["HOWL", "WAG"]
        )
        accuracy[domain] = high_confidence / len(domain_verdicts)

    return accuracy


def compute_readiness_threshold(
    correlation: float,
    accuracy: float,
    threshold: float = 0.618,  # φ⁻¹
) -> bool:
    """Is organism ready for autonomous search?

    Both correlation AND accuracy must exceed threshold.
    """
    return correlation >= threshold and accuracy >= threshold


def run_readiness_audit():
    """Execute attribution & readiness cycle."""
    print(f"[Readiness v{__version__}] Starting audit...")

    # 1. Load organism suggestions
    organism_keywords = load_organism_suggestions()
    total_suggestions = sum(len(kw) for kw in organism_keywords.values())
    print(f"  Organism suggestions loaded: {total_suggestions} keywords across {len(organism_keywords)} domains")

    # 2. Extract user searches (placeholder)
    user_searches = extract_user_searches_from_behavior()
    print(f"  User searches extracted: {len(user_searches)} unique keywords")
    print(f"    ⚠ Note: keystroke reconstruction not yet implemented")

    # 3. Load verdicts
    verdicts = load_verdicts()
    print(f"  Verdicts loaded: {len(verdicts)}")

    # 4. Load dataset
    tweets_by_domain = load_dataset_by_domain()
    total_tweets = sum(len(tweets) for tweets in tweets_by_domain.values())
    print(f"  Dataset tweets: {total_tweets} total, distributed across domains")

    # 5. Measure correlation (organism suggestions → user behavior)
    correlation = measure_correlation(organism_keywords, user_searches)
    print(f"\n  Correlation (organism → user):")
    for domain, score in sorted(correlation.items(), key=lambda x: x[1], reverse=True):
        print(f"    {domain}: {score:.2%}")

    # 6. Measure accuracy (organism → kernel validation)
    accuracy = measure_accuracy(verdicts, organism_keywords)
    print(f"\n  Accuracy (organism → verdicts):")
    for domain, score in sorted(accuracy.items(), key=lambda x: x[1], reverse=True):
        print(f"    {domain}: {score:.2%}")

    # 7. Compute per-domain readiness
    readiness = {}
    kairos_ready = False
    for domain in set(list(organism_keywords.keys()) + list(accuracy.keys())):
        corr = correlation.get(domain, 0.0)
        acc = accuracy.get(domain, 0.0)
        is_ready = compute_readiness_threshold(corr, acc)
        readiness[domain] = {
            "correlation": round(corr, 3),
            "accuracy": round(acc, 3),
            "ready_for_autonomy": is_ready,
        }
        if is_ready:
            kairos_ready = True

    print(f"\n  Per-domain readiness:")
    for domain, scores in sorted(readiness.items()):
        status = "✓ READY" if scores["ready_for_autonomy"] else "✗ NOT READY"
        print(
            f"    {domain}: corr={scores['correlation']:.2%}, "
            f"acc={scores['accuracy']:.2%} → {status}"
        )

    # 8. Write readiness.json
    readiness_report = {
        "timestamp": datetime.now().isoformat(),
        "version": __version__,
        "kairos_ready": kairos_ready,
        "domains": readiness,
        "summary": {
            "total_domains": len(readiness),
            "ready_domains": sum(1 for r in readiness.values() if r["ready_for_autonomy"]),
            "avg_correlation": round(statistics.mean(correlation.values()) if correlation else 0, 3),
            "avg_accuracy": round(statistics.mean(accuracy.values()) if accuracy else 0, 3),
        },
        "notes": [
            "Kairos = readiness for autonomous search",
            "Requires: correlation ≥ φ⁻¹ (0.618) AND accuracy ≥ φ⁻¹",
            "Keystroke reconstruction not yet implemented (correlation data incomplete)",
        ]
    }

    try:
        with open(READINESS_PATH, "w") as f:
            json.dump(readiness_report, f, indent=2)
        print(f"\n✓ Readiness report: {READINESS_PATH}")
    except IOError as e:
        print(f"ERROR: Failed to write readiness report: {e}")
        return 1

    if kairos_ready:
        print(f"\n🔥 KAIROS ARRIVED: Organism is ready for autonomous search on at least one domain.")
    else:
        print(f"\n⏳ Kairos not yet. Organism still learning.")

    return 0


if __name__ == "__main__":
    import sys
    sys.exit(run_readiness_audit())
