"""
CYNIC Self-Knowledge Layer (Layer 8) — Introspection and metacognition.

Part of the 10-layer organism architecture, the self-knowledge layer enables
CYNIC to understand its own state, capabilities, and limitations.

Introspection Capabilities:
    Health Monitoring: System resource usage and error rates
    Consciousness Awareness: Current level and latency budget
    Learning State: Q-Table convergence and confidence
    Axiom Alignment: Checking decisions against core values
    E-Score Tracking: Reputation and trust metrics
    Blindspot Identification: Known uncertainties and gaps

Metacognitive Functions:
    Self-Assessment: "How confident am I in this judgment?"
    Uncertainty Quantification: Confidence intervals on decisions
    Capability Discovery: "What types of decisions can I handle?"
    Learning Progress: "Am I improving over time?"

Typical usage:
    from cynic.kernel.organism.layers import SelfKnowledge
    self_know = SelfKnowledge()
    health = self_know.get_health_report()

See Also:
    cynic.kernel.organism.layers: 10-layer organism architecture
    cynic.interfaces.api.routers.introspection: Web API for self-knowledge
    cynic.kernel.observability: Metrics and monitoring
"""
