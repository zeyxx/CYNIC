"""
-NET Server  Somatic WebSocket Broadcaster.
Runs on a dedicated high port (58766) using IPv6.
Distributes -PULSE messages to all connected nerves (CLI, Web, etc.).
"""

from __future__ import annotations

import json
import logging
import socket
from typing import Any

import websockets

from cynic.kernel.protocol.kpulse import PulseMessage, PulseType
from cynic.kernel.core.event_bus import EventBus, CoreEvent, Event

logger = logging.getLogger("cynic.kernel.protocol.knet_server")


class KNetServer:
    """
    The Brain's Somatic Broadcaster.
    Maintains a pool of WebSocket connections and streams the organism's state.
    """

    def __init__(self, bus: EventBus, host: str = "::", port: int = 58766):
        self.bus = bus
        self.host = host
        self.port = port
        self.clients: set[Any] = set()
        self._server = None
        self._running = False
        
        # Subscribe to reputation syncs for broadcasting
        self.bus.on(CoreEvent.REPUTATION_SYNC, self._on_reputation_sync)

    async def _on_reputation_sync(self, event: Event) -> None:
        """Broadcast reputation profile to all connected nerves."""
        payload = event.dict_payload
        pulse = PulseMessage(
            type=PulseType.SOMATIC_SYNC,  # Or a new REPUTATION_SYNC pulse type
            data={"reputation": payload}
        )
        await self.broadcast(pulse)

    async def start(self):
        """
        Start the WebSocket server with IPv4 fallback and dynamic port retry.
        If port is 0, binds to a random free port.
        """
        max_retries = 5
        retry_count = 0
        current_port = self.port

        while retry_count < max_retries:
            try:
                self._server = await websockets.serve(
                    self._handler,
                    self.host,
                    current_port,
                    family=socket.AF_INET6 if ":" in self.host else socket.AF_INET,
                )
                self._running = True
                
                # Update port if it was 0 or changed due to retry
                actual_port = self._server.sockets[0].getsockname()[1]
                self.port = actual_port
                
                logger.info(f"[{self.bus.instance_id}] -NET Server active on [{self.host}]:{self.port}")
                return # Success
            except OSError as e:
                if e.errno == 10048 or "already in use" in str(e).lower():
                    logger.warning(f"[{self.bus.instance_id}] -NET port {current_port} busy. Retrying...")
                    current_port += 1
                    retry_count += 1
                else:
                    logger.error(f"Failed to start K-NET Server: {e}")
                    raise
            except Exception as e:
                if self.host == "::":
                    logger.debug("Falling back to 0.0.0.0 for K-NET Server")
                    self.host = "0.0.0.0"
                    # Don't increment retry_count for family fallback
                else:
                    logger.error(f"Critical K-NET Server error: {e}")
                    raise

        raise RuntimeError(f"Could not bind K-NET server after {max_retries} attempts.")

    async def stop(self):
        """Gracefully stop the server and unregister event listeners."""
        self._running = False

        # Unregister event bus listener
        try:
            self.bus.off(CoreEvent.REPUTATION_SYNC, self._on_reputation_sync)
        except Exception as e:
            logger.debug(f"Error unregistering KNetServer listener: {e}")

        if self._server:
            self._server.close()
            await self._server.wait_closed()
            logger.info(f"[{self.bus.instance_id}] -NET Server detached.")

    async def _handler(self, websocket, path):
        """Handle new nerve connections."""
        self.clients.add(websocket)

        # Immediate welcome pulse for instant UI update
        try:
            from cynic.interfaces.api.state import get_state

            org = get_state()
            if org:
                stats = await org.state.get_stats()
                data = {
                    "mind": {
                        "status": "AWAKE",
                        "thinking": stats.get("current_analysis", "Synchronizing..."),
                        "confidence": stats.get("confidence", 0.618),
                        "e_score": stats.get("e_score", 50.0),
                        "axiom_scores": stats.get("axiom_scores", {}),
                    }
                }
                # Add hardware
                if hasattr(org.metabolism, "body") and org.metabolism.body:
                    body_state = getattr(org.metabolism.body, "_last_state", None)
                    if body_state:
                        data["hardware"] = body_state.to_dict()

                welcome_pulse = PulseMessage(type=PulseType.SOMATIC_SYNC, data=data)
                await websocket.send(json.dumps(welcome_pulse.to_dict()))
        except Exception:
            pass

        try:
            async for message in websocket:
                # Handle incoming messages from the body (e.g. Sensory Input)
                await self._process_incoming(message, websocket)
        except websockets.exceptions.ConnectionClosed:
            pass
        finally:
            self.clients.remove(websocket)
            logger.info(f"-NET: Nerve detached. Total: {len(self.clients)}")

    async def broadcast(self, pulse: PulseMessage):
        """Stream a -PULSE to all connected nerves."""
        if not self.clients:
            return

        payload = json.dumps(pulse.to_dict())
        disconnected = set()

        for client in self.clients:
            try:
                await client.send(payload)
            except Exception:
                disconnected.add(client)

        for client in disconnected:
            self.clients.remove(client)

    async def _process_incoming(self, raw_message: str, websocket):
        """Handle data sent FROM the body to the brain."""
        try:
            data = json.loads(raw_message)
            pulse = PulseMessage.from_dict(data)
            logger.debug(f"[{self.bus.instance_id}] -NET: Received {pulse.type.value} from body")
            
            # Translate Pulse to Core Event
            await self.bus.emit(Event.typed(
                CoreEvent.PERCEPTION_RECEIVED,
                payload={
                    "content": pulse.data,
                    "reality": "SOMATIC",
                    "pulse_type": pulse.type.value
                },
                source="knet"
            ))
        except Exception as e:
            logger.warning(f"-NET: Failed to process message: {e}")


# DEPRECATED: Use Factory-managed instance instead
_KNET_SERVER: KNetServer | None = None

async def get_knet_server(bus: EventBus | None = None) -> KNetServer:
    """Legacy singleton accessor  updated to support instance bus."""
    global _KNET_SERVER
    if _KNET_SERVER is None:
        if bus is None:
            from cynic.kernel.core.event_bus import get_core_bus
            bus = get_core_bus("DEFAULT")
        _KNET_SERVER = KNetServer(bus=bus)
        await _KNET_SERVER.start()
    return _KNET_SERVER
