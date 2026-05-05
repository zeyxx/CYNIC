#!/usr/bin/env python3
"""
Feedback decision log — Hermes documents what it chose to do based on recommendations.

Semi-manual loop requires transparency:
  1. generate_domain_recommendation.py writes recommendation.json
  2. Claude/Hermes reads it, decides what to do
  3. Hermes calls this script to log the decision
  4. Next cron cycle reads the log

Usage:
    python feedback_decision_log.py --decision D1 --reason "High coverage deficit, HIGH priority" --action "Browse @gcrtrd recovery scammers"
"""

import argparse
import json
from datetime import datetime
from pathlib import Path

ORGAN_DIR = Path.home() / ".cynic" / "organs" / "hermes" / "x"
FEEDBACK_LOG = ORGAN_DIR / "feedback_decisions.jsonl"


def log_decision(decision_domain: str, reason: str, action: str, accepted: bool = True):
    """Log a domain selection decision."""
    ORGAN_DIR.mkdir(parents=True, exist_ok=True)

    entry = {
        "timestamp": datetime.now().isoformat(),
        "session_decision": decision_domain,
        "reason": reason,
        "action": action,
        "accepted_recommendation": accepted,
    }

    # Append to log
    with open(FEEDBACK_LOG, "a") as f:
        f.write(json.dumps(entry) + "\n")

    print(f"✓ Logged decision: {decision_domain}")
    print(f"  Action: {action}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Log Hermes domain decision")
    parser.add_argument("--decision", required=True, help="Domain ID (D1-D6)")
    parser.add_argument("--reason", required=True, help="Why this domain")
    parser.add_argument("--action", required=True, help="What Hermes will do")
    parser.add_argument("--reject", action="store_true", help="Rejected the recommendation")

    args = parser.parse_args()

    log_decision(
        decision_domain=args.decision,
        reason=args.reason,
        action=args.action,
        accepted=not args.reject,
    )
