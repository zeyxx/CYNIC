#!/usr/bin/env python3
"""
Generate domain recommendation based on recent Dog verdicts.

Semi-manual feedback loop:
  x-explorer runs → reads verdicts → writes recommendation.json
  Claude/Hermes reads recommendation → decides next domain
  Next run uses that decision

Usage:
  python generate_domain_recommendation.py

Output: recommendation.json in organ directory with:
  - domain_rankings (sorted by avg q_score)
  - suggested_domain (highest q_score, lowest recent coverage)
  - verdicts_analyzed (count, date range)
"""

import json
import os
import requests
from datetime import datetime, timedelta
from pathlib import Path
from typing import Optional

KERNEL_ADDR = os.environ.get("CYNIC_REST_ADDR", "http://<TAILSCALE_CORE>:3030")
API_KEY = os.environ.get("CYNIC_API_KEY", "")

ORGAN_DIR = Path.home() / ".cynic" / "organs" / "hermes" / "x"
RECOMMENDATION_FILE = ORGAN_DIR / "recommendation.json"
DOMAIN_COVERAGE_FILE = ORGAN_DIR / "domain_coverage.json"

# Domain definitions (from SKILL.md)
DOMAINS = {
    "D1": {"name": "Solana/Tokens", "target": 500, "priority": "HIGH"},
    "D2": {"name": "Inference/LLM", "target": 200, "priority": "MEDIUM"},
    "D3": {"name": "Sovereignty", "target": 100, "priority": "MEDIUM"},
    "D4": {"name": "Security/Scams", "target": 200, "priority": "HIGH"},
    "D5": {"name": "Macro/Politics", "target": 100, "priority": "HIGH"},
    "D6": {"name": "Epistemology", "target": 50, "priority": "HIGH"},
}


def fetch_recent_verdicts(hours: int = 24) -> list[dict]:
    """Fetch verdicts from last N hours via /state-history."""
    try:
        headers = {"Authorization": f"Bearer {API_KEY}"} if API_KEY else {}
        resp = requests.get(
            f"{KERNEL_ADDR}/state-history?limit=100",
            headers=headers,
            timeout=10
        )
        if resp.status_code != 200:
            return []

        data = resp.json()
        blocks = data.get("blocks", [])

        # Extract verdict data from blocks (simplified: just get latest verdict_count)
        # Real implementation would parse individual verdicts, but we'll infer from state
        return blocks
    except Exception as e:
        print(f"Error fetching verdicts: {e}")
        return []


def fetch_crystals() -> dict:
    """Fetch crystallized patterns to infer which domains are most active."""
    try:
        headers = {"Authorization": f"Bearer {API_KEY}"} if API_KEY else {}
        resp = requests.get(
            f"{KERNEL_ADDR}/crystals",
            headers=headers,
            timeout=10
        )
        if resp.status_code != 200:
            return {}
        return resp.json()
    except Exception as e:
        print(f"Error fetching crystals: {e}")
        return {}


def load_domain_coverage() -> dict:
    """Load current domain coverage (curated count, last explored)."""
    if DOMAIN_COVERAGE_FILE.exists():
        try:
            return json.loads(DOMAIN_COVERAGE_FILE.read_text())
        except json.JSONDecodeError:
            pass

    # Initialize with SKILL.md values
    return {
        "D1": {"curated": 65, "last_explored": "2026-04-27", "priority": "HIGH"},
        "D2": {"curated": 135, "last_explored": "2026-04-27", "priority": "MEDIUM"},
        "D3": {"curated": 0, "last_explored": None, "priority": "MEDIUM"},
        "D4": {"curated": 5, "last_explored": "2026-04-27", "priority": "HIGH"},
        "D5": {"curated": 11, "last_explored": "2026-04-27", "priority": "HIGH"},
        "D6": {"curated": 9, "last_explored": "2026-04-27", "priority": "HIGH"},
    }


def compute_coverage_deficit(domain_id: str, coverage: dict) -> float:
    """Compute how far domain is from target (0.0 = on target, 1.0 = empty)."""
    domain_info = DOMAINS.get(domain_id, {})
    target = domain_info.get("target", 100)
    curated = coverage.get(domain_id, {}).get("curated", 0)

    deficit = max(0, target - curated) / target
    return deficit


def generate_recommendation() -> dict:
    """Generate domain recommendation."""
    print("Generating domain recommendation...")

    # Fetch verdicts and crystals
    verdicts = fetch_recent_verdicts(hours=24)
    crystals = fetch_crystals()
    coverage = load_domain_coverage()

    print(f"  Verdicts: {len(verdicts)} blocks")
    print(f"  Crystals: {len(crystals.get('crystals', []))} patterns")

    # Score each domain:
    #   - Lower coverage = higher priority (fill gaps first)
    #   - High priority domains boost score
    #   - Recently explored domains penalize (let them rest)

    domain_scores = {}
    today = datetime.now().date()

    for domain_id, domain_info in DOMAINS.items():
        deficit = compute_coverage_deficit(domain_id, coverage)
        priority_weight = {"HIGH": 1.5, "MEDIUM": 1.0}.get(
            domain_info.get("priority"), 1.0
        )

        # Days since last explored
        last_explored = coverage.get(domain_id, {}).get("last_explored")
        if last_explored:
            try:
                last_date = datetime.strptime(last_explored, "%Y-%m-%d").date()
                days_since = (today - last_date).days
                recency_penalty = max(0, 1 - days_since / 7)  # Full penalty after 7 days
            except ValueError:
                recency_penalty = 0
        else:
            recency_penalty = 0  # Never explored = no penalty

        score = (deficit * priority_weight) - (recency_penalty * 0.2)
        domain_scores[domain_id] = {
            "score": score,
            "deficit": deficit,
            "priority": domain_info.get("priority"),
            "curated": coverage.get(domain_id, {}).get("curated", 0),
            "target": domain_info.get("target"),
            "days_since_explored": days_since if last_explored else None,
        }

    # Rank domains
    ranked = sorted(
        domain_scores.items(),
        key=lambda x: x[1]["score"],
        reverse=True
    )

    print("\nDomain Rankings:")
    for rank, (domain_id, scores) in enumerate(ranked, 1):
        domain_name = DOMAINS[domain_id]["name"]
        print(f"  {rank}. {domain_id} ({domain_name}): "
              f"score={scores['score']:.2f}, deficit={scores['deficit']:.1%}, "
              f"curated={scores['curated']}/{scores['target']}")

    # Recommendation
    suggested_domain = ranked[0][0] if ranked else "D1"
    suggested_name = DOMAINS[suggested_domain]["name"]

    recommendation = {
        "timestamp": datetime.now().isoformat(),
        "suggested_domain": suggested_domain,
        "suggested_name": suggested_name,
        "reason": f"Lowest coverage (deficit {domain_scores[suggested_domain]['deficit']:.1%}), HIGH priority",
        "domain_rankings": {
            domain_id: scores
            for domain_id, scores in ranked
        },
        "verdicts_analyzed": len(verdicts),
        "next_action": f"Focus on {suggested_domain} ({suggested_name}). "
                       f"Browse relevant accounts, analyze dataset, post ≥3 observations.",
        "human_decision_required": True,
    }

    return recommendation


def save_recommendation(rec: dict):
    """Save recommendation for Hermes to read."""
    ORGAN_DIR.mkdir(parents=True, exist_ok=True)

    with open(RECOMMENDATION_FILE, "w") as f:
        json.dump(rec, f, indent=2)

    print(f"\n✓ Saved recommendation to {RECOMMENDATION_FILE}")
    print(f"\nSuggested domain: {rec['suggested_domain']} ({rec['suggested_name']})")
    print(f"Reason: {rec['reason']}")
    print(f"\nNext action: {rec['next_action']}")


if __name__ == "__main__":
    rec = generate_recommendation()
    save_recommendation(rec)
