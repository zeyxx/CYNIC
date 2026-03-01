"""
CYNIC Perception Layer (Layer 10) — Environmental state unification and sensing.

Part of the 10-layer organism architecture, the perception layer integrates
sensory input from multiple channels into unified environmental models.

Sensory Channels:
    Discord: Community messages, reactions, and voting
    Telegram: Alternative community interface
    HTTP Webhooks: Programmatic event integration
    Blockchain: On-chain state (token balances, voting records)
    Direct API: Programmatic cell submission

Perception Functions:
    Integration: Unifies multimodal input into Cell objects
    Context Building: Enriches input with historical context
    Filtering: Removes spam and irrelevant noise
    Prioritization: Routes urgent events to fast decision paths

Typical usage:
    from cynic.kernel.organism.layers import PerceptionUnifier
    perception = PerceptionUnifier()
    unified_cell = perception.integrate(discord_message)

See Also:
    cynic.kernel.organism.layers: 10-layer organism architecture
    cynic.kernel.organism.perception.senses: Sensory worker implementations
    cynic.kernel.core.unified_state: Cell data model
"""
