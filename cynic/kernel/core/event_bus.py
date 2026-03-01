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

current_instance_id: ContextVar[str] = ContextVar("current_instance_id", default="DEFAULT")

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

class Event:
    def __init__(self, type: str, payload: Any = None, source: str = "unknown"):
        self.type = type
        self.payload = payload
        self.source = source
        self.event_id = str(time.time_ns())
        self.timestamp = time.time()

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
    def __init__(self, bus_id: str):
        self.bus_id = bus_id
        self._handlers: dict[str, list[Handler]] = defaultdict(list)
        self._pending_tasks: set[asyncio.Task] = set()

    def on(self, event_type: Union[str, CoreEvent], handler: Handler) -> None:
        name = event_type.value if hasattr(event_type, "value") else str(event_type)
        self._handlers[name].append(handler)

    def off(self, event_type: Union[str, CoreEvent], handler: Handler) -> None:
        name = event_type.value if hasattr(event_type, "value") else str(event_type)
        if name in self._handlers:
            self._handlers[name] = [h for h in self._handlers[name] if h != handler]

    async def emit(self, event: Event) -> None:
        handlers = self._handlers.get(event.type, [])
        wildcards = self._handlers.get("*", [])
        for h in handlers + wildcards:
            task = asyncio.create_task(h(event))
            self._pending_tasks.add(task)
            task.add_done_callback(self._pending_tasks.discard)

    async def drain(self, timeout: float = 2.0) -> None:
        if not self._pending_tasks: return
        try:
            await asyncio.wait_for(asyncio.gather(*self._pending_tasks, return_exceptions=True), timeout=timeout)
        except Exception: pass
        finally: self._pending_tasks.clear()

def get_bus(bus_id: str, instance_id: str | None = None) -> EventBus:
    target_id = instance_id or current_instance_id.get()
    key = f"{target_id}:{bus_id}"
    if key not in _buses:
        _buses[key] = EventBus(bus_id=key)
    return _buses[key]

def get_core_bus(instance_id: str | None = None) -> EventBus:
    return get_bus("CORE", instance_id)

def get_automation_bus(instance_id: str | None = None) -> EventBus:
    return get_bus("AUTOMATION", instance_id)

def get_agent_bus(instance_id: str | None = None) -> EventBus:
    return get_bus("AGENT", instance_id)
