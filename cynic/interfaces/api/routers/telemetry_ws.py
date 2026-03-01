"""
CYNIC telemetry WebSocket router — /ws/telemetry

Streams live CYNIC activity: judgments, learning events, SONA heartbeats, meta-cycles.
Claude Code watches this stream to observe CYNIC's internal state without consuming context.

Protocol:
  connect  → {"type": "connected", "ts": ..., "phi": 1.618}
  event    → {"type": "judgment|learning|meta_cycle|sona_tick", "payload": {...}, "ts": <float>}
  heartbeat → {"type": "heartbeat", "ts": <float>} (every 30s if no real events)
"""
from __future__ import annotations

import asyncio
import logging
import time
from typing import Any

from fastapi import APIRouter, WebSocket, WebSocketDisconnect

from cynic.kernel.core.event_bus import CoreEvent, Event, get_core_bus
from cynic.kernel.core.phi import PHI

logger = logging.getLogger("cynic.interfaces.api.telemetry_ws")

router = APIRouter(tags=["telemetry"])


@router.websocket("/ws/telemetry")
async def ws_telemetry(websocket: WebSocket) -> None:
    """
    WebSocket stream: /ws/telemetry

    Streams real-time CYNIC activity for external monitoring.
    Useful for Claude Code to watch CYNIC without polling.

    Streamed events:
      JUDGMENT_CREATED  — every judgment with Q-score and verdict
      LEARNING_EVENT    — learning rate updates, Q-table changes
      META_CYCLE        — periodic meta-cognition ticks (health, cycle_n)
      SONA_TICK         — SONA heartbeat (uptime, total_judgments)

    Returns:
      {"type": "connected", "ts": <float>, "phi": 1.618}
      {"type": "judgment", "q_score": 72, "verdict": "WAG", "ts": <float>}
      {"type": "learning", "learning_rate": 0.001, "q_table_entries": 1024, "ts": <float>}
      {"type": "meta_cycle", "cycle_n": 5, "health": 0.82, "ts": <float>}
      {"type": "sona_tick", "uptime_s": 3600, "total_judgments": 12500, "ts": <float>}
      {"type": "heartbeat", "ts": <float>}  ← 30s keepalive if no real events

    Client disconnect → automatic unsubscribe.
    Queue overflow (>100) → events dropped silently.
    """
    await websocket.accept()
    bus = get_core_bus("DEFAULT")
    queue: asyncio.Queue = asyncio.Queue(maxsize=100)

    async def on_event(event: Event) -> None:
        """Queue incoming events for transmission to client."""
        try:
            event_type_name = (
                event.event_type.name
                if hasattr(event.event_type, "name")
                else str(event.event_type)
            )

            # Map CoreEvent type → message type + extract fields
            msg: dict[str, Any] = {"ts": time.time()}

            if event_type_name == "JUDGMENT_CREATED":
                msg["type"] = "judgment"
                payload = event.dict_payload or {}
                msg["q_score"] = payload.get("q_score", 0)
                msg["verdict"] = payload.get("verdict", "UNKNOWN")

            elif event_type_name == "LEARNING_EVENT":
                msg["type"] = "learning"
                payload = event.dict_payload or {}
                msg["learning_rate"] = payload.get("learning_rate", 0.0)
                msg["q_table_entries"] = payload.get("q_table_entries", 0)

            elif event_type_name == "META_CYCLE":
                msg["type"] = "meta_cycle"
                payload = event.dict_payload or {}
                msg["cycle_n"] = payload.get("cycle_n", 0)
                msg["health"] = payload.get("health", 0.0)

            elif event_type_name == "SONA_TICK":
                msg["type"] = "sona_tick"
                payload = event.dict_payload or {}
                msg["uptime_s"] = payload.get("uptime_s", 0)
                msg["total_judgments"] = payload.get("total_judgments", 0)

            else:
                # Unknown event type — skip
                return

            queue.put_nowait(msg)

        except asyncio.QueueFull:
            pass  # Drop silently — client is slow, kernel must not block

    # Subscribe to telemetry events
    telemetry_events = [
        CoreEvent.JUDGMENT_CREATED,
        CoreEvent.LEARNING_EVENT,
        CoreEvent.META_CYCLE,
        CoreEvent.SONA_TICK,
    ]
    for ev_type in telemetry_events:
        bus.on(ev_type, on_event)

    async def _emit_loop() -> None:
        """Send queued events + heartbeat keepalive every 30s."""
        while True:
            try:
                msg = await asyncio.wait_for(queue.get(), timeout=30.0)
                await websocket.send_json(msg)
            except TimeoutError:
                # Keepalive heartbeat
                try:
                    await websocket.send_json({"type": "heartbeat", "ts": time.time()})
                except Exception as exc:
                    logger.debug("ws/telemetry heartbeat failed: %s", exc)
                    raise
            except Exception as exc:
                logger.error("ws/telemetry emit error: %s", exc, exc_info=True)
                raise

    async def _receive_loop() -> None:
        """Handle client disconnect."""
        while True:
            try:
                # Just consume messages to detect disconnect
                await websocket.receive_json()
                # Ignore any client messages (read-only stream)
            except Exception as exc:
                logger.debug("ws/telemetry receive ended: %s", exc)
                raise

    try:
        # Send connection banner
        await websocket.send_json({
            "type": "connected",
            "ts": time.time(),
            "phi": PHI,
        })
        # Run both loops
        await asyncio.gather(_emit_loop(), _receive_loop())
    except WebSocketDisconnect:
        pass
    except Exception as exc:
        logger.error("ws/telemetry error: %s", exc, exc_info=True)
    finally:
        # Unsubscribe from all events
        for ev_type in telemetry_events:
            bus.off(ev_type, on_event)
