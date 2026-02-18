"""
CYNIC Event Bus — 3 Buses + EventBusBridge

Three buses, one organism:
  CORE bus     — Judgment, Learning, Consciousness events (system-wide)
  AUTOMATION bus — Triggers, Ticks, Automation cycle events
  AGENT bus    — Dog signals, votes, PBFT protocol messages

EventBusBridge connects all 3 with genealogy tracking to prevent loops.

LAW (from JS lessons learned):
  - Wire events BEFORE start() — bus accepts connections immediately
  - ALL forwarded events carry _genealogy list (loop prevention)
  - Bridged events tagged _bridged=True — never re-forward
  - asyncio.Queue based — no threading, no locks, no races
"""
from __future__ import annotations

import asyncio
import logging
import time
import uuid
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Callable, Coroutine, Dict, List, Optional, Set, Tuple

logger = logging.getLogger("cynic.event_bus")


# ════════════════════════════════════════════════════════════════════════════
# EVENT ENVELOPE
# ════════════════════════════════════════════════════════════════════════════

@dataclass
class Event:
    """
    Universal event envelope — all 3 buses use this format.

    Genealogy tracks the chain of buses this event has traversed.
    If a bus ID appears in genealogy → do NOT re-forward (loop prevention).
    """
    type: str
    payload: Dict[str, Any] = field(default_factory=dict)
    event_id: str = field(default_factory=lambda: str(uuid.uuid4()))
    timestamp: float = field(default_factory=time.time)
    source: str = ""                     # Which Dog/module emitted this
    _genealogy: List[str] = field(default_factory=list)  # Bus IDs traversed
    _bridged: bool = False               # True if forwarded by bridge

    def with_genealogy(self, bus_id: str) -> "Event":
        """Return copy of this event with bus_id appended to genealogy."""
        return Event(
            type=self.type,
            payload=self.payload,
            event_id=self.event_id,
            timestamp=self.timestamp,
            source=self.source,
            _genealogy=[*self._genealogy, bus_id],
            _bridged=True,
        )

    def already_seen(self, bus_id: str) -> bool:
        """True if this event has already passed through bus_id."""
        return bus_id in self._genealogy


# ════════════════════════════════════════════════════════════════════════════
# CORE BUS EVENTS
# ════════════════════════════════════════════════════════════════════════════

class CoreEvent(str, Enum):
    """Events on the CORE bus (judgment pipeline, learning, consciousness)."""
    # Judgment lifecycle
    JUDGMENT_REQUESTED = "judgment.requested"
    JUDGMENT_CREATED   = "judgment.created"
    JUDGMENT_FAILED    = "judgment.failed"
    CONSENSUS_REACHED  = "consensus.reached"
    CONSENSUS_FAILED   = "consensus.failed"

    # Learning
    LEARNING_EVENT     = "learning.event"
    Q_TABLE_UPDATED    = "learning.q_table_updated"
    EWC_CHECKPOINT     = "learning.ewc_checkpoint"
    SONA_TICK          = "learning.sona_tick"
    META_CYCLE         = "learning.meta_cycle"

    # Perception
    PERCEPTION_RECEIVED = "perception.received"
    ANOMALY_DETECTED    = "perception.anomaly"

    # Consciousness
    CONSCIOUSNESS_CHANGED = "consciousness.changed"
    BUDGET_WARNING        = "budget.warning"
    BUDGET_EXHAUSTED      = "budget.exhausted"

    # User feedback
    USER_FEEDBACK    = "user.feedback"
    USER_CORRECTION  = "user.correction"

    # Emergence
    EMERGENCE_DETECTED = "emergence.detected"
    RESIDUAL_HIGH      = "emergence.residual_high"
    TRANSCENDENCE      = "emergence.transcendence"

    # Identity
    IDENTITY_VIOLATION = "identity.violation"

    # Decision / Act
    ACT_REQUESTED  = "act.requested"
    DECISION_MADE  = "decide.made"

    # SDK (Claude Code --sdk-url sessions)
    SDK_SESSION_STARTED  = "sdk.session_started"
    SDK_TOOL_JUDGED      = "sdk.tool_judged"
    SDK_RESULT_RECEIVED  = "sdk.result_received"


# ════════════════════════════════════════════════════════════════════════════
# AUTOMATION BUS EVENTS
# ════════════════════════════════════════════════════════════════════════════

class AutomationEvent(str, Enum):
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


# ════════════════════════════════════════════════════════════════════════════
# AGENT BUS EVENTS (Dogs)
# ════════════════════════════════════════════════════════════════════════════

class AgentEvent(str, Enum):
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


# ════════════════════════════════════════════════════════════════════════════
# EVENT BUS (asyncio-based, typed)
# ════════════════════════════════════════════════════════════════════════════

Handler = Callable[[Event], Coroutine[Any, Any, None]]


class EventBus:
    """
    Single-process asyncio event bus.

    Subscribers register handlers per event type (or "*" for all).
    Emit is fire-and-forget — handlers run as Tasks.

    No threading, no locks. Pure asyncio.
    """

    def __init__(self, bus_id: str) -> None:
        self.bus_id = bus_id
        self._handlers: Dict[str, List[Handler]] = {}
        self._history: List[Event] = []
        self._max_history = 1000
        self._emitted_count = 0
        self._error_count = 0

    def on(self, event_type: str, handler: Handler) -> None:
        """Register a handler for an event type (or '*' for all)."""
        if event_type not in self._handlers:
            self._handlers[event_type] = []
        self._handlers[event_type].append(handler)

    def off(self, event_type: str, handler: Handler) -> None:
        """Unregister a handler."""
        if event_type in self._handlers:
            try:
                self._handlers[event_type].remove(handler)
            except ValueError:
                pass

    async def emit(self, event: Event) -> None:
        """Emit an event. All handlers are invoked as asyncio Tasks."""
        self._emitted_count += 1

        # Record in history
        self._history.append(event)
        if len(self._history) > self._max_history:
            self._history.pop(0)

        # Collect matching handlers
        handlers = (
            self._handlers.get(event.type, [])
            + self._handlers.get("*", [])
        )

        # Fire as Tasks (fire-and-forget)
        for handler in handlers:
            asyncio.create_task(self._run_handler(handler, event))

    async def _run_handler(self, handler: Handler, event: Event) -> None:
        try:
            await handler(event)
        except Exception as e:
            self._error_count += 1
            logger.error(
                "Handler error on bus=%s type=%s: %s",
                self.bus_id, event.type, e,
                exc_info=True,
            )

    def emit_sync(self, event: Event) -> None:
        """
        Emit from sync context (creates task if loop running, else schedules).

        Use only from non-async code. Prefer async emit() everywhere else.
        """
        try:
            loop = asyncio.get_event_loop()
            if loop.is_running():
                loop.create_task(self.emit(event))
            else:
                loop.run_until_complete(self.emit(event))
        except RuntimeError:
            # No event loop — defer until available
            logger.warning("emit_sync called with no event loop for %s", event.type)

    def stats(self) -> Dict[str, Any]:
        return {
            "bus_id": self.bus_id,
            "emitted": self._emitted_count,
            "errors": self._error_count,
            "handlers": {k: len(v) for k, v in self._handlers.items()},
            "history_size": len(self._history),
        }


# ════════════════════════════════════════════════════════════════════════════
# THREE BUS SINGLETONS
# ════════════════════════════════════════════════════════════════════════════

_core_bus: Optional[EventBus] = None
_automation_bus: Optional[EventBus] = None
_agent_bus: Optional[EventBus] = None


def get_core_bus() -> EventBus:
    """CORE bus — judgment, learning, consciousness events."""
    global _core_bus
    if _core_bus is None:
        _core_bus = EventBus(bus_id="CORE")
    return _core_bus


def get_automation_bus() -> EventBus:
    """AUTOMATION bus — triggers, ticks, scheduled tasks."""
    global _automation_bus
    if _automation_bus is None:
        _automation_bus = EventBus(bus_id="AUTOMATION")
    return _automation_bus


def get_agent_bus() -> EventBus:
    """AGENT bus — Dog signals, PBFT, voting."""
    global _agent_bus
    if _agent_bus is None:
        _agent_bus = EventBus(bus_id="AGENT")
    return _agent_bus


def reset_all_buses() -> None:
    """Reset all buses (for testing only)."""
    global _core_bus, _automation_bus, _agent_bus
    _core_bus = None
    _automation_bus = None
    _agent_bus = None


# ════════════════════════════════════════════════════════════════════════════
# EVENT BUS BRIDGE (loop-safe genealogy tracking)
# ════════════════════════════════════════════════════════════════════════════

@dataclass
class ForwardRule:
    """A rule for forwarding events between buses."""
    source_bus_id: str
    target_bus_id: str
    event_types: Set[str]          # Empty set = forward all
    transform: Optional[Callable[[Event], Event]] = None  # Optional transform


class EventBusBridge:
    """
    Connects the 3 buses with genealogy-based loop prevention.

    Forward rules (from JS architecture, rewritten in Python):
      AGENT → CORE:  PBFT_REPLY, DOG_VETO, COLLECTIVE_SIGNAL, LLM_BENCHMARK_DONE
      CORE → AGENT:  JUDGMENT_CREATED, PERCEPTION_RECEIVED, BUDGET_WARNING
      AUTO → CORE:   PRICE_TICK, SOCIAL_SIGNAL, TX_CONFIRMED, TX_FAILED
      CORE → AUTO:   BUDGET_EXHAUSTED (trigger throttling)

    Loop prevention:
      Every forwarded event gets its genealogy updated with the source bus ID.
      If target bus ID is already in genealogy → skip (event already visited).
    """

    def __init__(self) -> None:
        self._rules: List[ForwardRule] = []
        self._buses: Dict[str, EventBus] = {}
        self._forwarded_count = 0
        self._loop_prevented_count = 0
        self._active = False
        self._forwarders: Dict[str, Handler] = {}  # bus_id → handler (for cleanup)

    def register_bus(self, bus: EventBus) -> None:
        self._buses[bus.bus_id] = bus

    def add_rule(self, rule: ForwardRule) -> None:
        self._rules.append(rule)

    def start(self) -> None:
        """Wire all buses according to registered rules."""
        if self._active:
            return
        self._active = True

        # Register a wildcard handler on each source bus; keep ref for stop()
        source_ids: Set[str] = {r.source_bus_id for r in self._rules}
        for bus_id in source_ids:
            bus = self._buses.get(bus_id)
            if bus:
                handler = self._make_forwarder(bus_id)
                self._forwarders[bus_id] = handler
                bus.on("*", handler)

        logger.info(
            "EventBusBridge started — %d buses, %d rules",
            len(self._buses), len(self._rules)
        )

    def stop(self) -> None:
        """Deregister all wildcard handlers."""
        if not self._active:
            return
        self._active = False
        for bus_id, handler in self._forwarders.items():
            bus = self._buses.get(bus_id)
            if bus:
                bus.off("*", handler)
        self._forwarders.clear()
        logger.info("EventBusBridge stopped")

    def _make_forwarder(self, source_bus_id: str) -> Handler:
        """Create a forwarding handler for a specific source bus."""
        async def _forward(event: Event) -> None:
            for rule in self._rules:
                if rule.source_bus_id != source_bus_id:
                    continue
                # Check if this event type matches the rule
                if rule.event_types and event.type not in rule.event_types:
                    continue
                # Loop prevention — don't re-forward if target already in genealogy
                if event.already_seen(rule.target_bus_id):
                    self._loop_prevented_count += 1
                    continue
                # Don't re-forward a bridged event from the same direction
                if event._bridged and source_bus_id in event._genealogy:
                    self._loop_prevented_count += 1
                    continue

                target_bus = self._buses.get(rule.target_bus_id)
                if not target_bus:
                    continue

                # Create bridged event with genealogy
                bridged = event.with_genealogy(source_bus_id)

                # Apply optional transform
                if rule.transform:
                    bridged = rule.transform(bridged)

                self._forwarded_count += 1
                await target_bus.emit(bridged)

        return _forward

    def stats(self) -> Dict[str, Any]:
        return {
            "active": self._active,
            "buses": list(self._buses.keys()),
            "rules": len(self._rules),
            "forwarded": self._forwarded_count,
            "loops_prevented": self._loop_prevented_count,
        }


# ════════════════════════════════════════════════════════════════════════════
# DEFAULT BRIDGE CONFIGURATION
# ════════════════════════════════════════════════════════════════════════════

def create_default_bridge() -> EventBusBridge:
    """
    Create and configure the default EventBusBridge.

    Forward rules reflect the CYNIC organism's information flow:
      AGENT (Dogs) → CORE (Judgment pipeline) — consensus results, vetoes
      CORE (Judgment) → AGENT (Dogs) — new judgments to process, budget alerts
      AUTO (Triggers) → CORE (Learning) — market/social/solana signals
      CORE (Budget) → AUTO (Throttling) — budget exhaustion slows triggers
    """
    bridge = EventBusBridge()

    core = get_core_bus()
    auto = get_automation_bus()
    agent = get_agent_bus()

    bridge.register_bus(core)
    bridge.register_bus(auto)
    bridge.register_bus(agent)

    # AGENT → CORE
    bridge.add_rule(ForwardRule(
        source_bus_id="AGENT",
        target_bus_id="CORE",
        event_types={
            AgentEvent.PBFT_REPLY,
            AgentEvent.DOG_VETO,
            AgentEvent.COLLECTIVE_SIGNAL,
            AgentEvent.LLM_BENCHMARK_DONE,
            AgentEvent.DOG_VOTE,
        },
    ))

    # CORE → AGENT
    bridge.add_rule(ForwardRule(
        source_bus_id="CORE",
        target_bus_id="AGENT",
        event_types={
            CoreEvent.JUDGMENT_CREATED,
            CoreEvent.PERCEPTION_RECEIVED,
            CoreEvent.BUDGET_WARNING,
            CoreEvent.BUDGET_EXHAUSTED,
        },
    ))

    # AUTOMATION → CORE
    bridge.add_rule(ForwardRule(
        source_bus_id="AUTOMATION",
        target_bus_id="CORE",
        event_types={
            AutomationEvent.PRICE_TICK,
            AutomationEvent.SOCIAL_SIGNAL,
            AutomationEvent.TX_CONFIRMED,
            AutomationEvent.TX_FAILED,
            AutomationEvent.MARKET_ALERT,
        },
    ))

    # CORE → AUTOMATION
    bridge.add_rule(ForwardRule(
        source_bus_id="CORE",
        target_bus_id="AUTOMATION",
        event_types={
            CoreEvent.BUDGET_EXHAUSTED,   # slow down triggers when broke
            CoreEvent.META_CYCLE,         # evolution tick → external consumers
            CoreEvent.EMERGENCE_DETECTED, # emergence pattern → automation layer
            CoreEvent.DECISION_MADE,      # DECIDE result → automation can react
        },
    ))

    # CORE → AGENT (extend: META_CYCLE lets Dogs receive evolution context)
    bridge.add_rule(ForwardRule(
        source_bus_id="CORE",
        target_bus_id="AGENT",
        event_types={
            CoreEvent.META_CYCLE,
            CoreEvent.EMERGENCE_DETECTED,
        },
    ))

    return bridge


# ════════════════════════════════════════════════════════════════════════════
# CONVENIENCE HELPERS
# ════════════════════════════════════════════════════════════════════════════

async def emit_core(event_type: str, payload: Dict[str, Any], source: str = "") -> None:
    """Emit to CORE bus."""
    await get_core_bus().emit(Event(type=event_type, payload=payload, source=source))


async def emit_automation(event_type: str, payload: Dict[str, Any], source: str = "") -> None:
    """Emit to AUTOMATION bus."""
    await get_automation_bus().emit(Event(type=event_type, payload=payload, source=source))


async def emit_agent(event_type: str, payload: Dict[str, Any], source: str = "") -> None:
    """Emit to AGENT bus."""
    await get_agent_bus().emit(Event(type=event_type, payload=payload, source=source))
