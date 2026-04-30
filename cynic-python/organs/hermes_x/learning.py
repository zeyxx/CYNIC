"""
Layer 5: APPRENTISSAGE — Extract wisdom, update SKILL.md, compound observations.

Inputs: PatternAnalysis
Outputs: Reflection (typed contract)

This layer:
1. Reads SKILL.md to understand current knowledge
2. Updates SKILL.md with new insights
3. Compounds observations across cycles
4. Determines organ health
"""

import json
from pathlib import Path
from datetime import datetime
from .schema import PatternAnalysis, Reflection


class SkillUpdater:
    """Update SKILL.md based on learned patterns"""

    def __init__(self, skill_path: Path = None):
        self.skill_path = skill_path or (
            Path.home() / ".cynic" / "organs" / "hermes" / "x" / "SKILL.md"
        )

    def read_skill(self) -> str:
        """Read current SKILL.md content"""
        if self.skill_path.exists():
            return self.skill_path.read_text()
        return "# Hermes X Organ Skills\n\nNo skills learned yet.\n"

    def write_skill(self, content: str) -> None:
        """Write updated SKILL.md"""
        self.skill_path.parent.mkdir(parents=True, exist_ok=True)
        self.skill_path.write_text(content)

    def extract_skill_updates(self, analysis: PatternAnalysis) -> list:
        """
        Extract new skills from pattern analysis.

        Returns:
            List of new skill entries to add to SKILL.md
        """
        updates = []

        # Skill: high-signal tweet detection
        if analysis.high_signal_tweets > 0:
            pct = 100 * analysis.high_signal_tweets / analysis.tweets_analyzed if analysis.tweets_analyzed > 0 else 0
            updates.append(
                f"- **High-Signal Detection:** {pct:.1f}% of tweets are high-signal (score > 0). "
                f"Average signal score: {analysis.avg_signal_score:.2f}"
            )

        # Skill: domain expertise
        for domain, metric in analysis.domain_metrics.items():
            if metric.confidence >= 0.618:
                updates.append(
                    f"- **{domain.capitalize()} Domain:** {metric.verdict_count} verdicts, "
                    f"avg confidence {metric.avg_q_score:.3f}, "
                    f"dominant type: {metric.howl_count} HOWL, {metric.bark_count} BARK"
                )

        # Skill: anomaly awareness
        if analysis.anomalies:
            updates.append(
                f"- **Anomalies Detected:** {', '.join(analysis.anomalies)}. "
                f"Investigate root causes in next cycle."
            )

        return updates

    def update_skill(self, analysis: PatternAnalysis) -> list:
        """
        Update SKILL.md with new learnings.

        Returns:
            List of new skill entries added
        """
        current_skill = self.read_skill()
        updates = self.extract_skill_updates(analysis)

        if not updates:
            return []

        # Append new skills
        new_content = current_skill + "\n\n## Cycle " + str(analysis.cycle) + "\n"
        for update in updates:
            new_content += update + "\n"

        self.write_skill(new_content)
        return updates


class OrganReflector:
    """Compound observations and reflect on organ health"""

    def __init__(self, reflections_path: Path = None):
        self.reflections_path = reflections_path or (
            Path.home() / ".cynic" / "organs" / "hermes" / "x" / ".reflections.jsonl"
        )

    def read_prior_reflections(self, max_cycles: int = 10) -> list:
        """Read prior cycle reflections"""
        reflections = []

        if self.reflections_path.exists():
            try:
                with open(self.reflections_path, 'r') as f:
                    for line in f:
                        try:
                            obj = json.loads(line)
                            reflections.append(obj)
                        except json.JSONDecodeError:
                            pass
            except Exception:
                pass

        return reflections[-max_cycles:]

    def compound_observations(self, prior_reflections: list) -> list:
        """
        Compound observations from prior cycles to show trends.

        Returns:
            List of compounded observations (e.g., "Signal improving for 3 cycles")
        """
        observations = []

        if not prior_reflections:
            return observations

        # Trend: is avg signal score improving?
        scores = [r.get("patterns", {}).get("avg_signal_score", 0) for r in prior_reflections]
        if len(scores) >= 3:
            recent = scores[-3:]
            if all(s > 0 for s in recent) and recent[-1] > recent[0]:
                observations.append("Signal score improving over last 3 cycles")
            elif all(s < 0 for s in recent) and recent[-1] < recent[0]:
                observations.append("Signal score degrading over last 3 cycles")

        return observations

    def assess_organ_health(self, analysis: PatternAnalysis, prior_reflections: list) -> dict:
        """
        Assess health of the organ.

        Returns:
            Health metrics
        """
        health = {
            "perception_ok": analysis.tweets_analyzed > 0 or analysis.verdicts_analyzed > 0,
            "transformation_ok": True,  # If we got here, transformation worked
            "analysis_ok": len(analysis.anomalies) == 0,
            "learning_ok": True,
        }

        # Overall health: all subsystems OK
        is_healthy = all(health.values())

        return {
            "subsystems": health,
            "is_healthy": is_healthy,
            "diagnosis": "All systems nominal" if is_healthy else "Anomalies detected in analysis",
        }

    def reflect(self, analysis: PatternAnalysis) -> Reflection:
        """
        Create a reflection from analysis.

        Compounds prior cycles, assesses health, extracts lessons.

        Returns:
            Reflection (wisdom from this cycle)
        """
        prior_reflections = self.read_prior_reflections()
        compounded = self.compound_observations(prior_reflections)
        health = self.assess_organ_health(analysis, prior_reflections)

        # Update SKILL.md and get updates
        updater = SkillUpdater()
        skill_updates = updater.update_skill(analysis)

        return Reflection(
            timestamp=analysis.timestamp,
            cycle=analysis.cycle,
            patterns=analysis,
            compounded_observations=compounded,
            skill_updates=skill_updates,
            organ_health=health["subsystems"],
            is_healthy=health["is_healthy"],
            diagnosis=health["diagnosis"],
        )

    def persist_reflection(self, reflection: Reflection) -> None:
        """Persist reflection to history for compounding"""
        self.reflections_path.parent.mkdir(parents=True, exist_ok=True)

        # Convert to dict for JSON serialization
        data = {
            "timestamp": reflection.timestamp,
            "cycle": reflection.cycle,
            "patterns": {
                "tweets_analyzed": reflection.patterns.tweets_analyzed,
                "avg_signal_score": reflection.patterns.avg_signal_score,
                "verdicts_analyzed": reflection.patterns.verdicts_analyzed,
                "anomalies": reflection.patterns.anomalies,
            },
            "is_healthy": reflection.is_healthy,
        }

        with open(self.reflections_path, 'a') as f:
            f.write(json.dumps(data) + '\n')
