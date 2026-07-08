#!/usr/bin/env python3
"""
Tier 2 INFRASTRUCTURE: Organ-owned Gemini briefing consumer.

Gemini Briefing Consumer — Phase 1 autonomous meta-agent.

Reads lab_briefing_latest.json (produced by lab.py) + verdicts/ directory.
Analyzes: which domains need exploration?
Writes: tasks to agent-tasks/ for Hermes 9B to claim and execute.
Updates: SKILL.md with learned patterns.

K15: Briefing (producer: lab.py) → Gemini (consumer, this script) → Tasks (producer) → Hermes 9B (consumer)

Usage:
  python3 gemini_briefing_consumer.py --organ-dir ~/.cynic/organs/hermes/x

Cron (run every 4 hours):
  hermes-gemini-briefing.service runs this through organs/twitter/src/entrypoints/hermes_x.py
"""

import os
import sys
import argparse
import urllib.request
from pathlib import Path

from organs.twitter.src.adapters import briefing_consumer_io
from organs.twitter.src.adapters.hermes_paths import HERMES_X_DIR
from organs.twitter.src.core.briefing_priorities import analyze_domain_priorities
try:
    from organs.twitter.src.entrypoints._bootstrap import bootstrap_repo_path
except ImportError:
    from _bootstrap import bootstrap_repo_path


PROJECT_ROOT = bootstrap_repo_path()


def load_briefing(organ_dir: str) -> dict:
    """Load lab_briefing_latest.json."""
    return briefing_consumer_io.load_briefing(organ_dir)


def create_task(domain_id: str, briefing: dict, organ_dir: str) -> bool:
    """POST a task to kernel /agent-tasks for Hermes to claim.

    Returns: True if task created, False on error.
    """
    briefing_consumer_io.urllib.request.urlopen = urllib.request.urlopen
    return briefing_consumer_io.create_task(domain_id, briefing, organ_dir)


def update_skill(briefing: dict, organ_dir: str):
    """Update SKILL.md with current domain priorities and calibration status.

    Gemini learns from verdicts and updates hints for Hermes 9B.
    """
    skill_path = Path(organ_dir) / "agent" / "SKILL.md"
    if not skill_path.exists():
        print("SKILL.md not found, skipping update")
        return

    calibration = briefing.get("analyses", {}).get("calibration", {})

    # For now, just log calibration status
    print(f"Calibration: r={calibration.get('r')} (pairs={calibration.get('pairs_found')})")
    print(f"Interpretation: {calibration.get('interpretation')}")

    # Full update would append learned rules to SKILL.md
    # Deferred: requires careful YAML/markdown parsing


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Gemini Briefing Consumer")
    parser.add_argument(
        "--organ-dir",
        default=os.environ.get("X_ORGAN_DIR", str(HERMES_X_DIR)),
        help="Hermes X organ directory",
    )
    return parser


def main():
    args = build_parser().parse_args()
    organ_dir = str(Path(args.organ_dir).expanduser())

    print(f"Loading briefing from {organ_dir}...")
    briefing = load_briefing(organ_dir)
    if not briefing:
        sys.exit(1)

    print(f"Analyzing domain priorities...")
    priorities = analyze_domain_priorities(briefing)

    # Create tasks for under-assigned domains with HIGH priority
    created_count = 0
    for domain_id, analysis in priorities.items():
        if analysis["status"] == "under-assigned" and analysis["priority"] == "HIGH":
            if create_task(domain_id, briefing, organ_dir):
                created_count += 1

    if created_count > 0:
        print(f"Created {created_count} tasks for Hermes 9B")

    print("Updating SKILL.md...")
    update_skill(briefing, organ_dir)

    print("✓ Briefing consumer cycle complete")


if __name__ == "__main__":
    main()
