"""
BusLoopClosureAdapter — Wires LoopClosureValidator to EventBus.

Routes judgment lifecycle events into LoopClosureValidator to detect
stalls and orphaned judgments.

Usage:
    adapter = BusLoopClosureAdapter(validator)
    bus.on("*", adapter.on_event)
"""
from __future__ import annotations

from cynic.kernel.core.event_bus import Event
from cynic.nervous.loop_closure import CyclePhase, LoopClosureValidator

# Events that open a new cycle
_CYCLE_START = {"core.judgment_requested", "core.perception_received"}

# Events that record a phase transition
_PHASE_MAP: dict[str, CyclePhase] = {
    "core.judgment_created":    CyclePhase.JUDGE,
    "core.judgment_failed":     CyclePhase.JUDGE,
    "core.consensus_reached":   CyclePhase.JUDGE,
    "core.consensus_failed":    CyclePhase.JUDGE,
    "core.decision_made":       CyclePhase.DECIDE,
    "core.action_proposed":     CyclePhase.DECIDE,
    "core.act_requested":       CyclePhase.ACT,
    "core.act_completed":       CyclePhase.ACT,
    "core.learning_event":      CyclePhase.LEARN,
    "core.q_table_updated":     CyclePhase.LEARN,
    "core.value_created":       CyclePhase.ACCOUNT,
    "core.reputation_sync":     CyclePhase.ACCOUNT,
    "core.emergence_detected":  CyclePhase.EMERGE,
    "core.transcendence":       CyclePhase.EMERGE,
}

# Events that close (finalize) a cycle
_CYCLE_CLOSE = {"core.emergence_detected", "core.transcendence", "core.act_completed"}


def _extract_judgment_id(event: Event) -> str | None:
    """Try to find a judgment_id in the event payload."""
    payload = event.dict_payload
    return (
        payload.get("judgment_id")
        or payload.get("id")
        or payload.get("cell_id")
    )


class BusLoopClosureAdapter:
    """Wildcard bus handler that feeds lifecycle events to LoopClosureValidator."""

    def __init__(self, validator: LoopClosureValidator) -> None:
        self._validator = validator
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
        """Route bus event to the appropriate validator call."""
        judgment_id = _extract_judgment_id(event)
        if not judgment_id:
            return  # Cannot track without a judgment_id

        if event.type in _CYCLE_START:
            await self._validator.start_cycle(
                judgment_id=judgment_id,
                initial_event_id=event.event_id,
                component=event.source,
            )

        elif event.type in _CYCLE_CLOSE:
            # Record the phase first (e.g. EMERGE), then close
            phase = _PHASE_MAP.get(event.type)
            if phase:
                await self._validator.record_phase(
                    judgment_id=judgment_id,
                    phase=phase,
                    event_id=event.event_id,
                    component=event.source,
                )
            await self._validator.close_cycle(judgment_id)

        elif event.type in _PHASE_MAP:
            await self._validator.record_phase(
                judgment_id=judgment_id,
                phase=_PHASE_MAP[event.type],
                event_id=event.event_id,
                component=event.source,
            )
