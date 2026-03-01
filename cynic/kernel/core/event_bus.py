"""
CYNIC Event Bus — The asynchronous nervous system.
"""

from __future__ import annotations

import asyncio
import logging
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
        if self.payload is None: return {}
        if isinstance(self.payload, dict): return self.payload
        if hasattr(self.payload, "model_dump"): return self.payload.model_dump()
        return vars(self.payload) if hasattr(self.payload, "__dict__") else {"data": self.payload}

    @classmethod
    def typed(cls, type: CoreEvent, payload: Any = None, source: str = "unknown") -> Event:
        return cls(type.value if hasattr(type, "value") else str(type), payload, source)

Handler = Callable[[Event], Coroutine[Any, Any, None]]

_buses: dict[str, EventBus] = {}

class EventBus:
    def __init__(self, bus_id: str, instance_id: str | None = None):
        self.bus_id = bus_id
        self.instance_id = instance_id or "unknown"
        self._handlers: dict[str, list[Handler]] = defaultdict(list)
        self._pending_tasks: set[asyncio.Task] = set()
        self._handler_timeout_s: float = 30.0
        self._emitted_count: int = 0
        self._error_count: int = 0
        self._handler_errors: dict[str, list[str]] = defaultdict(list)

    def on(self, event_type: Union[str, CoreEvent], handler: Handler) -> None:
        name = event_type.value if hasattr(event_type, "value") else str(event_type)
        self._handlers[name].append(handler)

    def off(self, event_type: Union[str, CoreEvent], handler: Handler) -> None:
        name = event_type.value if hasattr(event_type, "value") else str(event_type)
        if name in self._handlers:
            self._handlers[name] = [h for h in self._handlers[name] if h != handler]

    async def _safe_handler_wrapper(self, handler: Handler, event: Event, handler_name: str) -> None:
        """Wrap handler execution with timeout and error handling.

        Observable error handling: catches exceptions, logs with context, continues.
        Prevents one failing handler from blocking others.
        """
        try:
            await asyncio.wait_for(handler(event), timeout=self._handler_timeout_s)
        except asyncio.TimeoutError:
            error_msg = f"Handler {handler_name} timed out after {self._handler_timeout_s}s for event {event.type}"
            logger.warning(error_msg, extra={"event_id": event.event_id, "handler": handler_name})
            self._error_count += 1
            self._handler_errors[event.type].append(error_msg)
        except asyncio.CancelledError:
            # Task was cancelled, this is expected during shutdown
            logger.debug(f"Handler {handler_name} was cancelled for event {event.type}")
        except Exception as exc:
            error_msg = f"Handler {handler_name} failed: {type(exc).__name__}: {str(exc)}"
            logger.error(error_msg, exc_info=True, extra={"event_id": event.event_id, "handler": handler_name})
            self._error_count += 1
            self._handler_errors[event.type].append(error_msg)

    async def emit(self, event: Event) -> None:
        """Emit event to all registered handlers.

        Handlers are invoked concurrently. If a handler fails, it's logged but doesn't
        prevent other handlers from running. This provides resilience + observability.
        """
        self._emitted_count += 1
        handlers = self._handlers.get(event.type, [])
        wildcards = self._handlers.get("*", [])

        for i, h in enumerate(handlers + wildcards):
            handler_name = getattr(h, "__name__", f"handler_{i}")
            task = asyncio.create_task(self._safe_handler_wrapper(h, event, handler_name))
            self._pending_tasks.add(task)
            task.add_done_callback(self._pending_tasks.discard)

    async def drain(self, timeout: float = 2.0) -> None:
        """Wait for all pending handler tasks to complete.

        Observable error handling: logs timeout but doesn't raise.
        Ensures graceful shutdown even if handlers are slow.
        """
        if not self._pending_tasks:
            return
        try:
            await asyncio.wait_for(
                asyncio.gather(*self._pending_tasks, return_exceptions=True),
                timeout=timeout
            )
        except asyncio.TimeoutError:
            pending_count = len(self._pending_tasks)
            logger.warning(
                f"EventBus drain timed out with {pending_count} pending handler tasks still running",
                extra={"bus_id": self.bus_id, "timeout_s": timeout}
            )
        except Exception as exc:
            logger.error(f"Unexpected error during drain: {exc}", exc_info=True, extra={"bus_id": self.bus_id})
        finally:
            self._pending_tasks.clear()

    def stats(self) -> dict[str, Any]:
        """Return observability statistics about the bus."""
        return {
            "bus_id": self.bus_id,
            "emitted": self._emitted_count,
            "errors": self._error_count,
            "pending_tasks": len(self._pending_tasks),
            "error_rate": self._error_count / max(self._emitted_count, 1),
            "error_by_event": dict(self._handler_errors),
        }

def get_bus(bus_id: str, instance_id: str | None = None) -> EventBus:
    """
    Retrieve an isolated event bus.
    Requires an explicit instance_id or an active task context via ContextVar.
    """
    target_id = instance_id or current_instance_id.get()
    if target_id is None:
        raise RuntimeError(
            f"EventBus '{bus_id}' requested without instance context. "
            "Pass instance_id or set current_instance_id ContextVar."
        )
    key = f"{target_id}:{bus_id}"
    if key not in _buses:
        _buses[key] = EventBus(bus_id=key, instance_id=target_id)
    return _buses[key]

def get_core_bus(instance_id: str | None = None) -> EventBus:
    """Get the core nervous system bus."""
    return get_bus("CORE", instance_id)

def get_automation_bus(instance_id: str | None = None) -> EventBus:
    return get_bus("AUTOMATION", instance_id)

def get_agent_bus(instance_id: str | None = None) -> EventBus:
    return get_bus("AGENT", instance_id)
