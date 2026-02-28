"""
CYNIC Dogs — 11 independent judge neurons implementing Sefirot wisdom.

The 11 Dogs represent the Kabbalistic Sefirot tree — 11 perspectives that judge
proposals from complementary angles: wisdom (Chokmah), understanding (Binah),
justice (Gevurah), mercy (Chesed), beauty (Tiphereth), etc.

Each Dog:
- Judges cells/proposals independently
- Returns {decision: bool, reasoning: str, confidence: float}
- Contributes vote to PBFT consensus
- Learns from feedback via Q-Table updates

Dogs Architecture:
    dog_implementations: 11 Judge classes (Chokmah, Binah, Gevurah, etc.)
    judge_interface: JudgeContract base class
    judgment_voting: PBFT consensus aggregation

Typical usage:
    from cynic.brain.cognition.neurons import DOG_REGISTRY
    dog = DOG_REGISTRY.get('chokmah')
    verdict = await dog.judge(cell)

See Also:
    cynic.judges: Alternative judge implementations
    cynic.brain.cognition.cortex: Orchestrator that coordinates dogs
    cynic.brain.learning: Q-Table learning from dog judgments
"""
