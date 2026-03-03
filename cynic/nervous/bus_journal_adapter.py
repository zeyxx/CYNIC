"""
BusJournalAdapter  Wires EventJournal to EventBus via wildcard subscription.

Usage:
    adapter = BusJournalAdapter(journal)
    bus.on("*", adapter.on_event)
"""
from __future__ import annotations

from cynic.kernel.core.event_bus import Event
from cynic.nervous.event_journal import EventCategory, EventJournal

# Maps CoreEvent.value strings  EventCategory
_CATEGORY_MAP: dict[str, EventCategory] = {
    # PERCEPTION
    "core.perception_received": EventCategory.PERCEPTION,
    "core.judgment_requested": EventCategory.PERCEPTION,
    # JUDGMENT
    "core.judgment_created": EventCategory.JUDGMENT,
    "core.judgment_failed": EventCategory.JUDGMENT,
    "core.consensus_reached": EventCategory.JUDGMENT,
    "core.consensus_failed": EventCategory.JUDGMENT,
    "core.residual_high": EventCategory.JUDGMENT,
    # DECISION
    "core.decision_made": EventCategory.DECISION,
    "core.action_proposed": EventCategory.DECISION,
    # ACTION
    "core.act_requested": EventCategory.ACTION,
    "core.act_completed": EventCategory.ACTION,
    "mcp.tool_called": EventCategory.ACTION,
    "mcp.result_received": EventCategory.ACTION,
    "sdk.result_received": EventCategory.ACTION,
    "core.sdk_tool_judged": EventCategory.ACTION,
    # LEARNING
    "core.learning_event": EventCategory.LEARNING,
    "core.q_table_updated": EventCategory.LEARNING,
    "core.ewc_checkpoint": EventCategory.LEARNING,
    "core.user_correction": EventCategory.LEARNING,
    "core.sona_aggregated": EventCategory.LEARNING,
    # ACCOUNTING
    "core.value_created": EventCategory.ACCOUNTING,
    "core.reputation_sync": EventCategory.ACCOUNTING,
    "core.gossip_synced": EventCategory.ACCOUNTING,
    "core.budget_warning": EventCategory.ACCOUNTING,
    "core.budget_exhausted": EventCategory.ACCOUNTING,
    # EMERGENCE
    "core.emergence_detected": EventCategory.EMERGENCE,
    "core.transcendence": EventCategory.EMERGENCE,
    "core.self_improvement_proposed": EventCategory.EMERGENCE,
    "core.anomaly_detected": EventCategory.EMERGENCE,
    # SYSTEM
    "core.awakened": EventCategory.SYSTEM,
    "core.dormant": EventCategory.SYSTEM,
    "core.sona_tick": EventCategory.SYSTEM,
    "core.meta_cycle": EventCategory.SYSTEM,
    "core.lod_changed": EventCategory.SYSTEM,
    "core.consciousness_changed": EventCategory.SYSTEM,
    "core.configuration_mutated": EventCategory.SYSTEM,
    "core.axiom_activated": EventCategory.SYSTEM,
    "core.internal_error": EventCategory.SYSTEM,
    "core.sdk_session_started": EventCategory.SYSTEM,
    "core.sdk_session_ended": EventCategory.SYSTEM,
    "health.disk_pressure": EventCategory.SYSTEM,
    "health.disk_cleared": EventCategory.SYSTEM,
    "health.memory_pressure": EventCategory.SYSTEM,
    "health.memory_cleared": EventCategory.SYSTEM,
    # ADMIN
    "core.user_feedback": EventCategory.ADMIN,
}


class BusJournalAdapter:
    """Wildcard bus handler that records every event into an EventJournal."""

    def __init__(self, journal: EventJournal) -> None:
        self._journal = journal
        self._bus: Any = None  # Set via attach()


    def attach(self, bus: Any) -> None:
        """Store bus reference for cleanup."""
        self._bus = bus

    async def stop(self) -> None:
        """Unregister from bus wildcard subscription."""
        try:
            if self._bus is not None:
                self._bus.off("*", self.on_event)
        except Exception:
            pass

    async def on_event(self, event: Event) -> None:
        """Called by EventBus for every event (registered on "*")."""
        payload_dict = event.dict_payload
        if not isinstance(payload_dict, dict):
            payload_dict = {}
        category = _CATEGORY_MAP.get(event.type, EventCategory.SYSTEM)
        await self._journal.record(
            event_type=event.type,
            category=category,
            source=event.source,
            payload=payload_dict,
        )
