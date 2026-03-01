"""
CYNIC Autonomy Layer (Layer 6) — Independent decision-making and tier-based control.

Part of the 10-layer organism architecture, the autonomy layer governs
how CYNIC makes independent decisions at different scales and confidence levels.

Autonomy Tiers:
    Tier 0 (Locked): No autonomous action, human approval required
    Tier 1 (Supervised): Autonomous within defined parameters, human oversight
    Tier 2 (Autonomous): Full autonomy within learned boundaries
    Tier 3 (Emergent): Autonomy with emergent behavior (consensus-driven)

This layer integrates:
    - Consciousness level constraints
    - E-Score reputation thresholds
    - Learning loop feedback
    - External authorization constraints

Typical usage:
    from cynic.kernel.organism.layers import AutonomyTiers
    autonomy = AutonomyTiers(tier=Tier.AUTONOMOUS)
    can_act = autonomy.can_act(judgment, context)

See Also:
    cynic.kernel.organism.layers: 10-layer organism architecture
    cynic.kernel.organism.layers.perception: Perceives action constraints
    cynic.kernel.organism.brain.learning: Learning feedback constrains autonomy
"""
