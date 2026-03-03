"""
Database Connection Pooling  Optimized SQLAlchemy QueuePool wrapper.

Provides reusable connection pool with graceful degradation when database
is unavailable. Fibonacci-aligned pool sizing ensures predictable performance
across load levels.

Typical usage:
    pool = DatabasePool(pool_size=8, max_overflow=5, db_url="postgresql://...")
    connection = pool.get_connection()
    available = pool.available_connections()
    pool.dispose()  # Clean shutdown

See Also:
    cynic.kernel.core.phi: Fibonacci constants for pool sizing
    SQLAlchemy QueuePool documentation
"""

from __future__ import annotations

import logging
from typing import Any

logger = logging.getLogger("cynic.kernel.infrastructure.db_pool")


class DatabasePool:
    """
    Connection pool wrapper for SQLAlchemy QueuePool.

    Fibonacci-aligned defaults:
    - pool_size = F(6) = 8 (minimum concurrent connections)
    - max_overflow = F(5) = 5 (additional temporary connections)

    Graceful degradation: If db_url is None or SQLAlchemy is unavailable,
    pool operates in no-op mode (safe returns, no actual connections).
    """

    def __init__(
        self,
        pool_size: int = 8,
        max_overflow: int = 5,
        db_url: str | None = None,
        echo: bool = False,
    ) -> None:
        """
        Initialize connection pool.

        Args:
            pool_size: Minimum pool size (F(6)=8 by default)
            max_overflow: Max temporary overflow connections (F(5)=5 by default)
            db_url: Database URL (e.g., postgresql://...). If None, pool is no-op.
            echo: If True, log all SQL statements (development use only)
        """
        self.pool_size = pool_size
        self.max_overflow = max_overflow
        self.db_url = db_url
        self.echo = echo
        self._engine: Any = None
        self._is_no_op = db_url is None

        if not self._is_no_op:
            self._initialize_engine()

    def _initialize_engine(self) -> None:
        """Initialize SQLAlchemy engine with QueuePool settings."""
        try:
            from sqlalchemy import create_engine
            from sqlalchemy.pool import QueuePool

            self._engine = create_engine(
                self.db_url,
                poolclass=QueuePool,
                pool_size=self.pool_size,
                max_overflow=self.max_overflow,
                pool_pre_ping=True,  # Validate connections before use
                echo=self.echo,
                connect_args={"timeout": 10},  # Connection timeout
            )
            logger.info(
                f"Database pool initialized: pool_size={self.pool_size}, "
                f"max_overflow={self.max_overflow}, db_url={self.db_url[:20]}..."
            )
        except ImportError:
            logger.warning(
                "SQLAlchemy not available; database pool operating in no-op mode"
            )
            self._is_no_op = True
            self._engine = None
        except Exception as exc:
            logger.error(
                f"Failed to initialize database pool: {type(exc).__name__}: {str(exc)}",
                exc_info=True,
            )
            self._is_no_op = True
            self._engine = None

    def available_connections(self) -> int:
        """
        Return number of available connections in pool.

        Returns:
            pool_size if no-op mode, otherwise actual available slots from QueuePool
        """
        if self._is_no_op or self._engine is None:
            return self.pool_size

        try:
            # SQLAlchemy QueuePool has .pool and .checkedout attributes
            pool = self._engine.pool
            available = pool.size() - len(pool.checkedout())
            return max(available, 0)
        except Exception as exc:
            logger.warning(
                f"Failed to query pool availability: {type(exc).__name__}: {str(exc)}"
            )
            return self.pool_size

    def get_connection(self) -> Any:
        """
        Acquire a connection from the pool.

        Returns:
            SQLAlchemy connection object if engine exists, None if no-op mode

        Raises:
            sqlalchemy.exc.OperationalError: If connection cannot be acquired
        """
        if self._is_no_op or self._engine is None:
            logger.debug("get_connection called in no-op mode; returning None")
            return None

        try:
            return self._engine.connect()
        except Exception as exc:
            logger.error(
                f"Failed to acquire connection: {type(exc).__name__}: {str(exc)}",
                exc_info=True,
            )
            raise

    def dispose(self) -> None:
        """Close all connections in the pool and clean up resources."""
        if self._engine is not None:
            try:
                self._engine.dispose()
                logger.info("Database pool disposed; all connections closed")
            except Exception as exc:
                logger.error(
                    f"Error disposing pool: {type(exc).__name__}: {str(exc)}",
                    exc_info=True,
                )
        else:
            logger.debug("dispose() called on no-op pool; no-op")
