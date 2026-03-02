"""
CYNIC Event Bus — The asynchronous nervous system.
"""

from __future__ import annotations

import asyncio
import logging
import threading
import time
from collections import defaultdict
from collections.abc import Callable, Coroutine
from contextvars import ContextVar
from enum import Enum
from typing import Any, Optional, TypeVar, Union

logger = logging.getLogger("cynic.kernel.core.event_bus")

current_instance_id: ContextVar[str] = ContextVar("current_instance_id")

T = TypeVar("T")

class CynicError(Exception):
    pass

class EventBusError(CynicError):
    pass

class CoreEvent(str, Enum):
    AWAKENED = "core.awakened"
    DORMANT = "core.dormant"
    PERCEPTION_RECEIVED = "core.perception_received"
    CYCLE_STARTED = "core.cycle_started"
    JUDGMENT_REQUESTED = "core.judgment_requested"
    JUDGMENT_CREATED = "core.judgment_created"
    JUDGMENT_FAILED = "core.judgment_failed"
    CONSENSUS_REACHED = "core.consensus_reached"
    CONSENSUS_FAILED = "core.consensus_failed"
    LEARNING_EVENT = "core.learning_event"
    ANOMALY_DETECTED = "core.anomaly_detected"
    SONA_TICK = "core.sona_tick"
    LOD_CHANGED = "core.lod_changed"
    VALUE_CREATED = "core.value_created"
    DECISION_MADE = "core.decision_made"
    RESIDUAL_HIGH = "core.residual_high"
    EMERGENCE_DETECTED = "core.emergence_detected"
    SELF_IMPROVEMENT_PROPOSED = "core.self_improvement_proposed"
    AXIOM_ACTIVATED = "core.axiom_activated"
    ACT_COMPLETED = "core.act_completed"
    ACT_REQUESTED = "core.act_requested"
    DISK_PRESSURE = "health.disk_pressure"
    DISK_CLEARED = "health.disk_cleared"
    MEMORY_PRESSURE = "health.memory_pressure"
    MEMORY_CLEARED = "health.memory_cleared"
    META_CYCLE = "core.meta_cycle"
    MCP_TOOL_CALLED = "mcp.tool_called"
    Q_TABLE_UPDATED = "core.q_table_updated"
    TRANSCENDENCE = "core.transcendence"
    MCP_RESULT_RECEIVED = "mcp.result_received"
    SDK_RESULT_RECEIVED = "sdk.result_received"
    ACTION_PROPOSED = "core.action_proposed"
    CONSCIOUSNESS_CHANGED = "core.consciousness_changed"
    USER_FEEDBACK = "core.user_feedback"
    EWC_CHECKPOINT = "core.ewc_checkpoint"
    USER_CORRECTION = "core.user_correction"
    GOSSIP_SYNCED = "core.gossip_synced"
    CONFIGURATION_MUTATED = "core.configuration_mutated"
    SDK_TOOL_JUDGED = "core.sdk_tool_judged"
    SDK_SESSION_STARTED = "core.sdk_session_started"
    SDK_SESSION_ENDED = "core.sdk_session_ended"
    REPUTATION_SYNC = "core.reputation_sync"
    # --- Missing values (Priority 5: added to fix latent AttributeError) ---
    SONA_AGGREGATED = "core.sona_aggregated"
    INTERNAL_ERROR = "core.internal_error"
    BUDGET_WARNING = "core.budget_warning"
    BUDGET_EXHAUSTED = "core.budget_exhausted"
    # --- Priority 10: Proposal execution events ---
    PROPOSAL_EXECUTED = "core.proposal_executed"
    PROPOSAL_FAILED = "core.proposal_failed"
    # --- Topology & Change Analysis events ---
    SOURCE_CHANGED = "core.source_changed"
    CHANGE_ANALYZED = "core.change_analyzed"

class Event:
    def __init__(self, type: str, payload: Any = None, source: str = "unknown", instance_id: str | None = None):
        self.type = type
        self.payload = payload
        self.source = source
        self.instance_id = instance_id or current_instance_id.get(None) or "unknown"
        self.event_id = str(time.time_ns())
        self.timestamp = time.time()

    @property
    def metadata(self) -> dict:
        """SRE-standard metadata for the event."""
        return {
            "event_id": self.event_id,
            "instance_id": self.instance_id,
            "source": self.source,
            "timestamp": self.timestamp
        }

    @property
    def dict_payload(self) -> dict:
        if self.payload is None:
            return {}
        if isinstance(self.payload, dict):
            return self.payload
        if hasattr(self.payload, "model_dump"):
            return self.payload.model_dump()
        return vars(self.payload) if hasattr(self.payload, "__dict__") else {"data": self.payload}

    @classmethod
    def typed(cls, type: CoreEvent, payload: Any = None, source: str = "unknown") -> Event:
        return cls(type.value if hasattr(type, "value") else str(type), payload, source)

    def as_typed(self, payload_type: type[T]) -> T:
        """Validate and cast payload to the specified type.

        Args:
            payload_type: Pydantic model or dataclass to validate against

        Returns:
            Validated payload instance

        Raises:
            EventBusError: If payload is None or fails validation
        """
        if self.payload is None:
            raise EventBusError(f"Event payload is None, cannot cast to {payload_type.__name__}")

        # If payload is already the correct type, return it
        if isinstance(self.payload, payload_type):
            return self.payload

        # Try Pydantic validation if available
        try:
            if hasattr(payload_type, "model_validate"):
                return payload_type.model_validate(self.payload)
        except Exception as e:
            logger.debug("Pydantic validation failed for %s: %s", payload_type.__name__, e, exc_info=True)

        # Try direct instantiation if dict
        try:
            if isinstance(self.payload, dict):
                return payload_type(**self.payload)
        except Exception as e:
            raise EventBusError(f"Failed to validate payload as {payload_type.__name__}: {e}")

        raise EventBusError(f"Payload type {type(self.payload).__name__} cannot be cast to {payload_type.__name__}")

Handler = Callable[[Event], Coroutine[Any, Any, None]]

_buses: dict[str, EventBus] = {}
_buses_lock: threading.Lock = threading.Lock()  # Protect global buses dict

class EventBus:
    def __init__(self, bus_id: str, instance_id: str | None = None):
        self.bus_id = bus_id
        self.instance_id = instance_id or "unknown"
        self._handlers: dict[str, list[Handler]] = defaultdict(list)
        self._handlers_lock: threading.Lock = threading.Lock()  # Protect handler mutation (sync safe)
        self._pending_tasks: set[asyncio.Task] = set()
        self._handler_timeout_s: float = 30.0
        self._bridge: Optional[Any] = None  # Distributed bridge hook

        # High-Frequency Metrics (SRE Standard)
        self._emitted_count: int = 0
        self._error_count: int = 0
        self._handler_errors: dict[str, list[str]] = defaultdict(list)
        self._total_latency_ms: float = 0.0
        self._peak_pending: int = 0

        # Backpressure Settings (Lentille : Backend)
        self.MAX_PENDING = 1000 # Critical threshold for 10k TPS readiness
        self._backpressure_emitting: bool = False  # Guard against recursive emit

    def set_bridge(self, bridge: Any):
        """Attach a distributed bridge (e.g. Redis)."""
        self._bridge = bridge

    def on(self, event_type: Union[str, CoreEvent], handler: Handler) -> None:
        name = event_type.value if hasattr(event_type, "value") else str(event_type)
        with self._handlers_lock:
            self._handlers[name].append(handler)

    def off(self, event_type: Union[str, CoreEvent], handler: Handler) -> None:
        name = event_type.value if hasattr(event_type, "value") else str(event_type)
        with self._handlers_lock:
            if name in self._handlers:
                self._handlers[name] = [h for h in self._handlers[name] if h != handler]

    async def _safe_handler_wrapper(self, handler: Handler, event: Event, handler_name: str) -> None:
        """Wrap handler execution with timeout and error handling."""
        timeout = self._handler_timeout_s
        try:
            await asyncio.wait_for(handler(event), timeout=timeout)
        except asyncio.TimeoutError:
            error_msg = f"Handler {handler_name} timed out after {timeout}s for event {event.type}"
            logger.warning(error_msg, extra={"event_id": event.event_id, "handler": handler_name})
            self._error_count += 1
            self._handler_errors[event.type].append(error_msg)
        except asyncio.CancelledError:
            logger.debug(f"Handler {handler_name} was cancelled for event {event.type}")
        except Exception as exc:
            error_msg = f"Handler {handler_name} failed: {type(exc).__name__}: {str(exc)}"
            logger.error(error_msg, exc_info=True, extra={"event_id": event.event_id, "handler": handler_name})
            self._error_count += 1
            self._handler_errors[event.type].append(error_msg)

    async def emit(self, event: Event, distributed: bool = True) -> None:
        """Emit event locally and optionally distribute it via bridge."""
        self._emitted_count += 1
        
        # 1. Local Processing (Reflex path)
        pending_count = len(self._pending_tasks)
        if pending_count > self._peak_pending:
            self._peak_pending = pending_count
            
        if pending_count > self.MAX_PENDING:
            logger.warning(
                f"[{self.instance_id}] BACKPRESSURE: {pending_count} pending tasks. Throttling active.",
                extra={"bus_id": self.bus_id}
            )
            # Mandatory anomaly signal (guard against recursive emit)
            if event.type != CoreEvent.ANOMALY_DETECTED and not self._backpressure_emitting:
                self._backpressure_emitting = True
                def reset_flag(task):
                    self._backpressure_emitting = False
                task = asyncio.create_task(self.emit(Event.typed(
                    CoreEvent.ANOMALY_DETECTED,
                    {"type": "backpressure", "pending": pending_count},
                    source="event_bus"
                )))
                task.add_done_callback(reset_flag)

        # Snapshot handlers under lock to prevent concurrent modification
        with self._handlers_lock:
            handlers = list(self._handlers.get(event.type, []))
            wildcards = list(self._handlers.get("*", []))

        t_start = time.perf_counter()
        for i, h in enumerate(handlers + wildcards):
            handler_name = getattr(h, "__name__", f"handler_{i}")
            task = asyncio.create_task(self._safe_handler_wrapper(h, event, handler_name))
            self._pending_tasks.add(task)
            task.add_done_callback(self._pending_tasks.discard)
        
        self._total_latency_ms += (time.perf_counter() - t_start) * 1000

        # 2. Distributed Path (Consciousness synchronization)
        if distributed and self._bridge:
            # Fire and forget to the bridge
            asyncio.create_task(self._bridge.publish(event))

    async def drain(self, timeout: float = 10.0) -> None:
        """Wait for all pending handler tasks to complete."""
        if not self._pending_tasks:
            return
            
        try:
            # Snapshot pending tasks to avoid race with done_callbacks
            pending = list(self._pending_tasks)
            await asyncio.wait_for(
                asyncio.gather(*pending, return_exceptions=True),
                timeout=timeout
            )
        except asyncio.TimeoutError:
            logger.warning(f"[{self.instance_id}] EventBus drain timed out with {len(self._pending_tasks)} tasks remaining")
        except Exception as e:
            logger.error(f"[{self.instance_id}] EventBus drain failed: {e}")
        finally:
            # Let done_callbacks clean up individual tasks
            # Only clear if nothing new was added during drain
            if self._pending_tasks:
                self._pending_tasks.clear()

    def stats(self) -> dict[str, Any]:
        """Return high-frequency observability statistics."""
        return {
            "bus_id": self.bus_id,
            "instance_id": self.instance_id,
            "emitted": self._emitted_count,
            "errors": self._error_count,
            "pending_tasks": len(self._pending_tasks),
            "peak_pending": self._peak_pending,
            "avg_latency_ms": self._total_latency_ms / max(self._emitted_count, 1),
            "error_rate": self._error_count / max(self._emitted_count, 1),
            "load_factor": len(self._pending_tasks) / self.MAX_PENDING
        }

_buses: dict[str, EventBus] = {}

def get_bus(bus_id: str, instance_id: str | None = None) -> EventBus:
    """
    Retrieve an isolated event bus.
    Requires an explicit instance_id or an active task context via ContextVar.
    """
    target_id = instance_id or current_instance_id.get(None)
    if target_id is None:
        raise RuntimeError(
            f"EventBus '{bus_id}' requested without instance context. "
            "Pass instance_id or set current_instance_id ContextVar."
        )
    key = f"{target_id}:{bus_id}"
    with _buses_lock:
        if key not in _buses:
            _buses[key] = EventBus(bus_id=key, instance_id=target_id)
        return _buses[key]

def get_core_bus(instance_id: str | None = None) -> EventBus:
    """Get the core nervous system bus."""
    return get_bus("CORE", instance_id)

def get_automation_bus(instance_id: str | None = None) -> EventBus:
    return get_bus("AUTOMATION", instance_id)

async def reset_all_buses() -> None:
    """
    Safely reset all event buses.
    Drains all pending tasks before clearing the global registry.
    Use during shutdown or testing.
    """
    with _buses_lock:
        buses_to_drain = list(_buses.values())

    # Drain all buses (outside lock to avoid deadlock)
    for bus in buses_to_drain:
        await bus.drain(timeout=5.0)

    # Clear after draining
    with _buses_lock:
        _buses.clear()

def get_agent_bus(instance_id: str | None = None) -> EventBus:
    return get_bus("AGENT", instance_id)
