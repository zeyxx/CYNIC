"""
CYNIC ws router — ws/stream · ws/events
"""
from __future__ import annotations

import asyncio
import logging
import time
from typing import Any


from fastapi import APIRouter, WebSocket, WebSocketDisconnect

from cynic.core.event_bus import get_core_bus, Event, CoreEvent
from cynic.core.events_schema import ActRequestedPayload
from cynic.core.phi import PHI

logger = logging.getLogger("cynic.api.server")

router_ws = APIRouter(tags=["ws"])


# ════════════════════════════════════════════════════════════════════════════
# WS /ws/stream  (real-time event stream)
# ════════════════════════════════════════════════════════════════════════════

@router_ws.websocket("/ws/stream")
async def ws_stream(websocket: WebSocket) -> None:
    """
    WebSocket stream — bidirectional real-time kernel events.

    Events streamed (server -> client):
      JUDGMENT_CREATED  — every judgment result
      LEARNING_EVENT    — Q-table updates
      META_CYCLE        — periodic evolution ticks

    Messages received (client -> server):
      {"type": "ACT", "action": "...", "target": "..."} — emitted as ACT_REQUESTED
      {"type": "ping"}  — responds with {"type": "pong", "ts": ...}
      Any other type    — ignored silently

    Protocol:
      connect -> {"type": "connected", "phi": 1.618...}
      event   -> {"type": <CoreEvent.name>, "payload": {...}, "ts": <float>}
      ping    -> {"type": "ping", "ts": <float>}  (30s keepalive)

    Client disconnect -> clean unsubscribe from all events.
    Queue overflow (>100 buffered events) -> events dropped silently.
    """
    await websocket.accept()
    bus = get_core_bus()
    queue: asyncio.Queue = asyncio.Queue(maxsize=100)

    async def on_event(event: Event) -> None:
        try:
            queue.put_nowait({
                "type": event.event_type.name if hasattr(event.event_type, "name") else str(event.event_type),
                "payload": event.payload,
                "ts": time.time(),
            })
        except asyncio.QueueFull:
            pass  # Drop silently — client is slow, kernel must not block

    stream_events = [
        CoreEvent.JUDGMENT_CREATED,
        CoreEvent.LEARNING_EVENT,
        CoreEvent.META_CYCLE,
        CoreEvent.DECISION_MADE,
    ]
    for ev_type in stream_events:
        bus.on(ev_type, on_event)

    async def _emit_loop() -> None:
        """Send queued events to the WebSocket client."""
        while True:
            try:
                msg = await asyncio.wait_for(queue.get(), timeout=30.0)
                await websocket.send_json(msg)
            except TimeoutError:
                # Keepalive ping — proves connection is alive
                try:
                    await websocket.send_json({"type": "ping", "ts": time.time()})
                except Exception as exc:
                    logger.debug("ws/stream ping failed: %s", exc)
                    raise
            except Exception as exc:
                logger.error("ws/stream emit error: %s", exc, exc_info=True)
                raise

    async def _receive_loop() -> None:
        """Receive client messages and route them to the bus or respond directly."""
        while True:
            try:
                data = await websocket.receive_json()
                msg_type = data.get("type", "")
                if msg_type == "ping":
                    await websocket.send_json({"type": "pong", "ts": time.time()})
                elif msg_type == "ACT":
                    await bus.emit(Event.typed(
                        CoreEvent.ACT_REQUESTED,
                        ActRequestedPayload(
                            action=data.get("action", ""),
                            target=data.get("target", ""),
                        ),
                        source="ws_client",
                    ))
                # Any other type: ignored silently
            except Exception as exc:
                logger.error("ws/stream receive error: %s", exc, exc_info=True)
                raise

    try:
        await websocket.send_json({"type": "connected", "ts": time.time(), "phi": PHI})
        await asyncio.gather(_emit_loop(), _receive_loop())
    except WebSocketDisconnect:
        pass
    finally:
        for ev_type in stream_events:
            bus.off(ev_type, on_event)


# ════════════════════════════════════════════════════════════════════════════
# WS /ws/consciousness/ecosystem  (live ecosystem snapshot stream)
# ════════════════════════════════════════════════════════════════════════════

@router_ws.websocket("/ws/consciousness/ecosystem")
async def ws_consciousness_ecosystem(websocket: WebSocket) -> None:
    """
    WebSocket stream: /ws/consciousness/ecosystem

    Protocol:
      1. Client connects
      2. Server sends: {\"type\": \"connected\", \"phi\": 1.618, \"initial_snapshot\": {...}}
      3. Server sends periodic: {\"type\": \"ecosystem_update\", \"payload\": {...}, \"ts\": ...}
      4. Server sends keepalive: {\"type\": \"ping\", \"ts\": ...} every 30s

    Periodic updates every 5s from ConsciousnessService.get_ecosystem_state().
    Queue maxsize=50 (drop oldest on overflow).
    """
    await websocket.accept()

    from cynic.api.services.consciousness_service import ConsciousnessService

    service = ConsciousnessService()
    queue: asyncio.Queue = asyncio.Queue(maxsize=50)

    # Get initial snapshot
    initial = {
        "core_events": [],
        "automation_events": [],
        "agent_events": [],
        "timestamp": time.time(),
    }
    try:
        result = await service.get_ecosystem_state()
        if isinstance(result, dict):
            initial = result
    except Exception as e:
        logger.warning(f"get_ecosystem_state failed (using fallback): {e}")

    async def _emit_loop() -> None:
        """Send queued ecosystem updates to the WebSocket client."""
        while True:
            try:
                msg = await asyncio.wait_for(queue.get(), timeout=30.0)
                await websocket.send_json(msg)
            except TimeoutError:
                # Keepalive ping
                try:
                    await websocket.send_json({"type": "ping", "ts": time.time()})
                except Exception as exc:
                    logger.debug("ws/consciousness/ecosystem ping failed: %s", exc)
                    raise
            except Exception as exc:
                logger.error("ws/consciousness/ecosystem emit error: %s", exc, exc_info=True)
                raise

    async def _periodic_loop() -> None:
        """Get ecosystem snapshots every 5s and queue them."""
        while True:
            try:
                await asyncio.sleep(5.0)
                snapshot = await service.get_ecosystem_state()
                try:
                    queue.put_nowait({
                        "type": "ecosystem_update",
                        "payload": snapshot,
                        "ts": time.time(),
                    })
                except asyncio.QueueFull:
                    pass  # Drop silently if queue is full
            except Exception as exc:
                logger.error("ws/consciousness/ecosystem periodic error: %s", exc)
                raise

    try:
        # Send initial connection message
        await websocket.send_json({
            "type": "connected",
            "phi": PHI,
            "initial_snapshot": initial,
        })
        # Run both loops — if either fails, the whole connection closes
        await asyncio.gather(_emit_loop(), _periodic_loop())
    except WebSocketDisconnect:
        pass
    except Exception as exc:
        logger.error("ws/consciousness/ecosystem error: %s", exc, exc_info=True)


# ════════════════════════════════════════════════════════════════════════════
# WS /ws/events  (read-only all-events stream with client-side filter)
# ════════════════════════════════════════════════════════════════════════════

@router_ws.websocket("/ws/events")
async def ws_events(websocket: WebSocket) -> None:
    """
    Read-only WebSocket — streams ALL CoreEvents with client-side filtering.

    Protocol:
      connect  → {"type": "connected", "ts": ..., "phi": 1.618, "all_events": [...]}
      subscribe → client sends {"type": "subscribe", "events": ["JUDGMENT_CREATED", ...]}
                 → server only sends matching events (default: all)
      event    → {"type": <event_name>, "payload": {...}, "source": str, "ts": float}
      ping     → client sends {"type": "ping"} → server responds {"type": "pong", "ts": ...}

    Client disconnect → clean unsubscribe from all events.
    Queue overflow (>100 buffered events) → events dropped silently.
    """
    await websocket.accept()
    bus = get_core_bus()
    queue: asyncio.Queue = asyncio.Queue(maxsize=100)

    # All CoreEvent names → used for connected banner + subscribe validation
    all_event_names: list = [e.name for e in CoreEvent]

    # Active filter — None = all events pass; set = only matching names pass
    _active_filter: list = []  # mutable cell (empty = all events)
    _filter_lock = asyncio.Lock()

    async def on_any_event(event: Event) -> None:
        name = event.event_type.name if hasattr(event.event_type, "name") else str(event.event_type)
        async with _filter_lock:
            passes = (not _active_filter) or (name in _active_filter)
        if not passes:
            return
        try:
            queue.put_nowait({
                "type":    name,
                "payload": event.payload,
                "source":  getattr(event, "source", ""),
                "ts":      time.time(),
            })
        except asyncio.QueueFull:
            pass  # Drop silently — client is slow, kernel must not block

    # Subscribe to ALL CoreEvents
    for ev_type in CoreEvent:
        bus.on(ev_type, on_any_event)

    async def _emit_loop() -> None:
        """Send queued events to the WebSocket client."""
        while True:
            try:
                msg = await asyncio.wait_for(queue.get(), timeout=30.0)
                await websocket.send_json(msg)
            except TimeoutError:
                try:
                    await websocket.send_json({"type": "ping", "ts": time.time()})
                except Exception as exc:
                    logger.debug("ws/events ping failed: %s", exc)
                    raise
            except Exception as exc:
                logger.error("ws/events emit error: %s", exc, exc_info=True)
                raise

    async def _receive_loop() -> None:
        """Receive client messages: subscribe filter or ping."""
        nonlocal _active_filter
        while True:
            try:
                data = await websocket.receive_json()
                msg_type = data.get("type", "")
                if msg_type == "ping":
                    await websocket.send_json({"type": "pong", "ts": time.time()})
                elif msg_type == "subscribe":
                    requested = [e for e in (data.get("events") or []) if e in all_event_names]
                    async with _filter_lock:
                        _active_filter = requested
                    await websocket.send_json({
                        "type":       "subscribed",
                        "events":     requested or all_event_names,
                        "filter_all": not requested,
                        "ts":         time.time(),
                    })
            except Exception as exc:
                logger.error("ws/events receive error: %s", exc, exc_info=True)
                raise

    try:
        await websocket.send_json({
            "type":       "connected",
            "ts":         time.time(),
            "phi":        PHI,
            "all_events": all_event_names,
        })
        await asyncio.gather(_emit_loop(), _receive_loop())
    except WebSocketDisconnect:
        pass
    finally:
        for ev_type in CoreEvent:
            bus.off(ev_type, on_any_event)
