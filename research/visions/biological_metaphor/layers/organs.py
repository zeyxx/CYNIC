"""
CYNIC Organs Layer (Layer 2) — Specialized judge subsystems and decision organs.

Part of the 10-layer organism architecture, the organs layer houses
the 11 Dogs (Sefirot judges) that evaluate proposals from complementary angles.

Organ Systems:
    11 Dogs: Independent judge neurons (Chokmah, Binah, Gevurah, Chesed, etc.)
    Voting Consensus: PBFT Byzantine consensus aggregation
    Verdict Composition: Q-Score calculation (HOWL/WAG/GROWL/BARK)
    Confidence Scoring: Aggregates individual Dog confidence

Dog Roles:
    Chokmah (Wisdom): Intuitive, big-picture judgment
    Binah (Understanding): Analytical, detail-oriented analysis
    Gevurah (Severity): Critical, adversarial veto power
    Chesed (Mercy): Compassionate, community-focused perspective
    ... (7 more specialized judges)

Typical usage:
    from cynic.kernel.organism.layers import OrganSystem
    organs = OrganSystem()
    verdict = await organs.vote(cell)

See Also:
    cynic.kernel.organism.layers: 10-layer organism architecture
    cynic.brain.cognition.neurons: 11 Dogs implementation
    cynic.judges: Judge contracts and implementations
"""
