"""
CYNIC Event Bus — The asynchronous nervous system.

Core architecture for decoupled coordination between organs.
Uses asyncio tasks for non-blocking event distribution.
"""
from __future__ import annotations

import asyncio
import logging
import time
from collections import defaultdict
try:
    from enum import StrEnum
except ImportError:
    from enum import Enum
    class StrEnum(str, Enum): pass

from typing import Any, Callable, Coroutine, Optional, Type, TypeVar, Union

logger = logging.getLogger("cynic.kernel.core.event_bus")

T = TypeVar('T')  # Generic type for as_typed() method

class EventBusError(Exception):
    """Base class for all event bus errors."""
    pass

_EVENT_HISTORY_MAX = 55  # F(10) Fibonacci

class AutomationEvent(StrEnum):
    """Events on the AUTOMATION bus (triggers, ticks, scheduled tasks)."""
    TRIGGER_FIRED      = "trigger.fired"
    TRIGGER_BLOCKED    = "trigger.blocked"
    AUTOMATION_TICK    = "automation.tick"

    # Market
    PRICE_TICK         = "market.price_tick"
    MARKET_ALERT       = "market.alert"

    # Social
    SOCIAL_SIGNAL      = "social.signal"

    # Solana
    TX_CONFIRMED       = "solana.tx_confirmed"
    TX_FAILED          = "solana.tx_failed"

    # Scheduled tasks (Fibonacci intervals)
    PERCEIVE_CODE      = "schedule.perceive_code"
    PERCEIVE_SOLANA    = "schedule.perceive_solana"
    PERCEIVE_MARKET    = "schedule.perceive_market"
    PERCEIVE_SOCIAL    = "schedule.perceive_social"
    LEARN_BATCH        = "schedule.learn_batch"
    E_SCORE_UPDATE     = "schedule.e_score_update"


class AgentEvent(StrEnum):
    """Events on the AGENT bus (Dog signals, PBFT protocol)."""
    # Dog lifecycle
    DOG_ACTIVATED      = "dog.activated"
    DOG_DEACTIVATED    = "dog.deactivated"
    DOG_BENCHMARK_RUN  = "dog.benchmark_run"

    # PBFT consensus protocol (4 phases)
    PBFT_PRE_PREPARE   = "pbft.pre_prepare"
    PBFT_PREPARE       = "pbft.prepare"
    PBFT_COMMIT        = "pbft.commit"
    PBFT_REPLY         = "pbft.reply"
    PBFT_VIEW_CHANGE   = "pbft.view_change"

    # Dog voting
    DOG_VOTE           = "dog.vote"
    DOG_ABSTAIN        = "dog.abstain"
    DOG_VETO           = "dog.veto"      # GUARDIAN veto — blocks execution

    # Dog-to-Dog signals
    DOG_SIGNAL         = "dog.signal"
    COLLECTIVE_SIGNAL  = "collective.signal"

    # LLM routing
    LLM_SELECTED       = "llm.selected"
    LLM_BENCHMARK_DONE = "llm.benchmark_done"


class CoreEvent(StrEnum):
    """Events on the CORE bus (high-level organism lifecycle)."""
    # Lifecycle
    AWAKENED             = "core.awakened"
    DORMANT              = "core.dormant"
    HEARTBEAT            = "core.heartbeat"
    
    # Consciousness
    CONSCIOUSNESS_CHANGED = "core.consciousness_changed"
    LOD_CHANGED          = "core.lod_changed"
    SONA_TICK            = "core.sona_tick"           # Self-assessment pulse
    META_CYCLE           = "core.meta_cycle"
    MACRO_CYCLE          = "core.macro_cycle"
    MICRO_CYCLE          = "core.micro_cycle"
    REFLEX_CYCLE         = "core.reflex_cycle"
    
    # Judgment Loop
    PERCEPTION_RECEIVED  = "core.perception_received"
    PERCEPTION_FAILED    = "core.perception_failed"
    JUDGMENT_REQUESTED   = "core.judgment_requested"
    JUDGMENT_CREATED     = "core.judgment_created"     # Result is ready
    JUDGMENT_FAILED      = "core.judgment_failed"
    CONSENSUS_REACHED    = "core.consensus_reached"
    CONSENSUS_FAILED     = "core.consensus_failed"
    
    # Action / Metabolism
    DECISION_MADE        = "core.decision_made"        # Agent chose an action
    ACTION_PROPOSED      = "core.action_proposed"
    ACT_REQUESTED        = "core.act_requested"
    ACT_START            = "core.act_start"
    ACT_COMPLETED        = "core.act_completed"
    ACT_FAILED           = "core.act_failed"
    
    # Learning
    LEARNING_EVENT       = "core.learning_event"       # Feedback signal
    USER_FEEDBACK        = "core.user_feedback"
    USER_CORRECTION      = "core.user_correction"      # Direct human override
    Q_TABLE_UPDATED      = "core.q_table_updated"
    EWC_CHECKPOINT       = "core.ewc_checkpoint"       # Fisher importance consolidated
    
    # Sovereignty
    VALUE_CREATED        = "core.value_created"
    
    # Immune / Safety
    ANOMALY_DETECTED     = "core.anomaly_detected"     # Circuit breaker
    BUDGET_WARNING       = "core.budget_warning"
    BUDGET_EXHAUSTED     = "core.budget_exhausted"
    AXIOM_ACTIVATED      = "core.axiom_activated"
    
    # Health / Metrics
    DISK_PRESSURE        = "health.disk_pressure"
    DISK_CLEARED         = "health.disk_cleared"
    MEMORY_PRESSURE      = "health.memory_pressure"
    MEMORY_CLEARED       = "health.memory_cleared"
    CPU_PRESSURE         = "health.cpu_pressure"
    CPU_COOLDOWN         = "health.cpu_cooldown"
    STALL_DETECTED       = "health.stall_detected"
    SYSTEM_RESTARTED     = "health.system_restarted"
    # High-level Axiom Signals
    AUTONOMY             = "core.autonomy"
    SYMBIOSIS            = "core.symbiosis"
    EMERGENCE            = "core.emergence"
    ANTIFRAGILITY        = "core.antifragility"
    CONSCIOUSNESS        = "core.consciousness"
    TRANSCENDENCE        = "core.transcendence"
    
    RESIDUAL_HIGH        = "core.residual_high"        # High entropy detected
    EMERGENCE_DETECTED   = "core.emergence_detected"   # New pattern identified
    SELF_IMPROVEMENT_PROPOSED = "core.self_improvement_proposed"
    SELF_CORRECTION_MADE = "core.self_correction_made"
    POWER_GATE_OPENED    = "core.power_gate_opened"    # PowerLimiter allows action
    ALIGNMENT_FAILED     = "core.alignment_failed"     # Safety check failed
    
    # Topology / Nervous
    SOURCE_CHANGED       = "topology.source_changed"       # Code file changed (for ChangeAnalyzer)
    TOPOLOGY_CHANGED     = "topology.topology_changed"
    TOPOLOGY_SNAPSHOT    = "topology.topology_snapshot"
    CHANGE_ANALYZED      = "topology.change_analyzed"      # ChangeAnalyzer completed analysis

    # SDK / Developer Tools
    SDK_SESSION_STARTED  = "sdk.session_started"
    SDK_TOOL_JUDGED      = "sdk.tool_judged"
    SDK_RESULT_RECEIVED  = "sdk.result_received"
    SDK_SYNC_STARTED     = "sdk.sync_started"
    SDK_FEEDBACK_LOOP    = "sdk.feedback_loop"
    
    # MCP (Model Context Protocol bridge)
    MCP_BRIDGE_CONNECTED = "mcp.bridge_connected"
    MCP_TOOL_CALLED      = "mcp.tool_called"
    MCP_RESOURCE_READ    = "mcp.resource_read"

EventType = Union[str, CoreEvent, AgentEvent, AutomationEvent]

class Event:
    """A generic event container."""
    def __init__(self, type: str, payload: Any = None, source: str = "unknown"):
        self.type = type
        self.payload = payload
        self.source = source
        self.event_id = str(time.time_ns())
        self.timestamp = time.time()

    @property
    def dict_payload(self) -> dict:
        """Returns payload as a dictionary, even if it's a Pydantic model."""
        if self.payload is None:
            return {}
        if isinstance(self.payload, dict):
            return self.payload
        if hasattr(self.payload, "model_dump"):
            return self.payload.model_dump()
        if hasattr(self.payload, "__dict__"):
            return self.payload.__dict__
        return {"data": self.payload}

    @classmethod
    def typed(cls, type: CoreEvent, payload: Any = None, source: str = "unknown") -> Event:
        return cls(type.value, payload, source)

    def as_typed(self, payload_type: Type[T]) -> T:
        """Cast payload to the specified Pydantic model type."""
        if self.payload is None:
            raise EventBusError(f"Event has no payload for type {payload_type.__name__}")
        if isinstance(self.payload, payload_type):
            return self.payload
        # If payload is dict-like, try to construct the model
        if hasattr(payload_type, "model_validate"):
            # Pydantic v2
            return payload_type.model_validate(self.payload)
        elif hasattr(payload_type, "parse_obj"):
            # Pydantic v1
            return payload_type.parse_obj(self.payload)
        else:
            raise EventBusError(f"Cannot convert payload to {payload_type.__name__}")

    def to_dict(self) -> dict:
        return {
            "event_id": self.event_id,
            "type": self.type,
            "source": self.source,
            "timestamp": self.timestamp,
            "payload": self.payload
        }

Handler = Callable[[Event], Coroutine[Any, Any, None]]

class EventBus:
    """
    Asynchronous event bus with task tracking and graceful draining.
    """
    def __init__(self, bus_id: str = "default") -> None:
        self.bus_id = bus_id
        self._handlers: dict[str, list[Handler]] = defaultdict(list)
        self._history: list[Event] = []
        self._max_history = _EVENT_HISTORY_MAX
        self._emitted_count = 0
        self._error_count = 0
        self._pending_tasks: set[asyncio.Task] = set()
        self._handler_timeout_s = 30.0

    def on(self, event_type: Union[str, CoreEvent], handler: Handler) -> None:
        e_type = event_type.value if isinstance(event_type, CoreEvent) else event_type
        self._handlers[e_type].append(handler)

    def off(self, event_type: Union[str, CoreEvent], handler: Handler) -> None:
        e_type = event_type.value if isinstance(event_type, CoreEvent) else event_type
        if e_type in self._handlers and handler in self._handlers[e_type]:
            self._handlers[e_type].remove(handler)

    async def emit(self, event: Event) -> None:
        self._emitted_count += 1
        self._history.append(event)
        if len(self._history) > self._max_history:
            self._history.pop(0)

        handlers = self._handlers.get(event.type, [])
        wildcards = self._handlers.get("*", [])
        all_handlers = handlers + wildcards

        for h in all_handlers:
            task = asyncio.create_task(
                self._run_handler(h, event),
                name=f"bus_{self.bus_id}_{event.type}"
            )
            self._pending_tasks.add(task)
            task.add_done_callback(self._pending_tasks.discard)

    async def _run_handler(self, handler: Handler, event: Event) -> None:
        try:
            await asyncio.wait_for(handler(event), timeout=self._handler_timeout_s)
        except asyncio.TimeoutError:
            self._error_count += 1
            logger.warning("Handler timeout on bus %s: %s", self.bus_id, event.type)
        except Exception as e:
            self._error_count += 1
            logger.error("Handler error on bus %s [%s]: %s", self.bus_id, event.type, e, exc_info=True)

    async def drain(self, timeout: float = 2.0) -> None:
        """Await all pending background tasks before shutdown."""
        if not self._pending_tasks:
            return
        logger.info("EventBus '%s': draining %d tasks...", self.bus_id, len(self._pending_tasks))
        try:
            await asyncio.wait_for(
                asyncio.gather(*self._pending_tasks, return_exceptions=True),
                timeout=timeout
            )
        except asyncio.TimeoutError:
            logger.warning("EventBus '%s': drain timed out", self.bus_id)
        finally:
            self._pending_tasks.clear()

    def stats(self) -> dict:
        """Return statistics about the event bus."""
        return {
            "bus_id": self.bus_id,
            "pending_tasks": len(self._pending_tasks),
            "handlers_count": len(self._handlers),
            "event_types": list(self._handlers.keys()),
            "total_emitted": self._emitted_count,
            "total_errors": self._error_count,
            "history_size": len(self._history),
        }

# Singletons
_buses: dict[str, EventBus] = {}

def get_bus(bus_id: str) -> EventBus:
    if bus_id not in _buses:
        _buses[bus_id] = EventBus(bus_id)
    return _buses[bus_id]

def get_core_bus() -> EventBus:
    return get_bus("CORE")

def get_automation_bus() -> EventBus:
    return get_bus("AUTOMATION")

def get_agent_bus() -> EventBus:
    return get_bus("AGENT")

def reset_all_buses() -> None:
    """Reset all global event buses to clean state.

    Useful for test isolation. Clears all handlers and pending tasks.
    """
    global _buses
    for bus in _buses.values():
        bus._handlers.clear()
        bus._pending_tasks.clear()
        bus._history.clear()
        bus._emitted_count = 0
        bus._error_count = 0
    # Note: Don't clear _buses dict itself, just their state

def create_default_bridge() -> None:
    """Create default bridges between event buses.

    Legacy function for test compatibility. Does nothing in current architecture.
    """
    pass
