#!/usr/bin/env python3
"""
Gemini Learning from Observation Verdicts — K15 Seam 2 feedback synthesis.

Reads observation verdicts from organ-local observation-verdicts/ directory.
Extracts patterns: which narratives + signals lead to accurate findings?
Updates SKILL.md with learned rules for Hermes 9B.

This closes the learning loop:
  Kernel judges observation
    ↓
  Verdict stored with narrative + signal metadata
    ↓
  Gemini reads verdicts, clusters by outcome
    ↓
  Extracts rules: "narratives=[rug_warning] → D4" → Hermes learns
    ↓
  SKILL.md updated, Hermes reads next session

Usage:
    python3 gemini_learn_from_verdicts.py --organ-dir ~/.cynic/organs/hermes/x

Environment:
    X_ORGAN_DIR  — organ directory (optional)
"""

import json
import logging
from collections import defaultdict
from datetime import datetime
from pathlib import Path
from typing import Optional

logger = logging.getLogger("gemini-learn")


def load_observation_verdicts(organ_dir: str) -> list[dict]:
    """Load all observation verdicts from organ-local observation-verdicts/ directory."""
    verdict_dir = Path(organ_dir) / "observation-verdicts"
    verdicts = []

    if not verdict_dir.exists():
        logger.info("no observation-verdicts directory yet")
        return verdicts

    try:
        for verdict_file in sorted(verdict_dir.glob("*.json")):
            try:
                with open(verdict_file) as f:
                    verdicts.append(json.load(f))
            except json.JSONDecodeError:
                logger.warning("failed to parse %s", verdict_file.name)
                continue
    except OSError as e:
        logger.warning("failed to scan observation-verdicts: %s", e)

    return verdicts


def extract_patterns(verdicts: list[dict]) -> dict:
    """Analyze verdict patterns.

    Returns {
        "by_narrative": {narrative: {BARK: count, GROWL: count, ...}},
        "by_domain": {domain: {BARK: count, ...}},
        "high_confidence_rules": [{"when": "...", "then": "..."}],
    }
    """
    patterns = {
        "by_narrative": defaultdict(lambda: defaultdict(int)),
        "by_domain": defaultdict(lambda: defaultdict(int)),
        "high_confidence_rules": [],
    }

    for verdict in verdicts:
        obs_narratives = verdict.get("observation_narratives", [])
        obs_domain = verdict.get("observation_domain", "unknown")
        verdict_obj = verdict.get("verdict", {})

        # Handle both dict and string verdict formats
        if isinstance(verdict_obj, dict):
            verdict_type = verdict_obj.get("verdict", "UNKNOWN")
        else:
            verdict_type = str(verdict_obj)

        # Track by narrative
        for narrative in obs_narratives:
            patterns["by_narrative"][narrative][verdict_type] += 1

        # Track by domain
        patterns["by_domain"][obs_domain][verdict_type] += 1

    # Extract rules: narratives that consistently lead to high-confidence verdicts
    for narrative, verdicts_by_type in patterns["by_narrative"].items():
        total = sum(verdicts_by_type.values())
        if total < 2:  # need at least 2 data points
            continue

        # Calculate confidence: how often this narrative appears with HOWL or WAG?
        positive = verdicts_by_type.get("HOWL", 0) + verdicts_by_type.get("WAG", 0)
        negative = verdicts_by_type.get("BARK", 0)

        if positive > 0 and total >= 3:
            confidence = positive / total
            if confidence >= 0.618:  # φ⁻¹ threshold
                patterns["high_confidence_rules"].append(
                    {
                        "when": f"narratives include '{narrative}'",
                        "then": f"high-signal domain (confidence: {confidence:.2%})",
                        "evidence": f"{positive}/{total} verdicts were WAG/HOWL",
                    }
                )

    return patterns


def update_skill_md(organ_dir: str, patterns: dict) -> bool:
    """Update SKILL.md with learned rules.

    Appends a "Learned Patterns" section with extracted rules.
    """
    skill_path = Path(organ_dir) / "SKILL.md"
    if not skill_path.exists():
        logger.warning("SKILL.md not found at %s", skill_path)
        return False

    # Build the learned patterns section
    learned_section = "\n## Learned Patterns (from observation verdicts)\n\n"

    if patterns["high_confidence_rules"]:
        learned_section += "High-confidence rules extracted from agent observations:\n\n"
        for rule in patterns["high_confidence_rules"]:
            learned_section += f"- **{rule['when']}** → {rule['then']}\n"
            learned_section += f"  Evidence: {rule['evidence']}\n"
    else:
        learned_section += "(No high-confidence patterns yet — need more verdict data)\n"

    learned_section += "\n### Verdict Distribution\n\n"

    # Add distribution tables
    if patterns["by_narrative"]:
        learned_section += "**By Narrative Tag:**\n\n"
        learned_section += "| Narrative | HOWL | WAG | GROWL | BARK | Confidence |\n"
        learned_section += "|-----------|------|-----|-------|------|-------------|\n"
        for narrative, verdicts_by_type in patterns["by_narrative"].items():
            total = sum(verdicts_by_type.values())
            positive = verdicts_by_type.get("HOWL", 0) + verdicts_by_type.get("WAG", 0)
            confidence = (positive / total * 100) if total > 0 else 0
            learned_section += (
                f"| {narrative} | {verdicts_by_type.get('HOWL', 0)} | "
                f"{verdicts_by_type.get('WAG', 0)} | {verdicts_by_type.get('GROWL', 0)} | "
                f"{verdicts_by_type.get('BARK', 0)} | {confidence:.0f}% |\n"
            )

    learned_section += f"\n*Last updated: {datetime.utcnow().isoformat()}Z*\n"

    # Read current SKILL.md
    try:
        with open(skill_path) as f:
            current_content = f.read()
    except IOError as e:
        logger.error("failed to read SKILL.md: %s", e)
        return False

    # Check if "Learned Patterns" section already exists
    if "## Learned Patterns" in current_content:
        # Replace existing section
        parts = current_content.split("## Learned Patterns")
        new_content = parts[0] + learned_section
    else:
        # Append to end
        new_content = current_content.rstrip() + "\n" + learned_section

    # Write updated SKILL.md
    try:
        with open(skill_path, "w") as f:
            f.write(new_content)
        logger.info("updated SKILL.md with learned patterns")
        return True
    except IOError as e:
        logger.error("failed to write SKILL.md: %s", e)
        return False


def main():
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s %(name)s: %(message)s",
        datefmt="%H:%M:%S",
    )

    import argparse

    parser = argparse.ArgumentParser(description="Learn from observation verdicts and update SKILL.md")
    parser.add_argument(
        "--organ-dir",
        default=str(Path.home() / ".cynic" / "organs" / "hermes" / "x"),
        help="Organ directory containing observation-verdicts/",
    )
    args = parser.parse_args()

    organ_dir = str(Path(args.organ_dir).expanduser())
    if not Path(organ_dir).exists():
        logger.error("Organ directory not found: %s", organ_dir)
        return

    # Load verdicts
    verdicts = load_observation_verdicts(organ_dir)
    if not verdicts:
        logger.info("no observation verdicts to learn from yet")
        return

    logger.info("analyzing %d observation verdict(s)", len(verdicts))

    # Extract patterns
    patterns = extract_patterns(verdicts)

    # Update SKILL.md
    if update_skill_md(organ_dir, patterns):
        logger.info("✓ learning complete: SKILL.md updated")
    else:
        logger.error("learning failed: could not update SKILL.md")


if __name__ == "__main__":
    main()
