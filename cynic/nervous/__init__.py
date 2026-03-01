"""
CYNIC Tier 1 Nervous System

Foundational components for self-observing organism:
  - service_registry: Real-time health tracking of all components
  - event_journal: Queryable log of all events with causality tracing
  - decision_trace: Extract reasoning path for each judgment
  - loop_closure: Verify all feedback loops complete without orphans

These 4 components enable CYNIC to observe itself in real-time,
detect silent failures, and propose self-corrections (L4 meta-improvement).
"""

from cynic.nervous.decision_trace import (
    DecisionTrace,
    DecisionTracer,
    DogRole,
    DogVote,
    TraceNode,
)
from cynic.nervous.event_journal import (
    EventCategory,
    EventJournal,
    JournalEntry,
)
from cynic.nervous.loop_closure import (
    CyclePhase,
    LoopClosureEvent,
    LoopClosureValidator,
)
from cynic.nervous.service_registry import (
    ComponentSnapshot,
    ComponentType,
    HealthStatus,
    RegistrySnapshot,
    ServiceStateRegistry,
)
from cynic.nervous.bus_loop_closure_adapter import BusLoopClosureAdapter
from cynic.nervous.state_reconstructor import StateReconstructor

__all__ = [
    "ServiceStateRegistry",
    "ComponentSnapshot",
    "RegistrySnapshot",
    "ComponentType",
    "HealthStatus",
    "EventJournal",
    "EventCategory",
    "JournalEntry",
    "DecisionTracer",
    "DecisionTrace",
    "TraceNode",
    "DogVote",
    "DogRole",
    "LoopClosureValidator",
    "LoopClosureEvent",
    "CyclePhase",
    "BusLoopClosureAdapter",
    "StateReconstructor",
]
