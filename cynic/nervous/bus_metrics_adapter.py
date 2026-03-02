"""
BusMetricsAdapter — Wires EventMetricsCollector to EventBus.

Every bus event is recorded in EventMetricsCollector. Key event pairs
(JUDGMENT_REQUESTED → JUDGMENT_CREATED, ACT_REQUESTED → ACT_COMPLETED)
are used to compute real end-to-end latencies.

Every HISTORY_REPLAY_BATCH events, anomaly detection runs. If anomalies
are found and a bus reference is provided, ANOMALY_DETECTED is emitted.

Usage:
    adapter = BusMetricsAdapter(collector, bus=instance_bus)
    bus.on("*", adapter.on_event)
"""
from __future__ import annotations

import logging
from typing import TYPE_CHECKING

from cynic.kernel.core.event_bus import CoreEvent, Event
from cynic.kernel.core.formulas import HISTORY_REPLAY_BATCH
from cynic.nervous.event_metrics import EventMetricsCollector

if TYPE_CHECKING:
    from cynic.kernel.core.event_bus import EventBus

logger = logging.getLogger("cynic.nervous.bus_metrics_adapter")

# Request → Response event pairs for latency measurement
_LATENCY_PAIRS: dict[str, str] = {
    "core.judgment_requested": "core.judgment_created",
    "core.act_requested":      "core.act_completed",
    "core.perception_received": "core.decision_made",
}

# Add failure paths separately to map to the same request type
_RESPONSE_TO_REQUEST: dict[str, str] = {v: k for k, v in _LATENCY_PAIRS.items()}
_RESPONSE_TO_REQUEST["core.judgment_failed"] = "core.judgment_requested"

# Error event types
_ERROR_EVENTS = {"core.judgment_failed", "core.consensus_failed", "core.internal_error",
                 "core.budget_exhausted"}


def _extract_key(event: Event) -> str | None:
    """Extract correlation key (judgment_id or id) from payload."""
    payload = event.dict_payload
    return (
        payload.get("judgment_id")
        or payload.get("id")
        or payload.get("cell_id")
    )


class BusMetricsAdapter:
    """Wildcard bus handler that populates EventMetricsCollector."""

    def __init__(
        self,
        collector: EventMetricsCollector,
        bus: EventBus | None = None,
    ) -> None:
        self._collector = collector
        self._bus = bus
        # Tracks open latency pairs: (request_type + ":" + key) → request_timestamp_s
        self._open_latencies: dict[str, float] = {}
        self._event_count = 0

    async def stop(self) -> None:
        """Unregister from bus wildcard subscription."""
        try:
            if self._bus is not None:
                self._bus.off("*", self.on_event)
        except Exception as e:
            logger.debug(f"Error unregistering BusMetricsAdapter listener: {e}")

    async def on_event(self, event: Event) -> None:
        """Record event in collector; track pair latency; check for anomalies periodically."""
        is_error = event.type in _ERROR_EVENTS
        duration_ms = 0.0

        # Pair latency: open or close a tracked request-response pair
        key = _extract_key(event)
        if key:
            if event.type in _LATENCY_PAIRS:
                # Opening side of a pair
                pair_key = f"{event.type}:{key}"
                self._open_latencies[pair_key] = event.timestamp  # Unix seconds

            elif event.type in _RESPONSE_TO_REQUEST:
                # Closing side — compute latency
                request_type = _RESPONSE_TO_REQUEST[event.type]
                pair_key = f"{request_type}:{key}"
                if pair_key in self._open_latencies:
                    duration_ms = (event.timestamp - self._open_latencies.pop(pair_key)) * 1000.0

        await self._collector.record(event.type, duration_ms=duration_ms, is_error=is_error)

        self._event_count += 1
        if self._event_count % HISTORY_REPLAY_BATCH == 0:
            await self._check_anomalies()

    async def _check_anomalies(self) -> None:
        """Run anomaly detection; emit ANOMALY_DETECTED if anything found."""
        anomalies = await self._collector.detect_anomalies()
        if anomalies and self._bus:
            for anomaly in anomalies:
                await self._bus.emit(Event(
                    type=CoreEvent.ANOMALY_DETECTED.value,
                    source="bus_metrics_adapter",
                    payload={
                        "anomaly_type": anomaly.anomaly_type,
                        "event_type": anomaly.event_type,
                        "severity": anomaly.severity,
                        "message": anomaly.message,
                    },
                ))
                logger.warning(f"Anomaly detected: {anomaly.message}")
