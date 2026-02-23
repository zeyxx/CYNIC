"""EcosystemObserver — Read-only nervous system wrapper for ecosystem queries.

This is the L1 gateway to the nervous system (Tier 1 components):
- EventJournal: sequence of all events in order
- DecisionTrace: DAGs of judgment reasoning paths
- ServiceRegistry: component health snapshots

EcosystemObserver aggregates queries across these 3 sources.
No mutations, no state changes — purely observational.
"""

from __future__ import annotations

import logging
import time
from dataclasses import asdict
from typing import Any, Optional

from cynic.nervous.event_journal import EventJournal, JournalEntry, EventCategory
from cynic.nervous.decision_trace import DecisionTracer, TraceNode, DecisionTrace
from cynic.nervous.service_registry import ServiceStateRegistry, ComponentSnapshot, RegistrySnapshot

logger = logging.getLogger(__name__)


class EcosystemObserver:
    """Read-only observer of CYNIC nervous system (Tier 1).

    Wraps:
    1. EventJournal — all events in sequence
    2. DecisionTracer — judgment reasoning DAGs
    3. ServiceStateRegistry — component health snapshots

    Public methods:
    - event_history(limit=10) — recent events
    - perception_sources() — aggregate perception event counts by source
    - handler_traces(judgment_id) — decision trace nodes for judgment
    - ecosystem_snapshot() — full state aggregation
    """

    def __init__(
        self,
        journal: EventJournal,
        decision_trace: DecisionTracer,
        handlers_registry: ServiceStateRegistry,
    ):
        """Initialize with nervous system components."""
        self.journal = journal
        self.decision_trace = decision_trace
        self.handlers_registry = handlers_registry

    async def event_history(self, limit: int = 10) -> list[JournalEntry]:
        """Query recent events from event journal.

        Args:
            limit: max number of events to return

        Returns:
            List of recent JournalEntry objects, newest first
        """
        return await self.journal.recent(limit=limit)

    async def perception_sources(self) -> dict[str, int]:
        """Aggregate perception event sources from journal.

        Groups perception events by source and counts occurrences.

        Returns:
            Dict mapping source name → count of perception events
        """
        perception_events = await self.journal.filter_by_category(EventCategory.PERCEPTION)
        sources: dict[str, int] = {}

        for event in perception_events:
            source = event.source
            sources[source] = sources.get(source, 0) + 1

        return sources

    async def handler_traces(self, judgment_id: str) -> list[dict[str, Any]]:
        """Query decision trace nodes for a judgment.

        Args:
            judgment_id: ID of the judgment to trace

        Returns:
            List of trace node dicts, ordered by phase
        """
        trace = await self.decision_trace.get_trace_by_judgment(judgment_id)
        if not trace:
            return []

        # Convert TraceNode objects to dicts
        return [asdict(node) for node in trace.nodes]

    async def ecosystem_snapshot(self) -> dict[str, Any]:
        """Full ecosystem state aggregation.

        Combines data from all 3 nervous system components into a single snapshot.

        Returns:
            Dict with:
            - timestamp: snapshot time
            - event_count: total events in journal
            - recent_judgments: list of recent judgment info
            - perception_sources: aggregated perception sources
            - component_health: registry snapshot
        """
        now_ms = time.time() * 1000

        # Get recent events
        recent_events = await self.journal.recent(limit=10)

        # Get recent judgments from decision trace
        recent_traces = await self.decision_trace.recent_traces(limit=5)
        recent_judgments = []
        for trace in recent_traces:
            recent_judgments.append({
                "judgment_id": trace.judgment_id,
                "created_at_ms": trace.created_at_ms,
                "final_verdict": trace.final_verdict,
                "final_q_score": trace.final_q_score,
                "final_confidence": trace.final_confidence,
            })

        # Get perception sources
        perception_agg = await self.perception_sources()

        # Get component health
        registry_snapshot = await self.handlers_registry.snapshot()
        component_health = {
            name: comp.to_dict()
            for name, comp in registry_snapshot.components.items()
        }

        return {
            "timestamp": now_ms,
            "event_count": len(recent_events),
            "recent_events": [e.to_dict() for e in recent_events],
            "recent_judgments": recent_judgments,
            "perception_sources": perception_agg,
            "component_health": component_health,
            "total_components": registry_snapshot.total_components,
            "healthy_count": registry_snapshot.healthy_count,
        }
