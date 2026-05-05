#!/usr/bin/env python3
"""
Extract framing from behavior_log.jsonl and kernel verdicts.

Framing = domain understanding. What domains does the organism care about?
What signals matter in each domain? When do you engage?

Uses:
1. behavior_log.jsonl window names/context to infer active domains
2. kernel verdicts to measure what was judged valuable
3. behavioral profile temporal patterns for domain-specific timing

Output: framing.json with per-domain priority, search intent, temporal patterns.
"""

__version__ = "0.1.0"

import json
import logging
import re
from argparse import ArgumentParser
from collections import defaultdict
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Set

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(name)s: %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("framing-extractor")

# Domain mappings from window content heuristics
DOMAIN_KEYWORDS = {
    "D1": ["supply", "chain", "logistics", "token", "rug", "honeypot", "scam", "audit"],
    "D2": ["pump", "dump", "manipulation", "rug pull", "honeypot", "scam"],
    "D3": ["regulatory", "sec", "compliance", "law", "legal"],
    "D4": ["sovereignty", "decentralization", "freedom", "autonomy"],
    "D5": ["community", "culture", "vibes", "social", "engagement"],
    "D6": ["performance", "metrics", "quality", "signal", "verification"],
}

DOMAIN_DESCRIPTIONS = {
    "D1": "Token/Supply Chain - Rug detection, supply chain integrity",
    "D2": "Honeypot/Scam - Detection and prevention",
    "D3": "Regulatory - Legal compliance, oversight",
    "D4": "Sovereignty - Autonomy and freedom preservation",
    "D5": "Community - Social patterns, culture, vibes",
    "D6": "Verification - Quality metrics, signal validation",
}


def infer_domain_from_context(window_name: str) -> Set[str]:
    """Infer domains from window/application context."""
    domains = set()
    text = (window_name or "").lower()

    for domain, keywords in DOMAIN_KEYWORDS.items():
        if any(kw in text for kw in keywords):
            domains.add(domain)

    # X.com or Twitter context
    if "x.com" in text or "twitter" in text or "search" in text:
        domains.add("D6")  # Search is verification activity

    return domains or {"unknown"}


def extract_domain_activity(behavior_log_path: Path) -> Dict:
    """Extract per-domain activity patterns from behavior log."""
    domain_hours = defaultdict(lambda: defaultdict(int))
    domain_event_count = defaultdict(int)
    domain_first_activity = defaultdict(lambda: 24)
    domain_last_activity = defaultdict(lambda: 0)

    logger.info("analyzing behavior log for domain patterns...")

    try:
        with open(behavior_log_path) as f:
            for i, line in enumerate(f):
                try:
                    event = json.loads(line)
                    window_name = event.get("window_name", "")
                    ts_str = event.get("ts", "")

                    if "+" in ts_str:
                        ts_str = ts_str.split("+")[0]

                    dt = datetime.fromisoformat(ts_str)
                    hour = dt.hour

                    domains = infer_domain_from_context(window_name)
                    for domain in domains:
                        domain_hours[domain][hour] += 1
                        domain_event_count[domain] += 1
                        domain_first_activity[domain] = min(domain_first_activity[domain], hour)
                        domain_last_activity[domain] = max(domain_last_activity[domain], hour)

                except (json.JSONDecodeError, ValueError):
                    pass

                if (i + 1) % 100000 == 0:
                    logger.info("processed %d events...", i + 1)

    except IOError as e:
        logger.error("failed to read behavior log: %s", e)

    # Compute per-domain summaries
    domain_framing = {}
    for domain in set(domain_event_count.keys()):
        if domain == "unknown":
            continue

        hours = domain_hours[domain]
        total_events = domain_event_count[domain]
        peak_hour = max(hours.items(), key=lambda x: x[1])[0] if hours else 0

        domain_framing[domain] = {
            "event_count": total_events,
            "peak_hour": peak_hour,
            "active_hours": sorted([h for h, c in hours.items() if c > 0]),
            "first_activity_hour": int(domain_first_activity[domain]),
            "last_activity_hour": int(domain_last_activity[domain]),
            "description": DOMAIN_DESCRIPTIONS.get(domain, "Unknown domain"),
        }

    return domain_framing


def load_verdicts(verdicts_path: Path) -> List[Dict]:
    """Load kernel verdicts (if available)."""
    verdicts = []

    if not verdicts_path.exists():
        logger.warning("verdicts path not found: %s", verdicts_path)
        return verdicts

    try:
        for verdict_file in verdicts_path.glob("*.json"):
            try:
                with open(verdict_file) as f:
                    verdicts.append(json.load(f))
            except json.JSONDecodeError:
                pass
    except OSError:
        pass

    return verdicts


def extract_domain_priorities(domain_framing: Dict, verdicts: List[Dict]) -> Dict:
    """
    Assign priorities to domains based on:
    1. Behavioral activity (how much you engage)
    2. Verdict quality (what the kernel found valuable)
    """
    priorities = {}

    # Base priority from engagement frequency
    total_events = sum(d.get("event_count", 0) for d in domain_framing.values())

    for domain, profile in domain_framing.items():
        engagement = profile.get("event_count", 0) / max(total_events, 1)
        base_priority = engagement

        # Boost for domains with high-confidence verdicts (HOWL/WAG)
        domain_verdicts = [v for v in verdicts if v.get("observation_domain") == domain]
        if domain_verdicts:
            howl_count = sum(1 for v in domain_verdicts if v.get("verdict", {}).get("verdict") == "HOWL")
            wag_count = sum(1 for v in domain_verdicts if v.get("verdict", {}).get("verdict") == "WAG")
            verdict_quality = (howl_count + 0.5 * wag_count) / max(len(domain_verdicts), 1)
            base_priority = (base_priority + verdict_quality) / 2

        priorities[domain] = {
            "priority_score": round(base_priority, 3),
            "engagement_percent": round(100 * engagement, 1),
            "verdict_count": len(domain_verdicts),
        }

    return priorities


def main():
    parser = ArgumentParser(description="Extract framing from behavior log and verdicts")
    parser.add_argument(
        "--behavior-log",
        type=Path,
        default=Path.home() / ".cynic" / "organs" / "hermes" / "behavior" / "behavior_log.jsonl",
    )
    parser.add_argument(
        "--verdicts",
        type=Path,
        default=Path.home() / ".cynic" / "organs" / "hermes" / "x" / "observation-verdicts",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path.home() / ".cynic" / "organs" / "hermes" / "x" / "framing.json",
    )
    args = parser.parse_args()

    if not args.behavior_log.exists():
        logger.error("behavior log not found: %s", args.behavior_log)
        return 1

    logger.info("extracting framing from behavior patterns and verdicts")

    # Extract domain activity
    domain_framing = extract_domain_activity(args.behavior_log)
    logger.info("identified %d domains from behavior context", len(domain_framing))

    # Load verdicts
    verdicts = load_verdicts(args.verdicts)
    logger.info("loaded %d verdicts from kernel", len(verdicts))

    # Compute priorities
    domain_priorities = extract_domain_priorities(domain_framing, verdicts)

    # Assemble framing
    framing = {
        "version": __version__,
        "timestamp": datetime.now().isoformat(),
        "total_events_analyzed": sum(d.get("event_count", 0) for d in domain_framing.values()),
        "domains": {},
    }

    for domain in sorted(domain_framing.keys()):
        activity = domain_framing[domain]
        priority = domain_priorities.get(domain, {})

        framing["domains"][domain] = {
            "description": activity.get("description"),
            "priority_score": priority.get("priority_score", 0),
            "engagement_percent": priority.get("engagement_percent", 0),
            "peak_hour": activity.get("peak_hour"),
            "active_hours": activity.get("active_hours"),
            "first_activity_hour": activity.get("first_activity_hour"),
            "last_activity_hour": activity.get("last_activity_hour"),
            "event_count": activity.get("event_count"),
            "verdict_count": priority.get("verdict_count", 0),
        }

    # Write output
    args.output.parent.mkdir(parents=True, exist_ok=True)
    try:
        with open(args.output, "w") as f:
            json.dump(framing, f, indent=2)
        logger.info("✓ framing written: %s", args.output)
    except IOError as e:
        logger.error("failed to write framing: %s", e)
        return 1

    # Summary
    logger.info("\nFRAMING SUMMARY (Domain Priorities):\n")
    for domain in sorted(framing["domains"].items(), key=lambda x: x[1]["priority_score"], reverse=True):
        d_name, d_data = domain
        logger.info(
            "  %s (priority=%.3f): %d events, peak=%dh, verdicts=%d",
            d_name,
            d_data["priority_score"],
            d_data["event_count"],
            d_data["peak_hour"],
            d_data["verdict_count"],
        )

    return 0


if __name__ == "__main__":
    import sys

    sys.exit(main())
