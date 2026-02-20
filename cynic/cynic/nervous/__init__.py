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

from cynic.nervous.service_registry import (
    ServiceStateRegistry,
    ComponentSnapshot,
    RegistrySnapshot,
    ComponentType,
    HealthStatus,
    get_service_registry,
    reset_service_registry,
)
from cynic.nervous.event_journal import (
    EventJournal,
    EventCategory,
    JournalEntry,
)
from cynic.nervous.decision_trace import (
    DecisionTracer,
    DecisionTrace,
    TraceNode,
    DogVote,
    DogRole,
)
from cynic.nervous.loop_closure import (
    LoopClosureValidator,
    LoopClosureEvent,
    CyclePhase,
)

__all__ = [
    "ServiceStateRegistry",
    "ComponentSnapshot",
    "RegistrySnapshot",
    "ComponentType",
    "HealthStatus",
    "get_service_registry",
    "reset_service_registry",
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
]
