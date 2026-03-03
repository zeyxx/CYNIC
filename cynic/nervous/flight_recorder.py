"""
CYNIC Flight Recorder - Immutable Experiment Trace.

Listens to the EventBus and streams MCTS experiment data to SurrealDB.
Ensures that the organism's thought process (Fractal Trace) is preserved
even if the kernel crashes during auto-surgery.

Lentille: Data Engineer / Backend
"""

import logging
import time

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus
from cynic.kernel.core.storage.surreal import SurrealStorage

logger = logging.getLogger("cynic.nervous.flight_recorder")


class FlightRecorder:
    """
    Subscribes to experimental events and persists them asynchronously.
    """

    def __init__(self, storage: SurrealStorage, bus: EventBus):
        self.storage = storage
        self.bus = bus
        self._setup_subscriptions()

    def _setup_subscriptions(self) -> None:
        """Bind to MCTS and Surgery events."""
        # Using generic value_created for now, but should ideally be specific MCTS events
        self.bus.on(CoreEvent.VALUE_CREATED, self._on_experiment_event)
        self.bus.on(CoreEvent.PROPOSAL_EXECUTED, self._on_surgery_event)
        self.bus.on(CoreEvent.ANOMALY_DETECTED, self._on_anomaly)

    async def _on_experiment_event(self, event: Event) -> None:
        """Record MCTS hypothesis generation and evaluation."""
        if not hasattr(self.storage, "learning"):
            return

        payload = event.dict_payload
        # Only process if it's tagged as an experiment
        if payload.get("source") != "mcts_scientist":
            return

        record = {
            "timestamp": time.time(),
            "event_type": "experiment",
            "node_id": payload.get("node_id"),
            "hypothesis": payload.get("hypothesis"),
            "q_score": payload.get("q_score", 0.0),
            "status": payload.get("status", "UNKNOWN"),
        }

        try:
            # We reuse the learning repo as it's the closest fit for experimentation
            await self.storage.learning.save(record)
        except Exception as e:
            logger.error(f"Flight Recorder failed to save experiment trace: {e}")

    async def _on_surgery_event(self, event: Event) -> None:
        """Record Auto-Surgery attempts and outcomes."""
        payload = event.dict_payload
        if payload.get("executor") != "auto_surgeon":
            return

        record = {
            "timestamp": time.time(),
            "event_type": "surgery_attempt",
            "experiment_id": payload.get("experiment_id"),
            "success": payload.get("success", False),
            "diff_summary": payload.get("diff_summary", ""),
        }

        try:
            await self.storage.learning.save(record)
        except Exception as e:
            logger.error(f"Flight Recorder failed to save surgery trace: {e}")

    async def _on_anomaly(self, event: Event) -> None:
        """Record crashes during experiments."""
        payload = event.dict_payload
        if payload.get("context") != "sandbox":
            return

        record = {
            "timestamp": time.time(),
            "event_type": "sandbox_crash",
            "experiment_id": payload.get("experiment_id"),
            "error_trace": payload.get("error_trace", ""),
        }

        try:
            await self.storage.learning.save(record)
        except Exception as e:
            logger.error(f"Flight Recorder failed to save crash trace: {e}")
