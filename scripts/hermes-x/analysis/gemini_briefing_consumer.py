#!/usr/bin/env python3
# Tier 3
"""
Gemini Briefing Consumer — Phase 1 autonomous meta-agent.

Reads lab_briefing_latest.json (produced by lab.py) + verdicts/ directory.
Analyzes: which domains need exploration?
Writes: tasks to agent-tasks/ for Hermes 9B to claim and execute.
Updates: SKILL.md with learned patterns.

K15: Briefing (producer: lab.py) → Gemini (consumer, this script) → Tasks (producer) → Hermes 9B (consumer)

Usage:
  python3 gemini_briefing_consumer.py --organ-dir ~/.cynic/organs/hermes/x

Cron (run every 4 hours):
  0 */4 * * * python3 ~/cynic/scripts/hermes-x/gemini_briefing_consumer.py
"""

import json
import os
import sys
import uuid
from datetime import datetime, timedelta
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "core"))
from hermes_paths import HERMES_X_DIR


def load_briefing(organ_dir: str) -> dict:
    """Load lab_briefing_latest.json."""
    briefing_path = Path(organ_dir) / "analysis" / "lab_briefing_latest.json"
    if not briefing_path.exists():
        print(f"ERROR: Briefing not found: {briefing_path}")
        return {}

    try:
        with open(briefing_path) as f:
            return json.load(f)
    except json.JSONDecodeError as e:
        print(f"ERROR: Failed to parse briefing: {e}")
        return {}


def load_verdicts(organ_dir: str) -> list:
    """Load all verdicts from verdicts/ directory."""
    verdict_dir = Path(organ_dir) / "verdicts"
    verdicts = []

    if verdict_dir.exists():
        for json_file in sorted(verdict_dir.glob("*.json")):
            try:
                with open(json_file) as f:
                    verdicts.append(json.load(f))
            except json.JSONDecodeError:
                continue

    return verdicts


def load_skill(organ_dir: str) -> dict:
    """Load SKILL.md as YAML (parse section-by-section)."""
    skill_path = Path(organ_dir) / "agent" / "SKILL.md"
    if not skill_path.exists():
        return {}

    # For now, return empty dict. Full YAML parsing would require pyyaml.
    # Hermes 9B reads this as context, Gemini updates section-by-section.
    try:
        with open(skill_path) as f:
            return {"raw": f.read()}
    except IOError:
        return {}


def analyze_domain_priorities(briefing: dict) -> dict:
    """Analyze domain coverage vs targets. Return domains needing exploration.

    Returns: {domain_id: {"target": X, "current": Y, "deficit": Z, "priority": "HIGH|MEDIUM"}}
    """
    distribution = briefing.get("analyses", {}).get("distribution", {})
    coverage = briefing.get("analyses", {}).get("coverage_vs_ground_truth", {}).get("coverage", {})

    # Hard-coded domain targets (from domains.yaml)
    targets = {
        "D1": 500,
        "D2": 200,
        "D3": 100,
        "D4": 200,
        "D5": 100,
        "D6": 50,
    }

    priorities = {
        "D1": "MONITOR",  # over-assigned, reduce noise
        "D2": "MEDIUM",
        "D3": "MEDIUM",
        "D4": "HIGH",     # security signals
        "D5": "HIGH",     # macro signals
        "D6": "HIGH",     # epistemology signals (severely under-assigned)
    }

    analysis = {}
    for domain_id, target in targets.items():
        current = distribution.get(domain_id, {}).get("count", 0)
        deficit = max(0, target - current)
        status = "aligned" if deficit < target * 0.2 else "under-assigned"

        analysis[domain_id] = {
            "target": target,
            "current": current,
            "deficit": deficit,
            "status": status,
            "priority": priorities.get(domain_id, "MEDIUM"),
        }

    return analysis


def create_task(domain_id: str, briefing: dict, organ_dir: str) -> bool:
    """POST a task to kernel /agent-tasks for Hermes to claim.

    Returns: True if task created, False on error.
    """
    import urllib.request
    import urllib.error

    addr = os.environ.get("CYNIC_REST_ADDR", "")
    key = os.environ.get("CYNIC_API_KEY", "")
    if not addr or not key:
        print(f"CYNIC_REST_ADDR or CYNIC_API_KEY not set — task dispatch skipped")
        return False

    domain_info = briefing.get("recommendation", {})
    if domain_info.get("domain") == domain_id:
        context = f"Recommended by lab analysis: {domain_info.get('reason')}"
    else:
        context = "Under-assigned domain needing exploration"

    distribution = briefing.get("analyses", {}).get("distribution", {}).get(domain_id, {})
    avg_signal = distribution.get("mean", 0)

    content = (
        f"Explore {domain_id} signals on X (current signal avg: {avg_signal:.2f})\n"
        f"Context: {context}\n"
        f"Actions:\n"
        f"1. Browse X for {domain_id} domain signals\n"
        f"2. Look for patterns in high-engagement tweets\n"
        f"3. Post 3+ observations to /observe\n"
        f"Success: 3+ observations with clear domain signals"
    )

    body = json.dumps({
        "kind": "hermes",
        "domain": domain_id,
        "content": content,
        "agent_id": "gemini-briefing-consumer",
    }).encode("utf-8")

    url = f"http://{addr}/agent-tasks"
    req = urllib.request.Request(
        url, data=body,
        headers={"Authorization": f"Bearer {key}", "Content-Type": "application/json"},
        method="POST",
    )

    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            if resp.status < 300:
                result = json.loads(resp.read())
                task_id = result.get("task_id", "unknown")
                print(f"Created kernel task: {task_id} for {domain_id}")
                return True
            else:
                print(f"Error creating task: {resp.status}")
                return False
    except (urllib.error.URLError, urllib.error.HTTPError, OSError) as e:
        print(f"ERROR: Failed to POST task for {domain_id}: {e}")
        return False


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


def main():
    organ_dir = os.environ.get("X_ORGAN_DIR", str(HERMES_X_DIR))

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
