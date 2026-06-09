"""Run a Hermes X organ cycle"""

import json
from .sensors import HermesXSensors
from .transformation import DataTransformer
from .analysis import DataAnalyzer
from .learning import OrganReflector

def run_cycle():
    """Execute one complete organ cycle"""
    sensors = HermesXSensors()
    transformer = DataTransformer()
    analyzer = DataAnalyzer()
    reflector = OrganReflector()

    # Layer 1: Perception
    print("═ Layer 1: PERCEPTION")
    raw = sensors.perceive()
    print(f"  Tweets: {len(raw.tweets)}")
    print(f"  Verdicts: {len(raw.verdicts)}")
    print(f"  Agent logs: {len(raw.sessions)}")
    print(f"  Behavior events: {len(raw.behavior)}")

    # Layer 2: Transformation
    print("\n═ Layer 2: TRANSFORMATION")
    cleaned = transformer.transform(raw)
    print(f"  Tweets quality: {cleaned.quality_score():.1%}")
    print(f"  Verdicts: {len(cleaned.verdicts_valid)} valid, {cleaned.verdicts_dropped} dropped")
    if hasattr(cleaned, 'verdict_drop_reasons'):
        print(f"  Drop reasons: {json.dumps(cleaned.verdict_drop_reasons, indent=4)}")

    # Layer 4: Analysis
    print("\n═ Layer 4: ANALYSIS")
    cycle = 1
    patterns = analyzer.analyze(cleaned, cycle)
    print(f"  High-signal tweets: {patterns.high_signal_tweets}/{patterns.tweets_analyzed}")
    print(f"  Verdicts: {patterns.verdicts_analyzed} across {len(patterns.domain_metrics)} domains")
    print(f"  User engagement: {patterns.behaviors_analyzed} events, dominant: {patterns.dominant_engagement}")
    print(f"  Anomalies: {patterns.anomalies}")

    # Layer 5: Learning
    print("\n═ Layer 5: LEARNING")
    reflection = reflector.reflect(patterns)
    print(f"  Is healthy: {reflection.is_healthy}")
    print(f"  Skill updates: {len(reflection.skill_updates)}")
    for update in reflection.skill_updates:
        print(f"    - {update[:80]}...")

    # Persist
    reflector.persist_reflection(reflection)
    print("\n✓ Cycle complete. Reflection persisted.")

if __name__ == "__main__":
    run_cycle()
