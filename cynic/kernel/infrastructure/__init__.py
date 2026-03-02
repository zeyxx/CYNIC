"""
CYNIC Infrastructure Layer — Database Connection Pooling and Low-Level I/O Abstractions.

Provides optimized infrastructure primitives for CYNIC's performance tier:
- DatabasePool: SQLAlchemy QueuePool wrapper for connection reuse
- Fibonacci-aligned pool sizing (F(6)=8 min, F(5)=5 overflow)
- Graceful degradation when database is unavailable

Typical usage:
    from cynic.kernel.infrastructure.db_pool import DatabasePool
    pool = DatabasePool(pool_size=8, max_overflow=5)
    available = pool.available_connections()
    pool.dispose()

See Also:
    cynic.kernel.core.storage.postgres: asyncpg direct connection layer
    cynic.kernel.core.phi: Fibonacci constants used for pool sizing
"""
