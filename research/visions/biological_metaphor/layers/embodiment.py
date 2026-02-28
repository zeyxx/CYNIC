"""
CYNIC Embodiment Layer (Layer 7) — Boundary maintenance and interaction surface.

Part of the 10-layer organism architecture, the embodiment layer defines
CYNIC's external interface and boundaries with the environment.

Embodiment Interfaces:
    Discord: Community governance voting and proposal submission
    Telegram: Alternative community interface
    REST API: Programmatic integration
    Blockchain: Smart contract execution (NEAR/Ethereum)

Responsibilities:
    - Translates judgments into external actions
    - Maintains interaction protocol compliance
    - Enforces rate limits and resource constraints
    - Tracks external state changes and feedback

Typical usage:
    from cynic.organism.layers import Embodiment
    embodiment = Embodiment()
    await embodiment.act(judgment, action_type='vote')

See Also:
    cynic.organism.layers: 10-layer organism architecture
    cynic.organism.motor: Physical/blockchain action execution
    cynic.discord: Discord embodiment implementation
"""
