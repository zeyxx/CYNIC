"""
CYNIC Storage Layer â€” Abstract persistence for organism state.

Provides pluggable storage backends for CYNIC's data:
- Judgments, decisions, learning records
- Event history and audit trails
- Organism configuration and state

Implementations:
    MemoryStorage: In-memory backend (development/testing)
    SQLiteStorage: Local SQLite database
    PostgresStorage: Enterprise PostgreSQL backend

Typical usage:
    from cynic.kernel.core.storage import StorageFactory
    storage = StorageFactory.create('sqlite', db_path='cynic.db')
    judgments = await storage.find_judgments(category='governance')

See Also:
    cynic.kernel.core.unified_state: State models (Cell, Judgment, Event)
    cynic.kernel.core.event_bus: Event coordination
"""
