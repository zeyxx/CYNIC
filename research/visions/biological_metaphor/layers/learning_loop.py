"""
CYNIC Learning Loop Layer (Layer 5) — Feedback and judgment improvement.

Part of the 10-layer organism architecture, the learning loop enables
CYNIC to improve judgment quality through continuous feedback and refinement.

Learning Mechanisms:
    Q-Table Learning: Reinforcement learning on Dog judgments
    Feedback Integration: Incorporates community acceptance/rejection
    Axiom Refinement: Updates axiom weights based on outcome
    Session Tracking: Aggregates experience across judgment cycles

Feedback Signals:
    Community Vote: Did proposal pass or fail?
    Execution Result: Did action succeed or fail?
    Long-term Outcome: Did decision prove wise over time?
    Consensus Strength: Did Dog voting align with outcome?

Typical usage:
    from cynic.kernel.organism.layers import LearningLoop
    learning = LearningLoop(q_table)
    await learning.record_outcome(judgment, feedback)

See Also:
    cynic.kernel.organism.layers: 10-layer organism architecture
    cynic.kernel.organism.brain.learning.unified_learning: Q-Table and session management
    cynic.kernel.core.unified_state: Judgment records for learning
"""
