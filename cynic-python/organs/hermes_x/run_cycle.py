"""
Hermes X Organ — Full Cycle Orchestration

Runs all 5 layers of the data architecture:
1. PERCEPTION (sensors.py)
2. TRANSFORMATION (transformation.py)
3. STRUCTURATION (schema.py)
4. ANALYSE & COMPRÉHENSION (analysis.py)
5. APPRENTISSAGE (learning.py)

Entry point: __main__
"""

import json
import sys
from pathlib import Path
from datetime import datetime

from .sensors import HermesXSensors
from .transformation import DataTransformer
from .analysis import DataAnalyzer
from .learning import OrganReflector
from .schema import Reflection


class HermesXOrgan:
    """Hermes X organ — perception, analysis, and learning"""

    def __init__(self):
        self.sensors = HermesXSensors()
        self.transformer = DataTransformer()
        self.analyzer = DataAnalyzer()
        self.reflector = OrganReflector()
        self.cycle_count = 0

    def run_cycle(self) -> Reflection:
        """
        Execute one full cycle: perceive → transform → analyze → learn → reflect

        Returns:
            Reflection (wisdom from this cycle)
        """
        print(f"[Hermes X Organ] Cycle {self.cycle_count + 1}", file=sys.stderr)

        # LAYER 1: PERCEPTION
        print("  Layer 1: Perceiving data sources...", file=sys.stderr)
        perception = self.sensors.perceive()
        print(f"    Perceived: {len(perception.tweets)} tweets, {len(perception.verdicts)} verdicts, {len(perception.sessions)} sessions", file=sys.stderr)

        # LAYER 2: TRANSFORMATION
        print("  Layer 2: Cleaning and validating...", file=sys.stderr)
        cleaned = self.transformer.transform(perception)
        quality = cleaned.quality_score()
        print(f"    Quality score: {quality:.2%}", file=sys.stderr)
        print(f"    Retained: {len(cleaned.tweets_valid)} tweets ({cleaned.tweets_dropped} dropped), {len(cleaned.verdicts_valid)} verdicts ({cleaned.verdicts_dropped} dropped)", file=sys.stderr)

        # LAYER 4: ANALYSIS (skip Layer 3 structuration — it's implicit in schema)
        print("  Layer 4: Analyzing patterns...", file=sys.stderr)
        analysis = self.analyzer.analyze(cleaned, self.cycle_count)
        print(f"    Anomalies: {analysis.anomalies}", file=sys.stderr)
        print(f"    Opportunities: {analysis.opportunities}", file=sys.stderr)

        # LAYER 5: LEARNING
        print("  Layer 5: Learning and reflecting...", file=sys.stderr)
        reflection = self.reflector.reflect(analysis)
        self.reflector.persist_reflection(reflection)
        print(f"    Organ health: {reflection.is_healthy}", file=sys.stderr)
        print(f"    Skill updates: {len(reflection.skill_updates)}", file=sys.stderr)

        self.cycle_count += 1
        return reflection


def main():
    """Run one cycle of the Hermes X organ"""
    organ = HermesXOrgan()
    reflection = organ.run_cycle()

    # Output reflection as JSON
    print("\n=== HERMES X ORGAN REFLECTION ===")
    reflection_dict = {
        "timestamp": reflection.timestamp,
        "cycle": reflection.cycle,
        "is_healthy": reflection.is_healthy,
        "diagnosis": reflection.diagnosis,
        "patterns": {
            "tweets_analyzed": reflection.patterns.tweets_analyzed,
            "high_signal_tweets": reflection.patterns.high_signal_tweets,
            "avg_signal_score": reflection.patterns.avg_signal_score,
            "verdicts_analyzed": reflection.patterns.verdicts_analyzed,
            "anomalies": reflection.patterns.anomalies,
            "opportunities": reflection.patterns.opportunities,
            "recommendation": reflection.patterns.recommendation,
        },
        "compounded_observations": reflection.compounded_observations,
        "skill_updates": reflection.skill_updates,
    }
    print(json.dumps(reflection_dict, indent=2))
    print(f"\nReflection persisted to {Path.home() / '.cynic' / 'organs' / 'hermes' / 'x' / '.reflections.jsonl'}", file=sys.stderr)


if __name__ == '__main__':
    main()
