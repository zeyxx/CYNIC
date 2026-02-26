"""
CYNIC Core — Foundational constants, axioms, judgment models, and infrastructure.

Core modules establish CYNIC's mathematical and architectural foundation:

Constants & Axioms:
    cynic.core.phi: Golden ratio φ and mathematical foundations
    cynic.core.axioms: 11 immutable axioms (BURN, LOVE, KNOW, etc.)

Judgment & Consciousness:
    cynic.core.consciousness: Consciousness levels (REFLEX, MICRO, MACRO, META)
    cynic.core.unified_state: Immutable state models (Cell, Judgment, Event)
    cynic.core.judge_interface: Judge contract and protocols

Event & Message Systems:
    cynic.core.event_bus: Async publish-subscribe event coordination
    cynic.core.messages: Unified message protocols

Storage & Persistence:
    cynic.core.storage: Abstract storage layer with memory and database backends
    cynic.core.database: Persistence implementations

Typical usage:
    from cynic.core.phi import PHI
    from cynic.core.consciousness import ConsciousnessLevel
    from cynic.core.unified_state import Cell, Judgment

See Also:
    cynic.api.builders: Builder patterns using core models
    cynic.organism: Organism architecture built on core
"""
