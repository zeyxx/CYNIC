"""Verdict pattern extraction for Gemini learning loops."""

from __future__ import annotations

from collections import defaultdict
from datetime import datetime, timezone


def extract_patterns(verdicts: list[dict]) -> dict:
    """Analyze observation verdict patterns by narrative and domain."""
    patterns = {
        "by_narrative": defaultdict(lambda: defaultdict(int)),
        "by_domain": defaultdict(lambda: defaultdict(int)),
        "high_confidence_rules": [],
    }

    for verdict in verdicts:
        obs_narratives = verdict.get("observation_narratives", [])
        obs_domain = verdict.get("observation_domain", "unknown")
        verdict_obj = verdict.get("verdict", {})

        if isinstance(verdict_obj, dict):
            verdict_type = verdict_obj.get("verdict", "UNKNOWN")
        else:
            verdict_type = str(verdict_obj)

        for narrative in obs_narratives:
            patterns["by_narrative"][narrative][verdict_type] += 1
        patterns["by_domain"][obs_domain][verdict_type] += 1

    for narrative, verdicts_by_type in patterns["by_narrative"].items():
        total = sum(verdicts_by_type.values())
        if total < 2:
            continue

        positive = verdicts_by_type.get("HOWL", 0) + verdicts_by_type.get("WAG", 0)
        if positive > 0 and total >= 3:
            confidence = positive / total
            if confidence >= 0.618:
                patterns["high_confidence_rules"].append(
                    {
                        "when": f"narratives include '{narrative}'",
                        "then": f"high-signal domain (confidence: {confidence:.2%})",
                        "evidence": f"{positive}/{total} verdicts were WAG/HOWL",
                    }
                )

    return patterns


def render_learned_patterns_section(patterns: dict, now: datetime | None = None) -> str:
    """Render SKILL.md learned patterns from verdict analysis."""
    timestamp = (now or datetime.now(tz=timezone.utc)).isoformat()
    learned_section = "\n## Learned Patterns (from observation verdicts)\n\n"

    if patterns["high_confidence_rules"]:
        learned_section += "High-confidence rules extracted from agent observations:\n\n"
        for rule in patterns["high_confidence_rules"]:
            learned_section += f"- **{rule['when']}** -> {rule['then']}\n"
            learned_section += f"  Evidence: {rule['evidence']}\n"
    else:
        learned_section += "(No high-confidence patterns yet - need more verdict data)\n"

    learned_section += "\n### Verdict Distribution\n\n"

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

    learned_section += f"\n*Last updated: {timestamp}*\n"
    return learned_section


def replace_learned_patterns_section(current_content: str, learned_section: str) -> str:
    """Replace or append the learned patterns section in SKILL.md content."""
    if "## Learned Patterns" in current_content:
        return current_content.split("## Learned Patterns")[0] + learned_section
    return current_content.rstrip() + "\n" + learned_section
