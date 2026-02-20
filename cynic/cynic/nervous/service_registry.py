"""
CYNIC Tier 1 Nervous System — Real-Time Service State Registry

Component 1 (Foundation): Tracks runtime health and state of all kernel components.

Every Dog, Bus, Worker, and Storage system registers itself. The registry provides:
  - snapshot() — current state of all components
  - record_judgment(pipeline_id, judgment) — capture decision state
  - get_component(name) — query single component
  - changed_since(snapshot_t) — detect state changes

Pattern: Non-blocking (async), φ-bounded queries, rolled indices for efficiency.
Queryable via: GET /internal/registry

This component enables all other Tier 1 components (EventJournal, DecisionTrace, LoopClosureValidator).
"""
from __future__ import annotations

import asyncio
import logging
import time
from dataclasses import asdict, dataclass, field
from typing import Any, Optional
from enum import StrEnum

logger = logging.getLogger("cynic.nervous.service_registry")


# ════════════════════════════════════════════════════════════════════════════
# DATA STRUCTURES
# ════════════════════════════════════════════════════════════════════════════

class ComponentType(StrEnum):
    """Component types that register with the registry."""
    DOG = "dog"                    # Sefirot dogs (judge perspectives)
    BUS = "bus"                    # Event bus (core, agent, automation)
    WORKER = "worker"              # Background workers (perceive, learn, evolve)
    STORAGE = "storage"            # Database (SurrealDB, Postgres, SQLite)
    LEARNER = "learner"            # Learning modules (QTable, Thompson, EWC)
    DETECTOR = "detector"          # Detectors (residual, emergence, axiom)
    ORCHESTRATOR = "orchestrator"  # Decision orchestrators
    ROUTER = "router"              # Action routers and runners


class HealthStatus(StrEnum):
    """Component health status."""
    HEALTHY = "healthy"
    DEGRADED = "degraded"
    STALLED = "stalled"           # Not updated recently
    FAILED = "failed"
    UNKNOWN = "unknown"


@dataclass
class ComponentSnapshot:
    """State snapshot of a single component."""
    name: str
    type: ComponentType
    status: HealthStatus
    last_update_ms: float         # Time.time() * 1000
    metrics: dict[str, Any] = field(default_factory=dict)  # Type-specific metrics

    # Judgment-specific (if applicable)
    last_judgment_id: Optional[str] = None
    last_judgment_verdict: Optional[str] = None  # BARK/GROWL/WAG/HOWL
    last_judgment_q_score: float = 0.0

    def to_dict(self) -> dict[str, Any]:
        d = asdict(self)
        d["type"] = str(self.type)
        d["status"] = str(self.status)
        return d

    @staticmethod
    def from_dict(d: dict[str, Any]) -> ComponentSnapshot:
        d_copy = dict(d)
        d_copy["type"] = ComponentType(d_copy["type"])
        d_copy["status"] = HealthStatus(d_copy["status"])
        return ComponentSnapshot(**{k: v for k, v in d_copy.items() if k in ComponentSnapshot.__dataclass_fields__})


@dataclass
class RegistrySnapshot:
    """Full snapshot of all registered components."""
    timestamp_ms: float
    components: dict[str, ComponentSnapshot] = field(default_factory=dict)
    total_components: int = 0
    healthy_count: int = 0
    degraded_count: int = 0
    stalled_count: int = 0
    failed_count: int = 0

    def to_dict(self) -> dict[str, Any]:
        return {
            "timestamp_ms": self.timestamp_ms,
            "components": {k: v.to_dict() for k, v in self.components.items()},
            "total_components": self.total_components,
            "healthy_count": self.healthy_count,
            "degraded_count": self.degraded_count,
            "stalled_count": self.stalled_count,
            "failed_count": self.failed_count,
        }

    @staticmethod
    def from_dict(d: dict[str, Any]) -> RegistrySnapshot:
        d_copy = dict(d)
        components = {}
        for k, v in d_copy.get("components", {}).items():
            components[k] = ComponentSnapshot.from_dict(v)
        d_copy["components"] = components
        return RegistrySnapshot(**{k: v for k, v in d_copy.items() if k in RegistrySnapshot.__dataclass_fields__})


# ════════════════════════════════════════════════════════════════════════════
# SERVICE STATE REGISTRY
# ════════════════════════════════════════════════════════════════════════════

class ServiceStateRegistry:
    """
    Real-time registry of kernel component health and state.

    Non-blocking, always available for queries.
    Records component lifecycle and judgment outcomes.
    """

    def __init__(self, stall_threshold_sec: int = 30) -> None:
        """
        Initialize registry.

        Args:
            stall_threshold_sec: Mark component as STALLED if no update for N seconds.
        """
        self._components: dict[str, ComponentSnapshot] = {}
        self._lock = asyncio.Lock()
        self._stall_threshold_sec = stall_threshold_sec
        self._judgment_log: list[dict[str, Any]] = []  # Rolling cap = F(11) = 89

        logger.info(
            "ServiceStateRegistry initialized (stall_threshold=%ds)",
            stall_threshold_sec,
        )

    async def register(
        self,
        name: str,
        component_type: ComponentType,
        metrics: Optional[dict[str, Any]] = None,
    ) -> None:
        """Register a new component."""
        async with self._lock:
            now_ms = time.time() * 1000
            self._components[name] = ComponentSnapshot(
                name=name,
                type=component_type,
                status=HealthStatus.HEALTHY,
                last_update_ms=now_ms,
                metrics=metrics or {},
            )
            logger.debug("ServiceStateRegistry: registered %s (%s)", name, component_type)

    async def record_judgment(
        self,
        component_name: str,
        judgment_id: str,
        verdict: str,
        q_score: float,
        metadata: Optional[dict[str, Any]] = None,
    ) -> None:
        """
        Record a judgment outcome for a component.

        Args:
            component_name: Which component made the judgment
            judgment_id: Unique identifier for this judgment
            verdict: BARK / GROWL / WAG / HOWL
            q_score: Q-Score (0-100)
            metadata: Additional context (system state, dogs, etc.)
        """
        async with self._lock:
            # Only log judgments from registered components
            if component_name not in self._components:
                return  # Silently ignore unregistered components

            now_ms = time.time() * 1000

            # Update component snapshot
            comp = self._components[component_name]
            comp.last_update_ms = now_ms
            comp.status = HealthStatus.HEALTHY
            comp.last_judgment_id = judgment_id
            comp.last_judgment_verdict = verdict
            comp.last_judgment_q_score = q_score

            # Append to judgment log (rolling cap = F(11) = 89)
            self._judgment_log.append({
                "timestamp_ms": now_ms,
                "component": component_name,
                "judgment_id": judgment_id,
                "verdict": verdict,
                "q_score": q_score,
                "metadata": metadata or {},
            })

            # Apply rolling cap
            if len(self._judgment_log) > 89:  # F(11)
                self._judgment_log = self._judgment_log[-89:]

    async def record_metric(
        self,
        component_name: str,
        key: str,
        value: Any,
        status: Optional[HealthStatus] = None,
    ) -> None:
        """
        Update a single metric for a component.

        Args:
            component_name: Which component
            key: Metric name
            value: Metric value
            status: Optional status update (HEALTHY, DEGRADED, etc.)
        """
        async with self._lock:
            now_ms = time.time() * 1000

            if component_name not in self._components:
                return  # Not registered yet

            comp = self._components[component_name]
            comp.metrics[key] = value
            comp.last_update_ms = now_ms

            if status:
                comp.status = status

    async def get_component(self, name: str) -> Optional[ComponentSnapshot]:
        """Get snapshot of a single component."""
        async with self._lock:
            return self._components.get(name)

    async def snapshot(self) -> RegistrySnapshot:
        """
        Get full snapshot of all components.

        Computes health counts and detects stalled components.
        """
        async with self._lock:
            now_ms = time.time() * 1000
            stall_threshold_ms = self._stall_threshold_sec * 1000

            # Detect stalled components
            for comp in self._components.values():
                if comp.status != HealthStatus.FAILED:
                    age_ms = now_ms - comp.last_update_ms
                    if age_ms > stall_threshold_ms:
                        comp.status = HealthStatus.STALLED

            # Count statuses
            healthy = sum(1 for c in self._components.values() if c.status == HealthStatus.HEALTHY)
            degraded = sum(1 for c in self._components.values() if c.status == HealthStatus.DEGRADED)
            stalled = sum(1 for c in self._components.values() if c.status == HealthStatus.STALLED)
            failed = sum(1 for c in self._components.values() if c.status == HealthStatus.FAILED)

            return RegistrySnapshot(
                timestamp_ms=now_ms,
                components=dict(self._components),
                total_components=len(self._components),
                healthy_count=healthy,
                degraded_count=degraded,
                stalled_count=stalled,
                failed_count=failed,
            )

    async def changed_since(self, snapshot_t_ms: float) -> dict[str, ComponentSnapshot]:
        """
        Get all components that changed since timestamp (in milliseconds).

        Returns: {component_name: ComponentSnapshot} of changed components.
        """
        async with self._lock:
            return {
                name: comp
                for name, comp in self._components.items()
                if comp.last_update_ms > snapshot_t_ms
            }

    async def get_judgment_log(
        self,
        since_ms: Optional[float] = None,
        component: Optional[str] = None,
        verdict: Optional[str] = None,
    ) -> list[dict[str, Any]]:
        """
        Query judgment log with optional filters.

        Args:
            since_ms: Only return judgments after this timestamp
            component: Filter by component name
            verdict: Filter by verdict (BARK/GROWL/WAG/HOWL)

        Returns: List of judgment records matching filters.
        """
        async with self._lock:
            result = self._judgment_log

            if since_ms is not None:
                result = [j for j in result if j["timestamp_ms"] > since_ms]

            if component is not None:
                result = [j for j in result if j["component"] == component]

            if verdict is not None:
                result = [j for j in result if j["verdict"] == verdict]

            return result

    async def mark_failed(self, component_name: str, reason: str = "") -> None:
        """Mark a component as failed."""
        async with self._lock:
            if component_name in self._components:
                comp = self._components[component_name]
                comp.status = HealthStatus.FAILED
                if reason:
                    comp.metrics["failure_reason"] = reason
                logger.error(
                    "ServiceStateRegistry: %s marked FAILED (%s)",
                    component_name,
                    reason,
                )

    async def reset_for_testing(self) -> None:
        """Clear all state (for tests)."""
        async with self._lock:
            self._components.clear()
            self._judgment_log.clear()
            logger.debug("ServiceStateRegistry: reset for testing")


# ════════════════════════════════════════════════════════════════════════════
# SINGLETON ACCESSOR
# ════════════════════════════════════════════════════════════════════════════

_registry_instance: Optional[ServiceStateRegistry] = None


def get_service_registry() -> ServiceStateRegistry:
    """Get or create global Service State Registry singleton."""
    global _registry_instance
    if _registry_instance is None:
        _registry_instance = ServiceStateRegistry()
    return _registry_instance


def reset_service_registry() -> None:
    """Reset registry singleton (for testing)."""
    global _registry_instance
    _registry_instance = None
