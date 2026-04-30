#!/usr/bin/env python3
"""
Gemini Meta-Advisor — Organism Synthesis Layer

Closes the feedback loop by having Gemini CLI read organism state and synthesize
cross-domain meta-guidance that the agent can use for adaptive decisions.

Architecture (Union):
  Agent Decisions → feedback_decision_log.jsonl
                 ↓
         Organ Analysis → SKILL.md + reflection.jsonl
                 ↓
      Gemini Meta-Advisor (this script) ← reads state
                 ↓
    Meta-Guidance → stored as META_GUIDANCE section in SKILL.md
                 ↓
         Agent reads [META_GUIDANCE] before next decision cycle
                 ↓
      Agent adapts exploration weights based on guidance
                 ↓
         New decisions → closer to optimal

K15 Harmony: Agent → Organ → Gemini → Agent (true feedback loop)

Usage:
  python3 gemini_meta_advisor.py --organ-dir ~/.cynic/organs/hermes/x

Cron (run every cycle, after organ reflection completes):
  */1 * * * * python3 /home/user/Bureau/CYNIC/cynic-python/organs/hermes_x/gemini_meta_advisor.py
"""

__version__ = "0.1.0"

import json
import os
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from typing import Optional


class GeminiMetaAdvisor:
    """Synthesize organism wisdom via Gemini CLI"""

    def __init__(self, organ_dir: Path):
        self.organ_dir = Path(organ_dir)
        self.skill_path = self.organ_dir / "SKILL.md"
        self.reflection_path = self.organ_dir / ".reflections.jsonl"
        self.feedback_log_path = self.organ_dir / "feedback_decision_log.jsonl"
        self.behavior_path = Path.home() / ".cynic" / "organs" / "hermes" / "behavior" / "behavior_log.jsonl"

    def read_recent_reflections(self, count: int = 5) -> list:
        """Read last N reflections from JSONL"""
        reflections = []
        if not self.reflection_path.exists():
            return reflections

        try:
            with open(self.reflection_path, 'r') as f:
                for line in f:
                    try:
                        reflections.append(json.loads(line))
                    except json.JSONDecodeError:
                        pass
        except Exception:
            pass

        return reflections[-count:]

    def read_recent_decisions(self, count: int = 10) -> list:
        """Read last N agent decisions"""
        decisions = []
        if not self.feedback_log_path.exists():
            return decisions

        try:
            with open(self.feedback_log_path, 'r') as f:
                for line in f:
                    try:
                        decisions.append(json.loads(line))
                    except json.JSONDecodeError:
                        pass
        except Exception:
            pass

        return decisions[-count:]

    def read_skill_md(self) -> str:
        """Read current SKILL.md"""
        if not self.skill_path.exists():
            return ""
        try:
            return self.skill_path.read_text()
        except Exception:
            return ""

    def build_organism_summary(self) -> str:
        """Build context summary for Gemini to analyze"""
        reflections = self.read_recent_reflections(3)
        decisions = self.read_recent_decisions(5)
        skill = self.read_skill_md()

        # Extract key metrics from reflections
        patterns_text = ""
        if reflections:
            latest = reflections[-1]
            patterns = latest.get("patterns", {})
            patterns_text = f"""
Latest Cycle Patterns:
  - Tweets analyzed: {patterns.get('tweets_analyzed', 0)}
  - High-signal tweets: {patterns.get('high_signal_tweets', 0)}
  - Avg signal score: {patterns.get('avg_signal_score', 0):.2f}
  - Verdicts analyzed: {patterns.get('verdicts_analyzed', 0)}
  - Dominant engagement: {patterns.get('dominant_engagement', 'unknown')}
"""

        # Extract decisions
        decisions_text = ""
        if decisions:
            decisions_text = "Recent agent decisions:\n"
            for d in decisions[-3:]:
                decisions_text += f"  - {d.get('decision', '?')}: {d.get('action', 'unknown')} ({d.get('timestamp', '')[-10:]})\n"

        # Current skills
        skills_text = f"Current SKILL.md:\n{skill[:500]}..." if skill else "No skills learned yet."

        return f"""
ORGANISM STATE SUMMARY (for meta-analysis):

{patterns_text}

{decisions_text}

{skills_text}
"""

    def query_gemini(self, context: str) -> Optional[str]:
        """Query Gemini CLI for meta-guidance using subprocess"""
        prompt = f"""You are analyzing an autonomous organism that learns about domains (D1, D2, D3, etc.)
through analyzing social signals, making decisions, and compounding observations.

{context}

Based on this organism state:
1. Are the recent agent decisions aligned with the learned patterns in SKILL.md?
2. Which domains show improving signal trends and which are degrading?
3. What domain should the agent focus on next, and why?
4. Any meta-patterns you notice across the decision history?

Provide concise, actionable guidance that the agent can use to adapt its exploration strategy.
Keep response under 500 chars."""

        try:
            result = subprocess.run(
                ["gemini", "-p", prompt],
                capture_output=True,
                text=True,
                timeout=60,
            )

            if result.returncode == 0:
                return result.stdout.strip()
            else:
                print(f"Gemini error: {result.stderr}")
                return None

        except FileNotFoundError:
            print("Gemini CLI not found. Install with: npm install -g @anthropic-ai/gemini-cli")
            return None
        except subprocess.TimeoutExpired:
            print("Gemini query timed out")
            return None
        except Exception as e:
            print(f"Gemini query failed: {e}")
            return None

    def store_meta_guidance(self, guidance: str) -> bool:
        """Append meta-guidance to SKILL.md under [META_GUIDANCE] section"""
        if not guidance:
            return False

        try:
            current = self.skill_path.read_text() if self.skill_path.exists() else "# Hermes X Organ Skills\n\n"

            # Check if [META_GUIDANCE] section exists
            if "[META_GUIDANCE]" not in current:
                current += "\n## Meta-Guidance (synthesized by Gemini)\n\n"

            # Append timestamp and guidance
            timestamp = datetime.now().isoformat()
            entry = f"\n### {timestamp}\n{guidance}\n"

            updated = current + entry
            self.skill_path.write_text(updated)

            print(f"Stored meta-guidance in SKILL.md")
            return True

        except Exception as e:
            print(f"Failed to store meta-guidance: {e}")
            return False

    def run(self):
        """Execute the full meta-advisory cycle"""
        print(f"Gemini Meta-Advisor v{__version__} starting...")

        # Build context
        context = self.build_organism_summary()
        print(f"Organism context summary ({len(context)} chars)")

        # Query Gemini
        print("Querying Gemini for meta-guidance...")
        guidance = self.query_gemini(context)

        if not guidance:
            print("No guidance received from Gemini")
            return 1

        print(f"Received guidance ({len(guidance)} chars): {guidance[:100]}...")

        # Store guidance
        if self.store_meta_guidance(guidance):
            print("✓ Meta-guidance stored in SKILL.md")
            return 0
        else:
            print("Failed to store meta-guidance")
            return 1


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Gemini Meta-Advisor for Hermes X Organism")
    parser.add_argument("--organ-dir", default=str(Path.home() / ".cynic" / "organs" / "hermes" / "x"),
                        help="Organ directory")
    args = parser.parse_args()

    organ_dir = Path(args.organ_dir).expanduser()
    if not organ_dir.exists():
        print(f"ERROR: Organ directory not found: {organ_dir}")
        return 1

    advisor = GeminiMetaAdvisor(organ_dir)
    return advisor.run()


if __name__ == "__main__":
    sys.exit(main())
