"""
Health Check System for CYNIC

Provides comprehensive system health monitoring including:
- Database connectivity (SurrealDB or PostgreSQL)
- LLM registry availability (Ollama, Claude API, etc.)
- Organism consciousness state
- Event bus health
- Overall system status with graceful degradation
"""

from __future__ import annotations

import asyncio
import logging
from datetime import UTC, datetime
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from cynic.kernel.organism.brain.llm.adapter import LLMRegistry
    from cynic.kernel.organism.organism import Organism

logger = logging.getLogger("cynic.kernel.observability.health")


class HealthChecker:
    """Check all CYNIC subsystems and return comprehensive health status."""

    def __init__(
        self,
        organism: Organism | None = None,
        registry: LLMRegistry | None = None,
        db_pool: Any | None = None,
        surreal: Any | None = None,
    ):
        """
        Initialize health checker with optional system references.

        Args:
            organism: The Organism instance (brain, senses, metabolism, memory)
            registry: LLM registry for availability checks
            db_pool: asyncpg connection pool (legacy persistence)
            surreal: SurrealDB instance (primary persistence)
        """
        self.organism = organism
        self.registry = registry
        self.db_pool = db_pool
        self.surreal = surreal

    async def check(self) -> dict[str, Any]:
        """
        Check all systems in parallel and return comprehensive health status.

        Returns a dict with:
        - timestamp: ISO8601 timestamp
        - overall: "healthy" | "degraded" | "critical"
        - database: "ok" | "down"
        - llm: "ok" | "degraded" | "down"
        - consciousness: "ok" | "down"
        - event_bus: "ok" | "down"
        - app: "running" | "degraded"
        - uptime_s: seconds since kernel started
        """
        try:
            # Check all systems in parallel (no blocking on one failure)
            results = await asyncio.gather(
                self._check_database(),
                self._check_llm(),
                self._check_consciousness(),
                self._check_event_bus(),
                return_exceptions=True,
            )

            db_ok, llm_ok, consciousness_ok, event_bus_ok = results

            # Determine overall health based on critical systems
            overall = self._compute_overall_status(db_ok, llm_ok, consciousness_ok, event_bus_ok)

            # Build response
            uptime_s = self.organism.uptime_s if self.organism else 0.0

            return {
                "timestamp": datetime.now(UTC).isoformat(),
                "overall": overall,
                "database": self._status_str(db_ok),
                "llm": self._status_str(llm_ok),
                "consciousness": self._status_str(consciousness_ok),
                "event_bus": self._status_str(event_bus_ok),
                "app": "running" if overall != "critical" else "degraded",
                "uptime_s": round(uptime_s, 1),
            }

        except Exception as e:
            # Fallback response if health checker itself breaks
            logger.error("Health checker crashed: %s", e, exc_info=True)
            return {
                "timestamp": datetime.now(UTC).isoformat(),
                "overall": "critical",
                "error": str(e),
                "app": "degraded",
            }

    async def check_detailed(self) -> dict[str, Any]:
        """
        Detailed health check with remediation hints.

        Returns health status plus helpful hints for fixing failures.
        """
        status = await self.check()

        # Add remediation hints for failed systems
        if status.get("database") != "ok":
            if self.surreal is not None:
                status["database_hint"] = (
                    "SurrealDB connection failed. Check if SurrealDB is running and "
                    "SURREAL_URL env var is correct."
                )
            else:
                status["database_hint"] = (
                    "PostgreSQL connection failed. Check if PostgreSQL is running and "
                    "DATABASE_URL env var is correct."
                )

        if status.get("llm") != "ok":
            status["llm_hint"] = (
                "LLM registry unavailable. Check if Ollama is running on "
                "OLLAMA_URL (default: http://localhost:11434). "
                "CYNIC can operate without LLMs (heuristic mode only)."
            )

        if status.get("consciousness") != "ok":
            status["consciousness_hint"] = (
                "Organism not responsive. Check kernel logs for startup errors."
            )

        if status.get("event_bus") != "ok":
            status["event_bus_hint"] = (
                "Core event bus not responding. Check for deadlocks or event handler crashes."
            )

        return status

    # ── Private helper methods ──

    async def _check_database(self) -> bool:
        """Test database connectivity (SurrealDB or PostgreSQL)."""
        # SurrealDB is preferred
        if self.surreal is not None:
            try:
                # Try a simple ping query
                await self.surreal.qtable.get_all()
                return True
            except Exception as e:
                logger.warning("SurrealDB health check failed: %s", e)
                raise

        # Fall back to asyncpg
        if self.db_pool is not None:
            try:
                async with self.db_pool.acquire() as conn:
                    await conn.fetchval("SELECT 1")
                return True
            except Exception as e:
                logger.warning("PostgreSQL health check failed: %s", e)
                raise

        # No database configured — assume ok
        return True

    async def _check_llm(self) -> bool:
        """Test LLM registry availability."""
        if self.registry is None:
            # No registry configured
            return True

        try:
            available = self.registry.get_available()
            if available:
                return True
            else:
                # Registry exists but no LLMs available
                logger.warning("No LLMs available in registry (Ollama/APIs down?)")
                raise RuntimeError("No available LLMs")
        except Exception as e:
            logger.warning("LLM registry health check failed: %s", e)
            raise

    async def _check_consciousness(self) -> bool:
        """Test organism consciousness (is kernel responsive?)."""
        if self.organism is None:
            # No organism provided
            return True

        try:
            # Check if organism can respond to basic queries
            # The uptime_s property is a lightweight proxy
            uptime = self.organism.uptime_s
            if uptime < 0:
                raise RuntimeError("Invalid uptime")

            # Check if kernel mirror can snapshot (indicates cognition active)
            if hasattr(self.organism, "kernel_mirror"):
                snap = self.organism.kernel_mirror.snapshot(self.organism)
                if snap is None:
                    raise RuntimeError("Kernel mirror returned None")

            return True
        except Exception as e:
            logger.warning("Consciousness health check failed: %s", e)
            raise

    async def _check_event_bus(self) -> bool:
        """Test core event bus responsiveness."""
        if self.organism is None:
            return True

        try:
            # Check if event buses exist and are accessible
            from cynic.kernel.core.event_bus import get_agent_bus, get_automation_bus, get_core_bus

            core_bus = get_core_bus()
            if core_bus is None:
                raise RuntimeError("Core bus not initialized")

            auto_bus = get_automation_bus()
            if auto_bus is None:
                raise RuntimeError("Automation bus not initialized")

            agent_bus = get_agent_bus()
            if agent_bus is None:
                raise RuntimeError("Agent bus not initialized")

            return True
        except Exception as e:
            logger.warning("Event bus health check failed: %s", e)
            raise

    @staticmethod
    def _compute_overall_status(
        db_ok: Any, llm_ok: Any, consciousness_ok: Any, event_bus_ok: Any
    ) -> str:
        """
        Compute overall health status based on subsystem checks.

        CRITICAL: Database or Consciousness or EventBus failed (core systems)
        DEGRADED: LLM failed (can operate without LLMs)
        HEALTHY: All systems ok
        """
        critical_systems = [consciousness_ok, event_bus_ok, db_ok]
        optional_systems = [llm_ok]

        # Check if any critical system failed
        for result in critical_systems:
            if isinstance(result, Exception):
                return "critical"

        # Check optional systems
        for result in optional_systems:
            if isinstance(result, Exception):
                return "degraded"

        return "healthy"

    @staticmethod
    def _status_str(result: Any) -> str:
        """Convert check result to status string."""
        if isinstance(result, Exception):
            return "down"
        elif result is True or result is None:
            return "ok"
        else:
            return "unknown"
