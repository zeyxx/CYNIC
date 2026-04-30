"""Layer 5: APPRENTISSAGE — Learn and reflect."""
import json
from pathlib import Path
from .schema import PatternAnalysis, Reflection

class OrganReflector:
    def __init__(self):
        self.skill_path = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "SKILL.md"
        self.reflections_path = Path.home() / ".cynic" / "organs" / "hermes" / "x" / ".reflections.jsonl"

    def reflect(self, analysis: PatternAnalysis) -> Reflection:
        health = {
            "perception_ok": analysis.tweets_analyzed > 0 or analysis.verdicts_analyzed > 0,
            "transformation_ok": True,
            "analysis_ok": len(analysis.anomalies) == 0,
        }
        is_healthy = all(health.values())

        # Generate skill updates
        skill_updates = []
        if analysis.high_signal_tweets > 0:
            pct = 100 * analysis.high_signal_tweets / analysis.tweets_analyzed
            skill_updates.append(f"High-Signal: {pct:.1f}% of tweets (avg score {analysis.avg_signal_score:.2f})")
        for domain, metric in analysis.domain_metrics.items():
            if metric.confidence >= 0.618:
                skill_updates.append(f"{domain.capitalize()}: {metric.verdict_count} verdicts, confidence {metric.avg_q_score:.3f}")

        return Reflection(
            timestamp=analysis.timestamp,
            cycle=analysis.cycle,
            patterns=analysis,
            compounded_observations=[],
            skill_updates=skill_updates,
            organ_health=health,
            is_healthy=is_healthy,
            diagnosis="All systems nominal" if is_healthy else "Anomalies detected",
        )

    def persist_reflection(self, reflection: Reflection) -> None:
        self.reflections_path.parent.mkdir(parents=True, exist_ok=True)
        with open(self.reflections_path, 'a') as f:
            f.write(json.dumps({
                "timestamp": reflection.timestamp,
                "cycle": reflection.cycle,
                "is_healthy": reflection.is_healthy,
            }) + '\n')

        # Update SKILL.md
        self.skill_path.parent.mkdir(parents=True, exist_ok=True)
        content = f"# Hermes X Organ Skills\n\n## Cycle {reflection.cycle}\n"
        for update in reflection.skill_updates:
            content += f"- {update}\n"
        with open(self.skill_path, 'a') as f:
            f.write(content + "\n")
