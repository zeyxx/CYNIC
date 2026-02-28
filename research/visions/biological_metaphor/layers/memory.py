"""
CYNIC Memory Layer (Layer 4) — Historical state persistence and recall.

Part of the 10-layer organism architecture, the memory layer provides
persistent storage of judgments, decisions, and organism state for learning and accountability.

Memory Types:
    Judgment History: All past judgments (immutable audit trail)
    Decision Log: Actions taken and outcomes
    Learning State: Q-Table and session snapshots
    Configuration: Organism axioms and settings
    Event Log: Raw events processed by organism

Storage Backends:
    Memory: In-process cache (development/testing)
    SQLite: Local persistent storage
    PostgreSQL: Enterprise multi-instance storage
    S3/Cloud: Distributed backup and archival

Typical usage:
    from cynic.kernel.organism.layers import MemorySubstrate
    memory = MemorySubstrate(storage)
    past_judgments = await memory.recall(category='governance')

See Also:
    cynic.kernel.organism.layers: 10-layer organism architecture
    cynic.kernel.core.storage: Abstract storage layer
    cynic.brain.learning: Uses memory for experience replay
"""
